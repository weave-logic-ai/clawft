---
title: Sensor-ingestion pipeline primitive — journal from the whisper probe
created: 2026-04-23
status: living doc — WILL be updated as the second sensor ships
probe: whisper.cpp HTTP service (this spike)
next_probe: camera | ToF | IMU (one of)
companion: .planning/sensors/PIPELINE-PRIMITIVE-SPIKE.md
---

# Sensor-ingestion pipeline primitive — whisper probe journal

The goal of this journal is not to describe **how whisper is wired** — the code
does that — but to capture **every "this should be a general thing" moment**
the build produced, and to answer the questions the spike brief queued up.

A second-sensor iteration (camera / ToF / IMU) is blocked on this journal
being honest; a primitive designed off one data point is an extrapolation, not
an observation. Each section below ends with an explicit what-does-sensor-2-
tell-us bullet.

## 0. Build shape (one paragraph, for orientation)

`clawft-service-whisper` is an in-process daemon service. It holds a clone of
the kernel's `SubstrateService` (an `Arc`-backed DashMap), subscribes via
`substrate.subscribe("substrate/sensor/mic/pcm_chunk")`, decodes the b64
payload into s16le PCM, windows 1–3 s of samples, wraps them in a 44-byte
RIFF/WAV header, POSTs them as `multipart/form-data` to whisper.cpp's
`/inference` endpoint (a **separate HTTP process on localhost:8080**), parses
the returned `{"text": "..."}`, and publishes the transcript to
`substrate/derived/transcript/mic`.

## 1. The earlier FFI framing was wrong

The first spike brief called for `whisper-rs` — a Rust FFI wrapper over
whisper.cpp linked into the daemon process. That brief was superseded before
code was written, after the operator noticed whisper was already running as a
standalone HTTP service on this machine. The HTTP path is the correct one and
this section exists to make that decision legible.

**Why HTTP is the right primitive for the spike:**

1. **Lifecycle separation.** The whisper service loads a ~2 GB model once and
   holds it. A daemon reboot doesn't cost that load. An FFI-linked whisper
   would; every cold daemon start would re-load the model.
2. **Language isolation.** whisper.cpp is C++ + CUDA. Linking it into a Rust
   crate drags in a compiler toolchain, CUDA headers, and a non-trivial build
   matrix. Every developer who ever touches the workspace pays that cost.
   HTTP service-ification makes whisper *someone else's operational concern*.
3. **Replaceability.** The `/inference` contract is narrow enough that we
   could swap out for faster-whisper, remote GPU, or a different model entirely
   without changing a line of WeftOS.
4. **Multiplexing.** One whisper instance can serve N daemons. An FFI-linked
   whisper cannot.
5. **The service model forces primitive axes to surface.** An FFI call would
   have hidden them. See §4 below on "what HTTP-as-stage teaches that
   FFI-as-stage wouldn't."

## 2. Answers to §4.3 of the spike brief (six concrete questions)

### Q1. Does `substrate.publish` accept binary payloads today?

**No.** Substrate is JSON-only — `publish(path, value: serde_json::Value)`.
Shovelling raw bytes requires either (a) b64-encoding them inside a JSON
envelope, (b) adding a parallel `substrate.publish_bytes` RPC + `Entry`-value
variant, or (c) a binary side-channel on the daemon socket.

**What this spike chose:** option (a) — `{ "pcm_b64": "...", "sample_rate": 16000, ... }`.

**Cost on the wire:** base64 inflates by ≈33%. At 16 kHz s16le mono, a 500 ms
chunk is 16 000 bytes raw → 21 333 bytes b64. At 2 Hz chunk cadence that's
42.6 kB/s per mic. Acceptable on a loopback Unix socket + in-process DashMap.
NOT acceptable on the eventual ESP32 radio link, where every kB costs ~8 ms
of WiFi airtime at 2.4 GHz — **we'll want a native binary path before audio
leaves the local host.**

