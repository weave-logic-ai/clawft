# Cross-Plan Gap Analysis: Browser WASM + UI + Voice

**Date**: 2026-02-24
**Status**: Analysis Complete
**Scope**: Three workstreams -- W-BROWSER (6 phases), W-UI (3 sprints / 9 phases), W-VOICE (3 sprints / 9 phases)

---

## 1. Dependency Map

### 1.1 Cross-Workstream Dependency Graph

```
Legend:
  ---->  hard dependency (blocker)
  - - -> soft dependency (beneficial but not blocking)
  [W-X]  workstream prefix

                            EXISTING INFRASTRUCTURE
                     Phase 4 (Tiered Router, Plugin System)
                         C1 (Plugin Traits -- DONE)
                         C7 (PluginHost -- DONE)
                                    |
              +---------------------+---------------------+
              |                     |                     |
              v                     v                     v
      [W-BROWSER P1]         [W-UI S1.1]          [W-VOICE VS1.1]
      Foundation              Backend API           Audio Foundation
      (feature flags,         (Axum REST+WS,       (cpal, VAD,
       platform trait          GatewayConfig        VoiceConfig in
       split)                  extensions)          clawft-types)
              |                     |                     |
              v                     v                     v
      [W-BROWSER P2]         [W-UI S1.2]          [W-VOICE VS1.2]
      Core Engine             Frontend              STT + TTS
      (runtime                Scaffolding           (sherpa-rs,
       abstraction)           (Vite+React)          voice tools)
              |                     |                     |
              v                     v                     v
      [W-BROWSER P3]         [W-UI S1.3]          [W-VOICE VS1.3]
      LLM Transport           Core Views            VoiceChannel +
      (ProviderConfig          (Dashboard,          Talk Mode
       extensions)             WebChat)             (ChannelAdapter)
              |                     |                     |
              v                     v                     v
      [W-BROWSER P4]         [W-UI S2.1-2.5]      [W-VOICE VS2.1-2.3]
      BrowserPlatform          Canvas +             Wake Word +
      (Platform impl)          Advanced Views       Platform Integration
              |                     |                     |
              v                     |                     |
      [W-BROWSER P5]               |                     |
      WASM Entry +                 |                     |
      Tools                        |                     |
              |                     |                     |
              v                     v                     v
      [W-BROWSER P6]         [W-UI S3.1-3.5]      [W-VOICE VS3.1-3.3]
      Integration              Polish +             UI Integration +
      Testing                  Production           Cloud Fallback
                                    |                     |
                                    +------->  <----------+
                                    VS3.1 depends on S1+S2
```

### 1.2 Hard Dependencies (Blockers)

| Dependency | Source | Target | Nature |
|------------|--------|--------|--------|
| D1 | C1 (Plugin Traits) | W-VOICE VS1.1 | VoiceHandler trait, voice feature flag placeholder. **DONE.** |
| D2 | C7 (ChannelAdapter trait) | W-VOICE VS1.3 | VoiceChannel implements ChannelAdapter. **DONE.** |
| D3 | W-UI S1.1 (Backend API) | W-VOICE VS1.3 | VoiceChannel needs WebSocket handler for `voice:status` events |
| D4 | W-UI S1.1 + S1.2 + S1.3 | W-VOICE VS3.1 | Voice UI components need Dashboard shell, WS hooks, shadcn components |
| D5 | W-UI S2.1 (Live Canvas) | W-VOICE VS3.1 | Voice talk overlay integrates with Canvas layout (ResizablePanel) |
| D6 | W-BROWSER P3 | W-BROWSER P4 | BrowserPlatform needs LLM transport layer working in browser |
| D7 | W-BROWSER P1-P4 | W-BROWSER P5 | WASM entry point requires all platform abstractions complete |

### 1.3 Soft Dependencies (Beneficial But Not Blocking)

| Dependency | Source | Target | Nature |
|------------|--------|--------|--------|
| S1 | W-BROWSER P5 | W-UI (future) | UI frontend could load WASM module for browser-only deployment |
| S2 | W-UI S1.1 (Axum API) | W-BROWSER P6 | Browser WASM test harness could use UI's API for testing |
| S3 | W-BROWSER P1 (feature flags) | W-VOICE VS1.1 | Voice could adopt same `native`/`browser` feature-flag pattern, but Voice uses its own `voice` feature tree |
| S4 | W-VOICE VS1.3 (VoiceChannel) | W-UI S2.5 (Channels view) | Channel status view could show VoiceChannel state |

### 1.4 Does UI S1 Backend Need to Exist Before Browser WASM?

**No.** W-BROWSER and W-UI S1 are fully independent. Browser WASM runs the AgentLoop directly in the browser -- it does not need an Axum backend. The Browser WASM plan explicitly excludes `clawft-services` (which is where the UI API lives). They share no code paths at implementation time.

However, at **integration time** (W-BROWSER P6), the browser WASM test harness would benefit from UI frontend infrastructure if the team wants a real chat interface rather than a minimal HTML page. This is optional.

### 1.5 Does Browser WASM Phase 5 Block UI Integration?

