# Voice pipeline benchmarks (WEFT-229)

Harness crate: [`crates/clawft-bench-voice`](../../crates/clawft-bench-voice/).

## Why

The 0.7.0 voice audit (VS3.3) noted missing:

1. **Latency** suite — speech-end → first-response-byte  
2. **WER** against a reference English corpus  
3. **CPU** profiling with a hard **2% wake** budget  

This crate supplies the measurement APIs, default thresholds, and CI-safe
tests (mock pipeline + fixture transcripts). Live model runs are optional.

## Default thresholds

| Metric | Limit | Source |
|--------|-------|--------|
| Latency p95 (speech-end → first byte) | ≤ 500 ms | `.planning/voice_development.md` |
| WER (corpus micro-average) | ≤ 0.15 | planning / product gate placeholder |
| Wake-word CPU | &lt; 2% of one core | VS2.1.8 / VS3.3.7 |
| Full pipeline CPU | &lt; 10% of one core | VS3.3.7 |

Override via `VoiceBenchThresholds` or JSON
(`VoiceBenchThresholds::from_json`).

## Running

```bash
# Unit + integration (no models)
scripts/build.sh test clawft-bench-voice
scripts/build.sh check   # after adding the crate to the workspace

# Optional ignored soak
cargo test -p clawft-bench-voice -- --ignored
```

Reports serialize to JSON (`VoiceBenchReport`) for CI artifacts.

## Relationship to other benches

| Crate / path | Role |
|--------------|------|
| `clawft-bench-voice` | Voice latency / WER / CPU (this guide) |
| `clawft-graphify::bench` | Graphify extraction + graph ops ([graphify-bench.md](graphify-bench.md)) |
| `clawft-edge-bench` | ESP32-S3 edge compute/network scoring (out-of-workspace) |
| `scripts/bench/` | Native binary startup / bundle size |
| `clawft-core` criterion benches | Map contention, pipeline microbenches |

## Extending with real STT

1. Implement `clawft_bench_voice::latency::VoicePipeline` for your stack, **or**
   record timestamps yourself and `push_sample`.
2. Decode audio fixtures (e.g. `crates/clawft-voice-onnx/tests/fixtures/*.wav`)
   with the STT path behind the `onnx` feature of `clawft-voice-onnx`.
3. Feed hypothesis text into `word_error_rate` against a reference.
4. Keep default CI on the mock path so `scripts/build.sh test` never needs
   weights or a microphone.
