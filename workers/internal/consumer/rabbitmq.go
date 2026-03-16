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

// connectRetryIntervalSecs is the delay between RabbitMQ connection attempts.
const connectRetryIntervalSecs = 3

// Start connects to RabbitMQ and consumes from the ingest queue.
// Blocks until the channel is closed or an error occurs.
func Start(url string, w *writer.Writer) error {
	conn, err := dialWithRetry(url)
	if err != nil {
		return err
	}
	defer conn.Close()

	msgs, err := setupChannel(conn)
	if err != nil {
		return err
	}

	log.Printf("[ingest] Consuming from %s", constants.RabbitMQQueue)
	consumeMessages(msgs, w)
	return nil
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
// Each message is deserialized as an EventBatch and passed to the writer.
// Failed messages are dead-lettered via nack.
func consumeMessages(msgs <-chan amqp.Delivery, w *writer.Writer) {
	for msg := range msgs {
		var batch rangerpb.EventBatch
		if err := proto.Unmarshal(msg.Body, &batch); err != nil {
			log.Printf("[ingest] Failed to unmarshal EventBatch: %v", err)
			_ = msg.Nack(false, false)
			continue
		}
		if err := w.WriteEvents(&batch); err != nil {
			log.Printf("[ingest] Failed to write events: %v", err)
			_ = msg.Nack(false, false)
			continue
		}
		_ = msg.Ack(false)
	}
}
