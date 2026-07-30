# WEFT-596 result — ADR-057 substrate per-path read ACLs

**Branch**: `wave0a/weft-596-substrate-read-acl`  
**Commit**: `9a121457d8ebeb9d1433d3fcb3f4511ea17bc3a6`  
**Status**: first vertical slice landed  
**Date**: 2026-07-30  
**Agent**: coder-596 (wave-0a)

## Ticket AC

| Criterion | Status |
|---|---|
| Per-path ACL table under `substrate/<mesh-id>/acl/<glob>` (allow/deny/inherit, identity strings `node:` / `actor:` / `scope:` / `public`) | **Done** — in-memory table + storage-path helper; path-trie with `*` / `**` |
| Read RPCs (`substrate.read` / `.list` / `.subscribe`) enforce ACL; rejections emit distinct `acl_denied` + chain-log | **Done** — enforced in `SubstrateService::egress_check`; wire `acl_denied:…`; authenticated denials buffered → ExoChain `substrate.read.denied` from weave daemon |
| Deny-by-default for `sensor/` and `actor/<id>/`; public for cluster/health, meta, chain | **Done** — boot defaults + structural owner expansion |

## What shipped

### `crates/clawft-substrate`
- New module `src/acl.rs`:
  - `CallerIdentity`, `AclRule`, `AclTable` (path-trie), `AclDenied` (`CODE = "acl_denied"`)
  - `AclTable::with_boot_defaults(mesh_id)`, `publish_public`, `plan_publish_public`
  - `READ_DENIED_EVENT = "substrate.read.denied"`
  - Structural defaults: private `sensor/**` + `health/sensor/**` + `a-*/**`; public meta/health/cluster/chain
- `Substrate` gains optional ACL: `with_acl_defaults`, `set_acl`, `check_read`, `get_for`, `snapshot_for`, `publish_public`
- **19 unit tests** covering allow/deny prefixes, inherit, deny-leaf, publish_public, subscriber-vs-meta integration

### `crates/clawft-kernel`
- `SubstrateService` seeds ADR-057 boot ACL (`with_mesh_id`)
- `egress_check` runs ACL **before** sensitivity Capture gate
- `EgressDenied::{acl_denied, wire_message, is_acl_denied}`; `take_acl_denials` / `publish_public`
- List filters children through ACL so private path *names* do not leak
- **5 new service tests** + 2 existing gated-publish tests updated for private sensor reads
- Dep: `clawft-substrate` (ACL types shared with ontology tree)

### `crates/clawft-weave`
- `handle_substrate_{read,list,subscribe}` return `e.wire_message()` (`acl_denied:…`)
- `flush_acl_denials_to_chain` appends buffered denials to ExoChain when feature enabled

## Verification

```text
scripts/build.sh check                          # green
cargo test -p clawft-substrate --lib acl        # 19 passed
cargo test -p clawft-kernel --lib substrate_service  # 39 passed
cargo test -p clawft-weave --test substrate_rpc # 11 passed
```

## Residual (follow-ups / WEFT-429)

WEFT-429 (wire real ADR-012 `governance::Gate` through `Substrate::subscribe_adapter`) remains blocked only by product scheduling — ACL path gate is now in place for remote subscribers.

Not in this slice (documented residual):

1. **Explorer lock icon** for `acl_denied` (ADR-057 UI AC) — GUI work, separate ticket.
2. **Mesh id at boot**: service defaults mesh id `"local"`; mesh bootstrap should call `SubstrateService::with_mesh_id` / `set_acl_table` with the real mesh id.
3. **Persist ACL rules** into the live substrate tree under `substrate/<mesh>/acl/<encoded-glob>` (storage path helper exists; write path + signed ACL updates not wired).
4. **Anonymous denial rate-limit summarize** (ADR-057 step 5) — authenticated denials chain-log; anonymous are skipped (no summarize yet).
5. **`publish_public` on no_std ESP32 client** — `plan_publish_public` is the pure helper; firmware crate still needs to call signed publish + ACL seal.
6. **Capability token → `scope:` population** on RPC callers beyond raw `actor_id` string parsing.
7. **WEFT-429**: wire ADR-012 Gate through `subscribe_adapter` (ontology path), orthogonal but complementary to this daemon-side ACL.

## Files touched

- `crates/clawft-substrate/src/acl.rs` (new)
- `crates/clawft-substrate/src/lib.rs`
- `crates/clawft-substrate/src/snapshot.rs`
- `crates/clawft-kernel/Cargo.toml`
- `crates/clawft-kernel/src/lib.rs`
- `crates/clawft-kernel/src/substrate_service.rs`
- `crates/clawft-weave/src/daemon.rs`
- `docs/plans/wave-0a-WEFT-596-result.md` (this file)