**What the primitive probably wants:** payloads should be a tagged union
`enum Payload { Json(Value), Binary(Bytes) }` with the JSON case preserving
today's zero-cost path and the Binary case landing as a native pub/sub delta.
The substrate `Entry` gains a sibling field (or a refactored `value`), and
`SubscriberSink::ExternalStream` needs a framed wire format (length-prefixed,
or the kernel's existing RVF frame codec — already in the tree).

**Sensor-2 nudge:** a camera frame (1 MB at 720p JPEG, ~10 MB at 4K raw) makes
b64 untenable. ToF (depth grid, tens of kB per frame) is borderline. IMU
(tens of bytes per sample) is fine with JSON. **Camera will force this
axis off the default; ToF will wobble it; IMU won't move it.**

### Q2. Is a "stage" a new primitive, or is it just a Service that subscribes + publishes?

**From one data point: a Service is sufficient.** The whisper pipeline is
literally "tokio task that owns a subscription + a client + a publish call."
Nothing about the code begs for a `Stage` trait.

**What IS different from a generic service:** the shape of its lifecycle.
A pipeline stage wants:
- A **health probe** at startup (`wait_for_healthy`) that's common to most
  stages that talk to external processes.
- A **degraded-but-alive** state separate from "startup failed" — the whisper
  service stays subscribed even when whisper is unreachable, so the daemon
  doesn't crash when the user restarts whisper.cpp.
- A **shutdown drain** that awaits in-flight work before exiting.
- An **input/output declaration** — today expressed in source as
  `SUBSTRATE_PCM_INPUT_PATH` / `SUBSTRATE_TRANSCRIPT_OUTPUT_PATH` constants.
  A declarative pipeline config would want these as structured metadata.

**Provisional verdict:** a `Stage` trait is not required for the whisper
spike. A pattern is — a `PipelineStage` / `SensorStage` shape with the four
lifecycle hooks above, implemented as a convention on top of `Service`.

**Sensor-2 nudge:** a stage that CHAINS (VAD → whisper → punctuation) would
want the subscribe/publish wiring to be composable as a DAG rather than
hand-called per stage. Today we hard-code "sub input, pub output." Two-stage
pipelines will force us to either fan outputs into another subscribe loop, or
introduce a concept of a pipeline graph with declared edges.

### Q3. How does the service declare its substrate I/O?

**In code today** — literal `const` strings. `SUBSTRATE_PCM_INPUT_PATH` and
`SUBSTRATE_TRANSCRIPT_OUTPUT_PATH` are module-level constants; the config
struct carries mutable copies so the operator can override (useful for
multi-mic deployments publishing to differentiated paths).

**What didn't work:** nothing yet, because there's only one stage. A second
sensor will immediately hit "where does this path come from?" — a TOML file,
an Object-Type declaration, a capability-like registry. Not this spike.

**Sensor-2 nudge:** as soon as we have two stages, declaring I/O in code
becomes a smell. The config should live where the Workshop lives — in
substrate itself, per the Step 3 / Step 4 staircase in `.planning/ontology/
ADOPTION.md`. That turns the pipeline into a Workshop-Object-shaped value
and makes "pipelines are substrate" concrete.

### Q4. What's the smallest useful introspection surface?

**What we instrumented to debug the build** — this is the honest floor:
- `wait_for_healthy` outcome (true / false, with backoff count)
- `substrate: publish` tick + actor on each transcript
- `whisper service: dropped oldest window` warning counter (backpressure visibility)
- `transcription failed` error log with window start/end ms
- `base_url` of the whisper service (degrades visibility if the operator
  swaps it unknowingly)

**What the primitive should probably expose** on a `substrate/meta/service/whisper`
topic (not yet wired):
- `state`: `starting | healthy | degraded | stopped`
- `input_rate_hz`: rolling window
- `output_rate_hz`: rolling window
- `backlog_windows`: 0 or 1 with drop-oldest (always small; interesting
  only when the policy changes)
- `last_transcript_age_ms`: for staleness chip in Explorer
- `whisper_url` + `whisper_last_seen_ok_at`

**Where it lives:** a sibling topic under `substrate/meta/service/<id>/`,
owned by the service. That keeps Explorer's tree-walk uniform — every
service gets a health subtree at a predictable path.

**Sensor-2 nudge:** different sensors want different rate-kind metrics (frames
per second for camera, samples per second for IMU). A scalar `output_rate_hz`
is a lowest-common-denominator — the primitive may want per-stage metric
declarations in the capability metadata, not a fixed schema.

### Q5. In-process or supervised sidecar?

**Whisper answers both: the service client is in-process, the model is in a
sidecar.** This is not a cop-out — it's the "right" answer for this class of
stage and the distinction *is* the axis.

In-process parts:
- WhisperClient (tiny — just reqwest + a semaphore)
- WhisperService (tokio task wiring substrate ↔ client)
- Windower + WAV writer (pure data)

Out-of-process:
- whisper.cpp server (2 GB model in RAM, CUDA context, its own crash domain)

This decoupling is free in an HTTP-first design and expensive in an FFI-first
one. **This is the main lesson of the probe.**

**Sensor-2 nudge:** camera stages probably want their model (YOLO / Detectron)
in the *same* sidecar pattern for the same reasons. IMU / ToF preprocessing
is usually in-process because the per-sample math is cheap and has no model
to load. So "placement" is per-stage, not per-pipeline.

### Q6. What is the transcript Object Type?

**Shape emitted today:**
```json
{
  "text": " hello world",
  "start_ms": 0,
  "end_ms": 2000,
  "confidence": null,
  "lang": "en",
  "seq": 2
}
```

Notes:
- `confidence` is `null` because `response_format=json` doesn't carry it. A
  future path could hit `verbose_json` and populate from
  `segments[0].avg_logprob` — at the cost of one extra encoder sweep on the
  server. We judged that not worth the latency for live streaming; a
  batch-mode Object View variant is the better way to expose it.
- `seq` is the producer's chunk sequence id (the LAST chunk folded into the
  window). It lets a downstream joiner correlate against `substrate/sensor/mic/pcm_chunk`
  without timing assumptions.
