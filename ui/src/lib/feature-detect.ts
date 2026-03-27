/**
 * Browser feature detection for Axum and WASM mode requirements.
 *
 * Axum mode requires: fetch (always available in modern browsers).
 * WASM mode requires: WebAssembly, OPFS, Web Crypto, IndexedDB.
 */

// ---------------------------------------------------------------------------
// Feature report
// ---------------------------------------------------------------------------

export interface FeatureReport {
  webAssembly: boolean;
  opfs: boolean;
  webCrypto: boolean;
  serviceWorker: boolean;
  indexedDb: boolean;
  fetchStreaming: boolean;
}

/**
 * Detect browser features required for each mode.
 */
export async function detectFeatures(): Promise<FeatureReport> {
  return {
    webAssembly: typeof WebAssembly !== "undefined",
    opfs:
      typeof navigator !== "undefined" &&
      "storage" in navigator &&
      typeof (navigator.storage as { getDirectory?: unknown }).getDirectory ===
        "function",
    webCrypto:
      typeof globalThis.crypto !== "undefined" &&
      typeof globalThis.crypto.subtle !== "undefined",
    serviceWorker:
      typeof navigator !== "undefined" && "serviceWorker" in navigator,
    indexedDb: typeof indexedDB !== "undefined",
    fetchStreaming:
      typeof ReadableStream !== "undefined" &&
      typeof Response !== "undefined" &&
      typeof Response.prototype.body !== "undefined",
  };
}

// ---------------------------------------------------------------------------
// WASM mode compatibility check
// ---------------------------------------------------------------------------

export interface WasmCompatResult {
  ok: boolean;
  warnings: string[];
  errors: string[];
}

/**
 * Check whether WASM mode can run in this browser.
 */
export function canRunWasmMode(features: FeatureReport): WasmCompatResult {
  const errors: string[] = [];
  const warnings: string[] = [];

  if (!features.webAssembly) {
    errors.push("WebAssembly is not supported in this browser.");
  }

  if (!features.webCrypto) {
    errors.push("Web Crypto API is required for secure API key storage.");
  }

  if (!features.indexedDb) {
    errors.push("IndexedDB is required for configuration storage.");
  }

  if (!features.opfs) {
    warnings.push(
      "Origin Private File System is not available. File operations will use in-memory storage (data lost on reload).",
    );
  }

  if (!features.fetchStreaming) {
    warnings.push(
      "Fetch streaming is not available. LLM responses will not stream incrementally.",
    );
  }

  return { ok: errors.length === 0, warnings, errors };
}

// ---------------------------------------------------------------------------
// Preferred mode auto-detection
// ---------------------------------------------------------------------------

export type PreferredMode = "axum" | "wasm" | "mock";

/**
 * Determine the preferred backend mode based on URL params, env vars, and
 * feature detection.
 *
 * Priority:
 *  1. URL search param `?mode=wasm|axum|mock`
 *  2. VITE_BACKEND_MODE env var
 *  3. Auto-detect: try Axum health endpoint, fall back to WASM if supported
 */
export function preferredMode(): PreferredMode {
  // Check URL params
  if (typeof window !== "undefined") {
    const params = new URLSearchParams(window.location.search);
    const modeParam = params.get("mode");
    if (modeParam === "wasm" || modeParam === "axum" || modeParam === "mock") {
      return modeParam;
    }
  }

  // Check env var
  const envMode = (
    typeof import.meta !== "undefined"
      ? (import.meta as { env?: Record<string, string> }).env?.[
          "VITE_BACKEND_MODE"
        ]
      : undefined
  ) as string | undefined;

  if (envMode === "wasm" || envMode === "axum" || envMode === "mock") {
    return envMode;
  }

  // Default: axum (auto-detection handled by ModeProvider)
  return "axum";
}
