# SPARC UI Element: Web Dashboard + Live Canvas

**Workstream**: K1 (Web Dashboard + Live Canvas)
**Timeline**: Weeks 1-9 (parallel with voice sprint)
**Status**: Planning
**Dependencies**: B5 (shared tool registry builder), C3/C4/C6 (skill system for skill browser), H1/H2 (memory for explorer), L1 (agent routing), M1/M2 (delegation for monitor), D5/D6 (latency/cost data)
**Blocks**: K6 (native shells via Tauri)

---

## 1. Summary

Build a standalone web dashboard and Live Canvas for ClawFT. The UI is an independent application (Vite + React + TypeScript + shadcn/ui + Tailwind CSS) that connects to any clawft gateway instance via configurable API URL. It can be developed, tested, and deployed without the Rust backend running (mock API via MSW). The backend API layer extends the existing Axum gateway with REST endpoints and WebSocket real-time transport. The dashboard covers agent management, WebChat with streaming, session explorer, tool registry, Live Canvas (agent-driven interactive UI), skill browser, memory explorer, configuration editor, delegation monitoring, and Tauri desktop shell. Optionally embeddable into the `weft` binary for single-binary distribution.

---

## 2. Phases

### Phase S1.1: Backend API Foundation (Week 1) -- P0

| Deliverable | Description | Crate |
|-------------|-------------|-------|
| Axum REST router | `clawft-services/src/api/mod.rs` -- Axum router factory with `axum-extra` + `tower-http` | clawft-services |
| Auth middleware | `/api/auth/token` + Bearer middleware; `weft ui` generates one-time token, opens browser | clawft-services |
| Agent CRUD endpoints | `GET/POST/PATCH /api/agents/*`, start/stop actions | clawft-services |
| Session endpoints | `GET/DELETE /api/sessions/*` -- list, detail, delete | clawft-services |
| Tool listing | `GET /api/tools`, `GET /api/tools/:name/schema` -- registered tools with JSON Schema | clawft-services |
| WebSocket upgrade handler | `/ws` with topic subscription (`subscribe`/`unsubscribe` commands) | clawft-services |
| `weft ui` CLI command | Starts gateway + opens browser with token param | clawft-cli |
| Static file serving | `ui/dist/` from disk or `rust-embed` behind `--features ui` | clawft-services |
| CORS middleware | `tower-http::cors` with configurable `cors_origins` (default: `localhost:5173`) | clawft-services |

### Phase S1.2: Frontend Scaffolding (Week 1, parallel with S1.1) -- P0

| Deliverable | Description | Location |
|-------------|-------------|----------|
| Vite + React + TS project | Initialize standalone project in `ui/` with own `package.json` | ui/ |
| Tailwind CSS v4 + shadcn/ui | Full component library init with core components | ui/ |
| MSW mock layer | Mock handlers for agents, sessions, tools; fixture JSON data | ui/src/mocks/ |
| `api-client.ts` | Fetch wrapper with Bearer auth token, configurable API URL | ui/src/lib/ |
| `ws-client.ts` | Reconnecting WebSocket client with exponential backoff + topic subscription | ui/src/lib/ |
| `use-auth.ts` hook | Token extraction from URL param, localStorage persistence | ui/src/hooks/ |
| TanStack Router | File-based type-safe routing setup | ui/src/routes/ |
| `MainLayout` | Collapsible sidebar navigation shell | ui/src/components/layout/ |
| Dockerfile | Multi-stage build: Vite build -> nginx:alpine serving `dist/` | ui/ |

### Phase S1.3: Core Views (Weeks 2-3) -- P0

| Deliverable | Description | Location |
|-------------|-------------|----------|
| Dashboard Home | Agent count, channel status, recent sessions, system health cards | ui/src/routes/index.tsx |
| Agent Management | List view with status badges, start/stop actions, config form | ui/src/routes/agents.tsx |
| WebChat + Streaming | Session list sidebar + message thread + real-time WS display | ui/src/routes/chat.tsx |
| Tool Call Cards | Expandable cards showing tool name, args, result inline in chat | ui/src/components/chat/ |
| Session Explorer | DataTable with session keys, message counts, timestamps, export | ui/src/routes/sessions.tsx |
| Tool Registry | List all tools with JSON Schema tree viewer | ui/src/routes/tools.tsx |
| Theme toggle | Dark/light mode via shadcn theme system | ui/src/components/layout/ |
| Command palette | Cmd+K global navigation (shadcn CommandPalette) | ui/src/components/common/ |

