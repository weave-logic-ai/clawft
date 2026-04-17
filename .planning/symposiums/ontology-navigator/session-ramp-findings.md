# Addendum: Google RAMP as Assessment Ontology Pattern

**Source**: cloud.google.com/solutions/cloud-migration-program

## What RAMP Provides

A structured assessment methodology, not a tool. Two parallel
scoring systems that together answer "what's here, how mature is it,
and what should we do about it?"

### 1. Organizational Maturity Radar (6 dimensions)

| Dimension | What It Measures |
|-----------|-----------------|
| Learn | Cloud skills, training adoption |
| Lead | Executive sponsorship, governance, CoE |
| Scale | Automation, IaC, CI/CD maturity |
| Secure | Identity, compliance, policy-as-code |
| Manage | Operations, monitoring, incident response |
| Optimize | Cost management, FinOps, right-sizing |

Scored on 4 levels: Tactical → Strategic → Transformational → Autonomous.
Produces a spider/radar chart.

### 2. Per-Workload Technical Readiness (6R classification)

Every discovered workload gets a disposition:

| Disposition | When | Action |
|------------|------|--------|
| Rehost | High-dependency, low-complexity | Lift and shift |
| Replatform | Minor changes needed | Managed service swap |
| Refactor | Well-architected, ROI justifies | Rebuild for cloud-native |
| Repurchase | SaaS replacement available | Buy instead of build |
| Retire | Low value, unused | Decommission |
| Retain | Can't move yet | Keep, plan later |
| **Ratify** | Already correct, meets requirements | **No action — confirm and move on** |

Google's 6R lacks an explicit "this is fine" outcome. Retain implies
future action ("can't move *yet*"). Ratify means the assessment
reviewed it, confirmed it meets requirements, and explicitly signed
off. This matters because:

- It distinguishes "we looked and it's fine" from "we haven't looked"
- It closes the assessment loop — every entity gets a disposition
- It prevents re-assessment churn on things already validated
- In the topology browser, Ratified nodes render as confirmed/green
  while unclassified nodes remain neutral — you can see assessment
  coverage at a glance

Classification based on: technical complexity, business criticality,
dependency coupling, modernization ROI, team readiness.

### 3. Discovery: Declared vs Observed

RAMP distinguishes:
- **Declared dependencies**: what configs/docs say exists
- **Observed dependencies**: what traffic analysis shows actually communicates

The delta between declared and observed IS the gap analysis.
This maps directly to our Plan/Implemented/Actual diff overlay.

### 4. Wave Planning via Dependency Sort

Workloads grouped into migration waves by topological sort of their
dependency graph. Least-coupled services move first. This is a
derived view on the topology browser — sort by dependency depth,
color by wave assignment.

## How This Maps to WeftOS

### Assessment Ontology (assessment.yaml preset)

```yaml
name: assessment
label: "System Assessment (RAMP-informed)"
version: "1.0.0"

nodes:
  workload:
    iri: "weftos:assessment#Workload"
    geometry: layered
    contains: [service, database, integration]
    display_name: "Workload"

  service:
    iri: "weftos:assessment#Service"
    geometry: force
    contains: [endpoint, process]
    display_name: "Service"

  database:
    iri: "weftos:assessment#Database"
    geometry: grid
    display_name: "Database"

  integration:
    iri: "weftos:assessment#Integration"
    geometry: force
    display_name: "External Integration"

  team:
    iri: "weftos:assessment#Team"
    geometry: tree
    contains: [person]
    display_name: "Team"

  person:
    iri: "weftos:assessment#Person"
    geometry: force
    display_name: "Person"

edges:
  - type: depends_on
    from: service
    to: service
    cardinality: "N:M"
    style: { animated: true }

  - type: owns
    from: team
    to: workload
    cardinality: "1:N"

  - type: connects_to
    from: service
    to: database
    cardinality: "N:M"

  - type: integrates_with
    from: service
    to: integration
    cardinality: "N:M"

  - type: observed_traffic
    from: service
    to: service
    cardinality: "N:M"
    style: { dash: dotted, stroke: "#f59e0b" }

modes:
  heatmap:
    metrics:
      - field: maturity_score
        label: "Maturity Level"
        palette: rdylgn
        range: [0, 4]
      - field: complexity_score
        label: "Migration Complexity"
        palette: inferno
        range: [1, 5]
      - field: utilization
        label: "Resource Utilization"
        palette: viridis
        range: [0, 100]

  diff:
    sources:
      before: "declared-topology.json"
      after: "observed-topology.json"
```

