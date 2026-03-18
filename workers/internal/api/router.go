// Package api provides the Chi HTTP router and handler setup for the query API server.
package api

import (
	"github.com/ClickHouse/clickhouse-go/v2"
	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	httpSwagger "github.com/swaggo/http-swagger"
	"gorm.io/gorm"

	"github.com/pykul/ai-ranger/workers/internal/config"
	"github.com/pykul/ai-ranger/workers/internal/constants"
	"github.com/pykul/ai-ranger/workers/internal/store"
)

// NewRouter creates a Chi router with all API routes registered.
// All route paths reference named constants. Auth middleware is applied
// to protected routes and is a no-op in development mode.
func NewRouter(pg *gorm.DB, ch clickhouse.Conn, cfg config.Config) chi.Router {
	r := chi.NewRouter()
	r.Use(middleware.Logger)
	r.Use(middleware.Recoverer)

	chStore := store.NewClickHouseStore(ch)
	pgStore := store.NewPostgresStore(pg)

	// --- Unprotected routes (no auth required) ---

	// Health check — used by Docker and k8s probes
	r.Get(constants.RouteHealth, healthCheck())

	// Swagger UI at /docs/*
	r.Get("/docs/*", httpSwagger.WrapHandler)

	// Auth endpoints — must be accessible without a token
	r.Post(constants.RouteAuthLogin, authLogin(cfg))
	r.Post(constants.RouteAuthRefresh, authRefresh(cfg))

	// --- Protected routes (JWT required in production, bypassed in dev) ---

	r.Group(func(r chi.Router) {
		r.Use(AuthMiddleware(cfg))

		// Dashboard endpoints
		r.Get(constants.RouteDashboardOverview, dashboardOverview(chStore))
		r.Get(constants.RouteDashboardProviders, dashboardProviders(chStore))
		r.Get(constants.RouteDashboardUsers, dashboardUsers(chStore))
		r.Get(constants.RouteDashboardTraffic, dashboardTraffic(chStore))
		r.Get(constants.RouteDashboardFleet, fleetList(pgStore))
		r.Get(constants.RouteEvents, eventsList(chStore))

		// Admin endpoints
		r.Get(constants.RouteAdminTokensCreate, tokenList(pgStore))
		r.Post(constants.RouteAdminTokensCreate, tokenCreate(pgStore))
		r.Delete(constants.RouteAdminTokensDelete, tokenDelete(pgStore))
		r.Delete(constants.RouteAdminAgentsDelete, agentRevoke(pgStore))
	})

	return r
}
