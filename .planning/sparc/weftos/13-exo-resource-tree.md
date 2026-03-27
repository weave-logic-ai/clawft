# WeftOS Kernel: Exo-Resource-Tree -- Hierarchical Resource Namespace

```
ID:          W-KERNEL-13
Workstream:  W-KERNEL (WeftOS Kernel)
Title:       Exo-Resource-Tree -- Hierarchical Resource Namespace
Status:      Proposed
Date:        2026-02-28
Depends-On:  08-ephemeral-os-architecture.md, 10-agent-first-single-user.md,
             09-environments-and-learning.md, 11-local-inference-agents.md,
             12-networking-and-pairing.md, 07-ruvector-deep-integration.md
```

---

## 0. Implementation Status

### K0 (Implemented)
- Core types: ResourceId, ResourceKind, ResourceNode, Role, Action
- ResourceTree: CRUD, Merkle recomputation, namespace bootstrap
- MutationEvent: DAG-backed mutations with signatures
- Boot: from_checkpoint() and bootstrap_fresh()
- ServiceRegistry integration (tree nodes on register)
- CLI: weaver resource {tree, inspect, stats}
- **TreeManager facade**: Unified wrapper (`tree_manager.rs`) holding ResourceTree +
  MutationLog + ChainManager. Every tree mutation atomically produces a chain event
  and stores `chain_seq` metadata on the node for two-way traceability.
- **MutationLog wired**: MutationLog now receives events on every tree operation
  through TreeManager (previously created but never connected to tree operations).
- **Boot-to-chain logging**: All boot phases emit chain events; tree bootstrap and
  service registration go through TreeManager.
- **Chain integrity verification**: `ChainManager::verify_integrity()` walks all
  events, recomputes hashes, verifies linkage. Available via `weaver chain verify`.
- **Shutdown checkpoint**: Kernel shutdown emits `kernel.shutdown` chain event with
  tree_root_hash + chain_seq, then creates a chain checkpoint.

### K1 (Deferred — stubs only in K0)
- Permission engine: check() with delegation shortcut (Section 5)
- EffectiveAclCache (Section 5.2)
- DelegationCert lifecycle: grant/revoke/prune (Section 9)
- CapabilityChecker delegation to tree.check() (Section 7.2)
- CLI: weaver resource {grant, revoke, check}

### K2+ (Not started)
- IPC topic tree nodes (Section 7.3)
- WASM sandbox gating (Section 7.4)
- Container tree nodes (Section 7.5)
- App subtrees (Section 7.6)
- Environment subtrees (Section 7.7)

---

## 1. Core Insight: Everything Is a Resource Node

WeftOS has agents, services, processes, IPC topics, files, capabilities,
environments, sessions, network peers, GGUF models, and applications. Each is
governed by its own subsystem with its own access control checks, its own
namespace conventions, and its own audit path. The result is a scattered
permission model: K1 checks capabilities via `CapabilityChecker`, K2 checks IPC
scope via `IpcScope`, K3 checks sandbox policies via `SandboxPolicy`, K5 checks
app rules via `weftapp.toml`, K6 checks governance via `EffectVector` and the
CGR engine, and doc 12 checks network trust via pairing bonds.

The resource tree eliminates this fragmentation. **Everything in WeftOS maps to
a node in a single, content-addressed, cryptographically verifiable tree.** The
tree IS the OS namespace. Permission checks walk up the tree collecting policies.
Mutations are signed DAG events. Subtree Merkle roots enable fast verification.
The tree is built on ExoChain's cryptographic substrate -- BLAKE3 hashing,
Ed25519 signing, Hybrid Logical Clocks, DIDs, and bailment policies -- so every
node inherits ExoChain's auditability and consent model for free.

**Why this matters:**

1. **Unified access control.** One algorithm -- walk up the tree collecting
   policies, check delegation certs, evaluate against `exo_consent` -- replaces
   six separate access control paths across K0-K6.

2. **Content-addressed identity.** Every resource has a `ResourceId` derived from
   `Blake3Hash`. No name collisions. No ambiguous references. A PID, a topic
   name, and a file hash all become resource IDs in the same namespace.

3. **Cryptographic audit.** Every mutation (node creation, policy change,
   delegation grant/revoke) is a signed DAG event. Subtree Merkle roots allow
   fast verification of entire subtrees without walking every node.

4. **Agentic delegation.** Agents hold DIDs. The tree stores delegation
   certificates that grant time-bounded, role-scoped access. When a supervisor
   spawns a sub-agent, it signs a `DelegationCert` granting the sub-agent
   access to the resources it needs. No manual RBAC configuration.

5. **Namespace composability.** Applications mount their subtrees under
   `/apps/<name>/`. Environments scope under `/environments/<env>/`. Network
   peers appear under `/network/peers/`. The tree grows organically as the OS
   evolves, without schema migrations or namespace conflicts.

---

## 2. Crate Design: exo-resource-tree

### 2.1 Cargo.toml

```toml
[package]
name = "exo-resource-tree"
version = "0.1.0"
edition = "2021"

[dependencies]
exo-core = { path = "../exo-core" }
exo-identity = { path = "../exo-identity" }
exo-consent = { path = "../exo-consent" }
exo-dag = { path = "../exo-dag" }
serde = { version = "1.0", features = ["derive"] }
thiserror = "2"

[features]
default = []
cedar = ["dep:cedar-policy"]

[dependencies.cedar-policy]
version = "4"
optional = true
```

### 2.2 Crate Structure

```
exo-resource-tree/
  Cargo.toml
  src/
    lib.rs              -- crate root, re-exports
    model.rs            -- ResourceId, ResourceKind, ResourceNode, DelegationCert, Role
    tree.rs             -- ResourceTree struct, CRUD, Merkle recomputation
    permission.rs       -- Permission engine, check algorithm, EffectiveAclCache
    boot.rs             -- Boot-time tree construction from DAG checkpoint
    mutation.rs         -- Signed mutation events, DAG append
    cedar.rs            -- Optional Cedar policy integration (feature-gated)
    error.rs            -- Error types
```

### 2.3 LOC Target

Core logic (model + tree + permission): <400 LOC. Boot, mutation, cedar, and
tests add ~200 LOC each. Total crate including tests: ~1200 LOC.

---

## 3. Core Data Model

These types are the foundation of the resource tree. They are defined in
`src/model.rs` and re-exported from `src/lib.rs`.

### 3.1 ResourceId

```rust
use exo_core::{Blake3Hash, Signature, HLC};
use exo_identity::Did;
use exo_consent::{BailmentPolicy, ConsentProof};

/// Content-addressed resource identifier.
/// Derived from BLAKE3 hash of the resource's canonical representation.
/// Collision-free by construction (256-bit hash space).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ResourceId(pub Blake3Hash);

impl ResourceId {
    /// Create a ResourceId from a canonical byte representation.
    pub fn from_content(content: &[u8]) -> Self {
        Self(exo_core::blake3_hash(content))
    }

    /// Create a ResourceId from a well-known path string.
    /// Uses domain separator for deterministic, cross-node stable IDs.
    pub fn from_path(path: &str) -> Self {
        Self(exo_core::blake3_hash_with_domain(
            b"WEFTOS-RESOURCE-PATH-v1",
            path.as_bytes(),
        ))
    }
}
```

