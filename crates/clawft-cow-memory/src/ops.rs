//! The lineage-mutating half of `BranchableMemory`: `checkpoint`, `rollback`,
//! `promote`, `branch`, `fork`. Split from `branchable_memory.rs` purely to
//! keep files under ~500 lines; this is still `impl BranchableMemory`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::branchable_memory::{BranchableMemory, PromoteReport};
use crate::error::{CowMemoryError, Result};
use crate::node::{self, CheckpointId, MemoryNode};

/// One node's owned edits/texts/tombstones, snapshotted for replay in
/// `promote` (see its doc comment for why this is a clone-first pass).
type NodeReplaySnapshot = (HashMap<u64, Vec<f32>>, HashMap<u64, String>, HashSet<u64>);

/// Which point in `self`'s current lineage a `branch`/`fork` reads from.
enum ForkSource {
    /// Everything visible right now: `working` plus this lineage's own
    /// checkpoints, newest first (agenticow's "clone" -- fork off the tip).
    Head,
    /// A specific earlier checkpoint (and everything at/older than it),
    /// excluding `working` and anything newer ("clone-of-clone" -- you can
    /// fork from a fork's own checkpoint just as well as from the tip).
    Checkpoint(usize),
}

impl BranchableMemory {
    /// Freeze `working` into a new checkpoint (pushed onto `ancestors`,
    /// newest-first) and derive a fresh, empty, writable `working` from it.
    /// O(1) regardless of lineage size: `derive` copies zero vectors (see
    /// `MemoryNode::derive_from`).
    pub fn checkpoint(&mut self, label: impl Into<String>) -> Result<CheckpointId> {
        let label = label.into();
        self.working.freeze()?;
        let checkpoint_id = self.working.id;

        let new_id = CheckpointId::alloc();
        let new_path = self.dir.join(format!("node-{}.working.rvf", new_id.0));
        let new_working = MemoryNode::derive_from(
            new_id,
            &self.working.store,
            &new_path,
            self.opts.clone(),
            Some("working".into()),
        )?;

        let mut frozen = std::mem::replace(&mut self.working, new_working);
        frozen.label = Some(label);
        self.ancestors.insert(0, frozen);
        Ok(checkpoint_id)
    }

    /// Discard `working` and every checkpoint newer than `to`, then derive
    /// a fresh `working` from whatever is now the head (the target
    /// checkpoint, or `base` if `to` is `None`).
    ///
    /// `to` must name one of `self`'s *own* checkpoints (an id returned by
    /// a prior `checkpoint()` call on this same `BranchableMemory` that
    /// hasn't since been rolled back or promoted away). It cannot target
    /// `working` itself (never checkpointed, so not a valid rollback point)
    /// or an `inherited` node (frozen forever from a source lineage,
    /// nothing to "roll back to" there -- fork from it again instead).
    pub fn rollback(&mut self, to: Option<CheckpointId>) -> Result<()> {
        let target_idx = match to {
            None => None,
            Some(id) => Some(
                self.ancestors
                    .iter()
                    .position(|n| n.id == id)
                    .ok_or_else(|| node::not_found(id))?,
            ),
        };

        // `ancestors` is newest..oldest. Discard everything strictly newer
        // than the target (index range `0..idx`); `None` discards the
        // whole chain.
        let discarded: Vec<MemoryNode> = match target_idx {
            None => std::mem::take(&mut self.ancestors),
            Some(idx) => self.ancestors.drain(0..idx).collect(),
        };

        let new_id = CheckpointId::alloc();
        let new_path = self.dir.join(format!("node-{}.working.rvf", new_id.0));
        let new_working = match self.ancestors.first() {
            Some(head) => MemoryNode::derive_from(
                new_id,
                &head.store,
                &new_path,
                self.opts.clone(),
                Some("working".into()),
            )?,
            None => MemoryNode::derive_from(
                new_id,
                &self.base.store,
                &new_path,
                self.opts.clone(),
                Some("working".into()),
            )?,
        };
        let old_working = std::mem::replace(&mut self.working, new_working);

        let mut to_remove: Vec<PathBuf> = discarded.iter().map(|n| n.path.clone()).collect();
        to_remove.push(old_working.path.clone());
        drop(discarded);
        drop(old_working);
        for path in &to_remove {
            MemoryNode::remove_file_best_effort(path);
        }

        Ok(())
    }

