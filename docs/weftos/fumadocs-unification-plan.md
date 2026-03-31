# Fumadocs Unification Plan: weftos.weavelogic.ai

**Date**: 2026-03-27
**Author**: Documentation Unification Team
**Status**: Plan -- pending review and execution
**Target**: Deploy unified documentation at `weftos.weavelogic.ai`

---

## 1. Audit Results

### Stack A: Fumadocs Site (`docs/src/`)

**Technology**: Next.js 16 + Fumadocs Core 16.7.6 + Fumadocs MDX 14.2.11 + React 19 + Tailwind 4

**Configuration**:
- `source.config.ts` defines content directory as `content/docs`
- `lib/source.ts` sets `baseUrl: '/docs'`
- Two doc sections with `root: true` in their respective `meta.json` files
- Full-text search API at `/api/search` via `createFromSource`
- Static generation via `generateStaticParams`

**Page Inventory**:

| Section | Pages | URL Prefix | Topics |
|---------|------:|------------|--------|
| clawft | 13 | `/docs/clawft/` | Framework docs: architecture, getting-started, CLI, config, plugins, providers, channels, tools, skills, browser, deployment, security |
| weftos | 25 | `/docs/weftos/` | Kernel docs: architecture, kernel-phases, boot-sequence, process-table, IPC, capabilities, exochain, governance, WASM sandbox, containers, app-framework, mesh-networking, discovery, clustering, security, ECC, democritus, self-healing, observability, persistence, config-auth, GUI, decisions, kernel-guide |

**Total Fumadocs pages: 38** (13 clawft + 25 weftos)

**Landing page**: Custom home at `app/page.tsx` with two-card layout linking to `/docs/clawft` and `/docs/weftos`. Title says "clawft" throughout -- no WeftOS branding for the site itself.

**Build status**: Has `node_modules/` present (dependencies installed). Scripts: `dev`, `build`, `start`, `postinstall`. Should be buildable.

**Quality assessment**: The Fumadocs weftos pages are **comprehensive and high-quality**. The `index.mdx` alone is 171 lines with architecture diagrams, feature flag tables, phase roadmap, crate dependency graph, and documentation map. The `architecture.mdx` is 243 lines with a 5-layer diagram, crate structure listing all 25+ files, key types with Rust signatures, feature gate matrix, and boot/shutdown sequences. These are the richest docs in the project.

---

### Stack B: Standalone Markdown (`docs/` excluding `docs/src/`)

**File Inventory** (72 files across 12 directories):

#### docs/weftos/ (top-level kernel docs -- 12 files)

| File | Lines (approx) | Currency | Content |
|------|------:|----------|---------|
| `quickstart.md` | 174 | CURRENT | Zero-to-running guide, project init, boot, weave CLI |
| `INSTALL.md` | 118 | CURRENT | Prerequisites, source/Docker/debug builds, ONNX, troubleshooting |
| `configuration-reference.md` | 368 | CURRENT | Full weave.toml reference with every field, types, defaults, examples |
| `feature-flags.md` | 301 | CURRENT | All 22 crates' feature flags, dependency graph, recommended combos |
| `VISION.md` | 748 | CURRENT | Product thesis, arc from agency to OS, problem/solution, differentiation |
| `architecture.md` | ~200 | CURRENT | Crate map, kernel modules table, 5-layer diagram |
| `k-phases.md` | ~200 | CURRENT | K0-K6 status tables with component/file/test counts |
| `kernel-modules.md` | ~400 | CURRENT | Per-module reference for all 25 source files |
| `kernel-governance.md` | ~300 | CURRENT | Three-branch governance, effect algebra, dual-layer enforcement |
| `integration-patterns.md` | ~250 | CURRENT | ExoChain/ResourceTree/Governance integration guide for K3+ devs |
| `FEATURE_GATES.md` | ~50 | STALE | Older version superseded by `feature-flags.md` |
| `CONFIGURATION.md` | ~200 | PARTIAL | JSON config reference (clawft agent/gateway), overlaps with `configuration-reference.md` |
| `k6-development-notes.md` | ~150 | CURRENT | K6 mesh implementation notes |
| `weftos-gui-specifications.md` | ~800 | CURRENT | K8 GUI vision, agent-extensible UI, Tauri architecture |

