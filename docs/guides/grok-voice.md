# Grok Voice Agent Integration

Drive a WeftOS node by voice through the xAI **Grok Voice Agent API**
(`wss://api.x.ai/v1/realtime`). The voice session runs in xAI's cloud;
WeftOS exposes its tool surface to it as a **remote MCP server** over
HTTPS, so the agent can submit jobs, manage jobs, ask about jobs, read
sensors, and run ordinary WeftOS tasks — anything the node's tool
registry and kernel daemon expose.

No realtime audio flows through WeftOS at all. Audio streams between the
client device (MacBook, PC, phone) and xAI; xAI calls back into the node
only for tool execution:

```text
┌──────────────┐  mic/speaker   ┌─────────────────────┐
│ MacBook / PC │◄──────────────►│  Grok Voice Agent   │
│ voice client │   WebSocket    │  (xAI cloud)        │
└──────────────┘                └─────────┬───────────┘
                                          │  MCP over HTTPS
                                          │  (tools/list, tools/call)
                              ┌───────────▼───────────┐
                              │  public HTTPS ingress  │  Tailscale Funnel
                              │  https://…/mcp         │  or Cloudflare Tunnel
                              └───────────┬───────────┘
                                          │
        tailnet (private) ┌───────────────▼───────────────┐
   MacBook / other nodes ─►  weft gateway  (POST /mcp)     │  primary node
                          │    ├─ builtin tool registry    │  (always-on PC)
                          │    └─ weftos__* daemon bridge  │
                          │  weaver kernel daemon (UDS)    │
                          │    jobs · cron · substrate     │
                          └────────────────────────────────┘
```

## What ships in-tree

- **`POST /mcp`** on the gateway API listener
  (`clawft-services/src/api/mcp_http.rs`): stateless Streamable-HTTP MCP
  serving `initialize`, `tools/list`, and `tools/call` through the same
  dispatcher, composite provider, and middleware pipeline as
  `weft mcp-server` (security guard, allowlist filter, result guard,
  audit log). Gated by a static bearer token; refuses to come up
  without one.
- **`weftos__*` daemon-bridge tools**
  (`clawft-cli/src/commands/weftos_tools.rs`): a curated, voice-friendly
  tool surface that forwards to the kernel daemon RPC over its Unix
  socket. Registered on both `/mcp` and stdio `weft mcp-server`.

| Tool | Daemon RPC | Purpose |
|------|-----------|---------|
| `weftos__kernel_status` | `kernel.status` | Node health / uptime |
| `weftos__list_services` | `kernel.services` | Kernel services + health |
| `weftos__run_task` | `agent.chat` | "Do X" in natural language via the concierge agent loop |
| `weftos__list_jobs` | `kernel.ps` | Running jobs (pid, state, resources) |
| `weftos__job_status` | `agent.inspect` | Inspect one job by pid |
| `weftos__stop_job` | `agent.stop` | Stop a job |
| `weftos__spawn_agent` | `agent.spawn` | Start a registered agent as a background job |
| `weftos__schedule_job` | `cron.add` | Recurring job every N seconds |
| `weftos__list_schedules` | `cron.list` | List cron jobs |
| `weftos__remove_schedule` | `cron.remove` | Remove a cron job |
| `weftos__list_sensors` | `substrate.list` | Enumerate `substrate/sensor/…` |
| `weftos__read_sensor` | `substrate.read` | Read a sensor / substrate path |