**No.** The UI is a standalone React app connecting to an Axum backend via HTTP/WebSocket. It does not depend on WASM compilation. The synergy between them (embedding WASM in the React app for browser-only mode) is a future integration point, not a current blocker.

### 1.6 Does Voice VS3 (UI Integration) Depend on UI S2?

**Yes, partially.** VS3.1 (UI Voice Integration) explicitly requires:
- S1.1 (Backend API Foundation) for WebSocket transport
- S1.2 (Frontend Scaffolding) for `use-websocket.ts`, `api-client.ts`, shadcn component library
- S1.3 (Core Views) for MainLayout shell, Dashboard structure

VS3.1 also references S2 components (ResizablePanel from Canvas) but can work with a simpler layout if S2 is not complete. The voice orchestrator states dependencies as `UI Sprint S1 (Backend API + WebSocket), UI Sprint S2 (Dashboard Framework)`.

### 1.7 Are There Circular Dependencies?

**No.** The dependency graph is a directed acyclic graph (DAG). No workstream phase requires another workstream phase that in turn requires it.

---

## 2. Shared Infrastructure

### 2.1 Feature Flag System

| Workstream | Feature Flag Pattern | Scope |
|------------|---------------------|-------|
| W-BROWSER | `native` (default) / `browser` (mutually exclusive) | `clawft-types`, `clawft-platform`, `clawft-core`, `clawft-llm`, `clawft-tools`, `clawft-wasm` |
| W-UI | `api` flag on `clawft-services` | `clawft-services` only |
| W-VOICE | `voice`, `voice-stt`, `voice-tts`, `voice-wake` granular flags | `clawft-plugin`, `clawft-tools`, `clawft-cli` |

**Assessment:** These three flag systems are orthogonal and do not conflict. They use different dimensions:
- Browser: compile target (what platform are we on?)
- UI: optional server feature (do we want the API?)
- Voice: optional capability (do we want audio?)

However, there is a **gap**: the Browser WASM plan does not address whether the `voice` feature can be compiled for the browser target. Voice uses `cpal` (native audio) and `sherpa-rs` (native ONNX runtime), neither of which compile to `wasm32-unknown-unknown`. If voice-in-browser is ever desired, Web Audio API + WASM ONNX runtime would be needed. The Voice plan does not mention this, and the Browser plan excludes it. This is acceptable for now but should be documented as a known limitation.

**Recommendation:** No shared feature flag infrastructure is needed. Each workstream owns its own flags. Document the `voice` + `browser` mutual exclusivity (voice requires native audio APIs).

### 2.2 Platform Trait Changes

| Workstream | Change to Platform Trait | File |
|------------|-------------------------|------|
| W-BROWSER | Adds `BrowserPlatform` impl; adds `?Send` conditional async_trait; restructures `clawft-platform` module tree into `native/` and `browser/` subdirs | `crates/clawft-platform/src/lib.rs`, `src/browser/*.rs` |
| W-UI | **No changes** to Platform trait. API layer uses `Arc<dyn TraitObject>` wrappers, not Platform directly | N/A |
| W-VOICE | **No changes** to Platform trait. Voice uses `cpal` directly (OS audio, not Platform-abstracted). Uses `dirs::home_dir()` for model cache | N/A |

**Assessment:** Only W-BROWSER modifies the Platform trait system. No conflict. However, Voice's `ModelDownloadManager::default_cache_dir()` calls `dirs::home_dir()` directly -- the same function that W-BROWSER P1 feature-gates behind `native`. If `clawft-plugin` ever needs to compile for browser (it should not -- voice is native-only), this would break.

**Recommendation:** Voice plan is safe as-is because `clawft-plugin` is excluded from browser builds. No action needed.

### 2.3 Config Schema Changes (`clawft-types/src/config/mod.rs`)

This is the **highest-conflict file** across all three workstreams. All three add fields to types in this file:

| Workstream | Struct Modified | Fields Added |
|------------|----------------|-------------|
| W-BROWSER (P3) | `ProviderConfig` | `browser_direct: bool`, `cors_proxy: Option<String>` |
| W-UI (S1.1) | `GatewayConfig` | `api_port: u16`, `cors_origins: Vec<String>`, `api_enabled: bool` |
| W-VOICE (VS1.1) | `Config` (root) | `voice: VoiceConfig` (new struct with ~80 lines of nested types) |

**Assessment:** The modifications target **different structs** within the same file:
- Browser adds to `ProviderConfig` (line ~156-168 area)
- UI adds to `GatewayConfig` (line ~237-274 area)
- Voice adds a new `VoiceConfig` struct and a new field on root `Config` (adds ~200 lines to the file, plus a new field on root `Config`)

These will cause **merge conflicts** if branches touch adjacent lines, but the semantic changes do not overlap.

**Risk level:** MEDIUM. Git merge conflicts are likely if two workstreams land in the same PR window, but the conflicts are trivially resolvable (all additions, no modifications to existing lines).

**Recommendation:** Sequence the config changes. Have one workstream land its config additions first, then the others rebase. The ordering should be:
1. Voice `VoiceConfig` first (largest addition, most lines, adds to root `Config`)
2. UI `GatewayConfig` extensions second (moderate change)
3. Browser `ProviderConfig` extensions last (smallest change, most likely to rebase cleanly)

