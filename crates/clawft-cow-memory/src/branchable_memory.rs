//! `BranchableMemory` -- the crate's one public entry point. Construction,
//! ingest/delete/query, and introspection (`status`/`lineage`/`diff`) live
//! here; the lineage-mutating operations (`checkpoint`/`rollback`/`promote`/
//! `branch`/`fork`) live in `ops.rs` as a second `impl` block, split out
//! purely to keep any one file under ~500 lines.

use std::path::{Path, PathBuf};

use rvf_runtime::options::DistanceMetric;
use rvf_runtime::{QueryOptions, RvfOptions};

use crate::chain_walk::{self, ScoredId};
use crate::error::{CowMemoryError, Result};
use crate::id_gen::next_vector_id;
use crate::node::{CheckpointId, MemoryNode};
use crate::normalize::l2_normalize;

/// A vector to ingest. `id`, left `None`, is auto-assigned from the
/// process-wide monotonic counter (`crate::id_gen`) so it cannot collide
/// with an id assigned in a sibling fork of the same lineage.
pub struct VectorItem {
    pub id: Option<u64>,
    pub vector: Vec<f32>,
    /// Optional text payload carried alongside the vector (agenticow's
    /// `texts` map). Not persisted into the `.rvf` file; survives only as
    /// long as this process's `BranchableMemory` does.
    pub text: Option<String>,
}

impl VectorItem {
    pub fn new(vector: Vec<f32>) -> Self {
        Self {
            id: None,
            vector,
            text: None,
        }
    }

    pub fn with_id(mut self, id: u64) -> Self {
        self.id = Some(id);
        self
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }
}

/// What `promote` moved into `base`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PromoteReport {
    pub ingested: usize,
    pub deleted: usize,
}

/// What changed in `working` relative to its immediate parent -- i.e. what
/// this turn has done so far (plan §3: "what changed this turn").
#[derive(Clone, Debug, Default)]
pub struct MemoryDiff {
    pub added: Vec<u64>,
    pub deleted: Vec<u64>,
}

/// A node's position in the lineage, as reported by [`BranchableMemory::lineage`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeRole {
    /// The current writable tip.
    Working,
    /// A frozen checkpoint between `working` and `base`, newest to oldest.
    Checkpoint,
    /// This lineage's own writable, promotable root.
    Base,
    /// A read-only node inherited from the source lineage at fork time
    /// (empty unless this `BranchableMemory` was produced by `branch`/`fork`).
    Inherited,
}

/// One entry in [`BranchableMemory::lineage`]'s provenance walk.
#[derive(Clone, Debug)]
pub struct LineageEntry {
    pub role: NodeRole,
    pub id: CheckpointId,
    pub label: Option<String>,
    pub file_id: [u8; 16],
    pub parent_id: [u8; 16],
    pub lineage_depth: u32,
    pub created_at_ms: u64,
    /// Vectors ingested directly at this node (0 for `Inherited` nodes,
    /// whose edit-log is not carried forward -- see `MemoryNode::open_readonly_snapshot`).
    pub mutation_count: usize,
    pub tombstone_count: usize,
}

/// Snapshot of store health across the lineage, mirroring agenticow's
/// `status()` (plan §3) but reporting `working` and `base` separately since
/// there is no single `RvfStore` that represents "the whole lineage."
#[derive(Clone, Debug)]
pub struct MemoryStatus {
    pub dimension: u16,
    pub own_checkpoint_depth: usize,
    pub inherited_depth: usize,
    pub working: rvf_runtime::StoreStatus,
    pub base: rvf_runtime::StoreStatus,
}

/// A branchable, checkpointable, chain-walk-queryable view over an RVF
/// vector lineage -- the Rust port of agenticow's `RvfDatabase` orchestration
/// (`.planning/ruv/integration/agenticow-integration-plan.md`).
///
/// # Layout
///
/// ```text
/// query priority (newest first):
///     working                 <- current writable tip
///     ancestors[0..]          <- this lineage's own checkpoints, newest..oldest
///     base                    <- this lineage's own writable, promotable root
///     inherited[0..]          <- read-only, inherited from a fork/branch source, newest..oldest
/// ```
///
/// `query` walks every node in that order and merges (see [`crate::chain_walk`])
/// because `RvfStore::query` (crates.io 0.2.0) does not read through the COW
/// boundary on any platform the published crate ships for -- verified against
/// `store.rs:314-368`, which scans only `self.vectors.ids()`.
///
/// # Fixed metric discipline
///
/// Every store in a lineage is created with `DistanceMetric::L2` over
/// L2-normalized vectors, never `DistanceMetric::Cosine` directly -- see
/// `crate::normalize` for why. `ingest`/`query` normalize automatically;
/// callers never need to.
///
/// # What is *not* durable across a process restart
///
/// The `.rvf` files themselves are durable (fsynced per write). The
/// crate-level bookkeeping that makes chain-walk correct -- each node's
/// tombstone set, ingest edit-log, and text payloads, plus which files are
/// `ancestors` vs `inherited` vs `base` -- lives only in this struct. There
/// is no `save`/`load` manifest yet (agenticow's `index.js` has one; the
/// integration plan lists it as a later row in §3, and Phase 3's risk list
/// flags orphaned `.rvf` files on crash as open). [`BranchableMemory::open`]
/// is a best-effort partial reopen (base only) documented at its call site.
pub struct BranchableMemory {
    pub(crate) dir: PathBuf,
    pub(crate) opts: RvfOptions,
    pub(crate) working: MemoryNode,
    pub(crate) ancestors: Vec<MemoryNode>,
    pub(crate) base: MemoryNode,
    pub(crate) inherited: Vec<MemoryNode>,
}

