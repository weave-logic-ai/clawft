# UI Developer Guide

> ClawFT Dashboard -- React + TypeScript + Vite

## Tech Stack

| Layer | Technology | Version |
|-------|-----------|---------|
| Build | Vite | 7.x |
| Framework | React | 19.x |
| Language | TypeScript | 5.9 |
| Styling | Tailwind CSS v4 | 4.x |
| Routing | TanStack Router | 1.x |
| Data fetching | TanStack Query | 5.x |
| State management | Zustand | 5.x |
| Mocking | MSW (Mock Service Worker) | 2.x |
| Icons | Lucide React | 0.575+ |

## Project Structure

```
ui/
  src/
    App.tsx                    # Root component, router setup, ModeProvider
    main.tsx                   # Entry point, MSW initialization
    index.css                  # Tailwind v4 base styles
    lib/
      api-client.ts            # Fetch wrapper with Bearer token auth
      ws-client.ts             # Reconnecting WebSocket client
      types.ts                 # Shared TypeScript types (domain models)
      utils.ts                 # Utility functions (cn, formatRelativeTime, etc.)
      backend-adapter.ts       # BackendAdapter interface + domain types
      mode-context.tsx         # ModeProvider component (adapter initialization)
      mode-store.ts            # ModeContext and ModeContextValue (shared context)
      use-backend.ts           # useBackend() and useCapability() hooks
      feature-detect.ts        # Browser feature detection for WASM mode
      wasm-loader.ts           # WASM module loader with progress tracking
      adapters/
        axum-adapter.ts        # BackendAdapter implementation for Axum REST/WS
        wasm-adapter.ts        # BackendAdapter implementation for in-browser WASM
    components/
      ui/                      # Reusable UI primitives (button, card, badge, etc.)
      layout/
        MainLayout.tsx         # Sidebar navigation, theme toggle, WS status
      canvas/                  # Canvas renderer and element types
      voice/                   # Voice UI components (status bar, talk overlay)
      wasm/
        browser-config.tsx     # WASM mode configuration (provider, API key, model)
    routes/
      index.tsx                # Dashboard home
      agents.tsx               # Agent list and management
      canvas.tsx               # Canvas workspace
      chat.tsx                 # WebChat interface
      sessions.tsx             # Session browser
      tools.tsx                # Tool registry
      skills.tsx               # Skill marketplace
      memory.tsx               # Memory store viewer
      config.tsx               # System configuration
      cron.tsx                 # Cron job scheduler
      channels.tsx             # Channel status (Axum only)
      delegation.tsx           # Delegation monitoring (Axum only)
      monitoring.tsx           # Token usage and cost dashboards (Axum only)
      voice.tsx                # Voice settings
    stores/
      agent-store.ts           # WebSocket connection state, notifications
      theme-store.ts           # Light/dark theme preference
      canvas-store.ts          # Canvas element state
      skills-store.ts          # Skills state
      memory-store.ts          # Memory entries state
      config-store.ts          # Configuration state
      cron-store.ts            # Cron jobs state
      channels-store.ts        # Channel status state
      voice-store.ts           # Voice UI state
      delegation-store.ts      # Delegation monitoring state
      monitoring-store.ts      # Token usage and cost state
    mocks/
      handlers.ts              # MSW request handlers for all API endpoints
  public/                      # Static assets (WASM binary placed here for browser mode)
  vite.config.ts               # Vite configuration with proxy and WASM support
  tsconfig.json                # TypeScript project references
  tsconfig.app.json            # App TypeScript config (strict, ES2022)
  tsconfig.node.json           # Node TypeScript config (Vite config files)
  package.json                 # Dependencies and scripts
```

## Getting Started

### Prerequisites

- Node.js 20+
- pnpm or npm

### Install and Run

```bash
cd ui
npm install
npm run dev          # Starts Vite dev server at http://localhost:5173
```

The dev server proxies `/api` requests to `http://localhost:18789` (the Axum backend) and `/ws` WebSocket connections to `ws://localhost:18789`.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `VITE_API_URL` | `""` (empty = same origin) | Base URL for the Axum backend |
| `VITE_BACKEND_MODE` | `"axum"` | Backend mode: `axum`, `wasm`, `auto`, `mock` |
| `VITE_MOCK_API` | `"false"` | Enable MSW mocks (no backend required) |

### Mock Mode (No Backend)

