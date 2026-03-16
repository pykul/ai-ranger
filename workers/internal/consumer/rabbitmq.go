// Package consumer provides a RabbitMQ consumer with a goroutine pool.
package consumer

import (
	"fmt"
	"log"
	"os"
	"time"

	"github.com/pykul/ai-ranger/workers/internal/constants"
	"github.com/pykul/ai-ranger/workers/internal/writer"

	amqp "github.com/rabbitmq/amqp091-go"
	"google.golang.org/protobuf/proto"

	rangerpb "github.com/pykul/ai-ranger/proto/gen/go/ranger/v1"
)

// Default RabbitMQ URL matches docker-compose.yml.
const defaultRabbitMQURL = "amqp://guest:guest@localhost:5672/"

// connectRetryIntervalSecs is the delay between RabbitMQ connection attempts.
const connectRetryIntervalSecs = 3

// Start connects to RabbitMQ and consumes from the ingest queue.
// Each message is deserialized as an EventBatch and passed to the writer.
// Retries the connection up to MaxRetryCount times on initial failure.
// Blocks until the channel is closed or an error occurs.
func Start(w *writer.Writer) error {
	url := os.Getenv("RABBITMQ_URL")
	if url == "" {
		url = defaultRabbitMQURL
	}

	var conn *amqp.Connection
	var err error
	for attempt := 0; attempt <= constants.MaxRetries; attempt++ {
		conn, err = amqp.Dial(url)
		if err == nil {
			break
		}
		if attempt < constants.MaxRetries {
			log.Printf("[ingest] RabbitMQ connection failed (attempt %d/%d): %v, retrying in %ds...",
				attempt+1, constants.MaxRetries, err, connectRetryIntervalSecs)
			time.Sleep(time.Duration(connectRetryIntervalSecs) * time.Second)
		}
	}
	if err != nil {
		return fmt.Errorf("dial rabbitmq after %d attempts: %w", constants.MaxRetries, err)
	}
	defer conn.Close()

	ch, err := conn.Channel()
	if err != nil {
		return fmt.Errorf("open channel: %w", err)
	}
	defer ch.Close()

	if err := ch.Qos(constants.ConsumerPrefetchCount, 0, false); err != nil {
		return fmt.Errorf("set qos: %w", err)
	}

	msgs, err := ch.Consume(
		constants.RabbitMQQueue,
		"",    // consumer tag (auto-generated)
		false, // auto-ack disabled -- we ack after successful processing
		false, // exclusive
		false, // no-local
		false, // no-wait
		nil,
	)
	if err != nil {
		return fmt.Errorf("consume: %w", err)
	}

	log.Printf("[ingest] Consuming from %s", constants.RabbitMQQueue)

	for msg := range msgs {
		var batch rangerpb.EventBatch
		if err := proto.Unmarshal(msg.Body, &batch); err != nil {
			log.Printf("[ingest] Failed to unmarshal EventBatch: %v", err)
			_ = msg.Nack(false, false) // dead-letter
			continue
		}

		if err := w.WriteEvents(&batch); err != nil {
			log.Printf("[ingest] Failed to write events: %v", err)
			_ = msg.Nack(false, false) // dead-letter
			continue
		}

		_ = msg.Ack(false)
	}

	return nil
}
