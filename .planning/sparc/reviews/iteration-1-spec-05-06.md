# SPARC Specification Review: Elements 05 and 06 (Iteration 1)

> **Reviewer**: Specification Review Agent
> **Date**: 2026-02-19
> **Scope**: SPARC Elements 05 (Pipeline & LLM Reliability) and 06 (Channel Enhancements)
> **Source of Truth**: `.planning/improvements.md`
> **Cross-References**:
>   - `.planning/sparc/05-pipeline-reliability/00-orchestrator.md`
>   - `.planning/sparc/06-channel-enhancements/00-orchestrator.md`
>   - `.planning/drafts/01-biz-architecture.md` (Workstream D biz requirements)
>   - `.planning/drafts/01-biz-plugins-channels.md` (Workstream E biz requirements)
>   - `.planning/drafts/02-tech-pipeline.md` (Workstreams D, E tech specs)
>   - `.planning/drafts/review-1-biz-tech.md` (Prior biz/tech review)
>   - `.planning/drafts/review-1-devguide.md` (Prior dev guide review)

---

## 1. Element 05: Pipeline & LLM Reliability

### 1.1 Coverage Assessment

Checking every item from `improvements.md` Workstream D against the orchestrator:

| Item | Description | In Orchestrator? | Phase | Notes |
|------|-------------|:-:|-------|-------|
| D1 | Parallel tool execution (`futures::join_all`) | YES | D-Perf | Correctly placed |
| D2 | Streaming failover correctness (reset stream) | YES | D-Reliability | Correctly placed |
| D3 | Structured error variants for retry | YES | D-Reliability | Correctly placed |
| D4 | Configurable retry policy | YES | D-Reliability | Correctly placed |
| D5 | Record actual latency in ResponseOutcome | YES | D-Observability | Correctly placed |
| D6 | Thread sender_id for cost recording | YES | D-Observability | Correctly placed |
| D7 | StreamCallback to FnMut | YES | D-Reliability | Correctly placed |
| D8 | Bounded message bus channels | YES | D-Reliability | Correctly placed |
| D9 | MCP transport concurrency | YES | D-Observability | **Misplaced** -- this is a performance/throughput item, not an observability item |
| D10 | Cache skill/agent bootstrap files | YES | D-Perf | Correctly placed |
| D11 | Async file I/O in skills loader | YES | D-Perf | Correctly placed |

**Coverage**: 11/11 items present. Complete.

### 1.2 Exit Criteria Evaluation

The orchestrator defines 7 exit criteria for 11 items. Assessment:

| Exit Criterion | Testable? | Covers Items | Issues |
|----------------|:-:|:-:|--------|
| "Multiple tool calls execute concurrently (verify with timing test)" | YES | D1 | Good -- matches biz criteria "3 tools with 100ms simulated latency complete in < 200ms" but the orchestrator lacks this specific threshold |
| "Streaming failover produces clean output (no partial concatenation)" | YES | D2 | Acceptable but less precise than biz draft's "first provider fails mid-stream, second provider's complete output is delivered cleanly" |
| "Retry policy configurable via config.json" | PARTIAL | D4 | Missing validation of D3 -- no criterion verifies that string-prefix matching is eliminated. "Configurable via config.json" does not test the structured error variant change |
| "latency_ms populated in all ResponseOutcome records" | YES | D5 | Good -- could add "no hardcoded zeros" for emphasis |
| "Message bus has configurable buffer size with backpressure" | YES | D8 | Good |
| "sender_id available in CostTracker for per-user tracking" | YES | D6 | Good |
| "All existing tests pass" | YES | All | Standard regression gate |

**Missing exit criteria for:**

| Item | Missing Criterion | Recommended Addition |
|------|------------------|---------------------|
| D3 | No criterion for structured error variants | "Retry logic uses `ProviderError` enum variants; no string-prefix matching in `is_retryable()`" |
| D7 | No criterion for FnMut callback | "`StreamCallback` accepts `FnMut` closures; a stateful token-counting callback compiles and runs" |
| D9 | No criterion for MCP concurrency | "MCP stdio transport supports concurrent requests via request-ID multiplexing (verified by concurrent call test)" |
| D10 | No criterion for bootstrap file caching | "Bootstrap files (`SOUL.md`, `AGENTS.md`, skills) cached with mtime invalidation; second LLM call in same session does not hit disk" |
| D11 | No criterion for async file I/O | "Skills loader uses `tokio::fs`; no blocking `std::fs` calls on the async executor path" |

