# ADR-042: Three Operating Modes (Act, Analyze, Generate)

**Date**: 2026-04-03
**Status**: Accepted
**Deciders**: ECC Symposium (D5), Architecture Review

## Context

The WeftOS cognitive engine (CMVG/ECC) processes conversations through five sub-engines: DCTE (contribution tree), DSTE (beliefs and goals), RSTE (discourse relations), EMOT (affect tracking), and SCEN (dramatic arc). These engines were originally designed for real-time conversation in the ClawStage platform.

The ECC Symposium (D5: "DEMOCRITUS as Nervous System Operation") established that the cognitive substrate is not a batch processor but the natural operation of a distributed nervous system. A key architectural insight emerged: the same engine that runs a live conversation can also analyze an existing corpus and can also generate new conversations toward a goal. These are not three different systems -- they are three operating modes of the same engine, sharing the same data structures, cognitive tick, witness chain (ExoChain), and scoring system.

This decision defines the product architecture: what WeftOS IS as a cognitive platform, distinct from other agent orchestrators.

## Decision

The cognitive engine operates in three composable modes sharing identical infrastructure (ExoChain, CausalGraph, HNSW index, CrossRef store, ImpulseQueue, scoring system):

**Mode 1: Act (Real-Time Conversation)**
Actors (human and AI) produce utterances in real-time. DCTE creates nodes and advances the wavefront. DSTE updates beliefs and goals. RSTE scores coherence. EMOT tracks affect. SCEN manages dramatic arc. The tree of possibilities collapses as speculative branches are pruned, merged, or committed. Input: live utterances. Output: the conversation itself with full causal history.

**Mode 2: Analyze (Post-Hoc Understanding)**
Given an existing corpus (transcript, PR history, research paper, sprint commits), run the engines in read-only mode. DCTE reconstructs the contribution tree. DSTE infers participant goals and beliefs. RSTE maps discourse coherence and identifies gaps. EMOT tracks affect trajectory. SCEN identifies lifecycle position. Input: existing corpus modeled as a conversation. Output: structural analysis with goal completion status, coherence scores, causal decision graph, affect trajectory, quality metrics.

**Mode 3: Generate (Goal-Directed Conversation)**
Set a DSTE goal, spawn expert agent-processes as WeftOS kernel processes (architect, security-reviewer, performance-eng, domain-expert, devil-advocate), and let them converse toward the goal. SCEN structures the discussion into phases. The floor manager ensures each expert contributes. RSTE enforces coherence. Speculative branches are evaluated and pruned. The committed MainLine IS the output artifact, not a summary of a process. Input: goal definition + agent configurations. Output: the goal artifact with full causal provenance.

The three modes compose into a continuous loop linked by ExoChain provenance: Generate produces a plan, Analyze evaluates it for coherence and gaps, Act executes it with human participants, and the cycle repeats. Each transition is a causal edge in the CMVG. The Generate mode is the foundation for the assessment product (ADR-023).

## Consequences

### Positive
- One engine, three products: real-time collaboration, corpus analysis, and autonomous generation all reuse the same infrastructure
- Every scored witness entry is automatically a training sample for SONA (LoRA fine-tuning) -- no separate data pipeline needed
- The causal graph provides full decision traceability: which generation-time decision led to which execution-time problem
- Speculative branches (explored but pruned) are preserved as alternatives-considered documentation
- Analyze mode applied to Generate output provides automatic quality assurance before human review

### Negative
- The shared-engine assumption constrains how each mode can diverge; optimizations for one mode may degrade another
- Generate mode requires the full kernel process model (agent spawning, floor management, SCEN phases), making it the heaviest mode to implement
- Analyze mode's read-only constraint must be enforced at the engine level to prevent accidental mutation of the analyzed corpus
- Marketing three modes as one product requires careful positioning to avoid confusion with simpler agent orchestrators

### Neutral
- Training material (scored trajectories, causal decision matrices, coherence metrics) falls out naturally from the conversation structure -- it is not produced as a separate step
- The ExoChain witness chain ensures all cross-mode transitions are tamper-evident and verifiable via Merkle proofs
- The mode taxonomy (Act/Analyze/Generate) maps to the assessment product roadmap: Analyze is the immediate revenue opportunity, Generate powers automated assessment creation
