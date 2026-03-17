package api

import (
	"net/http"
	"strings"

	"github.com/golang-jwt/jwt/v5"

	"github.com/pykul/ai-ranger/workers/internal/config"
	"github.com/pykul/ai-ranger/workers/internal/constants"
)

// AuthMiddleware validates JWT Bearer tokens on protected routes.
// In development mode it is a no-op that passes all requests through.
func AuthMiddleware(cfg config.Config) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Development: bypass auth entirely.
			if cfg.Environment == constants.EnvironmentDevelopment {
				next.ServeHTTP(w, r)
				return
			}

			// Production: require valid Bearer JWT.
			auth := r.Header.Get("Authorization")
			if auth == "" || !strings.HasPrefix(auth, "Bearer ") {
				http.Error(w, "missing or invalid Authorization header", http.StatusUnauthorized)
				return
			}
			tokenStr := strings.TrimPrefix(auth, "Bearer ")

			token, err := jwt.Parse(tokenStr, func(t *jwt.Token) (any, error) {
				// Enforce HMAC signing method to prevent algorithm confusion attacks.
				if _, ok := t.Method.(*jwt.SigningMethodHMAC); !ok {
					return nil, jwt.ErrSignatureInvalid
				}
				return []byte(cfg.JWTSecret), nil
			})
			if err != nil || !token.Valid {
				http.Error(w, "invalid or expired token", http.StatusUnauthorized)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}
