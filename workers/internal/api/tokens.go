package api

import (
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"

	"github.com/pykul/ai-ranger/workers/internal/models"
	"github.com/pykul/ai-ranger/workers/internal/store"
)

// tokenPrefix is prepended to generated enrollment tokens for easy identification.
const tokenPrefix = "tok_"

// tokenRandomBytes is the number of random bytes used to generate the token secret.
// 32 bytes = 256 bits of entropy, hex-encoded to 64 characters.
const tokenRandomBytes = 32

func tokenList(pgStore *store.PostgresStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		tokens, err := pgStore.ListTokens()
		if err != nil {
			internalError(w, err)
			return
		}
		writeJSON(w, http.StatusOK, tokens)
	}
}

// TokenCreateRequest is the JSON body for creating an enrollment token.
type TokenCreateRequest struct {
	OrgID    string  `json:"org_id"`
	Label    *string `json:"label,omitempty"`
	MaxUses  int     `json:"max_uses"`
}

// TokenCreateResponse is returned after creating a token.
// The plaintext Token field is shown exactly once -- it cannot be retrieved later.
type TokenCreateResponse struct {
	ID      string `json:"id"`
	Token   string `json:"token"`
	OrgID   string `json:"org_id"`
	MaxUses int    `json:"max_uses"`
}

// @Summary      Create enrollment token
// @Description  Creates a new enrollment token for agent enrollment.
// @Tags         Admin
// @Accept       json
// @Produce      json
// @Param        body  body      TokenCreateRequest  true  "Token creation request"
// @Success      201   {object}  TokenCreateResponse
// @Failure      400   {string}  string  "Invalid request"
// @Failure      500   {string}  string  "Internal server error"
// @Router       /v1/admin/tokens [post]
func tokenCreate(pgStore *store.PostgresStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req TokenCreateRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, "invalid request body", http.StatusBadRequest)
			return
		}

		orgID, err := uuid.Parse(req.OrgID)
		if err != nil {
			http.Error(w, "invalid org_id", http.StatusBadRequest)
			return
		}

		if req.MaxUses < 1 {
			http.Error(w, "max_uses must be at least 1", http.StatusBadRequest)
			return
		}

		// Generate a cryptographically random plaintext token.
		randomBytes := make([]byte, tokenRandomBytes)
		if _, err := rand.Read(randomBytes); err != nil {
			internalError(w, fmt.Errorf("generate token: %w", err))
			return
		}
		plaintext := tokenPrefix + hex.EncodeToString(randomBytes)

		// Store the SHA256 hash -- the plaintext is never persisted.
		hash := sha256.Sum256([]byte(plaintext))
		tokenHash := hex.EncodeToString(hash[:])

		record := &models.EnrollmentToken{
			ID:        uuid.New(),
			OrgID:     orgID,
			Label:     req.Label,
			MaxUses:   req.MaxUses,
			TokenHash: tokenHash,
		}

		if err := pgStore.CreateToken(record); err != nil {
			internalError(w, err)
			return
		}

		// Return the plaintext token exactly once. It cannot be retrieved later.
		writeJSON(w, http.StatusCreated, TokenCreateResponse{
			ID:      record.ID.String(),
			Token:   plaintext,
			OrgID:   record.OrgID.String(),
			MaxUses: record.MaxUses,
		})
	}
}

// @Summary      Revoke enrollment token
// @Description  Deletes an enrollment token by ID, preventing further enrollments.
// @Tags         Admin
// @Accept       json
// @Produce      json
// @Param        id  path  string  true  "Token UUID"
// @Success      204  "Token revoked"
// @Failure      400  {string}  string  "Invalid UUID"
// @Failure      500  {string}  string  "Internal server error"
// @Router       /v1/admin/tokens/{id} [delete]
func tokenDelete(pgStore *store.PostgresStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id, err := uuid.Parse(chi.URLParam(r, "id"))
		if err != nil {
			http.Error(w, "invalid token id", http.StatusBadRequest)
			return
		}
		if err := pgStore.DeleteToken(id); err != nil {
			internalError(w, err)
			return
		}
		w.WriteHeader(http.StatusNoContent)
	}
}

// @Summary      Revoke agent
// @Description  Sets an agent's status to 'revoked', preventing further event submission.
// @Tags         Admin
// @Accept       json
// @Produce      json
// @Param        id  path  string  true  "Agent UUID"
// @Success      204  "Agent revoked"
// @Failure      400  {string}  string  "Invalid UUID"
// @Failure      404  {string}  string  "Agent not found"
// @Failure      500  {string}  string  "Internal server error"
// @Router       /v1/admin/agents/{id} [delete]
func agentRevoke(pgStore *store.PostgresStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		id, err := uuid.Parse(chi.URLParam(r, "id"))
		if err != nil {
			http.Error(w, "invalid agent id", http.StatusBadRequest)
			return
		}
		if err := pgStore.RevokeAgent(id); err != nil {
			internalError(w, err)
			return
		}
		w.WriteHeader(http.StatusNoContent)
	}
}
