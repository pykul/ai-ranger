// Package webhook provides the new-provider-first-seen alerting logic.
//
// When the ingest worker processes an event for a provider that has never been
// seen before for that organization, the Notifier inserts a row into
// known_providers and fires a webhook POST to the org's configured webhook URL.
//
// Failed webhooks are logged but not retried and never block event ingest.
package webhook

import (
	"bytes"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"time"

	"github.com/google/uuid"
	"gorm.io/gorm"

	"github.com/pykul/ai-ranger/workers/internal/constants"
	"github.com/pykul/ai-ranger/workers/internal/models"
)

// Notifier checks for new providers and fires webhooks when configured.
type Notifier struct {
	db     *gorm.DB
	client *http.Client
}

// NewNotifier creates a Notifier with the given database connection.
func NewNotifier(db *gorm.DB) *Notifier {
	return &Notifier{
		db: db,
		client: &http.Client{
			Timeout: time.Duration(constants.WebhookTimeoutSecs) * time.Second,
		},
	}
}

// Payload is the JSON body sent to the webhook URL when a new provider is detected.
type Payload struct {
	Event           string `json:"event"`
	OrgID           string `json:"org_id"`
	Provider        string `json:"provider"`
	ProviderSlug    string `json:"provider_slug"`
	FirstSeenAt     string `json:"first_seen_at"`
	MachineHostname string `json:"machine_hostname"`
	OsUsername      string `json:"os_username"`
}

// contentTypeJSON is the Content-Type header value for webhook POST requests.
const contentTypeJSON = "application/json"

// CheckAndNotifyByAgentID resolves the org_id from the agent's record and
// delegates to CheckAndNotify. Used by the ingest consumer which only has
// the agent_id from the event batch.
func (n *Notifier) CheckAndNotifyByAgentID(agentID string, provider string, hostname string, osUsername string) {
	parsedID, err := uuid.Parse(agentID)
	if err != nil {
		return
	}

	var agent models.Agent
	if err := n.db.Select("org_id").Where("id = ?", parsedID).First(&agent).Error; err != nil {
		log.Printf("[webhook] Failed to resolve org_id for agent %s: %v", agentID, err)
		return
	}

	n.CheckAndNotify(agent.OrgID, provider, hostname, osUsername)
}

// CheckAndNotify checks if the given provider has been seen before for the org.
// If it is new, inserts it into known_providers and fires the webhook (if configured).
// This method is safe to call concurrently. The unique constraint on (org_id, provider)
// with ON CONFLICT DO NOTHING ensures only one goroutine wins the insert.
func (n *Notifier) CheckAndNotify(orgID uuid.UUID, provider string, hostname string, osUsername string) {
	// Attempt to insert. If the row already exists, this is a no-op.
	now := time.Now().UTC()
	record := models.KnownProvider{
		ID:          uuid.New(),
		OrgID:       orgID,
		Provider:    provider,
		FirstSeenAt: now,
	}

	// Use raw SQL for the ON CONFLICT behavior since GORM's FirstOrCreate
	// does a SELECT then INSERT which is not atomic. Table name is derived
	// from the GORM model to avoid a magic string.
	tableName := models.KnownProvider{}.TableName()
	result := n.db.Exec(
		"INSERT INTO "+tableName+" (id, org_id, provider, first_seen_at) VALUES (?, ?, ?, ?) ON CONFLICT (org_id, provider) DO NOTHING",
		record.ID, record.OrgID, record.Provider, record.FirstSeenAt,
	)
	if result.Error != nil {
		log.Printf("[webhook] Failed to check known_providers: %v", result.Error)
		return
	}

	// If no row was inserted, this provider was already known.
	if result.RowsAffected == 0 {
		return
	}

	log.Printf("[webhook] New provider detected for org %s: %s", orgID, provider)

	// Look up the webhook URL for this org.
	var settings models.OrgSettings
	if err := n.db.Where("org_id = ?", orgID).First(&settings).Error; err != nil {
		// No settings row or no webhook configured -- nothing to fire.
		return
	}
	if settings.WebhookURL == nil || *settings.WebhookURL == "" {
		return
	}

	payload := Payload{
		Event:           constants.WebhookEventNewProvider,
		OrgID:           orgID.String(),
		Provider:        provider,
		ProviderSlug:    provider,
		FirstSeenAt:     now.Format(time.RFC3339),
		MachineHostname: hostname,
		OsUsername:      osUsername,
	}

	if err := n.fireWebhook(*settings.WebhookURL, payload); err != nil {
		log.Printf("[webhook] Failed to fire webhook for org %s: %v", orgID, err)
	}
}

// FireTestWebhook sends a test webhook payload to the given URL.
// Returns an error if the request fails or returns a non-2xx status.
func (n *Notifier) FireTestWebhook(url string) error {
	payload := Payload{
		Event:           constants.WebhookEventTest,
		OrgID:           "00000000-0000-0000-0000-000000000000",
		Provider:        "Test Provider",
		ProviderSlug:    "test_provider",
		FirstSeenAt:     time.Now().UTC().Format(time.RFC3339),
		MachineHostname: "test-machine",
		OsUsername:      "test-user",
	}
	return n.fireWebhook(url, payload)
}

// fireWebhook POSTs the payload as JSON to the given URL.
func (n *Notifier) fireWebhook(url string, payload Payload) error {
	body, err := json.Marshal(payload)
	if err != nil {
		return fmt.Errorf("marshal webhook payload: %w", err)
	}

	resp, err := n.client.Post(url, contentTypeJSON, bytes.NewReader(body))
	if err != nil {
		return fmt.Errorf("POST %s: %w", url, err)
	}
	defer resp.Body.Close()

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("POST %s returned HTTP %d", url, resp.StatusCode)
	}

	log.Printf("[webhook] Delivered to %s (HTTP %d)", url, resp.StatusCode)
	return nil
}