```bash
VITE_MOCK_API=true npm run dev
```

This starts MSW, which intercepts all `/api/*` requests and returns mock data defined in `src/mocks/handlers.ts`.

## Adding a New Route

1. Create a route component in `src/routes/my-feature.tsx`:

```tsx
export function MyFeaturePage() {
  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold">My Feature</h1>
    </div>
  );
}
```

2. Register it in `src/App.tsx`:

```tsx
import { MyFeaturePage } from './routes/my-feature';

const myFeatureRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/my-feature',
  component: MyFeaturePage,
});

// Add to routeTree
const routeTree = rootRoute.addChildren([
  // ...existing routes
  myFeatureRoute,
]);
```

3. Add a nav item in `src/components/layout/MainLayout.tsx`:

```tsx
const navItems: NavItem[] = [
  // ...existing items
  { path: "/my-feature", label: "My Feature", icon: "F" },
  // Optionally gate behind a capability:
  { path: "/my-feature", label: "My Feature", icon: "F", requiresCap: "channels" },
];
```

## Adding a New API Endpoint

The typical workflow for adding a new backend endpoint to the UI:

1. **Rust handler** -- Add the Axum handler in `crates/clawft-services/src/api/`
2. **TypeScript types** -- Add request/response types in `src/lib/types.ts`
3. **API client method** -- Add to the appropriate namespace in `src/lib/api-client.ts`
4. **MSW mock handler** -- Add a mock in `src/mocks/handlers.ts`
5. **BackendAdapter method** -- If the operation is adapter-aware, add to `BackendAdapter` interface and both adapters
6. **React hook or store** -- Create a Zustand store or use TanStack Query for data fetching
7. **Component** -- Build the UI component that consumes the data

## State Management Patterns

### When to Use Zustand

- WebSocket-driven state that updates in real-time
- UI state shared across components (theme, sidebar collapse)
- State that needs imperative updates (e.g., `addNotification`)

```tsx
import { create } from "zustand";

interface MyStore {
  items: Item[];
  setItems: (items: Item[]) => void;
  addItem: (item: Item) => void;
}

export const useMyStore = create<MyStore>((set) => ({
  items: [],
  setItems: (items) => set({ items }),
  addItem: (item) => set((s) => ({ items: [...s.items, item] })),
}));
```

### When to Use TanStack Query

- Server state fetched via REST API
- Data that benefits from caching, refetching, and stale-while-revalidate
- Paginated or infinite scrolling data

### When to Use BackendAdapter

All new data-fetching code should go through `useBackend()` to ensure compatibility with both Axum and WASM modes:

```tsx
import { useBackend } from "../lib/use-backend";

function MyComponent() {
  const { adapter } = useBackend();
  const [data, setData] = useState<AgentInfo[]>([]);

  useEffect(() => {
    adapter.listAgents().then(setData).catch(console.error);
  }, [adapter]);
}
```

## MSW Mock Patterns

All mocks are in `src/mocks/handlers.ts`. To add a new mock:

```tsx
import { http, HttpResponse } from "msw";

export const handlers = [
  // ...existing handlers

  http.get("/api/my-feature", () =>
    HttpResponse.json([
      { id: "1", name: "Example" },
    ])
  ),

  http.post("/api/my-feature", async ({ request }) => {
    const body = await request.json();
    return HttpResponse.json({ id: "new", ...body });
  }),
];
```

## Code Style

### TypeScript

- Strict mode enabled (`strict: true`)
- No unused locals or parameters (`noUnusedLocals`, `noUnusedParameters`)
- `erasableSyntaxOnly: true` -- no TypeScript parameter properties, use explicit field declarations
- Use `import type` for type-only imports (enforced by `verbatimModuleSyntax`)
- ES2022 target -- top-level await, private class fields supported

### React

- Function components only (no class components)
- Hooks follow the Rules of Hooks
- Custom hooks prefixed with `use`

### Tailwind CSS v4

- Utility-first styling via Tailwind classes
- `cn()` utility from `src/lib/utils.ts` for conditional class merging
- Dark mode via `.dark` class on `<html>`

### File Naming

- Components: PascalCase (`MainLayout.tsx`)
- Hooks/stores: kebab-case (`use-backend.ts`, `agent-store.ts`)
- Types/utils: kebab-case (`types.ts`, `api-client.ts`)
