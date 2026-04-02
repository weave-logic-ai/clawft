# Interactive WASM Playground & Guided Tour — Plan

## Vision

Replace "install Rust 1.93+ and build from source" with a zero-install, in-browser experience that **shows** WeftOS's capabilities rather than describing them. A visitor arrives at weftos.weavelogic.ai, clicks "Try It Now," and within 10 seconds has a live clawft agent running in their browser via WASM — no download, no signup, no API key required (using free models via OpenRouter).

### Philosophy: Equip the Guide, Don't Script the Tour

We are NOT building a fixed step-by-step tour with pre-recorded responses. We are building an **intelligent tour guide agent** and giving it the tools, knowledge, and stage it needs to walk any visitor through WeftOS naturally.

What we provide:
1. **The knowledge** — an RVF vector knowledge base of all docs/code/architecture (see `07-rvf-knowledge-base-plan.md`)
2. **The tools** — WASM runtime with the capability system, running live in the browser
3. **The stage** — playground UI with panels the agent can populate (chat, code, docs links, provenance, graph)
4. **The personality** — a well-crafted system prompt that makes the agent a knowledgeable, helpful guide

The agent decides what to show based on what the visitor asks. If they ask "how does governance work?" — it pulls governance docs from the KB and explains. If they say "show me" — it runs a live demo. If they want to install — it walks them through it for their platform.

The tour guide gets smarter over time as we:
- Improve the RVF knowledge base (richer corpus, better chunking)
- Add tools to the WASM runtime (governance checks, provenance display)
- Add UI panels the agent can populate (knowledge graph viz, audit trail)
- Refine the system prompt based on real visitor interactions

### What the agent can demonstrate (capability roadmap)

These are capabilities the agent progressively gains — not scripted tour steps:

| Capability | What It Shows | Implementation |
|-----------|---------------|----------------|
| Answer questions about WeftOS | KB-grounded RAG responses with doc links | RVF KB + system prompt |
| Chat with any LLM | Multi-provider routing in action | Already working in WASM |
| Analyze code | Tool use and capability system | Code analysis in system prompt |
| Enforce governance rules | Constitutional AI, safety checks | Client-side simulation → WASM governance |
| Show conversation memory | Persistent context across messages | Already in BrowserRuntime |
| Display audit trail | ExoChain provenance for every action | ProvenancePanel UI + simulated chain |
| Visualize knowledge | ECC graph of concepts and relationships | KnowledgePanel UI + graph from KB metadata |
| Guide installation | Platform-specific install instructions | KB chunks with install docs |

---

## What Already Exists (v0.3.1)

### Browser WASM Runtime (`clawft-wasm` with `browser` feature)
- `init(config_json)` — initializes with provider config, sets up BrowserLlmClient
- `send_message(text)` — sends message through LLM pipeline with conversation history
- `BrowserLlmClient` — makes LLM API calls directly from browser (fetch API)
- 9 provider support (Anthropic, OpenAI, OpenRouter, DeepSeek, Groq, Gemini, xAI, custom)
- Provider auto-routing (longest-prefix matching, fallback chain)
- CORS proxy support for providers that block browser-direct
- Conversation history management
- ~59KB WASM binary (wasip2), browser build TBD size

### Browser Test Harness (`crates/clawft-wasm/www/`)
- `index.html` — Dark-themed chat UI with config panel
- `main.js` — WASM loading, API key management, obfuscation, CORS proxy injection
- Dev server with `/proxy/*` endpoint for CORS bypass
- Environment key auto-injection from `.env-keys.json`
- Status indicator (loading/ready/error states)
- Timestamp on messages, user/assistant/system/error message types

### Build Pipeline
- `scripts/build.sh browser` — builds `wasm32-unknown-unknown` with `--features browser`
- `wasm-bindgen` generates JS glue into `www/pkg/`
- `scripts/build.sh serve` — serves test harness on localhost:8080

---

## Architecture

### Option A: Embed in Fumadocs Site (Recommended)

Bundle the WASM module directly into the Next.js docs site. The playground becomes a React component that can appear on the landing page, in docs pages, or as a standalone `/playground` route.

```
docs/src/
├── app/
│   ├── playground/
│   │   └── page.tsx              ← Standalone playground page
│   └── page.tsx                  ← Landing page with embedded mini-playground
├── components/
│   └── playground/
│       ├── WasmPlayground.tsx    ← Main playground component
│       ├── ChatPanel.tsx         ← Chat message display
│       ├── ConfigPanel.tsx       ← API key + provider config
│       ├── GuidedTour.tsx        ← Step-by-step tour controller
│       ├── TourStep.tsx          ← Individual tour step with instructions
│       ├── CapabilitiesPanel.tsx  ← Shows available tools, governance rules
│       ├── ProvenancePanel.tsx   ← Shows ExoChain audit trail (simulated)
│       └── KnowledgePanel.tsx    ← Shows ECC graph visualization (simulated)
└── public/
    └── wasm/
        ├── clawft_wasm_bg.wasm   ← Pre-built WASM binary
        └── clawft_wasm.js        ← wasm-bindgen JS glue
```

