# Release Engineering Notes: Sprint 11-12

## Releases
| Version | Date | Targets | Notes |
|---------|------|---------|-------|
| v0.1.0 | 2026-03-31 | 5 native | First release, installer URLs wrong |
| v0.1.1 | 2026-03-31 | 5 native | Fixed URLs, Homebrew tap |
| v0.1.2 | 2026-04-01 | 5 native | Automated deploy test |
| v0.1.3 | 2026-04-01 | 8 native + WASI | musl failed (openssl), Win ARM failed (ring) |
| v0.1.4 | 2026-04-01 | 7 native + WASI | Fixed musl, deferred Win ARM |
| v0.2.0 | 2026-04-01 | 7 native + WASI | Sprint 12 features |

## Distribution Channels
| Channel | URL | Status |
|---------|-----|--------|
| GitHub Releases | github.com/weave-logic-ai/weftos/releases | Live |
| crates.io | crates.io/crates/weftos | 10 crates |
| npm | npmjs.com/package/@weftos/core | @weftos/core 0.1.1 |
| Docker | ghcr.io/weave-logic-ai/weftos | Multi-arch |
| Homebrew | weave-logic-ai/homebrew-tap | 3 formulae |
| Docs | weftos.weavelogic.ai | Fumadocs + rustdoc |
| docs.rs | docs.rs/weftos | Auto-built |

## Key Secrets
| Secret | Location | Purpose |
|--------|----------|---------|
| HOMEBREW_TAP_TOKEN | GitHub repo secret | Homebrew formula push |
| CRATES_API_TOKEN | .env (local) | cargo publish |
| NPM_TOKEN | .env (local) | npm publish (needs Bypass 2FA) |
| VERCEL_TOKEN | .env (local) | Vercel deploy |

## Gotchas Log
1. Tag must match workspace version
2. [profile.dist] required
3. repository URL baked into installer scripts
4. crates.io rate limit: ~1 new crate/10 min
5. npm granular token with "Bypass 2FA on publish"
6. WASI workflow needs 30 min wait for cargo-dist
7. musl needs vendored-openssl (git2)
8. Windows ARM deferred (ring + xwin)
9. Ruvector fork crates: weftos-rvf-crypto, weftos-rvf-wire
10. Vercel root dir: docs/src (set in dashboard)