### 3.2 ResourceKind

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ResourceKind {
    Node {
        name: String,
        primary_agent: Did,
        group_members: Vec<Did>,
    },
    Leaf {
        payload_cid: Option<String>,
        owner_agent: Did,
    },
}

impl ResourceKind {
    pub fn primary_did(&self) -> &Did {
        match self {
            ResourceKind::Node { primary_agent, .. } => primary_agent,
            ResourceKind::Leaf { owner_agent, .. } => owner_agent,
        }
    }

    pub fn has_direct_access(&self, did: &Did) -> bool {
        match self {
            ResourceKind::Node { primary_agent, group_members, .. } => {
                primary_agent == did || group_members.contains(did)
            }
            ResourceKind::Leaf { owner_agent, .. } => owner_agent == did,
        }
    }
}
```

### 3.3 ResourceNode

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceNode {
    id: ResourceId,
    parent_id: Option<ResourceId>,
    kind: ResourceKind,
    hlc: HLC,
    rbac_policies: Vec<BailmentPolicy>,
    delegation_certs: Vec<DelegationCert>,
    signature: Signature,
    subtree_merkle_root: Blake3Hash,
}

impl ResourceNode {
    pub fn id(&self) -> &ResourceId { &self.id }
    pub fn parent_id(&self) -> Option<&ResourceId> { self.parent_id.as_ref() }
    pub fn kind(&self) -> &ResourceKind { &self.kind }
    pub fn hlc(&self) -> &HLC { &self.hlc }
    pub fn delegation_certs(&self) -> &[DelegationCert] { &self.delegation_certs }
    pub fn rbac_policies(&self) -> &[BailmentPolicy] { &self.rbac_policies }
    pub fn subtree_merkle_root(&self) -> &Blake3Hash { &self.subtree_merkle_root }

    /// Verify the Ed25519 signature against the primary agent's public key.
    pub fn verify_signature(&self, public_key: &[u8; 32]) -> bool {
        let canonical = self.canonical_bytes();
        exo_core::ed25519_verify(public_key, &canonical, &self.signature)
    }

    /// Canonical byte representation for signing (excludes signature field).
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.id.0);
        if let Some(ref pid) = self.parent_id { buf.extend_from_slice(&pid.0); }
        buf.extend_from_slice(&serde_json::to_vec(&self.kind).expect("serializable"));
        buf.extend_from_slice(&self.hlc.to_bytes());
        buf.extend_from_slice(&self.subtree_merkle_root);
        buf
    }
}
```

### 3.4 DelegationCert

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DelegationCert {
    delegate: Did,
    role: Role,
    expires_hlc: HLC,
    proof: ConsentProof,
}

impl DelegationCert {
    pub fn expired(&self, current_hlc: &HLC) -> bool {
        self.expires_hlc < *current_hlc
    }

    pub fn is_for(&self, did: &Did) -> bool { &self.delegate == did }

    pub fn allows(&self, action: &Action) -> bool { self.role.allows(action) }
}
```

### 3.5 Role

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Role {
    Owner,
    Admin,
    Operator,
    Viewer,
    Custom(String),
}

impl Role {
    pub fn allows(&self, action: &Action) -> bool {
        match self {
            Role::Owner => true,
            Role::Admin => !matches!(action, Action::TransferOwnership),
            Role::Operator => matches!(
                action, Action::Read | Action::Create | Action::Update | Action::Execute
            ),
            Role::Viewer => matches!(action, Action::Read),
            Role::Custom(_) => false, // resolved via full policy evaluation
        }
    }

    pub fn privilege_level(&self) -> u8 {
        match self { Role::Owner => 4, Role::Admin => 3, Role::Operator => 2,
                     Role::Viewer => 1, Role::Custom(_) => 0 }
    }
}
```

### 3.6 Action

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Action {
    Read, Create, Update, Delete, Execute, Delegate, TransferOwnership, Admin,
}
```

---

## 4. ResourceTree Implementation

The `ResourceTree` struct is the in-memory representation of the entire resource
namespace. Built at boot time from an ExoChain DAG checkpoint, maintained in
memory throughout the kernel's lifetime. Mutations are applied in-memory and
simultaneously appended to the DAG for persistence.

### 4.1 ResourceTree Struct

```rust
use std::collections::HashMap;

pub struct ResourceTree {
    /// Primary index: ResourceId -> ResourceNode.
    nodes: HashMap<ResourceId, ResourceNode>,
    /// Parent-to-children index (sorted for deterministic Merkle).
    children: HashMap<ResourceId, Vec<ResourceId>>,
    /// The root node's ResourceId.
    root_id: ResourceId,
    /// ExoChain DAG for persisting mutations.
    dag: Arc<exo_dag::DagStore>,
    /// HLC clock for timestamping mutations.
    clock: Arc<exo_core::HybridLogicalClock>,
    /// ACL evaluation cache for hot-path permission checks.
    acl_cache: EffectiveAclCache,
}
```

### 4.2 Boot-Time Construction

```rust
impl ResourceTree {
    /// Construct from an ExoChain DAG checkpoint. Called during kernel boot
    /// (K0) before any agents spawn.
    pub fn from_checkpoint(
        dag: Arc<exo_dag::DagStore>,
        root_event_id: &exo_core::EventId,
        clock: Arc<exo_core::HybridLogicalClock>,
        identity_resolver: &dyn IdentityResolver,
    ) -> Result<Self, ResourceTreeError> {
        let checkpoint = exo_dag::load_checkpoint(dag.as_ref(), root_event_id)?;
        let mut nodes = HashMap::new();
        let mut children: HashMap<ResourceId, Vec<ResourceId>> = HashMap::new();
        let mut root_id = None;

        for event in checkpoint.events_in_order() {
            let node: ResourceNode = serde_json::from_slice(event.payload())?;
            let pub_key = identity_resolver.resolve_public_key(node.kind().primary_did())?;
            if !node.verify_signature(&pub_key) {
                return Err(ResourceTreeError::InvalidSignature {
                    node_id: node.id().clone(), signer: node.kind().primary_did().clone(),
                });
            }
            if let Some(parent_id) = node.parent_id() {
                children.entry(parent_id.clone()).or_default().push(node.id().clone());
            } else {
                root_id = Some(node.id().clone());
            }
            nodes.insert(node.id().clone(), node);
        }

        let root_id = root_id.ok_or(ResourceTreeError::NoRootNode)?;
        for child_list in children.values_mut() { child_list.sort(); }

        let mut tree = Self {
            nodes, children, root_id, dag, clock,
            acl_cache: EffectiveAclCache::new(1024),
        };
        tree.verify_all_merkle_roots()?;
        Ok(tree)
    }

