# ADR-017: GEPA Prompt Evolution for pipeline/learner.rs

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Hermes Integration Analysis + Sprint 11 Symposium

## Context

The clawft pipeline includes a 6-stage processing chain: Classifier -> Router -> Assembler -> Transport -> Scorer -> Learner. The Learner stage is currently a 139-line no-op stub (`NoopLearner`) that silently discards all trajectory records. The Scorer stage (154 lines) produces basic quality scores but has no feedback loop. Hermes's GEPA (Genetic Evolution of Prompt Architectures) system demonstrates that genetic evolution of skill prompts using quality scores as fitness functions creates a self-improvement flywheel.

## Decision

Expand `pipeline/learner.rs` from the current no-op stub into a genetic prompt optimizer. Use the existing `pipeline/scorer.rs` quality scores as the fitness function. Track prompt lineage through the ECC causal graph. Gate deployment of evolved prompts through the governance engine. Adopt the GEPA pattern from Hermes but with WeftOS's governance and provenance layers as safety constraints.

## Consequences

### Positive
- Self-improvement flywheel: agents get better over time without manual prompt tuning
- ECC tracks prompt lineage -- every prompt mutation is causally traced
- Governance gates prevent deploying low-quality or unsafe evolved prompts
- Hermes has no equivalent safety layer on prompt mutations -- this is a differentiator

### Negative
- Significant implementation effort to expand the 139-line stub
- Genetic algorithms require careful tuning of mutation rates and population sizes
- Risk of prompt drift if fitness function is poorly calibrated

### Neutral
- The Scorer's current ~30-50% false positive rate on confidence scores needs calibration before GEPA can use it effectively
- This is a v0.2+ feature; v0.1 ships with the no-op learner
