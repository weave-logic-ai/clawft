# ClawFT UI Development Plan

> Extracted from `improvements.md` Workstream K. This item requires dedicated design,
> UI/UX research, wireframing, and prototyping before implementation begins.

---

## K1. Web Dashboard + Live Canvas

**Type:** Feature
**Status:** Pre-design -- needs UI/UX work before coding starts

### Web Dashboard (`weft ui`)

Stack: Axum + Leptos.

Core views:
- Agent management (list, configure, start/stop)
- Conversation view (real-time message stream, session history)
- Skill browser (installed skills, ClawHub search, install/uninstall)
- Tool registry inspector
- Session/memory explorer
- Configuration editor

### Live Canvas (OpenClaw parity)

Agent-driven visual workspace (A2UI pattern). The agent can dynamically render UI
components (buttons, forms, images, code blocks, charts) into a web canvas via WebSocket.
Users interact with rendered elements; clicks/inputs route back to the agent loop as tool
results. Canvas renders in browser, and can be embedded in native shells (K6).

### K6. Cross-platform Native Shells

**Deps:** K1 (this document)

Thin native wrappers around the web dashboard/canvas:
- **macOS:** WebKit-based app (via `tauri` or `wry`). Supports Voice Wake (G5) as background daemon, system tray icon, Spotlight integration.
- **Linux:** GTK/WebKit via Tauri. System tray, PipeWire audio integration.
- **Windows:** WebView2 via Tauri. System tray, notification integration.
- **iOS:** SwiftUI wrapper with Canvas in WKWebView. Push notifications for agent messages.
- **Android:** Jetpack Compose wrapper with WebView. FCM notifications.

Target: single `tauri` codebase for desktop, separate mobile shells that embed the Axum server as a library.

---

## Pre-Implementation Requirements

Before any code is written, the following must be completed:

1. **UI/UX design** -- wireframes for all core views
2. **Component library selection** -- Leptos component ecosystem, styling approach (Tailwind, custom CSS)
3. **WebSocket protocol design** -- message format for Live Canvas rendering commands and user interaction events
4. **State management architecture** -- how dashboard state syncs with the agent runtime
5. **Authentication & access control** -- who can access the dashboard, multi-user scenarios
6. **Accessibility audit** -- WCAG compliance plan
7. **Mobile-first vs desktop-first** -- responsive strategy
8. **Performance budget** -- target load times, bundle size, WebSocket throughput

---

## Dependencies

- Workstream C (Plugin system) -- skill browser needs the plugin/skill registry
- Workstream H (Memory) -- memory explorer needs the workspace/vector store APIs
- Workstream G (Voice) -- native shells need Voice Wake integration points