#### docs/weftos/specs/ (4 files -- new, from Sprint 11 Team 5)

| File | Currency | Content |
|------|----------|---------|
| `block-catalog.md` | CURRENT | Block types, Zod schemas, rendering targets, ports/actions |
| `console-commands.md` | CURRENT | Console command catalog, shell syntax, governance display |
| `journey-spec.md` | CURRENT | Journey DAG format, step structure, breakout support |
| `mentra-hud-constraints.md` | CURRENT | HUD renderer constraints, 400x240 display, deployment model |

#### docs/weftos/09-symposium/ (4 files)

| File | Content |
|------|---------|
| `00-symposium-overview.md` | K2 symposium plan |
| `01-graph-findings.md` | Graph analysis results |
| `02-decision-resolutions.md` | Decisions from K2 symposium |
| `03-weaver-review.md` | Weaver crate architecture review |

#### docs/weftos/k2-symposium/ (10 files)

K2 symposium: platform vision, K0-K2 audit, A2A services, industry landscape, RUV ecosystem, K3 integration, Q&A roundtable, results report, K2.1 follow-up.

#### docs/weftos/k3-symposium/ (7 files)

K3 symposium: tool catalog audit, lifecycle chain integrity, sandbox security, agent loop dispatch, live testing, Q&A, results.

#### docs/weftos/ecc-symposium/ (8 files)

ECC symposium: research synthesis, gap analysis, documentation update guide, Q&A, results, three modes, NetworkNav analysis, market opportunities.

#### docs/weftos/k5-symposium/ (6 files)

K5 symposium: mesh architecture, K6 readiness assessment, security and identity, K6 implementation plan, results.

#### docs/weftos/sprint11-symposium/ (12 files)

Sprint 11 symposium: opening plenary, pattern extraction, testing strategy, release engineering, UI/UX design, changelog, Mentra integration, algorithmic optimization, codebase report, optimization plan, synthesis, v0.1.0 release notes.

#### docs/weftos/sparc/ (1 file)

| File | Content |
|------|---------|
| `k6-cluster-networking.md` | SPARC plan for K6 |

#### docs/weftos/ (analysis outputs -- 3 files)

| File | Content |
|------|---------|
| `weaver-analysis-clawft.md` | First Weaver self-analysis (mid-session) |
| `weaver-analysis-v2.md` | Second Weaver analysis (post-sprint) |
| `external-analysis-results.md` | Weaver analysis of ruvector (external project) |

#### docs/ (framework-level -- 15 files)

| Directory | Files | Currency | Content |
|-----------|------:|----------|---------|
| `getting-started/` | 1 (`quickstart.md`) | CURRENT | clawft quickstart (agent CLI, not kernel) |
| `deployment/` | 3 (`docker.md`, `release.md`, `wasm.md`) | CURRENT | Docker, release process, WASM deployment |
| `development/` | 2 (`contributing.md`, `testing-three-workstreams.md`) | CURRENT | Dev setup, testing guide |
| `architecture/` | 2 (`overview.md`, `wasm-browser-portability-analysis.md`) | CURRENT | clawft crate architecture, WASM analysis |
| `reference/` | 4 (`cli.md`, `config.md`, `security.md`, `tools.md`) | CURRENT | CLI reference, config, security, tools |
| `guides/` | 10 files | CURRENT | channels, plugins, providers, routing, MCP, RVF, skills, tool-calls, workspaces, testing-mcp |
| `benchmarks/` | 1 (`results.md`) | CURRENT | Benchmark results |
| `skills/clawft/` | 5 files | CURRENT | Agent skill definitions |
| `external/` | 1 (`commander-ne-v11.5.19.1.md`) | REFERENCE | Commander library docs |

**Total standalone markdown files: ~87** (12 weftos top-level + 4 specs + 47 symposium + 3 analysis + 1 sparc + 15 framework + 5 skills)

---

### Overlap Analysis

The following content exists in BOTH stacks with varying degrees of overlap:

| Topic | Fumadocs (`docs/src/content/docs/weftos/`) | Standalone (`docs/weftos/`) | Overlap |
|-------|---------------------------------------------|------------------------------|---------|
| Architecture | `architecture.mdx` (243 lines, rich) | `architecture.md` (~200 lines) | HIGH -- both cover 5-layer arch, crate structure |
| Kernel phases | `kernel-phases.mdx` | `k-phases.md` | HIGH -- both cover K0-K6 status |
| Governance | `governance.mdx` | `kernel-governance.md` | MEDIUM -- standalone is more detailed |
| Feature flags | (embedded in `index.mdx`) | `feature-flags.md` (301 lines) | LOW -- standalone is far more complete |
| Configuration | `config-auth.mdx` | `configuration-reference.md` + `CONFIGURATION.md` | LOW -- standalone has full weave.toml ref |
| Boot sequence | `boot-sequence.mdx` | (in architecture docs) | LOW |
| GUI | `gui.mdx` | `weftos-gui-specifications.md` | LOW -- both exist but cover different depths |

**Key finding**: The Fumadocs WeftOS pages are well-written but represent a subset of the information in standalone docs. The standalone docs contain significantly more detail on configuration, feature flags, governance, and integration patterns. Additionally, the standalone docs contain entire categories (specs, symposium results, analysis outputs, quickstart, install guide) that have NO equivalent in Fumadocs.

---

## 2. Unified Site Structure for weftos.weavelogic.ai

The site should be WeftOS-focused. The clawft framework docs can remain as a section but the site identity should be WeftOS. Below is the proposed information architecture.

### Navigation Tree

```
weftos.weavelogic.ai
|
+-- / (Landing page -- rebrand from "clawft" to "WeftOS")
|
+-- /docs/
    |
    +-- getting-started/           [SECTION: Getting Started]
    |   +-- index                  Overview + what is WeftOS
    |   +-- quickstart             Zero to running kernel
    |   +-- installation           Build from source, Docker, cargo install
    |   +-- first-project          Project init, weave.toml, boot, status
    |
    +-- concepts/                  [SECTION: Concepts]
    |   +-- index                  WeftOS architecture overview
    |   +-- kernel-phases          K0-K8 phase roadmap
    |   +-- boot-sequence          Kernel lifecycle state machine
    |   +-- process-table          PID allocation, process management
    |   +-- ipc                    IPC, A2A routing, pub/sub
    |   +-- capabilities           RBAC, capability-based access control
    |   +-- exochain               Cryptographic audit trail
    |   +-- governance             Three-branch constitutional governance
    |   +-- ecc                    Ephemeral Causal Cognition (CMVG)
    |   +-- mesh-networking        P2P encrypted mesh layer
    |   +-- discovery              Peer discovery (mDNS, DHT, seed)
    |   +-- clustering             Distributed state, Raft consensus
    |   +-- wasm-sandbox           Wasmtime tool sandbox
    |   +-- containers             Container lifecycle management
    |   +-- app-framework          Application manifests, agent spawning
    |   +-- self-healing           OS patterns, dead-letter queue
    |   +-- observability          Metrics, structured logging
    |   +-- persistence            State persistence, checkpointing
    |   +-- democritus             Democritus subsystem
    |
    +-- guides/                    [SECTION: Guides]
    |   +-- index                  Guide index
    |   +-- configuration          weave.toml complete reference
    |   +-- feature-flags          Compile-time feature flags (all 22 crates)
    |   +-- integration-patterns   ExoChain/Tree/Governance integration for devs
    |   +-- deployment-docker      Docker deployment
    |   +-- deployment-wasm        WASM deployment
    |   +-- deployment-release     Release process and versioning
    |   +-- kernel-modules         Per-module reference (25 source files)
    |   +-- gui                    K8 GUI layer (React, Tauri, WebSocket)
    |
    +-- reference/                 [SECTION: Reference]
    |   +-- index                  Reference overview
    |   +-- console-commands       WeftOS console command catalog
    |   +-- block-catalog          Block types, schemas, rendering targets
    |   +-- block-descriptor       (TO WRITE) Block descriptor JSON schema
    |   +-- journey-spec           Journey DAG format specification
    |   +-- mentra-hud             Mentra HUD renderer constraints
    |   +-- config-auth            Authentication and authorization config
    |   +-- api                    (LINK) Rustdoc API reference
    |
    +-- vision/                    [SECTION: Vision & Strategy]
    |   +-- index                  The WeftOS vision (from VISION.md)
    |   +-- decisions              Symposium-derived design decisions
    |   +-- roadmap                (TO WRITE) Public roadmap
    |
    +-- contributing/              [SECTION: Contributing]
    |   +-- index                  Development setup, getting started
    |   +-- testing                Testing strategy (three workstreams)
    |   +-- architecture           clawft crate architecture overview
    |
    +-- clawft/                    [SECTION: clawft Framework] (existing, keep as-is)
    |   +-- (existing 13 pages)
    |
    +-- (symposium/ NOT in nav -- internal only, keep as standalone markdown)
```

