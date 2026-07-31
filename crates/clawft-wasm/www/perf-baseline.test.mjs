// WEFT-407 — Node unit tests for browser perf baseline helpers.
//
// Run: node --test crates/clawft-wasm/www/perf-baseline.test.mjs
//
// No browser, no WASM pkg — CI-friendly schema + target evaluation gate.

import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  SCHEMA,
  BENCHMARK,
  TARGETS,
  emptySamples,
  recordLoad,
  recordInit,
  recordMessage,
  evaluateTargets,
  buildReport,
  stubReport,
  validateReport,
  readJsHeap,
  readWasmBuffer,
  captureMemory,
} from "./perf-baseline.js";

describe("constants", () => {
  it("exposes schema and benchmark ids", () => {
    assert.equal(SCHEMA, "weftos.browser-perf.v1");
    assert.equal(BENCHMARK, "browser_perf_baseline");
  });

  it("defines BW6 targets", () => {
    assert.equal(TARGETS.load_ms, 500);
    assert.equal(TARGETS.init_ms, 200);
    assert.equal(TARGETS.first_msg_ms, 3000);
    assert.equal(TARGETS.subsequent_msg_ms, 2000);
    assert.equal(TARGETS.memory_wasm_heap_mb, 32);
  });
});

describe("samples + recording", () => {
  it("starts empty", () => {
    const s = emptySamples();
    assert.equal(s.load_ms, null);
    assert.equal(s.init_ms, null);
    assert.equal(s.first_msg_ms, null);
    assert.deepEqual(s.subsequent_msg_ms, []);
    assert.equal(s.memory.wasm_buffer_bytes, null);
  });

  it("records load, init, first and subsequent messages", () => {
    const s = emptySamples();
    recordLoad(s, 42.34);
    recordInit(s, 18.01);
    recordMessage(s, 900.4);
    recordMessage(s, 400.0);
    recordMessage(s, 450.2);
    assert.equal(s.load_ms, 42.3);
    assert.equal(s.init_ms, 18.0);
    assert.equal(s.first_msg_ms, 900.4);
    assert.deepEqual(s.subsequent_msg_ms, [400.0, 450.2]);
  });
});

describe("evaluateTargets", () => {
  it("skips null samples", () => {
    const checks = evaluateTargets(emptySamples());
    assert.ok(checks.every((c) => c.status === "skip"));
  });

  it("passes values under targets", () => {
    const s = emptySamples();
    recordLoad(s, 100);
    recordInit(s, 50);
    recordMessage(s, 1000);
    recordMessage(s, 500);
    s.memory.wasm_buffer_mb = 8;
    const checks = evaluateTargets(s);
    assert.ok(checks.every((c) => c.status === "pass"));
  });

  it("fails values over targets", () => {
    const s = emptySamples();
    recordLoad(s, 9999);
    const checks = evaluateTargets(s);
    const load = checks.find((c) => c.name === "load_ms");
    assert.equal(load?.status, "fail");
  });
});

describe("buildReport / stubReport / validateReport", () => {
  it("builds a valid stub report", () => {
    const report = stubReport({ host: "ci" });
    const v = validateReport(report);
    assert.equal(v.ok, true, v.errors.join("; "));
    assert.equal(report.mode, "stub");
    assert.equal(report.summary.ok, true);
    assert.equal(report.host, "ci");
  });

  it("builds a live report with failing check", () => {
    const s = emptySamples();
    recordLoad(s, 10);
    recordInit(s, 10);
    // first_msg over 3s target
    recordMessage(s, 5000);
    const report = buildReport({ samples: s, mode: "live" });
    assert.equal(report.summary.fail, 1);
    assert.equal(report.summary.ok, false);
    assert.equal(validateReport(report).ok, true);
  });

  it("rejects malformed reports", () => {
    assert.equal(validateReport(null).ok, false);
    assert.equal(validateReport({ schema: "nope" }).ok, false);
  });
});

describe("memory helpers", () => {
  it("returns null heap when performance.memory missing", () => {
    const heap = readJsHeap({ now: () => 0 });
    assert.equal(heap.js_heap_used_mb, null);
  });

  it("reads a mock performance.memory", () => {
    const heap = readJsHeap({
      memory: {
        usedJSHeapSize: 8 * 1024 * 1024,
        totalJSHeapSize: 16 * 1024 * 1024,
      },
    });
    assert.equal(heap.js_heap_used_mb, 8);
    assert.equal(heap.js_heap_total_mb, 16);
  });

  it("reads WebAssembly.Memory buffer size", () => {
    // 1 page = 64 KiB
    const mem = new WebAssembly.Memory({ initial: 2 });
    const out = readWasmBuffer({ memory: mem });
    assert.equal(out.wasm_buffer_bytes, 2 * 64 * 1024);
    assert.ok(out.wasm_buffer_mb != null && out.wasm_buffer_mb > 0);
  });

  it("captureMemory merges heap + wasm", () => {
    const s = emptySamples();
    const mem = new WebAssembly.Memory({ initial: 1 });
    captureMemory(s, {
      perf: {
        memory: {
          usedJSHeapSize: 1024 * 1024,
          totalJSHeapSize: 2 * 1024 * 1024,
        },
      },
      wasmModule: { memory: mem },
    });
    assert.equal(s.memory.js_heap_used_mb, 1);
    assert.equal(s.memory.wasm_buffer_bytes, 64 * 1024);
  });
});