### Phase S2.1: Live Canvas (Weeks 4-5) -- P0

| Deliverable | Description | Location |
|-------------|-------------|----------|
| `CanvasCommand` protocol | render, update, remove, reset, snapshot command types | clawft-types + ui/src/lib/types.ts |
| Canvas WS handler | `/ws/canvas` backend handler with command routing | clawft-services |
| `render_ui` tool | Agent-callable tool that pushes UI elements to Canvas | clawft-tools |
| `CanvasRenderer` component | Renders CanvasCommand stream into interactive elements | ui/src/components/canvas/ |
| Element types | text, button, input, image, code, chart, table, form | ui/src/components/canvas/ |
| Interaction routing | Click/submit events routed back to agent as tool results | ui/src/components/canvas/ |
| State persistence | Canvas snapshot/restore via API | clawft-services + ui/ |
| Split view | Canvas + Chat side-by-side (ResizablePanel) | ui/src/routes/canvas.tsx |

### Phase S2.2: Skill Browser (Week 5) -- P1

| Deliverable | Description | Location |
|-------------|-------------|----------|
| Skill listing endpoint | `GET /api/skills`, `DELETE /api/skills/:name` | clawft-services |
| ClawHub proxy | `GET /api/skills/hub/search` -- proxies to ClawHub registry | clawft-services |
| Installed skills view | Card grid with name, version, description, tools provided | ui/src/routes/skills.tsx |
| ClawHub search | Search bar + results grid with install button | ui/src/routes/skills.tsx |
| Install/uninstall actions | Install from ClawHub or local path, progress toast | ui/src/components/skills/ |

### Phase S2.3: Memory Explorer (Week 5) -- P1

| Deliverable | Description | Location |
|-------------|-------------|----------|
| Memory CRUD endpoints | `GET/POST/DELETE /api/memory/*`, `POST /api/memory/search` | clawft-services |
| Memory list view | DataTable with key, namespace, tags, timestamp | ui/src/routes/memory.tsx |
| Semantic search | Search bar with threshold slider, namespace filter | ui/src/routes/memory.tsx |
| Memory write dialog | Key, value, namespace, tags, TTL input form | ui/src/components/ |

### Phase S2.4: Configuration Editor (Week 6) -- P1

| Deliverable | Description | Location |
|-------------|-------------|----------|
| Config read/write endpoints | `GET/PATCH /api/config`, `GET /api/config/validate` | clawft-services |
| Tabbed config sections | Agents, providers, channels, tools, routing, delegation tabs | ui/src/routes/config.tsx |
| Inline editing | JSON schema validation with react-hook-form + zod | ui/src/routes/config.tsx |
| Diff view | Show pending changes before save confirmation | ui/src/components/ |

### Phase S2.5: Cron + Channels (Week 6) -- P1

| Deliverable | Description | Location |
|-------------|-------------|----------|
| Cron CRUD endpoints | `GET/POST/DELETE /api/cron/*` | clawft-services |
| Cron dashboard | Job list, expression editor, next-fire preview, execution log | ui/src/routes/cron.tsx |
| Channel status view | Connection state per channel, message counts, restart button | ui/src/routes/channels.tsx |
| Channel routing visualization | Visual routing table (channel -> agent mapping) | ui/src/routes/channels.tsx |

### Phase S3.1: Delegation & Monitoring (Week 7) -- P1

| Deliverable | Description | Location |
|-------------|-------------|----------|
| Delegation monitor | Active delegations list: target (local/Claude/Flow), status, latency | ui/src/routes/delegation.tsx |
| Rule editor | Regex pattern, target selector, complexity threshold editor | ui/src/routes/delegation.tsx |
| Pipeline inspector | Real-time 6-stage pipeline visualization per session | ui/src/components/ |
| Token usage dashboard | Per-session, per-provider, per-model usage charts (recharts) | ui/src/routes/delegation.tsx |
| Cost tracker | Daily/weekly/monthly cost breakdown by provider | ui/src/routes/delegation.tsx |

