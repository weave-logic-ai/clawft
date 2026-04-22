# Session 2 — Agentic UI State of the Art (RQ2 + RQ7)

**Status**: Research — Round 1
**Focus**: dev-panel chat (priority-one prototype target)
**Predicates under test**: ontology-addressable / self-describing / streaming-native / dual-operator
**Meta-test**: does the system treat display as passive paint, or collect return-signal (active-radar loop)?

---

## Scope

This session surveys the current industry state of agent-surfaced UIs, weighted toward the dev-panel category because that is the WeftOS priority-one prototype. For each system, we interrogate composition model, parallel-thread surfacing, interruption handling, voice, goal persistence, predicate conformance, and whether the surface is a passive paint or an active probe. The goal is not to catalogue features; it is to locate the gaps the canon must fill and to feed a concrete prototype brief for the dev-panel surface.

The research window is mid-2024 through April 2026. Sources are the vendor docs, changelogs, and recent comparison reporting (see inline citations throughout the teardowns).

---

## Teardowns

### 1. v0 (Vercel) — prompt-to-UI via shadcn

- **Composition model**: the agent *writes code*, it does not *drive a primitive tree*. v0 generates Next.js + Tailwind + shadcn/ui code on each prompt turn and pushes it into a live preview. Starting points are pre-built **templates** ("Next.js + shadcn/ui", "Forms", "Charts", "AI Chatbot", "Ecommerce", "Docs"); from there the model regenerates React source. The agent's substrate is **a git repo of React code**, not a surface tree.
- **Parallel threads**: none at the UI layer. Multiple projects, one conversation each.
- **Interruption**: text-only "stop generation" affordance. No checkpointing inside a generation turn.
- **Voice**: none first-class.
- **Goal persistence**: project-scoped prompt history only; no constitutional goal that outlives sessions.
- **Predicates**: 0/4. The output is framework-locked React code that satisfies none of the four — there is no ontology identity on the generated `<Button>` that an agent in another process could query.
- **Active-radar**: passive paint. v0 learns from preview clicks only to the extent they feed back into the *next generation turn*; there is no typed return-signal schema, no variant-id propagation, no ECC-style doppler.

**Failure mode that matters for us**: v0 conflates "generate code" with "compose a surface". That is the wrong primitive. If the renderer is disposable (our canon), the agent must not be generating React.

### 2. AI Elements (Vercel) — the 49-component library

- **Composition model**: **shadcn-style code-ownership** primitives. Four families: *Chatbot* (Message, Conversation, PromptInput, Suggestion, Reasoning, Plan, Task, Tool, Checkpoint, Context, ModelSelector, Queue, Shimmer), *Code* (Agent, Artifact, CodeBlock, FileTree, Terminal, Sandbox, JSXPreview, StackTrace, SchemaDisplay, Commit, WebPreview, TestResults), *Voice* (SpeechInput, AudioPlayer, MicSelector, VoiceSelector, Persona, Transcription), *Workflow* (Canvas, Node, Edge, Connection, Toolbar, Controls, Panel).
- **Parallel threads**: no. Each `Conversation` is a single linear message list; the `Agent` component is decorative, not a supervisor.
- **Interruption**: AI SDK-level `abort`; component-level is a stop button.
- **Voice**: has SpeechInput + Transcription as *React components*, but they are glue around browser MediaRecorder; no first-class protocol for voice-as-input-modality across all primitives.
- **Goal persistence**: none. `Context` component is a passive display.
- **Predicates**: 1/4 partial. Component set is rich but each is a **passive widget**. I confirmed `Artifact` is a layout container with Header/Title/Description/Actions/Content/Close and explicitly no state / affordance / confidence properties. There is no ontology identity; agents cannot query `Artifact.affordances` — they can only render one.
- **Active-radar**: none. Components emit React onClick; there is no variant-id, mutation schema, or typed return-signal.

**Lesson**: AI Elements is the **most advanced catalogue in the industry**, and it still doesn't clear the bar. It's a design language for human designers composing human-shaped chatbots; it is not an ontology an agent queries. Their `Plan`, `Task`, `Tool`, `Reasoning`, `Checkpoint` components name-match our canon — use them as **naming cues only**, do not adopt the model.

### 3. Continue.dev — open-source Cursor alternative

