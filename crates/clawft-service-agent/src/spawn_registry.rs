//! `SpawnRegistry` — task tracking for daemon-hosted subagents (M4 A.3).
//!
//! The registry is the source of truth for every subagent a parent
//! conversation kicks off (design D1/D5/D7). It maps:
//!
//! - `task_id → `[`TaskRecord`] — status, outcome, parent/child conv ids, and
//!   the detached driver's [`JoinHandle`](tokio::task::JoinHandle) (for
//!   `await: false` children, so cancel/parent-end-cascade can abort them).
//! - `group_id → Vec<task_id>` — swarm grouping (design D7); `task_status`
//!   / `group_status` aggregate a group.
//! - `parent_conv → live count` — the per-conversation concurrency cap
//!   (design D5): [`try_reserve`](SpawnRegistry::try_reserve) atomically
//!   admits at most `MAX_SUBAGENTS_PER_CONV` live children, mirroring
//!   `spawn_tool::ACTIVE_SPAWNS`.
//!
//! The registry holds no config and enforces no depth cap — depth is a
//! per-spec property the spawner checks (design D5). The registry only owns the
//! concurrency reservation because that is shared mutable state.

use std::sync::Arc;

use dashmap::DashMap;
use tokio::task::JoinHandle;

use clawft_core::agent::spawn::{TaskOutcome, TaskStatus};

/// A spawned-task identifier (also the primary registry key).
pub type TaskId = String;
/// A swarm-group identifier (design D7).
pub type GroupId = String;

/// One tracked subagent task.
pub struct TaskRecord {
    /// Stable task id (registry key).
    pub task_id: TaskId,
    /// The parent conversation that spawned this task (`conv_id = P`).
    pub parent_conv: String,
    /// The child conversation driving the work (`conv_id = C`).
    pub child_conv: String,
    /// Optional swarm group id.
    pub group_id: Option<GroupId>,
    /// Current lifecycle status.
    pub status: TaskStatus,
    /// Terminal (or latest) outcome; `None` while `Running` with nothing yet.
    pub outcome: Option<TaskOutcome>,
    /// The detached driver task for `await: false` children. `None` for
    /// inline (`await: true`) spawns. Aborted on cancel / parent-end cascade.
    pub join_handle: Option<JoinHandle<()>>,
}

impl TaskRecord {
    /// A fresh `Running` record with no outcome yet.
    fn new(
        task_id: TaskId,
        parent_conv: String,
        child_conv: String,
        group_id: Option<GroupId>,
    ) -> Self {
        Self {
            task_id,
            parent_conv,
            child_conv,
            group_id,
            status: TaskStatus::Running,
            outcome: None,
            join_handle: None,
        }
    }
}

/// Concurrent task/group/live-count index for daemon subagents.
#[derive(Default)]
pub struct SpawnRegistry {
    tasks: DashMap<TaskId, TaskRecord>,
    groups: DashMap<GroupId, Vec<TaskId>>,
    /// Live (non-terminal) child count per parent conversation. Reserved on
    /// admission, released on terminal transition. Guards the concurrency cap.
    live: DashMap<String, usize>,
}