    /// Bootstrap a fresh tree with the default WeftOS namespace structure.
    pub fn bootstrap_fresh(
        dag: Arc<exo_dag::DagStore>,
        clock: Arc<exo_core::HybridLogicalClock>,
        kernel_did: &Did,
        signing_key: &exo_core::SigningKey,
    ) -> Result<Self, ResourceTreeError> {
        let mut tree = Self {
            nodes: HashMap::new(), children: HashMap::new(),
            root_id: ResourceId::from_path("/"), dag, clock,
            acl_cache: EffectiveAclCache::new(1024),
        };
        // Create root and all top-level namespace nodes (see Section 6)
        for path in &[
            "/", "/kernel", "/kernel/services", "/kernel/processes", "/kernel/ipc",
            "/agents", "/apps", "/network", "/network/peers", "/network/sessions",
            "/environments", "/models", "/storage", "/storage/ephemeral",
            "/storage/persistent",
        ] {
            tree.create_namespace_node(path, kernel_did, signing_key)?;
        }
        Ok(tree)
    }
}
```

### 4.3 Tree CRUD Operations

```rust
impl ResourceTree {
    /// Insert a new resource node as a child of `parent_id`.
    /// Validates parent is a Node, signs the node, inserts into indexes,
    /// recomputes Merkle roots to root, appends DAG event, invalidates cache.
    pub fn insert(
        &mut self, parent_id: &ResourceId, kind: ResourceKind,
        rbac_policies: Vec<BailmentPolicy>, signing_key: &exo_core::SigningKey,
    ) -> Result<ResourceId, ResourceTreeError> {
        let parent = self.nodes.get(parent_id)
            .ok_or(ResourceTreeError::NodeNotFound(parent_id.clone()))?;
        if matches!(parent.kind(), ResourceKind::Leaf { .. }) {
            return Err(ResourceTreeError::CannotAddChildToLeaf(parent_id.clone()));
        }
        let hlc = self.clock.now();
        let id = ResourceId::from_content(
            &serde_json::to_vec(&(&kind, &hlc, parent_id)).expect("serializable"),
        );
        let mut node = ResourceNode {
            id: id.clone(), parent_id: Some(parent_id.clone()), kind, hlc,
            rbac_policies, delegation_certs: Vec::new(),
            signature: Signature::default(),
            subtree_merkle_root: exo_core::BLAKE3_EMPTY,
        };
        node.signature = exo_core::ed25519_sign(signing_key, &node.canonical_bytes());
        self.nodes.insert(id.clone(), node.clone());
        self.children.entry(parent_id.clone()).or_default().push(id.clone());
        if let Some(c) = self.children.get_mut(parent_id) { c.sort(); }
        self.recompute_merkle_chain(parent_id)?;
        self.dag.append_event(b"WEFTOS-RESOURCE-MUTATION-v1",
            &serde_json::to_vec(&MutationEvent::Insert { node })?, signing_key)?;
        self.acl_cache.invalidate_path(&id);
        Ok(id)
    }

    /// Remove a resource node and all its descendants.
    pub fn remove(
        &mut self, node_id: &ResourceId, signing_key: &exo_core::SigningKey,
    ) -> Result<Vec<ResourceNode>, ResourceTreeError> {
        if node_id == &self.root_id { return Err(ResourceTreeError::CannotRemoveRoot); }
        let parent_id = self.nodes.get(node_id)
            .ok_or(ResourceTreeError::NodeNotFound(node_id.clone()))?
            .parent_id().ok_or(ResourceTreeError::CannotRemoveRoot)?.clone();

        let mut to_remove = Vec::new();
        self.collect_descendants(node_id, &mut to_remove);
        to_remove.push(node_id.clone());

        let mut removed = Vec::new();
        for id in &to_remove {
            if let Some(n) = self.nodes.remove(id) { removed.push(n); }
            self.children.remove(id);
            self.acl_cache.invalidate_path(id);
        }
        if let Some(siblings) = self.children.get_mut(&parent_id) {
            siblings.retain(|id| id != node_id);
        }
        self.recompute_merkle_chain(&parent_id)?;
        self.dag.append_event(b"WEFTOS-RESOURCE-MUTATION-v1",
            &serde_json::to_vec(&MutationEvent::Remove {
                node_id: node_id.clone(), descendants_removed: to_remove.len(),
            })?, signing_key)?;
        Ok(removed)
    }

    pub fn get(&self, node_id: &ResourceId) -> Option<&ResourceNode> {
        self.nodes.get(node_id)
    }

    pub fn children_of(&self, node_id: &ResourceId) -> Vec<&ResourceNode> {
        self.children.get(node_id)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn ancestors(&self, node_id: &ResourceId) -> Vec<&ResourceNode> {
        let mut path = Vec::new();
        let mut current = node_id.clone();
        while let Some(node) = self.nodes.get(&current) {
            path.push(node);
            match node.parent_id() { Some(p) => current = p.clone(), None => break }
        }
        path
    }

    fn collect_descendants(&self, node_id: &ResourceId, out: &mut Vec<ResourceId>) {
        if let Some(children) = self.children.get(node_id) {
            for child_id in children {
                out.push(child_id.clone());
                self.collect_descendants(child_id, out);
            }
        }
    }
}
```

### 4.4 Merkle Root Recomputation

```rust
impl ResourceTree {
    /// Recompute subtree Merkle roots from `start_id` up to root. O(depth).
    fn recompute_merkle_chain(&mut self, start_id: &ResourceId) -> Result<(), ResourceTreeError> {
        let mut current = start_id.clone();
        loop {
            let new_root = self.compute_merkle_root(&current);
            if let Some(node) = self.nodes.get_mut(&current) {
                node.subtree_merkle_root = new_root;
                match node.parent_id() { Some(p) => current = p.clone(), None => break }
            } else { break; }
        }
        Ok(())
    }

    fn compute_merkle_root(&self, node_id: &ResourceId) -> Blake3Hash {
        match self.children.get(node_id) {
            Some(children) if !children.is_empty() => {
                let mut hasher = exo_core::Blake3Hasher::new();
                hasher.update(b"WEFTOS-MERKLE-SUBTREE-v1");
                for child_id in children { hasher.update(&child_id.0); }
                hasher.finalize()
            }
            _ => exo_core::BLAKE3_EMPTY,
        }
    }

    fn verify_all_merkle_roots(&self) -> Result<(), ResourceTreeError> {
        for (node_id, node) in &self.nodes {
            let expected = self.compute_merkle_root(node_id);
            if expected != node.subtree_merkle_root {
                return Err(ResourceTreeError::MerkleRootMismatch {
                    node_id: node_id.clone(), expected, actual: node.subtree_merkle_root.clone(),
                });
            }
        }
        Ok(())
    }
}
```

### 4.5 Mutation Events

```rust
/// Events appended to the ExoChain DAG on every tree mutation.
/// Domain separator: "WEFTOS-RESOURCE-MUTATION-v1" (exo-core pattern).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MutationEvent {
    Insert { node: ResourceNode },
    Remove { node_id: ResourceId, descendants_removed: usize },
    PolicyUpdate { node_id: ResourceId, old_policies: Vec<BailmentPolicy>, new_policies: Vec<BailmentPolicy> },
    DelegationGrant { node_id: ResourceId, cert: DelegationCert },
    DelegationRevoke { node_id: ResourceId, delegate: Did },
}
```

---

## 5. Permission Engine

The permission engine is the hot path. Every agent action routes through a
single `check()` call that walks from the target resource up to the root,
collecting policies and checking delegation certificates.

### 5.1 Permission Check Algorithm

```rust
#[derive(Debug, Clone)]
pub enum Decision {
    Allow,
    Deny { reason: String },
    Defer { reason: String },  // maps to cognitum-gate Defer (doc 07 K1)
}

