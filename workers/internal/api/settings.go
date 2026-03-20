package api

import (
	"encoding/json"
	"log"
	"net/http"
	"net/url"
	"strings"

	"github.com/google/uuid"
	"gorm.io/gorm"

	"github.com/pykul/ai-ranger/workers/internal/store"
	"github.com/pykul/ai-ranger/workers/internal/webhook"
)

// minURLSuffixLen is the number of characters shown at the end of a masked webhook URL.
// The rest of the URL is replaced with asterisks for display.
const minURLSuffixLen = 4

// SettingsResponse is the JSON response for GET /v1/admin/settings.
type SettingsResponse struct {
	OrgID      string  `json:"org_id"`
	WebhookURL *string `json:"webhook_url"`
}

// SettingsUpdateRequest is the JSON body for PUT /v1/admin/settings.
type SettingsUpdateRequest struct {
	OrgID      string  `json:"org_id"`
	WebhookURL *string `json:"webhook_url"`
}

// maskURL returns a masked version of a URL showing only the last few characters.
// For example, "https://hooks.slack.com/services/T00/B00/xxxx" becomes
// "****xxxx". Returns nil if the input is nil or empty.
func maskURL(raw *string) *string {
	if raw == nil || *raw == "" {
		return nil
	}
	s := *raw
	if len(s) <= minURLSuffixLen {
		masked := strings.Repeat("*", len(s))
		return &masked
	}
	masked := strings.Repeat("*", len(s)-minURLSuffixLen) + s[len(s)-minURLSuffixLen:]
	return &masked
}

// @Summary      Get org settings
// @Description  Returns the current org settings including the masked webhook URL.
// @Tags         Admin
// @Produce      json
// @Success      200  {object}  SettingsResponse
// @Failure      500  {string}  string  "Internal server error"
// @Router       /v1/admin/settings [get]
func settingsGet(pgStore *store.PostgresStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		settings, err := pgStore.GetOrgSettings()
		if err != nil {
			if err == gorm.ErrRecordNotFound {
				// No settings yet -- return empty response.
				writeJSON(w, http.StatusOK, SettingsResponse{})
				return
			}
			internalError(w, err)
			return
		}

		writeJSON(w, http.StatusOK, SettingsResponse{
			OrgID:      settings.OrgID.String(),
			WebhookURL: maskURL(settings.WebhookURL),
		})
	}
}

// @Summary      Update org settings
// @Description  Updates the webhook URL for the organization. The URL must be HTTPS.
// @Tags         Admin
// @Accept       json
// @Produce      json
// @Param        body  body      SettingsUpdateRequest  true  "Settings update"
// @Success      200   {object}  SettingsResponse
// @Failure      400   {string}  string  "Invalid request"
// @Failure      500   {string}  string  "Internal server error"
// @Router       /v1/admin/settings [put]
func settingsUpdate(pgStore *store.PostgresStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req SettingsUpdateRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, "invalid request body", http.StatusBadRequest)
			return
		}

		orgID, err := uuid.Parse(req.OrgID)
		if err != nil {
			http.Error(w, "invalid org_id", http.StatusBadRequest)
			return
		}

		// Allow clearing the webhook URL by passing null or empty string.
		if req.WebhookURL != nil && *req.WebhookURL != "" {
			parsed, err := url.Parse(*req.WebhookURL)
			if err != nil || parsed.Scheme != "https" {
				http.Error(w, "webhook_url must be a valid HTTPS URL", http.StatusBadRequest)
				return
			}
		}

		// Normalize empty string to nil for storage.
		webhookURL := req.WebhookURL
		if webhookURL != nil && *webhookURL == "" {
			webhookURL = nil
		}

		if err := pgStore.UpsertOrgSettings(orgID, webhookURL); err != nil {
			internalError(w, err)
			return
		}

		writeJSON(w, http.StatusOK, SettingsResponse{
			OrgID:      orgID.String(),
			WebhookURL: maskURL(webhookURL),
		})
	}
}

// @Summary      Test webhook
// @Description  Fires a test webhook to the configured URL for the organization.
// @Tags         Admin
// @Produce      json
// @Success      200  {string}  string  "Webhook delivered"
// @Failure      404  {string}  string  "No webhook configured"
// @Failure      502  {string}  string  "Webhook delivery failed"
// @Router       /v1/admin/settings/test [post]
func settingsTest(pgStore *store.PostgresStore, notifier *webhook.Notifier) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		settings, err := pgStore.GetOrgSettings()
		if err != nil || settings.WebhookURL == nil || *settings.WebhookURL == "" {
			http.Error(w, "no webhook URL configured", http.StatusNotFound)
			return
		}

		if err := notifier.FireTestWebhook(*settings.WebhookURL); err != nil {
			// Log the full error server-side but return a generic message to the
			// client, consistent with internalError(). The upstream error may
			// contain DNS resolution details or internal network topology.
			log.Printf("[api] test webhook failed: %v", err)
			writeJSON(w, http.StatusBadGateway, errorResponse{Error: "webhook delivery failed"})
			return
		}

		writeJSON(w, http.StatusOK, map[string]string{"status": "delivered"})
	}
}