### Phase S3.2: Advanced Canvas (Weeks 7-8) -- P2

| Deliverable | Description | Location |
|-------------|-------------|----------|
| Chart rendering | recharts/visx integration for Canvas chart elements | ui/src/components/canvas/ |
| Code editor element | Monaco or CodeMirror embedded in Canvas | ui/src/components/canvas/ |
| Form builder | Agent constructs multi-field forms with validation | ui/src/components/canvas/ |
| Canvas history | Undo/redo stack, replay timeline | ui/src/stores/canvas-store.ts |
| Canvas export | PNG screenshot, HTML export, JSON state export | ui/src/components/canvas/ |

### Phase S3.3: Mobile Responsive + PWA (Week 8) -- P2

| Deliverable | Description | Location |
|-------------|-------------|----------|
| Responsive sidebar | Drawer on mobile, collapsible on tablet | ui/src/components/layout/ |
| Mobile WebChat | Bottom-anchored input, swipe navigation | ui/src/routes/chat.tsx |
| PWA manifest | Service worker for offline dashboard shell | ui/public/ |
| Push notifications | Service worker + WS event bridge for browser notifications | ui/src/lib/ |

### Phase S3.4: Tauri Desktop Shell (Weeks 8-9) -- P2

| Deliverable | Description | Location |
|-------------|-------------|----------|
| Tauri project init | Tauri wraps `ui/dist/` in native window | ui/src-tauri/ |
| System tray icon | Agent status indicator in system tray | ui/src-tauri/ |
| Global hotkey | Cmd+Shift+W to toggle window visibility | ui/src-tauri/ |
| Auto-start gateway | Launch `weft gateway` on Tauri app start | ui/src-tauri/ |
| Spotlight integration | macOS Spotlight quick agent query | ui/src-tauri/ |
| Native notifications | Linux/Windows notification bridge | ui/src-tauri/ |

### Phase S3.5: Production Hardening (Week 9) -- P1

| Deliverable | Description | Location |
|-------------|-------------|----------|
| CSP headers | Content Security Policy + XSS protection on static file serving | clawft-services |
| Rate limiting | API endpoint rate limiting middleware | clawft-services |
| WS heartbeat | WebSocket heartbeat + dead connection cleanup | clawft-services |
| Error boundaries | Graceful degradation components with fallback UI | ui/src/components/ |
| E2E tests | Playwright tests for dashboard, WebChat, Canvas flows | ui/tests/ |
| Accessibility | axe-core audit + WCAG AA compliance fixes | ui/ |
| Tailscale auth | X-Tailscale-User header auth provider for remote access | clawft-services |
| Multi-user isolation | Per-user session isolation + permission scoping | clawft-services |

---

## 2.5 Internal Dependency Graph

### UI Phase Dependencies

```
S1.1 (Backend API)
  |
  +---> S1.3 (Core Views) -- needs real API endpoints (or MSW mocks)
  |       |
  |       +---> S2.1 (Live Canvas) -- extends WS transport from S1.1, builds on chat from S1.3
  |       |       |
  |       |       +---> S3.2 (Advanced Canvas) -- extends base Canvas elements
  |       |
  |       +---> S2.2 (Skill Browser) -- needs base DataTable/card patterns from S1.3
  |       +---> S2.3 (Memory Explorer) -- needs base DataTable patterns from S1.3
  |       +---> S2.4 (Config Editor) -- needs base form patterns from S1.3
  |       +---> S2.5 (Cron + Channels) -- needs base DataTable/status patterns from S1.3
  |       +---> S3.1 (Delegation Monitor) -- needs WS events + chart patterns
  |
  +---> S3.5 (Production Hardening) -- hardens backend layer from S1.1
  |
  +---> S3.4 (Tauri Desktop) -- wraps complete UI

S1.2 (Frontend Scaffolding)
  |
  +---> S1.3 (Core Views) -- needs layout shell, router, api-client, ws-client
  |
  +---> S3.3 (Mobile + PWA) -- adapts layout from S1.2
```

