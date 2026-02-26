# Step 5: S2.2-S2.5 Advanced Views

## Summary

Implemented 5 new pages for the ClawFT UI dashboard: Skill Browser, Memory Explorer, Configuration Editor, Cron Dashboard, and Channel Status. All pages follow the established patterns from S1.3 core views (Zustand stores, TanStack Router, Tailwind CSS v4 styling, api-client integration).

## Completed Tasks

### S2.2: Skill Browser (/skills)
- **Route**: `ui/src/routes/skills.tsx`
- **Store**: `ui/src/stores/skills-store.ts`
- Card grid of installed skills with name, version, description, status badge
- Debounced search input for filtering installed skills
- "Browse Registry" dialog for searching ClawHub registry
- Install/Uninstall buttons with loading states
- Registry results show name, description, version, author, stars, signed status

### S2.3: Memory Explorer (/memory)
- **Route**: `ui/src/routes/memory.tsx`
- **Store**: `ui/src/stores/memory-store.ts`
- DataTable of memory entries (key, namespace, tags, value preview, updated_at)
- Namespace filter dropdown
- Tag filter chips with toggle selection
- Semantic search input with debounce (400ms)
- Similarity threshold slider (0.0-1.0), results show similarity scores
- "New Entry" dialog with key, value, namespace, tags fields
- Delete button per entry

### S2.4: Configuration Editor (/config)
- **Route**: `ui/src/routes/config.tsx`
- **Store**: `ui/src/stores/config-store.ts`
- Tabbed interface: General, Agents, Providers, Channels, Gateway
- Form inputs per tab: text, number, toggle switches
- Change summary diff showing path, old value, new value
- Save button with loading state, Reset button
- Deep clone/compare for change detection
- API keys displayed as boolean flags only (never exposed)

### S2.5: Cron + Channels (/cron, /channels)
- **Cron Route**: `ui/src/routes/cron.tsx`
- **Cron Store**: `ui/src/stores/cron-store.ts`
- DataTable of cron jobs (name, schedule, status, last_run, next_run)
- Create dialog with cron expression input
- Enable/Disable toggle, "Run Now" button, Delete button
- Next-fire preview: computes next 5 fire times from cron expression
- Inline schedule preview on click

- **Channels Route**: `ui/src/routes/channels.tsx`
- **Channels Store**: `ui/src/stores/channels-store.ts`
- Card grid of channels with type label and status badge
- Real-time WS subscription to "channels" topic for status updates
- Message count stats, last activity, routing target
- Visual routing indicator per card (channel -> agent)

### Integration
- **App.tsx**: Added 5 new routes (skillsRoute, memoryRoute, configRoute, cronRoute, channelsRoute)
- **MainLayout.tsx**: Added 5 nav items (Skills, Memory, Config, Cron, Channels)
- **api-client.ts**: Added API methods for skills, memory, config, cron, channels
- **types.ts**: Added SkillData, RegistrySkill, MemoryEntry, ConfigData, CronJob, ChannelStatus

### MSW Mocks
- **`ui/src/mocks/handlers.ts`**: Created with mock handlers for all new endpoints
  - Skills: GET list, POST install, DELETE uninstall, GET registry search
  - Memory: GET list, GET search, POST create, DELETE entry
  - Config: GET config, PUT config
  - Cron: GET list, POST create, PUT update, DELETE, POST run
  - Channels: GET list

## Build Verification

- TypeScript: `npx tsc --noEmit` -- PASS (no errors)
- Production build: `npm run build` -- PASS
  - 204 modules transformed
  - JS bundle: 387.83 kB (114.10 kB gzip)
  - CSS bundle: 30.18 kB (6.22 kB gzip)
  - Build time: 2.17s

## Files Created/Modified

### New Files (13)
- `ui/src/stores/skills-store.ts`
- `ui/src/stores/memory-store.ts`
- `ui/src/stores/config-store.ts`
- `ui/src/stores/cron-store.ts`
- `ui/src/stores/channels-store.ts`
- `ui/src/routes/skills.tsx`
- `ui/src/routes/memory.tsx`
- `ui/src/routes/config.tsx`
- `ui/src/routes/cron.tsx`
- `ui/src/routes/channels.tsx`
- `ui/src/mocks/handlers.ts`

### Modified Files (4)
- `ui/src/lib/types.ts` -- Added 6 new interfaces
- `ui/src/lib/api-client.ts` -- Added 5 new API namespace groups
- `ui/src/App.tsx` -- Added 5 routes + imports
- `ui/src/components/layout/MainLayout.tsx` -- Added 5 nav items

## Design Decisions

1. **Zustand over TanStack Query for stores**: The Skill Browser, Memory, Config, Cron, and Channel stores use Zustand directly (matching existing canvas-store pattern) rather than TanStack Query. This keeps the API interaction pattern consistent with the store-based architecture while TanStack Query remains available for simpler fetch-and-cache patterns.

2. **Debounced search**: Both Skills and Memory pages debounce search input (300ms and 400ms respectively) to avoid excessive API calls.

3. **Config deep clone**: The config editor uses JSON-based deep clone for simplicity and correctness. The change summary computes diffs recursively.

4. **Cron expression parser**: A simplified client-side parser computes next fire times for preview. It handles `*`, `*/N`, and comma-separated values. More complex expressions (ranges, L, W, etc.) would need a full cron library.

5. **Channel WS integration**: Channels page subscribes to the "channels" WebSocket topic on mount and updates the store on `channel_status` events. This uses the existing `wsClient.subscribe()` API.

6. **No new UI primitives**: All pages reuse the existing component library (Card, Badge, Button, Skeleton, Dialog, Separator). Form inputs use standard HTML elements with Tailwind classes matching the established design system.
