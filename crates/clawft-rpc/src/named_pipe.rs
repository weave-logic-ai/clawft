//! Windows named-pipe server helpers for the kernel daemon (WEFT-11).
//!
//! The client side lives in [`crate::client`]. These helpers bind a
//! named pipe corresponding to a *logical* [`crate::socket_path`] so
//! the daemon accept loop can share path derivation with the client.
//!
//! # Status
//!
//! - **Client**: implemented (`DaemonClient` under `cfg(windows)`).
//! - **Server helpers**: implemented here; compile on Windows.
//! - **Daemon accept-loop wiring** in `clawft-weave::daemon`: wired
//!   under `cfg(windows)` (WEFT-559) via `create_listener` /
//!   `create_listener_next` into the transport-agnostic
//!   `handle_connection` / `handle_json_connection` path.
//!
//! # Accept-loop sketch
//!
//! ```ignore
//! use clawft_rpc::named_pipe::{create_listener, create_listener_next};
//!
//! let mut server = create_listener(&socket_path)?;
//! loop {
//!     server.connect().await?;
//!     let connected = server;
//!     server = create_listener_next(&socket_path)?;
//!     tokio::spawn(handle_json_connection(/* ... */, connected, /* ... */));
//! }
//! ```

use std::path::Path;

use crate::protocol::pipe_name_for_path;

/// Resolve the Windows named-pipe name for a logical socket path.
///
/// Thin wrapper over [`pipe_name_for_path`] for daemon-side call sites.
pub fn pipe_name(logical_socket_path: impl AsRef<Path>) -> String {
    pipe_name_for_path(logical_socket_path)
}

// ── Windows implementation ───────────────────────────────────────

#[cfg(windows)]
mod imp {
    use std::io;
    use std::path::Path;

    use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};

    use super::pipe_name;

    /// Create the **first** named-pipe server instance for `logical_socket_path`.
    ///
    /// Uses `first_pipe_instance(true)` so a second daemon cannot silently
    /// steal the endpoint. Subsequent accept-loop instances must use
    /// [`create_listener_next`].
    pub fn create_listener(logical_socket_path: impl AsRef<Path>) -> io::Result<NamedPipeServer> {
        let name = pipe_name(logical_socket_path.as_ref());
        ServerOptions::new()
            .first_pipe_instance(true)
            .create(name)
    }

    /// Create a subsequent named-pipe server instance (accept-loop body).
    ///
    /// Call after a previous instance has accepted a client and been
    /// moved into a connection handler.
    pub fn create_listener_next(
        logical_socket_path: impl AsRef<Path>,
    ) -> io::Result<NamedPipeServer> {
        let name = pipe_name(logical_socket_path.as_ref());
        ServerOptions::new().create(name)
    }
}

// ── Non-Windows stubs (so call sites can be written behind cfg) ──

#[cfg(not(windows))]
mod imp {
    use std::io;
    use std::path::Path;

    /// Placeholder type so non-Windows builds can type-check daemon
    /// wiring drafts behind `cfg(windows)` without pulling Win32 types.
    #[derive(Debug)]
    pub struct NamedPipeServer;

    /// Always errors off Windows — use Unix domain sockets instead.
    pub fn create_listener(_logical_socket_path: impl AsRef<Path>) -> io::Result<NamedPipeServer> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "named-pipe server is only available on Windows (WEFT-11)",
        ))
    }

    /// Always errors off Windows — use Unix domain sockets instead.
    pub fn create_listener_next(
        _logical_socket_path: impl AsRef<Path>,
    ) -> io::Result<NamedPipeServer> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "named-pipe server is only available on Windows (WEFT-11)",
        ))
    }
}

pub use imp::{NamedPipeServer, create_listener, create_listener_next};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::pipe_name_for_path;

    #[test]
    fn pipe_name_matches_protocol_helper() {
        let path = std::path::Path::new("/tmp/weft-test/kernel.sock");
        assert_eq!(pipe_name(path), pipe_name_for_path(path));
    }

    #[test]
    #[cfg(not(windows))]
    fn create_listener_errors_off_windows() {
        let err = create_listener("/tmp/nope/kernel.sock").unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::Unsupported);
    }
}
