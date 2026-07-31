# clawft-bench-voice (WEFT-229)

Latency, WER, and CPU budget harness for the WeftOS voice pipeline.

**No ONNX models, mic, or live STT/TTS required** for unit/integration tests.
Real stacks plug in via the `VoicePipeline` trait or by injecting measured
samples / transcripts.

## Metrics

| Metric | Definition | Default threshold |
|--------|------------|-------------------|
| **Latency** | speech-end → first-response-byte | p95 ≤ 500 ms |
| **WER** | word error rate vs reference | ≤ 0.15 (15%) |
| **CPU (wake)** | % of one core while listening for wake | &lt; 2% |
| **CPU (pipeline)** | % of one core full duplex path | &lt; 10% |

Defaults match `.planning/voice_development.md` (VS2.1.8, VS3.3.5–7).

## Usage

```rust
use clawft_bench_voice::latency::{LatencyHarness, MockPipeline};
use clawft_bench_voice::wer::{fixture_english_corpus, word_error_rate_corpus};
use clawft_bench_voice::cpu::{CpuBudget, CpuSample};
use clawft_bench_voice::thresholds::VoiceBenchThresholds;
use clawft_bench_voice::report::VoiceBenchReport;
use std::time::Duration;

let thresholds = VoiceBenchThresholds::default();

// Latency (mock)
let mut h = LatencyHarness::new(MockPipeline::with_delay(Duration::from_millis(5)));
h.run_n(10).unwrap();
let stats = h.stats().unwrap();

// WER (fixture corpus — perfect self-match)
let wer = word_error_rate_corpus(&fixture_english_corpus());

// CPU (inject samples from your profiler)
let wake = [CpuSample::from_percent(0.8)];
let pipe = [CpuSample::from_percent(4.0)];

let eval = thresholds.evaluate(Some(&stats), Some(&wer), &wake, &pipe);
let report = VoiceBenchReport::from_evaluation(thresholds, eval, Some(stats), Some(wer));
println!("{}", report.to_json_pretty().unwrap());
```

### Live / model-backed path (optional)

Implement `VoicePipeline::run_turn` around your STT→LLM→TTS path (or only STT
if measuring speech-end → text). The harness stamps timestamps around the call.
For external timers, use `LatencySample::from_instants` / `from_duration` and
`LatencyHarness::push_sample`.

WER: pass reference + hypothesis transcripts into `word_error_rate` /
`word_error_rate_corpus` (audio fixtures live under `clawft-voice-onnx` tests;
this crate stays weights-free).

CPU: supply `CpuSample`s from host tooling (`getrusage`, Instruments, etc.) or
`CpuSample::from_times(cpu, wall)`.

## Tests

```bash
# CI-friendly (default)
scripts/build.sh test clawft-bench-voice
# or
cargo test -p clawft-bench-voice

# Optional soak (ignored by default)
cargo test -p clawft-bench-voice -- --ignored
```

## Docs

See [docs/guides/voice-bench.md](../../docs/guides/voice-bench.md) for the
operator-facing note and threshold rationale.
