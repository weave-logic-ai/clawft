# WeaveLogic Web Presence Strategy

**Date**: 2026-03-27
**Status**: Comprehensive Blueprint
**Scope**: Three-property architecture for weavelogic.ai, weftos.weavelogic.ai, and the AI Assessor

---

## Table of Contents

1. [Audit: weavelogic.ai Current State](#1-audit-weavelogicai-current-state)
2. [Three-Property Architecture](#2-three-property-architecture)
3. [Content Ownership Model](#3-content-ownership-model)
4. [Buyer Journey (Funnel Map)](#4-buyer-journey-funnel-map)
5. [weftos.weavelogic.ai Improvements](#5-weftosweavelogicai-improvements)
6. [weavelogic.ai Rewrite Plan](#6-weavelogicai-rewrite-plan)
7. [Cross-Property Technical Architecture](#7-cross-property-technical-architecture)

---

## 1. Audit: weavelogic.ai Current State

### 1.1 Technical Stack

The current weavelogic.ai is a **Next.js 14 monorepo** (`weavelogic-monorepo`) with:
- **Frontend**: `services/frontend/` -- Next.js with Prisma, Bun, TypeScript
- **API**: `services/api/` -- Express-style backend
- **Infrastructure**: Docker Compose with PostgreSQL, Redis, NGINX
- **Hosting**: Google Cloud (GCP), with Docker-based deployment
- **Analytics**: Custom `weave_analytics` service, Vertex AI integration

### 1.2 Existing Pages

The current site has 12 routes:

| Route | Purpose | Status |
|-------|---------|--------|
| `/` (home) | Dynamic homepage with WeaveLogicHomepage component | ACTIVE -- generic SaaS messaging |
| `/about` | About page | EXISTS |
| `/services` | Service listings | EXISTS |
| `/solutions` | Solutions page | EXISTS |
| `/assessment` | Assessment page | EXISTS -- early placeholder |
| `/blog` | Blog | EXISTS |
| `/contact` | Contact page | EXISTS |
| `/demo` | Demo mode with test personas | EXISTS |
| `/roi-calculator` | Industry-specific ROI calculator | EXISTS |
| `/resources` | Resources page | EXISTS |
| `/signin` | Authentication | EXISTS |
| `/admin` | Admin dashboard | EXISTS -- internal |
| `/industry/[slug]` | Dynamic industry verticals (37) | EXISTS |

### 1.3 Current Messaging

**Title**: "WeaveLogic AI - Transform Your Business with Intelligent Automation"
**Meta description**: "Cut operational costs by 40% and boost efficiency with AI-powered automation. Join 2,847 companies already saving millions."

**Problems with current messaging**:

1. **Generic SaaS language**. "Transform your business with intelligent automation" could describe any of 10,000 AI companies. There is no differentiation.
2. **Fabricated social proof**. "2,847 companies already saving millions" and "Join 2,847 companies" are fictitious numbers. There are zero paying customers. This is a credibility risk that could destroy trust the moment anyone investigates.
3. **Wrong product narrative**. The site positions WeaveLogic as a "B2B SaaS platform delivering intelligent automation across 37 industry verticals." This was the original 2024 vision. The actual business is now a consulting firm selling AI assessments backed by WeftOS technology.
4. **No WeftOS mention**. The proprietary technology that differentiates WeaveLogic from every other consultancy is invisible.
5. **No assessment funnel**. The `/assessment` route exists but is not integrated into a buyer journey.
6. **37-vertical dilution**. Trying to serve 37 industries simultaneously signals "we serve nobody well." The current target is 5 specific verticals (law, accounting, healthcare, SaaS, manufacturing).

### 1.4 What Needs to Change

| Area | Current | Required |
|------|---------|----------|
| **Identity** | Generic AI SaaS platform | AI assessment consultancy powered by WeftOS |
| **Value prop** | "Cut costs by 40%" | "Understand your systems. Document tribal knowledge. Plan automation." |
| **Social proof** | Fake "2,847 companies" | Self-analysis case study ("analyzed its own birth") |
| **Funnel** | None | Intake questionnaire -> scoring -> report -> consultation booking |
| **Pricing** | None visible | Assessment packages: $500 / $2,500 / $5,000 / $7,500 |
| **ROI guarantee** | None | "$50K+ in identified savings or the assessment is free" |
| **Technology story** | Invisible | WeftOS as the engine behind the magic |
| **Verticals** | 37 (diluted) | 5 focused (law, accounting, healthcare, SaaS, manufacturing) |

---

## 2. Three-Property Architecture

### 2.1 Property Map

```
weavelogic.ai (MARKETING + PRODUCT -- the buyer journey)
  |
  +-- / (home)                  Value prop, hero, social proof, CTA
  +-- /assess                   AI Assessor intake (primary conversion)
  +-- /assess/report            Sample assessment report
  +-- /services                 Assessment packages + pricing
  +-- /services/fractional-cto  Fractional CTO offering
  +-- /how-it-works             Non-technical explanation of the process
  +-- /case-studies             "Analyzed its own birth" + industry examples
  +-- /case-studies/[slug]      Individual case study pages
  +-- /about                    Team, credentials, origin story
  +-- /blog                     Thought leadership + SEO content
  +-- /blog/[slug]              Individual blog posts
  +-- /contact                  Booking link, Calendly embed
  +-- /roi-calculator           Keep existing -- refine for 5 verticals
  +-- /technology               WeftOS overview for non-technical buyers
  +-- /admin                    Internal admin dashboard (auth-gated)
  +-- /signin                   Auth (admin only, not client-facing)

weftos.weavelogic.ai (PLATFORM + DOCS -- developer/community)
  |
  +-- / (landing)               Platform-oriented landing page
  +-- /docs/getting-started/    Quickstart, install, first project
  +-- /docs/concepts/           Architecture, kernel phases, ECC, mesh...
  +-- /docs/guides/             Configuration, feature flags, deployment...
  +-- /docs/reference/          Console commands, block catalog, specs...
  +-- /docs/vision/             Vision, decisions, roadmap
  +-- /docs/contributing/       Dev setup, testing, architecture
  +-- /docs/clawft/             Framework documentation (13 pages)
  +-- Subtle CTA in footer/sidebar: "Want us to analyze YOUR systems?"

assess.weavelogic.ai (PRODUCT APP -- assessment tool)
  |
  +-- /                         Assessment intake landing
  +-- /intake/[id]              Client self-assessment questionnaire
  +-- /report/[id]              Generated assessment report (client view)
  +-- /admin                    Assessment admin dashboard
  +-- /admin/assessments        Pipeline view
  +-- /admin/assessments/[id]   Individual assessment detail
  +-- /admin/meetings           Meeting scheduler + facilitation
  +-- /admin/knowledge          Knowledge graph browser
```

### 2.2 Rationale for Three Properties

**weavelogic.ai** is the front door. It speaks the language of buyers (CTOs, CEOs). It answers "why should I care?" and "what do I get?" No jargon, no kernel phases, no Merkle chains.

**weftos.weavelogic.ai** is the engine room. It speaks the language of developers and technical evaluators. It answers "how does this work?" and "can I trust this?" The docs site builds credibility with the CTO who does due diligence after the CEO says "look into this."

**assess.weavelogic.ai** is the product. The assessment tool is a distinct application with its own auth, data model, and user flows. It should be deployable and scalable independently. The marketing site links to it; the tool itself has no marketing chrome.

### 2.3 Phased Deployment

| Phase | Timeline | Deliverable |
|-------|----------|-------------|
| **Phase 1** | Week 1-2 | weavelogic.ai rewrite with new messaging, /assess intake form, /services pricing |
| **Phase 2** | Week 2-3 | weftos.weavelogic.ai deployed (existing Fumadocs + landing page rewrite) |
| **Phase 3** | Week 3-6 | assess.weavelogic.ai MVP (intake -> scoring -> report) |
| **Phase 4** | Week 6-8 | Cross-link all properties, analytics, shared header component |

---

## 3. Content Ownership Model

### 3.1 Content by Property

| Content Type | Lives On | Rationale |
|---|---|---|
| **Buyer-facing messaging** (value prop, pricing, ROI, case studies) | weavelogic.ai | This is marketing. It must be optimized for conversion, not comprehensiveness. |
| **Technical documentation** (API reference, kernel architecture, configuration, feature flags) | weftos.weavelogic.ai | Developers and technical evaluators need depth. Fumadocs provides search, navigation, and good UX for long-form technical content. |
| **"How it works" for non-technical buyers** | weavelogic.ai/technology | A 1-page, diagram-heavy explanation. No code blocks. Links to weftos.weavelogic.ai for the deep dive. |
| **Blog / thought leadership** | weavelogic.ai/blog | SEO value accrues to the marketing domain. Blog posts can reference and link to specific docs pages. |
| **Assessment product UI** | assess.weavelogic.ai | Separate app with its own auth, state, and user flows. |
| **Assessment methodology** (process template, question frameworks) | weavelogic.ai/how-it-works (summary) + internal only (full detail) | Buyers see a simplified version. The full methodology is internal IP. |
| **Vision / roadmap** | weftos.weavelogic.ai/docs/vision/ | This is for the open-source community and technical evaluators. Buyers do not read roadmaps. |
| **Symposium results / internal analysis** | Git repo only (not deployed anywhere) | Working documents. Not public. |

### 3.2 Cross-Linking Strategy

```
weavelogic.ai                         weftos.weavelogic.ai
+-----------------------+              +-----------------------+
| /technology           |  -------->   | /docs/concepts/ecc    |
| "Learn how WeftOS     |  "Deep dive" | (full ECC docs)       |
|  powers the analysis" |              |                       |
+-----------------------+              +-----------------------+

+-----------------------+              +-----------------------+
| /case-studies/        |  -------->   | /docs/vision/         |
| self-analysis         |  "See the    | (Weaver analysis      |
|                       |  data"       |  methodology)         |
+-----------------------+              +-----------------------+

weftos.weavelogic.ai                   weavelogic.ai
+-----------------------+              +-----------------------+
| Every page footer:    |  -------->   | /assess               |
| "Want us to analyze   |  subtle CTA  | (intake form)         |
|  YOUR codebase?"      |              |                       |
+-----------------------+              +-----------------------+

+-----------------------+              +-----------------------+
| /docs/concepts/       |  -------->   | /services             |
| index.mdx sidebar:    |  text link   | (assessment packages) |
| "Professional         |              |                       |
|  assessments"         |              |                       |
+-----------------------+              +-----------------------+
```

### 3.3 Dual Content Model (Static Framework Docs vs AI-Managed Docs)

Two categories of documentation exist and they have different update cycles:

**Static framework docs** (human-authored, PR-based updates):
- Getting started guides
- Configuration reference
- CLI reference
- Deployment guides
- Contributing guide
- Architecture overview

**AI-managed evolving docs** (Weaver-generated, programmatic updates):
- Weaver analysis outputs (gap reports, health scores)
- Decision tracking (symposium results, decision statuses)
- Codebase analysis reports (conversation maps, anomaly detection)
- Module dependency graphs (auto-generated from cargo metadata)

**How they coexist**:

1. All static docs live in `docs/src/content/docs/` as MDX files (Fumadocs source of truth)
2. AI-managed docs live in `docs/weftos/` as standalone markdown (internal working documents)
3. Selected AI-managed content is manually promoted to Fumadocs pages when stable (e.g., the decisions page already exists as `decisions.mdx`)
4. The Fumadocs site never auto-deploys AI-generated content. Promotion is always a human decision via PR.

**Migration flow**:

```
Code changes
    |
    v
Weaver analyzes (automated)
    |
    v
Analysis output written to docs/weftos/*.md (internal)
    |
    v
Human reviews and decides to promote (manual)
    |
    v
Content adapted to MDX in docs/src/content/docs/ (PR)
    |
    v
Fumadocs site rebuilds (CI/CD via Vercel)
    |
    v
weftos.weavelogic.ai updated (automatic on merge)
```

---

## 4. Buyer Journey (Funnel Map)

### 4.1 Full Funnel

```
AWARENESS         INTEREST          EVALUATION        DECISION         RETENTION
(stranger)        (curious)         (comparing)       (buying)         (client)
    |                 |                 |                 |                |
    v                 v                 v                 v                v
Blog post         /how-it-works     /case-studies     /assess          assess.weavelogic.ai
LinkedIn post     /technology       /services         Calendly call     deliverable review
Conference talk   ROI calculator    weftos docs       proposal/SOW      upsell to fCTO
Referral          Sample report     GitHub repo       contract          ongoing engagement
Google search                       Demo video
```

### 4.2 Stage-by-Stage Detail

#### Stage 1: AWARENESS -- "I didn't know I had this problem"

**Where they come from**:
- Google search: "AI automation assessment", "document tribal knowledge", "reduce technical debt"
- LinkedIn: Founder's posts about "the software that analyzed its own birth"
- Referrals from existing network
- Conference talks / webinars
- Content syndication (Medium, Dev.to for technical content)

**Where they land**:
- weavelogic.ai/blog/[topic] (SEO-optimized articles)
- weavelogic.ai/ (direct traffic from referrals)

**What they see**:
- Problem-first messaging: "Your best developer just quit. How much did they take with them?"
- The "analyzed its own birth" hook in the hero section
- Industry-specific proof points for their vertical

**CTA**: "See how it works" -> /how-it-works

#### Stage 2: INTEREST -- "This might apply to me"

**Where they go**:
- weavelogic.ai/how-it-works
- weavelogic.ai/technology
- weavelogic.ai/roi-calculator

**What they see**:
- 3-step process explanation (no jargon):
  1. "We map your systems" (Weaver ingests codebase, docs, infrastructure)
  2. "We find what's hidden" (gaps, tribal knowledge, single points of failure)
  3. "We build your roadmap" (prioritized, costed, dependency-mapped)
- ROI calculator: input company size + vertical -> estimated savings
- Non-technical explanation of WeftOS: "Our AI doesn't just search for similar words -- it traces cause and effect through your entire system"

**CTA**: "Get your free initial assessment" -> /assess (basic tier) or "See what we found in our own code" -> /case-studies/self-analysis

#### Stage 3: EVALUATION -- "Is this real? Can I trust them?"

**Where they go**:
- weavelogic.ai/case-studies (proof it works)
- weavelogic.ai/services (what exactly do I get?)
- weftos.weavelogic.ai (CTO does technical due diligence)
- GitHub repo (if they want to see the code)

**What they see on weavelogic.ai**:

**/case-studies/self-analysis** (the anchor case study):
- "We pointed WeftOS at its own codebase. Here's what it found."
- 12 conversations identified, 65 decisions tracked
- Critical anomaly: `agent_loop` -- 1,831 lines, 0 tests, 0 incoming edges
- Confidence trajectory: 0.62 -> 0.78 over the analysis period
- "This is what we do for your systems, except we also bring the human expertise to act on it."

**/services**:
- Package comparison table (see Section 6.3)
- ROI guarantee: "$50K+ in identified savings or the assessment is free"
- Clear deliverable list per package

**What they see on weftos.weavelogic.ai** (CTO due diligence):
- Real architecture docs, not marketing fluff
- 25 kernel concept pages with code examples
- 3,953 tests mentioned, actual Rust code visible
- The docs themselves are proof of engineering rigor

**CTA**: "Start your assessment" -> /assess or "Book a consultation" -> Calendly

#### Stage 4: DECISION -- "I'm ready to buy"

**Where they go**:
- weavelogic.ai/assess (self-service intake)
- Calendly link (consultation booking)

**What they see**:

**/assess** intake flow:
1. Company basics (name, size, industry, role)
2. 8-12 questions per domain (current AI usage, pain points, automation landscape)
3. Conditional branching based on answers
4. Immediate output: initial maturity score + gap indicators
5. CTA: "Want the full analysis? Book your discovery call."

**Conversion mechanism**:
- Self-service intake is free and produces enough value (initial score) to create commitment
- Discovery call is booked via Calendly after seeing initial results
- Call converts to paid engagement (assessment package selection)
- Contract/SOW signed, assessment begins

#### Stage 5: RETENTION -- "I'm a client, keep me engaged"

**Where they go**:
- assess.weavelogic.ai (assessment dashboard, deliverable access)
- weavelogic.ai/blog (ongoing thought leadership)
- Monthly check-in calls

**What they see**:
- Living assessment dashboard (not a static PDF)
- Progress tracking as assessment phases complete
- Deliverable access (2-year roadmap, executive deck, detailed study)
- Upsell prompt: "Ready to implement? Our fractional CTO service..."

**Upsell path**:
- Assessment ($500-$7,500) -> Fractional CTO ($10-15K/month)
- Assessment reveals gaps -> WeaveLogic implements the roadmap
- The assessment tool itself becomes a retention mechanism (living document, not a one-time report)

---

## 5. weftos.weavelogic.ai Improvements

### 5.1 Current State Assessment

Based on expert reviews (Marketing: D+, DevRel: B+, UX: C+) and the Fumadocs unification plan audit:

**Strengths**:
- 38 high-quality Fumadocs pages (25 WeftOS + 13 clawft)
- Architecture documentation is genuinely excellent (243-line architecture.mdx with diagrams, type signatures, boot sequences)
- Search, navigation, dark mode, responsive layout all functional
- Real technical depth -- this is not vaporware documentation

**Weaknesses**:
- Landing page says "WeftOS" with a subtitle about "AI operating system for understanding, documenting, and automating client systems" -- this is accurate but does not tell a developer WHY they should care
- No problem/solution framing on the landing page
- No CTA anywhere on the site
- Feels like "just docs" -- no platform identity
- Navigation is flat (two sections: WeftOS Kernel, clawft Framework) with no progressive disclosure
- No getting started path -- a new visitor sees a wall of kernel concepts

### 5.2 Landing Page Rewrite

The current landing page (`docs/src/app/page.tsx`) is a minimal two-card layout. Replace it with a structured landing page that tells a story.

**Wireframe: New Landing Page**

```
+------------------------------------------------------------------+
|  [Logo] WeftOS                     [Docs] [GitHub] [WeaveLogic]  |
+------------------------------------------------------------------+
|                                                                    |
|  YOUR CODEBASE HAS A STORY.                                       |
|  WEFTOS READS IT.                                                  |
|                                                                    |
|  A cognitive operating system that maps the causal structure       |
|  of software projects -- conversations, decisions, dependencies,   |
|  and gaps -- with cryptographic provenance.                        |
|                                                                    |
|  [Get Started]              [View on GitHub]                       |
|                                                                    |
+------------------------------------------------------------------+
|                                                                    |
|  THE PROBLEM                                                       |
|  +-----------+  +-----------+  +-----------+                       |
|  | Tribal    |  | Invisible |  | AI has no |                       |
|  | knowledge |  | tech debt |  | memory    |                       |
|  | leaves    |  | explodes  |  | across    |                       |
|  | when      |  | without   |  | sessions  |                       |
|  | people do |  | warning   |  |           |                       |
|  +-----------+  +-----------+  +-----------+                       |
|                                                                    |
+------------------------------------------------------------------+
|                                                                    |
|  WHAT WEFTOS PROVIDES                                              |
|  +------+  +----------+  +--------+  +---------+  +--------+      |
|  | A    |  | A        |  | A      |  | A       |  | A      |      |
|  | Brain|  | Conscience|  | Memory |  | Nervous |  | Growth |      |
|  | ECC  |  | Govrnance|  | ExoChn |  | System  |  | Instct |      |
|  | graph|  | 3-branch |  | Merkle |  | P2P     |  | Weaver |      |
|  +------+  +----------+  +--------+  +---------+  +--------+      |
|                                                                    |
+------------------------------------------------------------------+
|                                                                    |
|  PROVEN ON ITSELF                                                  |
|  "WeftOS analyzed the codebase that built it and found             |
|   12 conversations, 65 decisions, and a critical anomaly           |
|   in 1,831 lines of untested code."                                |
|                                                                    |
|  [Read the self-analysis ->]                                       |
|                                                                    |
+------------------------------------------------------------------+
|                                                                    |
|  BY THE NUMBERS                                                    |
|  173,923 lines | 22 crates | 3,953 tests | 7 platforms            |
|                                                                    |
+------------------------------------------------------------------+
|                                                                    |
|  GET STARTED                                                       |
|  cargo install clawft-cli && weft agent                            |
|                                                                    |
|  [Quickstart Guide]   [Architecture]   [Kernel Phases]             |
|                                                                    |
+------------------------------------------------------------------+
|                                                                    |
|  Want us to analyze YOUR systems?                                  |
|  WeaveLogic offers professional AI assessments powered by WeftOS.  |
|  [Learn more at weavelogic.ai ->]                                  |
|                                                                    |
+------------------------------------------------------------------+
```

**Key changes**:
1. Problem/solution framing instead of feature list
2. The self-analysis story as the hero proof point
3. Quick-start code snippet visible immediately
4. Subtle commercial CTA in the footer (not aggressive)
5. Navigation links to weavelogic.ai and GitHub

### 5.3 Making It Feel Like a "Platform" Not Just Docs

1. **Hero landing page** (described above) gives the site an identity beyond "API reference"
2. **"By the numbers" bar** reinforces that this is a substantial project, not a weekend experiment
3. **Getting Started path** as the primary entry point, not a flat list of kernel concepts
4. **Restructured navigation** with progressive disclosure:
   - Getting Started (the on-ramp)
   - Concepts (understand the architecture)
   - Guides (do specific things)
   - Reference (look things up)
   - Vision (where it's going)
   - Contributing (join the project)
   - clawft Framework (the agent CLI layer)
5. **GitHub integration**: Star count badge, "Edit this page" links, contribution guide prominently linked
6. **Roadmap page** (`/docs/vision/roadmap`) showing what's proven, in progress, and aspirational -- builds credibility through honesty

### 5.4 Subtle CTA Placement

The docs site is not a sales page. CTAs must feel like helpful suggestions, not pitches.

**Footer CTA** (every page):
```
---
Built by WeaveLogic. Want us to analyze your codebase?
Professional assessments at weavelogic.ai/assess
```

**Sidebar CTA** (concepts/index.mdx only):
```
> **Professional Assessments**
> WeaveLogic offers assessment services powered by WeftOS.
> We map your systems, find hidden gaps, and build your roadmap.
> [Learn more ->](https://weavelogic.ai/services)
```

**Vision/roadmap page** (natural context):
```
## Commercial Applications
WeftOS powers WeaveLogic's AI assessment practice. The same Weaver that
analyzed the clawft codebase is used to analyze client systems.
[See how it works ->](https://weavelogic.ai/how-it-works)
```

No pop-ups. No banners. No "book a demo" buttons on technical pages. The CTO who finds the docs site while evaluating should feel like they discovered something real, not like they walked into a sales pitch.

### 5.5 Handling the Dual Content Model

Per the Fumadocs unification plan, the recommendation is **Fumadocs as single source of truth** for public documentation. The implementation:

1. **Migrate 18 standalone markdown files** into Fumadocs MDX (detailed in the unification plan)
2. **Keep 52+ internal files** (symposiums, analysis outputs, SPARC plans) in the git repo only
3. **New content goes directly to Fumadocs** -- no more parallel standalone markdown for public content
4. **AI-generated content** (Weaver outputs) stays in `docs/weftos/` as internal working docs, promoted to Fumadocs manually when appropriate

---

## 6. weavelogic.ai Rewrite Plan

### 6.1 Technical Approach

The existing monorepo at `/claw/root/weavelogic/projects/weavelogic.ai/` is a substantial Next.js application with Prisma, Redis, and multiple services. Rather than rebuild from scratch:

1. **Keep the monorepo structure** -- it already has auth, API, database, and admin
2. **Gut the frontend messaging** -- replace the WeaveLogicHomepage component and all page copy
3. **Remove the 37-vertical system** -- replace with 5 focused verticals
4. **Integrate the Assessor intake** -- either embed the intake form or redirect to assess.weavelogic.ai
5. **Remove fabricated social proof** -- replace with real data from the self-analysis

### 6.2 Page-by-Page Content Plan

#### Home Page (`/`)

**Hero Section**:
```
YOUR BEST DEVELOPER JUST QUIT.
HOW MUCH DID THEY TAKE WITH THEM?

WeaveLogic finds the tribal knowledge locked in your systems,
documents it before it disappears, and builds the roadmap
to automate what should never have been manual.

[Get Your Assessment]        [See How It Works]
```

**Problem Section** (3 cards):
- **Tribal Knowledge Walks Out the Door**: "When a senior developer leaves, they take years of context with them. Why was that service built that way? What breaks if you change it? The answers are in their head, not in your docs."
- **Technical Debt Is Invisible Until It Explodes**: "Your most critical code paths often have the least test coverage. The modules that change together are never documented as coupled. You only discover the problem when production goes down."
- **AI Agents Have No Memory**: "Every AI conversation starts from zero. Your chatbot doesn't remember what it learned yesterday. Your automation tools can't explain why they made a decision."

**Solution Section** (the 3-step process):
1. **We Map Your Systems**: "Our Weaver AI ingests your codebase, documentation, and infrastructure. It builds a causal graph -- not just what's similar, but what caused what."
2. **We Find What's Hidden**: "Conversations happening in your code. Decisions with no documentation. Single points of failure with zero test coverage. Tribal knowledge that exists only in one person's head."
3. **We Build Your Roadmap**: "A prioritized, costed, dependency-mapped plan. Quick wins you can implement this month. Strategic investments for the next two years."

**Proof Section**:
```
THE SOFTWARE THAT ANALYZED ITS OWN BIRTH

We pointed WeftOS at the codebase that built it. Here's what it found:
- 12 active conversations happening in the code
- 65 architectural decisions, 43 implemented, 12 pending
- 1 critical anomaly: the kernel's main execution loop had 1,831 lines
  and zero tests
- Confidence rose from 0.62 to 0.78 as the analysis deepened

This is what we do for your systems.

[Read the full case study ->]
```

**ROI Section**:
```
THE GUARANTEE

We identify $50,000+ in savings and efficiency gains,
or the assessment is free.

Mid-market companies (100-2000 employees) typically find:
- 15-30% of engineering time spent on work that should be automated
- 3-5 critical single points of failure with no documentation
- $200K-$500K/year in recoverable operational waste

[Calculate Your ROI ->]
```

**CTA Section**:
```
START WITH A FREE ASSESSMENT

Answer 15 questions about your systems. Get an instant AI maturity
score and gap analysis. No commitment, no credit card.

[Start Free Assessment]

Or book a 30-minute consultation: [Schedule Call]
```

**Footer**: Standard links + "Powered by WeftOS" with link to weftos.weavelogic.ai

#### Services Page (`/services`)

See Section 6.3 for the full package structure.

#### How It Works (`/how-it-works`)

**Section 1: The Process** (4 phases, visual timeline):
1. Intake (self-service questionnaire, 15 minutes)
2. Discovery (3-6 guided sessions with your team, 1-2 weeks)
3. Analysis (Weaver + human expertise, 1 week)
4. Deliverables (roadmap, executive deck, detailed study)

**Section 2: The Technology** (non-technical):
- "Our AI doesn't just search for keywords. It traces cause and effect."
- Diagram showing: Code -> Weaver -> Causal Graph -> Insights
- "Every finding is linked to evidence. Every recommendation traces back to data."
- Link to weftos.weavelogic.ai for the technical deep-dive

**Section 3: What You Get** (deliverable samples):
- Screenshot/mockup of assessment dashboard
- Sample gap analysis table
- Sample roadmap timeline
- Sample executive summary page

#### Case Studies (`/case-studies`)

**Anchor Case Study: "The Software That Analyzed Its Own Birth"**
- Full narrative version of the Weaver self-analysis
- Data visualizations: conversation map, decision tracking, gap analysis
- Before/after: what the analysis revealed, what was fixed
- Bridge: "This is what we do for your organization"

**Industry-Specific Case Studies** (to be created as clients come):
- Template ready for law firms, accounting, healthcare, SaaS, manufacturing
- Until real case studies exist, use industry-relevant examples from the self-analysis methodology (e.g., "What a law firm's document management system might reveal")

#### Technology (`/technology`)

**Non-technical overview of WeftOS for buyers**:
- "Three layers that work together" (Semantic + Causal + Provenance) explained with business analogies
- "Not just what's similar, but what caused what, and we can prove it"
- Differentiator table: WeftOS vs. traditional consulting vs. generic AI tools
- Link to weftos.weavelogic.ai for technical details

#### About (`/about`)

- Origin story: "From automation agency to cognitive operating system"
- Team credentials (founder bio, relevant experience)
- Philosophy: "We believe every system has a story. We read it."
- Open source commitment: WeftOS is open source, assessments fund the roadmap

#### Blog (`/blog`)

Launch with 5 articles targeting key SEO terms (see Section 6.5).

#### Contact (`/contact`)

- Calendly embed for booking consultations
- Email contact form
- No phone number (consultancy, not call center)

### 6.3 Assessment Packages and Pricing

| | Starter | Standard | Professional | Enterprise |
|---|---|---|---|---|
| **Price** | $500 | $2,500 | $5,000 | $7,500 |
| **Target** | Small business (<50) | Mid-market (50-200) | Mid-market (200-500) | Enterprise (500+) |
| **Intake** | Self-service questionnaire | Self-service + 1 discovery call | Self-service + 3 discovery sessions | Self-service + 6 discovery sessions |
| **Analysis** | AI-only (automated) | AI + 4 hours consultant | AI + 12 hours consultant | AI + 24 hours consultant |
| **Data collection** | Questionnaire only | + code repo scan | + infrastructure discovery | + full stack audit |
| **Deliverables** | Maturity score + gap list | + executive summary + basic roadmap | + 2-year roadmap + detailed study | + executive deck + implementation plan |
| **ROI guarantee** | 2x fee identified | 6x fee identified | 6x fee identified | 8x fee identified |
| **Timeline** | 48 hours | 2 weeks | 3 weeks | 4 weeks |

### 6.4 How WeftOS Is Positioned (for buyers)

WeftOS is NOT the product being sold. The assessment is the product. WeftOS is the technology that makes the assessment uniquely valuable. The positioning:

**On weavelogic.ai**: "Powered by WeftOS" -- like "Intel Inside." The buyer does not need to understand the kernel. They need to understand the outcome.

- Home page: "Our proprietary AI engine, WeftOS, traces cause and effect through your entire system -- not just keyword similarity."
- Technology page: 1-page non-technical overview with a link to the docs site
- Case studies: "The WeftOS Weaver analyzed 29,029 data points and identified..." -- the technology proves the capability

**On weftos.weavelogic.ai**: Full technical depth. The CTO who wants to validate the claims can read 56 pages of architecture documentation, see the test counts, and evaluate the Rust code on GitHub.

The two sites serve different audiences looking at the same technology from different angles.

### 6.5 SEO Strategy

**Primary Keywords** (commercial intent, target with service/case study pages):
- "AI automation assessment"
- "technical debt assessment"
- "AI maturity assessment"
- "automation readiness assessment"
- "tribal knowledge documentation"
- "fractional CTO AI"

**Secondary Keywords** (informational intent, target with blog posts):
- "document tribal knowledge before developers leave"
- "how to assess AI readiness"
- "hidden technical debt"
- "AI governance for enterprise"
- "codebase analysis tools"
- "causal analysis software"

**Long-Tail Keywords** (vertical-specific, target with blog + landing pages):
- "AI assessment for law firms"
- "automation assessment healthcare"
- "SaaS technical debt audit"
- "manufacturing process automation assessment"
- "accounting firm AI readiness"

**SEO Architecture**:
- weavelogic.ai targets commercial and informational keywords
- weftos.weavelogic.ai targets developer/technical keywords ("cognitive operating system", "causal graph Rust", "WASM sandbox kernel")
- Blog posts on weavelogic.ai link to relevant docs on weftos.weavelogic.ai (domain authority sharing via cross-linking)
- Each vertical gets a dedicated blog post + FAQ page for long-tail capture

**Launch Blog Posts** (5 articles):
1. "The $2M Problem: What Happens When Your Best Developer Quits"
2. "Why Your AI Chatbot Forgets Everything (And How to Fix It)"
3. "We Pointed Our AI at Its Own Codebase. Here's What It Found."
4. "The 5 Signs Your Organization Needs an AI Assessment"
5. "Technical Debt Is Not a Metaphor: How to Actually Measure It"

### 6.6 Social Proof Strategy (Pre-Customer)

With zero paying customers, fabricating numbers is a non-starter. Instead:

**What to show**:

1. **The self-analysis case study**. This is the single most powerful proof point. Real data, real findings, real anomaly detected. "We analyzed ourselves and found a critical gap" is more credible than "2,847 companies trust us."

2. **Technical metrics**: 173,923 lines of Rust, 3,953 tests, 22 crates, 7 platform targets. These are verifiable on GitHub.

3. **External project analysis**: The Weaver was run on ruvector (109 crates, 2,484 commits, 470 modules). Results: 16 gaps identified, 5 critical untested modules found, all verified as genuine. This proves the technology works on codebases other than its own.

4. **The Weave-NN lineage**: 14 phases of development over ~1 year. The learning loop concept was validated with real usage data showing +10% success rate from memory priming.

5. **Open source credibility**: The code is visible. Anyone can verify the claims. "Don't trust our marketing -- read our tests."

6. **Founder credentials**: LinkedIn profile, prior consulting experience, conference talks (if any).

**What NOT to show**:
- Fake customer counts
- Fake testimonials
- Fake logos
- "As seen in" badges for publications that never covered you
- Vanity metrics ("10,000 GitHub stars" when you have 3)

**Social proof evolution plan**:
- Phase 1 (now): Self-analysis + technical metrics + open source
- Phase 2 (first 3 clients): Real testimonials, anonymized case study data
- Phase 3 (10+ clients): Named case studies, ROI data, industry benchmarks

---

## 7. Cross-Property Technical Architecture

### 7.1 DNS Configuration

```
weavelogic.ai           A/CNAME    -> Vercel (or current GCP hosting)
www.weavelogic.ai       CNAME      -> weavelogic.ai
weftos.weavelogic.ai    CNAME      -> cname.vercel-dns.com
assess.weavelogic.ai    CNAME      -> [assessor app hosting]
```

### 7.2 Hosting Strategy

| Property | Stack | Hosting | Rationale |
|---|---|---|---|
| weavelogic.ai | Next.js 14, Prisma, PostgreSQL, Redis | **Current GCP setup** (Docker Compose) or migrate to **Vercel + managed DB** | Already deployed on GCP. Migration to Vercel reduces ops burden but requires extracting the API to a separate service. Short-term: keep GCP. Mid-term: evaluate Vercel migration. |
| weftos.weavelogic.ai | Next.js 16, Fumadocs, static-exportable | **Vercel** | Zero-config deployment for Next.js. Free tier is sufficient for a docs site. Connect the clawft repo, set root directory to `docs/src`, auto-deploy on push to master. |
| assess.weavelogic.ai | Next.js + Express (from agentic_ai_assessor) | **Vercel** (frontend) + **GCP Cloud Run** (API/backend) | The assessor has backend requirements (AI processing, cloud discovery, document ingestion) that need server-side compute. Frontend on Vercel, API on Cloud Run. |

### 7.3 Shared Components

#### Shared Header/Footer

All three properties should feel like they belong to the same family. Shared elements:

**Header**:
- WeaveLogic logo (consistent across all properties)
- Navigation: [WeaveLogic](weavelogic.ai) | [WeftOS Docs](weftos.weavelogic.ai) | [Assessments](weavelogic.ai/assess) | [GitHub](github.com/...)
- Property indicator: subtle text showing which property you're on ("WeaveLogic", "WeftOS Docs", "AI Assessor")

**Footer**:
- Consistent across all properties
- Links to all three properties
- Copyright, privacy policy, terms of service

**Implementation**: Publish a shared `@weavelogic/ui` package (or simply copy the header/footer components) to all three frontends. Given that the properties use different Next.js versions (14 vs 16), a shared npm package is cleaner than a monorepo import.

Alternatively, use a shared Tailwind theme + design tokens and implement the header/footer natively in each project. This is simpler and avoids cross-version compatibility issues.

#### Shared Analytics

All properties should report to the same analytics pipeline:

- **PostHog** (self-hosted or cloud): Event tracking, funnels, session recording
- Events to track:
  - `page_view` (all properties)
  - `cta_click` (weavelogic.ai, weftos.weavelogic.ai footer CTA)
  - `assessment_started` (weavelogic.ai/assess, assess.weavelogic.ai)
  - `assessment_completed` (assess.weavelogic.ai)
  - `consultation_booked` (Calendly integration event)
  - `docs_search` (weftos.weavelogic.ai)
  - `cross_property_navigation` (user moves between properties)

#### Shared Auth

Only relevant for admin/internal users. Client-facing flows:
- weavelogic.ai: No auth required (public marketing site)
- weftos.weavelogic.ai: No auth required (public docs site)
- assess.weavelogic.ai: Auth required for client dashboard and admin

Auth approach:
- Use the existing auth system from weavelogic.ai (Prisma + session-based)
- Set auth cookies on `.weavelogic.ai` domain so they're shared across subdomains
- Admin users get access to both weavelogic.ai/admin and assess.weavelogic.ai/admin
- Client users only get access to assess.weavelogic.ai/report/[their-id]

### 7.4 Cookie and Session Sharing

```
Cookie domain: .weavelogic.ai
  - Shared auth session cookie (HttpOnly, Secure, SameSite=Lax)
  - Analytics user ID cookie (for cross-property tracking)

Property-specific cookies:
  - weavelogic.ai: theme preference, ROI calculator inputs
  - weftos.weavelogic.ai: docs search history, sidebar collapse state
  - assess.weavelogic.ai: assessment progress, form state
```

Setting cookies on `.weavelogic.ai` (with the leading dot) makes them available to all subdomains. This enables:
1. Single sign-on for admin users across properties
2. Cross-property analytics (same user ID on marketing site and docs site)
3. Funnel tracking (user reads docs, then starts assessment -- tracked as one journey)

### 7.5 SEO Configuration

#### Canonical URLs

Each property owns its own canonical URLs. No cross-property canonicalization.

```
weavelogic.ai/services       -> canonical: https://weavelogic.ai/services
weftos.weavelogic.ai/docs/   -> canonical: https://weftos.weavelogic.ai/docs/
assess.weavelogic.ai/intake  -> canonical: https://assess.weavelogic.ai/intake
```

#### Sitemaps

Each property generates its own sitemap:

```
weavelogic.ai/sitemap.xml           -- marketing pages, blog posts
weftos.weavelogic.ai/sitemap.xml    -- all documentation pages
assess.weavelogic.ai/sitemap.xml    -- public-facing pages only (not gated content)
```

weavelogic.ai should also include a sitemap index that references the other sitemaps:

```xml
<!-- weavelogic.ai/sitemap-index.xml -->
<sitemapindex>
  <sitemap><loc>https://weavelogic.ai/sitemap.xml</loc></sitemap>
  <sitemap><loc>https://weftos.weavelogic.ai/sitemap.xml</loc></sitemap>
</sitemapindex>
```

#### robots.txt

```
# weavelogic.ai/robots.txt
User-agent: *
Allow: /
Disallow: /admin
Disallow: /signin
Disallow: /api/
Sitemap: https://weavelogic.ai/sitemap.xml

# weftos.weavelogic.ai/robots.txt
User-agent: *
Allow: /
Sitemap: https://weftos.weavelogic.ai/sitemap.xml

# assess.weavelogic.ai/robots.txt
User-agent: *
Allow: /
Disallow: /admin
Disallow: /report/
Disallow: /intake/
Sitemap: https://assess.weavelogic.ai/sitemap.xml
```

Assessment intake and report pages are gated from search engines -- they contain client-specific data and should not be indexed.

#### Structured Data

weavelogic.ai should include:
- `Organization` schema (name, logo, URL, social profiles)
- `Service` schema (assessment packages with pricing)
- `Article` schema (blog posts)
- `FAQPage` schema (on /how-it-works and /services)
- `BreadcrumbList` schema (all pages)

weftos.weavelogic.ai should include:
- `SoftwareApplication` schema (WeftOS)
- `TechArticle` schema (documentation pages)
- `BreadcrumbList` schema (all pages)

### 7.6 Performance Requirements

| Metric | Target | Measurement |
|---|---|---|
| First Contentful Paint | < 1.5s | Lighthouse |
| Largest Contentful Paint | < 2.5s | Lighthouse |
| Time to Interactive | < 3.5s | Lighthouse |
| Cumulative Layout Shift | < 0.1 | Lighthouse |
| Lighthouse Performance Score | > 90 | All properties |
| Documentation search latency | < 200ms | Fumadocs client-side search |
| Assessment intake form submission | < 1s | Server response time |

---

## Appendix A: Implementation Priority

| Priority | Task | Effort | Dependency |
|---|---|---|---|
| **P0** | Remove fake social proof from weavelogic.ai | 1 hour | None -- do this immediately |
| **P0** | Rewrite weavelogic.ai home page copy | 4 hours | None |
| **P1** | Create /services page with pricing | 4 hours | Home page copy |
| **P1** | Create /how-it-works page | 4 hours | None |
| **P1** | Create self-analysis case study page | 8 hours | None |
| **P1** | Deploy weftos.weavelogic.ai (existing Fumadocs as-is) | 2 hours | DNS access, Vercel account |
| **P2** | Rewrite weftos.weavelogic.ai landing page | 4 hours | Deployment |
| **P2** | Create /technology page on weavelogic.ai | 4 hours | None |
| **P2** | Write 5 launch blog posts | 20 hours | Home page copy (for consistent messaging) |
| **P2** | Integrate /assess intake form | 16 hours | Assessor project progress |
| **P3** | Migrate 18 standalone docs to Fumadocs | 8 hours | weftos site deployed |
| **P3** | Set up cross-property analytics | 4 hours | All properties deployed |
| **P3** | Implement shared header/footer | 8 hours | All properties deployed |
| **P3** | Deploy assess.weavelogic.ai | 16+ hours | Assessor MVP complete |
| **P4** | SEO optimization (structured data, sitemaps, robots.txt) | 4 hours | All properties deployed |
| **P4** | Create industry-specific landing pages (5 verticals) | 20 hours | Blog posts, case study |

## Appendix B: Key Files Referenced

| File | Location | Relevance |
|---|---|---|
| Current homepage component | `/claw/root/weavelogic/projects/weavelogic.ai/services/frontend/app/WeaveLogicHomepage.tsx` | Needs full rewrite |
| Fumadocs landing page | `/claw/root/weavelogic/projects/clawft/docs/src/app/page.tsx` | Needs rewrite for weftos.weavelogic.ai |
| Fumadocs site source | `/claw/root/weavelogic/projects/clawft/docs/src/` | Ready to deploy |
| Assessment BRD | `/claw/root/weavelogic/projects/agentic_ai_assessor/docs/BUSINESS_REQUIREMENTS.md` | Defines assessor product |
| Assessment process template | `/claw/root/weavelogic/docs/business_plan/05-operations-processes/assessment-process-template.md` | Defines assessment methodology |
| WeftOS Vision | `/claw/root/weavelogic/projects/clawft/docs/weftos/VISION.md` | Source for narrative content |
| Weaver self-analysis | `/claw/root/weavelogic/projects/clawft/docs/weftos/weaver-analysis-v2.md` | Source for case study |
| External analysis (ruvector) | `/claw/root/weavelogic/projects/clawft/docs/weftos/external-analysis-results.md` | Proof of external portability |
| Fumadocs unification plan | `/claw/root/weavelogic/projects/clawft/docs/weftos/fumadocs-unification-plan.md` | Docs migration blueprint |
| Market opportunities | `/claw/root/weavelogic/projects/clawft/docs/weftos/ecc-symposium/market_opportunities.md` | Competitive landscape |
| weavelogic.ai monorepo | `/claw/root/weavelogic/projects/weavelogic.ai/` | Current marketing site codebase |
| Assessor project | `/claw/root/weavelogic/projects/agentic_ai_assessor/` | Assessment tool codebase |
