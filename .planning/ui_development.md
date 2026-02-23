# ClawFT UI Development Sprint Plan

> Comprehensive sprint plan for the ClawFT web dashboard and Live Canvas,
> targeting OpenClaw UI feature parity. Stack: **Vite + React + TypeScript + shadcn/ui + Tailwind CSS**.
> Backend API via **Axum** (Rust) with WebSocket real-time transport.
>
> **The UI is a standalone application.** It has its own repository-grade project structure,
> independent build pipeline, and connects to any clawft gateway instance via configurable
> API URL. It can be developed, tested, and deployed without the Rust backend running
> (mock API via MSW). Optionally embeddable into the `weft` binary for single-binary distribution.

---

## Table of Contents

1. [Standalone Architecture](#standalone-architecture)
2. [Architecture Overview](#architecture-overview)
3. [OpenClaw Feature Parity Matrix](#openclaw-feature-parity-matrix)
4. [Pre-Implementation Requirements](#pre-implementation-requirements)
5. [Backend API Dependencies](#backend-api-dependencies)
6. [Sprint Plan](#sprint-plan)
7. [Dependency Sprint (Parallel Track)](#dependency-sprint-parallel-track)
8. [Cross-Platform Native Shells (K6)](#cross-platform-native-shells-k6)

---

## Standalone Architecture

The UI is a **fully standalone application** that communicates with the clawft backend
exclusively through HTTP REST and WebSocket APIs. There is zero compile-time coupling
between the frontend and the Rust crates.

### Deployment Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| **Standalone dev** | `pnpm dev` in `ui/` -- Vite dev server on `:5173`, proxies API to configurable backend | Frontend development, no Rust build needed |
| **Standalone static** | `pnpm build` produces `ui/dist/` -- deploy to any static host (Nginx, Caddy, S3, Vercel) | Production deployment separate from backend |
| **Embedded in weft** | `weft ui` serves `ui/dist/` from disk or via `rust-embed` | Single-binary convenience (optional) |
| **Tauri desktop** | Tauri wraps `ui/dist/` in native window, manages `weft gateway` lifecycle | Desktop app |
| **Docker compose** | Frontend container (Nginx) + backend container (weft gateway) | Server deployment |

### Connection Configuration

The UI discovers the backend via environment variable or runtime config:

```typescript
// ui/src/lib/config.ts
export const config = {
  // Configurable at build time (env) or runtime (localStorage / URL param)
  apiUrl: import.meta.env.VITE_API_URL ?? "http://localhost:18789",
  wsUrl:  import.meta.env.VITE_WS_URL  ?? "ws://localhost:18789",
};
```

In dev mode, `vite.config.ts` proxies `/api/*` and `/ws` to the backend:

```typescript
// ui/vite.config.ts
export default defineConfig({
  server: {
    proxy: {
      "/api": { target: "http://localhost:18789" },
      "/ws":  { target: "ws://localhost:18789", ws: true },
    },
  },
});
```

### Mock API for Independent Development

The UI ships with a full **MSW (Mock Service Worker)** mock layer so frontend
development can proceed without the Rust backend:

```text
ui/src/mocks/
├── handlers/
│   ├── agents.ts       -- Mock agent CRUD
│   ├── sessions.ts     -- Mock session data
│   ├── tools.ts        -- Mock tool registry
│   ├── memory.ts       -- Mock memory store
│   ├── config.ts       -- Mock config
│   ├── skills.ts       -- Mock skill browser
│   └── ws.ts           -- Mock WebSocket events
├── data/
│   ├── agents.json     -- Fixture data
│   ├── sessions.json
│   └── tools.json
├── browser.ts          -- MSW browser setup
└── server.ts           -- MSW node setup (for Vitest)
```

Enable mocks via env var:
```bash
VITE_MOCK_API=true pnpm dev    # Frontend-only development
pnpm dev                        # Connects to real backend (default)
```

### Build Pipeline

```bash
# Frontend only (no Rust)
cd ui && pnpm install && pnpm dev        # Dev server with HMR
cd ui && pnpm build                       # Production build -> ui/dist/
cd ui && pnpm test                        # Vitest unit tests
cd ui && pnpm test:e2e                    # Playwright E2E (needs backend or mocks)
cd ui && pnpm lint                        # ESLint + Prettier
cd ui && pnpm type-check                  # tsc --noEmit

# Embedded in weft (optional)
cargo build -p clawft-cli --features ui   # Embeds ui/dist/ via rust-embed
weft ui                                    # Serves embedded or falls back to ui/dist/ on disk
```

### CORS & Security

When running standalone (different origin than backend):
- Backend Axum layer includes `tower-http::cors` middleware
- Allowed origins configurable via `GatewayConfig.cors_origins`
- Default: `["http://localhost:5173"]` (Vite dev server)
- Token auth works cross-origin via `Authorization: Bearer <token>` header

### Why Standalone?

1. **Independent iteration** -- Frontend devs work without Rust toolchain
2. **Separate deployment** -- CDN for static assets, backend on VPS/container
3. **Testability** -- Full mock layer means UI tests don't need a running backend
4. **Multiple backends** -- Same UI can connect to different clawft instances (dev, staging, prod)
5. **Tauri-ready** -- Tauri loads web assets from disk, doesn't care about Rust build
6. **CI parallelism** -- Frontend CI (lint, test, build) runs independently of `cargo test`

---

## Architecture Overview

```text
┌─────────────────────────────────────────────────────────┐
│                    Frontend (Vite + React)               │
│  ┌──────────┐ ┌──────────┐ ┌───────────┐ ┌───────────┐ │
│  │ Dashboard │ │ WebChat  │ │Live Canvas│ │  Settings │ │
│  └────┬─────┘ └────┬─────┘ └─────┬─────┘ └─────┬─────┘ │
│       │             │             │              │       │
│  ┌────┴─────────────┴─────────────┴──────────────┴────┐ │
│  │          State Management (Zustand / TanStack)     │ │
│  └────────────────────┬───────────────────────────────┘ │
│                       │                                 │
│  ┌────────────────────┴───────────────────────────────┐ │
│  │    Transport Layer (REST + WebSocket client)       │ │
│  └────────────────────┬───────────────────────────────┘ │
└───────────────────────┼─────────────────────────────────┘
                        │ HTTP/WS
┌───────────────────────┼─────────────────────────────────┐
│              Backend (Axum on Gateway)                   │
│  ┌────────────────────┴───────────────────────────────┐ │
│  │      REST API + WebSocket Upgrade Handler          │ │
│  └──┬──────────┬──────────┬──────────┬────────────────┘ │
│     │          │          │          │                   │
│  ┌──┴───┐ ┌───┴───┐ ┌───┴───┐ ┌───┴──────┐           │
│  │Agent │ │Session│ │Config │ │ Tool     │            │
│  │ API  │ │  API  │ │  API  │ │Registry  │            │
│  └──────┘ └───────┘ └───────┘ └──────────┘            │
│                                                         │
│  ┌─────────────────────────────────────────────────┐   │
│  │  clawft-core (AgentLoop, Pipeline, MessageBus)  │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Stack Decision: Vite + shadcn (not Leptos)

The original plan specified Axum + Leptos (full-stack Rust). This sprint uses **Vite + React + shadcn/ui** instead:

| Concern | Leptos (original) | Vite + React + shadcn (new) |
|---------|-------------------|-----------------------------|
| Ecosystem | Small, Rust-only | Massive, battle-tested |
| Component library | Limited | shadcn/ui (50+ components) |
| Developer pool | Rare | Abundant |
| Iteration speed | Slow (recompile) | Fast (HMR) |
| Tauri compatibility | Good | Good (web assets) |
| OpenClaw parity | OpenClaw uses React | Direct component parity |

**Frontend project lives in:** `ui/` directory at workspace root.

---

## OpenClaw Feature Parity Matrix

Features from OpenClaw's UI mapped to ClawFT equivalents:

| OpenClaw Feature | ClawFT Equivalent | Sprint | Status |
|------------------|--------------------|--------|--------|
| Control UI Dashboard | Dashboard home | S1 | Planned |
| WebChat | Conversation view | S1 | Planned |
| Live Canvas (A2UI) | Canvas workspace | S2 | Planned |
| Agent management | Agent CRUD + start/stop | S1 | Planned |
| Cron job dashboard | Cron scheduler view | S2 | Planned |
| Skill browser | Skill browser + ClawHub | S2 | Planned |
| Session history | Session explorer | S1 | Planned |
| Memory explorer | Memory/vector search | S2 | Planned |
| Tool registry inspector | Tool list + schema view | S1 | Planned |
| Configuration editor | Config CRUD | S2 | Planned |
| Token-based auth | Auth middleware | S1 | Planned |
| Multi-channel inbox | Channel status + routing | S2 | Planned |
| Browser automation view | CDP screenshot viewer | S3 | Planned |
| Delegation monitor | Flow/Claude delegation UI | S3 | Planned |
| Voice Wake indicator | Voice status bar | S3 (voice track) | Planned |
| Talk Mode overlay | Voice conversation UI | S3 (voice track) | Planned |
| Tailscale auth | Remote auth provider | S3 | Planned |

---

## Pre-Implementation Requirements

### P1. Axum REST + WebSocket API Layer (Backend, Rust)

The existing `GatewayConfig` binds to `127.0.0.1:18789`. The UI API extends this.

**Endpoints needed before UI sprint begins:**

```text
# Agent Management
GET    /api/agents                    -- List agents
GET    /api/agents/:id                -- Agent details
POST   /api/agents/:id/start          -- Start agent
POST   /api/agents/:id/stop           -- Stop agent
PATCH  /api/agents/:id                -- Update agent config

# Sessions
GET    /api/sessions                  -- List sessions
GET    /api/sessions/:key             -- Session history
DELETE /api/sessions/:key             -- Delete session

# Memory
GET    /api/memory                    -- List memory entries
GET    /api/memory/:key               -- Retrieve entry
POST   /api/memory/search             -- Semantic search
POST   /api/memory                    -- Store entry
DELETE /api/memory/:key               -- Delete entry

# Tools
GET    /api/tools                     -- List registered tools
GET    /api/tools/:name/schema        -- Tool JSON schema

# Skills
GET    /api/skills                    -- List installed skills
POST   /api/skills/install            -- Install skill
DELETE /api/skills/:name              -- Uninstall skill
GET    /api/skills/hub/search         -- Search ClawHub

# Config
GET    /api/config                    -- Current resolved config
PATCH  /api/config                    -- Update config section
GET    /api/config/validate           -- Validate config

# Channels
GET    /api/channels                  -- Channel status
POST   /api/channels/:name/restart    -- Restart channel

# Cron
GET    /api/cron                      -- List cron jobs
POST   /api/cron                      -- Create cron job
DELETE /api/cron/:id                  -- Delete cron job

# Delegation
GET    /api/delegation/status         -- Delegation engine status
GET    /api/delegation/rules          -- Active rules

# Auth
POST   /api/auth/token                -- Generate access token
GET    /api/auth/verify               -- Verify token

# WebSocket
WS     /ws                           -- Real-time event stream
WS     /ws/canvas                    -- Live Canvas channel
WS     /ws/chat/:session             -- Chat session stream
```

### P2. WebSocket Protocol Design

**Event types (server -> client):**

```typescript
type WsEvent =
  | { type: "agent:status"; agentId: string; status: "running" | "stopped" | "error" }
  | { type: "message:inbound"; sessionKey: string; message: InboundMessage }
  | { type: "message:outbound"; sessionKey: string; message: OutboundMessage }
  | { type: "tool:call"; sessionKey: string; toolName: string; args: Record<string, unknown> }
  | { type: "tool:result"; sessionKey: string; toolName: string; result: string }
  | { type: "pipeline:stage"; sessionKey: string; stage: string; data: unknown }
  | { type: "canvas:render"; canvasId: string; commands: CanvasCommand[] }
  | { type: "canvas:reset"; canvasId: string }
  | { type: "memory:update"; key: string; namespace: string }
  | { type: "channel:status"; channel: string; status: "connected" | "disconnected" | "error" }
  | { type: "cron:fired"; jobId: string; timestamp: string }
  | { type: "skill:reload"; skillName: string }
  | { type: "delegation:start"; taskId: string; target: string }
  | { type: "delegation:complete"; taskId: string; result: string }
  | { type: "voice:status"; state: "idle" | "listening" | "processing" | "speaking" }
```

**Commands (client -> server):**

```typescript
type WsCommand =
  | { type: "chat:send"; sessionKey: string; content: string }
  | { type: "canvas:interact"; canvasId: string; event: CanvasInteraction }
  | { type: "agent:command"; agentId: string; command: string }
  | { type: "subscribe"; topics: string[] }
  | { type: "unsubscribe"; topics: string[] }
```

### P3. Authentication Design

**Phase 1 (S1):** Token-based local auth
- `weft ui` generates a one-time token, opens browser with `?token=xxx`
- Token stored in `~/.clawft/ui-token` with configurable TTL
- API middleware validates Bearer token on all `/api/*` routes
- No multi-user -- single operator model (matches OpenClaw local-first)

**Phase 2 (S3):** Multi-user + remote access
- Tailscale identity headers (X-Tailscale-User-*)
- Optional password enforcement
- Per-user permission scoping

### P4. Frontend Project Scaffolding

```text
ui/                              # ← Standalone project, own package.json, own CI
├── package.json                 #   "name": "@clawft/ui"
├── pnpm-lock.yaml
├── vite.config.ts               #   Proxy config for dev, build output to dist/
├── tsconfig.json
├── tailwind.config.ts
├── postcss.config.js
├── .env                         #   VITE_API_URL, VITE_WS_URL (defaults to localhost:18789)
├── .env.mock                    #   VITE_MOCK_API=true (for frontend-only dev)
├── Dockerfile                   #   Multi-stage: build -> nginx:alpine serving dist/
├── index.html
├── public/
│   └── favicon.svg
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── mocks/                   # ← MSW mock layer for backend-independent dev
│   │   ├── handlers/
│   │   │   ├── agents.ts
│   │   │   ├── sessions.ts
│   │   │   ├── tools.ts
│   │   │   ├── memory.ts
│   │   │   ├── config.ts
│   │   │   ├── skills.ts
│   │   │   └── ws.ts
│   │   ├── data/                # Fixture JSON for mock responses
│   │   ├── browser.ts
│   │   └── server.ts
│   ├── routes/
│   │   ├── index.tsx              -- Dashboard home
│   │   ├── chat.tsx               -- WebChat
│   │   ├── chat.$session.tsx      -- Chat session
│   │   ├── agents.tsx             -- Agent management
│   │   ├── agents.$id.tsx         -- Agent detail
│   │   ├── skills.tsx             -- Skill browser
│   │   ├── tools.tsx              -- Tool registry
│   │   ├── sessions.tsx           -- Session explorer
│   │   ├── memory.tsx             -- Memory explorer
│   │   ├── config.tsx             -- Configuration editor
│   │   ├── cron.tsx               -- Cron scheduler
│   │   ├── channels.tsx           -- Channel status
│   │   ├── canvas.tsx             -- Live Canvas
│   │   └── delegation.tsx         -- Delegation monitor
│   ├── components/
│   │   ├── ui/                    -- shadcn/ui components
│   │   ├── layout/
│   │   │   ├── sidebar.tsx
│   │   │   ├── header.tsx
│   │   │   └── main-layout.tsx
│   │   ├── chat/
│   │   │   ├── message-bubble.tsx
│   │   │   ├── message-input.tsx
│   │   │   ├── tool-call-card.tsx
│   │   │   └── streaming-text.tsx
│   │   ├── canvas/
│   │   │   ├── canvas-renderer.tsx
│   │   │   ├── canvas-toolbar.tsx
│   │   │   └── canvas-element.tsx
│   │   ├── agents/
│   │   │   ├── agent-card.tsx
│   │   │   └── agent-config-form.tsx
│   │   ├── skills/
│   │   │   ├── skill-card.tsx
│   │   │   └── skill-install-dialog.tsx
│   │   └── common/
│   │       ├── status-badge.tsx
│   │       ├── json-viewer.tsx
│   │       └── code-block.tsx
│   ├── hooks/
│   │   ├── use-websocket.ts
│   │   ├── use-api.ts
│   │   └── use-auth.ts
│   ├── stores/
│   │   ├── agent-store.ts
│   │   ├── session-store.ts
│   │   ├── canvas-store.ts
│   │   └── ws-store.ts
│   ├── lib/
│   │   ├── api-client.ts
│   │   ├── ws-client.ts
│   │   ├── types.ts
│   │   └── utils.ts
│   └── styles/
│       └── globals.css
└── tests/
    ├── components/
    └── integration/
```

### P5. Component Library (shadcn/ui)

Core shadcn components needed:

| Component | Used In |
|-----------|---------|
| `Button`, `Input`, `Textarea` | All forms |
| `Card`, `Badge`, `Avatar` | Agent cards, status |
| `Dialog`, `Sheet`, `Popover` | Modals, skill install |
| `Table`, `DataTable` | Sessions, memory, tools |
| `Tabs`, `Accordion` | Config editor, agent detail |
| `Command`, `CommandPalette` | Global search, skill search |
| `Toast`, `Sonner` | Notifications |
| `Sidebar`, `NavigationMenu` | Layout |
| `ScrollArea` | Chat, logs |
| `CodeBlock` (custom) | Tool results, config |
| `Form` (react-hook-form) | Config editor, agent config |
| `Select`, `Switch`, `Slider` | Settings |
| `Skeleton` | Loading states |
| `ResizablePanel` | Canvas + chat split view |

### P6. Performance Budget

| Metric | Target |
|--------|--------|
| Initial bundle (gzipped) | < 200 KB |
| First Contentful Paint | < 1.5s |
| Time to Interactive | < 2.5s |
| WebSocket latency (local) | < 50ms |
| WebSocket latency (Tailscale) | < 200ms |
| Memory (idle dashboard) | < 50 MB |
| Canvas render (100 elements) | < 16ms (60fps) |

---

## Sprint Plan

### Sprint 1: Foundation + Core Views (Weeks 1-3)

**Goal:** Dashboard shell, authentication, WebChat, agent management, tool inspector.

#### S1.1 Backend API Foundation (Week 1)

| # | Task | Est | Crate |
|---|------|-----|-------|
| S1.1.1 | Add `axum-extra` + `tower-http` to `clawft-services/Cargo.toml` | 2h | clawft-services |
| S1.1.2 | Create `clawft-services/src/api/mod.rs` -- Axum router factory | 4h | clawft-services |
| S1.1.3 | Implement `/api/auth/token` + Bearer middleware | 4h | clawft-services |
| S1.1.4 | Implement agent CRUD endpoints (`/api/agents/*`) | 6h | clawft-services |
| S1.1.5 | Implement session endpoints (`/api/sessions/*`) | 4h | clawft-services |
| S1.1.6 | Implement tool listing (`/api/tools`, `/api/tools/:name/schema`) | 3h | clawft-services |
| S1.1.7 | WebSocket upgrade handler (`/ws`) with topic subscription | 8h | clawft-services |
| S1.1.8 | Wire API router into existing Gateway startup (`weft gateway`) | 4h | clawft-cli |
| S1.1.9 | CORS middleware (`tower-http::cors`) with configurable `cors_origins` | 2h | clawft-services |
| S1.1.10 | Add `weft ui` CLI command (starts gateway + opens browser with token) | 3h | clawft-cli |
| S1.1.11 | Optional static file serving: `ui/dist/` from disk or `rust-embed` behind `--features ui` | 3h | clawft-services |

**Week 1 Deliverable:** Backend API running. UI connects standalone via `pnpm dev` (Vite proxy) or `weft ui` (embedded).

#### S1.2 Frontend Scaffolding (Week 1, parallel with S1.1)

| # | Task | Est |
|---|------|-----|
| S1.2.1 | Initialize Vite + React + TypeScript project in `ui/` | 1h |
| S1.2.2 | Install and configure Tailwind CSS v4 | 1h |
| S1.2.3 | Initialize shadcn/ui (`npx shadcn@latest init`) | 1h |
| S1.2.4 | Install core shadcn components (Button, Card, Badge, Table, Tabs, Dialog, Sidebar, Toast) | 2h |
| S1.2.5 | Create `MainLayout` with collapsible sidebar navigation | 4h |
| S1.2.6 | Create `api-client.ts` (fetch wrapper with auth token) | 3h |
| S1.2.7 | Create `ws-client.ts` (WebSocket reconnecting client) | 4h |
| S1.2.8 | Create `use-auth.ts` hook (token extraction from URL, storage) | 2h |
| S1.2.9 | Create `types.ts` (TypeScript types mirroring Rust API types) | 3h |
| S1.2.10 | Set up TanStack Router for file-based routing | 3h |
| S1.2.11 | MSW mock handlers for agents, sessions, tools (backend-independent dev) | 4h |
| S1.2.12 | `Dockerfile` for standalone deployment (multi-stage: build -> nginx:alpine) | 2h |
| S1.2.13 | `ui/.env` / `.env.mock` configuration files | 1h |

**Week 1 Deliverable:** Navigable dashboard shell with sidebar, auth flow. Runs fully standalone via `VITE_MOCK_API=true pnpm dev` with no backend.

#### S1.3 Core Views (Weeks 2-3)

| # | Task | Est |
|---|------|-----|
| S1.3.1 | **Dashboard Home** -- Agent count, channel status, recent sessions, system health cards | 6h |
| S1.3.2 | **Agent Management** -- List view with status badges, start/stop actions | 6h |
| S1.3.3 | **Agent Detail** -- Config form, model selector, workspace path, permissions | 8h |
| S1.3.4 | **WebChat** -- Session list sidebar + message thread | 8h |
| S1.3.5 | **WebChat Streaming** -- Real-time message display via WebSocket | 6h |
| S1.3.6 | **WebChat Input** -- Message composer with send, file upload placeholder | 4h |
| S1.3.7 | **Tool Call Cards** -- Expandable cards showing tool name, args, result in chat | 6h |
| S1.3.8 | **Session Explorer** -- DataTable with session keys, message counts, timestamps | 4h |
| S1.3.9 | **Session Detail** -- Full conversation history with export button | 4h |
| S1.3.10 | **Tool Registry** -- List all tools with schema viewer (JSON tree) | 4h |
| S1.3.11 | Dark/light theme toggle (shadcn theme system) | 2h |
| S1.3.12 | Global command palette (Cmd+K) for navigation | 3h |
| S1.3.13 | Toast notifications for WebSocket events | 2h |

**Sprint 1 Deliverable:** Functional dashboard with auth, agent management, WebChat with streaming, session explorer, tool inspector.

---

### Sprint 2: Advanced Views + Live Canvas (Weeks 4-6)

**Goal:** Live Canvas, skill browser, memory explorer, config editor, cron dashboard, channel status.

#### S2.1 Live Canvas (Weeks 4-5)

| # | Task | Est |
|---|------|-----|
| S2.1.1 | Define `CanvasCommand` protocol (render, update, remove, reset, snapshot) | 4h |
| S2.1.2 | Backend: Canvas WebSocket handler (`/ws/canvas`) with command routing | 6h |
| S2.1.3 | Backend: Canvas tool (`render_ui` tool) agents call to push UI elements | 6h |
| S2.1.4 | Frontend: `CanvasRenderer` component (renders CanvasCommand stream) | 8h |
| S2.1.5 | Canvas element types: text, button, input, image, code, chart, table, form | 12h |
| S2.1.6 | Canvas interaction: click/submit events routed back to agent as tool results | 6h |
| S2.1.7 | Canvas state persistence (snapshot/restore via API) | 4h |
| S2.1.8 | Split-view: Canvas + Chat side-by-side (ResizablePanel) | 4h |
| S2.1.9 | Canvas toolbar: zoom, pan, fullscreen, snapshot, clear | 3h |

**Week 5 Deliverable:** Agent can render interactive UI elements in Canvas; user interactions route back to agent.

#### S2.2 Skill Browser + ClawHub (Week 5)

| # | Task | Est |
|---|------|-----|
| S2.2.1 | Backend: Skill listing + search endpoints (`/api/skills/*`) | 4h |
| S2.2.2 | Backend: ClawHub proxy endpoint (`/api/skills/hub/search`) | 4h |
| S2.2.3 | Installed skills view: card grid with name, version, description, tools provided | 4h |
| S2.2.4 | ClawHub search: search bar + results grid with install button | 4h |
| S2.2.5 | Skill detail: SKILL.md content rendered as markdown, tool list, permissions | 3h |
| S2.2.6 | Install/uninstall actions with progress toast | 3h |

#### S2.3 Memory Explorer (Week 5)

| # | Task | Est |
|---|------|-----|
| S2.3.1 | Backend: Memory CRUD + search endpoints (`/api/memory/*`) | 4h |
| S2.3.2 | Memory list view: DataTable with key, namespace, tags, timestamp | 3h |
| S2.3.3 | Memory detail view: full content with JSON/markdown rendering | 3h |
| S2.3.4 | Semantic search: search bar with threshold slider, namespace filter | 4h |
| S2.3.5 | Memory write dialog: key, value, namespace, tags, TTL | 3h |

#### S2.4 Configuration Editor (Week 6)

| # | Task | Est |
|---|------|-----|
| S2.4.1 | Backend: Config read/write endpoints with validation | 4h |
| S2.4.2 | Config viewer: tabbed sections (agents, providers, channels, tools, routing, delegation) | 6h |
| S2.4.3 | Inline editing with JSON schema validation | 6h |
| S2.4.4 | Config diff view: show changes before save | 3h |
| S2.4.5 | Provider config: model selector, API key status (masked), endpoint override | 4h |

#### S2.5 Cron + Channels (Week 6)

| # | Task | Est |
|---|------|-----|
| S2.5.1 | Backend: Cron CRUD endpoints (`/api/cron/*`) | 3h |
| S2.5.2 | Cron dashboard: job list, expression editor, next-fire preview, execution log | 4h |
| S2.5.3 | Channel status view: connection state per channel, message counts, restart button | 4h |
| S2.5.4 | Channel routing view: visual routing table (channel -> agent) | 3h |

**Sprint 2 Deliverable:** Full-featured dashboard with Live Canvas, skill browser, memory explorer, config editor, cron manager, channel status.

---

### Sprint 3: Polish + Advanced Features (Weeks 7-9)

**Goal:** Delegation monitor, advanced Canvas, mobile responsive, Tauri desktop, production hardening.

#### S3.1 Delegation & Monitoring (Week 7)

| # | Task | Est |
|---|------|-----|
| S3.1.1 | Delegation monitor: active delegations, target (local/Claude/Flow), status, latency | 4h |
| S3.1.2 | Delegation rule editor: regex pattern, target, complexity threshold | 4h |
| S3.1.3 | Pipeline stage inspector: real-time 6-stage pipeline visualization per session | 6h |
| S3.1.4 | Token usage dashboard: per-session, per-provider, per-model usage charts | 4h |
| S3.1.5 | Cost tracker: daily/weekly/monthly cost breakdown by provider | 4h |

#### S3.2 Advanced Canvas (Week 7-8)

| # | Task | Est |
|---|------|-----|
| S3.2.1 | Chart rendering (recharts/visx integration for Canvas chart elements) | 6h |
| S3.2.2 | Code editor element (Monaco or CodeMirror embedded in Canvas) | 6h |
| S3.2.3 | Form builder: agent can construct multi-field forms with validation | 4h |
| S3.2.4 | Canvas history: undo/redo stack, replay timeline | 4h |
| S3.2.5 | Canvas export: PNG screenshot, HTML export, JSON state export | 3h |

#### S3.3 Mobile Responsive + PWA (Week 8)

| # | Task | Est |
|---|------|-----|
| S3.3.1 | Responsive sidebar: drawer on mobile, collapsible on tablet | 4h |
| S3.3.2 | Mobile WebChat: bottom-anchored input, swipe navigation | 4h |
| S3.3.3 | PWA manifest + service worker (offline dashboard shell) | 3h |
| S3.3.4 | Push notification integration (service worker + WS events) | 4h |

#### S3.4 Tauri Desktop Shell (Week 8-9)

| # | Task | Est |
|---|------|-----|
| S3.4.1 | Initialize Tauri project wrapping `ui/dist` | 4h |
| S3.4.2 | System tray icon with agent status indicator | 4h |
| S3.4.3 | Global hotkey (Cmd+Shift+W) to toggle window | 2h |
| S3.4.4 | Auto-start `weft gateway` on app launch | 3h |
| S3.4.5 | macOS Spotlight integration (quick agent query) | 4h |
| S3.4.6 | Linux/Windows native notification bridge | 3h |

#### S3.5 Production Hardening (Week 9)

| # | Task | Est |
|---|------|-----|
| S3.5.1 | CSP headers and XSS protection on static file serving | 2h |
| S3.5.2 | Rate limiting on API endpoints | 3h |
| S3.5.3 | WebSocket heartbeat + dead connection cleanup | 2h |
| S3.5.4 | Error boundary components with graceful degradation | 3h |
| S3.5.5 | E2E tests with Playwright (dashboard + WebChat + Canvas) | 8h |
| S3.5.6 | Bundle analysis + tree-shaking optimization | 2h |
| S3.5.7 | Accessibility audit (axe-core) + WCAG AA fixes | 4h |
| S3.5.8 | Tailscale auth provider (X-Tailscale-User headers) | 4h |
| S3.5.9 | Multi-user session isolation | 4h |

**Sprint 3 Deliverable:** Production-ready dashboard with delegation monitor, advanced Canvas, Tauri desktop app, mobile PWA, auth hardening.

---

## Dependency Sprint (Parallel Track)

These items from the main `improvements.md` sprint must be completed to unblock UI features.
They can proceed in parallel with UI development by different developers.

### Tier 1: Required Before Sprint 1 Starts

| Item | Workstream | Blocks | Status | Notes |
|------|-----------|--------|--------|-------|
| Gateway API router extension | New (UI) | All UI | Not Started | New Axum routes on existing gateway |
| B5: Extract shared tool registry builder | B | S1 (tools API) | Not Started | Needed for clean API access to ToolRegistry |
| WebSocket event bus | New (UI) | S1 (streaming) | Not Started | Tap into MessageBus for WS broadcast |

### Tier 2: Required Before Sprint 2

| Item | Workstream | Blocks | Status | Notes |
|------|-----------|--------|--------|-------|
| C3: Skill Loader | C | S2 (skill browser) | Not Started | Skills list API needs skill registry |
| C4: Hot-reload | C | S2 (skill browser) | Not Started | `skill:reload` WS events |
| C6: MCP server for skills | C | S2 (tool inspector) | Not Started | Skill tools visible in registry |
| H1: Per-agent workspace | H | S2 (memory explorer) | Not Started | Workspace directory for memory API |
| H2.1: HNSW VectorStore | H | S2 (semantic search) | Not Started | Memory search API backend |
| B4: Unified cron storage | B | S2 (cron dashboard) | Not Started | Single cron data source |
| L1: Agent routing table | L | S2 (channel routing) | Not Started | Route visualization data |

### Tier 3: Required Before Sprint 3

| Item | Workstream | Blocks | Status | Notes |
|------|-----------|--------|--------|-------|
| M1: FlowDelegator | M | S3 (delegation monitor) | Not Started | Delegation status events |
| M2: flow_available detection | M | S3 (delegation monitor) | Not Started | Runtime delegation target |
| D5: Record actual latency | D | S3 (pipeline inspector) | Not Started | Real latency data |
| D6: Thread sender_id | D | S3 (cost tracker) | Not Started | Per-user cost attribution |
| K4: ClawHub registry | K | S2 (ClawHub search) | Not Started | External skill search |

### Tier 4: Nice-to-Have (Can Stub)

| Item | Workstream | Enhancement | Status | Notes |
|------|-----------|-------------|--------|-------|
| F4: Browser CDP | F | S3 (screenshot viewer) | Not Started | Can show static screenshots |
| H2.2: Production embedder | H | S2 (better search) | Not Started | HashEmbedder works for MVP |
| L2: Per-agent isolation | L | S3 (multi-agent view) | Not Started | Single-agent OK for MVP |

---

## Cross-Platform Native Shells (K6)

**Deps:** Sprint 3 completion (S3.4 Tauri shell)

### Desktop (Tauri)

Single Tauri codebase for macOS, Linux, Windows:
- Wraps `ui/dist/` web assets in native window
- Embeds or connects to `weft gateway` process
- System tray with agent status
- Global hotkey
- Auto-update via Tauri updater
- Voice Wake integration point (G5) -- mic capture runs natively, STT result pushed to WebSocket

### Mobile (Post-Sprint)

- **iOS:** SwiftUI shell with WKWebView loading dashboard
  - Push notifications via APNs
  - Siri Shortcuts for quick agent queries
  - Background fetch for proactive check-ins
- **Android:** Jetpack Compose with WebView
  - FCM push notifications
  - Widget for quick agent access
  - Background work manager for check-ins

---

## Timeline Summary

| Week | Sprint | Deliverables |
|------|--------|-------------|
| 1 | S1 | Backend API, frontend scaffold, auth flow |
| 2-3 | S1 | Dashboard home, WebChat + streaming, agent management, session explorer, tools |
| 4-5 | S2 | Live Canvas (A2UI), skill browser, memory explorer |
| 6 | S2 | Config editor, cron dashboard, channel status |
| 7 | S3 | Delegation monitor, pipeline inspector, advanced Canvas |
| 8 | S3 | Mobile responsive, PWA, Tauri desktop shell |
| 9 | S3 | Production hardening, E2E tests, accessibility, multi-user auth |

**Total: 9 weeks** (can overlap with voice sprint and dependency sprint)

---

## Key Technical Decisions

1. **Standalone-first:** UI is an independent app with its own `package.json`, build, and CI. Zero compile-time coupling to Rust. Connects via HTTP/WS to any clawft backend.
2. **Mock API (MSW):** Full mock layer ships with UI so frontend development and testing work without a running backend.
3. **State management:** Zustand for local UI state, TanStack Query for server state (API caching, refetching)
4. **Routing:** TanStack Router (file-based, type-safe)
5. **Forms:** react-hook-form + zod validation (for config editor, agent config)
6. **Charts:** recharts (for token usage, cost tracking)
7. **Code display:** Shiki (syntax highlighting in tool results, config)
8. **Markdown:** react-markdown + remark-gfm (SKILL.md rendering, memory content)
9. **WebSocket:** Custom reconnecting client with exponential backoff + topic subscription
10. **Testing:** Vitest (unit) + Playwright (E2E, runs against mock or real backend)
11. **Build:** `pnpm build` produces `ui/dist/`. Optionally embedded into `weft` binary via `rust-embed` behind `--features ui` flag.
12. **Deployment:** Static host (Nginx, Caddy, S3, Vercel), Docker Compose (frontend + backend containers), or embedded in `weft ui`. All equivalent.
13. **CORS:** Backend configurable `cors_origins` for cross-origin standalone mode. Default allows Vite dev server.

---

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Backend API not ready for S1 | **No block** -- UI is standalone | MSW mock layer enables full frontend dev without backend. Mock handlers ship with the UI and are maintained alongside real API contracts. |
| WebSocket protocol changes mid-sprint | Rework frontend handlers | Define protocol in shared `types.ts` + Rust types; version WS messages |
| Canvas complexity explosion | Scope creep | MVP Canvas: text, button, code, image only. Charts/forms in S3 |
| shadcn breaking changes | Component rework | Pin shadcn version, use `components.json` lock |
| Tauri + voice integration | Cross-platform audio issues | Tauri handles window only; voice is separate native daemon |
| Bundle size exceeds budget | Slow load times | Dynamic imports for Canvas, charts, Monaco. Measure with `vite-bundle-analyzer` |
