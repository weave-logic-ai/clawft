# ADR-014: Fumadocs as Single Documentation Source of Truth

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Sprint 11 Symposium Track 5 + Fumadocs Unification Plan

## Context

Documentation exists in two stacks: 38 Fumadocs MDX pages in `docs/src/content/docs/` (high quality, with search, navigation, dark mode) and 87+ standalone markdown files in `docs/` (broader coverage but no site infrastructure). Overlap exists in architecture, kernel phases, and governance topics. The standalone docs contain categories (specs, quickstart, install guide, VISION) with no Fumadocs equivalent. Two sources of truth means two places to update, and they always drift.

## Decision

Fumadocs is the single source of truth for all public documentation. Deploy to `weftos.weavelogic.ai`. Migrate 18 standalone docs into Fumadocs MDX pages. Symposium results, analysis outputs, and SPARC plans remain as internal-only markdown in the repo (not served by the site). Superseded standalone files get a migration header pointing to the canonical Fumadocs page.

Do not version docs at 0.x -- maintain a single "latest" site until 1.0.

## Consequences

### Positive
- Single place to update documentation
- Full-text search, navigation, table of contents, dark mode, responsive layout from Fumadocs
- 56 total pages after migration (from 38 + 87 fragmented)
- Deployed on Vercel with zero-config Next.js hosting

### Negative
- 11-14 hours of migration effort to move 18 files and write 3 gap pages
- Internal docs excluded from public site (symposium sessions, analysis outputs)
- MDX format is slightly more complex than plain markdown

### Neutral
- clawft framework docs remain as a sub-section of the WeftOS site
- Symposium and working documents stay in the repo as reference
