# ECC Application Mapping: Cold Case Homicide Investigation

**Status**: Research Complete  
**Date**: 2026-04-04  
**Source**: WeftOS ECC substrate (`crates/clawft-kernel/src/` -- causal.rs, democritus.rs, hnsw_service.rs, crossref.rs, calibration.rs, impulse.rs)

---

## 1. ECC Primitives to Investigation Concepts

### 1.1 Core Mapping Table

| ECC Component | Rust Type / Module | Investigation Application | Notes |
|---|---|---|---|
| **CausalNode** | `CausalNode { id: NodeId, label: String, metadata: Value }` in `causal.rs` | Evidence item, witness statement, suspect, location, event, hypothesis | The `metadata: serde_json::Value` field carries arbitrary structured data -- forensic report contents, GPS coordinates, testimony transcripts. The `label` field provides human-readable identification (e.g., `"witness:jane_doe:interview_2024-03-15"`). |
| **CausalEdge** | `CausalEdge { source, target, edge_type, weight, timestamp, chain_seq }` in `causal.rs` | Evidence relationships -- alibis, contradictions, corroboration, causal chains, temporal sequencing | The `weight: f32` field maps to confidence/strength of the evidentiary link (0.0 = speculative, 1.0 = forensically confirmed). `chain_seq` ties to ExoChain for provenance. |
| **CausalEdgeType** | 8-variant enum in `causal.rs` | See Section 1.2 below | Each variant maps to a specific investigative relationship type. |
| **HNSW vector search** | `HnswService` in `hnsw_service.rs` | Semantic similarity between cases, MO matching, finding related unsolved cases, witness statement comparison | Cosine similarity with configurable `ef_search=100`, `ef_construction=200`, `default_dimensions=384`. The `search_batch` method is critical -- it acquires the mutex once for multiple queries, enabling efficient cross-case comparison. |
| **DEMOCRITUS loop** | `DemocritusLoop` in `democritus.rs` | Continuous re-analysis cycle: SENSE (new tips, lab results) -> EMBED (vectorize) -> SEARCH (find related evidence) -> UPDATE (link evidence graph) -> COMMIT (audit trail) | The `tick_budget_us: u64` field prevents runaway analysis. `max_impulses_per_tick: 64` throttles intake. Budget-exceeded flag signals when incoming evidence volume exceeds processing capacity -- a direct analog to investigator workload. |
| **Spectral analysis** | `spectral_analysis()` and `spectral_partition()` in `causal.rs` | Suspect clustering, witness network analysis, evidence grouping, identifying disconnected evidence islands | Uses sparse Lanczos iteration at O(k*m) complexity. The Fiedler vector partitions the evidence graph into natural clusters -- e.g., separating two distinct suspect theories. |
| **Coherence score (lambda_2)** | `SpectralResult { lambda_2: f64, fiedler_vector, node_ids }` in `causal.rs` | Case completeness metric -- how well the evidence graph holds together | lambda_2 = 0 means the evidence graph is **disconnected** (critical gap). Higher values = stronger connectivity = more coherent case narrative. This is the single most valuable metric for cold case triage. |
| **Cross-references (BLAKE3)** | `CrossRef` and `CrossRefStore` in `crossref.rs` | Evidence corroboration -- when two independent pieces of evidence point to the same conclusion | `UniversalNodeId` uses BLAKE3 hash of `(structure_tag, context_id, hlc_timestamp, content_hash, parent_id)`. The forward/reverse `DashMap` indices enable bidirectional traversal: "what evidence supports this conclusion?" and "what conclusions does this evidence support?" |
| **CrossRefType** | 8-variant enum in `crossref.rs` | Investigative relationship semantics | `TriggeredBy` = investigation action prompted by evidence; `EvidenceFor` = direct evidentiary support; `Elaborates` = additional detail on existing evidence; `TomInference` = theory-of-mind -- what a suspect likely knew/intended. |
| **Impulse queue** | `ImpulseQueue` with `emit()` and `drain_ready()` in `impulse.rs` | New tips, lab results, witness re-interviews, CODIS hits -- ephemeral signals that decay if not acted on | HLC-sorted for causal ordering. `ImpulseType::NoveltyDetected` maps to unexpected new evidence. `ImpulseType::CoherenceAlert` maps to contradiction detection. `ImpulseType::BeliefUpdate` maps to revised witness statements. |
| **ExoChain** | Chain events in `chain.rs` | Chain of custody, audit trail for every analytical step | Every evidence insertion, relationship link, and analytical conclusion is immutably recorded with `chain_seq` provenance. The `chain_event_source`, `chain_event_kind`, and `chain_event_payload` trait methods ensure every action is attributable. |
| **Governance (3-branch)** | `GovernanceBranch { Legislative, Executive, Judicial }` in `governance.rs` | Investigation authorization, evidence handling protocols, case review board | **Legislative** = evidence handling SOPs, search warrant requirements, data protection rules. **Executive** = investigator authorization, resource allocation, case assignment. **Judicial** = CGR validation, bias checks, audit compliance, Brady disclosure enforcement. |
| **EffectVector (5D)** | `EffectVector { risk, fairness, privacy, novelty, security }` in `governance.rs` | Action risk assessment for investigative steps | `risk` = probability of evidence contamination or procedural error. `fairness` = bias detection in suspect selection. `privacy` = civilian data protection (wiretaps, cell records). `novelty` = untested investigative technique risk. `security` = chain of custody integrity. |
| **EccCalibration** | `run_calibration()` in `calibration.rs` | System capacity assessment -- how large a case graph can this deployment handle in real-time | `spectral_capable: bool` determines whether coherence scoring is feasible. `tick_interval_ms` auto-adjusts to hardware. A department laptop may run 50ms ticks; a regional crime lab server may run 10ms ticks with full spectral analysis. |

