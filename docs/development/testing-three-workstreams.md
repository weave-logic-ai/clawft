# Testing Guide: Three-Workstream Implementation

Manual testing instructions for the three workstreams (Browser WASM, UI, Voice)
and the unified build script. Work through each section in order.

## Prerequisites

```bash
# Rust targets
rustup target add wasm32-wasip1 wasm32-unknown-unknown

# Node.js (for UI)
node --version   # >= 20

# Install UI dependencies
cd ui && npm install && cd ..
```

---

## 1. Unified Build Script (`scripts/build.sh`)

The build script wraps all workflows. Test every subcommand.

### 1.1 Help and argument parsing

```bash
# Should print usage with all 11 commands and 5 options
scripts/build.sh --help
scripts/build.sh -h

# No arguments should also print usage (exit 0)
scripts/build.sh

# Unknown command should print error + usage (exit 1)
scripts/build.sh foobar

# Unknown option should print error + usage (exit 1)
scripts/build.sh native --badopt
```

**Verify:** Color-coded output, correct exit codes, all commands and options listed.

### 1.2 Dry-run mode

Every subcommand should print the cargo command without executing.

```bash
scripts/build.sh native --dry-run
scripts/build.sh native-debug --dry-run
scripts/build.sh wasi --dry-run
scripts/build.sh browser --dry-run
scripts/build.sh ui --dry-run
scripts/build.sh all --dry-run
scripts/build.sh test --dry-run
scripts/build.sh check --dry-run
scripts/build.sh clippy --dry-run
scripts/build.sh gate --dry-run
scripts/build.sh clean --dry-run
```

**Verify:** Each prints the exact cargo invocation prefixed with `DRY`. No actual
compilation happens. Gate dry-run shows all 11 checks with `[1/11]` through
`[11/11]` numbering and a PASSED: 11 summary.

### 1.3 Feature passthrough

```bash
scripts/build.sh native --features voice --dry-run
scripts/build.sh native --features voice,channels --dry-run
scripts/build.sh browser --dry-run                # auto-sets --features browser
```

**Verify:** The `--features` flag appears in the dry-run output exactly as
specified.

### 1.4 Profile override

```bash
scripts/build.sh native --profile debug --dry-run
scripts/build.sh native --profile release-wasm --dry-run
scripts/build.sh wasi --profile release --dry-run
```

**Verify:** The `--profile` flag overrides the default for each subcommand.

### 1.5 Real builds

```bash
# Fast compile check (no codegen)
scripts/build.sh check

# Debug binary (fastest full build)
scripts/build.sh native-debug

# Release binary
scripts/build.sh native
```

**Verify:** Each succeeds. Timer output shown (`TIME Xs`). Size reported for
native binary (expect ~6 MB release, ~50+ MB debug). The `check` subcommand
should NOT report a size.

### 1.6 WASM builds

```bash
scripts/build.sh wasi
scripts/build.sh browser
```

**Verify:** Both succeed. WASI reports `.wasm` size. Browser may be smaller.
If a target is not installed, a WARN message appears with `rustup target add`
instructions.

### 1.7 UI build

```bash
scripts/build.sh ui
```

**Verify:** Runs `tsc -b && vite build` inside `ui/`. Reports bundle size
(`SIZE  UI bundle: XXK`). The `ui/dist/` directory is created.

### 1.8 All builds

```bash
scripts/build.sh all
```

**Verify:** Runs native, wasi, browser, ui in sequence. Reports individual
sizes. Ends with `PASS  All builds succeeded` (green). If any step fails, it
continues and reports `N build(s) failed` (red) at the end.

### 1.9 Test and clippy

```bash
scripts/build.sh test
scripts/build.sh clippy
```

**Verify:** `test` runs `cargo test --workspace` (expect 2,500+ tests).
`clippy` runs with `-D warnings` (zero warnings = pass).

### 1.10 Gate (full phase gate)

```bash
scripts/build.sh gate
```

