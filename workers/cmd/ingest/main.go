// Ingest worker: consumes EventBatch messages from RabbitMQ, writes events
// to ClickHouse, and updates agent last_seen_at in Postgres.
//
// All configuration comes from environment variables loaded into config.Config.
// Gracefully shuts down on SIGTERM/SIGINT.
package main

import (
	"log"
	"os"
	"os/signal"
	"syscall"

	"github.com/pykul/ai-ranger/workers/internal/config"
	"github.com/pykul/ai-ranger/workers/internal/consumer"
	"github.com/pykul/ai-ranger/workers/internal/database"
	"github.com/pykul/ai-ranger/workers/internal/writer"
)

func main() {
	log.Println("[ingest] Starting ingest worker...")

	cfg := config.Load()

	pg, err := database.ConnectPostgres(cfg.DatabaseURL)
	if err != nil {
		log.Fatalf("[ingest] Postgres connection failed: %v", err)
	}
	log.Println("[ingest] Connected to Postgres")

	ch, err := database.ConnectClickHouse(cfg.ClickHouseAddr, cfg.ClickHouseDatabase)
	if err != nil {
		log.Fatalf("[ingest] ClickHouse connection failed: %v", err)
	}
	log.Println("[ingest] Connected to ClickHouse")

	w := writer.New(ch, pg)

	// Graceful shutdown.
	sigs := make(chan os.Signal, 1)
	signal.Notify(sigs, syscall.SIGTERM, syscall.SIGINT)
	go func() {
		<-sigs
		log.Println("[ingest] Shutting down...")
		os.Exit(0)
	}()

	if err := consumer.Start(cfg.RabbitMQURL, w); err != nil {
		log.Fatalf("[ingest] Consumer error: %v", err)
	}
}