impl ResourceTree {
    /// Check whether `principal` (DID) may perform `action` on `resource_id`.
    ///
    /// Hot path, O(log n) with cache:
    /// 1. Check ACL cache. Return cached decision if present.
    /// 2. Walk from resource_id to root. At each node:
    ///    a. Delegation shortcut: if principal has active, non-expired cert
    ///       with role allowing action, return Allow immediately.
    ///    b. Collect RBAC policies (most-specific-first).
    /// 3. Evaluate collected policies via exo_consent::evaluate().
    /// 4. Cache the result.
    pub fn check(
        &self, principal: &Did, resource_id: &ResourceId, action: Action,
    ) -> Result<Decision, ResourceTreeError> {
        if let Some(cached) = self.acl_cache.get(principal, resource_id, &action) {
            return Ok(cached);
        }

        let current_hlc = self.clock.now();
        let mut current_id = resource_id.clone();
        let mut collected_policies: Vec<BailmentPolicy> = Vec::new();

        while let Some(node) = self.nodes.get(&current_id) {
            // Delegation cert shortcut (O(1) per node)
            if let Some(_cert) = node.delegation_certs.iter().find(|c| {
                c.is_for(principal) && c.allows(&action) && !c.expired(&current_hlc)
            }) {
                let decision = Decision::Allow;
                self.acl_cache.put(principal.clone(), resource_id.clone(), action, decision.clone());
                return Ok(decision);
            }
            collected_policies.extend(node.rbac_policies.iter().cloned());
            match node.parent_id() { Some(p) => current_id = p.clone(), None => break }
        }

        let risk_score = self.risk_score(principal);
        let decision = exo_consent::evaluate(
            &collected_policies, principal, &action_to_consent_action(&action), &risk_score,
        );
        let result = match decision {
            exo_consent::Decision::Allow => Decision::Allow,
            exo_consent::Decision::Deny(r) => Decision::Deny { reason: r },
            exo_consent::Decision::Defer(r) => Decision::Defer { reason: r },
        };
        self.acl_cache.put(principal.clone(), resource_id.clone(), action, result.clone());
        Ok(result)
    }

    fn risk_score(&self, _principal: &Did) -> exo_identity::RiskScore {
        exo_identity::RiskScore { score: 50, confidence_bps: 5000 }
    }
}
```

### 5.2 EffectiveAclCache

```rust
/// LRU cache for effective ACL decisions.
/// Key: (principal DID, resource ID, action). Value: Decision.
/// Invalidated on node mutations, delegation changes, and policy updates.
pub struct EffectiveAclCache {
    entries: HashMap<(Did, ResourceId, Action), Decision>,
    max_size: usize,
    hits: u64,
    misses: u64,
}

impl EffectiveAclCache {
    pub fn new(max_size: usize) -> Self {
        Self { entries: HashMap::new(), max_size, hits: 0, misses: 0 }
    }
    pub fn get(&self, principal: &Did, resource_id: &ResourceId, action: &Action) -> Option<Decision> {
        self.entries.get(&(principal.clone(), resource_id.clone(), *action)).cloned()
    }
    pub fn put(&mut self, principal: Did, resource_id: ResourceId, action: Action, decision: Decision) {
        if self.entries.len() >= self.max_size { self.entries.clear(); }
        self.entries.insert((principal, resource_id, action), decision);
    }
    pub fn invalidate_path(&mut self, resource_id: &ResourceId) {
        self.entries.retain(|(_, rid, _), _| rid != resource_id);
    }
    pub fn invalidate_principal(&mut self, principal: &Did) {
        self.entries.retain(|(did, _, _), _| did != principal);
    }
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 { 0.0 } else { self.hits as f64 / total as f64 }
    }
}
```

### 5.3 Delegation Management

```rust
impl ResourceTree {
    /// Grant a delegation certificate. Grantor must have Delegate action.
    /// Creates ConsentProof via exo_consent for GDPR-compliant auditing.
    pub fn grant_delegation(
        &mut self, resource_id: &ResourceId, delegate: Did, role: Role,
        ttl: std::time::Duration, grantor_signing_key: &exo_core::SigningKey,
    ) -> Result<DelegationCert, ResourceTreeError> {
        let node = self.nodes.get_mut(resource_id)
            .ok_or(ResourceTreeError::NodeNotFound(resource_id.clone()))?;
        let expires_hlc = self.clock.now().advance_by(ttl);
        let proof = exo_consent::create_consent_proof(
            node.kind().primary_did(), &delegate,
            &format!("delegation:{}", role.privilege_level()), grantor_signing_key,
        )?;
        let cert = DelegationCert { delegate: delegate.clone(), role, expires_hlc, proof };
        node.delegation_certs.push(cert.clone());
        node.signature = exo_core::ed25519_sign(grantor_signing_key, &node.canonical_bytes());
        self.dag.append_event(b"WEFTOS-RESOURCE-MUTATION-v1",
            &serde_json::to_vec(&MutationEvent::DelegationGrant {
                node_id: resource_id.clone(), cert: cert.clone(),
            })?, grantor_signing_key)?;
        self.acl_cache.invalidate_principal(&delegate);
        Ok(cert)
    }

    /// Revoke a delegation certificate for a delegate DID.
    pub fn revoke_delegation(
        &mut self, resource_id: &ResourceId, delegate: &Did,
        revoker_signing_key: &exo_core::SigningKey,
    ) -> Result<(), ResourceTreeError> {
        let node = self.nodes.get_mut(resource_id)
            .ok_or(ResourceTreeError::NodeNotFound(resource_id.clone()))?;
        let before = node.delegation_certs.len();
        node.delegation_certs.retain(|c| !c.is_for(delegate));
        if node.delegation_certs.len() == before {
            return Err(ResourceTreeError::DelegationNotFound {
                node_id: resource_id.clone(), delegate: delegate.clone(),
            });
        }
        node.signature = exo_core::ed25519_sign(revoker_signing_key, &node.canonical_bytes());
        self.dag.append_event(b"WEFTOS-RESOURCE-MUTATION-v1",
            &serde_json::to_vec(&MutationEvent::DelegationRevoke {
                node_id: resource_id.clone(), delegate: delegate.clone(),
            })?, revoker_signing_key)?;
        self.acl_cache.invalidate_principal(delegate);
        Ok(())
    }