**Verify:** Runs all 11 checks sequentially:
1. `cargo test --workspace` — PASS
2. `cargo build --release --bin weft` — PASS
3. WASI WASM (wasm32-wasip1) — PASS
4-9. Browser WASM per crate (types, platform, core, llm, tools, wasm) — PASS or SKIP
10. UI build — PASS
11. Voice feature — PASS or SKIP

Each check shows `[N/11]` label, PASS/FAIL/SKIP in color, and timing.
Summary at bottom shows totals. Exit code 0 if no FAIL.

### 1.11 Verbose mode

```bash
scripts/build.sh check --verbose
```

**Verify:** Full cargo output is shown (not truncated to 5 lines).

### 1.12 Clean

```bash
scripts/build.sh clean
```

**Verify:** Runs `cargo clean`. If `ui/dist` exists, removes it and prints
`INFO  Removing ui/dist`. Ends with `PASS  Clean complete`.

---

## 2. W-BROWSER: Browser WASM Workstream (BW1-BW6)

Tests that the entire crate stack compiles for `wasm32-unknown-unknown`.

### 2.1 Per-crate browser WASM compilation

Each crate should compile independently for the browser target:

```bash
cargo check --target wasm32-unknown-unknown -p clawft-types --no-default-features --features browser
cargo check --target wasm32-unknown-unknown -p clawft-platform --no-default-features --features browser
cargo check --target wasm32-unknown-unknown -p clawft-core --no-default-features --features browser
cargo check --target wasm32-unknown-unknown -p clawft-llm --no-default-features --features browser
cargo check --target wasm32-unknown-unknown -p clawft-tools --no-default-features --features browser
cargo check --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser
```

**Verify:** All 6 pass with zero errors. No native-only (tokio, dirs, notify)
symbols leak into the browser build.

### 2.2 Full browser WASM build

```bash
cargo build --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser --profile release-wasm
```

**Verify:** Produces `target/wasm32-unknown-unknown/release-wasm/clawft_wasm.wasm`.
Check the file exists and note its size.

### 2.3 Native still works after browser changes

```bash
cargo check --workspace
cargo test --workspace
```

**Verify:** The feature-gating changes (BW1-BW5) haven't broken native
compilation or tests. All 2,500+ tests pass.

### 2.4 Browser platform modules

```bash
cargo test -p clawft-platform
```

**Verify:** Platform tests pass. The browser modules
(`crates/clawft-platform/src/browser/`) define `BrowserHttpClient`,
`BrowserFileSystem`, `BrowserEnvironment` — these compile behind `#[cfg(target_arch = "wasm32")]`.

### 2.5 LLM browser transport

```bash
cargo test -p clawft-llm
```

**Verify:** Tests pass. `crates/clawft-llm/src/browser_transport.rs` provides
`BrowserLlmClient` with CORS proxy URL resolution and browser-specific headers.

### 2.6 WASM entry point

Inspect the WASM entry module:

```bash
cargo doc -p clawft-wasm --no-deps --open
```

Or just verify the file exists and exports `init`, `send_message`, `set_env`:

```bash
grep -n 'pub fn\|pub async fn' crates/clawft-wasm/src/lib.rs
```

**Verify:** The `browser_entry` module (behind `#[cfg(feature = "browser")]`)
exports wasm-bindgen-annotated functions.

### 2.7 Test harness (manual browser test)

```bash
# Serve the HTML test harness
python3 -m http.server 8080 --directory crates/clawft-wasm/www
```

Open `http://localhost:8080` in a browser.

**Verify:** The page loads a dark-themed chat interface. The WASM module
initializes (check browser console for init messages). You can type a message
and see stub responses. Status indicator reflects connection state.

---

## 3. W-UI: React Dashboard Workstream (S1-S3.7)

### 3.1 Build and type-check

```bash
cd ui
npm run build
```

**Verify:** `tsc -b` (type check) and `vite build` both succeed. Output shows
module count (~1,920) and bundle size (~452KB JS, ~128KB gzip). `ui/dist/` is
created.

### 3.2 Lint

```bash
cd ui
npm run lint
```

**Verify:** Zero lint errors.

### 3.3 Dev server

```bash
cd ui
npm run dev
```

Open `http://localhost:5173` in a browser.