**Assessment**: 5 of 11 items lack dedicated exit criteria. The biz drafts (`01-biz-architecture.md`) define all 11 criteria but the orchestrator only captures 6 (plus the generic regression gate). This is a significant gap.

### 1.3 Technical Specification Coverage

Cross-referencing items against `02-tech-pipeline.md`:

| Item | Tech Spec Section | Status |
|------|------------------|--------|
| D1 | 12.1 (Parallel Tool Execution) | COVERED -- full implementation with code |
| D2 | 12.2 (Streaming Failover) | COVERED -- StreamFailoverController with reset protocol |
| D3 | 12.3 (Structured Error Variants) | COVERED -- ProviderError enum with is_retryable() |
| D4 | 12.4 (Configurable Retry Policy) | COVERED -- RetryPolicy struct with config JSON |
| D5 | 12.5 (Latency Recording) | COVERED -- wall-clock timing code |
| D6 | Not found | **MISSING** -- no dedicated tech section |
| D7 | 12.6 (StreamCallback FnMut) | COVERED -- type alias change documented |
| D8 | 12.7 (Bounded Message Bus) | COVERED -- BusConfig with overflow policy |
| D9 | Not found | **MISSING** -- no dedicated tech section |
| D10 | Not found | **MISSING** -- no dedicated tech section |
| D11 | Not found | **MISSING** -- no dedicated tech section |

**Assessment**: 4 of 11 items (D6, D9, D10, D11) lack tech specs. This was already flagged in `review-1-biz-tech.md` (Section 5.1) but remains unresolved.

### 1.4 Dependency Analysis

**Internal dependencies within Element 05:**

The orchestrator groups items into 3 phases (Perf, Reliability, Observability) with implied sequential execution. However, no explicit item-to-item dependencies are documented. Key implicit dependencies:

| Dependency | Reason | In Orchestrator? |
|-----------|--------|:-:|
| D3 before D4 | Retry policy should use structured errors, not strings | NO |
| D7 before D2 | Streaming failover needs FnMut callbacks for stateful reset tracking | NO |
| D1 independent | Can be done in parallel with all others | Implied by phase grouping |
| D10, D11 independent of each other | Different subsystems | Implied by phase grouping |

**Cross-element dependencies:**

| Dependency | Description | In Orchestrator? |
|-----------|-------------|:-:|
| D6 blocks 09 (multi-agent cost tracking) | sender_id needed for L4 planning strategies | YES -- correctly noted in header |
| D9 blocks M1 (FlowDelegator) | MCP transport concurrency needed for delegation | NO -- not mentioned |
| F6 dependency (E5a needs OAuth2 helper) | Relevant because E5a is in Element 06 | N/A for this element |

### 1.5 Crate Assignment Verification

| Item | Orchestrator Crate | improvements.md Crate | Correct? |
|------|-------------------|----------------------|:-:|
| D1 | Not specified | `clawft-core/src/agent/loop_core.rs` | N/A -- orchestrator omits file paths |
| D2 | Not specified | `clawft-llm/src/failover.rs` | N/A |
| D3 | Not specified | `clawft-llm/src/retry.rs`, `clawft-types/src/error.rs` | N/A |
| D4 | Not specified | `clawft-core/src/pipeline/llm_adapter.rs` | N/A |
| D5 | Not specified | `clawft-core/src/pipeline/traits.rs`, `loop_core.rs` | N/A |
| D6 | Not specified | `clawft-core/src/pipeline/tiered_router.rs` | N/A |
| D7 | Not specified | `clawft-core/src/pipeline/traits.rs` | N/A |
| D8 | Not specified | `clawft-core/src/bus.rs` | N/A |
| D9 | Not specified | `clawft-services/src/mcp/transport.rs` | N/A |
| D10 | Not specified | `clawft-core/src/agent/context.rs` | N/A |
| D11 | Not specified | `clawft-core/src/agent/skills_v2.rs` | N/A |

**Assessment**: The orchestrator does not include file/crate assignments at all. This information exists in `improvements.md` and `02-tech-pipeline.md` but is absent from the SPARC document. For a specification document that developers will reference, this is a gap.

### 1.6 Timeline Assessment

- **Stated timeline**: Weeks 2-5 (4 weeks)
- **Item count**: 11 items across 3 phases
- **Phase breakdown**: D-Perf (Week 2-3, 3 items), D-Reliability (Week 3-4, 5 items), D-Observability (Week 4-5, 3 items)