### 2.4 `ProviderConfig` Structural Divergence

The Browser plan adds `browser_direct` and `cors_proxy` fields to the existing `ProviderConfig`. The current `ProviderConfig` has:

```rust
pub struct ProviderConfig {
    pub api_key: SecretString,
    pub api_base: Option<String>,
    pub extra_headers: Option<HashMap<String, String>>,
}
```

The Browser plan shows a different field name: `base_url` instead of `api_base`. This is a **naming conflict** -- the Browser consensus plan (`00-consensus-plan.md` line 297) uses `base_url`, but the actual codebase uses `api_base`.

**Gap:** The Browser plan's code samples reference `provider.base_url` but the actual struct field is `api_base`. Implementation must use the real field name or add an alias.

### 2.5 CI Pipeline Changes

| Workstream | CI Changes Needed |
|------------|------------------|
| W-BROWSER | Add `wasm-browser-check` job, `wasm-browser-size` job to `pr-gates.yml` |
| W-UI | Add `ui-lint` and `ui-test` jobs (npm/pnpm) to `pr-gates.yml`; may need Docker build update |
| W-VOICE | None explicitly stated, but `voice` feature should be tested in CI if enabled |

**Gap:** No workstream explicitly owns CI pipeline changes holistically. Each adds jobs ad hoc.

**Recommendation:** Designate one CI pipeline update task that adds all three workstream gates at once:
- Browser WASM check
- UI lint + type-check + test
- Voice feature compilation check (e.g., `cargo check --features voice -p clawft-plugin`)

---

## 3. Conflicts and Collisions

### 3.1 High-Risk File Conflicts

| File | W-BROWSER | W-UI | W-VOICE | Risk |
|------|-----------|------|---------|------|
| `clawft-types/src/config/mod.rs` | Adds `browser_direct`, `cors_proxy` to `ProviderConfig` | Adds `api_port`, `cors_origins`, `api_enabled` to `GatewayConfig` | Adds `VoiceConfig` struct (~200 lines) + `voice` field on root `Config` | **HIGH** -- All three add to this file. Merge conflicts guaranteed if branches overlap. |
| `clawft-platform/Cargo.toml` | Restructures: makes `tokio`/`reqwest`/`dirs` optional, adds `native`/`browser` features + browser deps (`wasm-bindgen`, `web-sys`, `js-sys`, `gloo-net`) | No changes | No changes | **LOW** -- Only Browser touches this. |
| `clawft-platform/src/lib.rs` | Restructures module tree (`native/` subdir, `browser/` subdir), adds conditional `?Send` async_trait | No changes | No changes | **LOW** -- Only Browser. |
| `clawft-core/Cargo.toml` | Makes `notify`, `dirs`, `tokio-util` optional; adds `native`/`browser` features | No changes | No changes | **LOW** -- Only Browser. But if Voice adds deps to `clawft-plugin` that flow upward, or if UI's `api` feature needs core deps, this could conflict later. |
| `clawft-plugin/Cargo.toml` | No changes in Browser plan | No changes | Adds `sherpa-rs`, `cpal`, `rustpotter`, `ringbuf` behind `voice-*` features | **LOW** -- Only Voice. |
| `clawft-plugin/src/traits.rs` | No direct changes, but `?Send` async_trait applies to `Platform` trait (in `clawft-platform`, not here). Browser plan does NOT modify plugin traits | No changes | Implements `VoiceHandler` trait with real backend; adds `VoiceChannel` implementing `ChannelAdapter`. May extend `VoiceHandler` trait beyond placeholder | **LOW** -- Only Voice modifies, but Browser's `?Send` pattern could conflict if Voice traits need browser compat (they don't). |
| `clawft-services/Cargo.toml` | No changes | Adds `axum`, `axum-extra`, `tower-http`, `tokio-tungstenite` behind `api` feature | WebSocket voice events (`voice:status`) need additions here | **MEDIUM** -- UI adds API feature; Voice adds WS event types. If both modify concurrently, conflict on `Cargo.toml` features section. |
| `clawft-services/src/` (new files) | No changes | Creates `api/` module tree (5+ new files) | Adds WS event types for voice status | **LOW** -- UI creates new files, Voice extends the WS events (which UI defines). Sequential dependency (Voice VS1.3 depends on UI S1.1). |
| `clawft-wasm/Cargo.toml` | Restructures features, adds `browser` feature with core deps | No changes | No changes | **LOW** -- Only Browser. |
| `clawft-wasm/src/lib.rs` | Rewrites to wire real `AgentLoop<BrowserPlatform>` | No changes | No changes | **LOW** -- Only Browser. |
| `.github/workflows/pr-gates.yml` | Adds 2 new jobs | Adds 2-3 new jobs | Implicit need for voice feature test | **MEDIUM** -- All three want to modify CI. |
| `tests/fixtures/config.json` | May need `browser_direct`/`cors_proxy` fields | May need `api_port`/`cors_origins` fields | May need `voice` section | **MEDIUM** -- All three add config fields that test fixtures need to reflect. |

### 3.2 Collision Hotspot: `config/mod.rs`