- **Composition model**: two modes, Chat and Agent. Agent mode is Chat + tool-execution UI. Output is markdown bubbles + diff view + terminal panel — all pre-designed panels the model drops content *into*.
- **Parallel threads**: no. VS Code 1.107 added agent-session listing in the Chat view, but they are serial — you switch between them, you do not watch them concurrently.
- **Interruption**: stop button.
- **Voice**: third-party; 100%-offline Mac voice-to-text is *sold alongside* ($24 one-time), not baked in.
- **Goal persistence**: none structural; chat history + `.continuerc` config.
- **Predicates**: 0/4.
- **Active-radar**: passive paint.

**Failure mode**: Continue has a clean IDE panel shape but treats agent-output as "formatted chat reply". Nothing in the surface is queryable by a second agent.

### 4. Cursor — our current day-to-day

- **Composition model**: pre-designed chat panel (Composer/Agent tab) + inline diffs. In April 2026, Cursor 3 shipped an "agent-first interface" whose primary model is **managing parallel coding agents** rather than editing files — this is the right direction but still landed as "a list of agent cards with a chat each" rather than a composable surface.
- **Parallel threads**: yes, **finally** — Cursor 3 runs many agents in parallel across git worktrees, local/cloud/SSH. Each agent has its own worktree; no cross-stepping. Cursor-reported 40% drop in interruption vs unsandboxed runs.
- **Interruption**: per-agent stop; agents can be moved local→cloud to survive laptop close.
- **Voice**: not first-class.
- **Goal persistence**: `@Past Chats` lets you selectively import slices of old conversations; there is **no project goal that outlives chat turns**. "Context rot" is named as a problem in their own best-practices blog — fresh conversations outperform long ones.
- **Predicates**: 1/4 partial (streaming is real, everything else absent).
- **Active-radar**: passive paint.

**Failure modes worth naming, because we live inside them**:
1. **Terminal-trapped chat**: the composer is shaped like a terminal prompt even though it is a React panel. Multi-line input is second-class, attachments are a bolted-on `@` menu, voice is absent.
2. **Single-thread rendering per pane**: Cursor 3 added parallel agents, but each agent's reasoning is still a scrollable linear log. No way to see "what is agent A waiting on" and "what is agent B currently doing" at a glance without tabbing.
3. **Lost goals**: the system has *no concept of a project goal persisting across agent sessions*. @Past Chats is a band-aid; it's the user's job to re-hydrate context. There is nothing resembling a constitutional goal the agent is constrained by.
4. **Context rot is a confession of failure**: admitting "fresh conversations outperform long ones" is an admission that the system cannot manage its own working memory. Our ECC / governance layer should eat this lunch.
5. **No return-signal model**: every click, every accept-diff, every rollback is telemetry-to-Anysphere, not a typed echo the user's own system learns from. That is the single biggest missed value.

### 5. Cline / Roo-Cline (Roo Code) — VSCode extensions

- **Composition model**: native VS Code Chat view + `VscodeDiffViewProvider` side-by-side diffs + terminal panel. Every tool call renders as an inline card with an **Approve / Reject** button. Plan mode (read-only exploration) → Act mode (execute with per-step approval).
- **Parallel threads**: Cline is single-thread. **Roo Code** (fork) added **Orchestrator mode** + **Custom Modes** (named agents with scoped tool permissions) — the orchestrator dispatches subtasks to specialised modes. This is the closest thing in the industry to our mesh-runtime + specialised-agents model, and it is still surfaced as a *linear orchestrator transcript*, not a concurrent panel.
- **Interruption**: per-step approval is the interruption. Checkpoints after each tool call let you roll back to the exact pre-call state — this is a **genuinely good primitive** and we should steal it.
- **Voice**: not first-class.
- **Goal persistence**: none structural; Custom Modes carry persona prompts but not goals.
- **Predicates**: 1/4. Checkpoints come closest to streaming-native (snapshots on each commit), but still no ontology / affordance enumeration / dual-operator.
- **Active-radar**: passive paint. Approvals are a high-signal return but the system does not attribute them to mutation axes.

**Lesson**: the per-tool-call checkpoint pattern is the one mechanic from this cohort that maps cleanly onto our canon — it becomes a stream verb on every primitive invocation.

### 6. Aider — CLI-first pair programming