### 1.2 CausalEdgeType Mapping

| Edge Type | Investigation Meaning | Example |
|---|---|---|
| `Causes` | Direct causation | "Gunshot wound caused death" |
| `Inhibits` | Suppresses/prevents | "Alibi at 9pm inhibits suspect presence at crime scene at 9pm" |
| `Correlates` | Statistical correlation, co-occurrence | "Same MO appears in 3 other unsolved cases" |
| `Enables` | Precondition | "Access to victim's house enables opportunity" |
| `Follows` | Temporal sequence | "Witness saw suspect's car at 8:45pm, victim's phone went silent at 9:02pm" |
| `Contradicts` | Evidence conflicts | "Witness A says blue car; witness B says red truck" |
| `TriggeredBy` | Investigation action prompted by | "DNA test triggered by cold case review" |
| `EvidenceFor` | Supporting evidence | "Fingerprint match supports suspect identification" |

### 1.3 CrossRefType Mapping

| CrossRef Type | Investigation Meaning | Example |
|---|---|---|
| `TriggeredBy` | Action prompted by evidence | "Search warrant triggered by witness tip" |
| `EvidenceFor` | Corroboration across structures | "Cell tower data (HNSW match) corroborates witness timeline (CausalGraph)" |
| `Elaborates` | Additional detail | "Forensic report elaborates on physical evidence item" |
| `EmotionCause` | Motive inference | "Financial dispute caused emotional state leading to confrontation" |
| `GoalMotivation` | Intent/motive link | "Insurance policy creates goal motivation for homicide" |
| `SceneBoundary` | Crime scene delineation | "Evidence found at boundary of primary and secondary scenes" |
| `MemoryEncoded` | Witness memory reliability | "Statement recorded 3 days post-event; memory encoding quality assessed" |
| `TomInference` | Theory of mind | "Suspect likely knew victim would be alone based on social media posts" |

---

## 2. The Case Graph Model

### 2.1 Node Types

Each node type is a `CausalNode` with a typed `label` prefix and structured `metadata`. The `metadata: serde_json::Value` field carries type-specific attributes.

```
Node Label Convention: "{type}:{subtype}:{identifier}"
```

#### PERSON

```json
{
  "label": "person:suspect:john_doe_1987",
  "metadata": {
    "node_type": "PERSON",
    "subtype": "suspect|victim|witness|investigator|expert|informant|person_of_interest",
    "full_name": "John Michael Doe",
    "dob": "1987-03-15",
    "aliases": ["JD", "Johnny"],
    "status": "active_suspect|cleared|deceased|unknown",
    "relationship_to_victim": "ex-partner",
    "criminal_history_ref": "ncic:12345",
    "dna_on_file": true,
    "fingerprints_on_file": true,
    "last_known_address": "123 Main St, Springfield, IL",
    "contact_info": "redacted",
    "reliability_score": 0.0
  }
}
```

- `reliability_score` (0.0-1.0): For witnesses, tracks historical accuracy. Starts at 0.0 (unknown), updated as statements are corroborated or contradicted by physical evidence.

#### EVENT

```json
{
  "label": "event:crime:homicide_2019-07-14",
  "metadata": {
    "node_type": "EVENT",
    "subtype": "crime|interview|arrest|court_appearance|evidence_collection|search_warrant|autopsy|lab_analysis|tip_received|surveillance|canvass",
    "datetime": "2019-07-14T22:30:00Z",
    "datetime_uncertainty_minutes": 45,
    "location_ref": "location:crime_scene:123_elm_st",
    "participants": ["person:victim:jane_smith", "person:suspect:john_doe_1987"],
    "description": "Victim found deceased in residence",
    "source_document_ref": "document:report:incident_2019-4521",
    "verified": false
  }
}
```

- `datetime_uncertainty_minutes`: Critical for cold cases where exact times are unknown. A 45-minute uncertainty window enables timeline overlap analysis.

#### EVIDENCE

```json
{
  "label": "evidence:physical:dna_swab_47",
  "metadata": {
    "node_type": "EVIDENCE",
    "subtype": "physical|testimonial|digital|forensic|circumstantial|documentary",
    "evidence_class": "biological|firearm|toolmark|fingerprint|fiber|digital|document|trace",
    "collection_date": "2019-07-15",
    "collected_by": "person:investigator:det_garcia",
    "location_collected": "location:crime_scene:123_elm_st",
    "storage_location": "evidence_locker:shelf_14:bin_3",
    "tested": false,
    "test_results": null,
    "chain_of_custody_intact": true,
    "degradation_risk": "high",
    "degradation_deadline": "2025-12-31",
    "probative_value": 0.0,
    "exculpatory": false,
    "brady_disclosed": false
  }
}
```

