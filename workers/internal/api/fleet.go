package api

import (
	"net/http"

	"github.com/pykul/ai-ranger/workers/internal/store"
)

// @Summary      Get fleet listing
// @Description  Returns all enrolled agents with hostname, OS, version, last seen, and status.
// @Tags         Fleet
// @Accept       json
// @Produce      json
// @Success      200  {array}   store.FleetAgent
// @Failure      500  {string}  string  "Internal server error"
// @Router       /v1/dashboard/fleet [get]
func fleetList(pgStore *store.PostgresStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		agents, err := pgStore.GetFleet()
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		writeJSON(w, http.StatusOK, agents)
	}
}
