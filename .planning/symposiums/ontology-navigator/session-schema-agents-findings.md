# Addendum: Schema Builder Agents

## The Problem

Writing topology schemas by hand requires understanding the YAML
format, knowing what entity types exist, and translating domain
knowledge into formal declarations. This is a barrier to adoption.

## The Solution: Specialized Agents That Build Schemas

Agents that consume any source material and produce composable
`.topology.yaml` fragments. The agent doesn't need to produce a
complete schema — it produces the parts it can infer, and the user
composes them.

## Agent Types

### 1. Framework Schema Agent

**Input**: A published framework (NIST AI RMF, EU AI Act, RAMP,
ArchiMate, BPMN, DTDL, ML Schema, PROV-O, SOC2, ISO 27001, etc.)

**Process**:
- Parse the framework's entity definitions (from spec docs, OWL
  files, XSD schemas, or structured PDFs)
- Map entities to topology node types with IRIs
- Map relationships to edge types with cardinality
- Infer geometry from structure (hierarchical frameworks → tree,
  process frameworks → layered, network frameworks → force)
- Generate maturity/scoring dimensions if the framework defines them

**Output**: A composable `{framework-name}.topology.yaml` fragment

**Example invocation**:
```
weaver schema from-framework --source "NIST AI RMF 1.0"
weaver schema from-framework --source ./soc2-controls.pdf
weaver schema from-framework --source https://purl.org/dc/terms/
```

### 2. Codebase Schema Agent

**Input**: A source code repository or graphify KnowledgeGraph export

**Process**:
- Run graphify extraction (or read existing export)
- Analyze entity type distribution and relationship patterns
- Cluster entities by co-occurrence and naming conventions
- Infer containment hierarchy from directory structure + Contains edges
- Propose geometry per type based on Session 3 auto-detection heuristics
- Generate IRIs from repository URL + package structure

**Output**: A `{project-name}.topology.yaml` inferred from the code

**Example invocation**:
```
weaver schema infer --path ./my-project
weaver schema infer --from-export graphify-out/graph.json
```

This is the `weaver ontology infer` from Q&A decision #2, now
formalized as an agent.

### 3. Document Schema Agent

**Input**: Planning docs, architecture docs, requirements, RFPs,
meeting notes, research papers — any unstructured text

**Process**:
- Extract entity mentions via NER (people, systems, processes,
  data stores, interfaces, etc.)
- Classify entities against known framework types (if a framework
  schema is loaded as reference)
- Extract relationships from sentence structure ("Service A
  connects to Database B", "Team X owns Component Y")
- Resolve ambiguous types using IRI context (the "Service" problem)
- Generate a draft schema fragment with confidence scores per
  entity/edge — low confidence items flagged for human review

**Output**: A `{document-name}.topology.yaml` fragment with
confidence annotations

**Example invocation**:
```
weaver schema from-docs --path ./planning/architecture.md
weaver schema from-docs --path ./requirements/ --reference nist-ai-rmf.yaml
```

### 4. Visual Schema Agent

**Input**: Napkin drawings, whiteboard photos, diagrams, screenshots

**Process**:
- Vision model extracts shapes, labels, arrows from image
- Shapes mapped to node types (rectangles → services/components,
  circles → actors/processes, cylinders → databases, clouds →
  external systems)
- Arrows mapped to edge types (solid → direct dependency,
  dashed → optional/async, labeled arrows → named relationships)
- Spatial layout preserved as geometry hint (top-down → hierarchy,
  left-right → pipeline/flow, radial → network)
- Labels become entity names, resolved to IRIs if a reference
  schema is loaded

**Output**: A `{image-name}.topology.yaml` fragment

**Example invocation**:
```
weaver schema from-image --path ./whiteboard-photo.jpg
weaver schema from-image --path ./napkin-arch.png --reference software.yaml
```

### 5. Telemetry Schema Agent

**Input**: Runtime telemetry streams, APM data, log files, metrics
endpoints (Prometheus, OpenTelemetry, CloudWatch)

**Process**:
- Discover service names from metric labels/tags
- Infer topology from trace spans (service A called service B)
- Map metric types to entity properties (CPU, memory, latency,
  error rate)
- Detect communication patterns → edge types
- Compare against declared schema (if loaded) to find observed-
  but-undeclared connections (the RAMP declared-vs-observed pattern)

**Output**: An `observed-{source}.topology.yaml` fragment
representing what's actually running

**Example invocation**:
```
weaver schema from-telemetry --source prometheus://localhost:9090
weaver schema from-telemetry --source ./otel-traces.json
weaver schema from-telemetry --source cloudwatch --region us-east-1
```

## Composability

All agents produce fragments in the same YAML format. Fragments
compose via the Docker-config layering model:

```
base:       software.yaml              (shipped preset)
framework:  nist-ai-rmf.yaml           (from framework agent)
inferred:   my-project.topology.yaml   (from codebase agent)
docs:       architecture.topology.yaml (from document agent)
observed:   prod-traffic.topology.yaml  (from telemetry agent)
napkin:     whiteboard.topology.yaml    (from visual agent)
local:      overrides.local.yaml        (hand-edited tweaks)
```

The schema loader merges them in order. Conflicts flagged. The
diff between layers IS the assessment — inferred vs declared,
observed vs planned.

## Agent Implementation

Each agent is a WeftOS agent type (lives in the agency system),
spawnable via `weaver agent spawn -t schema-builder`:

```rust
// New agent types in clawft-core
enum SchemaBuilderVariant {
    Framework,   // parses published standards
    Codebase,    // runs graphify + clusters
    Document,    // NER + relationship extraction
    Visual,      // vision model + shape detection
    Telemetry,   // trace/metric analysis
}
```

Each variant implements the `Analyzer` trait from ADR-023, producing
findings that include a draft schema fragment. The assessment
pipeline already supports pluggable analyzers — schema builders
are a natural extension.

The LLM is used for:
- Document agent: NER, relationship extraction, type classification
- Visual agent: image understanding, shape/label extraction
- Framework agent: parsing unstructured spec documents (not needed
  for formal OWL/XSD sources)

Non-LLM paths exist for:
- Codebase agent: graphify AST extraction (tree-sitter, no LLM)
- Telemetry agent: trace/metric parsing (structured data, no LLM)
- Framework agent with formal schema: OWL/XSD/JSON-LD parsing

## Prompt Injection Consideration

Schema builder agents consume external content (documents, images,
telemetry) and produce YAML that controls system behavior. This is
a prompt injection vector — a malicious document could try to
inject schema rules that grant excessive access or hide entities.

Mitigation:
- All agent-produced schemas are **draft** — require human `ratify`
  (the 7th R) before activation
- Schema fragments include a `confidence` field per entry; items
  below threshold require explicit approval
- The PI-1 sanitization pipeline (Sprint 17) applies to all LLM
  inputs in the document and visual agents
- Schema validation rejects entries with suspicious IRIs or
  impossible cardinalities