**Verify:** The app loads. Navigation sidebar shows routes. Default route is
the Dashboard.

### 3.4 Dashboard route (`/`)

**Verify:**
- 4 stat cards visible (Agents, Sessions, Messages, Uptime or similar)
- Cards show mock/stub data
- Layout is responsive

### 3.5 Agents route (`/agents`)

**Verify:**
- Card grid of agents
- Start/stop controls visible on each card
- Search or filter if present

### 3.6 Chat route (`/chat`)

**Verify:**
- Message input area
- Send button
- Messages display in a scrollable container
- WebSocket indicator (may show disconnected without backend)

### 3.7 Sessions route (`/sessions`)

**Verify:**
- Table of sessions
- Expandable rows (click to see details)
- Export button if present

### 3.8 Tools route (`/tools`)

**Verify:**
- Grid or list view of registered tools
- Search/filter input
- Schema viewer (click a tool to see its JSON Schema)

### 3.9 Canvas route (`/canvas`)

**Verify:**
- Canvas area with elements
- 7+ element types supported (text, image, chart, code editor, form, etc.)
- **Undo/redo:** Make a change, press Ctrl+Z (undo), Ctrl+Shift+Z (redo)
- Chart element renders bar/line/pie via CSS/SVG
- Code editor element has line numbers and editable textarea
- Form element has validation (required fields, number ranges)

### 3.10 Skills route (`/skills`)

**Verify:**
- Card grid of skills
- Search bar
- Install/uninstall buttons

### 3.11 Memory route (`/memory`)

**Verify:**
- Data table of memory entries
- Namespace and tag filters
- Semantic search with threshold slider

### 3.12 Config route (`/config`)

**Verify:**
- Multi-tab config editor (5 tabs)
- Form fields for settings
- Diff viewer (if present)

### 3.13 Cron route (`/cron`)

**Verify:**
- CRUD for cron jobs
- Next-fire preview for each entry

### 3.14 Channels route (`/channels`)

**Verify:**
- Status cards for each channel type (Telegram, Slack, Discord)
- Real-time status indicators

### 3.15 Delegation route (`/delegation`)

**Verify:**
- 3 tabs: Active, Rules, History
- Delegation status visible

### 3.16 Monitoring route (`/monitoring`)

**Verify:**
- Summary cards (requests, errors, latency)
- Tables with monitoring data

### 3.17 Voice route (`/voice`)

**Verify:**
- Voice status bar (color-coded badge)
- Talk overlay (waveform animation area)
- Settings panel (config options)
- Push-to-talk button (circular)

### 3.18 Browser WASM mode detection

Open browser console and check:

```javascript
// Feature detection (from feature-detect.ts)
console.log(typeof WebAssembly !== 'undefined')   // true
```

**Verify:** The UI has a mode context (ModeProvider). In native mode, all
routes are visible. In WASM mode, channels/cron/delegation/monitoring routes
should be hidden from navigation.

### 3.19 Error boundary

**Verify:** The app is wrapped in an `ErrorBoundary` component. If a route
throws, the error is caught and a fallback UI is shown instead of a white
screen.

---

## 4. W-VOICE: Voice Pipeline Workstream (VS1-VS3.3)

### 4.1 Voice module compilation

```bash
cargo check --features voice -p clawft-plugin
```

**Verify:** Compiles cleanly. The voice module
(`crates/clawft-plugin/src/voice/`) contains: `mod.rs`, `capture.rs`,
`channel.rs`, `cloud_stt.rs`, `cloud_tts.rs`, `commands.rs`, `echo.rs`,
`fallback.rs`, `noise.rs`, `personality.rs`, `quality.rs`, `stt.rs`,
`transcript.rs`, `tts.rs`, `wake.rs`.

### 4.2 Voice plugin tests

```bash
cargo test -p clawft-plugin
```