impl BranchableMemory {
    /// Create a new lineage at `dir` (created if missing) for vectors of the
    /// given `dimension`. Forces `DistanceMetric::L2` (see crate docs).
    pub fn create(dir: impl AsRef<Path>, dimension: u16) -> Result<Self> {
        let mut opts = RvfOptions {
            dimension,
            ..Default::default()
        };
        opts.metric = DistanceMetric::L2;
        Self::create_with_options(dir, opts)
    }

    /// Create a new lineage with caller-supplied `RvfOptions` (HNSW
    /// parameters, witness config, security policy, ...). `metric` is
    /// always overridden to `L2` regardless of what's passed in -- see
    /// `crate::normalize` for why this is not optional.
    pub fn create_with_options(dir: impl AsRef<Path>, mut opts: RvfOptions) -> Result<Self> {
        if opts.dimension == 0 {
            return Err(CowMemoryError::InvalidDimension);
        }
        opts.metric = DistanceMetric::L2;

        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)?;

        let base_id = CheckpointId::alloc();
        let base_path = dir.join(format!("node-{}.base.rvf", base_id.0));
        let base = MemoryNode::create_root(base_id, &base_path, opts.clone(), Some("base".into()))?;

        let working_id = CheckpointId::alloc();
        let working_path = dir.join(format!("node-{}.working.rvf", working_id.0));
        let working = MemoryNode::derive_from(
            working_id,
            &base.store,
            &working_path,
            opts.clone(),
            Some("working".into()),
        )?;