    /// Prune all expired delegation certificates. Called by health system.
    pub fn prune_expired_certs(&mut self) -> usize {
        let current_hlc = self.clock.now();
        let mut pruned = 0;
        for node in self.nodes.values_mut() {
            let before = node.delegation_certs.len();
            node.delegation_certs.retain(|c| !c.expired(&current_hlc));
            pruned += before - node.delegation_certs.len();
        }
        if pruned > 0 { self.acl_cache = EffectiveAclCache::new(self.acl_cache.max_size); }
        pruned
    }
}
```

---

## 6. WeftOS Resource Mapping

### 6.1 Full Tree Diagram

```
/ (root)                        ResourceKind::Node -- primary_agent: kernel-did
+-- kernel/                     ResourceKind::Node -- primary_agent: kernel-did
|   +-- services/               ResourceKind::Node -- service registry (K0)
|   |   +-- cron                ResourceKind::Leaf -- CronService
|   |   +-- message-bus         ResourceKind::Leaf -- MessageBus
|   |   +-- inference           ResourceKind::Leaf -- InferenceService
|   +-- processes/              ResourceKind::Node -- process table (K1)
|   |   +-- pid:1               ResourceKind::Leaf -- agent process
|   |   +-- pid:2               ResourceKind::Leaf -- agent process
|   +-- ipc/                    ResourceKind::Node -- IPC topics (K2)
|   |   +-- agent.lifecycle     ResourceKind::Leaf -- topic
|   |   +-- model.inference     ResourceKind::Leaf -- topic
|   +-- containers/             ResourceKind::Node -- container registry (K4)
|       +-- rvf:abc123          ResourceKind::Leaf -- RVF container
+-- agents/                     ResourceKind::Node -- agent registry
|   +-- inference-service       ResourceKind::Leaf -- system agent
|   +-- network-service         ResourceKind::Leaf -- system agent
|   +-- user-agent-1            ResourceKind::Leaf -- spawned agent
+-- apps/                       ResourceKind::Node -- installed applications (K5)
|   +-- my-app/                 ResourceKind::Node -- with app manifest
|   |   +-- config              ResourceKind::Leaf
|   |   +-- agents/             ResourceKind::Node -- app-scoped agents
|   |       +-- worker-1        ResourceKind::Leaf
+-- network/                    ResourceKind::Node -- network peers (doc 12)
|   +-- peers/                  ResourceKind::Node
|   |   +-- did:exo:abc123      ResourceKind::Leaf -- paired peer
|   |   +-- did:exo:def456      ResourceKind::Leaf -- bonded peer
|   +-- sessions/               ResourceKind::Node -- active sessions
|       +-- cli-session-1       ResourceKind::Leaf -- CLI session
|       +-- web-session-2       ResourceKind::Leaf -- Web UI session
+-- environments/               ResourceKind::Node -- environment scoping (doc 09)
|   +-- dev/                    ResourceKind::Node -- low-risk env
|   +-- staging/                ResourceKind::Node -- quality-gated
|   +-- production/             ResourceKind::Node -- restricted
+-- models/                     ResourceKind::Node -- GGUF model registry (doc 11)
|   +-- ruvltra-0.5b            ResourceKind::Leaf -- model file
|   +-- router-fastgrnn         ResourceKind::Leaf -- routing model
+-- storage/                    ResourceKind::Node -- filesystem (doc 08)
    +-- ephemeral/              ResourceKind::Node
    +-- persistent/             ResourceKind::Node
```

### 6.2 Concept-to-Tree Mapping Table

| WeftOS Concept | Source Doc | ResourceKind | Parent Path | RBAC Policy |
|---|---|---|---|---|
| Kernel | K0 (01) | Node | `/` | Owner: kernel-did |
| SystemService | K0 (01) | Leaf | `/kernel/services/` | Viewer: all; Operator: service-agent |
| ProcessEntry | K1 (02) | Leaf | `/kernel/processes/` | Owner: spawning agent |
| IPC Topic | K2 (03) | Leaf | `/kernel/ipc/` | Publish/Subscribe per IpcScope |
| WASM Tool | K3 (04) | Leaf | `/kernel/services/<tool>` | Execute: agents with capability |
| Container | K4 (05) | Leaf | `/kernel/containers/` | Owner: deployment agent |
| AppManifest | K5 (06) | Node | `/apps/<name>/` | Admin: app owner |
| Agent | K1 + doc 10 | Leaf | `/agents/` | Owner: agent DID |
| Network Peer | doc 12 | Leaf | `/network/peers/` | Viewer: all; Operator: network-service |
| Session | doc 12 | Leaf | `/network/sessions/` | Owner: session creator |
| Environment | doc 09 | Node | `/environments/` | Admin: governance-agent |
| GGUF Model | doc 11 | Leaf | `/models/` | Execute: inference-service |
| File Entry | doc 08 | Leaf | `/storage/*/` | Policy from Gatekeeper |

### 6.3 Well-Known ResourceIds

```rust
lazy_static! {
    static ref ROOT_ID: ResourceId = ResourceId::from_path("/");
    static ref KERNEL_ID: ResourceId = ResourceId::from_path("/kernel");
    static ref SERVICES_ID: ResourceId = ResourceId::from_path("/kernel/services");
    static ref PROCESSES_ID: ResourceId = ResourceId::from_path("/kernel/processes");
    static ref IPC_ID: ResourceId = ResourceId::from_path("/kernel/ipc");
    static ref AGENTS_ID: ResourceId = ResourceId::from_path("/agents");
    static ref APPS_ID: ResourceId = ResourceId::from_path("/apps");
    static ref NETWORK_ID: ResourceId = ResourceId::from_path("/network");
    static ref ENVIRONMENTS_ID: ResourceId = ResourceId::from_path("/environments");
    static ref MODELS_ID: ResourceId = ResourceId::from_path("/models");
    static ref STORAGE_ID: ResourceId = ResourceId::from_path("/storage");
}
```

---

## 7. Integration with Kernel Phases

### 7.1 K0: Service Registry as Tree Nodes

Each `SystemService` registered via `ServiceRegistry` (doc 01) creates a leaf
node under `/kernel/services/`. The `HealthSystem` verifies tree node signatures.

```rust
impl ServiceRegistry {
    pub fn register_with_tree(
        &self, service: Arc<dyn SystemService>, tree: &mut ResourceTree,
        kernel_signing_key: &exo_core::SigningKey,
    ) -> Result<ResourceId> {
        self.register(service.clone())?;
        tree.insert(&SERVICES_ID, ResourceKind::Leaf {
            payload_cid: None, owner_agent: kernel_did(),
        }, vec![BailmentPolicy::viewer_all()], kernel_signing_key)
    }
}
```

### 7.2 K1: Capabilities as Tree Policies

`CapabilityChecker::check_tool_access()` (doc 02) delegates to
`tree.check(agent_did, tool_resource_id, Action::Execute)`. The
`AgentCapabilities` struct maps to `BailmentPolicy` entries on the agent's
resource nodes. `AgentSupervisor::spawn()` creates a process leaf and grants
the agent a DelegationCert on its own process node.

```rust
// Capability check via tree replaces manual per-field checking
fn check_tool_access(&self, agent_did: &Did, tool_name: &str, tree: &ResourceTree) -> Result<Decision> {
    let tool_resource_id = ResourceId::from_path(&format!("/kernel/services/{}", tool_name));
    tree.check(agent_did, &tool_resource_id, Action::Execute)
}
```

### 7.3 K2: IPC Topics as Tree Nodes

Each IPC topic (doc 03, `TopicRouter`) becomes a leaf under `/kernel/ipc/`.
Publish and subscribe permissions are checked via the tree. Messages can be
addressed by ResourceId in the distributed case (doc 08).

```rust
// IPC send: permission check via tree before delivery
let target_resource = match &msg.to {
    MessageTarget::Topic(topic) => ResourceId::from_path(&format!("/kernel/ipc/{}", topic)),
    MessageTarget::Service(name) => ResourceId::from_path(&format!("/kernel/services/{}", name)),
    MessageTarget::Pid(pid) => ResourceId::from_path(&format!("/kernel/processes/pid:{}", pid)),
    MessageTarget::Broadcast => KERNEL_ID.clone(),
};
match tree.check(from_did, &target_resource, Action::Execute)? {
    Decision::Allow => { /* proceed */ }
    Decision::Deny { reason } => return Err(IpcError::Denied(reason)),
    Decision::Defer { reason } => return Err(IpcError::Deferred(reason)),
}
```

### 7.4 K3: WASM Sandbox Policy as BailmentPolicy

`WasmSandboxConfig` and `SandboxPolicy` (doc 04) are stored as `BailmentPolicy`
entries on the agent's leaf node. WASM tool execution is gated by
`tree.check(agent_did, tool_resource_id, Action::Execute)`.

### 7.5 K4: Containers as Tree Nodes

RVF and Docker containers (doc 05, doc 07 K4) appear as leaves under
`/kernel/containers/`. Container lifecycle operations are permission-checked
via the tree.

### 7.6 K5: App Manifest as Subtree

App installation (doc 06, `AppManager::install()`) creates a subtree under
`/apps/<name>/` with sub-nodes for config, agents, and data. The
`weftapp.toml` `[rules]` section maps to `BailmentPolicy`. App agents get
scoped `DelegationCert` entries on install.

### 7.7 K6: Environment Scoping via Subtrees

Environments (doc 09) are subtrees under `/environments/<env>/`. Each
carries environment-specific `BailmentPolicy` with scoped risk thresholds.
Production environments modulate the permission check to Defer on risk > 0.3
(effect algebra from doc 08, section 5.3).

---

## 8. Boot Sequence Integration

### 8.1 Boot Order

```
1. Kernel boot begins (K0, doc 01)
2. Load ExoChain DAG checkpoint
   exo_dag::load_checkpoint(root_event_id)
   If no checkpoint: initialize fresh DAG