Alongside these, `/mcp` also exposes the builtin registry tools
(`read_file`, `exec_shell`, `web_search`, …) unless you restrict them —
see [Security](#security).

## 1. Node setup (always-on primary node)

Run `weaver` (kernel daemon) and `weft gateway` on the always-on box.
The daemon bridge talks over a Unix domain socket, and the Windows
named-pipe transport is not implemented yet (deferred to 0.8.x,
WEFT-483) — so **on a Windows PC, run the node inside WSL2** (or the
Docker image). GPU workloads (whisper.cpp, local LLMs) can still use
CUDA from WSL2.

`~/.clawft/config.json`:

```json
{
  "gateway": {
    "apiEnabled": true,
    "apiPort": 18789,
    "mcpEnabled": true,
    "mcpAllowedTools": ["weftos__*"]
  }
}
```

Generate a strong token and keep it out of config:

```bash
export WEFTOS_MCP_TOKEN="$(openssl rand -hex 32)"
weaver up          # kernel daemon (jobs, cron, substrate)
weft gateway       # channels + REST/WS API + POST /mcp
```

Sanity check from the same host:

```bash
curl -s -X POST http://127.0.0.1:18789/mcp \
  -H "Authorization: Bearer $WEFTOS_MCP_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | jq '.result.tools[].name'
```

## 2. Exposing `/mcp`

xAI's servers execute the MCP calls, so `/mcp` needs a **publicly
reachable HTTPS URL**. Everything else (dashboard, WS, daemon RPC)
should stay private. Options, in order of preference:

### Option A — Tailscale Serve + Funnel (recommended)

Tailscale is already on the node; Serve gives the tailnet private HTTPS,
and Funnel publishes a path to the internet through the same daemon — no
extra software, TLS handled for you.

```bash
# Private (tailnet-only) HTTPS for the dashboard/API — MacBook & other
# machines on the tailnet use this.
tailscale serve --bg 18789

# Public HTTPS for the MCP path only.
tailscale funnel --bg --set-path /mcp http://127.0.0.1:18789/mcp
```

The node becomes `https://<machine>.<tailnet>.ts.net/mcp`. Funnel
exposes only what you map — the rest of the API stays tailnet-private.
Note Funnel terminates TLS at Tailscale's edge and requires the funnel
node attribute in your tailnet policy.

### Option B — Cloudflare Tunnel

If you prefer a domain you own, run `cloudflared` on the node with an
ingress rule mapping `weftos-mcp.example.com → http://127.0.0.1:18789`,
and (recommended) restrict the hostname to path `/mcp` with a WAF rule.
Equivalent posture; one more daemon to run, plus a Cloudflare-managed
cert. Cloudflare Access "service tokens" can add a second auth layer via
the Grok `headers` field (see below).

### What about hosting on Cloudflare Containers / Vercel?

Not needed as a front end: the node itself is the server, and both
Funnel and Tunnel are outbound-only (no inbound firewall holes). A
hosted proxy in front would add a hop without adding capability — the
compute, drive space, and hardware access all live on the primary node
anyway. Where a hosted component *does* make sense later is serving the
static browser voice client (a page that opens the xAI realtime
WebSocket with an ephemeral token); that's a static deploy like the
existing dashboard (`docs/ui/deployment.md`), not a container.

## 3. Wiring the Grok session

Create a realtime session and hand it the WeftOS MCP server. Server
side (or via ephemeral token for browser/mobile clients — see xAI docs
on `xai-client-secret.…` subprotocol auth):

```jsonc
// first message after connecting to wss://api.x.ai/v1/realtime?model=grok-voice-latest
{
  "type": "session.update",
  "session": {
    "voice": "ara",
    "instructions": "You are the voice interface to WeftOS, the user's home node. Use the weftos tools to run tasks, manage jobs and schedules, and read sensors. Confirm before stopping jobs. Keep spoken replies short.",
    "turn_detection": { "type": "server_vad" },
    "audio": {
      "input":  { "format": { "type": "audio/pcm", "rate": 24000 } },
      "output": { "format": { "type": "audio/pcm", "rate": 24000 } }
    },
    "tools": [
      {
        "type": "mcp",
        "server_label": "weftos",
        "server_description": "WeftOS primary node: jobs, schedules, sensors, tasks",
        "server_url": "https://<machine>.<tailnet>.ts.net/mcp",
        "authorization": "Bearer <WEFTOS_MCP_TOKEN>",
        "allowed_tools": [
          "weftos__kernel_status", "weftos__list_services",
          "weftos__run_task", "weftos__list_jobs", "weftos__job_status",
          "weftos__stop_job", "weftos__spawn_agent",
          "weftos__schedule_job", "weftos__list_schedules",
          "weftos__remove_schedule",
          "weftos__list_sensors", "weftos__read_sensor"
        ]
      }
    ]
  }
}
```

MCP tools are executed **server-side by xAI** — the voice client needs
no function-call handling at all. Any device that can open the xAI
WebSocket (MacBook, the PC itself, phone) gets the same tool surface,
because the tools live with the session, not the device.

`weftos__run_task` drives a full LLM agent loop on the node and can take
tens of seconds; the bridge caps it at 120 s (other calls 15 s) and
returns a spoken-friendly error on timeout. Prefer `spawn_agent` /
`schedule_job` for long-running work, then check back with `list_jobs`.

## 4. Phone / MacBook client: the `/voice` page

The gateway serves a built-in voice client at **`/voice`** (public
route, embedded in the binary — no separate UI build). It runs on
anything with a browser and a mic: phone, MacBook, or the PC itself.

```text
                 ┌─────────────────────────────────────────────┐
                 │                xAI cloud                     │
   voice WS      │  Grok Voice Agent (grok-voice-latest)       │   MCP over HTTPS
  ┌─────────────▶│  ASR ─ reasoning ─ tool calls ─ TTS         │──────────────┐
  │              └─────────────────────────────────────────────┘              │
  │                                                                           ▼
┌─┴──────────────┐                                       ┌────────────────────────────────┐
│  Phone         │                                       │  4070 PC (always on, WSL2)     │
│  ─ mic/speaker │        Tailscale tailnet (private)    │                                │
│  ─ /voice page │◀─────────────────────────────────────▶│  Tailscale Funnel ── /mcp only │
│  ─ Tailscale   │   page + tokens + /ws activity feed   │        │ 127.0.0.1:18789       │
│    app         │                                       │        ▼                       │
└────────────────┘                                       │  weft gateway (axum)           │
                                                         │   /voice /ws /api /mcp         │
                                                         │        │ UDS kernel.sock       │
                                                         │        ▼                       │
                                                         │  weaver daemon (kernel RPC)    │
                                                         └────────────────────────────────┘
```

Connection sequence (all initiated by the page):

1. `POST /api/auth/token` (public bootstrap path, tailnet-only) → a
   24 h gateway API token.
2. `POST /api/voice/xai-token` (gateway-token gated) → the gateway
   calls `POST https://api.x.ai/v1/realtime/client_secrets` with the
   node's `XAI_API_KEY` and returns a ~5-minute ephemeral token. The
   long-lived key never leaves the node.
3. The page opens `wss://api.x.ai/v1/realtime?model=…` with the
   `xai-client-secret.<token>` subprotocol, sends the `session.update`
   from §3 (MCP URL + bearer come from the page's Settings dialog,
   stored in `localStorage`), and streams mic PCM16 @ 24 kHz.
4. Grok's cloud calls `POST /mcp` through the Funnel for tool work;
   spoken replies stream back to the page as `response.output_audio.delta`.

### Seeing what the node is doing (not just hearing it)

Voice alone hides the machinery, so the page has two panes:

- **Conversation** — live ASR of what you said plus Grok's reply
  transcript as it speaks.
- **Node activity** — the page subscribes to the gateway WebSocket
  (`/ws`, topics `voice-activity` and `canvas`). Every `tools/call`
  that hits `/mcp` is published to `voice-activity` as
  `{kind, tool, argsPreview, resultPreview, isError, ts}` (previews
  truncated at 600 chars), so each voice-triggered tool call renders as
  a card: tool name, arguments, and the JSON result. Any `data:image/…`
  URI or image URL in a result is rendered inline as an image.

Requirements: `XAI_API_KEY` exported in the gateway's environment, and
the page reached over HTTPS or `localhost` (browsers refuse mic access
on plain remote HTTP — Tailscale Serve gives you tailnet HTTPS for
free).

## 5. Security

- `/mcp` **cannot start without a bearer token** (`WEFTOS_MCP_TOKEN` or
  `gateway.mcpToken`); requests are checked with a constant-time
  compare, rate-limited per-IP, and pass the same `SecurityGuard` →
  `PermissionFilter` → `ResultGuard` → `AuditLog` middleware as the
  stdio MCP server.
- **Restrict the surface for voice.** `"mcpAllowedTools": ["weftos__*"]`
  exposes only the curated daemon bridge and keeps `exec_shell` /
  `write_file` etc. off the public endpoint. Grok's own `allowed_tools`
  is client-side defence-in-depth, not a substitute.
- Substrate reads carry an `actor_id` (default `voice-agent`) so
  ADR-057 read-ACLs and audit trails attribute voice traffic; capture-
  tier paths can be denied to that actor in kernel config.
- Rotate the token by restarting the gateway with a new
  `WEFTOS_MCP_TOKEN` and updating the session config; treat it like any
  channel credential (never commit it).
- Funnel/Tunnel only ever expose `/mcp`; the dashboard, `/ws`, and the
  daemon TCP relay (`kernel.ipc_tcp`, off by default) stay
  tailnet-private.

## 6. Troubleshooting

| Symptom | Likely cause |
|---------|--------------|
| `404 MCP endpoint not enabled` | `mcpEnabled` false, or no token configured (check gateway logs for the warning). |
| `401 invalid or missing bearer token` | Token mismatch between session config and `WEFTOS_MCP_TOKEN`. |
| Tool result says daemon not running | `weaver up` not running on the node, or gateway running outside WSL2 while the daemon is inside. |
| `substrate.read` denied | ADR-057 ACL: the path's tier requires a permitted `actor_id`. |
| Grok never calls tools | Check `allowed_tools` names include the `weftos__` prefix; verify the funnel URL from outside the tailnet. |

Related: `docs/guides/mcp.md`, `docs/guides/voice.md` (browser voice
dashboard), `docs/adr/adr-057-substrate-read-acl.md`,
`docs/reference/config.md` (§gateway).
