---
name: weftos-api-docs
description: Generate rustdoc API reference and deploy to weftos.weavelogic.ai/api
triggers:
  - api docs
  - rustdoc
  - generate api reference
  - update api docs
---

# WeftOS API Docs Skill

Generate the Rust API reference from rustdoc and deploy it as part of the Fumadocs site.

## How It Works

1. `cargo doc --no-deps` generates HTML for 8 publishable crates
2. `scripts/generate-api-docs.sh` copies output to `docs/src/public/api/`
3. Next.js serves the rustdoc HTML as static files at `/api/`
4. Vercel deploys on push to master

## Generate API Docs

```bash
scripts/generate-api-docs.sh
```

This generates ~1,177 HTML pages (~42MB) covering:
- `weftos` — product facade
- `clawft-kernel` — kernel (process table, services, governance, mesh, ExoChain)
- `clawft-core` — agent loop, pipeline, context, memory
- `clawft-types` — shared types
- `clawft-platform` — platform abstraction
- `clawft-plugin` — plugin SDK
- `clawft-llm` — LLM provider abstraction
- `exo-resource-tree` — hierarchical resource namespace

## Clean

```bash
scripts/generate-api-docs.sh --clean
```

## Access

- Local: `http://localhost:3000/api/weftos/index.html`
- Production: `https://weftos.weavelogic.ai/api/weftos/index.html`
- Fallback: `https://docs.rs/weftos` (auto-built by docs.rs from crates.io)

## Key Details

- Output goes to `docs/src/public/api/` (gitignored — generated, not committed)
- Source viewer is stripped to save ~11MB (users read source on GitHub)
- `/api/` redirects to `/api/weftos/index.html`
- Rustdoc includes its own CSS/JS/fonts — no Fumadocs styling conflict
- Search works within rustdoc pages (client-side, self-contained)

## When to Regenerate

- After any public API changes (new structs, functions, traits, doc comments)
- Before a release that updates docs
- After adding new publishable crates

## For Vercel Deployment

The `generate-api-docs.sh` should run as a pre-build step. Add to `docs/src/package.json`:

```json
"scripts": {
  "prebuild": "cd ../.. && scripts/generate-api-docs.sh",
  "build": "next build"
}
```

Or configure in Vercel dashboard: Build Command = `cd ../.. && scripts/generate-api-docs.sh && cd docs/src && npm run build`
