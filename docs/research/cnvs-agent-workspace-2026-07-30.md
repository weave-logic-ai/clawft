# Research: CNVS (cnvs.dev) × WeftOS Agent Workspace

**Date:** 2026-07-30  
**Decision records:** [ADR-073](../adr/adr-073-agent-workspace-cnvs-principles.md), [ADR-074](../adr/adr-074-interim-xai-grok-voice.md)  
**Primary demo:** [@_MaxBlade — Grok Voice Think Fast 2.0 in CNVS](https://x.com/_MaxBlade/status/2082679537377144936)

## Product snapshot

| | |
|--|--|
| **Product** | CNVS — “Command an army of agents with your voice” |
| **Site** | https://cnvs.dev/ |
| **Author** | Max Blade (@_MaxBlade) |
| **Platform** | macOS native app (Swift); iOS “coming soon” |
| **License** | Closed commercial — lifetime license (~$99–$169 tiers) |
| **Stack (public)** | Swift UI; NVIDIA Parakeet local STT claims; optional cloud Realtime; MCP + CLI; multi-agent harnesses (Claude/Cursor/Codex); remote VPS “canvas” |

CNVS is **not** open source. We copy **interaction principles**, not code.

## What the MaxBlade demo shows (effortless control)

From the 2026-07-30 post + marketing:

1. **Voice spawns and prompts agents** at high speed (Grok Voice Think Fast 2.0).  
2. **Windows management is a skill** — not a separate settings app: agents, browsers, Hermes, music.  
3. **Summarization of agent walls of text** into spoken/short feedback (critical for multi-agent density).  
4. **Everything stays visible** on one canvas (“vibe code like Tony Stark / Jarvis”).  
5. **Conductor role** — human directs; many workers build in parallel.

## Interaction model (normative for ADR-073)

```
[Voice / Keys / MCP]
         │
         ▼
  Agent Workspace (spatial stage)
  ┌──────────┐ ┌──────────┐ ┌──────────┐
  │ Agent A  │ │ Agent B  │ │ Tool pane│  freeform, resizable, always visible
  │ (attention)│ │ worker  │ │ term/web │
  └──────────┘ └──────────┘ └──────────┘
         │ WindowIntent + substrate
         ▼
  Local daemon  OR  remote mesh node (same schema later)
```

## WeftOS gap matrix (condensed)

| Affordance | WeftOS today | Gap |
|------------|--------------|-----|
| Multi freeform windows | Sidebar + single active app | WM v1 (0.9) |
| Visible agents | Agents can be background | spawn ⇒ pane policy |
| Voice → layout | Voice stack designed; not bound to WM | WindowIntent + ADR-074 |
| Attention glow/TTS | Logs/chips | Attention bus |
| Summarize agent output | Chat/agent only | Workspace verb |
| Remote canvas parity | Mesh/substrate | Schema later |
| STT quality path | Local Parakeet/whisper | Interim xAI S2S (ADR-074) |

## What we already have that CNVS does not

- Cross-platform egui (native + WASM panel)  
- Substrate honesty (empty/offline/loading)  
- Governance / ExoChain / capability model  
- Composer + DESIGN.md multi-target UI  
- Open-source kernel/agent stack  

## Non-goals

- Porting or licensing CNVS  
- Replacing calm 0.8 stock desktop  
- Using ADR-056 BVH as a window manager  
- macOS-only features without Linux/Windows path  

## Next work (Plane)

| WEFT | Cycle | Work |
|------|-------|------|
| [WEFT-685](https://app.plane.so/weftos/browse/WEFT-685/) | 0.8.x | ADR-073 Phase A — stock desktop + agents inventory |
| [WEFT-686](https://app.plane.so/weftos/browse/WEFT-686/) | 0.9.x | ADR-073 Phase B — freeform WM v1 |
| [WEFT-687](https://app.plane.so/weftos/browse/WEFT-687/) | 0.9.x | ADR-073 Phase C — Agent Workspace mode + attention |
| [WEFT-688](https://app.plane.so/weftos/browse/WEFT-688/) | 0.9.x | ADR-073 Phase D — WindowIntent conductor demo |
| [WEFT-689](https://app.plane.so/weftos/browse/WEFT-689/) | 0.8.x | ADR-074 V0 — xAI Realtime + VoiceConfig |
| [WEFT-690](https://app.plane.so/weftos/browse/WEFT-690/) | 0.8.x | ADR-074 V1 — S2S tool bridge → WindowIntent |
| [WEFT-691](https://app.plane.so/weftos/browse/WEFT-691/) | 0.8.x | ADR-074 V2 — hybrid + metrics + local fallback |

Full architecture decision text: ADR-073 / ADR-074.
