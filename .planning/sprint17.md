# Sprint 17 — Security Hardening + Ontology Graph Pipeline

**Started:** 2026-04-16
**Theme:** MAESTRO-informed security hardening, prompt injection defense pipeline, WebVOWL-inspired ontology visualization

---

## Context

Security audit against CSA MAESTRO 7-layer model (inspired by
`mondweep/agentic-ai-security-demo-rela8group-ciso-london-summit`)
revealed strong sandboxing and agent isolation but critical gaps in
RPC auth, prompt injection defense, and plugin supply chain. WebVOWL
(`VisualDataWeb/WebVOWL`) identified as reference for ontology-grade
graph visualization to complement graphify.

---

## P0 — Prompt Injection Defense Pipeline

The codebase has `sanitize_skill_instructions()` (strips injection
tokens) but it is only applied to skill loading. Seven other LLM
input paths are unprotected. We cannot fully solve prompt injection,
but we can build a pipeline where defenses plug in at every boundary.

### PI-1: Unified sanitization at LLM call boundary
- [ ] Create `sanitize_llm_input(content, source_label) -> String` in
      `clawft-core/src/security/mod.rs` that wraps
      `sanitize_skill_instructions()` + logs when suspicious patterns
      detected, tagged with the source boundary name.
- [ ] Apply at all 7 injection points:
  - `semantic_extract.rs:116` — file content → extraction prompt
  - `context.rs:467` — MEMORY.md read → system message
  - `context.rs:492` — session history replay → messages
  - `loop_core.rs` — tool results → message stream
  - `context.rs:228-275` — bootstrap files (SOUL.md etc) → system prompt
  - `vision_extract.rs` — image extraction prompt
  - `conversation.rs` — conversation-based extraction

### PI-2: Canary token detection
- [ ] Add canary/sentinel tokens at system prompt boundaries so the
      LLM can detect when injected content tries to escape its context
      window. Pattern: `<<BOUNDARY:user_content>>...<<END:user_content>>`
- [ ] Instrument in `context.rs` system prompt assembly.

### PI-3: Output filtering for secrets
- [ ] Before returning LLM output to the user or writing to memory,
      scan for secret patterns (API keys, tokens, credentials) using
      the existing `clawft-security` pattern matcher.
- [ ] Log + redact matches. Fail-open (don't block response) but
      emit audit event.

### PI-4: Memory content validation on write
- [ ] Before storing content to MEMORY.md or long-term memory, run
      `sanitize_llm_input()` to prevent persistent injection via
      RAG poisoning (the attack pattern from the MAESTRO analysis).

### PI-5: Prompt injection detection scoring (future)
- [ ] Design a pluggable `PromptInjectionDetector` trait in
      `clawft-core/src/security/` that scores input text for
      injection likelihood (0.0–1.0).
- [ ] Initial implementation: regex-based heuristics (role tags,
      instruction override phrases, encoding tricks).
- [ ] Future: EML-based classifier trained on injection datasets.
- [ ] Hook into the PI-1 sanitization pipeline as an optional gate.

---

## P0 — RPC Authentication

### RPC-1: Bearer token auth on Unix socket
- [ ] Generate a kernel secret at boot (random 32-byte token), write
      to `$RUNTIME_DIR/weft/kernel.token` with 0600 permissions.
- [ ] `clawft-rpc` client sends token in `Authorization` field of
      every `Request`.
- [ ] Daemon validates token before dispatching. Reject with
      `UNAUTHENTICATED` error on mismatch.
- [ ] **Fail-closed**: no token = no access.

### RPC-2: Method allowlist + parameter schema validation
- [ ] Define an `RpcSchema` registry in the daemon mapping method
      names to expected parameter shapes.
- [ ] Reject unknown methods. Validate param types before dispatch.
- [ ] Add rate limiting: 100 req/s default, configurable.

---

## P1 — Plugin Supply Chain

### SC-1: Plugin manifest signing
- [ ] Add `signature: Option<String>` field to `PluginManifest`.
- [ ] At build time: sign manifest JSON with Ed25519 key.
- [ ] At load time: verify signature before proceeding. Unsigned
      plugins rejected by default (configurable `allow_unsigned`).

### SC-2: WASM module integrity
- [ ] Add `wasm_sha256: Option<String>` to manifest.
- [ ] Compute SHA-256 of WASM binary at load, compare to manifest.
- [ ] Reject on mismatch.

---

## P1 — Ontology Graph Pipeline (WebVOWL-inspired)

WebVOWL implements the VOWL visual notation for OWL ontologies using
a split data model (type declarations + rich attribute objects joined
by ID), D3 force-directed layout, and SVG rendering. Key patterns to
adopt in graphify:

### OG-1: VOWL JSON export format
- [ ] Add `export::vowl` module to `clawft-graphify` that emits
      WebVOWL-compatible JSON from a `KnowledgeGraph`.
- [ ] Map graphify entity types → VOWL class types (Module → OwlClass,
      Function → OwlClass with stereotype, etc).
- [ ] Map relationship types → VOWL property types (Contains →
      SubClassOf, Calls → ObjectProperty, etc).
- [ ] Emit the 8-key VOWL JSON structure: header, namespace, metrics,
      class, classAttribute, property, propertyAttribute.
- [ ] This lets existing WebVOWL instances render our graphs directly.

### OG-2: OWL/RDF ingestion
- [ ] Add an OWL/RDF parser to graphify's extract pipeline (Turtle
      and JSON-LD at minimum) so graphify can ingest ontologies
      natively, not just source code.
