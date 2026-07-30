# WEFT-150 result — leaf-push path governance / ExoChain

**Branch:** `wave0h/weft-150-leaf-push-gov`  
**Worktree:** `/Users/mathewbeane/.grok/worktrees/mathewbeane-weftos/subagent-019fb4b6-6bee-7ac0-92b1-c4dcb61acc37`  
**Base:** `release/0.8-staging`  
**Ticket:** ws02: kernel — verify weftos-leaf-types push path goes through governance / chain  

## Problem

Unclear whether the v0.6.17 leaf-push CLI (`weaver leaf push` / scene
producers over `weftos-leaf-types`) routes publishes through governance
and ExoChain. No workstream claimed the path.

## Audit — end-to-end path

```
weaver leaf push|scene
  → CBOR encode (LeafPush audio | SceneEnvelope)
  → base64-wrap JSON { type: leaf_push|scene_push, cbor_b64, target_pubkey }
  → DaemonClient.call("ipc.publish", topic=mesh.leaf.<pk>.push)   [auth=admin on UDS]
  → clawft-weave daemon::dispatch "ipc.publish"
       optional actor_id + Ed25519 signature verify
       KernelMessage{ from: PID0, target: Topic(topic), payload }
       → A2ARouter::send_checked(msg, chain)     [exochain feature]
            cm.append(source=ipc, kind=ipc.send, …)   // intent before delivery
            A2ARouter::send
              routing gate (if set): action "ipc.send"
              Topic: CapabilityChecker::check_ipc_topic(from, topic)  // WEFT-150
              topic_router + mesh peers_for_topic fan-out
       → on success: cm.append(source=ipc, kind=ipc.publish, structured)  // WEFT-150
```

| Layer | Before WEFT-150 | After WEFT-150 |
|-------|-----------------|----------------|
| `weftos-leaf-types` | Schema-only (correct) | + `is_push_topic` / `is_announce_topic` helpers |
| RPC capability | `ipc.publish` defaulted to **Read** (anonymous could publish) | **Write** gated |
| Actor signature | Optional; wrong sig rejected | Unchanged (bring-up still allows missing actor_id) |
| Topic IpcScope | **Not enforced** on A2A Topic publish | `check_ipc_topic` on Topic branch |
| ExoChain | `ipc.send` via `send_checked` only | + structured `ipc.publish` with `leaf_push` flag |
| Browser BP-001 | Only when `platform=browser` on governance requests | Unchanged; Restricted scope now also blocks at A2A |

### Confirmed coverage

- **ExoChain:** Yes. Leaf pushes hit the chain twice under default
  `exochain` feature: generic `ipc.send` + structured `ipc.publish`
  (`leaf_push: true`, `wire_type`, `topic`, …).
- **Governance:** Yes at three layers after this fix — Write RPC cap,
  optional actor signature, A2A topic scope. Kernel PID 0 (CLI path)
  uses `IpcScope::All`. Browser / Restricted agents cannot publish
  `mesh.leaf.*` (not a public topic).

### Documented residual bypasses (intentional bring-up)

1. Missing `actor_id` still accepted (warn-logged) for unsigned operator
   publish — same as pre-existing ipc.subscribe_stream bring-up.
2. Local UDS `DaemonClient` auto-attaches `auth: admin` (filesystem
   socket trust model, WEFT-479). TCP relay remains capability-gated.
3. Routing-time gate on A2A uses action `ipc.send`, not `ipc.publish`
   (pre-existing dual-layer design); chain kind is `ipc.publish` for
   audit queries.

## What shipped

| File | Change |
|------|--------|
| `crates/weftos-leaf-types/src/lib.rs` | `is_push_topic` / `is_announce_topic` + tests |
| `crates/clawft-weave/src/capability.rs` | `ipc.publish` → Write; tests |
| `crates/clawft-kernel/src/a2a.rs` | Topic publish calls `check_ipc_topic`; Restricted denied on leaf topics |
| `crates/clawft-weave/src/daemon.rs` | On successful publish: chain `ipc.publish` event with `leaf_push` / `wire_type` |
| `crates/clawft-weave/tests/ipc_subscribe_stream.rs` | Publish with `auth: admin` |
| `docs/leaf-push-protocol.md` | Governance / ExoChain section + status row |
| `docs/plans/wave-0h-WEFT-150-result.md` | This report |

## Acceptance

| Criterion | Status |
|-----------|--------|
| Audit leaf-push path end-to-end | Yes (above) |
| Confirm governance + chain **or** document bypass | Yes — confirmed with residual bring-up notes |
| Add gates if missing | Yes — Write RPC + topic IpcScope + structured chain event |
| Test asserts push event lands in chain | Yes — `ipc_publish_leaf_push_lands_on_chain` |

## Verification

```text
scripts/build.sh test weftos-leaf-types
# 12 passed

cargo test -p clawft-kernel --lib restricted_scope_cannot_publish_leaf_push_topic
cargo test -p clawft-kernel --lib all_scope_can_publish_leaf_push_topic
# ok

cargo test -p clawft-weave --lib ipc_publish_leaf_push_lands_on_chain
cargo test -p clawft-weave --lib ipc_publish_non_leaf_not_tagged_leaf_push
cargo test -p clawft-weave --lib ipc_publish_requires_write
# ok

cargo test -p clawft-weave --test agent_register_and_sign
cargo test -p clawft-weave --test ipc_subscribe_stream
# ok

scripts/build.sh check
# ok
```

## How to re-test (tester)

```bash
# From worktree on wave0h/weft-150-leaf-push-gov
scripts/build.sh test weftos-leaf-types
cargo test -p clawft-kernel --lib leaf_push
cargo test -p clawft-weave --lib ipc_publish
cargo test -p clawft-weave --test ipc_subscribe_stream
scripts/build.sh check
```

Live operator path (needs running daemon + leaf):

```bash
weaver leaf push --target <pubkey_hex> chord --freqs 440 --dry-run
# then without --dry-run; inspect chain for kind=ipc.publish leaf_push=true
weaver chain local | jq '.[] | select(.kind=="ipc.publish")'
```

## Follow-ups (out of S scope)

- Require actor_id on production (drop anonymous bring-up publish).
- Map A2A routing gate action to `ipc.publish` when target is Topic.
- Force `leaf_push` topic path through a dedicated RPC (`leaf.push`)
  if operators want a narrower Write grant than full `ipc.publish`.
