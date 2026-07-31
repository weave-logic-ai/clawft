# WEFT-621 — FSL licensing decision: AgentBBS / late.sh source reuse

**Status**: Accepted  
**Ticket**: WEFT-621  
**Date**: 2026-07-31  
**Decision**: **NO-GO** on source reuse (copy, vendor, or link) of
AgentBBS / late.sh code into WeftOS / clawft  
**Allowed**: **Patterns-only / clean-room** (already the ADR-063..066 baseline)  
**Form**: Legal/governance decision note (not a numbered architecture ADR)

---

## 1. Question

May WeftOS (MIT-licensed) copy, vendor, or link **source code** from:

| Project | Repo | Relationship |
|---------|------|--------------|
| **late.sh** | [mpiorowski/late-sh](https://github.com/mpiorowski/late-sh) | Upstream SSH/TUI social platform |
| **AgentBBS** | [ruvnet/AgentBBS](https://github.com/ruvnet/AgentBBS) | Fork of late-sh + agent/community layer |

…into the WeftOS tree, or ship a binary that depends on those crates as
libraries, while remaining redistributable under WeftOS’s MIT license?

---

## 2. License facts (sources of interest)

### 2.1 License text on disk

Both projects ship **Functional Source License, Version 1.1, MIT Future License**
(**FSL-1.1-MIT**), as verified from the repositories’ `LICENSE` files
(2026-07-31 fetch):

- Abbreviation: `FSL-1.1-MIT`
- Copyright notice on the retrieved LICENSE: © 2026 Mateusz Piórowski
  (AgentBBS inherits late.sh licensing; AgentBBS is a GitHub fork of late-sh)
- AgentBBS README also states source-available FSL inherited from late.sh
  (badge text has mentioned Apache-2.0 future in marketing; **LICENSE file
  is authoritative** and currently grants **MIT** as the future license)

late.sh additionally documents intent in `LICENSING.md` (plain-English
policy; not a substitute for `LICENSE`).

### 2.2 What FSL-1.1-MIT grants *during* the protection period

Summarized from FSL-1.1 (not legal advice; see §8):

| Topic | Term |
|-------|------|
| **Grant** | Use, copy, modify, create derivatives, display, redistribute — **only for a Permitted Purpose** |
| **Permitted Purpose** | Any purpose **other than a Competing Use** |
| **Competing Use** | Making the Software available to others in a **commercial product or service** that (1) substitutes for the Software, (2) substitutes for any other product/service the licensor offers using the Software as of the make-available date, or (3) offers the **same or substantially similar functionality** |
| **Explicitly listed permitted examples** | Internal use; non-commercial education; non-commercial research; professional services to a licensee using the Software under the terms |
| **Redistribution** | FSL terms **apply to all copies, modifications, and derivatives**; redistributors must include the FSL terms and preserve copyright notices |
| **Change date** | On the **second anniversary** of the date that version was made available, an **irrevocable additional MIT license** becomes effective for that version |
| **Trademarks** | No grant of `late.sh` / project branding rights |
| **OSI status** | Source-available; **not** OSI Open Source during the FSL period |

### 2.3 WeftOS license posture

WeftOS / clawft workspace root `LICENSE` is **MIT** (Copyright 2025 The clawft
contributors). Downstream consumers and distributors expect MIT-compatible
redistribution without a competing-use restriction.

---

## 3. Decision

### **NO-GO — do not reuse AgentBBS or late.sh source in WeftOS**

Effective immediately for all cycles:

1. **Do not copy** AgentBBS or late.sh source into this repository
   (including “small” utilities, bridge helpers, TUI fragments, federation
   crates, or “temporary” vendored trees).
2. **Do not add** Cargo path/git dependencies on AgentBBS / late.sh crates
   that would make FSL code part of WeftOS build or redistributable artifacts.
3. **Do not relicense** or dual-license WeftOS modules under FSL to
   accommodate such reuse without an explicit project-owner written decision
   (out of scope of this ticket; default remains MIT).

### **GO — patterns-only / clean-room (already shipped)**

1. **Reading** AgentBBS / late.sh source, ADRs, and docs for **design ideas**
   is allowed and encouraged when relevant.
2. **Reimplementing** protocols and architecture in original WeftOS code under
   MIT is allowed (the path used by ADR-063..066).
3. **Citing** AgentBBS ADR numbers and pattern names in WeftOS ADRs is
   allowed with an FSL / no-code-copied notice (already the house style).

### **Conditional later paths (not authorized today)**

| Path | When it could become GO | Required action |
|------|-------------------------|-----------------|
| Use of a **specific version** under MIT after FSL change date | ≥ 2 years after that version’s make-available date | Re-verify LICENSE + change date per version; new governance note; still prefer clean-room unless there is a strong product need |
| Explicit dual-license or commercial grant from licensor | Written permission covering WeftOS commercial/product use and MIT redistribution | File new Plane item + attach grant; do not assume |
| Separate FSL-only product / sidecar outside WeftOS MIT tree | Product/legal decision to ship a **separate** artifact under FSL | Isolate completely from MIT crates; do not mix into workspace default license |

None of these conditional paths are approved by this decision.

---

## 4. Rationale

1. **License incompatibility with product intent**  
   WeftOS is intended to ship under MIT. FSL redistribution requires keeping
   FSL terms on derivatives. Mixing FSL source into an MIT monorepo forces
   either (a) dual-license contamination of the tree, or (b) inaccurate MIT
   claims to downstream users.

2. **Competing Use risk is real, not theoretical**  
   AgentBBS / late.sh center on multiplayer agent+human community boards,
   SSH/MCP access, federation, bridges, and hosted-style community
   services. WeftOS substrate/channels/mesh already implement adjacent
   surfaces (signed envelopes, anti-entropy, channel bridges, capability
   tokens — ADR-063..066). Shipping commercial WeftOS features that are
   “substantially similar” while containing FSL-derived code is exactly the
   risk FSL is designed to control. Clean-room avoids that analysis for
   every release.

3. **Patterns-only already delivers the needed architecture**  
   WEFT-618 / ADR-063..066 shipped the substrate/channels design informed by
   AgentBBS ADRs with **zero** code copied. That boundary is proven workable.

4. **Operational simplicity**  
   A hard no-go is cheaper than per-file provenance audits, NOTICE sprawl,
   and “is this fork still FSL?” drift tracking across AgentBBS and late.sh.

---

## 5. Clean-room boundary (normative for contributors)

### 5.1 Allowed

- Read public ADRs, READMEs, threat models, and API *descriptions*.
- Write original Rust/TS that implements similar *interfaces* or *protocols*.
- Name the inspiration in ADR “References” with FSL disclaimer.
- Use independent third-party crates that are MIT/Apache-2.0/etc. even if
  AgentBBS also uses them (e.g. common crypto crates) — dependency on a
  shared **OSI** crate is not FSL reuse.

### 5.2 Forbidden without a superseding decision

- Copy-paste or mechanical translation of AgentBBS / late.sh source.
- Vendoring those repos under `vendor/`, `third_party/`, or git submodules
  for compile-time use.
- `Cargo.toml` git/path deps on `agentbbs-*` / `late-*` packages from those
  trees.
- “Temporary” reuse with a plan to rewrite later.
- Shipping WeftOS binaries that statically/dynamically link FSL-protected
  AgentBBS/late.sh libraries.

### 5.3 Evidence baseline (already in tree)

| Artifact | Boundary statement |
|----------|--------------------|
| ADR-063 | AgentBBS FSL-1.1-MIT; patterns only; no code copied |
| ADR-064 | Same |
| ADR-065 | Same |
| ADR-066 | Same |
| WEFT-618 | Done provenance: patterns-only substrate/channels ADR set |

Any PR that introduces AgentBBS/late.sh source **fails this decision** and
must be rejected or reworked to clean-room.

### 5.4 Review checklist for PRs that touch “AgentBBS-like” surfaces

1. Diff contains no files sourced from those repos (spot-check unusual
   identical chunks if reviewer suspects leakage).
2. New ADRs that cite AgentBBS include the FSL / no-code-copied notice.
3. No new Cargo deps pointing at AgentBBS/late.sh.

---

## 6. Impact on related work

| Item | Impact |
|------|--------|
| **ADR-063..066** | Affirmed — patterns-only remains the correct path |
| **WEFT-618** | Affirmed Done; no reopening for code reuse |
| **K6 / exo-core / exo-dag vendoring** | **Not automatically blocked** by this decision. Re-check **those** crates’ licenses separately. This NO-GO applies only to AgentBBS / late.sh FSL source. If a candidate dependency is itself FSL or pulls AgentBBS/late.sh, treat as blocked until a dedicated clearance note exists. |
| **Future AgentBBS feature ports** | Implement as clean-room under MIT, or do not ship |

---

## 7. Acceptance criteria mapping (WEFT-621)

| Criterion | Outcome |
|-----------|---------|
| Determine FSL terms for AgentBBS/late.sh sources of interest | **Done** — §2 (FSL-1.1-MIT; Competing Use; 2-year MIT flip; redistribution of derivatives under FSL) |
| Written go/no-go on source reuse vs patterns-only | **Done** — §3 **NO-GO** source reuse; **GO** patterns-only |
| If no-go: document clean-room boundary used in ADR-063..066 | **Done** — §5 |

---

## 8. Disclaimer

This note is **project governance**, not formal legal advice. It records how
WeftOS maintainers choose to treat FSL-licensed AgentBBS / late.sh material
relative to the MIT tree. Product counsel may tighten or (with written grant)
relax the rule; until then, **NO-GO** is binding for contributors and agents.

---

## 9. References

- FSL overview: <https://fsl.software/>
- late.sh LICENSE / LICENSING.md (upstream)
- AgentBBS README License section + LICENSE (fork of late-sh)
- WeftOS `LICENSE` (MIT)
- `docs/adr/adr-063-substrate-signed-envelope-a2a.md`
- `docs/adr/adr-064-substrate-anti-entropy-reconciliation.md`
- `docs/adr/adr-065-channel-bridge-subkey-identity-loop-guard.md`
- `docs/adr/adr-066-capability-tokens-human-join-governance.md`
- Plane: WEFT-621, WEFT-618
