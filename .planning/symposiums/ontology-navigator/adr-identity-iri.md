# ADR: IRI-Based Identity for Ontology Interoperability

**Status**: Proposed
**Date**: 2026-04-17
**Context**: Symposium Q&A — "are we using IRI or UUID for classes?"

## Problem

EntityId is a blake3 hash of `(domain_tag, entity_type, name, source_file)`.
This is excellent for internal deduplication but has no semantic identity.

In OWL/RDF, the ontology class isn't the word "Person" — it's the IRI
`http://xmlns.com/foaf/0.1/Person`. Two ontologies can both have a
"Person" type but they're different classes if the IRIs differ. The
label is just a display string.

### The "Service" Problem

The word "service" appears in multiple domains with completely
different meanings:

- **`weftos:architecture#Service`** — a microservice or backend
  process (has endpoints, runs on infrastructure)
- **`weftos:support#Service`** — customer service / support process
  (has SLAs, handled by people)
- **`weftos:maintenance#Service`** — to service/repair a machine
  (is an action, has duration, performed by technician)

Today all three map to `EntityType::Service` (single Rust enum
variant, discriminant `"service"`). The system literally cannot
distinguish them. When a document says "service" we need to link
it to the correct ontology concept based on context — architecture
docs point to one IRI, support docs point to a different IRI.

**The word is a label. The IRI is the concept.** Same spelling,
completely different meaning in the graph. This is not a nice-to-have
— without it, the graph conflates unrelated things and the topology
becomes meaningless noise.

### What's broken without IRIs:
- Can't distinguish same-word-different-meaning entities
- Can't merge two graphs that both have "Service"
- Can't link to external ontologies (Dublin Core, Schema.org, FOAF)
- Can't say "this WeftOS Agent = that system's Actor"
- Can't publish our ontology for others to reference
- The topology schema keys are strings, not globally unique identifiers
- Classification depends on file path heuristics, not semantic identity

## Current Identity Model

```rust
// EntityId = blake3(domain_tag + entity_type + name + source_file)
// 32 bytes, deterministic, content-addressed
struct EntityId([u8; 32]);

// Entity has: id, entity_type, label, source_file, metadata
// No IRI, no namespace, no base URI
struct Entity {
    id: EntityId,
    entity_type: EntityType,
    label: String,
    ...
}
```

## Proposed: Dual Identity

Keep blake3 EntityId for internal use (it's fast and already
everywhere). Add an optional IRI for external identity.

```rust
struct Entity {
    id: EntityId,           // internal: blake3 hash (unchanged)
    iri: Option<String>,    // external: globally unique IRI
    entity_type: EntityType,
    label: String,
    ...
}
```

### IRI Construction

For WeftOS entities, generate IRIs from a base namespace:

```
https://weftos.weavelogic.ai/ontology/{domain}#{entity_type}
```

Examples:
- `https://weftos.weavelogic.ai/ontology/code#Module`
- `https://weftos.weavelogic.ai/ontology/code#Function`
- `https://weftos.weavelogic.ai/ontology/forensic#Person`
- `https://weftos.weavelogic.ai/ontology/forensic#Evidence`

For instances (specific entities, not classes):
```
https://weftos.weavelogic.ai/ontology/code#Module/auth
https://weftos.weavelogic.ai/ontology/forensic#Person/john-doe
```

For external mappings (equivalence):
```yaml
# In the topology schema
nodes:
  person:
    iri: "https://weftos.weavelogic.ai/ontology/forensic#Person"
    same_as:
      - "http://xmlns.com/foaf/0.1/Person"
      - "http://schema.org/Person"
    geometry: force
```

### Relationship IRIs

Same pattern for relationship types:
```
https://weftos.weavelogic.ai/ontology/code#calls
https://weftos.weavelogic.ai/ontology/code#contains
https://weftos.weavelogic.ai/ontology/forensic#witnessed_by
```

### Schema Identity

The topology schema itself gets an IRI:
```yaml
name: software
iri: "https://weftos.weavelogic.ai/schema/software/1.0"
```

This means two systems loading the same schema IRI know they're
talking about the same ontology, even if the YAML files differ
in layout/style configuration.

## Impact on EntityType

The Rust enum stays as a convenience for pattern matching, but it's
no longer the source of truth for identity. The enum variant is a
**shorthand** that maps to an IRI via the schema:

```rust
// EntityType enum stays for ergonomics
enum EntityType {
    Service,        // shorthand, ambiguous alone
    Custom(String), // for user-defined types
    ...
}

// Entity gains IRI as the real identity
struct Entity {
    id: EntityId,              // internal: blake3 hash
    iri: Option<String>,       // external: the real concept
    entity_type: EntityType,   // convenience for match arms
    label: String,             // human display
    ...
}
```

