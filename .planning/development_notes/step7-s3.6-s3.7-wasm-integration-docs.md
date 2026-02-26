# Step 7: S3.6 Browser WASM Integration + S3.7 Documentation

## Completed: 2026-02-24

## S3.6 Deliverables

### 1. BackendAdapter Interface (`ui/src/lib/backend-adapter.ts`)
- Defined `BackendMode` type: `"axum" | "wasm" | "mock"`
- Defined `BackendCapabilities` interface with flags: channels, cron, delegation, multiUser, skillInstall, realtime, monitoring, ready
- Defined domain types: `AgentInfo`, `SessionInfo`, `ToolInfo`, `ChatMessage`, `ToolCallRecord`, `MemoryEntry`
- Defined `BackendAdapter` interface with all method signatures:
  - Lifecycle: `init()`, `dispose()`
  - Agents: `listAgents()`, `getAgent()`, `startAgent()`, `stopAgent()`
  - Sessions: `listSessions()`, `getSessionMessages()`, `deleteSession()`
  - Chat: `sendMessage()`, `onMessage()`
  - Tools: `listTools()`, `getToolSchema()`
  - Memory: `listMemory()`, `searchMemory()`, `writeMemory()`, `deleteMemory()`
  - Config: `getConfig()`, `updateConfig?()`
  - Events: `subscribe?()`, `unsubscribe?()`, `onEvent?()`

### 2. AxumAdapter (`ui/src/lib/adapters/axum-adapter.ts`)
- Implements `BackendAdapter` wrapping existing `api-client.ts` and `ws-client.ts`
- All capabilities enabled
- Delegates agent, session, chat, tool, memory, config operations to existing API client
- Wires WebSocket events to message and event callbacks
- Handles auth token setup on construction

### 3. WasmAdapter (`ui/src/lib/adapters/wasm-adapter.ts`)
- Implements `BackendAdapter` for in-browser WASM mode
- Dynamic import of wasm-bindgen JS glue with progress callbacks
- Limited capabilities (all server-only features disabled)
- Single "browser-agent" always present
- In-memory session storage via `Map<string, ChatMessage[]>`
- In-memory memory store
- Browser-safe tool subset (read_file, write_file, edit_file, list_directory, web_search, web_fetch, memory_read, memory_write)

### 4. WASM Loader (`ui/src/lib/wasm-loader.ts`)
- `LoadPhase` type: download -> compile -> init -> ready | error
- `LoadProgress` interface with phase, percent, message
- `checkWasmCapabilities()` function for pre-flight checks
- `loadWasmModule()` with streaming compilation support and progress tracking

### 5. Feature Detection (`ui/src/lib/feature-detect.ts`)
- `FeatureReport` interface: webAssembly, opfs, webCrypto, serviceWorker, indexedDb, fetchStreaming
- `detectFeatures()` async function
- `canRunWasmMode()` returning errors/warnings
- `preferredMode()` auto-detection from URL params, env vars

### 6. Mode Context (`ui/src/lib/mode-context.tsx` + `ui/src/lib/mode-store.ts` + `ui/src/lib/use-backend.ts`)
- `ModeContext` and `ModeContextValue` in `mode-store.ts` (separated for react-refresh)
- `ModeProvider` component in `mode-context.tsx`:
  - Reads mode from URL param `?mode=`, then `VITE_BACKEND_MODE`, then default "axum"
  - Auto-detection mode tries Axum health endpoint, falls back to WASM
  - Loading screen with progress bar during WASM initialization
- `useBackend()` hook in `use-backend.ts` returning adapter, mode, capabilities
- `useCapability()` hook for checking individual capability flags
- App.tsx wrapped with ModeProvider

### 7. Browser Config Component (`ui/src/components/wasm/browser-config.tsx`)
- Provider selector: Anthropic (direct), OpenAI (proxy), Ollama (local), LM Studio (local), Custom
- API key input with Web Crypto AES-256-GCM encryption
- Model selector per provider
- CORS proxy URL input for providers that need it
- Custom base URL for custom providers
- IndexedDB storage for encrypted configuration
- Error handling and loading states

### 8. Vite Config Update (`ui/vite.config.ts`)
- Added `optimizeDeps.exclude` for `clawft_wasm`
- Set `build.target` to `es2022` for top-level await
- Added `assetsInclude` for `.wasm` files
- Added COOP/COEP headers for dev server

### 9. Route Gating (`ui/src/components/layout/MainLayout.tsx`)
- Added `requiresCap` field to NavItem interface
- Gated routes: cron (requires `cron`), channels (requires `channels`), delegation (requires `delegation`), monitoring (requires `monitoring`)
- Added "Browser Mode" badge in sidebar when in WASM mode
- WS status indicator shows "WASM" in browser mode

## S3.7 Documentation

### 1. Developer Guide (`docs/ui/developer-guide.md`)
- Project structure with directory tree
- Tech stack table
- Getting started instructions
- Environment variables reference
- Adding routes, components, API endpoints walkthrough
- State management patterns (Zustand vs TanStack Query vs BackendAdapter)
- MSW mock patterns
- Code style guidelines

### 2. API Reference (`docs/ui/api-reference.md`)
- All REST endpoints: auth, agents, sessions, tools, skills, memory, config, cron, channels, delegation, monitoring, voice, health
- Method, path, request/response types, example JSON
- WebSocket protocol: subscribe/unsubscribe, event types

### 3. Browser Mode Guide (`docs/ui/browser-mode.md`)
- What browser mode is and use cases
- Browser requirements table (Chrome 102+, Firefox 111+, Safari 15.2+)
- Building WASM module instructions
- Deploying browser-only UI
- Per-provider CORS setup: Anthropic (direct), OpenAI (proxy), Ollama (local), LM Studio (local)
- API key security considerations
- Feature limitations vs Axum mode table
- Troubleshooting: WASM loading, CORS errors, OPFS, IndexedDB

### 4. Deployment Guide (`docs/ui/deployment.md`)
- Docker deployment with Dockerfile and nginx config
- CDN deployment (S3, Cloudflare, Vercel)
- Reverse proxy configs (nginx, Caddy)
- Single binary (rust-embed)
- Tauri packaging
- Browser-only mode
- Environment variables reference

## Build Verification

- `tsc -b` -- zero type errors
- `npm run build` -- succeeds
- `npm run lint` -- zero new errors (only pre-existing issues in talk-overlay.tsx and form-element.tsx)

## Architecture Decisions

1. **Separated context from hooks** -- `ModeContext` lives in `mode-store.ts`, `ModeProvider` in `mode-context.tsx`, and hooks in `use-backend.ts`. This satisfies `react-refresh/only-export-components` rule.

2. **erasableSyntaxOnly compatibility** -- No TypeScript parameter properties used; all class fields are explicitly declared and assigned in constructor bodies.

3. **BackendAdapter optional methods** -- Methods like `updateConfig`, `subscribe`, `unsubscribe`, `onEvent` are optional (`?`) on the interface since WASM mode does not support them.

4. **No changes to existing api-client.ts** -- The AxumAdapter wraps the existing `api` export without modifying the underlying module, ensuring backward compatibility.

5. **Capabilities-based route gating** -- Navigation items declare a `requiresCap` field. The MainLayout filters items based on `capabilities` from `useBackend()`. This is extensible -- new capability-gated routes only need the flag.