- `tested: false` is the gap analysis trigger. Untested evidence with high `degradation_risk` generates priority impulses.
- `degradation_deadline` enables time-sensitive lead scoring.
- `brady_disclosed` tracks prosecution disclosure obligations.

#### LOCATION

```json
{
  "label": "location:crime_scene:123_elm_st",
  "metadata": {
    "node_type": "LOCATION",
    "subtype": "crime_scene|suspect_address|witness_location|cell_tower|workplace|vehicle|dump_site|staging_area",
    "address": "123 Elm Street, Springfield, IL 62701",
    "gps": { "lat": 39.7817, "lng": -89.6501 },
    "geofence_radius_meters": 50,
    "type": "residential",
    "access_points": ["front_door", "rear_window", "garage"],
    "surveillance_coverage": ["ring_doorbell", "neighbor_camera_east"],
    "processed": true,
    "processing_date": "2019-07-15"
  }
}
```

#### TIMELINE_POINT

```json
{
  "label": "timeline:suspect_movement:doe_2019-07-14_2130",
  "metadata": {
    "node_type": "TIMELINE_POINT",
    "datetime": "2019-07-14T21:30:00Z",
    "uncertainty_minutes": 15,
    "source": "witness_statement|cell_tower|surveillance|gps|financial_transaction|digital_log",
    "person_ref": "person:suspect:john_doe_1987",
    "location_ref": "location:gas_station:route_66_shell",
    "confidence": 0.85,
    "corroborated_by": ["evidence:digital:cell_tower_ping_332"]
  }
}
```

- Uncertainty ranges are first-class data, not afterthoughts. A `confidence: 0.85` timeline point with `uncertainty_minutes: 15` is fundamentally different from one with `confidence: 0.3` and `uncertainty_minutes: 120`.

#### DOCUMENT

```json
{
  "label": "document:report:autopsy_2019-4521",
  "metadata": {
    "node_type": "DOCUMENT",
    "subtype": "report|transcript|warrant|court_order|lab_result|forensic_analysis|tip_sheet|surveillance_log",
    "author": "person:expert:dr_pathologist",
    "date": "2019-07-16",
    "classification": "law_enforcement_sensitive",
    "file_ref": "/cases/2019-4521/documents/autopsy_report.pdf",
    "embedded": true,
    "embedding_model": "all-MiniLM-L6-v2"
  }
}
```

#### HYPOTHESIS

```json
{
  "label": "hypothesis:primary:doe_domestic_violence",
  "metadata": {
    "node_type": "HYPOTHESIS",
    "subtype": "primary|alternative|eliminated|prosecution_theory|defense_theory",
    "description": "John Doe killed victim during domestic dispute",
    "proposed_by": "person:investigator:det_garcia",
    "proposed_date": "2019-07-20",
    "supporting_evidence_count": 7,
    "contradicting_evidence_count": 2,
    "coherence_score": 0.42,
    "status": "active|suspended|eliminated|charged",
    "elimination_reason": null
  }
}
```

- `coherence_score` is computed by running spectral analysis on the subgraph of evidence nodes connected to this hypothesis. This is the lambda_2 of the hypothesis-local evidence graph.

### 2.2 Edge Types

All edges are `CausalEdge` instances with typed `edge_type`, `weight`, and ExoChain `chain_seq` provenance.

| Edge Type | Maps To | Weight Semantics | Example |
|---|---|---|---|
| `WITNESSED_BY` | `CausalEdgeType::EvidenceFor` | Witness reliability score | event:crime -> person:witness (weight=reliability) |
| `REPORTED_BY` | `CausalEdgeType::EvidenceFor` | Report credibility | event:tip -> person:informant |
| `FOUND_AT` | `CausalEdgeType::Correlates` | Association strength | evidence:physical -> location:crime_scene |
| `OCCURRED_AT` | `CausalEdgeType::Correlates` | Temporal certainty | event:crime -> location:crime_scene |
| `CONTRADICTS` | `CausalEdgeType::Contradicts` | Contradiction severity | evidence:statement_a -> evidence:statement_b |
| `CORROBORATES` | `CausalEdgeType::EvidenceFor` | Corroboration strength | evidence:cell_tower -> timeline:suspect_movement |
| `CAUSES` | `CausalEdgeType::Causes` | Causal confidence | event:dispute -> event:crime |
| `ENABLES` | `CausalEdgeType::Enables` | Precondition confidence | evidence:key_access -> hypothesis:forced_entry_unlikely |
| `ASSOCIATED_WITH` | `CausalEdgeType::Correlates` | Association strength (weak) | person:suspect -> person:associate (weight < 0.3) |
| `ALIBIED_BY` | `CausalEdgeType::Inhibits` | Alibi verification level | person:suspect -> evidence:alibi_statement |
| `IDENTIFIED_BY` | `CausalEdgeType::Causes` | Forensic match confidence | evidence:dna -> person:suspect (weight=match_probability) |
| `SUSPECTS` | `CausalEdgeType::EvidenceFor` | Investigator confidence | person:investigator -> person:suspect |
| `TIMELINE_PRECEDES` | `CausalEdgeType::Follows` | Temporal ordering confidence | timeline:point_a -> timeline:point_b |
| `TIMELINE_FOLLOWS` | `CausalEdgeType::Follows` | Temporal ordering confidence | timeline:point_b -> timeline:point_a (reverse) |
| `GAPS_IN` | Custom via `CausalEdgeType::Correlates` (weight=0.0) | Gap severity | hypothesis:primary -> evidence:untested_dna (weight=0.0 signals absence) |

