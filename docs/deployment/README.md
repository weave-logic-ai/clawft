# Deployment docs (repo operator guides)

**Decision (WEFT-468, 2026-07-31):** Keep long-form operator guides in
`docs/deployment/*.md` for offline/repo use, and surface the public
routes through Fumadocs under
[`docs/src/content/docs/weftos/guides/`](../src/content/docs/weftos/guides/).

This is **not** a bulk delete of `docs/deployment/`, and not a silent
fork: ADR-014 still treats Fumadocs as the **public site** source of
truth. Operator-depth markdown stays next to Docker/CI scripts that
link relative paths.

| Repo guide | Fumadocs public page |
|------------|----------------------|
| [docker.md](./docker.md) | [guides/deployment-docker](../src/content/docs/weftos/guides/deployment-docker.mdx) |
| [release.md](./release.md) | [guides/deployment-release](../src/content/docs/weftos/guides/deployment-release.mdx) |
| [wasm.md](./wasm.md) | [guides/deployment-wasm](../src/content/docs/weftos/guides/deployment-wasm.mdx) |
| [install.md](./install.md) | (summary in getting-started / INSTALL elsewhere) |
| SOPs | [guides/deployment-sops](../src/content/docs/weftos/guides/deployment-sops.mdx) |

When editing deployment content: update the Fumadocs page for anything
users find via the site; keep `docs/deployment/*.md` in sync for
operators cloning the repo.