### Page Count Summary

| Section | Pages | Source | Gaps (new content needed) |
|---------|------:|--------|--------------------------|
| Getting Started | 4 | Standalone: `quickstart.md`, `INSTALL.md`; Fumadocs: `index.mdx` | 1 gap: `first-project` (extract from quickstart) |
| Concepts | 17 | Fumadocs: 17 existing weftos MDX pages (reuse nearly all) | 0 gaps |
| Guides | 9 | Standalone: `configuration-reference.md`, `feature-flags.md`, `integration-patterns.md`, `kernel-modules.md`; Existing: deployment/, gui.mdx | 0 gaps |
| Reference | 7 | Standalone specs: 4 files; Fumadocs: `config-auth.mdx` | 1 gap: `block-descriptor` schema page; 1 link: rustdoc |
| Vision | 3 | Standalone: `VISION.md`, Fumadocs: `decisions.mdx` | 1 gap: public `roadmap` page |
| Contributing | 3 | Standalone: `contributing.md`, `testing-three-workstreams.md`, `overview.md` | 0 gaps |
| clawft | 13 | Fumadocs: existing 13 pages | 0 gaps |
| **Total** | **56** | | **3 gaps** |

### Content NOT included in the public site (internal only)

These files remain as standalone markdown. They are working documents, not public documentation:

- `docs/weftos/09-symposium/` (4 files) -- K2 symposium working docs
- `docs/weftos/k2-symposium/` (10 files) -- K2 symposium sessions
- `docs/weftos/k3-symposium/` (7 files) -- K3 symposium sessions
- `docs/weftos/ecc-symposium/` (8 files) -- ECC symposium sessions
- `docs/weftos/k5-symposium/` (6 files) -- K5 symposium sessions
- `docs/weftos/sprint11-symposium/` (12 files) -- Sprint 11 symposium sessions
- `docs/weftos/weaver-analysis-clawft.md` -- internal analysis output
- `docs/weftos/weaver-analysis-v2.md` -- internal analysis output
- `docs/weftos/external-analysis-results.md` -- internal analysis output
- `docs/weftos/sparc/` -- SPARC plans
- `docs/weftos/FEATURE_GATES.md` -- superseded by `feature-flags.md`
- `docs/weftos/CONFIGURATION.md` -- superseded by `configuration-reference.md`
- `docs/skills/clawft/` -- agent skill definitions (internal)
- `docs/external/` -- third-party library docs
- `docs/benchmarks/` -- internal benchmarks

Total internal files excluded: ~52 files. These stay in the repo but are not served by the documentation site.

---

## 3. Sync Strategy Recommendation

### Recommended approach: (a) Fumadocs is the source of truth

All public documentation lives in `docs/src/content/docs/`. Standalone markdown files in `docs/weftos/` and `docs/` are either:

1. **Migrated** into Fumadocs MDX pages (for content that belongs in the public site)
2. **Left as internal docs** (symposium results, analysis outputs, SPARC plans)
3. **Deleted** (superseded duplicates like `FEATURE_GATES.md`)

### Rationale

**Why not (b) standalone as source of truth?**
Fumadocs provides search, navigation, table of contents, dark mode, responsive layout, and static export. Extracting content from flat markdown into a site at build time would require a custom loader, custom frontmatter handling, and custom navigation generation. Fumadocs already solves this. The existing Fumadocs pages are the highest-quality docs in the project -- they should be the canonical format, not the derivative.

**Why not (c) hybrid?**
Two sources of truth means two places to update. When a kernel module changes, do you update the MDX page, the standalone markdown, or both? Hybrid approaches always drift. The symposium and analysis docs are genuinely different (internal working documents vs public documentation), so they naturally stay outside the site -- but that is not a "hybrid" strategy, it is simply excluding internal docs from the public site.