- `tick` is carried by the substrate publish envelope, not the value. Object
  Types that want tick-scoped correlation read it off the subscribe line.

**Was "Object Type = JSON schema + path binding" expressive enough?** Yes,
for a single-path scalar-ish Object. The metadata that's missing is the
capability info the ADOPTION.md staircase will need — *which Viewers render
me, which Functions accept me, which Actions link out*. That lives one
layer up, not in this spike.

**Sensor-2 nudge:** a camera frame Object Type is not a JSON value any
sensible schema can describe — it's a binary blob + a manifest. The Object
Type primitive will need a "payload reference" mode: value-in-substrate for
structured/small, path-reference-plus-manifest for binary/large. This spike
doesn't light up that requirement; camera will.

## 3. Nine axes (A1–A9) — whisper's observed values + variants

| Axis | Whisper value | One-line gloss + future variant |
|---|---|---|
| **A1 Source cardinality** | 1 | One `substrate/sensor/mic/pcm_chunk` path. Variant: N mics (multi-node mesh) fan into one transcript path; substrate-path wildcards or a multi-subscribe would be needed — neither exists today. |
| **A2 Sink cardinality** | 1 | One `substrate/derived/transcript/mic`. Variant: split transcript + per-segment confidence + speaker-id into sibling paths — just more publishes, no API change. |
| **A3 Payload shape** | b64 PCM (in) + structured JSON (out) | See Q1 above. Variants: scalars (IMU), structured JSON, binary frames (camera), tensors (vision models), event envelopes. The primitive will need per-stage payload declarations. |
| **A4 Composition** | linear — mic → whisper → transcript | Variant: chain (VAD → chunker → whisper → punctuation), DAG (transcript + audio features joined). Today we hard-code the substrate paths; DAG composition needs a pipeline-graph spec. |
| **A5 Backpressure** | **drop-oldest (chose client-side semaphore + single pending-window slot)** | Variant: queue (per stage), block upstream, per-subscriber policy. Whisper's server mutex (API §1) forced the choice — no 429 means we serialize client-side with 1 permit. New windows arriving while a window is in flight replace the pending slot. See §3.1 below. |
| **A6 Reconfig** | cold restart only today | Variant: hot-swap model (whisper's `POST /load` — we don't exercise it), change chunk_ms at runtime (would need a signal path). Today changing `window_ms` requires a service respawn. |
| **A7 Observability** | inline tracing logs | See Q4 above. Primitive should fold into a `substrate/meta/service/<id>/` subtree; not done in this spike. |
| **A8 Error handling** | 5xx→retry-once; 4xx→log+drop; 503→retry-with-backoff; panic→daemon keeps running (service drops, not the process) | Variant: isolate (mark-degraded), quarantine (dead-letter path), propagate upstream (pause the pcm_chunk source). Service-level supervision is implicit in the daemon — the pipeline task can fail without taking the daemon down. |
| **A9 Placement** | in-daemon client + out-of-daemon whisper server | **The key answer.** Variant: pure in-proc (IMU preprocessing), supervised sidecar (camera YOLO), remote HTTP (whisper today), remote gRPC (future), shared GPU worker (whisper-across-all-daemons). See §4 below. |

### 3.1 Sub-note on backpressure choice (A5)

There are three plausible policies for "PCM arrives faster than whisper
drains." They differ in what they optimize.