- [ ] Consider the `oxigraph` or `sophia` Rust crates for RDF parsing.
- [ ] Map RDF triples → graphify entities + relationships.

### OG-3: Force-directed layout engine (Rust-native)
- [ ] Implement a basic force-directed layout in Rust for server-side
      graph positioning (no browser dependency).
- [ ] Output positioned SVG or positioned JSON for client rendering.
- [ ] Start simple: repulsion + attraction + centering forces.
- [ ] This replaces the D3 dependency for headless/CLI use cases.

### OG-4: VOWL visual encoding rules
- [ ] If we do SVG export, encode VOWL visual rules: blue circles
      for classes, yellow rects for datatypes, dash patterns for
      deprecated, etc.
- [ ] Can layer on top of existing `export::html` or as standalone.

---

## P2 — Secondary Security Items

### SEC-1: Namespace isolation for shared services
- [ ] Memory backend: scope reads/writes by agent_id.
- [ ] Cron service: scope job visibility by agent_id.
- [ ] IPC topics: add optional namespace prefix.

### SEC-2: Log secret redaction
- [ ] Add `redact_secrets()` pass to kernel log output path.
- [ ] Use `clawft-security` pattern matcher for detection.

### SEC-3: Audit trail for cross-agent operations
- [ ] Log when agent A accesses resources owned by agent B.
- [ ] Emit structured audit events to the ring buffer.

---

## Completed (this sprint)

- [x] v0.6.11 — `weaver vault` command (frontmatter, wikilinks, link
      graph analysis, suggestions, auto-linking, backlinks)
- [x] CI fix: `eml-core` added to publish pipeline
- [x] CI fix: Docker workflow race eliminated (workflow_run trigger)
- [x] CI fix: Release gate marks failed releases as prerelease
- [x] Security audit against MAESTRO 7-layer model
- [x] Prompt injection attack surface mapped (7 unprotected LLM input paths)
- [x] WebVOWL architecture research for ontology graph pipeline

---

## References

- CSA MAESTRO 7-layer model: `mondweep/agentic-ai-security-demo-rela8group-ciso-london-summit`
- WebVOWL: `VisualDataWeb/WebVOWL` — VOWL JSON format, D3 force layout, SVG rendering
- VOWL specification: http://vowl.visualdataweb.org/v2/
- Existing security code: `clawft-core/src/security/mod.rs`, `clawft-security/src/checks/patterns.rs`
- Existing graph exports: `clawft-graphify/src/export/` (json, graphml, html, obsidian, wiki)
