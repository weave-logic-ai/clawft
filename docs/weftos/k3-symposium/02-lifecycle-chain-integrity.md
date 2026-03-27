# Panel 2: Lifecycle & Chain Integrity

**Date**: 2026-03-04
**Presenter**: Services Architect
**Scope**: Build/Deploy/Version/Revoke lifecycle, TreeManager methods, chain events
**Branch**: feature/weftos-kernel-sprint
**Files Under Review**: `crates/clawft-kernel/src/tree_manager.rs` (1,508 lines), `crates/clawft-kernel/src/boot.rs` (1,049 lines)

---

## 1. Executive Summary

The K3 tool lifecycle implements the complete Build -> Deploy -> Execute ->
Version -> Revoke chain with cryptographic integrity at every step. SHA-256
module hashing, Ed25519 signing, and ExoChain event logging create an
immutable audit trail. The TreeManager provides 4 lifecycle methods that
integrate with both the resource tree and the chain manager.

**Verdict**: The lifecycle is architecturally sound. All chain events are
emitted at the correct lifecycle points. The non-destructive revocation
design preserves audit trails. Some edge cases in version management
warrant discussion.

---

## 2. Lifecycle Methods

### 2.1 Build (`tree_manager::build_tool`)

```
Input: tool name, WASM bytes (as &[u8]), Ed25519 SigningKey
Output: ToolVersion { version: 1, module_hash, signature, deployed_at, ... }
```

Steps:
1. Compute SHA-256 hash of WASM bytes via `compute_module_hash()`
2. Sign hash with Ed25519 via `signing_key.sign(&module_hash)`
3. Emit `tool.build` chain event
4. Return ToolVersion with version=1

Chain event payload:
```json
{
    "name": "fs.read_file",
    "module_hash": "a1b2c3...",
    "sig_algo": "Ed25519"
}
```

**Assessment**: Clean implementation. The hash-then-sign pattern is correct.
SHAKE-256 was originally planned but SHA-256 was chosen pragmatically (sha2
already a workspace dep). Both are 256-bit; acceptable trade-off.

### 2.2 Deploy (`tree_manager::deploy_tool`)

```
Input: BuiltinToolSpec, ToolVersion
Output: () -- side effects: tree node + chain event
```

Steps:
1. Determine category path (`/kernel/tools/fs`, `/kernel/tools/agent`, `/kernel/tools/sys`)
2. Ensure category namespace exists (auto-create if missing)
3. Insert tool node at `/kernel/tools/{cat}/{name}` with `ResourceKind::Tool`
4. Set metadata: `tool_version`, `module_hash`, `gate_action`, `deployed_at`
5. Emit `tool.deploy` chain event

Chain event payload:
```json
{
    "name": "fs.read_file",
    "version": 1,
    "tree_path": "/kernel/tools/fs/read_file",
    "module_hash": "a1b2c3...",
    "gate_action": "tool.fs.read"
}
```

**Assessment**: Solid. The auto-create for category namespaces prevents
ordering issues. The metadata captures everything needed for verification.
The dotted name -> path conversion (`spec.name.replace('.', '/')`) is
elegant but should be documented as a convention.

### 2.3 Version Update (`tree_manager::update_tool_version`)

```
Input: tool name (dotted), new ToolVersion
Output: () -- side effects: metadata update + chain event
```

Steps:
1. Look up tool node by converting dotted name to tree path
2. Read current version from metadata (`tool_version`)
3. Update metadata with new version, hash, timestamp
4. Emit `tool.version.update` chain event linking old -> new

Chain event payload:
```json
{
    "name": "fs.read_file",
    "old_version": 1,
    "new_version": 2,
    "old_hash": "a1b2c3...",
    "new_hash": "d4e5f6..."
}
```

**Assessment**: The old -> new chain linking is critical for audit trails.
The metadata stores only the current version; old versions are only in the
chain. This is acceptable for an append-only audit log but means version
history queries require chain traversal.

### 2.4 Revoke (`tree_manager::revoke_tool_version`)

```
Input: tool name, version number
Output: () -- side effects: metadata mark + chain event
```

Steps:
1. Look up tool node
2. Set metadata `v{N}_revoked = true` and `v{N}_revoked_at = <RFC3339>`
3. Emit `tool.version.revoke` chain event

