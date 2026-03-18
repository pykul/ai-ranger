package api

import (
	"encoding/json"
	"log"
	"net/http"
)

// writeJSON encodes v as JSON and writes it to the response.
// Logs and returns 500 if encoding fails.
func writeJSON(w http.ResponseWriter, status int, v any) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	if err := json.NewEncoder(w).Encode(v); err != nil {
		log.Printf("[api] JSON encode error: %v", err)
	}
}

// errorResponse is the JSON body returned for internal errors.
type errorResponse struct {
	Error string `json:"error"`
}

// internalError logs the full error with context and returns a generic
// JSON error response to the client. Never exposes database errors,
// connection strings, or other internal details to the caller.
func internalError(w http.ResponseWriter, err error) {
	log.Printf("[api] internal error: %v", err)
	writeJSON(w, http.StatusInternalServerError, errorResponse{Error: "internal server error"})
}