### 2.3 Graph Schema Diagram

```
PERSON ──WITNESSED_BY──> EVENT
PERSON ──ALIBIED_BY───> EVIDENCE
PERSON ──SUSPECTS─────> PERSON
PERSON <──IDENTIFIED_BY── EVIDENCE

EVENT ──OCCURRED_AT───> LOCATION
EVENT ──TIMELINE_PRECEDES──> EVENT
EVENT ──CAUSES────────> EVENT

EVIDENCE ──FOUND_AT───> LOCATION
EVIDENCE ──CORROBORATES──> EVIDENCE
EVIDENCE ──CONTRADICTS──> EVIDENCE
EVIDENCE ──CORROBORATES──> HYPOTHESIS

HYPOTHESIS ──GAPS_IN──> EVIDENCE (untested/missing)
HYPOTHESIS ──supported_by subgraph──> (coherence_score = lambda_2)

DOCUMENT ──ELABORATES──> EVIDENCE
DOCUMENT ──ELABORATES──> EVENT

TIMELINE_POINT ──PRECEDES──> TIMELINE_POINT
TIMELINE_POINT ──CORROBORATED_BY──> EVIDENCE
```

---

## 3. Gap Analysis Engine

### 3.1 Architecture

The gap analysis engine leverages three ECC primitives working together:

1. **Spectral analysis** (`CausalGraph::spectral_analysis()`) computes lambda_2 as the case coherence score
2. **Cross-reference store** (`CrossRefStore::get_reverse()`) finds what should exist but does not
3. **HNSW search** (`HnswService::search()`) finds similar solved cases to identify patterns of evidence that are typically present

### 3.2 Gap Detection Patterns

Each gap type maps to a specific structural deficiency in the case graph, detectable algorithmically.

#### Pattern 1: Mentioned-But-Never-Interviewed Witnesses

**Detection**: Scan all `PERSON` nodes with `subtype: "witness"`. For each, check for existence of `EVENT` nodes with `subtype: "interview"` connected by a `WITNESSED_BY` edge. Witnesses mentioned in reports (connected to `DOCUMENT` nodes via `Elaborates` CrossRef) but lacking interview events are gaps.

```
FOR each person WHERE subtype = "witness":
    interview_edges = causal_graph.get_reverse_edges(person.id)
        .filter(|e| e.edge_type == EvidenceFor)
    IF interview_edges.is_empty():
        EMIT Impulse { type: CoherenceAlert, payload: "Witness {person.label} mentioned but never interviewed" }
```

**Coherence impact**: Each uninterviewed witness reduces graph connectivity. The system can compute a counterfactual lambda_2 by temporarily adding a synthetic interview node and measuring the resulting coherence increase.

#### Pattern 2: Collected-But-Never-Tested Physical Evidence

**Detection**: Scan `EVIDENCE` nodes where `metadata.tested == false`. Priority-rank by:
- `degradation_risk` (high = urgent)
- `degradation_deadline` proximity
- Estimated probative value based on HNSW similarity to evidence in solved cases

```
FOR each evidence WHERE metadata.tested == false:
    similar_tested = hnsw.search(evidence.embedding, k=10)
        .filter(|r| r.metadata.tested == true && r.metadata.case_status == "solved")
    estimated_value = mean(similar_tested.map(|r| r.metadata.probative_value))
    
    IF estimated_value > 0.5:
        EMIT Impulse {
            type: CoherenceAlert,
            payload: {
                "message": "Evidence {evidence.label} untested; similar evidence was probative in {count} solved cases",
                "estimated_coherence_delta": compute_counterfactual_lambda2(evidence),
                "degradation_deadline": evidence.metadata.degradation_deadline
            }
        }
```

**The killer metric**: "Coherence would increase from 0.42 to 0.71 if you tested the DNA from evidence item #47."

This is computed by:
1. Taking the current case subgraph
2. Adding a synthetic node representing the hypothetical test result
3. Adding edges to existing nodes based on HNSW similarity to analogous test results in solved cases
4. Running `spectral_analysis()` on the augmented graph
5. Reporting the delta: `augmented_lambda_2 - current_lambda_2`

#### Pattern 3: Unverified Alibis

**Detection**: Scan for `ALIBIED_BY` edges where the target evidence node has `metadata.verified == false`.

```
FOR each edge WHERE edge_type == Inhibits AND source.subtype == "suspect":
    alibi_evidence = causal_graph.get_node(edge.target)
    IF alibi_evidence.metadata.verified == false:
        verification_methods = suggest_verification(alibi_evidence)
        EMIT Impulse {
            type: CoherenceAlert,
            payload: {
                "message": "Alibi for {source.label} unverified",
                "alibi_claim": alibi_evidence.metadata.description,
                "suggested_verification": verification_methods,
                "impact_if_broken": compute_lambda2_without_alibi_edge(edge)
            }
        }
```

