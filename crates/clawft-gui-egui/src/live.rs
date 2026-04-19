//! Live kernel state — two implementations behind one public surface:
//!
//! - **Native** (`cfg(not(target_arch = "wasm32"))`) — background
//!   `std::thread` hosts a single-threaded tokio runtime and pokes the
//!   daemon IPC socket through `clawft_rpc::DaemonClient`.
//! - **Wasm / webview** (`cfg(target_arch = "wasm32")`) — lives inside a
//!   VSCode / Cursor WebviewPanel. Cannot use Unix sockets; instead
//!   posts JSON-RPC-shaped messages through `acquireVsCodeApi()` and
//!   the extension host proxies them to the daemon.
//!
//! Both publish the same `Snapshot` through an `Arc<Live>` — the UI
//! layer is target-agnostic.

use std::sync::Arc;

use parking_lot::RwLock;
use serde_json::Value;

#[cfg(not(target_arch = "wasm32"))]
mod native_live;

#[cfg(target_arch = "wasm32")]
mod wasm_live;

/// Point-in-time view of everything the transport has learned.
#[derive(Clone, Default)]
pub struct Snapshot {
    pub connection: Connection,
    pub status: Option<Value>,
    pub processes: Option<Vec<Value>>,
    pub services: Option<Vec<Value>>,
    pub logs: Option<Vec<Value>>,
    pub last_error: Option<String>,
    /// Incremented every successful poll tick so the UI can detect freshness.
    pub tick: u64,
    /// Monotonic ms since app start for the most recent successful poll.
    /// `Instant` doesn't exist on wasm so we use `f64` (performance.now).
    pub last_tick_at_ms: Option<f64>,
    /// Round-trip duration of the previous successful poll in milliseconds.
    pub last_tick_dur_ms: Option<f64>,
}

#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub enum Connection {
    #[default]
    Connecting,
    Connected,
    Disconnected,
}

/// Commands the UI pushes to the transport (e.g. from the Terminal block).
#[derive(Debug)]
pub enum Command {
    /// Fire a raw RPC call. Response is delivered via the oneshot reply
    /// if present, otherwise dropped.
    Raw {
        method: String,
        params: Value,
        reply: Option<ReplyTx>,
    },
}

/// Reply channel for commands. `tokio::sync::oneshot` on native, a tiny
/// futures-channel on wasm where tokio isn't available.
#[cfg(not(target_arch = "wasm32"))]
pub type ReplyTx = tokio::sync::oneshot::Sender<Result<Value, String>>;
#[cfg(target_arch = "wasm32")]
pub type ReplyTx = futures::channel::oneshot::Sender<Result<Value, String>>;

#[cfg(not(target_arch = "wasm32"))]
pub type ReplyRx = tokio::sync::oneshot::Receiver<Result<Value, String>>;
#[cfg(target_arch = "wasm32")]
pub type ReplyRx = futures::channel::oneshot::Receiver<Result<Value, String>>;

/// Create a new oneshot reply channel (target-agnostic).
pub fn reply_channel() -> (ReplyTx, ReplyRx) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::sync::oneshot::channel()
    }
    #[cfg(target_arch = "wasm32")]
    {
        futures::channel::oneshot::channel()
    }
}

/// Non-blocking receive attempt on a [`ReplyRx`]. Unifies the slightly
/// different APIs tokio and futures provide.
pub enum TryReply<T> {
    Empty,
    Closed,
    Done(T),
}

pub fn try_recv_reply(rx: &mut ReplyRx) -> TryReply<Result<Value, String>> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match rx.try_recv() {
            Ok(v) => TryReply::Done(v),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => TryReply::Empty,
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => TryReply::Closed,
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        use futures::channel::oneshot::Canceled;
        match rx.try_recv() {
            Ok(Some(v)) => TryReply::Done(v),
            Ok(None) => TryReply::Empty,
            Err(Canceled) => TryReply::Closed,
        }
    }
}

/// The public handle.
///
/// On native, `spawn()` starts a std::thread hosting tokio. On wasm,
/// `spawn()` registers a `message` event listener on the window and
/// wires `postMessage` to the VSCode extension host.
pub struct Live {
    inner: RwLock<Snapshot>,
    #[cfg(not(target_arch = "wasm32"))]
    cmd_tx: tokio::sync::mpsc::Sender<Command>,
    #[cfg(target_arch = "wasm32")]
    bridge: wasm_live::Bridge,
}

/// Monotonic milliseconds since app start (cross-target).
pub fn now_ms() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::OnceLock;
        use std::time::Instant;
        static APP_START: OnceLock<Instant> = OnceLock::new();
        let t0 = *APP_START.get_or_init(Instant::now);
        Instant::now().duration_since(t0).as_secs_f64() * 1000.0
    }
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.performance())
            .map(|p| p.now())
            .unwrap_or(0.0)
    }
}

impl Live {
    pub fn spawn() -> Arc<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            native_live::spawn()
        }
        #[cfg(target_arch = "wasm32")]
        {
            wasm_live::spawn()
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        self.inner.read().clone()
    }

    pub fn submit(&self, cmd: Command) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.cmd_tx.try_send(cmd).is_ok()
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.bridge.submit(cmd)
        }
    }

    #[allow(dead_code)]
    pub(crate) fn write(&self, mut f: impl FnMut(&mut Snapshot)) {
        f(&mut self.inner.write());
    }
}
