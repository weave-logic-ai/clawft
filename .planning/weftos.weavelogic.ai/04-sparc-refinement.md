# SPARC Refinement — weftos.weavelogic.ai Improvements

## Phase: R (Refinement)

### 1. Priority Refinement

After reviewing all five agent reports, the following priority ordering reflects cross-agent consensus weighted by impact on developer adoption:

#### P0 — Must Do (blocks adoption)
| Task | Sprint | Est. | Files |
|------|--------|------|-------|
| Fix `<title>` from "clawft" to "WeftOS" | 1 | 15 min | layout.tsx |
| Rewrite hero section (headline + CTAs) | 1 | 1 hr | page.tsx |
| Rewrite layer cards (benefits, not features) | 1 | 30 min | page.tsx |
| Create /docs index page | 2 | 1 hr | content/docs/index.mdx, meta.json |
| Three-tier quickstart rewrite | 3 | 2 hr | getting-started.mdx |

#### P1 — Should Do (improves conversion)
| Task | Sprint | Est. | Files |
|------|--------|------|-------|
| Add footer (GitHub, license, WeaveLogic link) | 1 | 30 min | layout.tsx or page.tsx |
| Rewrite feature highlights section | 1 | 30 min | page.tsx |
| Add previous/next navigation | 2 | 30 min | source.config.ts, Fumadocs config |
| Add "Edit on GitHub" links | 2 | 15 min | source.config.ts |
| Add expected output to all command examples | 3 | 1 hr | getting-started.mdx |

#### P2 — Nice to Have (builds credibility)
| Task | Sprint | Est. | Files |
|------|--------|------|-------|
| Create glossary page | 2 | 1 hr | glossary.mdx |
| Add use case examples (4 pages) | 4 | 3 hr | examples/*.mdx |
| Add performance benchmarks | 4 | 2 hr | benchmarks.mdx |
| Accessibility fixes | 5 | 1 hr | layout.tsx, page.tsx |
| Brand continuity with weavelogic.ai | 5 | 1 hr | layout.tsx, global.css |
| Social proof improvements | 5 | 30 min | page.tsx |
| Compliance mapping on security page | 4 | 1 hr | security.mdx |

### 2. Copy Refinement

All copy has been refined through three review passes (marketing agent, copy agent, competitive agent). Final versions:

#### Hero Headline Options (ranked)
1. "The AI framework that remembers everything." — Clear, benefit-driven, memorable
2. "Build AI agents you can actually trust." — Trust-focused, enterprise-friendly
3. "Ship agents that survive in production." — Operations-focused, Rust advantage implicit

**Recommendation**: Option 1 as primary, with option 2 as A/B test variant.

#### Hero Description (final)
> Build agents that persist knowledge across sessions, prove why they made every decision, and coordinate across machines — all in a single Rust runtime. Open source, production-tested with 3,900+ tests.

#### Layer Card Copy (final)
- **clawft**: "Run agents anywhere — Connect to any LLM, deploy on any channel (Slack, Teams, web, CLI), and let agents learn new skills automatically. Native binaries, browser WASM, or Docker."
- **WeftOS Kernel**: "Manage agents like processes — Every agent gets a PID, a cryptographic audit trail, and governance rules. When agents fail, the supervisor restarts them. When they misbehave, constitutional checks stop them."
- **ECC**: "Give agents a brain — A knowledge graph that grows with every interaction. Your agents remember what they learned last week, trace cause and effect, and get smarter over time."

### 3. Architectural Refinement

After reviewing the Fumadocs documentation and source.config.ts:

- **Previous/Next navigation**: Fumadocs supports this via `DocsPage` component props. Need to verify `lastUpdate`, `editOnGithub`, and pagination are configured.
- **Docs index**: Fumadocs uses `content/docs/index.mdx` as the root page. If it doesn't exist, /docs 404s. Creating this file is the fix.
- **Footer**: Can be added in the `RootLayout` component in `layout.tsx` to appear on all pages, or in `page.tsx` for landing page only. Recommend `layout.tsx` for consistency.

### 4. Risk Refinement

| Risk | Status | Mitigation |
|------|--------|------------|
| Fumadocs index.mdx might conflict with existing meta.json | Low | Test locally with `npm run build` before pushing |
| Performance benchmarks might show unfavorable comparisons | Low | Only benchmark clear Rust advantages (spawn time, memory, binary size) |
| Three-tier quickstart requires Docker image to be working | Confirmed | v0.3.1 Docker image is live on GHCR |
| Copy changes need to be tested for dark/light mode | Low | Review both modes after each change |

### 5. Iteration Plan

**Iteration 1**: Sprint 1 (P0 + P1 from Sprint 1) — ship in one commit
**Iteration 2**: Sprint 2 (docs structure) — ship in one commit
**Iteration 3**: Sprint 3 (quickstart) — ship in one commit
**Iteration 4**: Sprint 4+5 (content + polish) — can be parallelized with agents

Each iteration triggers a Vercel deploy. Verify live site after each push.