    /// Replay this lineage's own checkpoint chain (oldest first, so a later
    /// checkpoint's edits correctly win over an earlier one on id
    /// collision) plus `working` into `base`, then collapse: discard the
    /// checkpoint chain and `working`, and derive a fresh `working` from
    /// the now-updated `base`.
    ///
    /// `base` stays writable throughout (it is never frozen by
    /// `checkpoint`), so this is a plain `ingest_batch`/`delete` replay, not
    /// a COW operation.
    ///
    /// **Known limitation**, unlike `BranchableMemory::delete`: this *does*
    /// call `RvfStore::delete` on `base`, because `base`'s tombstones need
    /// to be physically enforceable independent of any particular
    /// in-process `BranchableMemory` (a reopened lineage has no working/
    /// ancestor tombstone sets at all -- see `BranchableMemory::open`). The
    /// tradeoff is `delete`'s sticky-bitmap behavior (see `delete`'s doc
    /// comment): if a promoted delete's id is later re-ingested into `base`
    /// under the same id, it will not become visible again short of a
    /// `compact()` (which itself needs care -- see `delete`'s note on why
    /// `compact` would drop the re-ingested data too). Flagged as a Phase 2
    /// open question rather than solved here.
    pub fn promote(&mut self) -> Result<PromoteReport> {
        // Snapshot each node's edits/tombstones as owned data first. This
        // sidesteps needing simultaneous `&mut` borrows of `base` and of
        // `ancestors`/`working` to do the replay in one pass -- the clone
        // cost is a Phase 1 simplicity trade-off, not a hard constraint.
        let mut replay: Vec<NodeReplaySnapshot> = Vec::with_capacity(self.ancestors.len() + 1);
        for node in self.ancestors.iter().rev() {
            replay.push((
                node.edit_log.clone(),
                node.texts.clone(),
                node.tombstones.clone(),
            ));
        }
        replay.push((
            self.working.edit_log.clone(),
            self.working.texts.clone(),
            self.working.tombstones.clone(),
        ));

        let mut ingested = 0usize;
        let mut deleted = 0usize;
        for (edit_log, texts, tombstones) in &replay {
            if !edit_log.is_empty() {
                let ids: Vec<u64> = edit_log.keys().copied().collect();
                let vectors: Vec<&[f32]> = ids.iter().map(|id| edit_log[id].as_slice()).collect();
                self.base.store.ingest_batch(&vectors, &ids, None)?;
                ingested += ids.len();
                for id in &ids {
                    if let Some(text) = texts.get(id) {
                        self.base.texts.insert(*id, text.clone());
                    }
                }
            }
            if !tombstones.is_empty() {
                let ids: Vec<u64> = tombstones.iter().copied().collect();
                // Meaningful for ids physically present in `base`; a no-op
                // otherwise. The BranchableMemory-level tombstone set
                // (below) is what actually masks `inherited` history.
                self.base.store.delete(&ids)?;
                deleted += ids.len();
                self.base.tombstones.extend(ids.iter().copied());
            }
        }

        let discarded_ancestors = std::mem::take(&mut self.ancestors);
        let new_id = CheckpointId::alloc();
        let new_path = self.dir.join(format!("node-{}.working.rvf", new_id.0));
        let new_working = MemoryNode::derive_from(
            new_id,
            &self.base.store,
            &new_path,
            self.opts.clone(),
            Some("working".into()),
        )?;
        let old_working = std::mem::replace(&mut self.working, new_working);

        let mut to_remove: Vec<PathBuf> =
            discarded_ancestors.iter().map(|n| n.path.clone()).collect();
        to_remove.push(old_working.path.clone());
        drop(discarded_ancestors);
        drop(old_working);
        for path in &to_remove {
            MemoryNode::remove_file_best_effort(path);
        }

        Ok(PromoteReport { ingested, deleted })
    }

    /// Fork off the current tip: a brand-new, independent `BranchableMemory`
    /// at `path` that can see everything this lineage sees *right now*
    /// (`working` + this lineage's own checkpoints + `base` + anything this
    /// lineage itself inherited), but whose own writes never appear back in
    /// `self`, and vice versa (agenticow's "isolated per-actor view" /
    /// cubecow's "clone").
    pub fn branch(&self, path: impl AsRef<Path>) -> Result<BranchableMemory> {
        self.spawn_independent(ForkSource::Head, path.as_ref())
    }

    /// Fork from a specific earlier checkpoint in `self`'s own chain
    /// (cubecow's "clone-of-clone" -- forking a fork, or forking a point in
    /// the past rather than the tip). `from` must be one of `self`'s own
    /// checkpoint ids (not `working`, not an `inherited` node).
    pub fn fork(&self, from: CheckpointId, path: impl AsRef<Path>) -> Result<BranchableMemory> {
        let idx = self
            .ancestors
            .iter()
            .position(|n| n.id == from)
            .ok_or_else(|| node::not_found(from))?;
        self.spawn_independent(ForkSource::Checkpoint(idx), path.as_ref())
    }

    fn spawn_independent(&self, source: ForkSource, target_dir: &Path) -> Result<BranchableMemory> {
        std::fs::create_dir_all(target_dir)?;

        // The new lineage's `inherited` chain: read-only reopens of every
        // node visible from the chosen source point, newest first, ending
        // at `self.base` and then anything `self` itself inherited (so
        // forking a fork carries the whole history forward).
        let mut inherited: Vec<MemoryNode> = Vec::new();
        match source {
            ForkSource::Head => {
                inherited.push(self.working.open_readonly_snapshot()?);
                for n in &self.ancestors {
                    inherited.push(n.open_readonly_snapshot()?);
                }
            }
            ForkSource::Checkpoint(idx) => {
                for n in &self.ancestors[idx..] {
                    inherited.push(n.open_readonly_snapshot()?);
                }
            }
        }
        inherited.push(self.base.open_readonly_snapshot()?);
        for n in &self.inherited {
            inherited.push(n.open_readonly_snapshot()?);
        }

        let head_store = &inherited
            .first()
            .ok_or_else(|| {
                CowMemoryError::Io(std::io::Error::other(
                    "spawn_independent: inherited chain unexpectedly empty",
                ))
            })?
            .store;

        let new_base_id = CheckpointId::alloc();
        let new_base_path = target_dir.join(format!("node-{}.base.rvf", new_base_id.0));
        let new_base = MemoryNode::derive_from(
            new_base_id,
            head_store,
            &new_base_path,
            self.opts.clone(),
            Some("base".into()),
        )?;

        let new_working_id = CheckpointId::alloc();
        let new_working_path = target_dir.join(format!("node-{}.working.rvf", new_working_id.0));
        let new_working = MemoryNode::derive_from(
            new_working_id,
            &new_base.store,
            &new_working_path,
            self.opts.clone(),
            Some("working".into()),
        )?;

        Ok(BranchableMemory {
            dir: target_dir.to_path_buf(),
            opts: self.opts.clone(),
            working: new_working,
            ancestors: Vec::new(),
            base: new_base,
            inherited,
        })
    }
}
