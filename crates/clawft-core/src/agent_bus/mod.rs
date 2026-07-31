//! Inter-agent communication bus and swarm coordination (WEFT-185).
//!
//! # Modules
//!
//! - [`bus`] — per-agent inboxes with TTL + capacity backpressure
//! - [`worker`] — [`worker_message_loop`] and pluggable handlers
//! - [`coordinator`] — [`SwarmCoordinator`] fan-out / collect + demo
//!
//! # Topology (WEFT-199)
//!
//! Runtime is flat fan-out ([`SwarmTopology::Flat`]). Mesh / hierarchical /
//! adaptive labels are reserved for claude-flow **prompt-only** swarm roles.
//! See `docs/architecture/swarm-topology.md`.
//!
//! # Production wiring
//!
//! Hosts construct an [`AgentBus`] (via [`SwarmCoordinator::with_capacity`]
//! or [`AgentBus::new`]), attach it with
//! [`AppContext::set_agent_bus`](crate::bootstrap::AppContext::set_agent_bus),
//! and spawn worker loops with [`spawn_worker_loop`] or
//! [`SwarmCoordinator::spawn_workers`].

mod bus;
mod coordinator;
mod worker;

pub use bus::{AgentBus, AgentInbox};
pub use coordinator::{run_swarm_demo, SwarmCoordinator, SwarmTopology};
pub use worker::{
    spawn_worker_loop, spawn_worker_loop_with_ttl, worker_message_loop, EchoWorker,
    RuntimeBackedWorker, WorkerHandler, WorkerOutcome, DEFAULT_REPLY_TTL,
};
