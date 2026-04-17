# Addendum: AI Adoption & Pipeline Frameworks for Ontology Validation

## Summary

Surveyed 20+ frameworks across 6 categories. Seven have formal,
importable schemas that can serve as topology presets without manual
ontology construction.

## Directly Importable (formal schema exists)

| Framework | Format | Entity Types | Maps To |
|-----------|--------|-------------|---------|
| **ML Schema** (W3C) | OWL/RDF | Run, Algorithm, Dataset, Model, ModelEvaluation, Task | AI pipeline topology preset |
| **PROV-O** (W3C) | OWL | Entity, Activity, Agent + derivation/usage/generation | Provenance tracking layer across ALL topologies |
| **DTDL v3** (Microsoft) | JSON-LD | Interface, Telemetry, Property, Command, Relationship, Component | IoT/sensor topology preset (robotics.yaml) |
| **AAS** (Industry 4.0) | JSON/XML | Asset, Submodel, SubmodelElement, ConceptDescription | Industrial asset topology preset |
| **ArchiMate 3.2** (Open Group) | XML | BusinessProcess, ApplicationComponent, Node, Artifact + 3 layers | Base enterprise topology schema |
| **BPMN 2.0** (OMG) | XSD | Process, Task, Gateway, Event, Flow | Workflow topology layer |
| **OPC UA** | XML NodeSet | Object, Variable, Method, ReferenceType | Industrial automation topology |

## Maturity/Scoring Frameworks (conceptual, use as dimensions)

| Framework | Dimensions | Levels | Use As |
|-----------|-----------|--------|--------|
| **NIST AI RMF** | Govern, Map, Measure, Manage | Maturity tiers per function | AI governance overlay scoring |
| **EU AI Act** | Risk classification | Unacceptable/High/Limited/Minimal | Risk property on AI system nodes |
| **Google MLOps Levels** | Pipeline maturity | 0 (manual) → 1 (automated) → 2 (CI/CD) | ML infrastructure maturity score |
| **Microsoft AI Maturity** | Strategy, Culture, Capabilities, Org | 5 levels (Foundational→Transformational) | Org-unit maturity radar |
| **ISO/IEC 42001** | AI management controls | Plan-Do-Check-Act | Compliance assessment preset |

## Ontology Validation Strategy

Use these frameworks to verify and refine our topology model:

### 1. Import formal schemas as reference presets

Load ML Schema, PROV-O, DTDL, ArchiMate as reference ontologies.
Compare their entity types and relationships against our schema
presets. Where they define entities we don't have → gap in our model.
Where we define entities they don't have → domain-specific extension.

### 2. Cross-reference maturity dimensions

Our 8 analyzers + RAMP's 6 dimensions + NIST's 4 functions + EU AI
Act's risk tiers = a multi-framework maturity assessment. Each
framework contributes scoring dimensions. The topology browser's
heatmap overlay can switch between framework-specific colorings:

```yaml
modes:
  heatmap:
    metrics:
      - field: ramp_maturity
        label: "RAMP Maturity (0-4)"
        palette: rdylgn
      - field: nist_ai_rmf_govern
        label: "NIST AI RMF - Govern"
        palette: viridis
      - field: eu_ai_act_risk
        label: "EU AI Act Risk Tier"
        palette: inferno
        range: [0, 3]  # 0=minimal, 3=unacceptable
      - field: mlops_level
        label: "MLOps Maturity"
        palette: rdylgn
        range: [0, 2]
```

### 3. PROV-O as universal provenance layer

PROV-O's Entity/Activity/Agent triad maps to EVERYTHING:
- Software: Module (Entity) built by CI (Activity) authored by
  Developer (Agent)
- Investigation: Evidence (Entity) collected by Search (Activity)
  led by Detective (Agent)
- Assessment: Finding (Entity) produced by Analyzer (Activity)
  run by WeftOS (Agent)

Add PROV-O as a composable overlay schema that layers onto any
domain preset, giving provenance tracking for free.

### 4. DTDL for sensor/IoT validation

DTDL's Interface/Telemetry/Property/Command model validates our
robotics.yaml preset. Where DTDL defines relationships we don't
have (e.g., Component composition, Relationship semantics with
properties on edges) → extend our schema model.

### 5. ArchiMate as enterprise baseline

ArchiMate's three-layer model (Business, Application, Technology)
validates our 7-layer ontology from the symposium agenda. Mapping:

| Our Layer | ArchiMate Layer |
|-----------|----------------|
| Concepts | Motivation (goals, requirements, principles) |
| Architecture | Application (components, services, interfaces) |
| Code | Implementation (artifacts, functions) |
| Data | Passive structure (data objects, business objects) |
| Agents | Active structure (application components, processes) |
| Infrastructure | Technology (nodes, devices, networks) |
| People | Business (actors, roles, collaborations) |

Good alignment. ArchiMate adds Motivation layer (goals, drivers,
requirements) that we mapped to "Concepts" — confirms the layer
is needed.

## Impact on Build Plan

### Immediate
- Add PROV-O entity types (Entity, Activity, Agent) to clawft-types
  as a composable mixin, not a separate domain
- Add EU AI Act risk_tier as a standard metadata property
- Add MLOps level as a standard metadata property

### Phase 2
- Import ML Schema OWL as a reference; generate mlops.yaml preset
- Import DTDL JSON-LD; validate against robotics.yaml
- Import ArchiMate XML; validate against software.yaml + our 7 layers

### Phase 3
- Multi-framework maturity dashboard: spider chart with selectable
  framework (RAMP / NIST AI RMF / Microsoft AI Maturity / MLOps)
- PROV-O provenance edges on ExoChain events

## Key References

- ML Schema: ml-schema.github.io (W3C Community Group)
- PROV-O: w3.org/TR/prov-o/
- DTDL v3: github.com/Azure/opendigitaltwins-dtdl
- AAS: admin-shell-io/aas-specs
- ArchiMate 3.2: pubs.opengroup.org/architecture/archimate3-doc/
- BPMN 2.0: omg.org/spec/BPMN/2.0/
- NIST AI RMF: nist.gov/artificial-intelligence/risk-management-framework
- EU AI Act: artificialintelligenceact.eu
- ISO/IEC 42001: iso.org/standard/81230.html