### External Workstream Dependencies

```
Main Sprint Workstream              UI Phase That Needs It
-------------------------------     ----------------------
B5  (shared tool registry)     ---> S1.1 (tool listing API needs ToolRegistry access)
C3  (skill loader)             ---> S2.2 (skill browser needs skill registry)
C4  (hot-reload)               ---> S2.2 (skill:reload WS events)
C6  (MCP skill exposure)       ---> S2.2 (skill tools visible in tool inspector)
H1  (per-agent workspace)      ---> S2.3 (memory API needs workspace directories)
H2  (HNSW VectorStore)         ---> S2.3 (semantic search API backend)
L1  (agent routing table)      ---> S2.5 (channel routing visualization data)
M1  (FlowDelegator)            ---> S3.1 (delegation status events)
M2  (flow_available detection) ---> S3.1 (runtime delegation target info)
D5  (record actual latency)    ---> S3.1 (pipeline inspector latency data)
D6  (thread sender_id)         ---> S3.1 (per-user cost attribution)
K4  (ClawHub registry)         ---> S2.2 (ClawHub search proxy)
```

---

## 3. Exit Criteria

### S1.1 Backend API Foundation

- [ ] Axum REST router compiles and serves `/api/agents`, `/api/sessions`, `/api/tools`
- [ ] Bearer token auth middleware rejects unauthenticated requests with 401
- [ ] `/api/auth/token` generates valid JWT/opaque token with configurable TTL
- [ ] WebSocket `/ws` accepts connections, supports `subscribe`/`unsubscribe` commands
- [ ] `weft ui` command starts gateway and opens browser with token parameter
- [ ] CORS middleware allows configurable origins (default: `localhost:5173`)
- [ ] Static file serving works from `ui/dist/` on disk or via `rust-embed`

### S1.2 Frontend Scaffolding

- [ ] `pnpm dev` starts Vite dev server on `:5173` with HMR
- [ ] `VITE_MOCK_API=true pnpm dev` runs fully without any backend
- [ ] MSW mock handlers return realistic fixture data for agents, sessions, tools
- [ ] `api-client.ts` attaches Bearer token to all requests
- [ ] `ws-client.ts` reconnects with exponential backoff on disconnect
- [ ] Sidebar navigation renders all route links
- [ ] `pnpm build` produces optimized `ui/dist/` under 200 KB gzipped
- [ ] Dockerfile builds and serves via nginx:alpine

### S1.3 Core Views

- [ ] Dashboard home displays agent count, channel status, recent sessions
- [ ] Agent list shows status badges; start/stop actions update via WS
- [ ] Agent detail form allows editing config and selecting model
- [ ] WebChat displays real-time streaming messages via WebSocket
- [ ] Tool call cards expand to show tool name, arguments, and result
- [ ] Session explorer lists sessions with search, sort, and export
- [ ] Tool registry lists all tools with expandable JSON Schema viewer
- [ ] Dark/light theme toggle persists across sessions
- [ ] Cmd+K command palette navigates to any view

### S2.1 Live Canvas

- [ ] `CanvasCommand` types defined in both Rust and TypeScript
- [ ] `/ws/canvas` handler routes commands between agents and frontend
- [ ] `render_ui` tool callable by agents to push UI elements
- [ ] `CanvasRenderer` renders text, button, input, image, code, table elements
- [ ] Button clicks and form submits route back to agent as tool results
- [ ] Canvas state persists across page refreshes via snapshot API
- [ ] Split view (Canvas + Chat) works with resizable panels
- [ ] Canvas renders 100 elements within 16ms (60fps target)

### S2.2 Skill Browser

- [ ] Installed skills display in card grid with name, version, tools provided
- [ ] ClawHub search returns results via proxy endpoint
- [ ] Install action triggers skill download + hot-reload notification
- [ ] Uninstall action removes skill and updates tool registry
- [ ] SKILL.md content renders as formatted markdown

### S2.3 Memory Explorer