### New Entity Property: Disposition (6R)

Add to entity metadata, not the schema:

```rust
// In entity metadata (serde_json::Value)
{
    "disposition": "refactor",      // 7R classification (6R + Ratify)
    "maturity": 2,                  // 0-4 scale
    "wave": 3,                      // migration wave assignment
    "complexity": 3,                // 1-5 technical readiness
    "criticality": "high",         // business criticality
    "declared_deps": ["auth", "db"],
    "observed_deps": ["auth", "db", "legacy-cache"]  // observed has extra
}
```

### Maturity Radar as Navigator Mode

The 6-dimension maturity radar is a new Wardley-like geometry:

```rust
enum Geometry {
    Force, Tree, Layered, Timeline,
    Stream, Grid, Geo, Radial,
    Wardley,
    Radar,  // NEW: axes = maturity dimensions, position = score
}
```

Or simpler: render the radar chart in the detail panel when a
workload/team node is selected, showing its scores across all 6
RAMP dimensions. The topology view shows the dependency graph;
the detail panel shows the maturity assessment.

### Connecting to Existing Analyzers (ADR-023)

Map the 8 existing analyzers to RAMP dimensions:

| WeftOS Analyzer | RAMP Dimension |
|----------------|----------------|
| ComplexityAnalyzer | Scale (automation maturity) |
| DependencyAnalyzer | Scale + Manage |
| SecurityAnalyzer | Secure |
| TopologyAnalyzer | Manage (infrastructure awareness) |
| DataSourceAnalyzer | Manage (data governance) |
| NetworkAnalyzer | Secure + Manage |
| RabbitMQAnalyzer | Scale (message infrastructure) |
| TerraformAnalyzer | Scale (IaC maturity) |

Each analyzer already produces findings. Adding a maturity score
per dimension is straightforward — count findings by severity,
apply a scoring function, output 0-4 maturity level.

### Declared vs Observed: The Key Assessment Pattern

The most actionable RAMP pattern: graph the declared topology
(from configs, docs, schemas) alongside the observed topology
(from traffic analysis, runtime telemetry, actual connections).

This IS the Diff overlay with:
- **Plan** = declared topology (config files, architecture docs)
- **Implemented** = observed topology (network flows, actual calls)

Gaps = services that configs say exist but traffic never reaches.
Rogues = connections that exist in traffic but aren't in configs.
This is the highest-value use of the topology browser for an
enterprise assessment engagement.

## Impact on Build Plan

Add to Phase 1 (Sprint 17-18):
- assessment.yaml preset with RAMP-informed maturity dimensions
- Disposition property (6R) in entity metadata
- Maturity scoring function per analyzer → 0-4 scale

Add to Phase 4 (Sprint 20-21):
- Wave planning derived view (topological sort of dependency graph)
- Declared vs observed diff (configs vs traffic analysis)
- Radar chart in detail panel for maturity assessment

## References

- Google Cloud Migration Program: cloud.google.com/solutions/cloud-migration-program
- RAMP methodology (internal Google engagement model)
- StratoZone → Migration Center (discovery tooling)
- WeftOS ADR-023: Assessment as Kernel Service
- WeftOS assessment analyzers: clawft-kernel/src/assessment/