impl SpawnRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Atomically admit one more live child for `parent_conv` if the current
    /// live count is below `max`. Returns `Ok(new_count)` on admission, or
    /// `Err(current)` when the cap is already reached (design D5).
    ///
    /// Reservation must precede `insert`, and a *failed* dispatch that never
    /// inserts must call [`release`](Self::release) to give the slot back.
    pub fn try_reserve(&self, parent_conv: &str, max: u32) -> Result<usize, usize> {
        let mut entry = self.live.entry(parent_conv.to_string()).or_insert(0);
        if *entry >= max as usize {
            return Err(*entry);
        }
        *entry += 1;
        Ok(*entry)
    }

    /// Release one reserved live slot for `parent_conv` (saturating at 0).
    pub fn release(&self, parent_conv: &str) {
        if let Some(mut entry) = self.live.get_mut(parent_conv) {
            *entry = entry.saturating_sub(1);
        }
    }

    /// Live (non-terminal) child count for a parent conversation.
    pub fn live_count(&self, parent_conv: &str) -> usize {
        self.live.get(parent_conv).map(|e| *e.value()).unwrap_or(0)
    }

    /// Register a freshly-spawned `Running` task. Indexes it by id and, when
    /// grouped, appends it to its group. Does **not** touch the live counter —
    /// call [`try_reserve`](Self::try_reserve) first.
    pub fn insert(
        &self,
        task_id: impl Into<TaskId>,
        parent_conv: impl Into<String>,
        child_conv: impl Into<String>,
        group_id: Option<GroupId>,
    ) {
        let task_id = task_id.into();
        if let Some(ref g) = group_id {
            self.groups
                .entry(g.clone())
                .or_default()
                .push(task_id.clone());
        }
        let record = TaskRecord::new(
            task_id.clone(),
            parent_conv.into(),
            child_conv.into(),
            group_id,
        );
        self.tasks.insert(task_id, record);
    }

    /// Attach the detached driver handle for an `await: false` task.
    pub fn set_join_handle(&self, task_id: &str, handle: JoinHandle<()>) {
        if let Some(mut rec) = self.tasks.get_mut(task_id) {
            rec.join_handle = Some(handle);
        }
    }

    /// Take (remove) the driver handle — used by cancel to abort the task.
    pub fn take_join_handle(&self, task_id: &str) -> Option<JoinHandle<()>> {
        self.tasks
            .get_mut(task_id)
            .and_then(|mut rec| rec.join_handle.take())
    }

    /// Record a task's terminal (or latest) outcome and status. When the new
    /// status is terminal, releases the parent's live slot exactly once (the
    /// prior status must have been non-terminal).
    pub fn set_outcome(&self, task_id: &str, outcome: TaskOutcome) {
        let Some(mut rec) = self.tasks.get_mut(task_id) else {
            return;
        };
        let was_terminal = rec.status.is_terminal();
        rec.status = outcome.status;
        let now_terminal = outcome.status.is_terminal();
        rec.outcome = Some(outcome);
        let parent = rec.parent_conv.clone();
        drop(rec);
        if now_terminal && !was_terminal {
            self.release(&parent);
        }
    }

    /// Transition status without a full outcome (e.g. `Cancelled`). Releases the
    /// live slot on the first terminal transition.
    pub fn set_status(&self, task_id: &str, status: TaskStatus) {
        let Some(mut rec) = self.tasks.get_mut(task_id) else {
            return;
        };
        let was_terminal = rec.status.is_terminal();
        rec.status = status;
        if let Some(ref mut out) = rec.outcome {
            out.status = status;
        }
        let parent = rec.parent_conv.clone();
        drop(rec);
        if status.is_terminal() && !was_terminal {
            self.release(&parent);
        }
    }

    /// The current status of a task, if tracked.
    pub fn status(&self, task_id: &str) -> Option<TaskStatus> {
        self.tasks.get(task_id).map(|r| r.status)
    }

    /// The child conversation id for a task, if tracked.
    pub fn child_conv(&self, task_id: &str) -> Option<String> {
        self.tasks.get(task_id).map(|r| r.child_conv.clone())
    }

    /// A snapshot outcome for a task. Synthesizes a bare outcome from the record
    /// when no full outcome has been stored yet (e.g. a `Running` task).
    pub fn outcome(&self, task_id: &str) -> Option<TaskOutcome> {
        let rec = self.tasks.get(task_id)?;
        Some(rec.outcome.clone().unwrap_or_else(|| TaskOutcome {
            task_id: rec.task_id.clone(),
            child_conv_id: rec.child_conv.clone(),
            status: rec.status,
            result: None,
            finish_reason: None,
            iterations: 0,
            group_id: rec.group_id.clone(),
            error: None,
        }))
    }

    /// Outcomes for every task in a swarm group (design D7).
    pub fn group_outcomes(&self, group_id: &str) -> Vec<TaskOutcome> {
        let Some(ids) = self.groups.get(group_id) else {
            return Vec::new();
        };
        ids.iter().filter_map(|id| self.outcome(id)).collect()
    }

    /// Task ids whose parent is `parent_conv` and which are still non-terminal —
    /// the parent-end cascade-cancel set (design D5).
    pub fn live_children(&self, parent_conv: &str) -> Vec<TaskId> {
        self.tasks
            .iter()
            .filter(|r| r.parent_conv == parent_conv && !r.status.is_terminal())
            .map(|r| r.task_id.clone())
            .collect()
    }

    /// Total tracked tasks (terminal + live).
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Whether the registry holds no tasks.
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