- [ ] Memory list displays entries with key, namespace, tags, timestamp
- [ ] Semantic search returns results ranked by similarity score
- [ ] Namespace filter restricts search to selected namespace
- [ ] Memory write dialog creates entries with key, value, namespace, tags, TTL
- [ ] Delete action removes entry with confirmation dialog

### S2.4 Configuration Editor

- [ ] Config loads current resolved configuration into tabbed view
- [ ] Inline editing validates against JSON schema
- [ ] Diff view shows pending changes before save
- [ ] Save action sends PATCH and refreshes config display
- [ ] Validation endpoint catches invalid config before save

### S2.5 Cron + Channels

- [ ] Cron job list displays expression, next fire time, last run status
- [ ] Create/delete cron jobs via form with cron expression validation
- [ ] Channel status shows connection state with real-time WS updates
- [ ] Channel routing visualization maps channels to assigned agents

### S3.1 Delegation & Monitoring

- [ ] Delegation monitor shows active delegations with target, status, latency
- [ ] Rule editor creates/edits delegation rules with regex pattern matching
- [ ] Pipeline inspector visualizes 6-stage pipeline in real-time per session
- [ ] Token usage chart displays per-session, per-provider, per-model breakdown
- [ ] Cost tracker shows daily/weekly/monthly cost by provider

### S3.2 Advanced Canvas

- [ ] Charts render via recharts/visx within Canvas elements
- [ ] Code editor element (Monaco/CodeMirror) supports syntax highlighting
- [ ] Form builder allows agents to construct multi-field validated forms
- [ ] Undo/redo stack tracks Canvas history with replay capability
- [ ] Export produces PNG screenshot, HTML, and JSON state

### S3.3 Mobile Responsive + PWA

- [ ] Sidebar collapses to drawer on screens < 768px
- [ ] WebChat input anchors to bottom on mobile with swipe navigation
- [ ] PWA manifest enables Add to Home Screen
- [ ] Push notifications fire on key WS events (agent error, task complete)

### S3.4 Tauri Desktop Shell

- [ ] Tauri app launches and loads `ui/dist/` in native window
- [ ] System tray icon shows agent status (green/yellow/red)
- [ ] Cmd+Shift+W toggles window visibility globally
- [ ] `weft gateway` auto-starts on app launch if not already running
- [ ] Native notifications bridge WS events to OS notification center

### S3.5 Production Hardening

- [ ] CSP headers block inline scripts and restrict resource origins
- [ ] API rate limiting enforces per-IP request limits
- [ ] WebSocket heartbeat detects and cleans up dead connections
- [ ] Error boundaries catch React errors with graceful fallback UI
- [ ] Playwright E2E tests cover dashboard, WebChat, and Canvas flows
- [ ] axe-core audit passes with zero critical WCAG AA violations
- [ ] Tailscale auth provider validates X-Tailscale-User headers
- [ ] Multi-user sessions are isolated (no cross-user data leakage)

---

## 4. Security Requirements

### 4.1 Content Security Policy (CSP)

The static file serving layer MUST set CSP headers that:
- Block inline script execution (`script-src 'self'`)
- Restrict resource loading to same origin + configured CDN domains
- Disable `eval()` and dynamic code generation
- Allow WebSocket connections only to the configured backend origin

### 4.2 Cross-Site Scripting (XSS) Prevention

- All user-generated content (chat messages, memory values, tool results) MUST be rendered through React's default escaping (no `dangerouslySetInnerHTML` on untrusted input)
- Markdown rendering (SKILL.md, memory content) MUST use `rehype-sanitize` to strip dangerous HTML
- Canvas elements received from agents MUST be validated against a strict schema before rendering; arbitrary HTML/JS injection via Canvas commands is prohibited

### 4.3 CORS Configuration

- Backend CORS middleware MUST reject requests from origins not in `GatewayConfig.cors_origins`
- Default allowed origin: `http://localhost:5173` (Vite dev server only)
- Production deployments MUST configure explicit allowed origins
- Credentials (`Access-Control-Allow-Credentials`) MUST be restricted to allowed origins only

### 4.4 Authentication & Token Handling

