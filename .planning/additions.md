**ClawFT (weft) Roadmap → "OpenClaw-like" Rust-native AI Agent Platform (vNext / "IronClaw" or "ClawFT Open")**

ClawFT is already an excellent foundation: a **modular, async, high-performance Rust framework** with:
- Multi-channel messaging (Telegram, Slack, Discord) via `PluginHost` adapters.
- OpenAI-compatible LLM abstraction + native providers.
- 6-stage pipeline (Classifier → Router → Assembler → Transport → Scorer → Learner).
- Built-in tools (sandboxed file ops, shell exec, memory, web search/fetch, message dispatch, agent spawning).
- MCP (Model Context Protocol) server/client support (perfect for VS Code/Copilot, Claude Desktop, etc.).
- Session persistence (JSONL + consolidation/summaries), optional vector memory, CronService, WASM crate, tiny binaries, strong sandboxing.

**OpenClaw** (the viral TS/Node project at openclaw/openclaw) is the target reference: local-first personal AI assistant with:
- Broad messaging (WhatsApp, Signal, iMessage, etc.).
- Declarative + executable **skills** (`SKILL.md` with YAML frontmatter + natural-language instructions; community ClawHub registry; agent can auto-create skills).
- Browser CDP automation, Gmail/email, calendar, file/shell ops.
- Proactive behavior (heartbeat/cron).
- Local Markdown-based memory/workspace.
- Personality via `SOUL.md`/`AGENTS.md`.
- Docker/WASM-like sandboxing, runs everywhere.

**Vision**: Turn ClawFT into the **performant, safe, production-grade Rust successor** (or "IronClaw" variant) to OpenClaw. Emphasize:
- **Plugins/Skills** as first-class (WASM-first for security/portability, with SKILL.md compatibility layer).
- Native tools for **software development** (git, cargo, testing, IDE integration via MCP).
- **Apps & automations** (browser, email, calendar, etc.).
- **Voice** (STT + TTS, local-first where possible).
- Everything extensible without core forks — agents can create/load skills dynamically.

**Why Rust wins here**: Smaller/faster binaries, memory safety, true async, easy WASM sandboxing, no Python/TS runtime bloat. Target: <10 MB binary, sub-100 ms cold starts, full offline capability.

### Phase 0: Foundation & Plugin System (4–6 weeks, Priority: Critical)
**Goal**: Make **everything** a plugin/skill so no more core modifications needed for new capabilities.

Tasks:
1. Define `clawft-plugin` trait crate (Tool, ChannelAdapter, PipelineStage, Skill, MemoryBackend, VoiceHandler).
2. Implement **WASM plugin host** (using `wasmtime` + `wit` component model for typed interfaces). Plugins = `.wasm` + manifest (JSON/YAML) + optional `SKILL.md` for LLM guidance.
3. Add **Skill Loader** compatible with OpenClaw: parse `SKILL.md` (YAML frontmatter → tool description + execution hints), auto-register as WASM or native wrapper. Support `ClawHub` discovery (simple HTTP index + git clone).
4. Dynamic loading at runtime (hot-reload for skills, with sandbox isolation).
5. Update `PluginHost` to unify channels + tools + new types. Add `SOUL.md`/`AGENTS.md` personality injection into pipeline (Learner/Assembler stages).
6. Extend MCP server to expose loaded skills/tools automatically.

**Milestone**: `weft skill install github.com/openclaw/skills/coding-agent` works; agent can `weft skill create "new skill for X"` and it compiles to WASM.

**Output**: New crates: `clawft-plugin`, `clawft-skill`, updated `clawft-tools` & `clawft-channels`.

### Phase 1: Communication & Voice Parity (3–5 weeks)
**Goal**: Match/exceed OpenClaw channels + add voice.

Tasks:
- **Email**: New channel/tool plugin (IMAP + SMTP via `lettre` + `imap` crates; Gmail OAuth2 via `google-apis-rs` or `oauth2`). Full read/reply/attach, proactive inbox triage via cron.
- **Extra messaging**: WhatsApp (via `whatsapp-web.js` bridge or official Cloud API wrapper), Signal/iMessage (via `signal-cli` or macOS bridge), generic Matrix/IRC.
- **Voice (TTS/STT)**: 
  - STT: `whisper-rs` (local, ONNX) or `vosk-api`.
  - TTS: `piper-rs` (local, high-quality) or `tts` crate + ElevenLabs fallback.
  - New `VoiceChannel` or tool: audio → STT → pipeline → TTS response. Support microphone/file input, streaming.
