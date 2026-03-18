package api

import (
	"net/http"
	"strconv"

	"github.com/pykul/ai-ranger/workers/internal/store"
)

// defaultEventsLimit is the default page size for the events endpoint.
const defaultEventsLimit = 25

// maxEventsLimit caps the page size to prevent unbounded queries.
const maxEventsLimit = 100

func eventsList(chStore *store.ClickHouseStore) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		q := r.URL.Query()

		days := parseDays(r)
		search := q.Get("q")
		sort := q.Get("sort")
		order := q.Get("order")

		page, err := strconv.Atoi(q.Get("page"))
		if err != nil || page < 1 {
			page = 1
		}

		limit, err := strconv.Atoi(q.Get("limit"))
		if err != nil || limit < 1 {
			limit = defaultEventsLimit
		}
		if limit > maxEventsLimit {
			limit = maxEventsLimit
		}

		result, err := chStore.GetEvents(r.Context(), search, days, page, limit, sort, order)
		if err != nil {
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}
		writeJSON(w, http.StatusOK, result)
	}
}