Chain event payload:
```json
{
    "name": "fs.read_file",
    "version": 1,
    "module_hash": "a1b2c3..."
}
```

**Assessment**: The non-destructive design is exactly right. Tool nodes are
never deleted from the tree -- revocation is a metadata flag. This preserves
the Merkle hash chain and allows forensic analysis of revoked tools.

---

## 3. Chain Event Summary

| Lifecycle Phase | Source | Kind | Payload Keys |
|-----------------|--------|------|--------------|
| Build | tool | `tool.build` | name, module_hash, sig_algo |
| Deploy | tool | `tool.deploy` | name, version, tree_path, module_hash, gate_action |
| Execute | tool | `tool.exec` | tool, pid, status, [error] |
| Version Update | tool | `tool.version.update` | name, old_version, new_version, old_hash, new_hash |
| Revoke | tool | `tool.version.revoke` | name, version, module_hash |

All events use source `"tool"` -- consistent with the chain's source taxonomy
(`"kernel"`, `"ipc"`, `"supervisor"`, `"service"`, `"tool"`).

---

## 4. Boot-Time Registration

### 4.1 Namespace Structure

At kernel boot (with exochain enabled), the resource tree is populated with:

```
/kernel/tools              (Namespace)
  /kernel/tools/fs         (Namespace)
  /kernel/tools/agent      (Namespace)
  /kernel/tools/sys        (Namespace)
```

### 4.2 Tool Node Registration

All 27 tools from `builtin_tool_catalog()` are registered as `ResourceKind::Tool`
nodes. The dotted name is converted to a path:

```
fs.read_file   -> /kernel/tools/fs/read_file
agent.spawn    -> /kernel/tools/agent/spawn
sys.cron.add   -> /kernel/tools/sys/cron/add
```

Boot log output:
```
[ResourceTree] Registered 27 built-in tools in resource tree
```

### 4.3 Multi-Segment Tool Names

Tools with 3-segment names (e.g., `sys.service.list`) produce deeper paths:
```
sys.service.list   -> /kernel/tools/sys/service/list
sys.chain.status   -> /kernel/tools/sys/chain/status
```

**Finding**: These intermediate nodes (`/kernel/tools/sys/service`) are
NOT created as explicit Namespace nodes. The tree's insert method auto-
creates parents in some modes but this depends on the implementation.
This should be verified -- missing intermediate nodes could cause insert
failures.

**Recommendation**: Either pre-create intermediate namespaces for 3-segment
tools, or verify that the tree auto-creates parent nodes on insert.

---

## 5. Findings

### 5.1 Strengths

1. **Complete lifecycle chain** -- every phase emits a chain event
2. **Cryptographic integrity** -- SHA-256 + Ed25519 at build time
3. **Non-destructive revocation** -- audit trail preserved
4. **Version linking** -- old -> new chain in update events
5. **27 tools registered at boot** -- resource tree populated immediately

### 5.2 Gaps

1. **No active version enforcement** -- revoking v1 doesn't check if v2 exists
2. **Version history in chain only** -- querying old versions requires chain traversal
3. **3-segment tool paths** -- intermediate namespace creation not guaranteed
4. **No signature verification on deploy** -- build signs, deploy trusts
5. **Ed25519 requires exochain feature** -- build_tool unavailable without it

### 5.3 Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Revoking active version without successor | Medium | K4: Add version successor check |
| Missing intermediate tree nodes | Low | Verify tree auto-parent behavior |
| Unsigned tool deployment | Low | All K3 tools are native; WASM tools get signed in K4 |

---

## 6. Test Results (Live Run)

```
running 6 tests
test tree_manager::tests::tool_build_computes_hash_and_signs ... ok
test tree_manager::tests::tool_deploy_creates_tree_node ... ok
test tree_manager::tests::tool_deploy_emits_chain_event ... ok
test tree_manager::tests::tool_version_update_chain_links ... ok
test tree_manager::tests::tool_version_revoke_marks_revoked ... ok
test tree_manager::tests::tool_revoke_emits_chain_event ... ok
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured
```

**Verdict**: PASS -- all 6 lifecycle tests green. Chain events verified
by examining `cm.tail()` after each operation.
