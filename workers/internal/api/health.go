package api

import (
	"encoding/json"
	"net/http"
)

// healthResponse is the JSON body returned by the health check endpoint.
type healthResponse struct {
	Status  string `json:"status"`
	Service string `json:"service"`
}

// healthCheck godoc
// @Summary      Health check
// @Description  Returns HTTP 200 if the API server is running. No auth required.
// @Tags         Health
// @Produce      json
// @Success      200  {object}  healthResponse
// @Router       /health [get]
func healthCheck() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		json.NewEncoder(w).Encode(healthResponse{
			Status:  "ok",
			Service: "api",
		})
	}
}