- **Composition model**: terminal REPL; LLM output is raw text + fenced code blocks parsed into SEARCH/REPLACE diffs that Aider applies to the git tree. Output surface = terminal scrollback.
- **Parallel threads**: no; one process, one conversation.
- **Interruption**: Ctrl-C; `/undo` reverts the last commit.
- **Voice**: **yes, and first-class** — `/voice` records, transcribes, and feeds the text as input. It is the only tool in this cohort where voice is a core verb not a plugin. Simple but correct.
- **Goal persistence**: `--restore-chat-history` + `.aider.chat.history.md`; no goal primitive.
- **Predicates**: 0/4 (terminal-only = hard-fail on dual-operator for non-humans).
- **Active-radar**: passive paint; every output is a git commit, so the exhaust is structured, but it's structured for humans (diff + message), not for ontology.

**Lesson**: `/voice` being a simple first-class verb in a CLI is instructive — voice does not need a WebSpeechAPI empire; it needs a single verb the agent understands.

### 7. Plandex — planning-mode dev agent

- **Composition model**: terminal; plan-as-artefact. Plans have versions, diffs, branches. You can "try two approaches in parallel branches" of a plan.
- **Parallel threads**: plan branches are parallel *in the version graph*, but execution is one-at-a-time.
- **Interruption**: autonomy levels (None / Basic / Plus / Semi-Auto / Full-Auto) are the dial. Auto-debug with rollback.
- **Voice**: none.
- **Goal persistence**: a **plan** is a durable artefact, with branches and history — this is the closest thing in the CLI cohort to goal persistence, and it's project-scoped not session-scoped. Good.
- **Predicates**: 0/4 at UI level, but the **plan as durable branching artefact** is a conceptual win worth naming.
- **Active-radar**: passive paint.

**Lesson**: plans-as-artefacts with a branch/version graph is the right shape for "goals that outlive sessions". We want this, but as a first-class substrate entity, not a flat file per plan.

### 8. Open Interpreter — natural-language execution surface

- **Composition model**: terminal REPL that renders a ChatGPT-style conversation; OS mode uses GPT-4V + mouse/keyboard to *drive the host OS*.
- **Parallel threads**: no.
- **Interruption**: Ctrl-C; OS mode is notoriously startling — community has repeatedly flagged discomfort about the screen takeover with no obvious safe-stop.
- **Voice**: the separate `01` project (ESP32 hardware + desktop app) is the voice surface. Not integrated into the core product.
- **Goal persistence**: none.
- **Predicates**: 0/4.
- **Active-radar**: nothing; in fact, the OS-mode pattern is a **negative example** of how *not* to do dual-operator: the agent drives the human's input devices as if the human were not there. Our canon instead says the agent drives *primitives* and the human's pointer/voice drives the same primitives.

**Lesson**: the way Open Interpreter fuses "agent action" with "host-OS pixels" is exactly the wrong inversion. Don't let the agent fight for the mouse.

### 9. AutoGen / AutoGen Studio — multi-agent orchestration UI

- **Composition model**: AutoGen Studio is the most visible attempt to surface multi-agent runtime state. Drag-and-drop **Team Builder** (agents, tools, termination conditions), **Playground** with live message streaming, message-flow visualisation with directed edges. Microsoft explicitly labels it "not for production — a research prototype".
- **Parallel threads**: visualised as a DAG + live stream. `SelectorGroupChat` picks next speaker; `Swarm` is tool-based selection; `GraphFlow` is declarative workflow. This is the richest *visualisation* in the cohort, by a distance.
- **Interruption**: `UserProxyAgent` injects human input mid-run.
- **Voice**: none.
- **Goal persistence**: team spec is persisted JSON; goals are not a separate concept.
- **Predicates**: 1/4. Message stream is streaming-native. Agents are not ontology-addressable in the sense we mean (they have names, not queryable typed identities).
- **Active-radar**: passive paint, but the visual message flow is as close as anyone in this cohort gets to showing *what the agents collectively are doing*.

**Lesson**: AutoGen Studio proves users *can* read a multi-agent stream when it is visualised well. The visualisation is a panel of agent chips + a message-flow canvas. We should adopt this shape for the dev-panel multi-agent view, raised to primitive status.

### 10. GitHub Copilot Chat — the industry default