The IRI resolves the ambiguity that the enum cannot:

```rust
// Same EntityType::Service, different IRIs = different concepts
Entity { iri: Some("weftos:arch#Service"), entity_type: Service, label: "Auth Service" }
Entity { iri: Some("weftos:support#Service"), entity_type: Service, label: "Customer Service" }
Entity { iri: Some("weftos:maint#ServiceAction"), entity_type: Custom("service_action"), label: "Service Motor" }
```

## Impact on TopologySchema

The schema moves from string keys to IRI-identified concepts.
The string key becomes a local shorthand; the IRI is the identity:

```yaml
nodes:
  # The key "service" is local shorthand within this schema
  # The IRI is the globally unique concept identity
  service:
    iri: "https://weftos.weavelogic.ai/ontology/architecture#Service"
    same_as:
      - "http://schema.org/WebAPI"
    geometry: force
    contains: [endpoint, config]
    display_name: "Microservice"

  # Same word, different concept, different key
  support_service:
    iri: "https://weftos.weavelogic.ai/ontology/support#Service"
    same_as:
      - "http://schema.org/CustomerService"
    geometry: tree
    contains: [ticket, escalation]
    display_name: "Support Service"

  service_action:
    iri: "https://weftos.weavelogic.ai/ontology/maintenance#ServiceAction"
    same_as:
      - "http://schema.org/RepairAction"
    geometry: timeline
    time_field: scheduled_at
    display_name: "Service/Repair"

edges:
  - type: witnessed_by
    iri: "https://weftos.weavelogic.ai/ontology/forensic#witnessedBy"
    from: event
    to: person
```

## Impact on Classification Pipeline

When the system encounters the word "service" in a document, it
needs to resolve it to the correct IRI. This is a **word sense
disambiguation** problem. The pipeline becomes:

```
1. Extract mention: "service" in context "...the auth service
   handles OAuth tokens..."

2. Context classification:
   - File path: architecture/auth.md → architecture domain
   - Surrounding terms: OAuth, tokens, endpoint → tech context
   - Schema lookup: architecture domain has
     weftos:arch#Service with keywords [microservice, backend,
     API, endpoint]

3. IRI resolution: "service" → weftos:arch#Service (not support,
   not maintenance)

4. Entity creation: Entity { iri: weftos:arch#Service, label:
   "Auth Service", ... }
```

This is where EML can help: train a classifier on
(context_words, file_path, domain_tag) → IRI. The schema provides
the candidate IRIs; EML scores the match.

## Impact on Graph Merging

With IRIs, merging two graphs becomes:
1. Align entities by IRI (not by label or type string)
2. Entities with same IRI = same thing, merge metadata
3. Entities with different IRIs but `same_as` = equivalent, link
4. Entities with no IRI match = separate, show both
5. Same-word-different-IRI = explicitly distinct, never merge

This solves the multi-domain question from Session 2: load both
`software.yaml` and `investigation.yaml`, IRIs prevent collision.

## Impact on the Navigator

The navigator renders by IRI, not by label. Two nodes labeled
"Service" appear as visually distinct because they have different
IRIs and therefore different schema entries (different colors,
shapes, geometry). The detail panel shows the IRI and any
`same_as` equivalences.

## Impact on VOWL/RDF Export

With IRIs on entities and relationships, the VOWL JSON export and
any future RDF export become semantically correct — not just
structural graphs but proper linked data that can be loaded into
a triple store or linked with external ontologies.

## Implementation

### Phase 1 — Identity (Sprint 17)
1. Add `iri: Option<String>` to `Entity` struct (~5 LOC)
2. Add `iri` and `same_as` fields to `NodeTypeConfig` (~10 LOC)
3. Add `iri` field to `EdgeTypeConfig` (~5 LOC)
4. Generate default IRIs from `weftos.weavelogic.ai/ontology/` base
5. Schema loader validates IRI uniqueness within a schema

### Phase 2 — Resolution (Sprint 18)
6. Context-based IRI resolution in vault enrichment
7. EML word-sense classifier (context → IRI)
8. Fallback: if no IRI resolved, use `EntityType::discriminant()`
   mapped through schema to default IRI

### Phase 3 — Merging (Sprint 19)
9. Graph merge by IRI alignment
10. `same_as` transitive closure for equivalence
11. Conflict detection: same IRI, incompatible metadata → warn
