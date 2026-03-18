/**
 * In-memory JWT token storage. Tokens are never persisted to localStorage
 * or sessionStorage. A page refresh requires re-login in production.
 * This is intentional for a security tool.
 */

let accessToken: string | null = null;
let refreshToken: string | null = null;

export function getAccessToken(): string | null {
  return accessToken;
}

export function setTokens(access: string, refresh: string): void {
  accessToken = access;
  refreshToken = refresh;
}

export function getRefreshToken(): string | null {
  return refreshToken;
}

export function clearTokens(): void {
  accessToken = null;
  refreshToken = null;
}

export async function login(
  email: string,
  password: string
): Promise<boolean> {
  const res = await fetch("/api/v1/auth/login", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email, password }),
  });
  if (!res.ok) return false;
  const data = (await res.json()) as {
    access_token: string;
    refresh_token: string;
  };
  setTokens(data.access_token, data.refresh_token);
  return true;
}

export async function refresh(): Promise<boolean> {
  const rt = getRefreshToken();
  if (!rt) return false;
  const res = await fetch("/api/v1/auth/refresh", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ refresh_token: rt }),
  });
  if (!res.ok) {
    clearTokens();
    return false;
  }
  const data = (await res.json()) as { access_token: string };
  accessToken = data.access_token;
  return true;
}