- **Composition model**: VS Code Chat view. `@workspace` / `@terminal` / `@file` mentions, agent mode with per-session permission levels (Default / Bypass Approvals / Autopilot). MCP servers bridge in from early 2026; as of April 2026, bridging to Claude agent sessions has been silently broken since late March (`hook_6` bug — tool-permission stream closes before response).
- **Parallel threads**: **GitHub Copilot coding agent** is the cloud-native parallel agent; in IDE, sessions appear in a list with status/progress/file-change stats; archive/unarchive exists. Still surfaced as a list of cards, one active at a time.
- **Interruption**: stop button + per-permission-level approval.
- **Voice**: none first-class.
- **Goal persistence**: **custom agents** (file-based persona+instructions) are the persistence primitive; there is still no project-goal-as-substrate-entity.
- **Predicates**: 1/4 (streaming only).
- **Active-radar**: passive paint.

**Failure mode**: Copilot's current bug (silent chat when tool-permission stream drops) is instructive — when the system *does* collect a return-signal (the permission hook) and that signal is stateful and async, it can desync the whole conversation. The lesson is not "don't collect signal"; it is **signal collection must be part of the primitive protocol, not a side-channel bolted on**.

### 11. Devin / Devin-style — autonomous agent panels

- **Composition model**: **four panels** — chat, planner, browser, editor, terminal — each a pre-designed pane. Each "tab" is its own environment with its own codebase/browser/planner. Planner + Coder + Critic roles internally. Full replay timeline records every terminal command, file edit, browser action. A 2026 **Focus Mode** hides chrome for distraction-free chat; an **Agents tab** auto-appears when a session spawns child sessions (todos + PRs per child).
- **Parallel threads**: yes — parallel sessions and child-sessions with a dedicated tab. The multi-session surface is the most mature in the cohort for autonomous work.
- **Interruption**: deliberately designed for it — "Devin welcomes being interrupted" — you can watch the screen in real-time and intervene via chat.
- **Voice**: not first-class.
- **Goal persistence**: per-session, not project-scoped; sessions are long-lived but goals are prompts.
- **Predicates**: 1.5/4 — streaming-native (replay timeline) and a partial dual-operator in the sense that the user can interject, but the surface is still pre-designed panels, not an ontology.
- **Active-radar**: the **replay timeline** is the richest return-signal collection in the cohort — every action is attributable, timestamped, re-playable. Still not surfaced as ECC-style variant/doppler/bearing, but the raw substrate is there.

**Lesson**: Devin's four-panel + child-session shape is the best-in-class **reference posture** for a dev-panel today. The replay timeline is the best-in-class **return-signal substrate**. We can do strictly better by making both first-class ontology entities.

### 12. Claude Code — the CLI surface you're running inside right now

- **Composition model**: terminal REPL + VS Code extension + JetBrains + Desktop + Web. Same engine, five surfaces. Inline diffs, @-mentions, plan review, conversation history in VS Code; the VS Code extension renders diffs natively and has a task list (Ctrl+T since v2.1.16, Jan 2026). Sub-agents via the Task tool (full tool access, empty context). Voice via `/voice` + spacebar-push-to-talk (March 2026).
- **Parallel threads**: `Task` spawns concurrent sub-agents, but their output collapses back into the main thread's transcript. There is no surface that shows sub-agent A and sub-agent B running concurrently. **Child-task trees exist as data; they are not a surfaced primitive.**
- **Interruption**: Esc cancels the current turn; Esc+Esc rewinds to before the previous edit. Strong keyboard story. Still single linear transcript.
- **Voice**: `/voice` + hold-space, March 2026. Good that it exists; still post-hoc.
- **Goal persistence**: `CLAUDE.md` at project root (and nested), auto-memory that accretes across sessions, scheduled/recurring routines. This is **the best goal-persistence story in the cohort** but it is still a flat-file substrate, not an ontology-backed constitutional goal.
- **Predicates**: 1.5/4 — streaming yes, partial dual-operator (CLI + VS Code), no ontology identity, no affordance enumeration.
- **Active-radar**: passive paint. Edits are git-diffed and loggable, but no variant-id, no ECC hook, no return-signal schema for the primitive-render pulse because there are no primitive renders.

