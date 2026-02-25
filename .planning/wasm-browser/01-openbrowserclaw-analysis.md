# Reference Analysis: sachaa/openbrowserclaw

**Repo**: https://github.com/sachaa/openbrowserclaw
**Language**: TypeScript (100%)
**License**: MIT
**Created**: 2026-02-21 | **Stars**: 188

---

## What It Is

A pure TypeScript browser application where the browser tab IS the server. No WASM compilation of application code. Entire app compiles to static HTML/CSS/JS deployable to any CDN.

## Architecture

```
Browser Tab (PWA)
  +-- Chat UI / Settings / Task Manager (React 19)
  +-- Orchestrator (main thread)
  |     +-- Message queue & routing
  |     +-- State machine (idle/thinking/responding)
  |     +-- Task scheduler (cron via setInterval)
  +-- Agent Worker (Web Worker)
  |     +-- Claude API tool-use loop (raw fetch, 25 iterations max)
  |     +-- Tool execution engine
  |     +-- Shell emulator (pure JS, 747 lines)
  +-- IndexedDB (messages, tasks, config, sessions)
  +-- OPFS (per-group file storage)
  +-- Channels: Browser Chat (built-in) + Telegram (optional HTTPS long-poll)
```

## Key Design Decisions Relevant to clawft

### 1. Browser Storage Architecture

Three storage layers:

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Config | IndexedDB `config` store (key-value) | API keys, model prefs, trigger patterns |
| Messages | IndexedDB `messages` store (indexed by group+time) | Conversation history |
| Files | OPFS (Origin Private File System) | Per-group workspace, CLAUDE.md memory |

Config keys stored:
- `anthropic_api_key` (encrypted via Web Crypto AES-256-GCM)
- `model`, `max_tokens`, `assistant_name`
- `telegram_bot_token`, `telegram_chat_ids`
- `passphrase_salt`, `passphrase_verify`

### 2. API Key Security

- API keys encrypted at rest with non-extractable AES-256-GCM CryptoKey
- CryptoKey stored in separate IndexedDB (`obc-keystore`)
- `extractable: false` means JS cannot read raw key material
- Key generated once per browser origin, persists across sessions

### 3. LLM Communication

Direct browser `fetch()` to Anthropic API:
```typescript
const res = await fetch(ANTHROPIC_API_URL, {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'x-api-key': apiKey,
    'anthropic-version': ANTHROPIC_API_VERSION,
    'anthropic-dangerous-direct-browser-access': 'true',  // Critical header
  },
  body: JSON.stringify(body),
});
```

The `anthropic-dangerous-direct-browser-access: true` header bypasses Anthropic's CORS restrictions.

**Implications for clawft**: If targeting Anthropic API only, this header works. For other providers (OpenAI, etc.), a CORS proxy is needed.

### 4. Tool-Use Loop

Web Worker implements Claude tool-use loop:
- Max 25 iterations
- Sequential tool execution within each iteration
- Worker communicates via `postMessage` with typed messages:
  - Inbound: `invoke`, `compact`, `cancel`
  - Outbound: `response`, `error`, `typing`, `tool-activity`, `thinking-log`, `token-usage`

### 5. Shell Emulator

Pure JS implementation supporting: `echo`, `printf`, `cat`, `head`, `tail`, `wc`, `grep`, `sort`, `uniq`, `tr`, `cut`, `sed`, `awk`, `ls`, `mkdir`, `cp`, `mv`, `rm`, `touch`, `pwd`, `cd`, `date`, `env`, `export`, `base64`, `md5sum`/`sha256sum` (Web Crypto), `sleep`, `seq`, `jq`, `tee`, `test`

Operators: `|`, `>`, `>>`, `&&`, `||`, `;`, `$()`, backticks, `$VAR`

All operations run against OPFS filesystem.

### 6. PWA Support

- `display: standalone` (looks like native app when installed)
- Service worker for offline caching
- Persistent storage requested at runtime (`navigator.storage.persist()`)

### 7. Zero Runtime Dependencies for Core

The entire agent logic (API calls, tool execution, IndexedDB, OPFS, crypto, shell, orchestration) uses zero external libraries. Only React + markdown rendering + icons + state management (zustand) are dependencies.

### 8. Context Management

- Last 50 messages loaded from IndexedDB per group
- `CLAUDE.md` persistent memory file per group in OPFS
- Manual context compaction (Claude summarizes, old messages replaced)

## What clawft Should Adopt

1. **IndexedDB for config** -- Maps to `BrowserFileSystem` impl writing to virtual `/.clawft/config.json`
2. **OPFS for workspace files** -- Maps to `BrowserFileSystem` impl for all `Platform::fs()` operations
3. **Web Crypto for API key encryption** -- JS-side encryption before passing to WASM
4. **Direct fetch for LLM calls** -- `BrowserHttpClient` using `web-sys` fetch API
5. **Web Worker for agent loop** -- Keep UI responsive; WASM runs in Worker

## What clawft Should NOT Adopt

1. **Pure JS shell emulator** -- clawft already has proper tool implementations via `ToolRegistry`
2. **Rewriting in TypeScript** -- clawft's Rust core is more valuable; compile to WASM instead
3. **Single-provider assumption** -- clawft supports multiple LLM providers via tiered routing
4. **Flat message queue** -- clawft's pipeline (classify->route->assemble->transport->score) is superior

## File Inventory

```
src/
  main.tsx, App.tsx           -- React entry
  orchestrator.ts             -- Main thread coordinator
  agent-worker.ts             -- Web Worker: tool-use loop
  tools.ts                    -- Tool definitions
  shell.ts                    -- JS shell emulator (747 lines)
  vm.ts                       -- Optional v86 WebVM
  db.ts                       -- IndexedDB layer
  storage.ts                  -- OPFS helpers
  config.ts                   -- Config constants
  crypto.ts                   -- AES-256-GCM encryption
  router.ts                   -- Group prefix routing
  types.ts                    -- TypeScript interfaces
  task-scheduler.ts           -- Cron evaluator
  channels/
    browser-chat.ts           -- In-browser chat channel
    telegram.ts               -- Telegram Bot API channel
  stores/
    orchestrator-store.ts     -- Zustand state bridge
    theme-store.ts            -- Theme persistence
  components/                 -- React components
```