#### Pattern 4: Surveillance Footage Never Pulled

**Detection**: For each `LOCATION` node connected to the crime, check `metadata.surveillance_coverage`. For each surveillance source listed, verify existence of a corresponding `EVIDENCE` node with `subtype: "digital"` and `evidence_class: "surveillance"`.

#### Pattern 5: Cell Tower Records Never Subpoenaed

**Detection**: For each `PERSON` node with `subtype: "suspect"` or `subtype: "person_of_interest"`, check whether `EVIDENCE` nodes with `evidence_class: "digital"` and source type `cell_tower` exist for the relevant time window.

#### Pattern 6: Known Associates Never Investigated

**Detection**: For suspects, query HNSW for persons with high `ASSOCIATED_WITH` edge similarity. Cross-reference against interview events. Associates who appear in the graph (mentioned in documents, connected via social network data) but lack direct investigation events are gaps.

#### Pattern 7: Similar MO Cases Never Cross-Referenced

**Detection**: This is the HNSW search primary use case.

```
case_embedding = embed(case_summary_text)
similar_cases = hnsw.search(case_embedding, k=20)
    .filter(|r| r.score > correlation_threshold && r.metadata.case_id != current_case)

FOR each similar_case:
    crossref_exists = crossref_store.get_forward(current_case_universal_id)
        .any(|cr| cr.target == similar_case.universal_id)
    IF NOT crossref_exists:
        EMIT Impulse {
            type: NoveltyDetected,
            payload: {
                "message": "Similar MO case {similar_case.case_id} (score: {score}) never cross-referenced",
                "shared_features": extract_shared_features(current_case, similar_case),
                "solved": similar_case.metadata.status == "solved"
            }
        }
```

#### Pattern 8: Timeline Gaps

**Detection**: Sort all `TIMELINE_POINT` nodes for a given person by datetime. Identify gaps larger than a threshold where no location or activity is documented.

```
FOR each person WHERE subtype IN ("suspect", "victim"):
    points = timeline_points_for(person).sort_by(|p| p.datetime)
    FOR i in 1..points.len():
        gap_minutes = (points[i].datetime - points[i-1].datetime).as_minutes()
        IF gap_minutes > critical_gap_threshold:
            EMIT Impulse {
                type: CoherenceAlert,
                payload: {
                    "message": "Timeline gap: {person.label} unaccounted for {gap_minutes} minutes between {points[i-1].datetime} and {points[i].datetime}",
                    "gap_start": points[i-1].datetime,
                    "gap_end": points[i].datetime,
                    "includes_crime_window": overlaps(gap, crime_event.datetime, crime_event.uncertainty)
                }
            }
```

### 3.3 Coherence Dashboard

The system maintains a real-time coherence dashboard:

| Metric | ECC Source | Investigation Meaning |
|---|---|---|
| Overall lambda_2 | `causal_graph.spectral_analysis(50).lambda_2` | Case graph connectivity -- 0.0 = disconnected evidence islands, >0.5 = well-connected |
| Hypothesis-local lambda_2 | Subgraph spectral analysis per hypothesis | Which theory has the strongest evidence support? |
| Untested evidence count | Scan EVIDENCE nodes where tested=false | How much latent information exists? |
| Unverified alibi count | Scan Inhibits edges where target.verified=false | How many suspect exclusions are unverified? |
| Cross-case match count | HNSW search results above threshold | How many potentially linked cases exist? |
| Gap impulse queue depth | `impulse_queue.pending_count()` | How many actionable gaps are waiting? |
| Counterfactual delta (max) | Best single-action coherence improvement | "Testing evidence #47 would improve coherence by +0.29" |

---

## 4. Crime Scene Reconstruction Model

### 4.1 Event Sequence Reconstruction

The `CausalGraph` with `Follows` and `Causes` edges naturally represents event sequences. Crime scene reconstruction is a constraint satisfaction problem over the graph.

**Approach**: Build a timeline subgraph containing only `EVENT` and `TIMELINE_POINT` nodes connected by `TIMELINE_PRECEDES` / `FOLLOWS` edges. Each node carries `datetime` and `uncertainty_minutes`. The reconstruction algorithm:

1. **Collect all temporal constraints** from edges (A precedes B, C follows D)
2. **Propagate uncertainty** using interval arithmetic -- if A occurred at 21:00 +/- 15min and B occurred 10 minutes after A, then B occurred at 21:10 +/- 15min
3. **Detect contradictions** where propagated intervals produce empty intersections (a Contradicts edge)
4. **Produce a timeline** with uncertainty bands visualizable as a Gantt chart with fuzzy boundaries

### 4.2 Competing Timelines

Multiple `HYPOTHESIS` nodes can each anchor a different event sequence subgraph. The system supports this natively because:

- Each hypothesis is a `CausalNode`
- Evidence nodes connect to hypotheses via `EvidenceFor` or `Contradicts` edges
- Spectral analysis runs independently on each hypothesis subgraph
- The **Fiedler vector** from `spectral_partition()` naturally separates evidence that supports hypothesis A from evidence that supports hypothesis B

```
Prosecution Theory (hypothesis:prosecution:doe_domestic):
    lambda_2 = 0.71
    Supporting evidence: 12 nodes
    Contradicting evidence: 2 nodes

Defense Theory (hypothesis:defense:doe_alibi):
    lambda_2 = 0.38
    Supporting evidence: 5 nodes
    Contradicting evidence: 4 nodes

SYSTEM NOTE: Prosecution theory has 1.87x higher coherence.
             Defense theory weakened by 4 contradicting evidence items.
             Key discriminating evidence: evidence:physical:dna_swab_47 (untested)
```

### 4.3 Probability-Weighted Scenarios

Each edge `weight: f32` carries confidence. For a complete causal chain hypothesis:

```
chain_probability = product(edge.weight for edge in causal_path)
```

Using `CausalGraph::find_path(from, to, max_depth)`, the system can find the shortest causal path between any two nodes and compute the cumulative confidence:

```rust
// From causal.rs -- BFS path finding with parent tracking
pub fn find_path(&self, from: NodeId, to: NodeId, max_depth: usize) -> Option<Vec<NodeId>>
```

For each hop in the path, multiply the edge weights. A 5-hop chain with weights [0.9, 0.8, 0.95, 0.7, 0.85] yields cumulative confidence 0.41 -- meaning the overall causal claim is moderately supported.

### 4.4 "What If" Analysis

**Question**: "If we assume suspect A did it, does the evidence graph become more or less coherent?"

**Implementation**:

1. Clone the current case subgraph (or work on a shadow copy)
2. Add a synthetic `Causes` edge: `person:suspect:A -> event:crime:homicide` with weight 1.0
3. For each piece of evidence currently `ASSOCIATED_WITH` suspect A, upgrade the edge type to `EvidenceFor` with increased weight
4. For each alibi (`Inhibits` edge), check if it overlaps the crime window -- if so, add a `Contradicts` edge
5. Run `spectral_analysis()` on the augmented graph
6. Compare augmented lambda_2 to baseline lambda_2
7. Report: "Assuming suspect A, coherence changes from 0.42 to {new_lambda_2}. Key supporting evidence: [...]. Key contradictions: [...]."

This directly parallels how investigators mentally "try on" a theory and see if it fits the evidence.

---

## 5. Incoming Case Classification

### 5.1 Solvability Factor Extraction

When a new case enters the system, extract solvability factors as structured metadata:

```json
{
  "solvability_factors": {
    "witness_count": 3,
    "witness_reliability_avg": 0.6,
    "physical_evidence_items": 12,
    "physical_evidence_tested": 4,
    "suspect_identified": true,
    "motive_known": true,
    "weapon_recovered": false,
    "time_since_crime_days": 1825,
    "media_coverage": "low",
    "victim_cooperation": "deceased",
    "digital_evidence_available": true,
    "surveillance_available": false,
    "forensic_technology_advances_since": ["touch_dna", "familial_dna", "cell_site_analysis"]
  }
}
```

### 5.2 Historical Pattern Comparison

Embed the solvability factor vector into HNSW and search against the corpus of historical cases:

```
solvability_embedding = embed(solvability_factors_text)
similar_cases = hnsw.search(solvability_embedding, k=50)

solved_similar = similar_cases.filter(|c| c.metadata.status == "solved")
unsolved_similar = similar_cases.filter(|c| c.metadata.status == "unsolved")

solvability_estimate = solved_similar.len() / similar_cases.len()
```

The `search_batch` method from `HnswService` enables parallel comparison across multiple feature dimensions:

```rust
// From hnsw_service.rs -- batch search acquires mutex once
pub fn search_batch(&self, queries: &[&[f32]], top_k: usize) -> Vec<Vec<HnswSearchResult>>
```

### 5.3 MO Matching via HNSW

Embed crime characteristics (weapon type, victim profile, location type, time of day, staging behavior, etc.) and search for similar MOs:

```
mo_embedding = embed(mo_description)
mo_matches = hnsw.search(mo_embedding, k=20)
    .filter(|r| r.score > 0.75)  // High similarity threshold for MO matching

FOR each match:
    IF match.metadata.suspect_identified AND NOT match.metadata.suspect_cleared:
        FLAG: "Potential serial pattern -- suspect {match.metadata.suspect} linked to case {match.metadata.case_id} (MO similarity: {match.score})"
```

### 5.4 Evidence Degradation Risk Scoring

Time-sensitive leads get priority based on degradation modeling:

```
FOR each evidence WHERE tested == false:
    days_since_collection = (now - evidence.collection_date).days()
    degradation_rate = degradation_model(evidence.evidence_class)
    
    remaining_viability = max(0.0, 1.0 - (days_since_collection * degradation_rate))
    
    priority_score = remaining_viability * estimated_probative_value * (1.0 / time_to_deadline_days)
    
    IF priority_score > threshold:
        EMIT Impulse {
            type: CoherenceAlert,
            payload: { "urgency": "time_sensitive", "days_remaining": time_to_deadline_days }
        }
```

