/**
 * WASM module loader with progress tracking.
 *
 * Phases:
 *   0% - download: fetching wasm-bindgen JS glue
 *  30% - compile:  compiling WASM binary (streaming if supported)
 *  70% - init:     running wasm-bindgen default() initialization
 * 100% - ready:    module fully loaded and ready
 */

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type LoadPhase = "download" | "compile" | "init" | "ready" | "error";

export interface LoadProgress {
  phase: LoadPhase;
  percent: number;
  message: string;
}

export type ProgressCallback = (progress: LoadProgress) => void;

// ---------------------------------------------------------------------------
// Capability check
// ---------------------------------------------------------------------------

/**
 * Check if the browser supports all features needed for WASM mode.
 */
export async function checkWasmCapabilities(): Promise<{
  supported: boolean;
  missing: string[];
}> {
  const missing: string[] = [];

  if (typeof WebAssembly === "undefined") {
    missing.push("WebAssembly");
  }

  if (
    !("storage" in navigator) ||
    typeof (navigator.storage as unknown as Record<string, unknown>)
      .getDirectory !== "function"
  ) {
    missing.push("Origin Private File System (OPFS)");
  }

  if (!("crypto" in globalThis) || !("subtle" in globalThis.crypto)) {
    missing.push("Web Crypto API");
  }

  return { supported: missing.length === 0, missing };
}

// ---------------------------------------------------------------------------
// Loader
// ---------------------------------------------------------------------------

/**
 * Load the clawft WASM module with streaming compilation and progress tracking.
 *
 * @param wasmJsUrl - URL to the wasm-bindgen generated JS glue file.
 * @param onProgress - Callback invoked at each phase transition.
 * @returns The loaded wasm-bindgen module.
 */
export async function loadWasmModule(
  wasmJsUrl: string,
  onProgress: ProgressCallback,
): Promise<unknown> {
  try {
    onProgress({
      phase: "download",
      percent: 0,
      message: "Downloading WASM module...",
    });

    // Dynamic import of the wasm-bindgen JS glue
    const module = await import(/* @vite-ignore */ wasmJsUrl);

    onProgress({
      phase: "compile",
      percent: 30,
      message: "Compiling WASM binary...",
    });

    // wasm-bindgen default export initializes and compiles the WASM binary.
    // If the browser supports WebAssembly.compileStreaming, wasm-bindgen
    // will use it automatically for streaming compilation.
    onProgress({
      phase: "init",
      percent: 70,
      message: "Initializing WASM runtime...",
    });

    await module.default();

    onProgress({
      phase: "ready",
      percent: 100,
      message: "WASM module ready",
    });

    return module;
  } catch (error) {
    const message =
      error instanceof Error ? error.message : String(error);
    onProgress({
      phase: "error",
      percent: 0,
      message: `Failed to load WASM: ${message}`,
    });
    throw error;
  }
}
