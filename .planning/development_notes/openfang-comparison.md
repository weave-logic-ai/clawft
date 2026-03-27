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
