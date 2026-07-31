// WEFT-407 — Browser performance profiling baseline helpers.
//
// Shared by the HTML harness (main.js) and Node unit tests. Pure JS:
// no DOM, no WASM import — so it runs under `node --test` in CI without
// Chrome or a built pkg/.
//
// Metrics (BW6 / docs/browser/performance.md):
//   load_ms          — WASM module import + wasm-bindgen default init
//   init_ms          — exported init(config_json)
//   first_msg_ms     — first send_message after init
//   subsequent_msg_ms — later send_message samples
//   memory           — JS heap (Chrome) + WASM linear memory buffer size
//
// Schema id: weftos.browser-perf.v1

/** @typedef {{
 *   load_ms: number|null,
 *   init_ms: number|null,
 *   first_msg_ms: number|null,
 *   subsequent_msg_ms: number[],
 *   memory: {
 *     js_heap_used_mb: number|null,
 *     js_heap_total_mb: number|null,
 *     wasm_buffer_bytes: number|null,
 *     wasm_buffer_mb: number|null,
 *   },
 * }} PerfSamples */

export const SCHEMA = "weftos.browser-perf.v1";
export const BENCHMARK = "browser_perf_baseline";

/** BW6 targets — lower is better for all latency/memory metrics. */
export const TARGETS = Object.freeze({
  load_ms: 500,
  init_ms: 200,
  first_msg_ms: 3000,
  subsequent_msg_ms: 2000,
  memory_wasm_heap_mb: 32,
});

/**
 * Create an empty samples object.
 * @returns {PerfSamples}
 */
export function emptySamples() {
  return {
    load_ms: null,
    init_ms: null,
    first_msg_ms: null,
    subsequent_msg_ms: [],
    memory: {
      js_heap_used_mb: null,
      js_heap_total_mb: null,
      wasm_buffer_bytes: null,
      wasm_buffer_mb: null,
    },
  };
}

/**
 * Read Chrome `performance.memory` when available (non-standard).
 * @param {Performance|undefined|null} perf
 * @returns {{ js_heap_used_mb: number|null, js_heap_total_mb: number|null }}
 */
export function readJsHeap(perf) {
  const mem = perf && /** @type {any} */ (perf).memory;
  if (!mem || typeof mem.usedJSHeapSize !== "number") {
    return { js_heap_used_mb: null, js_heap_total_mb: null };
  }
  return {
    js_heap_used_mb: round3(mem.usedJSHeapSize / (1024 * 1024)),
    js_heap_total_mb: round3(mem.totalJSHeapSize / (1024 * 1024)),
  };
}

/**
 * Derive WASM linear-memory size from a wasm-bindgen module instance.
 * Looks for `memory` / `__wbindgen_export_*` exports or `WebAssembly.Memory`.
 * @param {any} wasmModule
 * @returns {{ wasm_buffer_bytes: number|null, wasm_buffer_mb: number|null }}
 */
export function readWasmBuffer(wasmModule) {
  if (!wasmModule) {
    return { wasm_buffer_bytes: null, wasm_buffer_mb: null };
  }
  /** @type {WebAssembly.Memory|null} */
  let memory = null;
  if (wasmModule.memory instanceof WebAssembly.Memory) {
    memory = wasmModule.memory;
  } else if (
    wasmModule.__wbg_memory &&
    wasmModule.__wbg_memory instanceof WebAssembly.Memory
  ) {
    memory = wasmModule.__wbg_memory;
  } else {
    // wasm-bindgen often re-exports memory under a mangled name.
    for (const key of Object.keys(wasmModule)) {
      const v = wasmModule[key];
      if (v instanceof WebAssembly.Memory) {
        memory = v;
        break;
      }
    }
  }
  if (!memory) {
    return { wasm_buffer_bytes: null, wasm_buffer_mb: null };
  }
  const bytes = memory.buffer.byteLength;
  return {
    wasm_buffer_bytes: bytes,
    wasm_buffer_mb: round3(bytes / (1024 * 1024)),
  };
}

/**
 * Snapshot all memory fields into samples.memory.
 * @param {PerfSamples} samples
 * @param {{ perf?: Performance|null, wasmModule?: any }} opts
 */
export function captureMemory(samples, opts = {}) {
  const heap = readJsHeap(opts.perf ?? globalThis.performance);
  const wasm = readWasmBuffer(opts.wasmModule);
  samples.memory = {
    js_heap_used_mb: heap.js_heap_used_mb,
    js_heap_total_mb: heap.js_heap_total_mb,
    wasm_buffer_bytes: wasm.wasm_buffer_bytes,
    wasm_buffer_mb: wasm.wasm_buffer_mb,
  };
  return samples.memory;
}

/**
 * Record load latency (ms).
 * @param {PerfSamples} samples
 * @param {number} ms
 */
export function recordLoad(samples, ms) {
  samples.load_ms = round1(ms);
}

/**
 * Record init latency (ms).
 * @param {PerfSamples} samples
 * @param {number} ms
 */
export function recordInit(samples, ms) {
  samples.init_ms = round1(ms);
}

/**
 * Record a message latency. First sample → first_msg_ms; rest → subsequent.
 * @param {PerfSamples} samples
 * @param {number} ms
 */
export function recordMessage(samples, ms) {
  const v = round1(ms);
  if (samples.first_msg_ms == null) {
    samples.first_msg_ms = v;
  } else {
    samples.subsequent_msg_ms.push(v);
  }
}