### 5.5 Resource Allocation Recommendation

Combine solvability estimate, evidence degradation urgency, and available investigator capacity:

```
case_priority = (solvability_estimate * 0.4) 
              + (max_coherence_delta * 0.3)
              + (degradation_urgency * 0.2)
              + (victim_family_engagement * 0.1)

recommendation = {
    "priority_rank": rank,
    "estimated_hours": hours_model(case_complexity),
    "recommended_actions": top_3_coherence_improving_actions,
    "specialist_needed": ["dna_analyst", "digital_forensics", "cell_site_expert"],
    "similar_solved_cases_for_reference": top_3_similar_solved
}
```

---

## 6. The "Case as Conversation" Model

### 6.1 Evidence as Messages

The DEMOCRITUS loop (`democritus.rs`) already models this. Each piece of evidence is an `Impulse` -- an ephemeral signal that enters the system, gets embedded, searched against existing knowledge, linked into the causal graph, and committed to the audit trail.

```
Impulse (evidence arrives)
    |
    v
SENSE: ImpulseQueue.drain_ready() -- new lab result, witness tip, CODIS hit
    |
    v
EMBED: EmbeddingProvider.embed_batch() -- vectorize the evidence description
    |
    v
SEARCH: HnswService.search_batch() -- find related evidence and cases
    |
    v
UPDATE: CausalGraph.add_node() + .link() -- integrate into case graph
    |   CrossRefStore.insert() -- link across structures
    |
    v
COMMIT: ExoChain records the action -- immutable audit trail
```

This IS a conversation. Each impulse is a "message" from the physical world into the analytical system. The system "responds" with updated coherence scores, gap alerts, and suggested follow-up actions.

### 6.2 Investigator Queries as Graph Traversal

Investigators ask questions; the system translates them to graph operations:

| Natural Language Query | Graph Operation |
|---|---|
| "What do we know about suspect A?" | `traverse_forward(suspect_A, depth=3)` -- all nodes within 3 hops |
| "Does the timeline hold together?" | `spectral_analysis(50).lambda_2` on timeline subgraph |
| "What contradicts our primary theory?" | `get_edges_by_type(hypothesis_primary, Contradicts)` |
| "What evidence supports the alibi?" | `get_reverse_edges(alibi_node).filter(EvidenceFor)` |
| "Are there similar unsolved cases?" | `hnsw.search(case_embedding, 20).filter(unsolved)` |
| "What should we do next?" | Gap analysis engine -- highest coherence-delta action |
| "Show me the chain of custody for evidence #47" | `crossref_store.get_forward(evidence_47).filter(TriggeredBy)` + ExoChain audit trail |
| "Who is connected to who?" | `spectral_partition()` -- Fiedler vector reveals natural clusters |
| "What's missing?" | Full gap analysis sweep (Section 3) |

### 6.3 System-Generated Follow-Up Questions

After each DEMOCRITUS tick processes new evidence, the system generates suggested follow-up questions based on gap analysis:

```
ON tick_complete WHERE result.edges_added > 0:
    new_gaps = run_gap_analysis(case_subgraph)
    
    FOR each gap IN new_gaps.sorted_by(coherence_delta, descending):
        generate_question(gap):
            Pattern 1 (untested evidence): 
                "Evidence item {id} has never been tested. Similar evidence was decisive in {n} solved cases. Should we submit for analysis?"
            Pattern 2 (uninterviewed witness):
                "Witness {name} is mentioned in {document} but has never been interviewed. Interview could improve case coherence by {delta}."
            Pattern 3 (timeline gap):
                "Suspect's whereabouts are unknown between {start} and {end}, which overlaps the crime window. Cell tower records could fill this gap."
            Pattern 4 (MO match):
                "Case {case_id} from {jurisdiction} has {score} MO similarity and a convicted suspect. Has cross-jurisdiction comparison been done?"
```

### 6.4 The DEMOCRITUS Loop as Continuous Re-Evaluation

The DEMOCRITUS configuration from `democritus.rs` maps directly to investigation cadence:

```rust
DemocritusConfig {
    max_impulses_per_tick: 64,       // Process up to 64 new evidence items per cycle
    search_k: 5,                     // Find 5 nearest neighbors for each new item
    correlation_threshold: 0.7,      // Items above 0.7 similarity are "correlated"
    tick_budget_us: 15_000,          // 15ms budget per analysis cycle
}
```

For cold case investigation, the tick interval should be much longer (minutes to hours rather than milliseconds) since evidence arrives slowly. The `EccCalibrationConfig` can be tuned:

```rust
// Cold case investigation calibration
EccCalibrationConfig {
    calibration_ticks: 30,
    tick_interval_ms: 60_000,       // 1-minute ticks (vs 50ms for real-time)
    tick_budget_ratio: 0.8,         // 80% of tick for compute (not latency-sensitive)
    vector_dimensions: 384,         // Full embedding dimensionality
}
```

With an 80% budget ratio and 1-minute ticks, the system has 48 seconds per cycle for full spectral analysis, gap detection, and cross-case search -- ample for even large case graphs.

### 6.5 New Evidence Updates the Conversation