- Auth tokens MUST NOT be stored in cookies (use `localStorage` or memory only to avoid CSRF)
- Tokens are sent exclusively via `Authorization: Bearer <token>` header
- Token TTL defaults to 24 hours; configurable in `GatewayConfig`
- `weft ui` one-time token in URL parameter MUST be consumed on first use (single-use, not replayable)
- Token stored in `~/.clawft/ui-token` MUST have `0600` file permissions
- API MUST return 401 on expired/invalid tokens without leaking token details

### 4.5 WebSocket Security

- WebSocket upgrade MUST validate Bearer token before completing handshake
- WebSocket connections MUST enforce same-origin or configured allowed origins
- Heartbeat interval (30s default) detects and terminates dead connections
- Per-connection message rate limiting prevents flooding
- Canvas commands received from server MUST be schema-validated before render
- Client-to-server commands MUST be validated against expected types (reject unknown command types)

### 4.6 API Security

- All `/api/*` routes require Bearer token authentication (except `/api/auth/token`)
- Rate limiting on all endpoints (configurable, default: 100 req/min per token)
- Request body size limits (default: 1 MB for API, 10 MB for file upload endpoints)
- Path parameters MUST be validated (UUID format for IDs, alphanumeric for names)
- Config PATCH endpoint MUST NOT allow overwriting security-sensitive fields (auth tokens, secret keys) via the UI

### 4.7 Multi-User Security (S3.5)

- Tailscale auth relies on trusted `X-Tailscale-User-*` headers; backend MUST verify these headers originate from the Tailscale proxy (not forgeable by clients)
- Per-user permission scoping prevents users from accessing other users' sessions or memory
- Audit logging records all config changes with user identity

---

## 5. Risks

| Risk | Likelihood | Impact | Score | Mitigation |
|------|-----------|--------|-------|------------|
| Backend API not ready for S1 frontend work | Medium | Low | **3** | UI is standalone-first. Full MSW mock layer enables frontend development and testing without any backend. Mock handlers are maintained alongside real API type contracts. |
| WebSocket protocol changes mid-sprint | Medium | Medium | **6** | Define protocol types in shared `types.ts` (frontend) and `clawft-types` (Rust). Version WS message schema from day one. Breaking changes require mock handler updates. |
| Canvas complexity explosion beyond MVP scope | High | Medium | **8** | MVP Canvas supports text, button, input, code, image, table only. Charts, Monaco editor, and form builder deferred to S3.2 (P2). Strict element type allowlist enforced in renderer. |
| External workstream dependencies (C3/H1/M1) not ready on time | Medium | Medium | **6** | Each UI phase that depends on external workstreams has MSW mock fallbacks. Skill browser can show mock data until C3 lands. Memory explorer can use HashEmbedder until H2 lands. Delegation monitor can show mock events until M1 lands. |
| Bundle size exceeds 200 KB budget | Medium | Low | **4** | Dynamic imports for Canvas, charts (recharts), code editor (Monaco). Measure with `vite-bundle-analyzer` in CI. Tree-shaking enforced via ES module imports. |
| XSS via agent-generated Canvas commands | Low | Critical | **6** | Canvas commands validated against strict schema. No raw HTML rendering. React default escaping for all text content. `rehype-sanitize` for markdown. CSP blocks inline scripts. |
| Tauri cross-platform audio/notification issues | Medium | Low | **4** | Tauri handles window management only. Voice is a separate native daemon (G5 workstream). Notifications use OS-native APIs with graceful fallback to in-app toast. |
| WebSocket connection instability on unreliable networks | Medium | Medium | **6** | Reconnecting client with exponential backoff (1s, 2s, 4s, ... 30s max). Missed events recovered via REST API polling on reconnect. Connection status indicator in UI header. |
| shadcn/ui or Tailwind breaking changes during sprint | Low | Medium | **3** | Pin all dependency versions in `pnpm-lock.yaml`. Use `components.json` lock for shadcn. Only upgrade dependencies between sprints, not during. |
| Multi-user auth bypass via header forgery (Tailscale) | Low | Critical | **5** | Backend MUST verify `X-Tailscale-User-*` headers originate from Tailscale proxy (check source IP or use Tailscale HTTPS cert verification). Disable header auth when not behind Tailscale proxy. |