**Realism check**:
- D1 (parallel tool execution): Medium complexity, ~2-3 days. Requires careful race condition testing.
- D2 (streaming failover): High complexity, ~3-5 days. New StreamFailoverController, stream reset protocol, channel adapter integration.
- D3 (structured errors): Medium complexity, ~2-3 days. Straightforward enum replacement.
- D4 (configurable retry): Low complexity, ~1-2 days. Config struct + wiring.
- D5 (latency recording): Low complexity, ~1 day. Simple timing instrumentation.
- D6 (sender_id threading): Medium complexity, ~2-3 days. Pipeline wiring across multiple call sites.
- D7 (FnMut callback): Low complexity, ~1 day. Type alias change + caller updates.
- D8 (bounded bus): Medium complexity, ~2-3 days. Needs overflow policy and config.
- D9 (MCP concurrency): High complexity, ~3-5 days. Request-ID multiplexer for stdio.
- D10 (bootstrap caching): Low-Medium, ~1-2 days. Standard mtime-based cache.
- D11 (async file I/O): Low, ~1 day. tokio::fs drop-in.

**Estimated total**: ~20-30 developer-days for 11 items in a 20-workday window (4 weeks). This is achievable for a single developer focused full-time, but tight. Two developers working the Perf and Reliability tracks in parallel would be more realistic.

**Timeline verdict**: Feasible but aggressive. The orchestrator does not note the critical path or parallelization opportunities within the element.

### 1.7 Element 05 Score: 5/10

**Strengths**:
- All 11 items from improvements.md are captured
- Phase grouping is logical (perf, reliability, observability)
- Dependency on Element 09 is correctly identified
- Header metadata is clean (timeline, status, blocks)

**Weaknesses**:
- 5 of 11 items have no exit criterion (D3, D7, D9, D10, D11)
- 4 of 11 items lack tech specs (D6, D9, D10, D11) -- not in orchestrator and not written
- No file/crate assignments in the orchestrator
- No internal item-to-item dependency ordering
- D9 miscategorized as "Observability" (it is a performance/throughput improvement)
- No cross-element dependency on M1 for D9
- Exit criteria are less precise than the biz draft counterparts (missing numeric thresholds)
- No risk callouts for individual items (D1 race conditions, D2 partial output already rendered)

---

## 2. Element 06: Channel Enhancements

### 2.1 Coverage Assessment

Checking every item from `improvements.md` Workstream E against the orchestrator:

| Item | Description | In Orchestrator? | Phase | Notes |
|------|-------------|:-:|-------|-------|
| E1 | Discord Resume (OP 6) | YES | E-Fix | Correctly placed |
| E2 | Email channel plugin (IMAP + SMTP + OAuth2) | YES (implied) | E-Enterprise (not listed by number) | Absent from phase table but covered by exit criteria |
| E3 | WhatsApp channel | YES | E-Consumer | Correctly placed |
| E4 | Signal / iMessage bridge | YES | E-Consumer | Correctly placed |
| E5 | Matrix / IRC channels | YES | E-Consumer | Correctly placed |
| E5a | Google Chat channel | YES | E-Enterprise | Correctly placed |
| E5b | Microsoft Teams channel | YES | E-Enterprise | Correctly placed |
| E6 | Enhanced heartbeat / proactive check-in | YES | E-Fix | Correctly placed |

**Coverage**: 8/8 items present. Complete.

**ISSUE**: E2 (Email channel) is absent from the Phase E-Enterprise table despite being the highest-priority new channel (P1). The exit criteria reference it ("Email channel receives, triages, and replies to messages") but the phase table jumps from E5a to E5b with no E2 row. This appears to be an omission in the table, not a scope exclusion.

### 2.2 Exit Criteria Evaluation

The orchestrator defines 6 exit criteria for 8 items:

| Exit Criterion | Testable? | Covers Items | Issues |
|----------------|:-:|:-:|--------|
| "Discord reconnects via Resume instead of re-Identify" | YES | E1 | Good |
| "Email channel receives, triages, and replies to messages" | PARTIAL | E2 | Missing: OAuth2 flow criterion, attachment handling, proactive triage |
| "WhatsApp channel sends and receives text messages" | YES | E3 | Good but narrow -- biz draft also requires Cloud API media support |
| "At least one enterprise channel (Google Chat or Teams) operational" | YES | E5a, E5b | Good -- pragmatic scope |
| "All new channels implement `ChannelAdapter` plugin trait" | YES | E2-E5b | Good |
| "All existing channel tests pass" | YES | E1 | Standard regression gate |

