# SPARC Specification — weftos.weavelogic.ai Improvements

## Phase: S (Specification)

### 1. Project Scope

Transform weftos.weavelogic.ai from internal engineering documentation into a developer-facing product site that converts curious visitors into WeftOS users. The site must answer "what problem does this solve?" and "why should I try it?" within 5 seconds, provide a working quickstart under 5 minutes, and establish visual/brand continuity with weavelogic.ai.

### 2. Target Audiences

**Primary: Platform Engineer Building AI Infrastructure**
- Title: Senior/Staff Engineer, Platform Engineer, ML Infrastructure Engineer
- Pain: Every AI framework (LangChain, CrewAI, AutoGen) has the same gaps — no memory between sessions, no audit trail, no governance, Python-only
- Trigger: Hit a wall with existing frameworks, or needs to justify build vs buy
- Needs to hear: "Rust-native, persistent memory, cryptographic audit trail, 3,900+ tests"

**Secondary: Open Source Evaluator**
- Scanning GitHub for agent frameworks, comparing stars/activity/architecture
- Needs: Quick install, working example, clear architecture overview
- Needs to hear: "curl install, one command, here's what it does that LangChain can't"

### 3. Success Metrics

| Metric | Current | Target | Measurement |
|--------|---------|--------|-------------|
| 5-second comprehension | 2/5 (UX audit) | 4/5 | User testing |
| Time to first "hello world" | 15+ min (source build) | < 2 min | Timed walkthrough |
| Landing page CTAs | 0 | 2 (Get Started + GitHub) | Page audit |
| Docs index at /docs | 404 | Working page | HTTP check |
| HTML title accuracy | "clawft" | "WeftOS" | DOM check |
| Cross-site navigation | None | Bidirectional links | Page audit |
| Messaging score | 3.0/10 | 7+/10 | Copy review |
| UX heuristic average | 3.5/5 | 4.0/5 | Heuristic evaluation |

### 4. Functional Requirements

#### FR-1: Landing Page Rewrite
- Benefit-driven hero headline (problem → solution, not feature inventory)
- Primary CTA button: "Get Started in 5 Minutes" → quickstart
- Secondary CTA: "View on GitHub" with star count
- Hero visual: terminal recording or animated demo
- Remove acronyms from above-fold copy (ECC, DEMOCRITUS, HNSW)
- Rewrite layer cards as benefits, not feature lists
- Add footer with GitHub, license, community, company links
- Move badges below the fold into a "Status" section
- Fix `<title>` tag from "clawft" to "WeftOS"

#### FR-2: Quickstart Overhaul
- Pre-built binary install path (30 seconds)
- Docker one-liner path (60 seconds)
- Show expected output for every command
- List prerequisites before install commands
- Add troubleshooting section
- Three-tiered entry: "Try it" (Docker demo) → "Build something" (10 lines) → "Go deep" (full tutorial)

#### FR-3: Docs Index Page
- Create /docs landing page (currently 404)
- Explain clawft/WeftOS relationship
- Recommended reading order for newcomers
- Quick links to both quickstarts
- Search integration

#### FR-4: Information Architecture Improvements
- Add previous/next page navigation at bottom of doc pages
- Add "Edit this page on GitHub" links
- Create glossary page for terms (ExoChain, ECC, DEMOCRITUS, etc.)
- Add breadcrumbs (Fumadocs supports this)
- Fix heading hierarchy (H3 before H2 on landing page)

#### FR-5: Brand Continuity
- Add "By WeaveLogic" link in header or footer
- Add "Need enterprise support?" CTA linking to weavelogic.ai/contact
- Align primary brand colors with weavelogic.ai
- Consistent product hierarchy: WeftOS (platform) > clawft (runtime) > ECC (cognitive)

#### FR-6: Social Proof & Trust Signals
- Dynamic GitHub stars badge
- crates.io download counts
- Link to Discord/community channel (if exists)
- "Built With" section showing WeaveLogic consulting use
- Performance benchmarks section (agent spawn time, memory footprint, vs Python)

#### FR-7: Use Case Examples
- 3-4 concrete examples with 20-line configs and terminal output:
  1. DevOps automation agent with governance constraints
  2. Code review agent with cryptographic audit trail
  3. Multi-agent research swarm with mesh networking
  4. Compliance-aware bot with constitutional governance

#### FR-8: Accessibility
- Skip-to-content link
- Heading hierarchy audit
- Code block aria-labels
- Copy confirmation toast on code blocks
- Keyboard shortcut for previous/next page

### 5. Non-Functional Requirements

- All changes must be in `docs/src/` directory
- Build must pass: `npm run build` (Next.js 16 + Fumadocs)
- No changes to Rust codebase required
- Must work in both light and dark mode
- Mobile responsive
- Deploy via existing Vercel pipeline (push to master)

### 6. Out of Scope

- Rust codebase changes
- weavelogic.ai site changes (separate project)
- npm package creation (@weftos/core)
- API reference generation (separate skill: weftos-api-docs)
- Community infrastructure (Discord server setup)

### 7. Dependencies

- Marketing review completed (this document)
- v0.3.1 deployed to all channels (confirmed 2026-04-02)
- Vercel project "clawft" configured with Root Directory = docs/src (confirmed)
- Fumadocs v16.7.6 + Next.js 16 + Tailwind 4 stack (confirmed)

### 8. Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Copy changes don't resonate | Medium | Medium | A/B test key headlines if PostHog added |
| Performance benchmarks don't favor WeftOS | Low | High | Only publish benchmarks where Rust wins clearly |
| Breaking docs build | Low | Medium | Run `npm run build` before every commit |
| Scope creep into Rust codebase | Medium | Low | Strict out-of-scope boundary |