**Verify:** All tests pass (~166 tests). Key test areas:
- STT/TTS stubs return expected results
- Echo canceller stub processes frames
- Noise suppressor tracks noise floor EMA
- Audio quality metrics (RMS, peak, clipping, SNR)
- Wake word detector stub
- Voice command registry with fuzzy matching (Levenshtein distance <= 2)
- Cloud STT/TTS provider trait definitions
- Fallback chains (local-first, cloud on error or low confidence < 0.60)
- Voice personality config
- Transcript logger format

### 4.3 Voice tools compilation

```bash
cargo test -p clawft-tools
```

**Verify:** All tests pass. Voice-related tools exist:
- `audio_transcribe.rs` — audio transcription tool
- `audio_synthesize.rs` — audio synthesis tool
- `voice_listen.rs` — voice listen tool
- `voice_speak.rs` — voice speak tool

### 4.4 Voice CLI commands

```bash
# Check that voice commands are registered
cargo run --bin weft -- voice --help
```

**Verify:** The `voice` subcommand exists with sub-subcommands like `talk`,
`wake`, `install-service`. If the voice feature isn't compiled in by default,
build with `--features voice`:

```bash
cargo run --features voice --bin weft -- voice --help
```

### 4.5 Voice config types

```bash
cargo test -p clawft-types
```

**Verify:** Tests pass (~224 tests). Voice config types exist in
`crates/clawft-types/src/config/voice.rs`:
- `VoiceConfig` — top-level voice settings
- `SttConfig`, `TtsConfig` — speech-to-text / text-to-speech config
- `WakeWordConfig` — wake word settings
- `VoicePersonality` — per-agent voice customization (voice_id, provider, speed, pitch)

### 4.6 Cloud providers

Test that cloud STT/TTS providers have proper trait definitions:

```bash
cargo test -p clawft-plugin -- cloud
```

**Verify:** Tests covering:
- `WhisperSttProvider` (OpenAI Whisper API)
- `OpenAiTtsProvider` (6 voices)
- `ElevenLabsTtsProvider` (4 voices)
- `SttFallbackChain` — local-first, cloud on confidence < 0.60 or error
- `TtsFallbackChain` — local-first, cloud on error

### 4.7 Audio quality metrics

```bash
cargo test -p clawft-plugin -- quality
```

**Verify:** Tests for RMS level, peak amplitude, clipping detection, SNR estimation.

### 4.8 Voice command fuzzy matching

```bash
cargo test -p clawft-plugin -- command
```

**Verify:** Voice command registry uses Levenshtein fuzzy matching with
distance <= 2 for trigger words.

### 4.9 Platform service files

```bash
# Check that service file templates exist
ls scripts/clawft-wake.service    # systemd
ls scripts/com.clawft.wake.plist  # launchd
```

**Verify:** Both files exist. The CLI `install-service` command detects the
platform and installs the appropriate file.

### 4.10 Test fixture config

```bash
cat tests/fixtures/config.json | python3 -m json.tool | grep -A5 voice
```

**Verify:** The test fixture includes voice configuration fields (stt, tts,
wake_word settings).

---

## 5. Cross-Cutting: Full Integration

### 5.1 Full phase gate

```bash
scripts/build.sh gate
```

**Verify:** 11/11 checks pass (or pass+skip). No FAIL.

### 5.2 Full test suite

```bash
cargo test --workspace 2>&1 | tail -20
```

**Verify:** 2,500+ tests, all passing.

### 5.3 Canvas types in Rust

```bash
cargo test -p clawft-types -- canvas
```

**Verify:** Canvas types (7+ element variants, 5 command variants, 3
interaction variants) all have tests (~216 canvas type tests).

### 5.4 Feature isolation

Verify features don't leak across boundaries:

```bash
# Native without voice — should compile
cargo check -p clawft-plugin

# Browser without native — should compile
cargo check --target wasm32-unknown-unknown -p clawft-wasm --no-default-features --features browser
```

**Verify:** Both succeed, confirming feature gates are correct.

### 5.5 Docs build

Verify all new documentation files exist and are well-formed:

```bash
ls docs/browser/         # 5 files: building, quickstart, api-reference, architecture, deployment
ls docs/ui/              # 4 files: developer-guide, api-reference, browser-mode, deployment
```

**Verify:** Files exist and contain meaningful content (not stubs).