**Missing exit criteria for:**

| Item | Missing Criterion | Recommended Addition |
|------|------------------|---------------------|
| E4 | No criterion for Signal/iMessage | "Signal channel sends and receives messages via `signal-cli` subprocess" |
| E5 | No criterion for Matrix/IRC | "Matrix channel joins rooms and sends/receives messages" |
| E6 | No criterion for enhanced heartbeat | "Enhanced heartbeat triggers proactive check-ins across all configured channels on cron schedule" |
| E2 (Gmail OAuth2) | Not tested separately | "Gmail OAuth2 flow completes without plaintext passwords in config" |

**Assessment**: 3 of 8 items (E4, E5, E6) lack explicit exit criteria. E2's criterion exists but is incomplete (omits OAuth2 and attachments). The biz draft (`01-biz-plugins-channels.md`) defines 14 success criteria for this workstream; the orchestrator captures only 6.

### 2.3 Technical Specification Coverage

Cross-referencing items against `02-tech-pipeline.md`:

| Item | Tech Spec Section | Status |
|------|------------------|--------|
| E1 | 13.1 (Discord Resume) | COVERED -- full OP 6 flow with code |
| E2 | 13.2 (Email Channel) | COVERED -- architecture, types, IMAP/SMTP/OAuth2 |
| E3 | 13.3 (WhatsApp) | COVERED -- Cloud API architecture, webhook |
| E4 | 13.4 (Signal/iMessage) | COVERED -- subprocess architecture |
| E5 | 13.5 (Matrix/IRC) | COVERED -- config types |
| E5a | 13.6 (Google Chat) | COVERED -- Pub/Sub architecture |
| E5b | 13.7 (Teams) | COVERED -- Bot Framework architecture |
| E6 | Not found | **MISSING** -- no dedicated tech section |

**Assessment**: 7 of 8 items have tech specs. Only E6 (enhanced heartbeat) lacks a dedicated section. This was also flagged in `review-1-biz-tech.md` (Section 5.1, row E6).

### 2.4 Dependency Analysis

**Declared dependencies in orchestrator header:**
- 04/C1 (plugin traits for new channels)
- 03/A4 (credentials cleanup for email)

**Missing dependencies:**

| Dependency | Description | Source | Impact |
|-----------|-------------|--------|--------|
| E5a depends on F6 (OAuth2 helper) | Google Chat reuses generic OAuth2 flow from Workstream F | `improvements.md` E5a, `01-biz-plugins-channels.md` dependency table | **CRITICAL** -- F6 is scheduled at Week 8 in the dev guide (Element 07 scope) but E5a is in Week 5-7 (Element 06 E-Enterprise phase). This is the F6/E5a dependency ordering violation flagged in `review-1-devguide.md`. |
| E6 depends on B4 (cron unification) | Enhanced heartbeat builds on unified CronService | `improvements.md` E6, `01-biz-plugins-channels.md` dependency table | **MEDIUM** -- B4 is in Element 04 (Architecture). If B4 is not complete by Week 4, E6 cannot start. |
| E2 depends on A4 (credential redaction) | Email credentials must not be stored as plaintext | Already listed | Correctly captured |
| E2+E6 joint dependency | Proactive email triage needs both email channel and heartbeat | `01-biz-plugins-channels.md` feature summary row "E2 + E6" | Not captured as intra-element dependency |

**The F6/E5a violation** is the most significant finding. The orchestrator lists no dependency on F6 at all. Either:
1. E5a must be moved to Week 8+ (after F6), or
2. F6 must be accelerated to Week 6-7 and added as an explicit dependency of Element 06, or
3. E5a must implement its own OAuth2 flow independently (duplicating F6 work)

### 2.5 Crate Assignment Verification

The orchestrator does not include file/crate assignments. From `improvements.md` and `02-tech-pipeline.md`:

| Item | Expected Crate/Location | Status |
|------|------------------------|--------|
| E1 | `clawft-channels/src/discord/channel.rs` | Not in orchestrator |
| E2 | `clawft-channels/src/email/` (new, feature-gated) | Not in orchestrator |
| E3 | `clawft-channels/src/whatsapp/` (new, feature-gated) | Not in orchestrator |
| E4 | `clawft-channels/src/signal/`, `clawft-channels/src/imessage/` | Not in orchestrator |
| E5 | `clawft-channels/src/matrix/`, `clawft-channels/src/irc/` | Not in orchestrator |
| E5a | `clawft-channels/src/google_chat/` | Not in orchestrator |
| E5b | `clawft-channels/src/teams/` | Not in orchestrator |
| E6 | `clawft-services/src/heartbeat/` | Not in orchestrator |

