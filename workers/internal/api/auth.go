// Package api - authentication handlers for dashboard login.
//
// POST /v1/auth/login  - validate admin credentials, return JWT + refresh token
// POST /v1/auth/refresh - exchange a refresh token for a new JWT
//
// In development mode these endpoints still exist but are not required.
// The dashboard skips the login screen entirely.
package api

import (
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"log"
	"net/http"
	"sync"
	"time"

	"github.com/golang-jwt/jwt/v5"
	"golang.org/x/crypto/bcrypt"

	"github.com/pykul/ai-ranger/workers/internal/config"
)

// accessTokenExpiry is the lifetime of a JWT access token.
const accessTokenExpiry = 24 * time.Hour

// refreshTokenExpiry is the lifetime of a refresh token.
const refreshTokenExpiry = 7 * 24 * time.Hour

// refreshTokenBytes is the number of random bytes in a refresh token.
const refreshTokenBytes = 32

// refreshEntry stores metadata for an issued refresh token.
type refreshEntry struct {
	email     string
	expiresAt time.Time
}

// refreshStore is an in-memory store for refresh tokens.
// A server restart invalidates all refresh tokens, which is acceptable
// for a single-admin tool.
var refreshStore = struct {
	sync.RWMutex
	tokens map[string]refreshEntry
}{tokens: make(map[string]refreshEntry)}

// LoginRequest is the JSON body for POST /v1/auth/login.
type LoginRequest struct {
	Email    string `json:"email"`
	Password string `json:"password"`
}

// LoginResponse is returned on successful authentication.
type LoginResponse struct {
	AccessToken  string `json:"access_token"`
	RefreshToken string `json:"refresh_token"`
	ExpiresIn    int    `json:"expires_in"`
}

// RefreshRequest is the JSON body for POST /v1/auth/refresh.
type RefreshRequest struct {
	RefreshToken string `json:"refresh_token"`
}

// RefreshResponse is returned on successful token refresh.
type RefreshResponse struct {
	AccessToken string `json:"access_token"`
	ExpiresIn   int    `json:"expires_in"`
}

// authLogin handles POST /v1/auth/login.
func authLogin(cfg config.Config) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req LoginRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, "invalid request body", http.StatusBadRequest)
			return
		}

		if req.Email == "" || req.Password == "" {
			http.Error(w, "email and password are required", http.StatusBadRequest)
			return
		}

		// Check credentials against the admin email and the bcrypt hash
		// computed once at startup from the plaintext ADMIN_PASSWORD env var.
		if req.Email != cfg.AdminEmail {
			http.Error(w, "invalid credentials", http.StatusUnauthorized)
			return
		}
		if len(cfg.AdminPasswordHash) == 0 {
			http.Error(w, "admin credentials not configured", http.StatusInternalServerError)
			return
		}
		if err := bcrypt.CompareHashAndPassword(cfg.AdminPasswordHash, []byte(req.Password)); err != nil {
			http.Error(w, "invalid credentials", http.StatusUnauthorized)
			return
		}

		// Issue JWT access token.
		accessToken, err := issueAccessToken(cfg.JWTSecret, req.Email)
		if err != nil {
			log.Printf("[auth] Failed to sign access token: %v", err)
			http.Error(w, "internal server error", http.StatusInternalServerError)
			return
		}

		// Issue refresh token.
		refreshToken, err := issueRefreshToken(req.Email)
		if err != nil {
			log.Printf("[auth] Failed to generate refresh token: %v", err)
			http.Error(w, "internal server error", http.StatusInternalServerError)
			return
		}

		writeJSON(w, http.StatusOK, LoginResponse{
			AccessToken:  accessToken,
			RefreshToken: refreshToken,
			ExpiresIn:    int(accessTokenExpiry.Seconds()),
		})
	}
}

// authRefresh handles POST /v1/auth/refresh.
func authRefresh(cfg config.Config) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req RefreshRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, "invalid request body", http.StatusBadRequest)
			return
		}

		if req.RefreshToken == "" {
			http.Error(w, "refresh_token is required", http.StatusBadRequest)
			return
		}

		// Look up and validate the refresh token.
		refreshStore.RLock()
		entry, exists := refreshStore.tokens[req.RefreshToken]
		refreshStore.RUnlock()

		if !exists || time.Now().After(entry.expiresAt) {
			// Clean up expired token if it exists.
			if exists {
				refreshStore.Lock()
				delete(refreshStore.tokens, req.RefreshToken)
				refreshStore.Unlock()
			}
			http.Error(w, "invalid or expired refresh token", http.StatusUnauthorized)
			return
		}

		// Issue new access token.
		accessToken, err := issueAccessToken(cfg.JWTSecret, entry.email)
		if err != nil {
			log.Printf("[auth] Failed to sign access token: %v", err)
			http.Error(w, "internal server error", http.StatusInternalServerError)
			return
		}

		writeJSON(w, http.StatusOK, RefreshResponse{
			AccessToken: accessToken,
			ExpiresIn:   int(accessTokenExpiry.Seconds()),
		})
	}
}

// issueAccessToken creates a signed JWT with standard claims.
func issueAccessToken(secret, email string) (string, error) {
	now := time.Now()
	claims := jwt.RegisteredClaims{
		Subject:   email,
		IssuedAt:  jwt.NewNumericDate(now),
		ExpiresAt: jwt.NewNumericDate(now.Add(accessTokenExpiry)),
	}
	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	return token.SignedString([]byte(secret))
}

// issueRefreshToken generates a random refresh token and stores it in memory.
func issueRefreshToken(email string) (string, error) {
	b := make([]byte, refreshTokenBytes)
	if _, err := rand.Read(b); err != nil {
		return "", err
	}
	token := hex.EncodeToString(b)

	refreshStore.Lock()
	refreshStore.tokens[token] = refreshEntry{
		email:     email,
		expiresAt: time.Now().Add(refreshTokenExpiry),
	}
	refreshStore.Unlock()

	return token, nil
}
