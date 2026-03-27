# S1.3 Core Views - Development Notes

## Completed: 2026-02-24

## Summary

Replaced all five route stubs with fully functional views for the ClawFT UI dashboard. Added shared UI components, TypeScript types, Zustand stores, and extended the API client.

## Files Created

### Shared UI Components (`ui/src/components/ui/`)
- `button.tsx` - Button with 5 variants (default, secondary, outline, ghost, destructive) and 3 sizes (sm, md, lg)
- `card.tsx` - Card, CardHeader, CardTitle, CardContent, CardFooter
- `badge.tsx` - Badge with 5 variants (default, secondary, destructive, outline, success)
- `skeleton.tsx` - Animated skeleton loading placeholder
- `separator.tsx` - Horizontal/vertical separator
- `tooltip.tsx` - Hover tooltip using local state (no external library)
- `dialog.tsx` - Modal dialog with Dialog, DialogHeader, DialogTitle, DialogFooter. Escape key closes.

### Types (`ui/src/lib/types.ts`)
- AgentStatus, AgentSummary, AgentDetail
- SessionSummary, SessionDetail
- ChatMessage, ToolCallInfo
- ToolInfo, SystemHealth

### Stores (`ui/src/stores/`)
- `agent-store.ts` - WS connection state, notifications queue
- `theme-store.ts` - Light/dark theme with localStorage persistence via zustand/persist

### Utilities (`ui/src/lib/utils.ts`)
- `cn()` - clsx-based className merger
- `formatRelativeTime()` - Human-readable relative timestamps
- `truncate()` - String truncation with ellipsis

## Files Modified

### API Client (`ui/src/lib/api-client.ts`)
- Rewired to use shared types from `types.ts`
- Added: `agents.start()`, `agents.stop()`
- Added: `sessions.create()`, `sessions.export()`
- Added: `chat.send()`
- Added: `system.health()`

### MainLayout (`ui/src/components/layout/MainLayout.tsx`)
- Theme toggle button (light/dark) with html class sync
- WebSocket connection status indicator (green/red dot)
- Ctrl+K command palette placeholder
- Active link highlighting
- Sidebar collapse preserves icons

### Route Views

**Dashboard (`routes/index.tsx`)**
- 4 summary cards: Agents, Sessions, Tools, System Health
- Recent sessions list (5 most recent)
- Skeleton loading states
- Error banner for API failures

**Agents (`routes/agents.tsx`)**
- Card grid layout with status badges (green=running, gray=stopped, red=error)
- Start/Stop mutation buttons with loading states
- 10s polling interval for live updates
- Empty state handling

**Chat (`routes/chat.tsx`)**
- Session sidebar with selection
- Message thread with role-based styling (user=blue, assistant=gray, system=yellow, tool=purple)
- Input area with Enter-to-send
- WebSocket streaming support (stream_chunk/stream_done events)
- Optimistic updates on send
- Auto-scroll to bottom
- Tool call display within messages

**Sessions (`routes/sessions.tsx`)**
- Table layout with expand/collapse per row
- Columns: Session Key, Agent, Messages, Last Updated
- Expanded view shows message preview list
- Export JSON button (downloads as file)
- Delete button with confirmation
- 15s polling interval

**Tools (`routes/tools.tsx`)**
- Grid/List view toggle
- Search filter by name and description
- Expandable JSON schema viewer
- Tool count badge

## Dependencies Added
- `sonner` (for future toast notifications; not yet wired)

## Build Verification
- `tsc -b && vite build` passes cleanly
- 183 modules transformed
- Output: 344.75 kB JS (106.07 kB gzipped), 25.73 kB CSS (5.58 kB gzipped)

## Design Decisions
- All UI components are self-contained with Tailwind CSS classes (no @shadcn/ui dependency)
- Used `clsx` (already installed) for className composition instead of `tailwind-merge`
- Theme persistence via zustand/persist to localStorage
- WebSocket connection managed in MainLayout, global state in agent-store
- Command palette is a placeholder (Ctrl+K opens, Escape closes) for future implementation
- API types are separate from the API client for cleaner imports
