# Symposium: Cold Case Investigation via Embodied Causal Cognition

**Working Title**: "Computational Causal Analysis for Cold Case Homicide Investigation"
**Status**: Planning
**Target Date**: TBD
**Location**: Virtual (initial), potential in-person follow-up with OPD

---

## Executive Summary

Explore adapting WeftOS's Embodied Causal Cognition (ECC) engine — originally built for software system analysis — to cold case homicide investigation. The ECC's causal graph, gap analysis, coherence scoring, and continuous re-evaluation loop map directly to the investigative process of building a case from fragmented evidence.

**Key contact**: Retired homicide detective, former professor at Northwestern University. Will connect us with Orlando PD cold case unit and relevant academic experts.

---

## Symposium Agenda

### Session 1: Problem Space (45 min)
**Lead: Retired Detective (Northwestern)**

- Current state of cold case investigation at OPD
- Why cases go cold: witness attrition, evidence backlogs, institutional knowledge loss
- The "solvability factors" framework — how detectives decide where to spend time
- What technology is currently used (and what's missing)
- What a "solved" case looks like vs what keeps cases stuck

**Goal**: Ground the technical team in reality. No solutioning — just listen.

### Session 2: The ECC Engine (30 min)
**Lead: WeaveLogic Engineering**

- Demo of WeftOS ECC on a codebase analysis (what it does today)
- The causal graph: nodes, edges, community detection, coherence scoring
- DEMOCRITUS loop: continuous Sense → Embed → Search → Update → CrossRef → Prune → Report
- Gap analysis: how the system identifies what's MISSING, not just what's there
- HNSW semantic search: finding similar patterns across large datasets
- ExoChain: tamper-evident audit trail for every analytical step

**Goal**: Show the detective what the engine does without jargon. Use visual demos.

### Session 3: Mapping ECC to Investigation (60 min)
**Lead: Joint — Engineering + Detective + Academic**

- Evidence as nodes, relationships as edges
- The "Case Graph" model: persons, events, evidence, locations, timelines, hypotheses
- Gap analysis applied: "What questions haven't been asked?"
  - Untested evidence (DNA, fingerprints, ballistics)
  - Uninterviewed witnesses
  - Unverified alibis
  - Unpulled records (cell, surveillance, financial)
- Coherence score as case completeness metric
- Crime scene reconstruction via competing timeline models
- "What if" analysis: test hypotheses against the evidence graph

**Goal**: Validate that the mapping makes sense to an experienced investigator.

### Session 4: Data Sources & Ingestion (45 min)
**Lead: Engineering + Law Enforcement Data Expert**

- What data exists in a typical cold case file
- Police RMS systems, CAD records, evidence management systems
- Digitization challenges (handwritten notes, old reports, microfiche)
- Chain of custody requirements for digital evidence
- Integration with existing systems (VICAP, NamUs, CODIS, AFIS, NIBIN)
- Cell tower data, social media archives, surveillance footage logs
- Privacy constraints, CJIS compliance, Florida public records law

**Goal**: Understand what data we can actually access and how to ingest it.

### Session 5: Incoming Case Classification (30 min)
**Lead: Engineering + Detective**

- Triage system for new homicide cases
- Solvability scoring based on initial evidence
- MO matching via HNSW similarity search against historical cases
- Resource allocation recommendation
- Time-sensitive lead detection (evidence that degrades)
- Serial pattern detection across cases

**Goal**: Explore the "front door" use case — not just cold cases but live case support.

### Session 6: Legal, Ethical & Admissibility (30 min)
**Lead: Legal Expert / Academic**

- Admissibility of AI-assisted analysis in Florida courts
- Brady obligations — system must surface exculpatory evidence too
- Bias mitigation — system must not reinforce existing investigative biases
- Chain of custody for digitized and computationally analyzed evidence
- Privacy constraints on data aggregation
- Precedent: genetic genealogy admissibility, Palantir use in law enforcement
- ExoChain as audit trail — every analytical step is logged and verifiable

**Goal**: Identify legal guardrails before building anything.

### Session 7: Prototype Scope & Next Steps (30 min)
**Lead: All**

- Define MVP: one cold case, manually entered data, graph visualization + gap analysis
- Timeline to working prototype
- What OPD would need to see to engage formally
- Academic partnership opportunities (Northwestern, UCF)
- Funding: NIJ (National Institute of Justice) grants, BJA Smart Policing
- Publication opportunity: academic paper on computational cold case analysis

**Goal**: Leave with a concrete plan and assigned owners.

---

## Expert Panel

### Core Team

| Role | Who | Notes |
|------|-----|-------|
| **Homicide Investigation** | Retired Detective (Northwestern Prof) | Primary advisor. Connects us to OPD. Decades of homicide investigation + academic research |
| **WeftOS/ECC Engineering** | WeaveLogic team | ECC engine, causal graphs, HNSW, spectral analysis, gap analysis |
| **Product/Strategy** | WeaveLogic team | Application mapping, market analysis, prototype scoping |

### Invited Experts (To Be Recruited)

| Domain | Role Needed | Where to Find | Why |
|--------|-------------|---------------|-----|
| **Cold Case Investigation** | Active OPD cold case detective or supervisor | Via retired detective contact | Ground truth on current process, data access, pain points |
| **Forensic Science** | DNA/forensic lab director or supervisor | UCF National Center for Forensic Science, FDLE | Evidence backlog reality, what retesting is possible |
| **Criminal Intelligence Analysis** | Crime analyst (sworn or civilian) | OPD Real Time Crime Center, FDLE | Current tools (i2, Palantir), data systems, what works/doesn't |
| **Prosecutorial** | ADA or former prosecutor (homicide) | State Attorney's Office, 9th Judicial Circuit | Admissibility requirements, what makes a case prosecutable |
| **Defense/Civil Liberties** | Public defender or innocence project attorney | Innocence Project of Florida, ACLU FL | Ensure system surfaces exculpatory evidence, bias concerns |
| **Academic — Computational Criminology** | Researcher in comp crim or criminal justice analytics | Northwestern, UCF, Michigan State (School of Criminal Justice) | Academic rigor, research methodology, publication pathway |
| **Academic — Causal Inference** | Researcher in causal inference / Bayesian networks | MIT, Stanford, Columbia (Causal AI Lab) | Validate the causal graph methodology for evidence analysis |
| **Law Enforcement Technology** | Vendor or consultant in LE tech integration | CJIS compliance expert, RMS vendor | Integration reality, data format standards, security requirements |
| **Victims' Advocacy** | Victims' rights advocate or cold case family liaison | National Center for Victims of Crime, local advocacy groups | Ethical grounding, impact perspective, community trust |
| **GIS/Spatial Analysis** | Crime mapping / geographic profiling expert | Police Foundation, RTM researchers | Spatial components of the case graph |

### Academic Institutions to Engage

| Institution | Department | Relevance |
|-------------|-----------|-----------|
| **Northwestern University** | School of Law — Center on Wrongful Convictions; Pritzker School of Law | Our detective connection. Wrongful conviction research validates the "exculpatory evidence" requirement |
| **UCF** | National Center for Forensic Science; Criminal Justice Dept | Local to Orlando, forensic science expertise, potential student researchers |
| **Michigan State** | School of Criminal Justice | Leading computational criminology program |
| **University of Maryland** | Institute for Advanced Computer Studies (UMIACS) | AI for criminal justice research |
| **Carnegie Mellon** | Language Technologies Institute | NLP for report/statement analysis |

---

## Key Deliverables

### Pre-Symposium
- [ ] Research document: current cold case methods + technology landscape
- [ ] ECC-to-investigation mapping document (technical)
- [ ] Prototype data schema (Case Graph model)
- [ ] 5-minute ECC demo video (codebase analysis → "imagine this is evidence")

### Symposium Output
- [ ] Validated Case Graph schema (reviewed by detective)
- [ ] Prioritized gap analysis features
- [ ] Legal guardrails document
- [ ] MVP prototype scope definition
- [ ] Partnership structure (academic + law enforcement)

### Post-Symposium
- [ ] MVP prototype: one case, manual entry, graph + gap analysis
- [ ] Grant application (NIJ or BJA) draft
- [ ] Academic paper outline
- [ ] OPD engagement proposal

---

## Reference Documents

| Document | Location |
|----------|----------|
| Research foundations | `research-foundations.md` |
| ECC application mapping | `ecc-application-mapping.md` |
| WeftOS ECC documentation | `docs/src/content/docs/weftos/ecc.mdx` |
| Governance model | `docs/src/content/docs/weftos/governance.mdx` |
| Causal graph implementation | `crates/clawft-kernel/src/causal.rs` |
| DEMOCRITUS loop | `crates/clawft-kernel/src/democritus.rs` |
| HNSW search | `crates/clawft-kernel/src/hnsw_service.rs` |
| Cross-reference engine | `crates/clawft-kernel/src/crossref.rs` |