**Assessment**: Same gap as Element 05 -- no crate assignments in the orchestrator document.

### 2.6 Channel Trait Migration Path

A critical architectural concern identified in `review-1-biz-tech.md` and the dev guide reviews: the existing channels (Telegram, Slack, Discord) implement a `Channel` trait, but the orchestrator states "All new channels implement `ChannelAdapter` plugin trait."

**Missing specification**: There is no documented migration path from the old `Channel` trait to the new `ChannelAdapter` trait. Questions that remain unanswered:

1. Will existing channels (Telegram, Slack, Discord) be migrated to `ChannelAdapter` in this sprint?
2. If not, how do two different trait systems coexist in `PluginHost`?
3. Is there a shim/adapter pattern from `Channel` to `ChannelAdapter`?
4. Does E1 (Discord Resume) modify the existing `Channel` impl or a new `ChannelAdapter` impl?

The biz draft (`01-biz-plugins-channels.md` CE-14) states "all new channels implemented as `ChannelAdapter` plugins (zero changes to `clawft-core`)" but does not address the old-to-new trait transition.

The tech spec (`02-tech-pipeline.md` Section 13.2) shows `EmailChannel` implementing the old `Channel` trait (`impl Channel for EmailChannel`), NOT the new `ChannelAdapter` trait from `clawft-plugin`. This is an inconsistency between the orchestrator's stated goal ("All new channels implement `ChannelAdapter` plugin trait") and the tech spec's actual code.

### 2.7 Timeline Assessment

- **Stated timeline**: Weeks 4-8 (5 weeks)
- **Item count**: 8 items (including E5a, E5b) across 3 phases
- **Phase breakdown**: E-Fix (Week 4-5, 2 items), E-Enterprise (Week 5-7, 3 items), E-Consumer (Week 6-8, 3 items)

**Realism check**:
- E1 (Discord Resume): Low-Medium, ~2-3 days. Well-defined protocol, code exists but is unused.
- E2 (Email): High complexity, ~5-8 days. IMAP/SMTP/OAuth2/attachment/triage pipeline.
- E3 (WhatsApp): Medium, ~3-5 days. Webhook + REST API wrapper.
- E4 (Signal/iMessage): Medium, ~3-4 days (Signal), ~2-3 days (iMessage). Subprocess management.
- E5 (Matrix/IRC): Medium, ~3-4 days each (6-8 total). Two distinct protocol adapters.
- E5a (Google Chat): Medium, ~3-5 days. Pub/Sub + REST, assuming F6 OAuth2 exists.
- E5b (Teams): Medium-High, ~4-6 days. Bot Framework + Azure AD.
- E6 (Enhanced heartbeat): Low-Medium, ~2-3 days. Extends existing CronService.

**Estimated total**: ~28-42 developer-days for 8 items in a 25-workday window (5 weeks). This requires 1.5-2 developers working in parallel. The consumer channels (E3, E4, E5) can be developed independently and in parallel.

**Timeline concern**: E2 (Email) is the most complex single item and is not explicitly phased in the orchestrator. It appears to be implicitly in E-Enterprise (Week 5-7) but the exit criteria suggest full triage capability, which requires E6 (heartbeat) from E-Fix (Week 4-5). This internal dependency ordering is reasonable if E6 completes in Week 5 before E2's triage feature in Week 6-7.

**F6 blocker**: If E5a truly requires F6 (OAuth2 helper), and F6 is not available until Week 8, then E5a cannot complete in its stated Week 5-7 window.

**Timeline verdict**: Feasible with 2 developers, but the F6/E5a dependency makes the E-Enterprise phase timeline unreliable without resolution.

### 2.8 Element 06 Score: 5/10

**Strengths**:
- All 8 items from improvements.md are captured
- Phase grouping is logical (fix, enterprise, consumer)
- C1 and A4 dependencies correctly identified
- Exit criteria for core channels (Discord, Email, WhatsApp, enterprise) are present

