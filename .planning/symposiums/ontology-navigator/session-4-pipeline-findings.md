# Session 4 Findings: Pipeline Architecture

## Data Flow

```
Sources → Ingestion → Classification → Edge Extraction → Topology Encoding → Layout → Serialization → Renderer
```

Six stages, each a trait object in a composable chain.

## What's Built vs Missing

| Stage | Crate | Status |
|-------|-------|--------|
| AST extraction | clawft-graphify + clawft-plugin-treesitter | Built |
| Vault enrichment | clawft-graphify::vault | Built (v0.6.11) |
| Config classification | clawft-kernel (TopologyAnalyzer) | Built |
| EntityId hashing | clawft-graphify::entity | Built |
| IRI assignment | — | **Missing** |
| Schema-validated edges | clawft-graphify::validation | **Minimal** (JSON shape only) |
| EML parameter computation | eml-core | Built (not wired to layout) |
| Resource tree (nesting) | exo-resource-tree | Built (Merkle, no triage formalization) |
| RVF serialization | clawft-weave | Built |
| Telemetry ingestor | — | **Missing** |
| OWL/RDF ingestor | — | **Missing** |
| Layout engine | — | **Missing** (entire visual pipeline) |
| JSON serializer | — | **Missing** |
| Tree calculus dispatch | — | **Missing** (implicit in ResourceTree, not formalized) |

## Priority Gaps

1. **Schema-validated edge checking** — highest leverage. Gates data quality for everything downstream.
2. **IRI resolution layer** — context-based disambiguation before hashing.
3. **Layout engine** — the entire visual pipeline. Consumes ResourceTree + schema geometry + EML params → positioned geometry.
4. **Tree calculus formalization** — make Atom/Sequence/Branch classification explicit in exo-resource-tree.
5. **Telemetry + OWL/RDF ingestors** — new source types behind an Ingest trait.
6. **JSON serialization** — straightforward once layout engine exists.

## Architecture Decision: Trait Chain

Pipeline stages compose as trait objects:

```rust
trait Ingest { fn ingest(&self, source: &Source) -> Vec<RawEntity>; }
trait Classify { fn classify(&self, entity: &RawEntity, schema: &TopologySchema) -> Entity; }
trait ExtractEdges { fn extract(&self, entities: &[Entity]) -> Vec<Relationship>; }
trait Encode { fn encode(&self, graph: &KnowledgeGraph) -> TopologyTree; }
trait Layout { fn layout(&self, tree: &TopologyTree, schema: &TopologySchema) -> PositionedGraph; }
trait Serialize { fn serialize(&self, graph: &PositionedGraph) -> Vec<u8>; }
```

Each swappable, testable, extensible independently.