This file is ~400 lines currently. After all three workstreams:
- Voice adds ~200 lines (VoiceConfig + sub-types)
- UI adds ~20 lines (GatewayConfig fields + defaults)
- Browser adds ~5 lines (ProviderConfig fields)

Total growth: ~225 lines. File will be ~625 lines, approaching the 500-line project guideline. Consider splitting `VoiceConfig` into a `config/voice.rs` sub-module (Voice plan does not mention this).

### 3.3 Collision Hotspot: `Cargo.toml` Feature Sections

The workspace `Cargo.toml` will need feature coordination:
- Browser adds `native`/`browser` workspace features
- Voice adds `voice`/`voice-stt`/`voice-tts`/`voice-wake` workspace features
- UI adds `api`/`ui` features to `clawft-services` and `clawft-cli`

These are additive and non-conflicting in semantics, but concurrent edits to the same `[features]` section will produce merge conflicts.

---

## 4. Gaps

### 4.1 Browser WASM Gaps

| # | Gap | Severity | Detail |
|---|-----|----------|--------|
| G1 | IndexedDB session persistence not specified | MEDIUM | The consensus plan mentions IndexedDB for config storage and OPFS for file operations, but does not specify how conversation sessions (currently JSONL files) map to browser storage. The `SessionManager` uses `Platform::fs()` for JSONL reads/writes -- this should work via `BrowserFileSystem` + OPFS, but the plan does not explicitly verify this path or address OPFS storage quotas. |
| G2 | `ProviderConfig.base_url` vs `api_base` naming mismatch | LOW | Plan code samples use `base_url` but actual struct has `api_base`. Implementation must use `api_base` or add a serde alias. |
| G3 | `getrandom` crate WASM feature not pre-wired | LOW | Noted in risk register (R6) but not in any task. Need to add `getrandom = { features = ["js"] }` to workspace deps. Should be a subtask of P1. |
| G4 | Web Worker architecture deferred as stretch goal | LOW | P6.6 defers Web Worker to stretch. Without it, heavy pipeline computation blocks the UI thread. Acceptable for MVP but should be promoted if performance testing (P6.5) shows issues. |
| G5 | No plan for config migration | MEDIUM | Browser adds `browser_direct`/`cors_proxy` to ProviderConfig with `#[serde(default)]`, so existing configs parse fine. But there is no explicit migration or versioning strategy when all three workstreams add fields simultaneously. |

### 4.2 UI Gaps

| # | Gap | Severity | Detail |
|---|-----|----------|--------|
| G6 | No browser-only deployment plan | MEDIUM | The UI plan assumes an Axum backend is always running. If users want a browser-only deployment (no Rust server), they would need Browser WASM. But the UI plan does not reference Browser WASM at all, and Browser WASM's test harness (P6.1) is a separate minimal HTML page, not the React dashboard. There is no plan to bridge these. |
| G7 | `weft ui` command assumes NativePlatform | LOW | The `weft ui` command in `clawft-cli/src/commands/ui.rs` creates a `NativePlatform::new()`. If Browser WASM restructures `clawft-platform` to gate `NativePlatform` behind `#[cfg(feature = "native")]`, the CLI code is fine (CLI is always native), but the import path may change. |
| G8 | WebSocket authentication not specified | MEDIUM | The WS upgrade at `/ws` does not validate Bearer tokens. The `require_auth` middleware skips `/api/auth/token` but does not mention `/ws`. The S1 plan's auth middleware needs to be extended or the WS handler must validate the token from query params or the first message. |
| G9 | No TypeScript type generation from Rust | LOW | S2 states "All protocol types are defined in `clawft-types` first, then mirrored to TypeScript." But there is no automated generation step (e.g., `ts-rs`, `specta`). Manual mirroring will drift. |

### 4.3 Voice Gaps

| # | Gap | Severity | Detail |
|---|-----|----------|--------|
| G10 | No browser audio support (Web Audio API) | LOW | Voice uses `cpal` for native audio capture/playback. The plan does not mention Web Audio API at all. If voice-in-browser is desired, this is a major gap -- sherpa-rs ONNX models would need a browser ONNX runtime (ort-web or onnxruntime-web). However, the Voice plan explicitly targets native-only, so this is acceptable as a documented limitation. |
| G11 | `dirs::home_dir()` in `ModelDownloadManager` | LOW | `ModelDownloadManager::default_cache_dir()` calls `dirs::home_dir()`. This is fine for native but would break if `clawft-plugin` ever compiled for browser. Since `clawft-plugin` is excluded from browser builds, this is not a current issue. |
| G12 | VoiceChannel WebSocket events need UI S1.1 | MEDIUM | VS1.3 specifies `voice:status` WebSocket events (`idle`/`listening`/`processing`/`speaking`). These events route through the `BusAccess` trait and the WS handler from UI S1.1. If Voice VS1.3 starts before UI S1.1 is complete, the voice events have no transport to the frontend. The CLI-only fallback (terminal status display) is not specified. |
| G13 | `sherpa-rs` version pinning deferred | MEDIUM | VS1.1.1 says "Exact `sherpa-rs` version pinned after VP1 prototype validation." SHA-256 hashes in `ModelDownloadManager` are `PLACEHOLDER_SHA256_*`. VP (pre-implementation validation) must complete before VS1.1 can produce a working build. |
| G14 | Voice model download requires `reqwest` | LOW | `ModelDownloadManager::ensure_model()` uses `reqwest::Client::new()`. The `clawft-plugin` Cargo.toml does not list `reqwest` as a dependency. Either add `reqwest` behind a voice feature, or use the Platform `HttpClient` trait (but `clawft-plugin` does not depend on `clawft-platform`). |

