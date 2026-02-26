# Step 2 - S1.2: Frontend Scaffolding

**Date**: 2026-02-24
**Phase**: S1.2 (UI Workstream)
**Branch**: feature/three-workstream-implementation

## Summary

Created a standalone React/TypeScript frontend scaffold in the `ui/` directory using Vite as the build tool. The UI is fully independent from the Rust backend and builds standalone.

## Technology Stack

| Layer | Technology | Version |
|-------|-----------|---------|
| Build Tool | Vite | 7.x |
| Framework | React | 19.x |
| Language | TypeScript | 5.x |
| Routing | @tanstack/react-router | latest |
| Data Fetching | @tanstack/react-query | latest |
| State Management | zustand | latest |
| Styling | Tailwind CSS v4 | latest (via @tailwindcss/vite) |
| Icons | lucide-react | latest |
| API Mocking | msw | latest (dev) |
| Utilities | clsx | latest |

## Files Created

### Configuration
- `ui/vite.config.ts` - Vite config with Tailwind plugin, dev server proxy to Rust backend (port 18789)
- `ui/.gitignore` - Updated to include `.vite/` directory

### Library Layer
- `ui/src/lib/api-client.ts` - Typed HTTP client with auth token management, matching Rust API types
- `ui/src/lib/ws-client.ts` - WebSocket client with auto-reconnect, pub/sub pattern, exponential backoff

### Layout
- `ui/src/components/layout/MainLayout.tsx` - Shell layout with collapsible sidebar navigation

### Route Pages (stubs)
- `ui/src/routes/index.tsx` - Dashboard page
- `ui/src/routes/agents.tsx` - Agents page (placeholder for S1.3)
- `ui/src/routes/chat.tsx` - Chat page (placeholder for S1.3)
- `ui/src/routes/sessions.tsx` - Sessions page (placeholder for S1.3)
- `ui/src/routes/tools.tsx` - Tools page (placeholder for S1.3)

### App Wiring
- `ui/src/App.tsx` - Router + QueryClient setup with all routes
- `ui/src/main.tsx` - Entry point (cleaned up from Vite defaults)
- `ui/src/index.css` - Tailwind CSS v4 import only

### Removed
- `ui/src/App.css` - Replaced by Tailwind CSS

## Dev Server Proxy Configuration

The Vite dev server proxies API and WebSocket requests to the Rust backend:
- `/api/*` -> `http://localhost:18789`
- `/ws` -> `ws://localhost:18789`

## API Client Design

The `api-client.ts` module provides a typed interface matching the Rust HTTP API endpoints:
- `api.agents.list()` / `api.agents.get(name)`
- `api.sessions.list()` / `api.sessions.get(key)` / `api.sessions.delete(key)`
- `api.tools.list()` / `api.tools.schema(name)`
- `api.auth.createToken()`

Auth tokens are stored in localStorage under `clawft-token`.

## WebSocket Client Design

The `ws-client.ts` module provides:
- Auto-reconnect with exponential backoff (1s to 30s)
- Event-based pub/sub (`on`, `emit`)
- Topic subscribe/unsubscribe
- Wildcard listener (`*`) for all messages

## Build Verification

- TypeScript type-check: PASS (zero errors)
- Vite production build: PASS (produces `ui/dist/`)
- Bundle size: ~303 KB JS (95 KB gzipped), ~6.7 KB CSS (2.2 KB gzipped)

## Next Steps (S1.3)

- Implement agent list/detail views with react-query hooks
- Build WebChat component with streaming support
- Create session explorer with message viewer
- Add tool registry browser
- Wire up WebSocket real-time updates
- Add shadcn/ui component library (requires manual setup)