**What a panel surface would solve, that the current CLI will not**:
1. **Simultaneous sub-agent visibility** — right now Task sub-agents are invisible while they run; a panel gives each its own surface with live state + bearing.
2. **Non-linear reasoning display** — one agent's plan, another's diff, another's test run, a fourth's web search — these are concurrent and should be rendered as such, not serialised into a scrollback.
3. **Voice-as-first-class across surfaces** — `/voice` is command-shaped; voice should be a *modality* that can address any primitive.
4. **Goal primitive that isn't a markdown file** — CLAUDE.md is excellent as a substrate hint but it is not queryable-at-runtime by the agent as a typed object with satisfied/unsatisfied predicates. A surface primitive for the project goal makes it legible to both operators.
5. **Return-signal attribution** — the CLI cannot vary a render because there is no render to vary. A panel is the precondition for the active-radar loop.

---

## What is missing from all of them

After twelve teardowns, the absences collapse into eight clean gaps. These are the gaps the canon fills.

1. **No ontology-addressable primitives, anywhere.** Every system above treats the UI as strings-and-widgets. None of them expose `Button#ontology://…` that a second agent in a second process can `query`/`invoke`/`subscribe` against. AI Elements comes closest by naming things well; the names are not handles.

2. **No self-describing affordance enumeration.** Every tool-call UI in the cohort (Cline approve/reject, Cursor accept-diff, Copilot autopilot) is a human-facing button the agent *emits* — not a typed affordance the agent *enumerates from the primitive*. Agents trigger operations and hope, when they should read the affordance set and pick.

3. **No confidence channel on rendered values.** Nothing in this cohort tells the user *where the displayed number came from* (inference vs cache vs deterministic) or how fresh it is. The gauge is always the same whether the model is 99% sure or 51% sure.

4. **Multi-agent parallel output is either serialised or visualised-only.** AutoGen Studio visualises it, Roo Code orchestrates it, Cursor 3 worktrees isolate it, Devin child-sessions bundle it — none compose multiple live agent surfaces into one legible dashboard that a human and a second agent can both read. Everyone treats parallel as "list of tabs".

5. **Interruption is pause/stop, not interruption-priority.** Every system has a stop button. None have a typed interruption policy that says "this agent is in a hard loop against a build, defer a cosmetic suggestion by 30 seconds; the user is idle on this surface, a high-confidence prompt is safe to surface now". Our ECC-backed interruption score has no counterpart.

6. **Voice is a tacked-on input method, not a dual-operator modality.** Aider does `/voice` well for its scope. Claude Code added `/voice` in March 2026. Vercel AI Elements ships SpeechInput as a component. But voice against *every primitive* (addressing a specific button/field/chip by ontology identity) is absent. Voice is still "dictate into a text box".

7. **Goals are prompts, memory files, or custom-agent personas — never substrate entities.** CLAUDE.md, Continue's config, Roo Custom Modes, Copilot custom agents, Devin sessions, Plandex plans — all of these are file-shaped. None is a substrate-resident constitutional goal the running agents are constrained by in a governance sense, queryable with state (satisfied / in-progress / blocked / violated), with an on-chain audit trail. This is our strongest differentiation.

8. **Display is universally passive paint.** Not one of the twelve treats the emitted display as an active probe. Variant-ids do not exist. Mutation schemas do not exist. The return-signal (hover, dwell, scroll, accept, reject, latency-to-action, bearing across affordances) is either discarded or piped to the vendor's analytics, never fed back into the user's own ECC / governance model. **This is the biggest missed asset in the industry.**

Three smaller but real gaps that will bite us if we leave them implicit:

- **No wire format for inner-vs-outer composition.** A chart rendered inside a chat bubble inside a panel inside a workspace — every system invents ad-hoc parent/child context passing. Our protocol needs composition as a first-class verb (mount / unmount / nest / route events).
- **No distinction between Tier-A primitive and Tier-B wrapper.** Everyone mixes "chat bubble containing an iframe" with "chat bubble containing a data chip" as if they were the same thing. They aren't — one leaks signal, one is the signal.
- **No separation of renderer from protocol.** Every surface above is fused to its renderer (React, VS Code webview, native panel, terminal). Swap the renderer and you lose the system. That is the predicate the canon *already* enforces (renderer disposable); no competitor does.

---

## Recommendations for canon

Numbered, each citing the foundation elements it anchors in. These feed directly into the prototype brief for the dev-panel.

