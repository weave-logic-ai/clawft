/**
 * URL validation helpers shared between the browser-mode config screen and
 * any future Tauri shell config screens.
 *
 * The validator is a pure function: no DOM, no React, no IndexedDB. It can
 * be unit-tested without a browser environment.
 *
 * Security requirement (S3.6): a `cors_proxy` URL routed through the
 * browser-only mode MUST be HTTPS in production. Plain HTTP is only
 * accepted when the host resolves to a loopback address (localhost,
 * 127.0.0.1, or ::1) so developers can iterate against a local proxy
 * without having to terminate TLS.
 */

const LOCALHOST_HOSTS = new Set<string>([
  "localhost",
  "127.0.0.1",
  "::1",
  "[::1]",
]);

export interface UrlValidationResult {
  valid: boolean;
  /** User-visible error message, populated only when `valid` is false. */
  error?: string;
}

type UrlRole = "cors_proxy" | "provider_base";

/**
 * Shared HTTPS-or-loopback check used by CORS proxy and custom provider
 * base URL fields (WEFT-310, WEFT-571).
 */
function validateHttpsOrLoopback(
  input: string | null | undefined,
  role: UrlRole,
): UrlValidationResult {
  const trimmed = (input ?? "").trim();
  if (trimmed === "") {
    return { valid: true };
  }

  let parsed: URL;
  try {
    parsed = new URL(trimmed);
  } catch {
    return {
      valid: false,
      error: `"${trimmed}" is not a valid URL.`,
    };
  }

  if (parsed.protocol === "https:") {
    return { valid: true };
  }

  if (parsed.protocol === "http:") {
    // Strip IPv6 brackets for comparison; URL preserves them in `hostname`.
    const host = parsed.hostname.toLowerCase();
    if (LOCALHOST_HOSTS.has(host) || LOCALHOST_HOSTS.has(`[${host}]`)) {
      return { valid: true };
    }
    const subject =
      role === "cors_proxy" ? "HTTP CORS proxy URLs" : "HTTP provider base URLs";
    return {
      valid: false,
      error: `${subject} are only allowed for localhost. Use HTTPS in production so your API key is not exfiltrated over the wire.`,
    };
  }

  return {
    valid: false,
    error: `Unsupported URL scheme "${parsed.protocol}". Use https:// (or http:// for localhost).`,
  };
}

/**
 * Validate a CORS proxy URL.
 *
 * - Empty / missing input returns `valid: true` (the proxy is optional;
 *   callers that require a value should check separately).
 * - Non-parseable strings are rejected.
 * - HTTPS URLs are always accepted.
 * - HTTP URLs are accepted only when the host is a loopback address.
 */
export function validateCorsProxyUrl(input: string | null | undefined): UrlValidationResult {
  return validateHttpsOrLoopback(input, "cors_proxy");
}

/**
 * Validate a custom OpenAI-compatible provider base URL (WEFT-571).
 *
 * Same rules as {@link validateCorsProxyUrl}: HTTPS in production,
 * HTTP only for loopback. Empty is allowed (optional field).
 */
export function validateProviderUrl(input: string | null | undefined): UrlValidationResult {
  return validateHttpsOrLoopback(input, "provider_base");
}