- Proactive heartbeat (enhance existing CronService with "check-in" mode).

**Milestone**: Voice-enabled Telegram/Slack bot; agent clears Gmail inbox via voice command or scheduled task.

### Phase 2: Software Development & App Ecosystem (6–8 weeks)
**Goal**: Make it a true **software engineering co-pilot** + app automator.

Tasks:
- **Dev Tools Plugin Pack** (as WASM or native):
  - Git (via `git2` crate: clone, commit, PR, branch).
  - Cargo integration (build, test, clippy, publish).
  - Code editing/analysis (tree-sitter for parsing, LSP client).
  - Debugger/REPL hooks.
  - MCP deep integration: expose as VS Code extension backend (agent edits code live in IDE).
- **Browser Automation**: New `browser-cdp` tool (using `chromiumoxide` – async Rust CDP client, like Puppeteer). Headless/full control, screenshot, form fill, scraping. Sandboxed via separate process.
- **App Integrations** (as plugins):
  - Calendar (Google/Outlook/iCal via APIs).
  - Files/Drive, Notion, Slack (deeper), GitHub (already partial via skills).
  - Generic REST + OAuth2 helper plugin.
- Shell & file ops: already strong; add Docker/Podman orchestration, secure workspace (`~/.clawft/workspace` like OpenClaw).

**Milestone**: Agent can "fork repo, implement feature X, test, PR" end-to-end; browser automates flight check-in; full `coding-agent` skill from OpenClaw ported.

### Phase 3: Memory, Polish, Scale & Community (4–6 weeks + ongoing)
- **Memory**: Add Markdown workspace (`~/.clawft/workspace` with SKILL.md, SOUL.md, USER.md, conversation logs) alongside JSONL/vector. Auto-summarization + long-term vector store.
- **UI/Deployment**: Web dashboard (`weft ui` via Axum + Leptos), Docker images (multi-arch, with voice deps), one-click VPS scripts (like OpenClaw tutorials).
- **Security/Proactive**: Enhanced sandbox (WASM + seccomp/landlock), permission system per skill, audit logs.
- **Community**: ClawHub-compatible registry CLI, examples repo, skill templates, benchmarks (vs OpenClaw TS).
- **Advanced**: Multi-agent swarming (leverage existing `.swarm/` dir + spawning), planning (ReAct/Plan-and-Execute in Router), cost/latency tracking in Scorer.

**Milestone**: ClawFT passes "OpenClaw feature parity test suite" + outperforms on speed/memory; public "ClawFT Skills" registry with 50+ skills.

### Recommended Tech Additions (minimal, Rust-native)
- `wasmtime` + `wit-bindgen` (plugins)
- `lettre`, `imap`, `chromiumoxide`, `whisper-rs`, `piper-rs`, `git2`, `oauth2`, `tree-sitter`
- `serde`, `tokio`, `tracing` (already present)
- Feature flags: `voice`, `browser`, `email-full`

### Timeline & Resourcing (assuming 2–4 person team)
- **MVP (Phases 0–1)**: 2–3 months → usable OpenClaw-like with plugins, email, voice.
- **Full Vision (Phases 0–3)**: 4–6 months.
- **Ongoing**: Monthly skill releases, quarterly core updates.

**Risks & Mitigations**:
- Plugin API stability: Start with internal dogfooding + versioning.
- Dependency bloat: Keep core tiny; heavy stuff in optional plugins.
- Voice quality: Local models first, cloud fallback.
- Compatibility: SKILL.md parser as thin layer (LLM interprets + calls Rust tools).

**Next Immediate Steps (this week)**:
1. Create `feature/plugin-system` branch + issue template.
2. Prototype WASM skill loader (1–2 days).
3. Port 3 OpenClaw skills (e.g., GitHub, email-triage, browser) as proof-of-concept.
4. Update README with this roadmap + "How to contribute a skill".
5. (Optional) Rename/marketing: "ClawFT – The Rust-powered OpenClaw" or spin "IronClaw" crate.

This roadmap keeps ClawFT's strengths (performance, safety, MCP) while directly addressing your asks: **plugins first**, software dev superpowers, email/respond, TTS/STT, app/tools extensibility. It positions ClawFT as the **fast, secure, future-proof** alternative that OpenClaw users will migrate to.

Let me know priorities (e.g., start with voice? plugins?), team assignments, or if you want:
- Detailed crate diagrams
- Sample `SKILL.md` → WASM conversion code
- GitHub issues/PR template
- Cost/dependency audit

We're building the lobster that actually ships. Let's make it unstoppable. 🚀🦀