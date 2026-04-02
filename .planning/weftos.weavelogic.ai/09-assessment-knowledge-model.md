# Assessment Knowledge Model — Systems × People × Gaps

## The Insight

Three graphs, overlaid:

1. **System Graph** — applications, services, databases, APIs, infrastructure (built by WeftOS scanning code/config/infra)
2. **Org Graph** — people, teams, roles, responsibilities (built from org chart + Paperclip patterns)
3. **Knowledge Graph** — who knows what about which system (built by inverting the learner model onto each person × each system node)

The intersection of these three graphs IS the assessment product. Every gap in Graph 3 is a finding. Every single-person dependency is a risk. Every undocumented connection in Graph 1 is tribal knowledge waiting to walk out the door.

## Three Graphs

### Graph 1: System Graph (what exists)

Built automatically via SOP 2 (treesitter, git mining, dependency analysis, infra scanning):

```
Nodes:
  - Applications (weavelogic.ai, weftos docs site, assessment portal)
  - Services (Next.js frontend, Supabase auth, Vercel deploy, GitHub Actions CI)
  - Databases (Supabase PostgreSQL, localStorage, RVF segment files)
  - APIs (LLM providers, GitHub API, Vercel API, PostHog)
  - Infrastructure (Vercel, GHCR, crates.io, npm, GoDaddy DNS)
  - Modules (per-crate in Rust, per-component in React)

Edges:
  - depends_on (service A calls service B)
  - deploys_to (app → infrastructure)
  - stores_in (service → database)
  - authenticates_via (app → auth provider)
  - monitors_with (app → observability)

Properties:
  - test_coverage: 85%
  - last_modified: 2026-04-01
  - documentation_level: "partial" | "full" | "none"
  - complexity_score: lines, cyclomatic, dependency depth
```

### Graph 2: Org Graph (who's responsible)

Built from org chart data + Paperclip Company/OrgChart types (already implemented in Sprint 13):

```
Nodes:
  - People (name, role, team, tenure, skills)
  - Teams (engineering, DevOps, product, etc.)
  - Roles (backend lead, frontend dev, SRE, etc.)

Edges:
  - reports_to (person → person)
  - member_of (person → team)
  - owns (person/team → system node)
  - contributes_to (person → system node, via git blame)
  - on_call_for (person → service)

Properties:
  - tenure: "2 years"
  - bus_factor: 1 (only person who knows this)
  - last_commit: "2026-03-15"
  - domain_expertise: ["billing", "auth", "infra"]
```

### Graph 3: Knowledge Graph (who knows what)

Built by inverting the learner model onto each person × system node:

```
Nodes: (reuses Graph 1 system nodes + Graph 2 people nodes)

Edges:
  - knows_about (person → system node)
    Properties:
      depth: "awareness" | "understanding" | "working" | "expert"
      confidence: 0.0-1.0
      source: "git_history" | "questionnaire" | "assessment" | "self_reported"
      last_verified: "2026-04-01"
      evidence: ["committed to module X 47 times", "answered 8/10 correctly"]

Derived metrics:
  - bus_factor(system_node) = count(people where depth >= "working")
  - knowledge_coverage(person) = count(known_nodes) / count(all_nodes)
  - tribal_knowledge_risk(system_node) = 1 / bus_factor
  - knowledge_gap(person, system_node) = expected_depth - actual_depth
```

## Knowledge Sources (how we fill Graph 3)

### Automated (passive — no human input needed)

| Source | What It Reveals | Confidence |
|--------|----------------|------------|
| `git blame` / `git log` | Who has touched which files/modules | High for "contributed to", low for "understands" |
| PR review history | Who reviews which areas | Medium — reviewing ≠ understanding |
| On-call/incident history | Who debugs which services | High — debugging requires deep knowledge |
| Slack/Teams search | Who answers questions about which topics | Medium |
| Jira/Linear assignments | Who is assigned to which areas | Low — assignment ≠ knowledge |
| CI/CD pipeline ownership | Who maintains deployment configs | Medium |

### Questionnaires (active — targeted gap filling)

This is where the assessment product differentiates. WeftOS generates questionnaires to fill specific gaps:

```
System: "The billing module has bus_factor=1 (only Jane has committed).
        We need to assess whether anyone else understands it."

Generated questionnaire for team:
  Q1: "Can you describe how the billing reconciliation job works?"
  Q2: "What happens when a subscription payment fails?"  
  Q3: "Where are billing events stored and how long are they retained?"
  Q4: "What external services does billing depend on?"
  Q5: "What would you check first if billing totals were off by 2%?"

Scoring:
  - Each answer evaluated against known system facts from Graph 1
  - Depth assessed: awareness (knows it exists) → expert (can debug edge cases)
  - Confidence scored by comparing answer accuracy to code/config reality
```

### Self-Assessment (person reports their own knowledge)

```
"Rate your knowledge of each system (1-5):"

  [ ] Authentication service    ████░  4/5
  [ ] Billing module            █░░░░  1/5  ← gap detected
  [ ] API gateway               ███░░  3/5
  [ ] Deployment pipeline       ██░░░  2/5
  [ ] Database schema           ███░░  3/5

"Your self-assessment vs. evidence:"
  - Auth: You rated 4/5. Git history confirms 127 commits. ✓ Consistent
  - Billing: You rated 1/5. Confirmed — 0 commits, 0 reviews. ✓ Consistent  
  - Deploy: You rated 2/5. But you've resolved 5 deploy incidents. ↑ May be higher than you think
```

## Questionnaire Generation

WeftOS generates questionnaires dynamically based on gaps:

### Gap Types and Question Strategies

| Gap Type | Strategy | Example |
|----------|----------|---------|
| **Bus factor = 1** | Ask everyone else about that system | "Who besides Jane can explain the billing reconciliation?" |
| **No documentation** | Ask the contributor to explain | "You wrote the auth middleware — can you describe the session token lifecycle?" |
| **Recent changes, no tests** | Ask about the change's impact | "This module changed 3 times last month with 0 test changes. What should be tested?" |
| **Cross-system dependency** | Ask about the connection | "Service A calls Service B's internal endpoint. What happens if B is down for 30 min?" |
| **Stale knowledge** | Re-assess after time/changes | "You last touched the payment module 8 months ago. It's changed significantly. Can you walk through the current flow?" |

### Question Difficulty Calibration

Questions are calibrated to the person's current assessed depth:

```
Person at "awareness" level → Ask identification questions:
  "What database does the user service use?"

Person at "understanding" level → Ask explanation questions:
  "Why does the user service use PostgreSQL instead of DynamoDB?"

Person at "working" level → Ask application questions:
  "If we needed to add multi-tenancy to the user service, what would you change?"

Person at "expert" level → Ask edge-case questions:
  "The user service handles ~500 concurrent connections. What breaks first if that doubles?"
```

### Questionnaire Delivery

Multiple modes depending on context:

1. **In-app tour guide** (playground model) — conversational, adaptive, real-time
2. **Structured form** (assessment portal) — fixed questions, async, scored offline
3. **Slack/Teams bot** — lightweight check-ins, one question at a time
4. **PR comments** — "You're modifying the billing module. Quick check: can you describe the reconciliation flow?"
5. **Onboarding flow** — new hire works through system-by-system with the tour guide

## Assessment Output

### Per-Person Knowledge Profile

```
┌─────────────────────────────────────────────────┐
│  Jane Chen — Senior Backend Engineer            │
│  Team: Platform  │  Tenure: 3 years             │
│                                                  │
│  Knowledge Coverage: 67% (12/18 system nodes)   │
│                                                  │
│  Expert:       billing, payments, auth          │
│  Working:      API gateway, user service        │
│  Understanding: deploy pipeline, monitoring     │
│  Awareness:    frontend, mobile app             │
│  Unknown:      ML pipeline, data warehouse,     │
│                search indexing, admin portal,    │
│                partner API, compliance module    │
│                                                  │
│  Unique Knowledge (bus_factor=1):               │
│    ⚠ billing reconciliation                     │
│    ⚠ payment provider failover logic            │
└─────────────────────────────────────────────────┘
```

### Per-System Risk Assessment