### Migration Steps

1. **Move new content into Fumadocs**: Convert the following standalone files into MDX pages in `docs/src/content/docs/weftos/` (or new section directories):
   - `quickstart.md` -> `getting-started/quickstart.mdx`
   - `INSTALL.md` -> `getting-started/installation.mdx`
   - `configuration-reference.md` -> `guides/configuration.mdx`
   - `feature-flags.md` -> `guides/feature-flags.mdx`
   - `kernel-modules.md` -> `guides/kernel-modules.mdx`
   - `integration-patterns.md` -> `guides/integration-patterns.mdx`
   - `kernel-governance.md` -> merge into existing `governance.mdx` (add detail)
   - `VISION.md` -> `vision/index.mdx`
   - `specs/block-catalog.md` -> `reference/block-catalog.mdx`
   - `specs/console-commands.md` -> `reference/console-commands.mdx`
   - `specs/journey-spec.md` -> `reference/journey-spec.mdx`
   - `specs/mentra-hud-constraints.md` -> `reference/mentra-hud.mdx`
   - Docker, WASM, release deployment docs -> `guides/deployment-*.mdx`
   - Contributing, testing -> `contributing/*.mdx`

2. **Update navigation**: Restructure `meta.json` files to reflect the new section layout.

3. **Update landing page**: Rebrand from "clawft" to "WeftOS" for the site identity. Keep the clawft section as a sub-section.

4. **Add redirects**: If any external links point to old standalone doc paths, add redirects in `next.config.mjs`.

5. **Mark superseded standalone files**: Add a header to each migrated standalone file:
   ```
   <!-- MIGRATED: This content has been moved to the documentation site.
        Canonical source: docs/src/content/docs/weftos/guides/configuration.mdx
        This file is kept for reference but is no longer maintained. -->
   ```

6. **Do NOT delete standalone files yet**: They serve as reference during migration. Delete in a follow-up PR after migration is verified.

### File Flow Diagram

```
CURRENT STATE:
  docs/src/content/docs/weftos/*.mdx  (25 pages, high quality, partial coverage)
  docs/weftos/*.md                     (12 pages, high quality, broader coverage)
  docs/weftos/specs/*.md               (4 pages, new specs)
  docs/weftos/*-symposium/             (47 files, internal)
  docs/{getting-started,deployment,development,reference,guides,architecture}/*.md
                                       (23 pages, clawft-focused)

FUTURE STATE:
  docs/src/content/docs/               (56 pages, single source of truth)
    getting-started/                    (4 pages -- NEW section)
    weftos/                             (17 pages -- existing, reorganized as "concepts")
    guides/                             (9 pages -- NEW section, migrated from standalone)
    reference/                          (7 pages -- NEW section, from specs/)
    vision/                             (3 pages -- NEW section, from VISION.md)
    contributing/                       (3 pages -- NEW section)
    clawft/                             (13 pages -- existing, unchanged)

  docs/weftos/*-symposium/             (47 files -- internal, unchanged, NOT in site)
  docs/weftos/weaver-analysis-*.md     (3 files -- internal, unchanged)
  docs/weftos/sparc/                   (1 file -- internal, unchanged)
```

---

## 4. Deployment Plan

### Platform: Vercel

**Rationale**: Fumadocs is a Next.js application. Vercel is the first-party hosting platform for Next.js with zero-config deployment, automatic preview URLs for PRs, edge caching, and custom domain support. Alternatives (Cloudflare Pages, GitHub Pages) require additional configuration for Next.js SSR/ISR features. Given the site is static-exportable, Cloudflare Pages is a viable backup, but Vercel provides the smoothest path.

### DNS Configuration

```
weftos.weavelogic.ai  CNAME  cname.vercel-dns.com.
```

Add the custom domain in the Vercel project settings. Vercel handles TLS automatically.

### CI/CD Pipeline

**GitHub Actions workflow** (`.github/workflows/docs-deploy.yml`):

