# OpenFang Comparison Analysis

> Internal development reference -- not for public distribution.
> Last updated: 2026-02-28

## Overview

OpenFang is an open-source "Agent Operating System" by RightNow AI (released Feb 24, 2026). Single ~32MB Rust binary. 5,592 GitHub stars in 4 days. 137K lines across 14 crates. MIT + Apache-2.0.

Repository: https://github.com/RightNow-AI/openfang

Both projects share Rust + Axum foundations but solve different problems: OpenFang is an agent OS for autonomous agents; clawft is a conversational AI framework optimized for intelligent routing and voice.

---

## Architecture Comparison

| Dimension | OpenFang | clawft/weft |
|-----------|----------|-------------|
| Language | Rust | Rust |
| HTTP | Axum | Axum |
| Frontend | Vanilla JS + Alpine.js (embedded) | React + TypeScript + Vite (SPA) |
| Desktop | Tauri 2.0 | None |
| Database | SQLite | File-based + HNSW vector |
| Config | TOML | JSON |
| WASM | Wasmtime sandbox for tools | wasm-bindgen browser target |
| Architecture | Kernel/OS model with boot sequence | 6-stage pluggable pipeline |

OpenFang uses a kernel architecture (18-step boot, agent supervisor, scheduler, RBAC). We use a pipeline architecture (TaskClassifier -> ModelRouter -> ContextAssembler -> LlmTransport -> QualityScorer -> LearningBackend).

---

## Where They Are Ahead

### Channel Breadth

40 adapters vs our ~13. They cover LINE, Viber, Facebook Messenger, Mastodon, Bluesky, Reddit, LinkedIn, Twitch, XMPP, Guilded, Revolt, and more.

### Autonomous Agents ("Hands")

Pre-built autonomous packages that run on schedules without prompts. HAND.toml manifests with multi-phase system prompts and SKILL.md domain expertise. 7 pre-built: Clip, Lead, Collector, Predictor, Researcher, Twitter, Browser. Our agents are reactive only.

### Desktop Application

Tauri 2.0 native app with system tray, keyboard shortcuts, auto-update.

### Security Layers

16 runtime layers including WASM sandboxing for tools, Merkle hash-chain audit trails, Ed25519 signed manifests, taint tracking, prompt injection scanner.

### Other Advantages

- OpenAI-compatible `/v1/chat/completions` drop-in API
- P2P wire protocol (OFP) with HMAC-SHA256 mutual auth
- Agent marketplace (FangHub)
- JavaScript and Python SDKs
- TUI dashboard (ratatui, 20+ screens)
- Migration tools from LangChain, AutoGPT, OpenClaw
- Workflow engine with 5 execution modes
- Session repair (7-phase validation)

---

## Where We Are Ahead

### Voice Pipeline

Our 21-file voice system is a generation ahead. Full STT, TTS (OpenAI + ElevenLabs + browser), VAD, wake word, echo cancellation, noise suppression, audio quality analysis, talk mode. They have a basic TTS tool only.

### WASM Browser Target

We compile the entire agent to run in-browser. They only use WASM for tool isolation.

### 6-Stage Pluggable Pipeline

Trait-based stages allow swapping implementations. More sophisticated than their monolithic agent loop.

### Quality + Learning Loop

Post-response quality scoring feeds back into routing. Closed-loop learning system.

### React Frontend

Modern SPA with TypeScript, Zustand stores, proper routing. Their vanilla JS is functional but less maintainable.

### Live Canvas

Real-time collaboration with typed elements (text, button, input, table, code, chart, form). No equivalent in OpenFang.

### Tiered Model Routing

3-tier routing with WASM fast path (sub-1ms). Cost/latency optimized.

### Plugin Architecture

Feature-flagged crates for lean builds. They compile everything monolithically.

### Budget Management

Per-sender daily/monthly cost limits with pre-reservation and reconciliation.

---

## Patterns Worth Adopting

### 1. Session Repair (High Priority)

7-phase validation: fixes orphaned ToolResult messages, removes empty messages, merges consecutive same-role messages. Prevents corruption over long sessions.

### 2. Canonical Sessions (High Priority)

Cross-channel context sharing via compaction summaries. If a user talks via Telegram then Discord, context follows them.

### 3. Loop Guard (Medium Priority)

SHA256-based tool call repetition detection with warn/block/circuit-break thresholds. Defense against infinite loops.

### 4. Text-to-Tool Recovery (Medium Priority)

Detects when models emit tool calls as plain text and converts to proper tool structures.

