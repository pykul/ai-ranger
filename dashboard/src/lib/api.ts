/**
 * Thin fetch wrapper that adds Bearer token and handles 401 → refresh → retry.
 * All dashboard API calls go through this function.
 */

import { getAccessToken, refresh, clearTokens } from "./auth";

const API_BASE = "/api";

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string
  ) {
    super(message);
  }
}

export async function api<T>(
  path: string,
  options: RequestInit = {}
): Promise<T> {
  const res = await doFetch(path, options);

  if (res.status === 401) {
    const refreshed = await refresh();
    if (refreshed) {
      const retry = await doFetch(path, options);
      if (retry.ok) return retry.json() as Promise<T>;
    }
    clearTokens();
    window.location.href = "/login";
    throw new ApiError(401, "Session expired");
  }

  if (!res.ok) {
    const text = await res.text();
    throw new ApiError(res.status, text);
  }

  return res.json() as Promise<T>;
}

function doFetch(path: string, options: RequestInit): Promise<Response> {
  const headers = new Headers(options.headers);
  const token = getAccessToken();
  if (token) {
    headers.set("Authorization", `Bearer ${token}`);
  }
  return fetch(`${API_BASE}${path}`, { ...options, headers });
}

/**
 * Check if auth is required by making an unauthenticated request.
 * Returns true if auth is needed (401), false if dev mode (200).
 */
export async function isAuthRequired(): Promise<boolean> {
  try {
    const res = await fetch(`${API_BASE}/v1/dashboard/overview`);
    return res.status === 401;
  } catch {
    return true;
  }
}