**Weaknesses**:
- E2 missing from the E-Enterprise phase table (present only in exit criteria)
- 3 of 8 items lack explicit exit criteria (E4, E5, E6)
- E6 missing tech spec
- F6/E5a dependency violation: E5a needs F6 (OAuth2) but F6 is not listed as a dependency and is scheduled later
- B4 dependency for E6 not listed in orchestrator header
- No crate/file assignments in the orchestrator
- No Channel-to-ChannelAdapter migration path documented
- Tech spec inconsistency: `02-tech-pipeline.md` shows old `Channel` trait, not `ChannelAdapter`
- Exit criteria less specific than biz draft (missing OAuth2, attachment, media, heartbeat criteria)

---

## 3. Cross-Element Dependency Analysis (05 vs 06)

### 3.1 Direct Dependencies

There are no direct dependencies between Elements 05 and 06 documented in either orchestrator. However:

| Interaction | Type | Impact |
|------------|------|--------|
| D8 (bounded bus) affects all channels | Implicit | If D8 changes the MessageBus API (bounded channels, overflow policy), all channel implementations including new ones in E must use the new API. D8 is in Week 3-4; new channels start Week 5+. Timeline is safe but the API contract must be stable before E-Enterprise begins. |
| D6 (sender_id) used by channels | Implicit | Channels produce `InboundMessage` which carries `sender_id`. If D6 changes how `sender_id` is threaded, channel implementations need to provide it. Not a blocking dependency since channels already set this field, but the contract should be documented. |
| D9 (MCP concurrency) and E6 (heartbeat) | Indirect | Both touch `clawft-services`. D9 modifies MCP transport; E6 modifies heartbeat/cron. No shared files, but the same crate is modified. |

### 3.2 Shared Crate Concerns

Both elements touch overlapping crates:

| Crate | Element 05 Items | Element 06 Items | Conflict Risk |
|-------|-----------------|-----------------|:--:|
| `clawft-core` | D1, D5, D6, D7, D8, D10, D11 | None directly | LOW |
| `clawft-services` | D9 | E6 | LOW (different subdirectories) |
| `clawft-channels` | None | E1, E2, E3, E4, E5, E5a, E5b | LOW (Element 05 does not touch channels) |
| `clawft-types` | D3 (new error types) | None directly | LOW |

**Assessment**: Low conflict risk between Elements 05 and 06. They operate on largely disjoint codebases. The main coordination point is the MessageBus API (D8), which should be stabilized before channel work begins.

---

## 4. Known Issues Status

Checking all known issues flagged in prior reviews:

### 4.1 Missing Tech Specs for D6, D9, D10, D11

**Status**: UNRESOLVED

All four items were flagged in `review-1-biz-tech.md` Section 5.1 as "PARTIAL -- Missing tech spec." The orchestrator for Element 05 does not include tech specs (it is a planning/coordination document, not a tech spec). However, `02-tech-pipeline.md` still lacks sections for these items.

**Impact**: Developers implementing D6, D9, D10, D11 will have only the `improvements.md` descriptions to work from. D6 (sender_id threading) is the most architecturally significant -- it requires pipeline wiring across multiple call sites (`InboundMessage` -> `ChatRequest` -> `RoutingDecision` -> `CostTracker.update()`), and without a tech spec showing the exact type changes, there is risk of implementation inconsistency.

**Recommendation**: Write a tech spec section for D6 before implementation begins. D10 and D11 are straightforward enough for inline implementation guidance. D9 needs at minimum a multiplexer design sketch.

### 4.2 Missing Tech Spec for E6 (Enhanced Heartbeat)

**Status**: UNRESOLVED

Flagged in `review-1-biz-tech.md` as "PARTIAL -- Missing dedicated tech section."

**Impact**: Low. E6 extends existing `CronService` with a "check-in" mode. The pattern is well-established (cron triggers a lambda that sends a message via the bus). A short tech spec showing the new config fields and the proactive message format would suffice.

### 4.3 F6/E5a Dependency Ordering Violation

**Status**: UNRESOLVED

