/**
 * CommonJS-compatible WASM loader for Node.js
 *
 * This file provides a way to load the WASM module without requiring
 * the --experimental-wasm-modules flag by manually loading the WASM bytes.
 *
 * Usage:
 *   const wasm = require('./ruvector_onnx_embeddings_wasm_cjs.js');
 *   await wasm.init(); // or wasm.initSync(wasmBytes)
 */

const fs = require('fs');
const path = require('path');

// Re-export everything from the JS bindings
const bindings = require('./ruvector_onnx_embeddings_wasm_bg.js');

// Track initialization state
let initialized = false;
let initPromise = null;

/**
 * Initialize the WASM module asynchronously
 * Automatically loads the .wasm file from the same directory
 */
async function init(wasmInput) {
  if (initialized) return bindings;

  if (initPromise) {
    await initPromise;
    return bindings;
  }

  initPromise = (async () => {
    let wasmBytes;

    if (wasmInput instanceof WebAssembly.Module) {
      // Already compiled module
      const instance = await WebAssembly.instantiate(wasmInput, getImports());
      bindings.__wbg_set_wasm(instance.exports);
      finishInit();
      return;
    } else if (wasmInput instanceof ArrayBuffer || wasmInput instanceof Uint8Array) {
      // Raw bytes provided
      wasmBytes = wasmInput;
    } else if (typeof wasmInput === 'string') {
      // Path to WASM file
      wasmBytes = fs.readFileSync(wasmInput);
    } else {
      // Auto-detect WASM file location
      const wasmPath = path.join(__dirname, 'ruvector_onnx_embeddings_wasm_bg.wasm');
      wasmBytes = fs.readFileSync(wasmPath);
    }

    const wasmModule = await WebAssembly.compile(wasmBytes);
    const instance = await WebAssembly.instantiate(wasmModule, getImports());

    bindings.__wbg_set_wasm(instance.exports);
    finishInit();
  })();

  await initPromise;
  return bindings;
}

/**
 * Initialize the WASM module synchronously
 * Requires the WASM bytes to be provided
 */
function initSync(wasmBytes) {
  if (initialized) return bindings;

  if (!wasmBytes) {
    const wasmPath = path.join(__dirname, 'ruvector_onnx_embeddings_wasm_bg.wasm');
    wasmBytes = fs.readFileSync(wasmPath);
  }

  const wasmModule = new WebAssembly.Module(wasmBytes);
  const instance = new WebAssembly.Instance(wasmModule, getImports());

  bindings.__wbg_set_wasm(instance.exports);
  finishInit();

  return bindings;
}

/**
 * Get the WASM import object
 */
function getImports() {
  return {
    './ruvector_onnx_embeddings_wasm_bg.js': bindings,
  };
}

/**
 * Finalize initialization
 */
function finishInit() {
  if (typeof bindings.__wbindgen_init_externref_table === 'function') {
    bindings.__wbindgen_init_externref_table();
  }
  initialized = true;
}

/**
 * Check if initialized
 */
function isInitialized() {
  return initialized;
}

// Export init functions and all bindings
module.exports = {
  init,
  initSync,
  isInitialized,
  default: init,
  // Re-export all bindings
  WasmEmbedder: bindings.WasmEmbedder,
  WasmEmbedderConfig: bindings.WasmEmbedderConfig,
  PoolingStrategy: bindings.PoolingStrategy,
  cosineSimilarity: bindings.cosineSimilarity,
  normalizeL2: bindings.normalizeL2,
  simd_available: bindings.simd_available,
  version: bindings.version,
};