3. Build resource tree from checkpoint
   ResourceTree::from_checkpoint(dag, root_event_id, clock, identity_resolver)
   OR on first boot: ResourceTree::bootstrap_fresh(dag, clock, kernel_did, signing_key)
4. Tree available in Arc<RwLock<ResourceTree>>
   Shared with all subsystems
5. Service registry initialization (K0) -- each service creates tree node
6. Root agent spawned (doc 10) -- gets Owner cert on / (root)
7. Service agents boot (doc 10) -- appear under /agents/ and /kernel/services/
8. Network service starts (doc 12) -- peers appear under /network/peers/
9. Boot complete. KernelState -> Running. Tree fully populated.
```

### 8.2 Checkpoint Save/Load

```rust
impl ResourceTree {
    pub fn save_checkpoint(&self, signing_key: &exo_core::SigningKey) -> Result<exo_core::EventId> {
        let nodes_bytes: Vec<Vec<u8>> = self.nodes.values()
            .map(|node| serde_json::to_vec(node).expect("serializable")).collect();
        exo_dag::create_checkpoint(self.dag.as_ref(), &nodes_bytes, signing_key)
    }
}
```

### 8.3 Recovery

If checkpoint loading fails (signature error, Merkle mismatch), the kernel
tries older checkpoints in reverse chronological order. If all fail, it performs
a full DAG replay: walk the entire event history from genesis, replaying each
`MutationEvent` to reconstruct the tree. This is O(n) in total events but
guarantees correctness because each event is independently signed.

---

## 9. Agentic Delegation

### 9.1 DelegationCert Lifecycle

```
1. Supervisor spawns sub-agent (doc 10, doc 02)
2. Supervisor calls tree.grant_delegation(resource_id, sub_agent_did, Role::Operator, 1h, key)
3. DelegationCert stored on the resource node
4. Sub-agent operates -- permission checks find cert in O(1) (delegation shortcut)
5. TTL expires (HLC-based, handles clock skew) -- cert auto-invalidates
6. Alternatively: explicit revocation via tree.revoke_delegation()
```

### 9.2 Sub-Agent Spawning Creates Certs

When a supervisor spawns a sub-agent, it automatically creates `DelegationCert`
entries for each resource the sub-agent needs, derived from the sub-agent's
`.agent.toml` capabilities section:

```rust
pub fn spawn_sub_agent(
    tree: &mut ResourceTree, supervisor_did: &Did,
    manifest: &AgentManifest, supervisor_key: &exo_core::SigningKey,
) -> Result<Vec<DelegationCert>> {
    let ttl = manifest.ttl.unwrap_or(Duration::from_secs(86400));
    let mut certs = Vec::new();
    // Tools -> Operator certs on /kernel/services/<tool>
    for tool in &manifest.capabilities.tools {
        certs.push(tree.grant_delegation(
            &ResourceId::from_path(&format!("/kernel/services/{}", tool)),
            manifest.did.clone(), Role::Operator, ttl, supervisor_key,
        )?);
    }
    // Publish topics -> Operator certs; Subscribe topics -> Viewer certs
    for topic in &manifest.capabilities.topics_publish {
        certs.push(tree.grant_delegation(
            &ResourceId::from_path(&format!("/kernel/ipc/{}", topic)),
            manifest.did.clone(), Role::Operator, ttl, supervisor_key,
        )?);
    }
    for topic in &manifest.capabilities.topics_subscribe {
        certs.push(tree.grant_delegation(
            &ResourceId::from_path(&format!("/kernel/ipc/{}", topic)),
            manifest.did.clone(), Role::Viewer, ttl, supervisor_key,
        )?);
    }
    Ok(certs)
}
```

### 9.3 Group Nodes for Team-Based Access

Multiple agents share access via `ResourceKind::Node` with `group_members`.
Group membership is checked before delegation certs during permission evaluation.

### 9.4 Ephemeral Instance Attachment

Ephemeral agents (single-task, short-lived) attach as leaf nodes under
instance-pool nodes with short TTL DelegationCerts (e.g., 5 minutes).

---

## 10. Cryptographic Guarantees

### 10.1 Signed DAG Events

Every `MutationEvent` is appended to the ExoChain DAG with domain separator
`b"WEFTOS-RESOURCE-MUTATION-v1"`, Ed25519 signature by the mutating agent's key,
HLC timestamp for causal ordering, and content-addressed EventId via BLAKE3.

### 10.2 Subtree Merkle Roots

Each `ResourceNode` stores `subtree_merkle_root` computed from children's IDs.
This enables fast subtree verification (compare root against trusted value),
efficient distributed sync (compare Merkle roots to find differing subtrees),
and audit proofs via `EventInclusionProof` from exo-dag's MMR accumulator.

### 10.3 CGR Kernel Invariants

| Invariant | Description | Enforcement |
|---|---|---|
| No unsigned node | Every ResourceNode has non-default signature | insert()/update() sign before storing |
| No ownerless node | Every ResourceNode has primary_agent DID | ResourceKind enum requires DID |
| Bailment-typed policies | rbac_policies uses BailmentPolicy | Type system enforces Vec<BailmentPolicy> |
| Root immutability | Root node cannot be removed | remove() rejects root |
| Merkle consistency | subtree_merkle_root matches computed value | Verified at boot; recomputed on mutation |
| Signature chain | Event signatures form verifiable chain | exo-dag enforces on append |

### 10.4 EventInclusionProof

Any mutation can be proven via `exo_dag::prove_inclusion(dag, event_id)` which
returns the MMR path from the event to the accumulator root. A verifier
reconstructs the hash path bottom-up and compares against the known root.

---

## 11. Optional: Cedar Policy Integration

For deployments needing richer expressions, the `cedar` feature gate enables
[Cedar policy language](https://www.cedarpolicy.com/) integration.

### 11.1 Why Cedar

Cedar provides typed/analyzable policies, rich conditional expressions (e.g.,
"allow if agent.role == 'DevOps' AND resource.environment == 'staging'"), and
formal verification. This complements the tree's graph-based model: **Zanzibar
for structure, Cedar for conditions.**

### 11.2 Hybrid Evaluation

```rust
#[cfg(feature = "cedar")]
impl ResourceTree {
    pub fn check_with_cedar(
        &self, principal: &Did, resource_id: &ResourceId,
        action: Action, cedar_ctx: &CedarContext,
    ) -> Result<Decision, ResourceTreeError> {
        let tree_decision = self.check(principal, resource_id, action)?;
        match tree_decision {
            Decision::Deny { .. } => Ok(tree_decision), // Deny is final
            Decision::Allow | Decision::Defer { .. } => {
                // Cedar can only restrict, not expand access
                let cedar_decision = self.evaluate_cedar(principal, resource_id, &action, cedar_ctx)?;
                match (&tree_decision, &cedar_decision) {
                    (Decision::Allow, Decision::Allow) => Ok(Decision::Allow),
                    (Decision::Allow, Decision::Deny { .. }) => Ok(cedar_decision),
                    (Decision::Defer { .. }, _) => Ok(tree_decision),
                    _ => Ok(cedar_decision),
                }
            }
        }
    }
}
```

Cedar policies are stored alongside `BailmentPolicy` entries. A `PolicySet` is
compiled at boot from the `cedar_policy_dir` configuration.

---

## 12. CLI Commands

```
weave resource tree                          Show resource tree (depth-limited)
weave resource tree --depth <n>              Show tree to depth n (default: 3)
weave resource tree --path <path>            Show subtree rooted at path
weave resource inspect <id>                  Show node details (kind, policies, certs, Merkle)
weave resource grant <id> <did> <role>       Add delegation cert (--ttl <duration>)
weave resource revoke <id> <did>             Remove delegation cert
weave resource check <id> <did> <action>     Test permission check
weave resource audit <id>                    Show DAG mutation history
weave resource verify <id>                   Verify signature + Merkle for subtree
weave resource stats                         Show tree statistics (count, depth, cache rate)
```

**Example output:**

```
$ weave resource tree --depth 2
/
+-- kernel/                  [Node] primary: did:exo:kernel  children: 4
|   +-- services/            [Node] primary: did:exo:kernel  children: 5
|   +-- processes/           [Node] primary: did:exo:kernel  children: 3
|   +-- ipc/                 [Node] primary: did:exo:kernel  children: 5
|   +-- containers/          [Node] primary: did:exo:kernel  children: 2
+-- agents/                  [Node] primary: did:exo:kernel  children: 6
+-- apps/                    [Node] primary: did:exo:kernel  children: 2
+-- network/                 [Node] primary: did:exo:kernel  children: 2
+-- environments/            [Node] primary: did:exo:kernel  children: 3
+-- models/                  [Node] primary: did:exo:kernel  children: 3
+-- storage/                 [Node] primary: did:exo:kernel  children: 2
Total: 42 nodes, 18 leaves, 24 interior nodes
```

```
$ weave resource check /kernel/services/inference did:exo:user-agent-1 execute
Decision: Allow
  Matched: DelegationCert { delegate: did:exo:user-agent-1, role: Operator }
  Path: /kernel/services/inference (delegation shortcut at depth 0)