/**
 * Compare samples against TARGETS.
 * Null samples are skipped (not a fail) so stub/partial runs stay green.
 * @param {PerfSamples} samples
 * @param {typeof TARGETS} [targets]
 * @returns {{ name: string, value: number|null, target: number, status: "pass"|"fail"|"skip" }[]}
 */
export function evaluateTargets(samples, targets = TARGETS) {
  /** @type {{ name: string, value: number|null, target: number, status: "pass"|"fail"|"skip" }[]} */
  const checks = [];

  const add = (name, value, target) => {
    if (value == null || Number.isNaN(value)) {
      checks.push({ name, value: null, target, status: "skip" });
      return;
    }
    checks.push({
      name,
      value,
      target,
      status: value <= target ? "pass" : "fail",
    });
  };

  add("load_ms", samples.load_ms, targets.load_ms);
  add("init_ms", samples.init_ms, targets.init_ms);
  add("first_msg_ms", samples.first_msg_ms, targets.first_msg_ms);

  const subsequent =
    samples.subsequent_msg_ms.length > 0
      ? samples.subsequent_msg_ms.reduce((a, b) => a + b, 0) /
        samples.subsequent_msg_ms.length
      : null;
  add("subsequent_msg_ms_avg", subsequent, targets.subsequent_msg_ms);

  const memMb =
    samples.memory.wasm_buffer_mb != null
      ? samples.memory.wasm_buffer_mb
      : samples.memory.js_heap_used_mb;
  add("memory_mb", memMb, targets.memory_wasm_heap_mb);

  return checks;
}

/**
 * Build a serializable report document.
 * @param {object} opts
 * @param {PerfSamples} opts.samples
 * @param {"live"|"stub"|"partial"} opts.mode
 * @param {string} [opts.note]
 * @param {Record<string, unknown>} [opts.extra]
 */
export function buildReport({ samples, mode, note, extra }) {
  const checks = evaluateTargets(samples);
  const failed = checks.filter((c) => c.status === "fail").length;
  const passed = checks.filter((c) => c.status === "pass").length;
  const skipped = checks.filter((c) => c.status === "skip").length;

  return {
    schema: SCHEMA,
    benchmark: BENCHMARK,
    mode,
    note: note ?? null,
    timestamp: new Date().toISOString(),
    targets: { ...TARGETS },
    samples: {
      load_ms: samples.load_ms,
      init_ms: samples.init_ms,
      first_msg_ms: samples.first_msg_ms,
      subsequent_msg_ms: [...samples.subsequent_msg_ms],
      memory: { ...samples.memory },
    },
    checks,
    summary: {
      pass: passed,
      fail: failed,
      skip: skipped,
      ok: failed === 0,
    },
    ...(extra && typeof extra === "object" ? extra : {}),
  };
}

/**
 * CI-friendly stub report — schema-valid, no real timings.
 * Used when Chrome / pkg / network LLM are unavailable.
 */
export function stubReport(extra = {}) {
  const samples = emptySamples();
  return buildReport({
    samples,
    mode: "stub",
    note:
      "CI stub: no live browser timings. Run scripts/bench/browser-perf-baseline.sh " +
      "without --stub after `scripts/build.sh browser` + harness for real numbers.",
    extra,
  });
}

/**
 * Validate a report object against the v1 schema (structural).
 * @param {unknown} report
 * @returns {{ ok: boolean, errors: string[] }}
 */
export function validateReport(report) {
  /** @type {string[]} */
  const errors = [];
  if (!report || typeof report !== "object") {
    return { ok: false, errors: ["report must be an object"] };
  }
  const r = /** @type {Record<string, any>} */ (report);
  if (r.schema !== SCHEMA) {
    errors.push(`schema must be ${SCHEMA}, got ${JSON.stringify(r.schema)}`);
  }
  if (r.benchmark !== BENCHMARK) {
    errors.push(`benchmark must be ${BENCHMARK}`);
  }
  if (!["live", "stub", "partial"].includes(r.mode)) {
    errors.push(`mode must be live|stub|partial, got ${JSON.stringify(r.mode)}`);
  }
  if (!r.samples || typeof r.samples !== "object") {
    errors.push("samples object required");
  } else {
    for (const k of ["load_ms", "init_ms", "first_msg_ms"]) {
      const v = r.samples[k];
      if (v != null && (typeof v !== "number" || Number.isNaN(v))) {
        errors.push(`samples.${k} must be number|null`);
      }
    }
    if (!Array.isArray(r.samples.subsequent_msg_ms)) {
      errors.push("samples.subsequent_msg_ms must be an array");
    }
    if (!r.samples.memory || typeof r.samples.memory !== "object") {
      errors.push("samples.memory object required");
    }
  }
  if (!r.targets || typeof r.targets !== "object") {
    errors.push("targets object required");
  }
  if (!r.summary || typeof r.summary.ok !== "boolean") {
    errors.push("summary.ok boolean required");
  }
  if (!Array.isArray(r.checks)) {
    errors.push("checks array required");
  }
  return { ok: errors.length === 0, errors };
}

function round1(n) {
  return Math.round(Number(n) * 10) / 10;
}

function round3(n) {
  return Math.round(Number(n) * 1000) / 1000;
}
