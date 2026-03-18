// Package consumer provides a RabbitMQ consumer for the ingest queue.
package consumer

import (
	"fmt"
	"log"
	"time"

	"github.com/pykul/ai-ranger/workers/internal/constants"
	"github.com/pykul/ai-ranger/workers/internal/writer"

	amqp "github.com/rabbitmq/amqp091-go"
	"google.golang.org/protobuf/proto"

	rangerpb "github.com/pykul/ai-ranger/proto/gen/go/ranger/v1"
)

// connectRetryIntervalSecs is the initial delay between RabbitMQ connection attempts.
const connectRetryIntervalSecs = 3

// Start connects to RabbitMQ and consumes from the ingest queue.
// Automatically reconnects if the connection drops after initial connect.
// Blocks indefinitely until an unrecoverable error occurs.
func Start(url string, chWriter *writer.ClickHouseWriter, pgWriter *writer.PostgresWriter) error {
	for {
		conn, err := dialWithRetry(url)
		if err != nil {
			return err
		}

		msgs, err := setupChannel(conn)
		if err != nil {
			_ = conn.Close()
			return err
		}

		// Watch for connection close events to trigger reconnection.
		connClose := make(chan *amqp.Error, 1)
		conn.NotifyClose(connClose)

		log.Printf("[ingest] Consuming from %s", constants.RabbitMQQueue)
		done := make(chan struct{})
		go func() {
			consumeMessages(msgs, chWriter, pgWriter)
			close(done)
		}()

		// Wait for either the consumer to finish or the connection to drop.
		select {
		case amqpErr := <-connClose:
			if amqpErr != nil {
				log.Printf("[ingest] RabbitMQ connection lost: %v, reconnecting...", amqpErr)
			} else {
				log.Printf("[ingest] RabbitMQ connection closed, reconnecting...")
			}
			_ = conn.Close()
		case <-done:
			// msgs channel was closed without a connection error - reconnect.
			log.Printf("[ingest] Consumer channel closed, reconnecting...")
			_ = conn.Close()
		}

		// Brief pause before reconnecting to avoid tight loops.
		time.Sleep(time.Duration(connectRetryIntervalSecs) * time.Second)
	}
}

// dialWithRetry attempts to connect to RabbitMQ up to MaxRetries times.
func dialWithRetry(url string) (*amqp.Connection, error) {
	var conn *amqp.Connection
	var err error
	for attempt := 0; attempt <= constants.MaxRetries; attempt++ {
		conn, err = amqp.Dial(url)
		if err == nil {
			return conn, nil
		}
		if attempt < constants.MaxRetries {
			log.Printf("[ingest] RabbitMQ connection failed (attempt %d/%d): %v, retrying in %ds...",
				attempt+1, constants.MaxRetries, err, connectRetryIntervalSecs)
			time.Sleep(time.Duration(connectRetryIntervalSecs) * time.Second)
		}
	}
	return nil, fmt.Errorf("dial rabbitmq after %d attempts: %w", constants.MaxRetries, err)
}

// setupChannel opens a channel, sets QoS, and starts consuming from the ingest queue.
func setupChannel(conn *amqp.Connection) (<-chan amqp.Delivery, error) {
	ch, err := conn.Channel()
	if err != nil {
		return nil, fmt.Errorf("open channel: %w", err)
	}
	if err := ch.Qos(constants.ConsumerPrefetchCount, 0, false); err != nil {
		return nil, fmt.Errorf("set qos: %w", err)
	}
	msgs, err := ch.Consume(
		constants.RabbitMQQueue,
		"",    // consumer tag (auto-generated)
		false, // auto-ack disabled — we ack after successful processing
		false, // exclusive
		false, // no-local
		false, // no-wait
		nil,
	)
	if err != nil {
		return nil, fmt.Errorf("consume: %w", err)
	}
	return msgs, nil
}

// consumeMessages processes messages from the delivery channel.
// Each message is deserialized as an EventBatch and written to ClickHouse and
// Postgres independently. A failure in one writer does not block the other.
// The message is only nacked if deserialization or the ClickHouse write fails.
func consumeMessages(msgs <-chan amqp.Delivery, chWriter *writer.ClickHouseWriter, pgWriter *writer.PostgresWriter) {
	for msg := range msgs {
		var batch rangerpb.EventBatch
		if err := proto.Unmarshal(msg.Body, &batch); err != nil {
			log.Printf("[ingest] Failed to unmarshal EventBatch: %v", err)
			_ = msg.Nack(false, false)
			continue
		}

		// Write events to ClickHouse.
		agentID, chErr := chWriter.WriteEvents(&batch)
		if chErr != nil {
			log.Printf("[ingest] ClickHouse write failed: %v", chErr)
		}

		// Update agent last_seen_at in Postgres independently.
		pgWriter.UpdateAgentLastSeen(agentID)

		if chErr != nil {
			_ = msg.Nack(false, false)
			continue
		}
		_ = msg.Ack(false)
	}
}