| Policy | Optimizes | Cost |
|---|---|---|
| **Drop-oldest** (chosen) | freshness — "what's being said right now" | transcription gaps when utterances span dropped windows |
| **Queue bounded** | completeness for short backlogs | latency monotonically grows until queue clears; on sustained overload, degenerates |
| **Block upstream** | integrity — every sample is considered | back-pressure propagates into the PCM producer (ESP32 WiFi, host audio ring buffer) and can drop samples there instead — usually worse than dropping whole windows |

For live speech, drop-oldest is the common-sense choice; for offline batch
transcription, queue-bounded makes sense. **The primitive should expose the
policy as a config axis** rather than pick one.

## 4. What HTTP-as-stage teaches that FFI-as-stage would have hidden

This is the most important section for the primitive's shape, and it only
exists because we almost built the wrong thing.

### 4.1 Stage placement is its own axis (A9 gets promoted)

With an FFI-linked whisper, placement is invisible — the stage is a function
call in the same process. You'd never think to model placement as a first-
class concern. With HTTP, the placement is forced into the open on day one:

- The service has its own lifecycle (it can be up, down, loading, restarting).
- It has its own backpressure model that isn't yours to design.
- It has its own observability surface (stderr, port, timing blocks) that
  you don't control.
- It has its own failure modes (network partition, DNS failure, TLS cert
  rotation — none of which apply to an FFI call).

**The primitive needs to model placement as a first-class concern because
stages WILL migrate between placements over their lifetime.** A stage that
starts as in-proc (prototyping phase) will become a sidecar (production
phase) will become a remote worker (scale phase). If the primitive bakes
placement into the stage definition, every migration is a rewrite.

### 4.2 Health probes are not a cross-cutting nicety, they are a stage contract

The FFI version would have had `whisper::init()` as a synchronous 1–5 s
block at service startup. If init failed, the service wouldn't start.

The HTTP version has a multi-state health surface: `"ok"`, `"loading model"`,
unreachable, 4xx (you're talking to the wrong service), 5xx (it's crashed).
The service has to handle each differently — and "unreachable at startup" can't
abort the daemon or we have a boot-ordering ratrace with whisper.

**Primitive consequence:** every stage needs a declared `ready()` contract
with richer states than "started." The primitive should provide it as a
default no-op that stages override, not as an opt-in.

### 4.3 Client-side concurrency is a stage property, not a plumbing detail

Whisper's one-in-flight model is a service property, but the *mitigation*
(semaphore with permits=1) is client-side. A different whisper service
instance (bigger GPU farm) might allow parallel requests. The primitive
should let each stage declare a concurrency ceiling — and the service
registry should coordinate when multiple clients want to share a stage.

This is invisible to an FFI model: the function call is the function call.
The HTTP model forces you to think of the stage as having its own rate limit
that you must honor.

### 4.4 The test story gets better, not worse

Wiremock-backed tests (`end_to_end_with_mocked_whisper`,
`service_survives_whisper_down_at_start`, `drops_oldest_window_when_inference_slow`)
are hermetic, fast, and don't require a whisper binary, a model file, or
CUDA. With FFI-linked whisper, the equivalent tests would need a stub
whisper-rs, which is either a mocked dyn-trait layer (complex) or a real
whisper tiny.en load (slow + fragile).

**Primitive consequence:** stages that talk to a declared external
endpoint get test ergonomics for free via the endpoint mock. The primitive
should make "declare your external endpoints" part of the stage contract
so harnesses can construct appropriate mocks automatically.

## 5. Fidget-level observations (fodder, not conclusions)

- **"Write a WAV header" is 30 lines of known code.** No crate needed. If
  the only reason to pull a dep is to avoid writing `b"RIFF"`, skip the dep.
- **base64 in JSON works but feels ugly.** Every sensor stage that carries
  binary payload will touch this; the ugliness is a tax on the spike's
  choice to not change substrate. A primitive proposal that preserves the
  b64 path for flexibility but defaults to binary is probably correct.
- **`tokio::select!` with an optional in-flight future is awkward.** We use
  `std::future::pending()` to park the arm when there's no handle. The
  shape of that code wants to be `pipeline.run()` with backpressure as a
  builder option — another reason the stage primitive wants its own runtime,
  not raw tokio.
- **Whisper's "leading space" convention** (API §3) is a trap for naive
  clients; we strip it. If the primitive has a per-stage "output sanitizer"
  concept, it should default-in for known quirks of known endpoints.
