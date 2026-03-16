// Query API server: serves dashboard and fleet management endpoints.
// Connects to ClickHouse for event queries and Postgres for fleet/token management.
//
// Swagger UI at http://localhost:8081/docs
// Gracefully shuts down on SIGTERM/SIGINT.
//
// @title           AI Ranger Query API
// @version         0.1.0
// @description     Dashboard and fleet management endpoints for AI Ranger.
// @host            localhost:8081
// @BasePath        /
package main

import (
	"context"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/pykul/ai-ranger/workers/internal/api"
	"github.com/pykul/ai-ranger/workers/internal/constants"
	"github.com/pykul/ai-ranger/workers/internal/database"
)

func main() {
	log.Println("[api] Starting query API server...")

	pg, err := database.ConnectPostgres()
	if err != nil {
		log.Fatalf("[api] Postgres connection failed: %v", err)
	}
	log.Println("[api] Connected to Postgres")

	ch, err := database.ConnectClickHouse()
	if err != nil {
		log.Fatalf("[api] ClickHouse connection failed: %v", err)
	}
	log.Println("[api] Connected to ClickHouse")

	router := api.NewRouter(pg, ch)

	addr := fmt.Sprintf(":%d", constants.APIServerPort)
	srv := &http.Server{
		Addr:    addr,
		Handler: router,
	}

	// Graceful shutdown.
	go func() {
		sigs := make(chan os.Signal, 1)
		signal.Notify(sigs, syscall.SIGTERM, syscall.SIGINT)
		<-sigs
		log.Println("[api] Shutting down...")
		ctx, cancel := context.WithTimeout(context.Background(),
			time.Duration(constants.GracefulShutdownTimeout)*time.Second)
		defer cancel()
		srv.Shutdown(ctx)
	}()

	log.Printf("[api] Listening on %s (Swagger UI: http://localhost:%d/docs)", addr, constants.APIServerPort)
	if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
		log.Fatalf("[api] Server error: %v", err)
	}
}