```

---

## 13. Configuration

### 13.1 weft.toml Section

```toml
[resource_tree]
enabled = true
cache_size = 1024                   # ACL cache entries (~200 bytes each)
lazy_eval = false                   # Defer evaluation without enforcement (dev mode)
cedar_enabled = false               # Requires `cedar` feature gate
cedar_policy_dir = ".policies/cedar/"
default_delegation_ttl_secs = 86400 # 24 hours
cert_prune_interval_secs = 3600    # Prune expired certs every hour
checkpoint_interval_secs = 300     # Checkpoint tree every 5 minutes
max_depth = 32                      # Prevent runaway nesting
audit_all_checks = false            # Log all checks to witness chain
```

### 13.2 Rust Configuration Type

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceTreeConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_cache_size")]
    pub cache_size: usize,
    #[serde(default)]
    pub lazy_eval: bool,
    #[serde(default)]
    pub cedar_enabled: bool,
    #[serde(default = "default_cedar_dir")]
    pub cedar_policy_dir: String,
    #[serde(default = "default_ttl")]
    pub default_delegation_ttl_secs: u64,
    #[serde(default = "default_prune")]
    pub cert_prune_interval_secs: u64,
    #[serde(default = "default_checkpoint")]
    pub checkpoint_interval_secs: u64,
    #[serde(default = "default_depth")]
    pub max_depth: u32,
    #[serde(default)]
    pub audit_all_checks: bool,
}

fn default_true() -> bool { true }
fn default_cache_size() -> usize { 1024 }
fn default_cedar_dir() -> String { ".policies/cedar/".into() }
fn default_ttl() -> u64 { 86400 }
fn default_prune() -> u64 { 3600 }
fn default_checkpoint() -> u64 { 300 }
fn default_depth() -> u32 { 32 }
```

---

## 14. Testing Strategy

### 14.1 Unit Tests

| Test | Module | Description |
|---|---|---|
| `resource_id_from_content` | model | Deterministic ID derivation from content |
| `resource_id_from_path` | model | Stable well-known path IDs |
| `resource_kind_primary_did` | model | Node/Leaf return correct DID |
| `resource_kind_direct_access` | model | Group membership detection |
| `role_hierarchy` | model | Owner > Admin > Operator > Viewer |
| `role_allows_action` | model | Each role permits/denies correct actions |
| `delegation_cert_expiry` | model | Expired certs detected via HLC |
| `delegation_cert_allows` | model | Cert role checked against action |
| `tree_insert` | tree | Insert leaf under Node, verify indexes |
| `tree_insert_under_leaf_fails` | tree | Cannot add child to Leaf |
| `tree_remove` | tree | Remove node + descendants, verify cleanup |
| `tree_remove_root_fails` | tree | Root cannot be removed |
| `tree_children_of` | tree | Returns correct sorted children |
| `tree_ancestors` | tree | Walk from leaf to root |
| `merkle_root_empty` | tree | Leaf has BLAKE3_EMPTY |
| `merkle_root_recomputation` | tree | Insert updates parent Merkle root |
| `merkle_root_chain` | tree | Depth-3 insert updates roots to root |
| `merkle_verify_all` | tree | Full verification passes after inserts |
| `check_delegation_shortcut` | permission | Active cert grants immediate Allow |
| `check_expired_cert_ignored` | permission | Expired cert ignored |
| `check_policy_collection` | permission | Policies collected leaf to root |
| `check_deny_overrides` | permission | Ancestor Deny overrides leaf Allow |
| `check_cache_hit` | permission | Cached triple returns immediately |
| `check_cache_invalidation` | permission | Mutation invalidates cache |
| `acl_cache_eviction` | permission | Eviction on max_size |
| `grant_delegation` | delegation | Cert created and attached |
| `revoke_delegation` | delegation | Cert removed, cache invalidated |
| `prune_expired_certs` | delegation | Expired certs removed tree-wide |
| `bootstrap_fresh` | boot | Fresh tree has all namespace nodes |
| `checkpoint_roundtrip` | boot | Save then load produces identical tree |
| `signature_verification` | crypto | Valid signature verified |
| `signature_tampering` | crypto | Modified node fails verification |

### 14.2 Integration Tests