```yaml
name: Deploy Docs
on:
  push:
    branches: [master]
    paths:
      - 'docs/src/**'
  pull_request:
    paths:
      - 'docs/src/**'

jobs:
  deploy:
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: docs/src
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with:
          node-version: 22
          cache: npm
          cache-dependency-path: docs/src/package-lock.json
      - run: npm ci
      - run: npm run build
      # Vercel CLI deploys automatically via Vercel GitHub integration
      # OR use vercel-action for explicit control:
      # - uses: amondnet/vercel-action@v25
      #   with:
      #     vercel-token: ${{ secrets.VERCEL_TOKEN }}
      #     vercel-org-id: ${{ secrets.VERCEL_ORG_ID }}
      #     vercel-project-id: ${{ secrets.VERCEL_PROJECT_ID }}
      #     working-directory: docs/src
```

Alternatively, connect the GitHub repo directly to Vercel with:
- **Root directory**: `docs/src`
- **Build command**: `npm run build`
- **Output directory**: `.next`
- **Node.js version**: 22

This is the simplest approach -- no GitHub Actions needed. Vercel watches the repo and auto-deploys on push to master.

### Vercel Project Configuration

Add a `vercel.json` in `docs/src/`:

```json
{
  "buildCommand": "npm run build",
  "outputDirectory": ".next",
  "framework": "nextjs"
}
```

### Estimated Effort

| Task | Effort | Dependencies |
|------|--------|-------------|
| 1. Vercel project setup + DNS | 30 min | Vercel account, DNS access for weavelogic.ai |
| 2. Verify Fumadocs builds cleanly | 15 min | None |
| 3. Rebrand landing page (clawft -> WeftOS) | 1 hour | None |
| 4. Create new section directories + meta.json | 1 hour | None |
| 5. Migrate 12 standalone docs to MDX | 4-6 hours | Task 4 |
| 6. Write 3 gap pages (first-project, block-descriptor, roadmap) | 2-3 hours | Task 5 |
| 7. Update navigation (meta.json for all sections) | 1 hour | Task 5 |
| 8. Test full site build and navigation | 1 hour | Task 7 |
| 9. Deploy to Vercel + verify custom domain | 30 min | Task 1 + 8 |
| 10. Mark migrated standalone files as superseded | 30 min | Task 9 |
| **Total** | **11-14 hours** | Spread across 2-3 work sessions |

### Critical Path

```
DNS setup (30 min, can happen in parallel)
  |
  v
Verify build (15 min)
  |
  v
Create sections + migrate docs (5-7 hours)
  |
  v
Write gap pages (2-3 hours)
  |
  v
Test + deploy (1.5 hours)
```

The DNS propagation can happen while content migration is in progress. First deployment can go live with the existing 38 pages while migration of the remaining 18 pages continues.

### Phased Rollout

**Phase 1 (Day 1)**: Deploy existing Fumadocs site as-is to weftos.weavelogic.ai. Rebrand landing page. This gives us a live URL immediately with 38 pages.

**Phase 2 (Day 2-3)**: Migrate standalone docs into new sections (getting-started, guides, reference, vision, contributing). Deploy incrementally.

**Phase 3 (Day 4)**: Write gap pages, final navigation polish, mark superseded files.

---

## 5. Open Questions

1. **Should `clawft` docs remain on the WeftOS site?** The current Fumadocs site serves both clawft (framework) and weftos (kernel) docs. For `weftos.weavelogic.ai`, the clawft section could either stay (as context for how the kernel integrates with the framework) or move to a separate `clawft.weavelogic.ai` site. Recommendation: keep it for now -- separation adds complexity with no user benefit.

2. **Rustdoc hosting**: The API reference page should link to hosted rustdoc output. Options: (a) host on the same Vercel site under `/api/rustdoc/`, (b) use a separate docs.rs-style deployment, (c) link to docs.rs after crates.io publication. Recommendation: defer until crates.io publication; for now, document the `cargo doc` command.

3. **Symposium archive**: Should any symposium content be publicly accessible? The synthesis documents and decision resolutions have value as "how we got here" context. Recommendation: include only the Sprint 11 synthesis and the decisions page (already in Fumadocs as `decisions.mdx`). Full symposium sessions stay internal.

4. **Search scope**: Fumadocs search currently indexes all pages. Should internal/advanced pages (kernel-modules, integration-patterns) be weighted lower? Recommendation: no custom weighting for now -- the default full-text search is adequate.