        Ok(Self {
            dir,
            opts,
            working,
            ancestors: Vec::new(),
            base,
            inherited: Vec::new(),
        })
    }

    /// Best-effort reopen of a lineage previously created at `dir`.
    ///
    /// **Lossy.** Only `base` is reopened from disk; any checkpoints
    /// (`ancestors`), any fork/branch inheritance (`inherited`), and every
    /// node's tombstones/edit-log/texts are gone -- they only ever lived in
    /// the `BranchableMemory` struct of the process that created them, not
    /// in the `.rvf` files. A fresh `working` is derived from the reopened
    /// `base`. This is enough to keep using a lineage's *promoted* content
    /// after a restart; it is not crash recovery for an in-flight turn
    /// (that is Phase 3 scope per the integration plan's risk table).
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        let base_path = Self::find_base_file(&dir)?;
        let base_store = rvf_runtime::RvfStore::open(&base_path)?;
        let opts = RvfOptions {
            dimension: base_store.dimension(),
            metric: DistanceMetric::L2,
            ..Default::default()
        };

        let base_id = CheckpointId::alloc();
        let base = MemoryNode::adopt(base_id, base_store, base_path, Some("base".into()));

        let working_id = CheckpointId::alloc();
        let working_path = dir.join(format!("node-{}.working.rvf", working_id.0));
        let working = MemoryNode::derive_from(
            working_id,
            &base.store,
            &working_path,
            opts.clone(),
            Some("working".into()),
        )?;

        Ok(Self {
            dir,
            opts,
            working,
            ancestors: Vec::new(),
            base,
            inherited: Vec::new(),
        })
    }

    fn find_base_file(dir: &Path) -> Result<PathBuf> {
        let mut candidates: Vec<PathBuf> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.ends_with(".base.rvf"))
                    .unwrap_or(false)
            })
            .collect();
        candidates.sort();
        candidates.into_iter().next().ok_or_else(|| {
            CowMemoryError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("no *.base.rvf file found in {}", dir.display()),
            ))
        })
    }

    pub fn dimension(&self) -> u16 {
        self.opts.dimension
    }

    /// Ingest a batch of vectors into `working`. Each vector is
    /// L2-normalized before it reaches `RvfStore` (see `crate::normalize`).
    /// Returns the assigned id for each item, in input order.
    pub fn ingest(&mut self, items: &[VectorItem]) -> Result<Vec<u64>> {
        if items.is_empty() {
            return Ok(Vec::new());
        }

        let dim = self.opts.dimension as usize;
        let mut normalized: Vec<Vec<f32>> = Vec::with_capacity(items.len());
        let mut ids: Vec<u64> = Vec::with_capacity(items.len());
        for item in items {
            if item.vector.len() != dim {
                return Err(CowMemoryError::DimensionMismatch {
                    expected: dim,
                    got: item.vector.len(),
                });
            }
            normalized.push(l2_normalize(&item.vector));
            ids.push(item.id.unwrap_or_else(next_vector_id));
        }

        let refs: Vec<&[f32]> = normalized.iter().map(|v| v.as_slice()).collect();
        self.working.store.ingest_batch(&refs, &ids, None)?;

        for i in 0..items.len() {
            let id = ids[i];
            self.working.edit_log.insert(id, normalized[i].clone());
            // Re-ingesting a previously-tombstoned id (within this same
            // node) un-hides it -- the fresh write is what should be seen.
            self.working.tombstones.remove(&id);
            if let Some(text) = &items[i].text {
                self.working.texts.insert(id, text.clone());
            }
        }

        Ok(ids)
    }

    /// Hide `ids` from this lineage's view.
    ///
    /// Deliberately does **not** call `RvfStore::delete` on `working` for
    /// locally-present ids. `RvfStore::delete` sets a bit in that store's
    /// `deletion_bitmap` that nothing clears short of `compact()` (verified
    /// against `store.rs`: `ingest_batch` never touches the bitmap, so
    /// re-ingesting under the same id afterward would leave the fresh
    /// vector data present but permanently excluded from `query()` by the
    /// stale bit -- and `compact()` would then *remove* that fresh data too,
    /// since it drops every id still in the bitmap regardless of whether it
    /// was re-ingested since). So this crate's own tombstone set is the
    /// *only* mechanism for hiding an id, at every level of the chain --
    /// including within a single node -- see `crate::chain_walk`, which
    /// masks a node's own results against its own tombstones, not just
    /// older nodes'. This also means `delete` followed by `ingest` under
    /// the same id, within `working`, correctly un-hides it.
    pub fn delete(&mut self, ids: &[u64]) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        for id in ids {
            self.working.edit_log.remove(id);
            self.working.texts.remove(id);
        }
        self.working.tombstones.extend(ids.iter().copied());
        Ok(())
    }

    /// Chain-walk k-nearest-neighbor query across the whole visible lineage
    /// (`working`, this lineage's own checkpoints, `base`, and anything
    /// inherited from a fork source). See `crate::chain_walk` for why this
    /// is a manual merge rather than a single `RvfStore::query` call.
    pub fn query(&self, vector: &[f32], k: usize) -> Result<Vec<ScoredId>> {
        let dim = self.opts.dimension as usize;
        if vector.len() != dim {
            return Err(CowMemoryError::DimensionMismatch {
                expected: dim,
                got: vector.len(),
            });
        }
        if k == 0 {
            return Ok(Vec::new());
        }
        let normalized = l2_normalize(vector);
        let chain = self.chain_priority_order();
        let qopts = QueryOptions::default();
        chain_walk::query_chain(&chain, &normalized, k, &qopts)
    }

    pub(crate) fn chain_priority_order(&self) -> Vec<&MemoryNode> {
        let mut v: Vec<&MemoryNode> =
            Vec::with_capacity(1 + self.ancestors.len() + 1 + self.inherited.len());
        v.push(&self.working);
        v.extend(self.ancestors.iter());
        v.push(&self.base);
        v.extend(self.inherited.iter());
        v
    }

    /// What `working` has added/deleted relative to its immediate parent --
    /// "what this turn changed" (plan §3). Does not look at ancestors
    /// checkpointed earlier in this same turn; call `checkpoint`-by-
    /// `checkpoint` if that finer granularity is needed.
    pub fn diff(&self) -> MemoryDiff {
        let mut added: Vec<u64> = self.working.edit_log.keys().copied().collect();
        added.sort_unstable();
        let mut deleted: Vec<u64> = self.working.tombstones.iter().copied().collect();
        deleted.sort_unstable();
        MemoryDiff { added, deleted }
    }

    /// Walk the full provenance chain, newest to oldest, `Working` first
    /// and the inherited fork source (if any) last.
    pub fn lineage(&self) -> Vec<LineageEntry> {
        let mut out = Vec::with_capacity(1 + self.ancestors.len() + 1 + self.inherited.len());
        out.push(Self::describe(&self.working, NodeRole::Working));
        out.extend(
            self.ancestors
                .iter()
                .map(|n| Self::describe(n, NodeRole::Checkpoint)),
        );
        out.push(Self::describe(&self.base, NodeRole::Base));
        out.extend(
            self.inherited
                .iter()
                .map(|n| Self::describe(n, NodeRole::Inherited)),
        );
        out
    }

    fn describe(node: &MemoryNode, role: NodeRole) -> LineageEntry {
        LineageEntry {
            role,
            id: node.id,
            label: node.label.clone(),
            file_id: *node.store.file_id(),
            parent_id: *node.store.parent_id(),
            lineage_depth: node.store.lineage_depth(),
            created_at_ms: node.created_at_ms,
            mutation_count: node.edit_log.len(),
            tombstone_count: node.tombstones.len(),
        }
    }

    pub fn status(&self) -> MemoryStatus {
        MemoryStatus {
            dimension: self.opts.dimension,
            own_checkpoint_depth: self.ancestors.len(),
            inherited_depth: self.inherited.len(),
            working: self.working.store.status(),
            base: self.base.store.status(),
        }
    }
}