Flagged in `review-1-devguide.md` Section 5F (Issue #1) and Section 11 Critical Fix #1: "F6 (OAuth2 helper) from Week 8 but E5a (Google Chat) is in Week 7."

The Element 06 orchestrator does not list F6 as a dependency at all. The `improvements.md` explicitly states `E5a` deps include `C1`, and the biz draft (`01-biz-plugins-channels.md`) dependency table shows "E5a depends on F6 (OAuth2 helper) -- Reuses generic OAuth2 flow."

**Impact**: HIGH. E5a cannot be implemented as designed without F6. Either:
- F6 must be accelerated (moved from Element 07 to Element 06 or done concurrently), or
- E5a must implement a standalone OAuth2 flow, or
- E5a must be deferred past Week 8

**Recommendation**: Add F6 as an explicit dependency in the Element 06 orchestrator header. Coordinate with Element 07 to either accelerate F6 or defer E5a to E-Consumer phase (Week 6-8+).

### 4.4 Missing sender_id Success Criterion for D6

**Status**: PARTIALLY RESOLVED

`review-1-biz-tech.md` Section 10.2 noted "D6 (sender_id threading) -- biz draft mentions it but no success criterion." The Element 05 orchestrator *does* include a criterion: "sender_id available in CostTracker for per-user tracking." This addresses the gap.

However, the criterion could be more precise. Recommended enhancement: "sender_id propagated from InboundMessage through ChatRequest and RoutingDecision to CostTracker.update(); integration test verifies end-to-end flow with a mock channel message."

### 4.5 Channel Trait Migration Path

**Status**: UNRESOLVED -- NOT PREVIOUSLY FLAGGED

This is a new finding. Neither the orchestrator, the biz drafts, nor the tech specs document how the existing `Channel` trait relates to the new `ChannelAdapter` trait from `clawft-plugin` (C1). The tech spec (`02-tech-pipeline.md`) shows `EmailChannel` implementing the old `Channel` trait, contradicting the orchestrator's exit criterion ("All new channels implement `ChannelAdapter` plugin trait").

**Impact**: MEDIUM. Without a migration plan, implementers may build new channels against the wrong trait, causing rework when C7 (PluginHost unification) lands.

**Recommendation**: Add a section to the Element 06 orchestrator (or the C1/C7 tech spec) documenting:
1. Whether new channels should implement `Channel` or `ChannelAdapter`
2. If `ChannelAdapter`, provide a blanket impl or adapter shim for backward compatibility
3. If `Channel` initially (for practical reasons), document the planned migration to `ChannelAdapter` under C7

---

## 5. Specific Gaps and Fix Recommendations

### 5.1 Element 05 Fixes

| # | Gap | Severity | Fix |
|---|-----|----------|-----|
| 05-1 | 5 items lack exit criteria (D3, D7, D9, D10, D11) | HIGH | Add the criteria listed in Section 1.2 above |
| 05-2 | No file/crate assignments | MEDIUM | Add a "File Ownership" column to each phase table referencing the crate and primary file |
| 05-3 | D9 miscategorized as Observability | LOW | Move D9 to D-Perf or create a D-Transport sub-phase |
| 05-4 | No internal dependency ordering | MEDIUM | Add dependency notes: D3 before D4; note D1 independence |
| 05-5 | Missing cross-element dependency: D9 blocks M1 | LOW | Add to header or a dependency section |
| 05-6 | Exit criteria lack numeric thresholds | MEDIUM | Incorporate thresholds from biz draft (e.g., "3 tools, 100ms each, complete in <200ms") |
| 05-7 | 4 items lack tech specs (D6, D9, D10, D11) | HIGH | Write tech spec sections for D6 and D9 at minimum; D10/D11 can be brief stubs |
| 05-8 | No risk section | LOW | Add 2-3 key risks (D1 race conditions, D2 partial output rendering, D9 multiplexer complexity) from biz draft risk register |

### 5.2 Element 06 Fixes

| # | Gap | Severity | Fix |
|---|-----|----------|-----|
| 06-1 | E2 missing from E-Enterprise phase table | HIGH | Add E2 row to the E-Enterprise table with description, transport (IMAP+SMTP), and auth (Password/OAuth2) |
| 06-2 | 3 items lack exit criteria (E4, E5, E6) | HIGH | Add criteria listed in Section 2.2 above |
| 06-3 | F6 dependency not listed | HIGH | Add F6 (OAuth2 helper) to dependency header; coordinate timeline resolution |
| 06-4 | B4 dependency not listed for E6 | MEDIUM | Add 04/B4 to dependency header for E6 (heartbeat builds on unified CronService) |
| 06-5 | No Channel vs ChannelAdapter migration path | MEDIUM | Add a section documenting which trait new channels should implement and the migration plan |
| 06-6 | Tech spec shows old `Channel` trait, not `ChannelAdapter` | MEDIUM | Update `02-tech-pipeline.md` Section 13.2 to use `ChannelAdapter` or document the intentional use of `Channel` with migration plan |
| 06-7 | No file/crate assignments | MEDIUM | Add crate locations to phase tables |
| 06-8 | E6 lacks tech spec | LOW | Write a brief tech spec for enhanced heartbeat (config fields, proactive message format, cron integration) |
| 06-9 | No risk section | LOW | Add risks: WhatsApp rate limits, OAuth2 token refresh, Signal subprocess management (from biz draft risk register) |
| 06-10 | Missing E2+E6 intra-element dependency | LOW | Note that proactive email triage requires both E2 and E6 |

---

## 6. Development Readiness Assessment

### Element 05: Pipeline & LLM Reliability

| Criterion | Status |
|-----------|--------|
| All items captured | PASS |
| Exit criteria complete | FAIL (5 of 11 missing) |
| Tech specs available | FAIL (4 of 11 missing) |
| Dependencies documented | PARTIAL (cross-element partially, internal missing) |
| Crate assignments present | FAIL |
| Timeline realistic | PASS (aggressive but feasible) |
| Risks identified | FAIL |

**Development Readiness: NEEDS MAJOR WORK**

Items D1-D5, D7, D8 are ready for development (they have tech specs and can derive exit criteria from the biz draft). Items D6, D9 need tech specs before development. Items D10, D11 are simple enough to implement from improvements.md descriptions alone but should have exit criteria added. The orchestrator itself needs significant expansion to serve as a specification-quality document.

### Element 06: Channel Enhancements

| Criterion | Status |
|-----------|--------|
| All items captured | PASS (but E2 missing from table) |
| Exit criteria complete | FAIL (3 of 8 missing, 1 incomplete) |
| Tech specs available | PARTIAL (7 of 8, only E6 missing) |
| Dependencies documented | FAIL (F6 and B4 missing) |
| Crate assignments present | FAIL |
| Timeline realistic | FAIL (F6/E5a violation makes E-Enterprise timeline unreliable) |
| Risks identified | FAIL |

**Development Readiness: NEEDS MAJOR WORK**

E1 (Discord Resume) is fully ready -- it has a complete tech spec and clear exit criteria. E2, E3, E5a, E5b have tech specs but E2 is missing from the phase table and E5a has an unresolved F6 dependency. E4, E5, E6 need exit criteria. The Channel vs ChannelAdapter trait question must be resolved before any new channel implementation begins.

---

## 7. Summary of Findings

### By Severity

**HIGH (6 issues)**:
1. Element 05: 5 items lack exit criteria (D3, D7, D9, D10, D11)
2. Element 05: 4 items lack tech specs (D6, D9, D10, D11) -- D6 and D9 are architecturally significant
3. Element 06: E2 (highest-priority new channel) missing from E-Enterprise phase table
4. Element 06: 3 items lack exit criteria (E4, E5, E6)
5. Element 06: F6/E5a dependency ordering violation -- E5a cannot be built without F6
6. Element 06: F6 not listed as a dependency at all

**MEDIUM (6 issues)**:
7. Both elements: No file/crate assignments in orchestrator documents
8. Element 05: No internal item-to-item dependency ordering
9. Element 05: Exit criteria lack numeric thresholds present in biz drafts
10. Element 06: B4 dependency for E6 not listed
11. Element 06: Channel vs ChannelAdapter migration path undocumented
12. Element 06: Tech spec uses old `Channel` trait, contradicting orchestrator goal

**LOW (5 issues)**:
13. Element 05: D9 miscategorized as Observability
14. Element 05: Missing D9 blocks M1 cross-element dependency
15. Both elements: No risk sections
16. Element 06: E6 lacks tech spec (straightforward to write)
17. Element 06: E2+E6 intra-element dependency not documented

### Recommended Priority for Fixes

**Before development begins (blockers):**
1. Resolve F6/E5a dependency ordering (HIGH)
2. Add missing exit criteria to both orchestrators (HIGH)
3. Write tech spec for D6 sender_id threading (HIGH)
4. Clarify Channel vs ChannelAdapter trait choice for new channels (MEDIUM)

**During first implementation week:**
5. Add E2 to Element 06 phase table (HIGH -- trivial fix)
6. Write tech spec for D9 MCP concurrency (HIGH)
7. Add crate/file assignments to both orchestrators (MEDIUM)
8. Add B4 dependency to Element 06 header (MEDIUM)

**Can be addressed incrementally:**
9. Add risk sections to both orchestrators (LOW)
10. Refine exit criteria with numeric thresholds (MEDIUM)
11. Write brief tech stubs for D10, D11, E6 (LOW)
