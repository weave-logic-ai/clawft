# ADR-015: Three-Property Web Architecture

**Date**: 2026-03-28
**Status**: Accepted
**Deciders**: Web Presence Strategy Team

## Context

WeaveLogic needs a web presence that serves three distinct audiences: buyers (CTOs, CEOs evaluating AI assessments), developers (technical evaluators, open-source contributors), and assessment clients (using the product). A single site mixing marketing copy with API documentation with product UI creates confusion and dilutes each audience's experience.

## Decision

Deploy three web properties with distinct purposes:

1. **weavelogic.ai** -- Marketing + product landing. Buyer journey: value proposition, assessment packages, pricing, case studies, technology overview for non-technical buyers. Next.js rewrite with corrected messaging (no fabricated social proof).

2. **weftos.weavelogic.ai** -- Platform documentation. Developer and technical evaluator audience. Fumadocs site with 56+ pages covering concepts, guides, reference, vision, and contributing. Subtle cross-link CTA: "Want us to analyze YOUR systems?"

3. **assess.weavelogic.ai** -- Assessment product app. Separate application with its own auth, data model, and user flows. Intake questionnaire, scoring, generated reports, admin dashboard.

## Consequences

### Positive
- Each property speaks its audience's language without mixing concerns
- Marketing site optimized for conversion; docs site optimized for depth
- Assessment app deployable and scalable independently
- Cross-linking creates a natural buyer journey: learn -> evaluate -> use

### Negative
- Three properties to maintain, deploy, and keep consistent
- Shared header/nav component needed for brand consistency
- DNS and hosting configuration for three subdomains

### Neutral
- Phased deployment: marketing site (week 1-2), docs site (week 2-3), assessment MVP (week 3-6)
- weavelogic.ai/technology page bridges buyers to the technical docs