### 5. Stability Guidelines (Low Priority)

Appending behavioral rules to every system prompt to prevent degenerate LLM patterns.

### 6. Cost-Weighted Rate Limiting (Low Priority)

GCRA token bucket where weight = LLM cost, not request count.

### 7. HAND.toml Manifests (Consider)

Clean packaging format for autonomous agents. Could adapt for our skills system.

### 8. OpenAI-Compatible API (Consider)

Drop-in `/v1/chat/completions` would enable existing tooling integration.

---

## Summary Matrix

| Dimension | OpenFang | clawft | Winner |
|-----------|----------|--------|--------|
| Channel breadth | 40 | ~13 | OpenFang |
| Voice pipeline | Basic TTS | Full duplex | **clawft** |
| WASM deployment | Tool sandbox | Browser target | **clawft** |
| LLM routing | Complexity + fallback | 6-stage + 3-tier + learning | **clawft** |
| Autonomous agents | Hands (7 pre-built) | Reactive only | OpenFang |
| Security (runtime) | 16 layers | Audit checks | OpenFang |
| Desktop app | Tauri 2.0 | None | OpenFang |
| Frontend quality | Vanilla JS | React + TS | **clawft** |
| Live canvas | None | Full impl | **clawft** |
| Plugin granularity | Monolithic | Feature flags | **clawft** |
| Quality + learning | None | Full loop | **clawft** |
| P2P networking | OFP protocol | None | OpenFang |
| Marketplace | FangHub | None | OpenFang |
| SDKs | JS + Python | None | OpenFang |

---

## Conclusion

Different products for overlapping use cases. OpenFang optimizes for breadth (channels, autonomous agents, marketplace). We optimize for depth (routing intelligence, voice, quality, browser execution). The most impactful things to adopt: session repair, canonical sessions, and broader channel coverage.

---

## Gap triage (WEFT-549, 2026-07-31)

Competitive-analysis items only — **not** research deliverables. Each gap is
owned, deferred (`no-op` / later cycle), or already partially covered in-tree.
New implementation work must open a **new** Plane item under the owning
workstream; this section is the triage outcome that closes WEFT-549.

| # | Gap target | Owner workstream | Disposition | Notes / follow-up |
|---|------------|------------------|-------------|-------------------|
| 1 | Channel breadth (40 vs ~13) | **ws06-channels** | adopt selectively | Prioritize high-value nets (Bluesky/X/Matrix depth) over LINE/Viber parity; file per-adapter Plane items when scheduled |
| 2 | Autonomous Hands agents | **ws11-agent-core** | adopt pattern | HAND.toml-like packaging maps to skills/agents manifests; no 1:1 OpenFang Hands port |
| 3 | Tauri 2.0 desktop | **ws09-gui** | defer / no-op for 0.8–0.9 | Browser WASM + clawft-ui SPA is current desktop story; Tauri only if product demands native shell |
| 4 | 16-layer security stack | **ws08-security** | partial / map | Keep WeftOS model (governance gates, ExoChain, wasm-sandbox, rate limits); do not clone OpenFang layer list — track deltas via security audits |
| 5 | OpenAI-compatible API | **ws04-api** / services | adopt | `/v1/chat/completions` façade is valuable for tooling; schedule under services HTTP when capacity |
| 6 | P2P OFP wire protocol | **ws07-mesh** | no-op as OFP | Prefer existing mesh (ADR-026/031) over OpenFang OFP clone |
| 7 | Agent marketplace (FangHub) | product / **ws14-deployment** | defer | Marketplace is product/ops, not core runtime; revisit post-1.0 |
| 8 | JS + Python SDKs | **ws15-mcp** + docs | partial | MCP is the external control plane (ADR-075/076); thin language SDKs optional later |
| 9 | ratatui TUI | **ws09-gui** / console | partial / no-op new TUI | Kernel console + weaver already cover ops CLI; full ratatui dashboard not planned |
| 10 | Migration tools (LangChain/AutoGPT/…) | docs / **ws14-deployment** | docs-only | Migration guides in docs when demanded; no in-tree converter priority |
| 11 | Session repair / canonical sessions | **ws11-agent-core** | adopt | Highest-value patterns from “Patterns Worth Adopting”; open dedicated tickets when implementing |
| 12 | Loop guard / text-to-tool recovery | **ws11-agent-core** | adopt | Medium priority hardening; group with agent-loop reliability |

**Rule:** do not leave orphans under ws17-research — competitive gaps belong to
product workstreams above.
