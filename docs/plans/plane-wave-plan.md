# Plane Dependency DAG & Wave Plan

> Generated: 2026-07-31 03:28 UTC
> Full inventory: [`plane-board-inventory.md`](./plane-board-inventory.md)
> DAG data: [`plane-dag.json`](./plane-dag.json)
> Skill: `.grok/skills/plane-dag/SKILL.md`
> Helper: `scripts/plane-dag.sh`

## Graph model

```
Node  = Plane work item (WEFT-N)
Edge  = blocked_by (A → B means B waits for A Done)
Wave  = open nodes whose open blockers are empty
Lane  = parallel track inside a wave (A–J)
```

### Inferred edges

| From | To | Reason |
|------|----|--------|
| WEFT-519 | WEFT-520 | LeWM invariants before core |
| WEFT-543 | WEFT-520 | dim decision before core types |
| WEFT-520 | WEFT-521 | core traits before impls |
| WEFT-521 | WEFT-522 | impls before facade |
| WEFT-522 | WEFT-523 | facade before sensor |
| WEFT-522 | WEFT-524 | facade before service |
| WEFT-522 | WEFT-525 | facade before delegation |
| WEFT-523 | WEFT-526 | sensor crates before mesh topics |
| WEFT-522 | WEFT-527 | facade before LatticeApi |
| WEFT-527 | WEFT-528 | Lattice before SIGReg |
| WEFT-527 | WEFT-529 | Lattice before planner |
| WEFT-529 | WEFT-530 | planner before rollback |
| WEFT-528 | WEFT-530 | SIGReg before rollback |
| WEFT-522 | WEFT-533 | facade before attestation |
| WEFT-115 | WEFT-105 | protocol types before chain replay |
| WEFT-115 | WEFT-106 | protocol types before merkle |
| WEFT-113 | WEFT-107 | clock before key rotation |
| WEFT-112 | WEFT-105 | test harness before chain replay |
| WEFT-109 | WEFT-105 | merge decision before replay |
| WEFT-144 | WEFT-106 | signing before merkle mutations |
| WEFT-131 | WEFT-554 | DelegationCert before full ACL |
| WEFT-628 | WEFT-615 | Phase1 ERL before barge-in |
| WEFT-615 | WEFT-638 | barge-in path before cutover |

## Wave overview

| Wave | Tickets | High/Urgent | In 0.8.x |
|-----:|--------:|------------:|---------:|
| 0 | 230 | 18 | 142 |
| 1 | 9 | 6 | 2 |
| 2 | 2 | 1 | 0 |
| 3 | 1 | 1 | 0 |
| 4 | 5 | 5 | 0 |
| 5 | 3 | 3 | 0 |
| 6 | 1 | 1 | 0 |

## Critical paths

- **len 7**: WEFT-543 → WEFT-520 → WEFT-521 → WEFT-522 → WEFT-527 → WEFT-529 → WEFT-530
- **len 7**: WEFT-519 → WEFT-520 → WEFT-521 → WEFT-522 → WEFT-527 → WEFT-529 → WEFT-530
- **len 3**: WEFT-628 → WEFT-615 → WEFT-638
- **len 2**: WEFT-688 → WEFT-695
- **len 2**: WEFT-686 → WEFT-695
- **len 2**: WEFT-561 → WEFT-575
- **len 2**: WEFT-144 → WEFT-106
- **len 2**: WEFT-131 → WEFT-554
- **len 2**: WEFT-115 → WEFT-106
- **len 2**: WEFT-113 → WEFT-107
- **len 2**: WEFT-112 → WEFT-105
- **len 2**: WEFT-109 → WEFT-105

## Lanes

| Lane | Focus |
|------|-------|
| A | Release/CI (ws14) |
| B | Kernel/Mesh (ws02, ws13) |
| C | Memory/Vector (ws06, ws12) |
| D | Agent/Hermes (ws11, ws07) |
| E | Voice (ws10) |
| F | UI/Surface (ws08, ws09, ws18) |
| G | WASM/Browser (ws16) |
| H | Channels (ws05) |
| I | Research/LeWM (ws17) |
| J | Tooling/Plane (ws15, tests) |

