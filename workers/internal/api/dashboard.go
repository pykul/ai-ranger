package api

import (
	"encoding/json"
	"net/http"

	"github.com/pykul/ai-ranger/workers/internal/store"
)

// @Summary      Get dashboard overview
// @Description  Returns org-wide summary stats: total events, providers, agents, and events in the last 24h.
// @Tags         Dashboard
// @Accept       json
// @Produce      json
// @Success      200  {object}  store.OverviewStats
// @Failure      500  {string}  string  "Internal server error"
// @Router       /v1/dashboard/overview [get]
func dashboardOverview(chStore *store.ClickHouseStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		stats, err := chStore.GetOverview(r.Context())
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(stats)
	}
}

// @Summary      Get provider breakdown
// @Description  Returns per-provider stats: connection count and unique users.
// @Tags         Dashboard
// @Accept       json
// @Produce      json
// @Success      200  {array}   store.ProviderBreakdown
// @Failure      500  {string}  string  "Internal server error"
// @Router       /v1/dashboard/providers [get]
func dashboardProviders(chStore *store.ClickHouseStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		providers, err := chStore.GetProviders(r.Context())
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(providers)
	}
}

// @Summary      Get user activity
// @Description  Returns per-user activity: username, hostname, provider, app, connection count, last active.
// @Tags         Dashboard
// @Accept       json
// @Produce      json
// @Success      200  {array}   store.UserActivity
// @Failure      500  {string}  string  "Internal server error"
// @Router       /v1/dashboard/users [get]
func dashboardUsers(chStore *store.ClickHouseStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		users, err := chStore.GetUsers(r.Context())
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(users)
	}
}

// @Summary      Get traffic timeseries
// @Description  Returns hourly traffic by provider for the last 7 days.
// @Tags         Dashboard
// @Accept       json
// @Produce      json
// @Success      200  {array}   store.TrafficPoint
// @Failure      500  {string}  string  "Internal server error"
// @Router       /v1/dashboard/traffic/timeseries [get]
func dashboardTraffic(chStore *store.ClickHouseStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		traffic, err := chStore.GetTrafficTimeseries(r.Context())
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(traffic)
	}
}
