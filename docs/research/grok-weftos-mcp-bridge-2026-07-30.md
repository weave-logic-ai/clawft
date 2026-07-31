# Research: Grok CLI as WeftOS MCP client

**Date:** 2026-07-30  
**Decision:** [ADR-075](../adr/adr-075-grok-weftos-mcp-client-bridge.md)  
**Operator guide:** [docs/guides/grok-weftos-mcp.md](../guides/grok-weftos-mcp.md)

## Question

Given WeftOS’s MCP architecture, can Grok Build connect to a WeftOS instance as a client/agent and work within it?

## Answer (short)

**Yes.** Primary path is **Grok = MCP client**, **`weft mcp-server` = MCP server**. Same outbound bridge already documented for Claude Code. Product work is L1 docs/config (done in ADR-075 G0 artifacts), then curated control plane, `WindowIntent` tools, remote HTTP serve, and session capability.

## Architecture (condensed)

```
Grok Build ──stdio MCP──► weft mcp-server ──middleware──► tools / skills
                                              └── future: WindowIntent, agents.*
```

| Layer | Exists? |
|-------|---------|
| `McpServerShell` + stdio | Yes |
| Claude bridge docs | Yes |
| Grok config + guide | Yes (this batch) |
| Curated control profile | Ticketed |
| HTTP MCP listen | Ticketed |
| Session capability tokens | Ticketed |

## Separation of concerns

| Surface | Role |
|---------|------|
| Grok → `weftos` MCP | Drive WeftOS tools / future workspace |
| Grok → `ruflo` MCP | Swarm/orchestration (host) |
| ADR-074 xAI Voice | Speech path, not CLI MCP |
| WeftOS → external MCP | Inbound client (`weft mcp add`) |

## Related Plane

| WEFT | Phase |
|------|-------|
| WEFT-692 | G0 docs/config |
| WEFT-693 | G1 curated profile |
| WEFT-694 | G2 control tools |
| WEFT-695 | G3 WindowIntent |
| WEFT-696 | G4 remote HTTP MCP |
| WEFT-697 | G5 session capability |