## Wave 0 — ready now (0.8.x high/medium first)

- **WEFT-15** [LB/low/0.9.x] 🔧WIP — ws01: kernel-config — wire LogQuantizedStubConfig + SimdDistanceStubConfig runtime
- **WEFT-11** [LB/medium/0.8.x] — ws01: rpc — implement Windows daemon transport (named pipes) for DaemonClient
- **WEFT-13** [LG/medium/0.8.x] — ws01: platform — implement OPFS-backed BrowserFileSystem persistence
- **WEFT-135** [LB/medium/0.8.x] — ws02: workspace — clean ~150 clippy errors (pre-existing debt)
- **WEFT-170** [LH/medium/0.8.x] — ws05: PluginHost C7 unification — migrate Telegram/Discord/Slack to ChannelAdapter
- **WEFT-217** [LE/medium/0.8.x] — ws10: EchoCanceller and NoiseSuppressor — replace deceptive passthroughs with real DSP
- **WEFT-413** [LB/medium/0.8.x] — ws13: clawft-app — wire ADR-015 rule 6 once clawft-adapter exists
- **WEFT-598** [LF/medium/0.8.x] — ws09: Dependabot — triage 142 npm-side vulnerabilities (5 critical/41 high)
- **WEFT-613** [LE/medium/0.8.x] ⚠️weak-spec — Voicelab parity: Chatterbox cloned-voice fast tier (native port)
- **WEFT-644** [LE/medium/0.8.x] ⚠️weak-spec — SileroVoiceness: neural VAD behind the Voiceness trait (model staging + stateful ONNX + fallback)
- **WEFT-14** [LG/low/0.8.x] — ws01: platform — land OPFS-or-equivalent BrowserEnvironment persistence
- **WEFT-48** [LB/low/0.8.x] — ws03: rate-limiter — expose rate-limiter metrics via admin endpoint (Element-09)
- **WEFT-49** [LB/low/0.8.x] — ws03: rate-limiter — expose rate-limiter LRU maintenance via admin endpoint (Element-09)
- **WEFT-54** [LB/low/0.8.x] — ws03: pipeline — review FitnessScorer.error_indicators allowlist (localization, jailbreak)
- **WEFT-55** [LB/low/0.8.x] — ws03: pipeline — verify experimental-attention CI build/test wiring
- **WEFT-85** [LC/low/0.8.x] — ws06: substrate — emit chain_event! for session.append on every appended turn (MW-7)
- **WEFT-87** [LC/low/0.8.x] — ws06: sessions — ship weft session gc (or self-cleanup migration path) (MW-9)
- **WEFT-95** [LC/low/0.8.x] — ws06: identity — route IdentityLoader::current through Platform::fs() (MW-17)
- **WEFT-96** [LC/low/0.8.x] — ws06: identity — define journal substrate read-on-every-turn path (WS-D1)
- **WEFT-97** [LC/low/0.8.x] — ws06: identity — substrate-backed Identity::source variant set (WS-D4 / WS-D5)
- **WEFT-123** [LB/low/0.8.x] — ws02: services-api — add HTTP facade integration tests once profile/pairing types land
- **WEFT-125** [LB/low/0.8.x] — ws02: vector — add ecc.vector-config RPC endpoint
- **WEFT-129** [LA/low/0.8.x] — ws02: kernel — ship real Wasmtime backend for spectral_embedding (or move to deferred)
- **WEFT-152** [LB/low/0.8.x] — ws02: tests — confirm cognitum-gate-tilezero Permit/Defer/Deny path is exercised
- **WEFT-153** [LB/low/0.8.x] — ws02: chain — add EVENT_KIND_* constants for minor non-kernel chain gaps
- **WEFT-174** [LH/low/0.8.x] — ws05: Slack — add unknown_envelope counter for API drift detection
- **WEFT-175** [LH/low/0.8.x] — ws05: iMessage scope — implement AppleScript bridge or formally drop from tracker
- **WEFT-176** [LH/low/0.8.x] — ws05: WeftOS white-label — add brand() accessor and remove hard-coded clawft strings
- **WEFT-193** [LD/low/0.8.x] — ws07: IDE provider — replace IdeToolProvider::stub() with real implementation
- **WEFT-195** [LD/low/0.8.x] — ws07: delegate_tool — drop hardcoded claude_available=true, query the delegator for liveness
- **WEFT-196** [LD/low/0.8.x] — ws07: weft delegate — add debug subcommand to surface routing decisions
- **WEFT-197** [LD/low/0.8.x] — ws07: weft doctor — add multi-agent checks (claude on PATH, auto-delegation, ≥1 route)
- **WEFT-199** [LB/low/0.8.x] — ws07: SwarmCoordinator topology — implement mesh/hierarchical/adaptive or document as prompt-only
- **WEFT-200** [LD/low/0.8.x] — ws07: notifications/tools/list_changed — handle inbound and advertise outbound
- **WEFT-201** [LD/low/0.8.x] — ws07: Auto-delegation classifier — improve regex+keyword accuracy or document fragility (3H MIN-02)
- **WEFT-220** [LE/low/0.8.x] — ws10: Windows install-service — automate schtasks or document manual route as final
- **WEFT-224** [LE/low/0.8.x] — ws10: SC-3 cloud-fallback transparency log line
- **WEFT-225** [LE/low/0.8.x] — ws10: SC-6 anti-replay nonce and transcription-echo confirmation
- **WEFT-226** [LE/low/0.8.x] — ws10: SC-8 voice rate limiting (commands/min, wake/min, post-fail cooldown)
- **WEFT-227** [LE/low/0.8.x] — ws10: Speaker diarization via sherpa-rs
- **WEFT-228** [LE/low/0.8.x] — ws10: Tauri-side native mic capture — replace browser-only getUserMedia path
- **WEFT-229** [LE/low/0.8.x] — ws10: Latency + WER + CPU benchmarks for voice pipeline
- **WEFT-230** [LE/low/0.8.x] — ws10: Adaptive silence timeout learning
- **WEFT-231** [LE/low/0.8.x] — ws10: UI partial-transcription streaming and TTS word highlighting
- **WEFT-232** [LE/low/0.8.x] — ws10: Discord voice bridge — clawft-channels voice → STT → agent → TTS → VC audio
- **WEFT-233** [LE/low/0.8.x] — ws10: audio_transcribe / audio_synthesize tools — real WAV/MP3/OGG/WebM codec support
- **WEFT-234** [LE/low/0.8.x] — ws10: Cleanup orphan voice surfaces (events, statuses, voice-chat.ts, model_path types)
- **WEFT-236** [LE/low/0.8.x] — ws10: clawft-service-whisper — drop legacy dual-publish path post Phase-4 migration
- **WEFT-239** [LE/low/0.8.x] — ws10: CloudFallbackConfig — config-string to provider router
- **WEFT-240** [LE/low/0.8.x] — ws10: WakeConfig.sensitivity vs WakeWordConfig.threshold — unify the knob
- **WEFT-263** [LF/low/0.8.x] — ws08: terminal panel — multi-tab terminal (HashMap<SessionId, Terminal>)
- **WEFT-264** [LF/low/0.8.x] — ws08: terminal panel — real WASM terminal renderer
- **WEFT-275** [LF/low/0.8.x] — ws08: explorer — Lineage Object Type + viewer (metadata convention sign-off)
- **WEFT-281** [LF/low/0.8.x] — ws08: graph viewer — editable Phase 3+ patch UI (egui_node_graph migration)
- **WEFT-282** [LF/low/0.8.x] — ws08: vscode panel — capture sidecar (mic/camera) for vscode#303293
- **WEFT-283** [LF/low/0.8.x] — ws08: vscode panel — typed active-radar return schema (variant-id echo)
- **WEFT-284** [LF/low/0.8.x] — ws08: vscode panel — ThreadDock primitive for per-agent parallel output
- **WEFT-285** [LF/low/0.8.x] — ws08: vscode panel — WSP-0.1 verb support (raw RPC only today)
- **WEFT-326** [LD/low/0.8.x] — ws11: agent-core-v1.1 — stabilize append_turns_are_monotonic flake via injectable clock
- **WEFT-327** [LD/low/0.8.x] — ws11: agent-core-v1.1 — promote overlay_probe + resolver_live_probe diagnostics into CI
- **WEFT-329** [LD/low/0.8.x] — ws11: agent-core-v1.1 — notify-driven hot-reload watcher for identity files
- **WEFT-332** [LD/low/0.8.x] — ws11: agent-core-v1.1 — per-user agent_ids for multi-tenant chat
- **WEFT-333** [LD/low/0.8.x] — ws11: agent-core-v1.1 — register agent.chat SystemService for weft status
- **WEFT-334** [LD/low/0.8.x] — ws11: agent-core-v1.1 — typed error variants for agent.chat
- **WEFT-336** [LD/low/0.8.x] — ws11: agent-core-v1.1 — weft routing trace + replay commands
- **WEFT-341** [LD/low/0.8.x] — ws11: agent-core-v1.1 — per-tool Permit token + proof-of-permission API
- **WEFT-343** [LD/low/0.8.x] — ws11: agent-core-v1.1 — Arc<RwLock<LlmClient>> runtime swap on env rotation
- **WEFT-348** [LD/low/0.8.x] — ws11: agent-core — Phase 4 skills auto-promotion from .claude/skills to .clawft/skills
- **WEFT-349** [LD/low/0.8.x] — ws11: agent-core — cross-agent delegation via existing delegate_tool
- **WEFT-350** [LD/low/0.8.x] — ws11: agent-core — Phase 2 voice + streaming chat path
- **WEFT-354** [LC/low/0.8.x] — ws12: KG-013 — spatio-temporal GNN for sonobuoy (K-STEMIT)
- **WEFT-355** [LC/low/0.8.x] — ws12: KG-015 — EA-Agent entity alignment for multi-repo dedup
- **WEFT-356** [LC/low/0.8.x] — ws12: KG-017 — knowledge distillation for edge EML (SevenNet-Nano)
- **WEFT-358** [LC/low/0.8.x] — ws12: OG-2 — OWL/RDF ingestion (Turtle, JSON-LD)
- **WEFT-359** [LC/low/0.8.x] — ws12: OG-3 — Barnes-Hut force layout + positioned-SVG export
- **WEFT-360** [LC/low/0.8.x] — ws12: OG-4 — VOWL visual encoding rules in SVG export
- **WEFT-361** [LC/low/0.8.x] — ws12: KG-004 — benchmark RFF vs Lanczos vs EML lambda₂ on 1K/10K/100K graphs
- **WEFT-362** [LC/low/0.8.x] — ws12: layout — implement Sugiyama layered layout (currently falls back to tree)
- **WEFT-364** [LC/low/0.8.x] — ws12: vector — ecc.vector-config RPC to show active backend
- **WEFT-368** [LC/low/0.8.x] — ws12: ingest — replace StubHttpClient with real reqwest-based HTTP client
- _…plus 150 more ready items (see inventory)_

## Commands

```bash
scripts/plane-dag.sh refresh
scripts/plane-dag.sh ready --cycle 0.8.x --priority high
scripts/plane-dag.sh show WEFT-593
scripts/plane-dag.sh claim WEFT-593
scripts/plane-dag.sh done WEFT-593 --shipped '...' --commits abc123 --tests 'scripts/build.sh test' --build 'scripts/build.sh check'
```

## Lifecycle

| Event | Action |
|-------|--------|
| Claim | `plane-dag.sh claim WEFT-N` |
| Note | `plane-dag.sh note WEFT-N "..."` |
| Done | `plane-dag.sh done WEFT-N --shipped ... --commits ...` then `refresh` |
| Defer | `plane.sh defer <uuid> 0.9.x --reason "..."` |