### 4.4 Cross-Cutting Gaps

| # | Gap | Severity | Detail |
|---|-----|----------|--------|
| G15 | No unified config migration strategy | MEDIUM | All three workstreams add fields to `Config` subtypes with `#[serde(default)]`. This works for forward-compatibility (old configs parse fine). But there is no config versioning, no `config migrate` command, and no documentation that explains to users what new fields are available and what they do. Each workstream writes its own docs but there is no unified config reference. |
| G16 | CI pipeline ownership is fragmented | LOW | Browser adds 2 CI jobs, UI adds 2-3, Voice needs 1. No single task coordinates these additions. Risk: conflicting edits to `pr-gates.yml`. |
| G17 | Test fixture `config.json` will need all three extensions | LOW | `tests/fixtures/config.json` is used by integration tests. All three workstreams add config fields. If tests run against a fixture that lacks a field, `#[serde(default)]` handles it, but tests that validate config completeness will need the fixture updated for each workstream. |
| G18 | No shared error handling strategy for new config sections | LOW | Voice adds `VoiceConfig` validation (model paths, threshold ranges). UI adds `api_port` validation. Browser adds `cors_proxy` URL validation. Each workstream validates independently but there is no unified config validation framework (Phase 4's Phase H covers tiered routing config validation but not other sections). |

---

## 5. Synergies

### 5.1 Browser + UI: WASM Embedding in React Dashboard

The UI's React frontend could load the Browser WASM module to enable a **serverless mode** where no Axum backend is needed. The architecture would be:

```
React Dashboard (ui/)
    |
    +-- Normal mode: REST/WS to Axum backend
    |
    +-- Browser-only mode: load clawft-wasm, route API calls to local WASM module
```

This is not planned by either workstream but is a natural future integration. The Browser WASM entry points (`init(config_json)`, `send_message(text)`) map directly to what the WebChat component needs.

**Effort to realize:** Medium. Requires a thin adapter layer in the React app that implements the same API interface as `api-client.ts` but routes to the WASM module instead of fetch.

### 5.2 Browser + Voice: Voice in Browser via Web Audio API + WASM

A future integration could bring voice capabilities to the browser by:
- Using Web Audio API for microphone capture (replacing cpal)
- Using onnxruntime-web for STT/TTS model inference (replacing sherpa-rs native)
- Running the voice pipeline in a Web Worker alongside the WASM agent loop

**Effort to realize:** High. Requires entirely new implementations of AudioCapture, AudioPlayback, STT, and TTS for the browser. Not feasible in the current timeline.

### 5.3 UI + Voice: Voice Controls in Dashboard

VS3.1 already plans this: VoiceStatusBar, TalkModeOverlay, AudioWaveform, VoiceSettings, PushToTalkButton. The synergy is already captured in the Voice plan.

### 5.4 All Three: Shared Config Schema

All three workstreams extend `clawft-types/src/config/mod.rs`. The shared benefit is a single config file (`config.json`) that controls all features. The risk is merge conflicts (addressed in Section 3).

### 5.5 All Three: Shared CI Pipeline

A single `pr-gates.yml` update could add gates for all three workstreams. This avoids three separate PRs that conflict on the CI file.

### 5.6 UI + Browser WASM: Static File Serving

UI S1.1 adds static file serving (`ui/dist/` via `tower-http::services::ServeDir`). Browser WASM P6.1 creates a minimal HTML test harness in `crates/clawft-wasm/www/`. These could share the static serving infrastructure, or the WASM test harness could be served from the UI's Vite dev server.

---

## 6. Recommended Execution Sequence

### 6.1 Phased Timeline

The goal is to minimize merge conflicts (serialize conflicting edits), maximize parallelism (independent work runs concurrently), deliver user value early, and tackle risky work first.

```
Week 1     Week 2     Week 3     Week 4     Week 5     Week 6     Week 7+
  |          |          |          |          |          |          |
  +-- CONFIG PATCH (all three config changes land together) ---------+
  |                                                                   |
  +== W-BROWSER P1 (Foundation) ====+                                 |
  |                                  |                                |
  +== W-UI S1.1 (Backend API) ======+== S1.3 (Core Views) ==+       |
  +== W-UI S1.2 (Frontend) ========+|                        |       |
  |                                  |                        |       |
  +== W-VOICE VS1.1 (Audio) =======+== VS1.2 (STT/TTS) ====+       |
  |                                                           |       |
  |          +== W-BROWSER P2 (Core Engine) ====+            |       |
  |          |                                   |            |       |
  |          |          +== W-BROWSER P3 (LLM) =+            |       |
  |          |          |                        |            |       |
  |          |          +== W-VOICE VS1.3 ======+            |       |
  |          |          | (needs S1.1 WS done)   |            |       |
  |          |          |                        |            |       |
  |          |          |          +== W-BROWSER P4 ==========+       |
  |          |          |          +== W-UI S2 ===============+       |
  |          |          |          +== W-VOICE VS2 ===========+       |
  |          |          |                                      |       |
  |          |          |                    +== W-BROWSER P5+P6 =====+
  |          |          |                    +== W-UI S3 =============+
  |          |          |                    +== W-VOICE VS3 =========+
```

### 6.2 Detailed Sequence with Conflict Avoidance

**Step 0: Config Unification (Before Week 1)**

Create a single preparatory PR that adds all new config types:
- `VoiceConfig` struct + `voice` field on root `Config` (from Voice plan)
- `api_port`, `cors_origins`, `api_enabled` fields on `GatewayConfig` (from UI plan)
- `browser_direct`, `cors_proxy` fields on `ProviderConfig` (from Browser plan)
- CI pipeline additions for all three workstreams

This eliminates the highest-risk merge conflicts by landing all shared-file changes in one commit.

**Step 1: Week 1-2 (Three Workstreams Start in Parallel)**

| Workstream | Phase | Can Run In Parallel? | Notes |
|------------|-------|---------------------|-------|
| W-BROWSER | P1: Foundation | YES | Touches `clawft-types/Cargo.toml`, `clawft-platform/` only |
| W-UI | S1.1 + S1.2 | YES | Touches `clawft-services/` + creates `ui/` directory |
| W-VOICE | VS1.1 | YES | Touches `clawft-plugin/Cargo.toml` + creates `voice/` module |

No file conflicts between these three. Full parallelism is safe.

**Step 2: Week 2-3 (Continue in Parallel)**

| Workstream | Phase | Dependencies | Notes |
|------------|-------|-------------|-------|
| W-BROWSER | P2: Core Engine | Needs P1 | Touches `clawft-core/Cargo.toml`, `loop_core.rs`, adds `runtime.rs` |
| W-UI | S1.3: Core Views | Needs S1.1 + S1.2 | All in `ui/` directory -- no Rust file conflicts |
| W-VOICE | VS1.2: STT + TTS | Needs VS1.1 | All in `clawft-plugin/src/voice/` + `clawft-tools/` voice tools |

No file conflicts. Full parallelism remains safe.

**Step 3: Week 3-4 (First Cross-Stream Dependency)**

| Workstream | Phase | Dependencies | Notes |
|------------|-------|-------------|-------|
| W-BROWSER | P3: LLM Transport | Needs P2 | Touches `clawft-llm/Cargo.toml` |
| W-VOICE | VS1.3: VoiceChannel + Talk Mode | Needs VS1.2 + **UI S1.1** (WS handler) | Voice channel sends `voice:status` events over WebSocket. If S1.1 is not done, voice can mock the WS layer or add events to the bus without the WS transport. |

**Risk:** VS1.3 depends on S1.1's WebSocket handler. If S1.1 is delayed:
- **Mitigation:** VoiceChannel can publish to `MessageBus` directly. The WS transport is a delivery mechanism, not a functional dependency. Voice works in CLI-only mode (Talk Mode outputs to terminal) until the WS handler is wired.

**Step 4: Week 4-6 (Three Tracks, Mostly Parallel)**

| Workstream | Phase | Dependencies | Notes |
|------------|-------|-------------|-------|
| W-BROWSER | P4: BrowserPlatform | Needs P3 | Creates `clawft-platform/src/browser/` (new files only) |
| W-UI | S2.1-S2.5: Canvas + Advanced | Needs S1.3 | Mix of `ui/` (React) and `clawft-types` (canvas.rs) + `clawft-services` (canvas WS) |
| W-VOICE | VS2.1-VS2.3: Wake Word + Platform | Needs VS1.3 | `clawft-plugin/src/voice/` + `scripts/` + `clawft-channels/` (Discord) |

**Conflict risk:** UI S2.1 adds `canvas.rs` to `clawft-types/src/`. Voice VS2 does not touch `clawft-types`. Browser P4 does not touch `clawft-types`. Low conflict.

**Step 5: Week 5-6+ (Final Phases)**

| Workstream | Phase | Dependencies | Notes |
|------------|-------|-------------|-------|
| W-BROWSER | P5 + P6: WASM Entry + Testing | Needs P4 | `clawft-wasm/`, `clawft-tools/` (file_tools.rs fix) |
| W-UI | S3.1-S3.5: Polish + Production | Needs S2 | `ui/` + `clawft-services/` |
| W-VOICE | VS3.1-VS3.3: UI Integration + Cloud + Advanced | Needs VS2 + **UI S1+S2** | `ui/src/components/voice/` + `clawft-plugin/` + `clawft-tools/` |

**Conflict risk for tools:** Browser P5.1 fixes `tokio::fs::metadata` in `clawft-tools/src/file_tools.rs`. Voice adds `voice_listen.rs` and `voice_speak.rs` as new files in `clawft-tools/`. No conflict on the same file, but the `clawft-tools/Cargo.toml` features section will be touched by both (Browser adds `browser` feature, Voice adds `voice` feature).

### 6.3 Value Delivery Priority

Ranked by "what gives users something usable earliest":

1. **W-UI S1 (Weeks 1-3):** A web dashboard with real-time agent monitoring is immediately useful for all existing users. Highest impact per effort.

2. **W-VOICE VS1 (Weeks 1-3):** Voice control of agents is a differentiated feature. Delivers `voice_listen` and `voice_speak` tools by Week 2, Talk Mode by Week 3.

3. **W-BROWSER P1-P3 (Weeks 1-4):** Foundation work with no user-visible outcome until P5-P6. Value delivery is back-loaded.

4. **W-UI S2 (Weeks 4-6):** Live Canvas is a novel capability, but builds on S1 which must land first.

5. **W-BROWSER P4-P6 (Weeks 4-6+):** Browser WASM is a major differentiator (run agents without installation) but requires all foundation phases to complete first.

6. **W-VOICE VS2-VS3 (Weeks 4-9):** Quality improvements and UI integration. Important for polish but not for initial value.

### 6.4 Risk-Adjusted Ordering

| Phase | Risk Level | Why |
|-------|-----------|-----|
| W-BROWSER P1-P2 | **HIGH** | Feature-gating existing code is the riskiest change (can break native builds). Should start early to catch regressions immediately. |
| W-VOICE VP (pre-validation) | **HIGH** | sherpa-rs model selection and cpal platform compatibility need validation before committing to the voice architecture. Must precede VS1. |
| W-UI S1.1 | **MEDIUM** | Axum API is new code (no regression risk), but thread-safety of wrapping core types in `Arc<dyn Trait>` needs careful design. |
| W-BROWSER P3 | **MEDIUM** | CORS handling in browser is a known pain point. Validate early with a real Anthropic API call. |
| W-VOICE VS1.3 | **MEDIUM** | VoiceChannel integrating with MessageBus is the first cross-system integration point. |
| W-UI S2.1 | **LOW** | Live Canvas is greenfield; few regression risks. |
| W-BROWSER P5 | **LOW** | Wiring the real AgentLoop is straightforward if P1-P4 are solid. |

---

## 7. Context Window Strategy

### 7.1 Information Hierarchy

Each SPARC execution should follow a "funnel" pattern: broad context narrows to implementation specifics.

```
Level 0: Cross-Plan Overview        (~2000 tokens)
    This document's Section 6.2 (Sequence) + dependency graph

Level 1: Workstream Summary         (~1500 tokens each)
    00-orchestrator.md for the workstream
    Provides: phase list, dependencies, exit criteria

Level 2: Phase Specification        (~4000-6000 tokens each)
    01-phase-*.md for the specific phase
    Provides: current code, deliverables, file-level task breakdown

Level 3: Implementation Context     (~2000-4000 tokens)
    The actual source files being modified
    Only the files listed in the phase spec
```

### 7.2 Self-Contained Phase Specs

Each phase spec should include (and already mostly does):

1. **"Current Code" section** -- Verbatim snippets of the structs/functions being modified. The implementer does not need to read the full source file to understand the starting point.

2. **"Deliverables" section** -- Complete code for new files, diffs for modified files. The implementer can work from the spec alone.

3. **"Phase Gate" section** -- Exact commands to verify the phase is complete. No ambiguity about what "done" means.

### 7.3 What Each Phase Needs to Read

| Phase | Docs Needed (max 2-3) | Source Files Needed |
|-------|----------------------|-------------------|
| W-BROWSER P1 | `00-consensus-plan.md` (architecture), `03-feature-flag-spec.md` (feature details) | `clawft-types/Cargo.toml`, `clawft-platform/Cargo.toml`, `clawft-platform/src/lib.rs` |
| W-BROWSER P2 | `05-task-breakdown.md` (P2 section), `03-feature-flag-spec.md` | `clawft-core/Cargo.toml`, `clawft-core/src/agent/loop_core.rs`, `clawft-core/src/agent/mod.rs` |
| W-BROWSER P3 | `00-consensus-plan.md` (CORS section), `05-task-breakdown.md` (P3 section) | `clawft-llm/Cargo.toml`, `clawft-types/src/config/mod.rs` (ProviderConfig) |
| W-BROWSER P4 | `05-task-breakdown.md` (P4 section) | `clawft-platform/src/lib.rs`, `clawft-platform/src/http.rs`, `clawft-platform/src/fs.rs` |
| W-BROWSER P5 | `05-task-breakdown.md` (P5 section) | `clawft-wasm/src/lib.rs`, `clawft-tools/src/file_tools.rs` |
| W-BROWSER P6 | `05-task-breakdown.md` (P6 section) | `clawft-wasm/src/lib.rs` (entry points) |
| W-UI S1.1 | `01-phase-S1-foundation-core-views.md` (Section 3.1) | `clawft-services/Cargo.toml`, `clawft-types/src/config/mod.rs` (GatewayConfig), `clawft-core/src/bootstrap.rs` |
| W-UI S1.2 | `01-phase-S1-foundation-core-views.md` (Section 3.2) | None (greenfield `ui/` directory) |
| W-UI S1.3 | `01-phase-S1-foundation-core-views.md` (Section 3.3) | `ui/src/` (S1.2 output) |
| W-UI S2.* | `02-phase-S2-canvas-advanced-views.md` | `clawft-types/src/lib.rs`, `clawft-services/src/api/` (S1.1 output) |
| W-VOICE VS1.1 | `01-phase-VS1-audio-foundation.md` (Sections 3 VS1.1.*) | `clawft-plugin/Cargo.toml`, `clawft-plugin/src/traits.rs`, `clawft-types/src/config/mod.rs` |
| W-VOICE VS1.2 | `01-phase-VS1-audio-foundation.md` (Sections 3 VS1.2.*) | `clawft-plugin/src/voice/` (VS1.1 output) |
| W-VOICE VS1.3 | `01-phase-VS1-audio-foundation.md` (Sections 3 VS1.3.*) | `clawft-plugin/src/voice/`, `clawft-services/src/api/ws.rs` (S1.1 output) |
| W-VOICE VS2.* | `02-phase-VS2-wake-word-platform.md` | `clawft-plugin/src/voice/` (VS1 output) |
| W-VOICE VS3.* | `03-phase-VS3-advanced-ui-integration.md` | `ui/src/` (S1+S2 output), `clawft-plugin/src/voice/` (VS2 output) |

### 7.4 Recommended Reading Order per Agent

When a SPARC agent starts a phase:

1. Read **this document** (Section 6.2 only) for sequencing context (~500 tokens)
2. Read the **phase spec** for the specific phase (~4000-6000 tokens)
3. Read the **source files** listed in the phase spec (~2000-4000 tokens)

Total context per agent: ~6500-10500 tokens. Well within a single message window.

---

## 8. Action Items

### Immediate (Before Implementation Starts)

| # | Action | Owner | Description |
|---|--------|-------|-------------|
| A1 | Unified config PR | Any workstream | Land all `config/mod.rs` additions (VoiceConfig, GatewayConfig extensions, ProviderConfig extensions) in one PR to eliminate merge conflicts |
| A2 | CI pipeline update | Any workstream | Add all three workstream CI gates to `pr-gates.yml` in one PR |
| A3 | Fix `ProviderConfig` naming | W-BROWSER | Update Browser plan code samples to use `api_base` instead of `base_url`, or add `#[serde(alias = "base_url")]` to the field |
| A4 | Voice VP validation | W-VOICE | Complete pre-implementation prototype to pin sherpa-rs version and fill in SHA-256 hashes before VS1.1 starts |

### During Implementation

| # | Action | Owner | Description |
|---|--------|-------|-------------|
| A5 | Config file split | W-VOICE | If VoiceConfig exceeds 150 lines, split into `config/voice.rs` sub-module to keep `config/mod.rs` under 500 lines |
| A6 | WS auth gap | W-UI | Add token validation to the WebSocket upgrade handler (missing from S1.1 spec) |
| A7 | Voice reqwest dep | W-VOICE | Add `reqwest` to `clawft-plugin` behind `voice` feature for ModelDownloadManager, or refactor to use Platform HttpClient |
| A8 | TypeScript type gen | W-UI | Evaluate `ts-rs` or `specta` for automated Rust-to-TypeScript type generation to prevent drift |

### Post-Implementation

| # | Action | Owner | Description |
|---|--------|-------|-------------|
| A9 | Config reference doc | All | Write unified config.json reference covering all new sections (voice, api, browser provider fields) |
| A10 | Browser+UI integration spike | W-BROWSER + W-UI | Prototype embedding WASM module in React dashboard for serverless mode |
| A11 | Voice browser feasibility | W-VOICE + W-BROWSER | Document whether voice-in-browser is technically feasible and at what effort level |

---

## 9. Summary

### Key Findings

1. **No circular dependencies.** All three workstreams can start in parallel in Week 1.

2. **One high-risk merge hotspot:** `clawft-types/src/config/mod.rs`. Mitigated by landing all config changes in a preparatory PR (Action A1).

3. **One hard cross-stream dependency:** Voice VS1.3 depends on UI S1.1's WebSocket handler. Mitigable by having Voice work in CLI-only mode until the WS transport lands.

4. **One deferred dependency:** Voice VS3.1 (UI Integration) depends on UI S1+S2 being complete. This is already sequenced correctly in both plans.

5. **Browser WASM is fully independent** of both UI and Voice at the implementation level. Integration synergies exist but are future work.

6. **Config migration needs a strategy.** All three workstreams add fields with `#[serde(default)]`, which is correct for backward compatibility, but no unified documentation or migration tooling is planned.

7. **CI pipeline changes should be batched** to avoid conflicting edits to `pr-gates.yml`.

### Risk Matrix

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| `config/mod.rs` merge conflicts | HIGH | LOW | Action A1: preparatory config PR |
| Voice VS1.3 blocked on UI S1.1 | MEDIUM | MEDIUM | Voice works CLI-only until WS handler lands |
| Browser P1 breaks native builds | MEDIUM | HIGH | Phase gate: `cargo test --workspace` after every change |
| sherpa-rs version incompatibility | MEDIUM | HIGH | Action A4: VP prototype validation |
| CI file conflicts | LOW | LOW | Action A2: batch CI updates |
| Config drift (no unified docs) | LOW | MEDIUM | Action A9: post-implementation config reference |
