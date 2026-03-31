# ADR-018: Hermes Models as clawft-llm Provider (Not Framework Adoption)

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Hermes Integration Analysis

## Context

NousResearch's Hermes is three things: open-weight LLMs, an agent framework (Python, 18.5K stars), and a function-calling protocol. clawft already implements a direct Rust parallel to the Hermes agent framework with 17,795 lines of agent + pipeline code and feature parity or superiority in most areas (7-stage pipeline, tiered routing, skill hot-reload, WASM support, compiled performance). Adopting the Hermes framework would be redundant. However, Hermes open-weight LLMs are valuable for air-gapped deployments and cloud-independent operation.

## Decision

Add Hermes models as an `clawft-llm` provider via OpenAI-compatible endpoint support, enabling locally-hosted Hermes LLMs for air-gapped deployments. Do not adopt the Hermes agent framework -- clawft IS the agent framework. Hermes agents can optionally run on WeftOS as Python processes alongside clawft agents, tracked via the kernel's PID system and governed by the governance engine.

## Consequences

### Positive
- Cloud-independent LLM capability for air-gapped and on-premise deployments
- "Hermes runs on WeftOS" positions WeftOS as infrastructure that makes agents enterprise-ready
- No framework duplication -- clawft retains its pipeline, skills, and governance advantages
- OpenAI-compatible endpoint means any local LLM server (vLLM, llama.cpp) works

### Negative
- Open-weight models have lower capability than frontier cloud models for complex tasks
- Local model hosting requires GPU infrastructure
- Supporting heterogeneous agents (Rust + Python) adds operational complexity

### Neutral
- clawft's advantages over Hermes: 7-stage pipeline, ExoChain provenance, governance engine, WASM sandboxing, compiled performance, governed prompt evolution
- Hermes's advantages: context compression, GEPA prompt evolution, user modeling (gaps clawft should close per ADR-017)