| Test | Scope | Description |
|---|---|---|
| `kernel_boot_with_tree` | K0 + tree | Boot with tree, services registered as nodes |
| `agent_spawn_creates_node` | K1 + tree | Spawn creates ProcessEntry AND tree leaf |
| `tool_check_via_tree` | K1 + tree | CapabilityChecker delegates to tree.check() |
| `ipc_send_via_tree` | K2 + tree | IPC send gated by tree permission |
| `wasm_execute_via_tree` | K3 + tree | WASM execution gated by tree |
| `app_install_subtree` | K5 + tree | Install creates /apps/<name>/ subtree |
| `env_risk_modulation` | K6 + tree | Production Defers on risk > 0.3 |
| `cross_node_tree_sync` | K6 + tree | Two nodes sync via DAG, Merkle roots match |
| `delegation_lifecycle` | tree | Grant -> use -> expire -> denied |
| `full_boot_sequence` | all | Fresh boot -> bootstrap -> services -> agents -> verify |

### 14.3 Benchmark Targets

| Benchmark | Target | Description |
|---|---|---|
| `check_cached` | <100ns | Permission check with warm cache |
| `check_delegation_shortcut` | <1us | Check hitting cert at depth 1 |
| `check_full_walk_depth_5` | <10us | Full walk, depth 5, cold cache |
| `insert_leaf` | <50us | Insert + Merkle recomputation |
| `merkle_recompute_depth_10` | <100us | Merkle chain recomputation |
| `bootstrap_100_nodes` | <10ms | Fresh tree with 100 nodes |
| `from_checkpoint_1000` | <100ms | Load from checkpoint, 1000 nodes |

---

## 15. Risks and Mitigations

| ID | Risk | Severity | Likelihood | Mitigation |
|---|---|---|---|---|
| R1 | Tree too large for memory | Medium | Low | 1000 nodes ~= 500KB. Subtree sharding for >10K. Monitor via `weave resource stats`. |
| R2 | Permission latency on deep trees | Medium | Medium | Delegation shortcut avoids full walk. ACL cache. Max depth 32. |
| R3 | ACL cache staleness | High | Medium | Cache invalidated on every mutation. Conservative: invalidate principal + resource. |
| R4 | DAG checkpoint corruption | High | Low | Multiple checkpoint versions. Full DAG replay as fallback. BLAKE3 checksums. |
| R5 | DelegationCert TTL too short | Medium | Medium | Default 24h (configurable). Renewal before expiry. Health monitor for near-expiry. |
| R6 | Cedar policy conflicts | Medium | Medium | Cedar only restricts, never expands. Deny from either source is final. |
| R7 | Merkle recomputation on wide trees | Low | Low | Recomputation is O(depth) not O(width). Single-level hash is O(children) once. |
| R8 | Clock skew causes premature cert expiry | Medium | Medium | HLC handles skew (exo-core). Certs use HLC not wall-clock. |
| R9 | Signature verification bottleneck at boot | Medium | Low | Ed25519: ~15K verifications/sec. 10K nodes in <1s. Parallelizable. |
| R10 | Breaking exochain API changes | High | Medium | Pin git commits. Adapter layer isolates types (doc 07 principle). |

---

## 16. Relationship to Other Documents

| Document | ID | Relationship |
|---|---|---|
| `00-orchestrator.md` | W-KERNEL | New cross-cutting subsystem. Extends workstream by one crate. |
| `01-phase-K0-kernel-foundation.md` | K0 | ServiceRegistry gains tree node creation. HealthSystem verifies signatures. Tree loads at boot. |
| `02-phase-K1-supervisor-rbac.md` | K1 | CapabilityChecker delegates to tree.check(). ProcessEntry creates tree leaf. AgentCapabilities -> BailmentPolicy. |
| `03-phase-K2-a2a-ipc.md` | K2 | IPC topics are tree leaves. Send permission via tree. TopicRouter gated by tree. |
| `04-phase-K3-wasm-sandbox.md` | K3 | WASM execution gated by tree. SandboxPolicy stored as BailmentPolicy. |
| `05-phase-K4-containers.md` | K4 | Containers are tree leaves. Lifecycle operations gated by tree. |
| `06-phase-K5-app-framework.md` | K5 | App install creates subtree. weftapp.toml rules -> BailmentPolicy. App agents get certs. |
| `07-ruvector-deep-integration.md` | W-KERNEL-07 | Uses same exochain crates. No conflict (different concerns). |
| `08-ephemeral-os-architecture.md` | W-KERNEL-08 | Distributed sync via DAG. Environment scoping via subtrees. Governance hash includes Merkle root. |
| `09-environments-and-learning.md` | W-KERNEL-09 | EnvironmentClass -> subtree with scoped risk thresholds. Promotion changes certs. |
| `10-agent-first-single-user.md` | W-KERNEL-10 | Root agent gets Owner cert on /. Service agents are tree leaves. .agent.toml -> certs. |
| `11-local-inference-agents.md` | W-KERNEL-11 | GGUF models are tree leaves under /models/. Loading gated by Execute cert. |
| `12-networking-and-pairing.md` | W-KERNEL-12 | Peers are tree leaves. Pairing bond level -> cert role (Viewer/Operator/Admin). |

---

## 17. Definition of Done / Exit Criteria

### Crate Implementation

- [ ] `exo-resource-tree` crate compiles with `cargo check`
- [ ] Core types (ResourceId, ResourceKind, ResourceNode, DelegationCert, Role, Action) per Section 3
- [ ] ResourceTree with insert, remove, get, children_of, ancestors per Section 4
- [ ] Merkle root computation and chain recomputation per Section 4.4
- [ ] Permission check with delegation shortcut per Section 5.1
- [ ] EffectiveAclCache per Section 5.2
- [ ] Delegation grant/revoke per Section 5.3
- [ ] Boot-time construction from DAG checkpoint per Section 4.2
- [ ] Fresh bootstrap with well-known namespace nodes per Section 4.2
- [ ] Mutation events appended to DAG per Section 4.5
- [ ] Checkpoint save/load per Section 8.2
- [ ] Core crate <400 LOC; total with tests <1200 LOC

### Integration

- [ ] clawft-kernel depends on exo-resource-tree behind `resource-tree` feature gate
- [ ] ServiceRegistry creates tree nodes on register (K0)
- [ ] AgentSupervisor creates tree nodes on spawn (K1)
- [ ] CapabilityChecker delegates to tree.check() (K1)
- [ ] IPC send checks tree permission (K2)
- [ ] Tree loads during boot before agents spawn
- [ ] CLI `weave resource tree|inspect|grant|revoke|check|audit|verify|stats`

### Testing

- [ ] All 30+ unit tests pass (Section 14.1)
- [ ] All 10 integration tests pass (Section 14.2)
- [ ] Benchmark targets met (cached check <100ns, delegation shortcut <1us)
- [ ] Existing K0-K5 tests pass with `resource-tree` feature disabled
- [ ] WASM browser build unaffected

### Cryptographic

- [ ] Every ResourceNode has valid Ed25519 signature
- [ ] All Merkle roots verified at boot
- [ ] Mutations use domain separator `WEFTOS-RESOURCE-MUTATION-v1`
- [ ] EventInclusionProof generation works
- [ ] CGR invariants enforced (Section 10.3)

### Documentation

- [ ] `///` doc comments on all public types and functions
- [ ] Rustdoc builds without warnings
- [ ] CLI help text for `weave resource` subcommands
- [ ] This SPARC document committed to `.planning/sparc/weftos/`