### Option B: Iframe the Existing Harness

Serve the existing `www/` harness from a separate CDN/subdomain and iframe it into the docs site.

**Recommendation: Option A.** The iframe approach creates a disconnected experience. Embedding directly allows the playground to use Fumadocs theming, share state with the docs navigation, and progressively reveal tour steps alongside documentation content.

---

## Guided Tour Design

### Tour Structure

The tour is a sequence of steps. Each step has:
- **Instruction text** (what to do)
- **Explanation** (what's happening under the hood)
- **Auto-triggered action** or **user action** (do it or watch it)
- **Highlight** (which UI element to focus)
- **Completion criteria** (how we know the step succeeded)

### Step 1: "Meet Your Agent" (0 seconds — auto-start)

**What happens**: The WASM module loads automatically. A free OpenRouter model (llama-3.1-8b-instruct:free) is pre-configured. No API key needed.

**User sees**:
```
┌─────────────────────────────────────────────┐
│  ● clawft-wasm v0.3.1 loaded (47ms)        │
│                                              │
│  Welcome! You're running a clawft agent     │
│  entirely in your browser — no download,    │
│  no server, just WASM.                      │
│                                              │
│  Try saying hello.                          │
│                                              │
│  [Type a message...]              [Send]    │
└─────────────────────────────────────────────┘
```

**Under the hood**: "This is a full clawft agent runtime compiled to WebAssembly. It's making API calls to an LLM directly from your browser. The same code runs on Linux, macOS, Windows, Docker, and now your browser tab."

**Completion**: User sends a message and gets a response.

### Step 2: "Switch Providers" (after first response)

**What happens**: A provider selector appears showing the 9 supported providers.

**User action**: Switch from OpenRouter (free) to another provider by pasting an API key.

**Explanation**: "clawft routes to 9 LLM providers using longest-prefix matching. `anthropic/claude-sonnet-4-20250514` goes to Anthropic. `openai/gpt-4o` goes to OpenAI. Model names are the routing table — no config files needed."

**Completion**: User successfully sends a message through a different provider.

**Alternative**: Auto-demo by showing a split view — same prompt sent to two providers, responses side by side. Shows routing in action without requiring a second API key.

### Step 3: "Tools, Not Just Chat" (after provider demo)

**What happens**: The agent is given a task that requires tool use.

**Pre-loaded prompt**: "Analyze this code snippet for potential issues:"
```rust
fn process(data: &str) -> Result<(), Box<dyn Error>> {
    let parsed: Value = serde_json::from_str(data)?;
    let key = parsed["api_key"].as_str().unwrap();
    std::fs::write("/tmp/keys.log", key)?;
    Ok(())
}
```

**Explanation**: "clawft agents don't just generate text — they use tools. In the full runtime, agents can read files, search code, execute commands, and call APIs. Each tool is sandboxed with capability checks. In this browser demo, we're showing the analysis pipeline — in production, the agent would also fix the code."

**UI enhancement**: Show a "Tools Available" panel listing the 8 browser-available tools with locked/unlocked icons.

**Completion**: Agent identifies the security issues (hardcoded key logging, unwrap on untrusted input).

### Step 4: "Governance — Rules Your Agents Follow" (after tool demo)

**What happens**: Demonstrate constitutional governance.

**Scenario**: Show a pre-configured governance rule: "Never output API keys, credentials, or secrets."

**User action**: Ask the agent "What was the API key in that code snippet?"

**Expected behavior**: The agent refuses or redacts, citing the governance rule.

**Explanation**: "WeftOS has a three-branch governance system — like a constitution for AI. Rules are defined declaratively, checked at every stage of the pipeline, and violations are logged to the audit trail. This isn't prompt engineering — it's enforcement at the kernel level."

**UI enhancement**: Show the governance rule firing in real-time — a sidebar showing "Rule triggered: SECRET_REDACTION — blocked output containing credential pattern."

**Note**: This requires either wiring the governance system into the WASM build, or simulating it client-side for the demo. For v1, client-side simulation with realistic behavior is acceptable, with a note: "In the full runtime, this is enforced by the WeftOS kernel — not client-side JavaScript."

### Step 5: "Memory That Persists" (after governance demo)

**What happens**: Demonstrate that the agent remembers context across messages.

**User action**: Tell the agent something specific ("My project uses PostgreSQL 15 and is deployed on AWS ECS"), then later ask a question that requires that context ("What database should I use for the migration?").

**Explanation**: "Most AI tools start from zero every conversation. clawft maintains conversation history in the runtime. In the full WeftOS deployment, the ECC cognitive layer builds a persistent knowledge graph — your agent remembers what it learned last week, traces cause and effect, and gets smarter over time."

**UI enhancement**: Show a mini knowledge graph visualization — nodes appearing as the agent learns facts. Even if simplified, this visual is extremely compelling.

**Completion**: Agent correctly recalls earlier context in its response.

### Step 6: "Provenance — Prove Every Decision" (after memory demo)

**What happens**: Show the audit trail for the entire conversation.

**UI enhancement**: An "Audit Trail" tab that shows:
```
Event #1  [SHAKE-256: a4f2...] Agent initialized
Event #2  [SHAKE-256: b7e1...] User message received
Event #3  [SHAKE-256: c3d8...] LLM request sent (provider: openrouter, model: llama-3.1-8b)
Event #4  [SHAKE-256: d9a4...] LLM response received (tokens: 247, cost: $0.00)
Event #5  [SHAKE-256: e2f6...] Governance check: PASS
Event #6  [SHAKE-256: f1b3...] Response delivered to user
```

**Explanation**: "Every action your agent takes is logged to an append-only chain with cryptographic hashes. You can verify that no event was tampered with, trace exactly why a decision was made, and produce compliance reports. This is ExoChain — a tamper-evident audit trail for AI."

**Note**: For v1, this can be simulated client-side with realistic hash values. The actual ExoChain is in the Rust kernel and not yet wired to the browser WASM build.

### Step 7: "What's Next" (tour completion)

**Shows**:
- "Install for real" — curl command, Docker, crates.io
- "Read the docs" — link to /docs/clawft/getting-started
- "Deploy WeftOS" — link to /docs/weftos (kernel, mesh, governance)
- "Get an assessment" — link to weavelogic.ai (commercial offering)
- "Star on GitHub" — repo link

---

## Implementation Phases

### Phase 1: Stage + Brain (MVP) — Est. 6-8 hours

**Goal**: WASM playground with RVF knowledge base — the tour guide can answer questions about WeftOS grounded in real documentation.

**Tasks**:
1. Build `build-kb` tool (Rust binary that reads MDX → chunks → embeds → outputs RVF)
2. Generate `weftos-docs.rvf.json` from the 68 doc pages
3. Build browser WASM and copy to `docs/src/public/wasm/`
4. Create `WasmPlayground.tsx` component (loads WASM + KB, wires chat)
5. Write the tour guide system prompt (personality, KB-grounded RAG, doc linking)
6. Add `/playground` route + "Try It Now" on landing page
7. Handle CORS via Vercel API route proxy or OpenRouter browser-direct

**The agent can**: Answer any question about WeftOS with grounded, accurate responses and link to the right documentation page. It's already useful — a smart assistant that knows the project inside out.

**Deliverable**: /playground with a knowledgeable tour guide agent.

### Phase 2: Better Equipment — Est. 4-6 hours

**Goal**: Give the tour guide more tools and UI panels to work with.

**Tasks**:
1. `ConfigPanel.tsx` — provider selector, API key input (upgrade from free model)
2. `CodePanel.tsx` — code display panel the agent can populate with examples
3. Provider switching demo (agent can show the same query routed to different providers)
4. Improve system prompt with code analysis capability
5. Add Transformers.js option for semantic KB search (vs keyword fallback)

**The agent can**: Show code examples inline, demonstrate provider routing, analyze code snippets visitors paste in.

### Phase 3: Provenance + Governance Panels — Est. 6-8 hours

**Goal**: Give the tour guide visual panels for provenance and governance demos.

**Tasks**:
1. `ProvenancePanel.tsx` — hash-linked event timeline the agent populates
2. `GovernancePanel.tsx` — rule display, violation alerts
3. Client-side governance simulation (pattern matching + rule display)
4. WITNESS chain verification display for the KB itself
5. Improve system prompt to use panels when demonstrating features

**The agent can**: Show a live audit trail of the conversation, demonstrate governance rules firing, and verify the integrity of its own knowledge base as a provenance demo.

### Phase 4: Knowledge Graph + Polish — Est. 6-8 hours

**Goal**: Visual knowledge graph and production readiness.

**Tasks**:
1. `KnowledgePanel.tsx` — graph visualization (React Flow) showing concept relationships
2. Build concept graph from KB metadata (tags, cross-references between chunks)
3. Mobile responsive design, dark/light mode
4. Loading states, error handling, accessibility
5. Pre-built WASM + KB in CI pipeline
6. Analytics on playground usage (what questions do people ask?)

**The agent can**: Show a visual map of WeftOS concepts, highlight which parts of the knowledge graph it's using to answer, and provide a fully polished experience.

---

## Technical Decisions

### WASM Binary Distribution

**Option A**: Check WASM binary into git (in `docs/src/public/wasm/`)
- Pro: Simple, always available
- Con: ~1-5MB binary in git, bloats repo

**Option B**: Build in CI, deploy as Vercel asset
- Pro: No binary in git
- Con: Requires CI changes, build step dependency

**Option C**: Load from GitHub Release assets at runtime
- Pro: No binary in git or CI changes
- Con: Cross-origin loading, CDN dependency, version pinning

**Recommendation**: Option B for production, Option A for development. Add a `scripts/build.sh playground` command that builds the WASM and copies to `docs/src/public/wasm/`.

### Free Model Strategy

The playground MUST work without any API key for the "zero friction" experience.

**Options**:
1. **OpenRouter free tier** — llama-3.1-8b-instruct:free, no key required for low volume
   - Risk: Rate limits, model quality varies
2. **WeaveLogic proxy** — run a small API proxy that provides limited free access
   - Pro: Full control, any model
   - Con: Cost, infrastructure to maintain
3. **Local inference via WebLLM** — run a small model entirely in-browser
   - Pro: Zero API calls, works offline
   - Con: Large download (100MB+), slow on weak hardware

**Recommendation**: Option 1 (OpenRouter free) for v1, with Option 3 (WebLLM) as a future "offline mode" toggle. OpenRouter's free models work well enough for a demo and require zero infrastructure.

**Actually**: OpenRouter free models DO require an API key. For true zero-key experience, we'd need either:
- A WeaveLogic API proxy that rate-limits demo usage
- Or WebLLM for in-browser inference
- Or a pre-recorded demo mode that replays realistic responses

**Updated recommendation**: Start with a "demo mode" that replays pre-recorded responses for the guided tour steps (fast, reliable, no API key), with an "API mode" toggle where users can paste their own key for live interaction. This is what Vercel's AI SDK playground does.

### Governance Simulation

The WeftOS governance system (constitutional checks, effect vectors, gate backend) is in the Rust kernel and not compiled into the browser WASM build. For the playground:

**Phase 1**: Client-side simulation — JavaScript intercepts messages, checks patterns, shows governance UI
**Phase 2**: Compile governance crate to WASM (if size budget allows)
**Phase 3**: Full kernel-in-browser (long-term)

The simulation must be clearly labeled: "In the full runtime, governance is enforced at the kernel level."

### Knowledge Graph Visualization

**Options**:
1. D3.js force-directed graph — full control, complex
2. vis.js / vis-network — simpler API, good defaults
3. React Flow — React-native, good for node-based UIs
4. Mermaid.js — simple, already supported in Fumadocs
5. Custom SVG — lightweight, tailored to our needs

**Recommendation**: React Flow for the interactive playground (drag nodes, zoom, click for details). Mermaid for static diagrams in docs pages.

---

## Success Metrics

| Metric | Target |
|--------|--------|
| Time to first interaction | < 10 seconds (WASM load + auto-init) |
| Tour completion rate | > 40% of starters finish all 7 steps |
| Playground → docs navigation | > 25% click through to docs |
| Playground → install | > 10% proceed to install |
| WASM binary size | < 2MB gzipped |
| Works without API key | Yes (demo mode) |
| Mobile responsive | Yes |

---

## Dependencies

- `clawft-wasm` browser build working (`scripts/build.sh browser`)
- `wasm-bindgen` CLI installed
- Next.js 16 dynamic import for WASM modules
- Vercel supports WASM files in `public/` (confirmed)
- OpenRouter API or demo mode for zero-key experience

---

## Open Questions

1. **CORS for LLM providers**: Which providers allow browser-direct? Anthropic does (confirmed in code). OpenRouter? Need to test.
2. **WASM binary size**: The browser build with all features — how big is it? Need to measure.
3. **Governance in WASM**: Can we compile `clawft-kernel` governance module to `wasm32-unknown-unknown`? It doesn't depend on tokio/reqwest, so it might work.
4. **WebLLM integration**: Would adding WebLLM as a provider in `clawft-llm` be feasible? It would enable fully offline playground.
5. **State persistence**: Should playground state persist in IndexedDB so users can resume tours?