/// Shared handle type used across the spawner and daemon wiring.
pub type SharedSpawnRegistry = Arc<SpawnRegistry>;

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(task_id: &str, child: &str, status: TaskStatus, result: Option<&str>) -> TaskOutcome {
        TaskOutcome {
            task_id: task_id.into(),
            child_conv_id: child.into(),
            status,
            result: result.map(Into::into),
            finish_reason: Some("stop".into()),
            iterations: 1,
            group_id: None,
            error: None,
        }
    }

    #[test]
    fn insert_and_get_roundtrips() {
        let reg = SpawnRegistry::new();
        reg.insert("t1", "P", "sub:P:1", None);
        assert_eq!(reg.status("t1"), Some(TaskStatus::Running));
        assert_eq!(reg.child_conv("t1").as_deref(), Some("sub:P:1"));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn set_outcome_marks_terminal_and_stores_result() {
        let reg = SpawnRegistry::new();
        reg.try_reserve("P", 5).unwrap();
        reg.insert("t1", "P", "sub:P:1", None);
        reg.set_outcome("t1", outcome("t1", "sub:P:1", TaskStatus::Completed, Some("4")));
        let got = reg.outcome("t1").unwrap();
        assert_eq!(got.status, TaskStatus::Completed);
        assert_eq!(got.result.as_deref(), Some("4"));
        // Terminal transition released the reservation.
        assert_eq!(reg.live_count("P"), 0);
    }

    #[test]
    fn running_task_synthesizes_bare_outcome() {
        let reg = SpawnRegistry::new();
        reg.insert("t1", "P", "sub:P:1", None);
        let got = reg.outcome("t1").unwrap();
        assert_eq!(got.status, TaskStatus::Running);
        assert!(got.result.is_none());
        assert_eq!(got.child_conv_id, "sub:P:1");
    }

    #[test]
    fn group_outcomes_aggregate_siblings() {
        let reg = SpawnRegistry::new();
        reg.insert("t1", "P", "sub:P:1", Some("g".into()));
        reg.insert("t2", "P", "sub:P:2", Some("g".into()));
        reg.set_outcome("t1", outcome("t1", "sub:P:1", TaskStatus::Completed, Some("a")));
        let outs = reg.group_outcomes("g");
        assert_eq!(outs.len(), 2);
        assert!(outs.iter().any(|o| o.status == TaskStatus::Completed));
        assert!(outs.iter().any(|o| o.status == TaskStatus::Running));
    }

    #[test]
    fn concurrency_cap_refuses_the_sixth() {
        let reg = SpawnRegistry::new();
        for i in 0..5 {
            assert!(
                reg.try_reserve("P", 5).is_ok(),
                "reservation {i} should be admitted"
            );
            reg.insert(format!("t{i}"), "P", format!("sub:P:{i}"), None);
        }
        // The 6th live child is refused.
        assert_eq!(reg.try_reserve("P", 5), Err(5));
        assert_eq!(reg.live_count("P"), 5);

        // Completing one frees a slot for the next.
        reg.set_outcome("t0", outcome("t0", "sub:P:0", TaskStatus::Completed, Some("x")));
        assert_eq!(reg.live_count("P"), 4);
        assert!(reg.try_reserve("P", 5).is_ok());
    }

    #[test]
    fn cap_is_per_parent_conversation() {
        let reg = SpawnRegistry::new();
        reg.try_reserve("P", 1).unwrap();
        assert_eq!(reg.try_reserve("P", 1), Err(1));
        // A different parent has its own budget.
        assert!(reg.try_reserve("Q", 1).is_ok());
    }

    #[test]
    fn live_children_lists_only_non_terminal() {
        let reg = SpawnRegistry::new();
        reg.try_reserve("P", 5).unwrap();
        reg.try_reserve("P", 5).unwrap();
        reg.insert("t1", "P", "sub:P:1", None);
        reg.insert("t2", "P", "sub:P:2", None);
        reg.set_outcome("t1", outcome("t1", "sub:P:1", TaskStatus::Completed, Some("done")));
        let live = reg.live_children("P");
        assert_eq!(live, vec!["t2".to_string()]);
    }

    #[test]
    fn set_status_cancel_releases_slot_once() {
        let reg = SpawnRegistry::new();
        reg.try_reserve("P", 5).unwrap();
        reg.insert("t1", "P", "sub:P:1", None);
        reg.set_status("t1", TaskStatus::Cancelled);
        assert_eq!(reg.status("t1"), Some(TaskStatus::Cancelled));
        assert_eq!(reg.live_count("P"), 0);
        // Idempotent: a second terminal set does not underflow the counter.
        reg.set_status("t1", TaskStatus::Cancelled);
        assert_eq!(reg.live_count("P"), 0);
    }
}
