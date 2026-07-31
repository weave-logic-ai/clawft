# WEFT-275 Decision: Lineage metadata placement + schema

**Date**: 2026-07-31  
**Status**: Accepted (sign-off)  
**Ticket**: WEFT-275  
**Branch**: `feat/weft-275-lineage`  
**Deciders**: developer (grounded in EXPLORER-MANAGEMENT-SURFACE §6 Q3 + PIPELINE-PRIMITIVE-JOURNAL `LineageMode`)  
**Form**: Ticket-local decision note (not a numbered ADR)

---

## Context

Affordance #14 (Explorer management surface): given a derived path such as

```
substrate/<daemon-node-id>/derived/transcript/<esp32-node-id>/mic
```

show the lineage graph (sources → actor → derived publish). That requires a
**placement convention** for lineage metadata and a typed Object Type + viewer.

Open question §6.3 offered two options:

| Option | Placement |
|--------|-----------|
| **Inline** | Derived-value payload carries a nested `lineage` field on every emission |
| **Sibling** | Lineage lives once at `<derived-path>/meta/lineage` |

`PIPELINE-PRIMITIVE-JOURNAL` already sketched `LineageMode::{Inline, Sibling, None}`
with Sibling matching the §6.3 proposal.

---

## Decision

**Primary: sibling path** — `LineageMode::Sibling`.

1. **Path convention**  
   Publishers of derived substrate values write lineage once at:

   ```
   <derived-path>/meta/lineage
   ```

   Example:

   ```
   substrate/n-daemon/derived/transcript/n-6f3a9c/mic/meta/lineage
   ```

2. **Document shape** (value *at* that path — flat, not double-wrapped):

   ```json
   {
     "kind": "lineage",
     "source_paths": [
       "substrate/n-6f3a9c/sensor/mic"
     ],
     "via_actor": "whisper",
     "target_path": "substrate/n-daemon/derived/transcript/n-6f3a9c/mic",
     "derivation": "transform",
     "ts": 1700000000000
   }
   ```

   | Field | Required | Notes |
   |-------|----------|-------|
   | `kind` | recommended | Discriminator `"lineage"`; strong match for Object Type |
   | `source_paths` | **required** | Non-empty array of substrate path strings |
   | `via_actor` | optional | Actor / pipeline stage id that produced the derivation |
   | `target_path` | optional | Derived path; if omitted, Explorer strips `/meta/lineage` from the selected path |
   | `derivation` | optional | Free-form tag (`transform`, `clone`, `snapshot`, …) |
   | `ts` | optional | Unix ms (u64) or ISO-8601 string |

3. **Why sibling (not default-inline)**  
   - Keeps high-cadence derived envelopes small (transcripts, PCM summaries).  
   - Matches existing `meta/label` pattern for node/sensor metadata.  
   - Lineage is relatively static per pipeline wiring — once per topic is enough.  
   - Explorer can open the sibling path without re-parsing every emission.

4. **Inline remains allowed as an opt-in** (`LineageMode::Inline`) for sinks that
   must stamp per-emission provenance on the payload itself under a nested
   `"lineage"` key. The **Lineage Object Type does not claim** a parent payload
   merely because it contains a nested `lineage` field (would steal transcript /
   sensor viewers). Inline documents only classify when the selected value
   *is* the lineage document (strong `kind` / top-level `source_paths`).

5. **Viewer**  
   `LineageViewer` (priority 13) paints identity chrome + builds a
   `ui://graph` shape and delegates to `GraphViewer` for the node-link diagram
   (sources → via_actor → target).

6. **Object Type**  
   `lineage` registered in the ontology cascade ahead of generic mesh/node
   envelopes; priority 13.

---

## Consequences

- Pipeline sinks default to `LineageMode::Sibling` when they opt into lineage.
- Explorer affordance #14 is unblocked for READS-ONLY.
- No substrate schema migration: convention only; publishers adopt as they land.
- Chain-level `LineageRecord` / `record_lineage` remains the cryptographic witness
  path; this ticket covers **substrate Explorer metadata**, not ExoChain records.

---

## Acceptance mapping

| AC | Status |
|----|--------|
| Decision memo on metadata placement | this document |
| Lineage Object Type in registry | `ontology/types/lineage.rs` |
| Lineage graph viewer | `explorer/viewers/lineage.rs` |
| Smoke test (derived path + lineage attached) | unit tests on type + viewer + `infer` |