---

## Appendix A: Complete File Inventory

### Fumadocs Pages (38 total)

**clawft/ (13)**:
`index.mdx`, `getting-started.mdx`, `architecture.mdx`, `cli-reference.mdx`, `configuration.mdx`, `plugins.mdx`, `providers.mdx`, `channels.mdx`, `tools.mdx`, `skills.mdx`, `browser.mdx`, `deployment.mdx`, `security.mdx`

**weftos/ (25)**:
`index.mdx`, `architecture.mdx`, `kernel-phases.mdx`, `boot-sequence.mdx`, `process-table.mdx`, `ipc.mdx`, `capabilities.mdx`, `exochain.mdx`, `governance.mdx`, `wasm-sandbox.mdx`, `containers.mdx`, `app-framework.mdx`, `mesh-networking.mdx`, `discovery.mdx`, `clustering.mdx`, `security.mdx`, `ecc.mdx`, `democritus.mdx`, `self-healing.mdx`, `observability.mdx`, `persistence.mdx`, `config-auth.mdx`, `gui.mdx`, `decisions.mdx`, `kernel-guide.mdx`

### Standalone Docs to Migrate (18 files)

1. `docs/weftos/quickstart.md` -> `getting-started/quickstart.mdx`
2. `docs/weftos/INSTALL.md` -> `getting-started/installation.mdx`
3. `docs/weftos/VISION.md` -> `vision/index.mdx`
4. `docs/weftos/configuration-reference.md` -> `guides/configuration.mdx`
5. `docs/weftos/feature-flags.md` -> `guides/feature-flags.mdx`
6. `docs/weftos/kernel-modules.md` -> `guides/kernel-modules.mdx`
7. `docs/weftos/integration-patterns.md` -> `guides/integration-patterns.mdx`
8. `docs/weftos/kernel-governance.md` -> merge into `concepts/governance.mdx`
9. `docs/weftos/specs/block-catalog.md` -> `reference/block-catalog.mdx`
10. `docs/weftos/specs/console-commands.md` -> `reference/console-commands.mdx`
11. `docs/weftos/specs/journey-spec.md` -> `reference/journey-spec.mdx`
12. `docs/weftos/specs/mentra-hud-constraints.md` -> `reference/mentra-hud.mdx`
13. `docs/deployment/docker.md` -> `guides/deployment-docker.mdx`
14. `docs/deployment/wasm.md` -> `guides/deployment-wasm.mdx`
15. `docs/deployment/release.md` -> `guides/deployment-release.mdx`
16. `docs/development/contributing.md` -> `contributing/index.mdx`
17. `docs/development/testing-three-workstreams.md` -> `contributing/testing.mdx`
18. `docs/architecture/overview.md` -> `contributing/architecture.mdx`

### Gap Pages to Write (3 files)

1. `getting-started/first-project.mdx` -- Extract "Initialize a Project" + "Project Structure After Init" from quickstart into its own focused page
2. `reference/block-descriptor.mdx` -- JSON schema for block descriptors (extract from block-catalog)
3. `vision/roadmap.mdx` -- Public roadmap (extract from VISION.md "What Comes Next" section)

### Internal Docs (not migrated, 52+ files)

- `docs/weftos/09-symposium/` (4 files)
- `docs/weftos/k2-symposium/` (10 files)
- `docs/weftos/k3-symposium/` (7 files)
- `docs/weftos/ecc-symposium/` (8 files)
- `docs/weftos/k5-symposium/` (6 files)
- `docs/weftos/sprint11-symposium/` (12 files)
- `docs/weftos/weaver-analysis-clawft.md`
- `docs/weftos/weaver-analysis-v2.md`
- `docs/weftos/external-analysis-results.md`
- `docs/weftos/sparc/k6-cluster-networking.md`
- `docs/weftos/FEATURE_GATES.md` (superseded)
- `docs/weftos/CONFIGURATION.md` (superseded)
- `docs/weftos/k6-development-notes.md`
- `docs/weftos/weftos-gui-specifications.md` (internal planning)
- `docs/skills/clawft/` (5 files)
- `docs/external/` (1 file)
- `docs/benchmarks/` (1 file)