- **The in-process `SubstrateService::clone`** (it's `Arc`-backed) is exactly
  the sharing pattern a pipeline wants. Tokio tasks get their own handle,
  all tasks see the same state. If the primitive ships with a well-named
  `SubstrateHandle` wrapper that hides the Arc, it's more discoverable.

## 6. Provisional primitive shape — SEEDED, NOT FINAL

**A `SensorStage` in WeftOS, circa whisper-probe:**

```rust
trait SensorStage {
    // Identity + introspection
    fn id(&self) -> &'static str;
    fn input_topics(&self) -> &'static [TopicDecl];
    fn output_topics(&self) -> &'static [TopicDecl];

    // Placement (A9)
    fn placement(&self) -> Placement;
    //   InProc | Sidecar { process_path } | RemoteHttp { url } | RemoteGrpc { addr }

    // Lifecycle
    async fn ready(&self) -> Readiness;
    //   Ready | LoadingModel | Degraded(reason) | Down(reason)

    // Backpressure (A5)
    fn input_policy(&self) -> BufferPolicy;
    //   DropOldest | BlockCapped | Refuse     (borrowed verbatim from ADR-017)

    // The pipeline body
    async fn run(self: Arc<Self>, substrate: SubstrateHandle, shutdown: Shutdown);
}

// Standalone observability — what the primitive layer publishes *for* the
// stage, without the stage opting in.
//   substrate/meta/service/<id>/state        — Ready | LoadingModel | …
//   substrate/meta/service/<id>/rates        — { in_hz, out_hz, dropped_hz }
//   substrate/meta/service/<id>/last_output  — tick + age_ms
```

Things that are **deliberately missing** from this sketch:
- Composition (multi-stage DAG) — wait for sensor 2.
- Hot-reload — wait for a use case that can't tolerate a restart.
- Permissions / governance — slot-shaped, filled by the ontology later.
- Declarative config — stays in code until a second stage demands it.
- Object-Type metadata on topics (capabilities, viewer bindings, etc.) —
  belongs to the ontology layer, not the pipeline layer.

**This sketch is wrong in some way.** Whisper is one data point. The rule
of three demands a second non-whisper stage before we promote any of this
to a published ADR.

## 7. Sensor-2 selection criteria (restatement of spike §7)

The next sensor must force at least one axis to a value whisper didn't take.
Ranked by how much each candidate changes:

| Candidate | Axis moves most | Why pick |
|---|---|---|
| **Camera** | A3 payload shape (binary frames), A1 source cardinality (multi-camera is common), A4 composition (frame + detections is always a DAG) | Maximum primitive pressure. Forces the binary substrate path out of hiding. |
| **ToF** | A3 payload shape (depth grid), A4 composition (heatmap + segmentation is a DAG) | Moderate — doesn't force the binary path as hard as camera, but forces per-frame structured payloads. |
| **IMU** | A4 composition (always needs a feature extractor before anything else), A9 placement (no reason to sidecar — forces pure in-proc variant) | Least pressure on payload, most pressure on composition + feature-extraction patterns. |

**Current lean: camera.** Biggest axis pressure on the parts the whisper
spike *couldn't* exercise — binary substrate, DAG composition, non-trivial
sink cardinality.

## 8. What this journal does NOT claim

- It does not propose an API. It notes where an API would be helpful.
- It does not justify the b64-in-JSON choice beyond "avoided touching
  substrate; acceptable on loopback." It explicitly flags the ESP32 path
  as a later problem.
- It does not export an Object Type. The transcript shape is in code, not
  registered with any typed layer — because no typed layer exists yet.
- It does not claim the whisper spike is "done." The spike is done when
  success criteria §9 of the brief are all ticked; this journal ticks them
  but leaves the second-sensor work open.

## 9. Where this journal plugs into the rest of the tree

- `.planning/sensors/PIPELINE-PRIMITIVE-SPIKE.md` — the brief this answers.
- `.planning/ontology/ADOPTION.md` §12 — the four-verbs table. This stage
  is `process` (Machinery + Functions). An eventual `ui://transcript`
  primitive is `display`. The writeback end of a transcribed command
  (imagine "turn off lights") is `govern` + Actions.
- `crates/clawft-service-whisper/src/service.rs` — the code these notes
  describe.
- `docs/handoff.md` — the INMP441 MEMS-mic context that the ESP32 bridge
  lives in; the upstream half of this pipeline's input path.
- `~/llama.cpp/docs/whisper-service-api.md` — the external contract this
  service is a client of.

---

*This is a living document. When the second sensor's journal merges, this one
gets revised into the primitive proposal — not kept as-is.*