1. **Ship the dev-panel as an egui host speaking the primitive protocol, embedded in Cursor/VSCode via a local sidecar + webview, from day one.** The webview renders an egui wasm build bound to the substrate over a local IPC socket; VS Code extension hosts the webview and forwards keyboard/focus/paste events as primitive verbs. This directly tests predicates 1-4 end to end in a real IDE. *(Anchors: AGENDA Two-Modes table, RQ7; foundations §Two tiers, §Reverse-DDD arrow.)*

2. **Make the project goal a substrate entity with a typed `ProjectGoal` primitive, not a markdown file.** Ontology identity, declared affordances (`refine`, `split`, `satisfy`, `violate`, `archive`), state machine (`active` / `blocked` / `satisfied` / `violated`), governance-counsel signoff. Every agent surface renders against the *active goal set*; a surface composed that contradicts an active goal is refused by the governance gate, not the renderer. *(Anchors: foundations §Digital exhaust = intent; AGENDA RQ8; existing governance-counsel agent.)*

3. **Adopt the Roo-Cline per-tool-call checkpoint pattern, raised to a `commit` stream verb on every primitive invocation.** Each primitive operation emits `open` → `chunk`* → `commit|cancel`, with an automatic checkpoint keyed by the (user, surface, primitive, invocation-id). Rollback is a first-class affordance on the session, not a bespoke diff-revert. *(Anchors: foundations §Streaming-native; Cline checkpoints teardown.)*

4. **Surface multi-agent parallel work as a `MeshPanel` primitive, not a tab list.** The panel is composed of per-agent surfaces side-by-side (Tier-A primitive each), with a shared `MessageFlow` canvas primitive for visualising cross-agent traffic in the AutoGen Studio shape. Interruption priority drives which sub-surface gets focus-pull. Steal the Devin child-session auto-appear model: when an agent spawns a child, the MeshPanel grows a new cell; when it terminates, the cell collapses with its summary pinned. *(Anchors: foundations §Tier A primitives; AGENDA Two Modes / Mission Console; AutoGen Studio + Devin teardowns.)*

5. **Treat voice as a modality addressed at the ontology, not at a text input.** Voice input carries an `intent-vector` + optional `primitive-target-id`. "Accept the auth diff" parses to `(intent=accept, target=diff://session-X/file-auth.rs)` and is dispatched as a primitive invocation — any primitive with `accept` in its affordances can receive it. Voice is thus a dual-operator modality, not a dictation field. *(Anchors: foundations §Dual-operator; AGENDA non-goals "No CLI-shaped chat UI"; Aider + Claude Code /voice teardowns.)*

6. **Stream the reasoning trace as typed `Reasoning` + `Plan` + `Task` primitives, not markdown chat bubbles.** The agent emits ontology-addressable thought objects the panel arranges; the user can open/close/cite them, a second agent can subscribe to them. Rendering is a view into the stream; the stream is the truth. Name-match AI Elements' components only — ship the ontology behind them. *(Anchors: foundations §Self-describing, §Streaming-native; AI Elements teardown.)*

7. **Bind the active-radar loop into the protocol from the start: every primitive render carries a `variant-id`, every return-signal (click, utterance, dwell ≥ threshold, scroll-to, accept, reject, latency) flows back as a typed event tagged with that variant-id.** The mutation schema per primitive declares which axes are legal to vary. GEPA-style small mutations run by default on non-safety primitives. The ECC updates its causal model per-user per-task. **This is the single most important canon decision because no competitor does it.** *(Anchors: foundations §Active-radar loop; foundations §Digital exhaust = intent.)*

8. **Adopt a tight 12-primitive starter set to prove the dev-panel end-to-end.** `Conversation`, `Message`, `PromptInput`, `Reasoning`, `Plan`, `Task`, `ToolInvocation`, `Diff`, `Checkpoint`, `ProjectGoal`, `MeshPanel`, `ForeignSurface`. Every one is Tier-A except `ForeignSurface`. The prototype's success criterion is that an agent, given the ontology + project goal, can compose a working dev-panel surface *nobody on the team designed*, and it reads correctly. *(Anchors: foundations §Two tiers; AGENDA Success criteria.)*

