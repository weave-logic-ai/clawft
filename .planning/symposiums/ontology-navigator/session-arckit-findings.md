# Addendum: ArcKit as Governance Ontology Pattern

**Source**: github.com/tractorjuice/arc-kit

## What It Is

Enterprise Architecture Governance Toolkit — 68 AI-assisted slash
commands that produce typed Markdown artifacts across 9 lifecycle
phases: Discovery, Planning, Architecture, Governance, Compliance,
Operations, Procurement, Research, Reporting.

Not a runtime system. A document-generation framework with a
well-defined **document-type ontology** (60 artifact types) and
**traceability matrix** (requirement → design → implementation → test).

## Why It Matters

### 1. ArcKit's doc-type ontology IS a topology schema

ArcKit defines ~60 typed artifacts with relationships:

```
BR (Business Req) → FR (Functional Req) → HLD (High-Level Design)
    → DLD (Detailed Design) → ADR (Architecture Decision)
        → Test Case → Compliance Finding
```

This is exactly the graph our navigator should render. Each doc type
maps to a node type. Each cross-reference maps to an edge. The
9 categories map to ontology layers. The traceability matrix IS the
Diff overlay (Plan vs Implemented vs Tested).

### 2. Wardley Map evolution axis = a layout dimension

ArcKit positions components on two axes:
- Y: Value chain (user-visible → invisible infrastructure)
- X: Evolution (Genesis 0.00 → Custom 0.25 → Product 0.50 → Commodity 1.00)

This is a native geometry for the topology browser. A `wardley`
geometry type in the schema would lay out nodes by evolution stage
(X) and value chain position (Y). This is a better fit than force
or tree for strategic analysis.

### 3. The assessment pipeline connection

WeftOS already has:
- `weft assess` with 8 pluggable analyzers (Complexity, Dependency,
  Security, Topology, DataSource, Network, RabbitMQ, Terraform)
- Assessment findings with severity (Info → Critical)
- Progressive discovery (config scan → infrastructure detection)
- Three-Graph model: System Graph × Org Graph × Knowledge Graph

ArcKit adds:
- Structured output templates for each finding type
- Governance phase tracking (discovery → planning → architecture)
- Compliance framework mapping (HM Treasury, ISO, SOC2)
- Vendor scoring and procurement workflows
- Executive reporting (board packs, stakeholder comms)

**The combination**: WeftOS discovers and analyzes (runtime).
ArcKit-style templates structure the output (governance artifacts).
The topology browser renders the traceability graph and shows gaps.

## Integration Path

### Phase 1: Governance doc-type schema
Add a `governance.yaml` topology preset:

```yaml
name: governance
label: "AI-SDLC Governance"
version: "1.0.0"

nodes:
  business_requirement:
    iri: "weftos:governance#BusinessRequirement"
    geometry: layered
    contains: [functional_requirement]
    display_name: "Business Requirement"

  functional_requirement:
    iri: "weftos:governance#FunctionalRequirement"
    geometry: layered
    contains: [design_decision]
    display_name: "Functional Requirement"

  design_decision:
    iri: "weftos:governance#DesignDecision"
    geometry: layered
    display_name: "Design Decision (ADR)"

  compliance_finding:
    iri: "weftos:governance#ComplianceFinding"
    geometry: force
    display_name: "Compliance Finding"

  assessment_report:
    iri: "weftos:governance#AssessmentReport"
    geometry: timeline
    time_field: timestamp
    contains: [finding]
    display_name: "Assessment Report"

  finding:
    iri: "weftos:governance#Finding"
    geometry: grid
    display_name: "Finding"

edges:
  - type: traces_to
    from: business_requirement
    to: functional_requirement
    cardinality: "1:N"
  - type: implemented_by
    from: functional_requirement
    to: design_decision
    cardinality: "N:M"
  - type: tested_by
    from: design_decision
    to: test_case
    cardinality: "1:N"
  - type: violates
    from: compliance_finding
    to: "*"
    cardinality: "N:M"

modes:
  diff:
    sources:
      before: "requirements.json"    # Plan: what should exist
      after: "assessment-report.json" # Actual: what was found
```

### Phase 2: Wardley geometry type
Add `wardley` to the Geometry enum:

```rust
enum Geometry {
    Force, Tree, Layered, Timeline,
    Stream, Grid, Geo, Radial,
    Wardley,  // NEW: X=evolution, Y=value-chain
}
```

Nodes need `evolution` and `value_chain` fields in metadata.
Layout is a simple 2D scatter with labeled axes.

### Phase 3: Assessment → governance artifact pipeline
`weft assess run` produces findings. A new `weft govern` command
templates those findings into typed governance artifacts (BR, FR,
ADR, compliance finding) following ArcKit's document-type ontology.
The traceability graph builds automatically.

## Connection to Three-Graph Assessment Model

The existing assessment model defines three graphs:
1. **System Graph** — apps, services, databases, APIs
2. **Org Graph** — people, teams, roles
3. **Knowledge Graph** — person × system with depth scores

ArcKit's governance phases map to assessment lifecycle:
- Discovery → System Graph construction (SOP 2)
- Planning → Knowledge Graph gap analysis
- Architecture → ADR generation from findings
- Governance → Three-branch rule evaluation
- Compliance → SecurityScanner + framework mapping
- Reporting → Assessment dashboard + board packs

The topology browser renders all three graphs with dimensional
overlays. The governance.yaml schema defines the artifact types.
The traceability matrix is the Diff overlay.

## References

- ArcKit: github.com/tractorjuice/arc-kit
- Doc-type ontology: arckit-claude/config/doc-types.mjs
- Wardley Maps: wardleymaps.com
- WeftOS ADR-023: Assessment as Kernel Service
- WeftOS ADR-033: Three-Branch Constitutional Governance
