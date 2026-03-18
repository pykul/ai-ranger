package api

import (
	"net/http"
	"strconv"

	"github.com/pykul/ai-ranger/workers/internal/store"
)

// defaultDays is the default time range for dashboard queries.
const defaultDays = 7

// parseDays reads the ?days query parameter, defaulting to 7.
func parseDays(r *http.Request) int {
	d, err := strconv.Atoi(r.URL.Query().Get("days"))
	if err != nil || d <= 0 {
		return defaultDays
	}
	return d
}

func dashboardOverview(chStore *store.ClickHouseStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		stats, err := chStore.GetOverview(r.Context(), parseDays(r))
		if err != nil {
			internalError(w, err)
			return
		}
		writeJSON(w, http.StatusOK, stats)
	}
}

func dashboardProviders(chStore *store.ClickHouseStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		providers, err := chStore.GetProviders(r.Context(), parseDays(r))
		if err != nil {
			internalError(w, err)
			return
		}
		writeJSON(w, http.StatusOK, providers)
	}
}

func dashboardUsers(chStore *store.ClickHouseStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		days := parseDays(r)
		provider := r.URL.Query().Get("provider")
		users, err := chStore.GetUsers(r.Context(), days, provider)
		if err != nil {
			internalError(w, err)
			return
		}
		writeJSON(w, http.StatusOK, users)
	}
}

func dashboardTraffic(chStore *store.ClickHouseStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		traffic, err := chStore.GetTrafficTimeseries(r.Context(), parseDays(r))
		if err != nil {
			internalError(w, err)
			return
		}
		writeJSON(w, http.StatusOK, traffic)
	}
}