9. **Expose interruption-priority as a primitive-level property, not a global stop button.** Each streaming primitive declares `interruptable`/`coalescable`/`deferrable` and a priority class. The panel's interruption policy sees the ECC's current attention-state context fact and routes accordingly — a `voice://dictation` surface in active use defers low-priority prompts; a `terminal://active-build` surface blocks cosmetic suggestions. *(Anchors: foundations §Tier B signals "time interruptions well"; AGENDA RQ8.)*

10. **Log every inner-vs-outer composition through the wire format — no ad-hoc parent/child prop drilling.** `mount`, `unmount`, `nest`, and `route-event` are protocol verbs. The renderer never sees a primitive without its typed parent-composition context. This is what makes a second renderer writable without touching composition logic (AGENDA success criterion 1). *(Anchors: foundations §Reverse-DDD arrow; AGENDA Non-goals "No framework lock-in".)*

---

## Open questions

These are the questions the research surfaced that this session cannot answer alone. Each is a candidate for a follow-up session or ADR.

1. **Webview vs native-sidecar for IDE embedding.** Cursor and VS Code both expose webviews with real constraints (no persistent GPU context, limited SharedArrayBuffer, strict CSP). egui-wasm in a webview is tested; egui-native via a sidecar protocol (X11-style) is more powerful but far more platform work. Which does the prototype ship first? RQ7 owes a decision.

2. **How does the active-radar loop interact with privacy non-negotiables?** Variant-ids and dwell telemetry are the exhaust = intent. The non-negotiables demand per-user, on-chain, consent-tracked storage with signal-lossy interpretation. Need a concrete session covering how per-user causal models stay local while still being portable across the user's own devices.

3. **What's the right wire shape — CBOR / FlatBuffers / Cap'n Proto — for a stream that carries both high-frequency return-signal events and lower-frequency composition verbs?** RQ6 territory; the dev-panel prototype will force this decision within weeks.

4. **Can Roo Custom Modes' orchestrator model be read as a pre-protocol sketch of our MeshPanel+ProjectGoal combination?** If so, a public compat layer could accelerate adoption.

5. **Voice-driven ontology addressing has no prior art worth copying.** How big is the grammar problem of mapping utterances to `(intent, primitive-id)` tuples at usable accuracy? Needs a spike — likely a single-session research task against an ASR + a simple grammar scoped to the starter 12 primitives.

6. **How does a second renderer (even a toy one) prove out without re-implementing the composition model?** AGENDA success criterion 1 — we need a concrete second target (terminal? a minimal React host?) to keep the protocol honest. Devin's replay timeline is a clue: a replay-only renderer is a cheap second.

7. **Goal primitives + governance-counsel: what is the ADR-worthy boundary between "a goal blocks a surface composition" and "a goal warns"?** Refusal is powerful; silent defer may be more helpful. Needs a policy session with the constitutional governance track.

---

**Word count**: ~3380 (within ≤3500 budget).

**Sources cited inline across the teardowns**; key anchor pages:
- v0: `https://v0.app/docs`, Vercel academy v0 guide.
- AI Elements: `https://elements.ai-sdk.dev/` (component catalogue; Artifact page verified as passive container).
- Continue.dev: `https://docs.continue.dev/ide-extensions/agent/how-it-works`.
- Cursor: `https://cursor.com/docs`, `https://cursor.com/blog/agent-best-practices`, Cursor 3 release coverage (InfoQ April 2026).
- Cline / Roo: `https://docs.cline.bot/`, DeepWiki Plan/Act modes + checkpoints, `https://roocode.com/`, Qodo Roo-vs-Cline 2026 comparison.
- Aider: `https://aider.chat/docs/usage/voice.html`, `https://aider.chat/docs/usage/modes.html`.
- Plandex: `https://plandex.ai/`, `https://docs.plandex.ai/`, Plandex GitHub README.
- Open Interpreter: `https://docs.openinterpreter.com/guides/os-mode`, `github.com/openinterpreter/01`.
- AutoGen / Studio: `https://microsoft.github.io/autogen/stable/`, AutoGen Studio v0.4 announcement (Victor Dibia).
- Copilot Chat: `https://code.visualstudio.com/docs/copilot/`, GitHub issue #308999 (tool-permission stream bug).
- Devin: `https://docs.devin.ai/release-notes/2026`, Augment Code Devin-vs-Codex comparison.
- Claude Code: `https://code.claude.com/docs/en/overview`, v2.1.16 release notes, March 2026 /voice announcement.