```
┌─────────────────────────────────────────────────┐
│  Billing Module — RISK: HIGH                    │
│                                                  │
│  Bus Factor: 1 (Jane only)                      │
│  Documentation: Partial (API docs, no internals)│
│  Test Coverage: 72%                             │
│  Last Modified: 2026-03-28                      │
│  Complexity: High (2,400 LoC, 14 external deps) │
│                                                  │
│  Knowledge Distribution:                        │
│    Jane     ████████░░  Expert (0.95)           │
│    Marcus   ██░░░░░░░░  Awareness (0.2)         │
│    Others   ░░░░░░░░░░  Unknown                 │
│                                                  │
│  Recommendation:                                 │
│    1. Cross-train Marcus (questionnaire ready)  │
│    2. Document reconciliation flow (Jane)       │
│    3. Add integration tests for failover paths  │
│    4. Estimated knowledge transfer: 2-3 weeks   │
│                                                  │
│  Evidence: 47 commits by Jane, 0 by others.     │
│  ExoChain ref: [e2f6a4...] (cryptographic proof)│
└─────────────────────────────────────────────────┘
```

### Organization-Wide Heatmap

```
                    Auth  Billing  API  Deploy  Frontend  ML  Search  DB
Jane                ████   ████   ███   ██      █        ░    ░      ██
Marcus              ███    █      ████  ███     ░        ░    ░      ███
Sarah               █      ░      ██   ████    ████     ░    ░      ██
Alex                ░      ░      █    ██      ████     ███  ███    █
Priya               ██     ░      ░    █       ░        ████ ████   ████

Bus Factor:          3      1      3    4       2        1    1      3
Risk:               Low    HIGH   Low  Low     Med      HIGH HIGH   Low
```

## Connection to Existing Codebase

### Already Built (v0.3.1)
- `clawft-kernel/causal.rs` — CausalGraph with typed edges → System Graph
- `clawft-kernel/hnsw_service.rs` — HNSW vector search → semantic questionnaire matching
- `clawft-kernel/crossref.rs` — CrossRef linking → person ↔ system ↔ knowledge edges
- `clawft-kernel/chain.rs` — ExoChain → cryptographic evidence for every finding
- `clawft-kernel/democritus.rs` — DEMOCRITUS loop → continuous re-assessment
- Paperclip `Company`/`OrgChart` types (Sprint 13) → Org Graph nodes
- `clawft-plugin-git` — git blame/log → automated knowledge signals
- `clawft-plugin-treesitter` — AST analysis → system module extraction
- `rvf_io.rs` — RVF segment files → persistence for all three graphs

### Needs Building
- Questionnaire generator (LLM-powered, calibrated to depth level)
- Knowledge scoring engine (compare answers to system facts)
- Assessment report renderer (the output examples above)
- Self-assessment UI (rating + comparison to evidence)
- Team heatmap visualization
- `weft assess` CLI command (unified entry point)

## Integration with Playground Tour Guide

The playground IS the questionnaire engine for WeftOS itself:

1. Visitor arrives → learner model starts tracking
2. Tour guide teaches WeftOS concepts → fills Knowledge Graph (person=visitor, system=WeftOS)
3. Tour guide detects gaps → asks targeted questions to fill them
4. By the end of the session, the visitor has a knowledge profile of WeftOS
5. "You've covered 5 of 8 major areas. Here's what you know and what's left."

This demonstrates the assessment product through direct experience. The visitor IS the assessment subject. When they see their own knowledge profile at the end, they immediately understand what this would look like for their team and their codebase.

## The WeaveLogic Product Arc

```
Free tier:     Playground tour guide builds YOUR knowledge profile of WeftOS
               (you experience the product by being assessed)

Starter ($500): Point WeftOS at YOUR codebase → System Graph
               Self-assessment questionnaire → initial Knowledge Graph
               Output: "Here's what we can see. Here are the gaps."

Standard ($2,500): Full assessment with team questionnaires
               Git/PR/incident history analysis → automated Knowledge Graph
               Targeted questionnaires for gap filling
               Output: Per-person profiles, per-system risk, heatmap

Professional ($7,500): Continuous assessment + cross-training plan
               DEMOCRITUS loop for ongoing re-assessment
               Questionnaire campaigns for knowledge transfer tracking
               Output: Quarterly knowledge health reports, $50K+ ROI guarantee
```

Each tier is the same engine with more data sources and more assessment depth. The pricing maps directly to the three graphs: Starter = Graph 1 only, Standard = Graphs 1+2+3, Professional = continuous monitoring of all three.
