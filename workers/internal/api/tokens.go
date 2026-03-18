package api

import (
	"encoding/json"
	"net/http"

	"github.com/go-chi/chi/v5"
	"github.com/google/uuid"

	"github.com/pykul/ai-ranger/workers/internal/models"
	"github.com/pykul/ai-ranger/workers/internal/store"
)

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
type TokenCreateResponse struct {
	ID       string `json:"id"`
	OrgID    string `json:"org_id"`
	MaxUses  int    `json:"max_uses"`
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

		token := &models.EnrollmentToken{
			ID:      uuid.New(),
			OrgID:   orgID,
			Label:   req.Label,
			MaxUses: req.MaxUses,
			// token_hash would be set by the admin workflow -- placeholder for now.
			TokenHash: uuid.New().String(),
		}

		if err := pgStore.CreateToken(token); err != nil {
			internalError(w, err)
			return
		}

		writeJSON(w, http.StatusCreated, TokenCreateResponse{
			ID:      token.ID.String(),
			OrgID:   token.OrgID.String(),
			MaxUses: token.MaxUses,
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