When a lab result comes back, a witness calls in, or a CODIS hit triggers:

1. **Impulse emitted**: `ImpulseQueue.emit(source_structure, source_node, target_structure, impulse_type, payload, hlc_timestamp)`
2. **SENSE phase** drains it on the next tick
3. **EMBED phase** vectorizes it alongside existing evidence
4. **SEARCH phase** finds what it correlates with -- this is where the "aha" moments happen. A DNA result from evidence item #47 suddenly links to a suspect from a different case via HNSW similarity.
5. **UPDATE phase** creates new causal edges, updates hypothesis coherence scores
6. **COMMIT phase** records everything in ExoChain -- the immutable conversation log

The `classify_edge` method from `democritus.rs` automatically determines the relationship type:

```rust
fn classify_edge(&self, impulse: &Impulse, score: f32) -> CausalEdgeType {
    if score >= self.config.correlation_threshold {
        return CausalEdgeType::Correlates;  // High similarity = statistical correlation
    }
    match &impulse.impulse_type {
        ImpulseType::BeliefUpdate => CausalEdgeType::Follows,      // Revised understanding
        ImpulseType::NoveltyDetected => CausalEdgeType::Follows,   // New evidence
        ImpulseType::EdgeConfirmed => CausalEdgeType::Causes,      // Confirmed link
        ImpulseType::CoherenceAlert => CausalEdgeType::EvidenceFor, // Gap detected
        ImpulseType::EmbeddingRefined => CausalEdgeType::Enables,  // Better understanding
        ImpulseType::Custom(_) => CausalEdgeType::Follows,         // Default
    }
}
```

### 6.6 Governance in the Conversation

Every investigative action passes through the three-branch governance model:

- **Legislative branch** enforces evidence handling SOPs. An `EffectVector` with `privacy: 0.9` (e.g., wiretap request) triggers higher scrutiny. A `severity: Blocking` rule prevents unauthorized access to sealed records.
- **Executive branch** controls resource allocation. Only authorized investigators can modify case graphs. Agent lifecycle management ensures proper case assignment.
- **Judicial branch** provides oversight. CGR validation ensures no evidence is suppressed. Brady disclosure rules are encoded as `Blocking` severity governance rules. Bias checks flag if investigation disproportionately focuses on suspects based on protected characteristics.

The `GovernanceDecision` outcomes map to investigation workflow:

| Decision | Investigation Meaning |
|---|---|
| `Permit` | Action authorized, proceed |
| `PermitWithWarning(String)` | Proceed but flag for supervisor review |
| `EscalateToHuman(String)` | Requires supervisor/prosecutor approval |
| `Deny(String)` | Action blocked -- warrant required, evidence sealed, etc. |

---

## 7. Implementation Priorities

### Phase 1: Foundation
1. Define custom `StructureTag::Custom(0x10)` for "Investigation" structure type
2. Implement node type schemas (Section 2.1) as metadata validation
3. Extend `CausalEdgeType` or use weighted combinations for investigation-specific edge types
4. Build case import pipeline: police reports -> impulses -> case graph

### Phase 2: Gap Analysis
1. Implement the 8 gap detection patterns (Section 3.2)
2. Build counterfactual lambda_2 computation
3. Create coherence dashboard (Section 3.3)
4. Integrate with DEMOCRITUS tick for continuous monitoring

### Phase 3: Cross-Case Intelligence
1. Build case embedding pipeline for HNSW corpus
2. Implement MO matching (Section 5.3)
3. Build solvability scoring model (Section 5.2)
4. Enable cross-jurisdiction case linking via CrossRefStore

### Phase 4: Conversation Interface
1. Natural language query -> graph operation translation
2. System-generated follow-up questions
3. Investigator-facing coherence dashboard
4. Timeline visualization with uncertainty bands

---

## 8. Key Technical References

| Component | File | Key API |
|---|---|---|
| Causal graph | `crates/clawft-kernel/src/causal.rs` | `add_node`, `link`, `find_path`, `spectral_analysis`, `spectral_partition`, `traverse_forward` |
| DEMOCRITUS loop | `crates/clawft-kernel/src/democritus.rs` | `tick()`, `sense()`, `embed()`, `search()`, `update()`, `classify_edge()` |
| HNSW vector search | `crates/clawft-kernel/src/hnsw_service.rs` | `insert`, `search`, `search_batch`, `save_to_file`, `load_from_file` |
| Cross-references | `crates/clawft-kernel/src/crossref.rs` | `CrossRefStore::insert`, `get_forward`, `get_reverse`, `by_type` |
| Impulse queue | `crates/clawft-kernel/src/impulse.rs` | `emit`, `drain_ready`, `pending_count` |
| Calibration | `crates/clawft-kernel/src/calibration.rs` | `run_calibration` (returns `EccCalibration` with `spectral_capable`, `tick_interval_ms`) |
| Governance | `crates/clawft-kernel/src/governance.rs` | `GovernanceRule`, `GovernanceRequest`, `EffectVector`, `GovernanceDecision` |
| ExoChain | `crates/clawft-kernel/src/chain.rs` | `chain_event_source`, `chain_event_kind`, `chain_event_payload` |
