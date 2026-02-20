# Development Notes: Element 06 -- Channel Enhancements

**Element**: 06 - Channel Enhancements
**Workstream**: E (Channel Enhancements)
**Timeline**: Weeks 4-8
**SPARC Plan**: `.planning/sparc/06-channel-enhancements/`
**Tracker**: `.planning/sparc/06-channel-enhancements/04-element-06-tracker.md`
**Status**: Active Development

---

## Directory Structure

Each phase directory contains four tracking files:

| File | Purpose |
|------|---------|
| `decisions.md` | Key design decisions with rationale |
| `blockers.md` | Active blockers requiring escalation |
| `difficult-tasks.md` | Tasks needing extra attention or research |
| `notes.md` | General development notes, findings, tips |

## Phase Directories

### `e-fix/` -- Existing Channel Fixes (Week 4-5)

Items: E1 (Discord Gateway Resume), E6 (Enhanced Heartbeat)

Focus: Fix Discord reconnect behavior and add proactive heartbeat/check-in mode. E6 depends on B4 (CronService) from Element 03.

### `e-enterprise/` -- Enterprise Channels (Week 5-7)

Items: E2 (Email IMAP+SMTP+OAuth2), E5a (Google Chat), E5b (Microsoft Teams)

Focus: Enterprise-grade channel integrations. E2 is P0 MVP. E5a has a timeline risk due to F6 (OAuth2 helper) dependency from Element 07. All implement `ChannelAdapter` trait.

### `e-consumer/` -- Consumer Channels (Week 6-8)

Items: E3 (WhatsApp Cloud API), E4 (Signal subprocess), E5 (Matrix/IRC)

Focus: Consumer messaging platform integrations. E4 requires careful subprocess management. All implement `ChannelAdapter` trait and are feature-gated.

## Conventions

### Decision Records

```markdown
### D-{number}: {title}
**Date**: YYYY-MM-DD
**Status**: Accepted | Superseded | Rejected
**Context**: Why was this decision needed?
**Decision**: What was decided?
**Consequences**: What are the implications?
```

### Blocker Records

```markdown
### B-{number}: {title}
**Date Identified**: YYYY-MM-DD
**Date Resolved**: YYYY-MM-DD | OPEN
**Severity**: Critical | High | Medium
**Blocked Items**: {list of items}
**Description**: What is blocking progress?
**Resolution**: How was it resolved? (or proposed resolution)
```

### Difficult Task Records

```markdown
### DT-{number}: {title}
**Item**: {E1, E2, E3, etc.}
**Expected Difficulty**: Low | Medium | High
**Actual Difficulty**: Low | Medium | High | Critical
**Description**: What made this harder than expected?
**Lessons Learned**: What should future agents know?
```

## Cross-References

- Orchestrator: `.planning/sparc/06-channel-enhancements/00-orchestrator.md`
- Tracker: `.planning/sparc/06-channel-enhancements/04-element-06-tracker.md`
- Review (Iteration 1): `.planning/sparc/reviews/iteration-1-spec-05-06.md`
- Review (Iteration 3): `.planning/sparc/reviews/iteration-3-spec-03-06.md`
