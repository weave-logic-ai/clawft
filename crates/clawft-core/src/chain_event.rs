//! Lightweight chain event markers for crates without direct
//! [`ChainManager`](https://docs.rs/clawft-kernel) access.
//!
//! Since `clawft-core` does not depend on `clawft-kernel`, it cannot
//! call `ChainManager::append()` directly.  Instead, state-modifying
//! methods emit **structured tracing events** at `INFO` level using the
//! `chain_event!` macro.  The daemon layer (`clawft-weave`) subscribes
//! to the `chain_event` target and forwards matching spans to the real
//! ExoChain via `ChainManager::append()`.
//!
//! This design keeps `clawft-core` free of kernel dependencies while
//! still providing a consistent chain-logging surface.
//!
//! # Event kind constants
//!
//! These mirror the `EVENT_KIND_*` constants in `clawft-kernel::chain`
//! so that producers and consumers share a vocabulary without a compile
//! dependency.

/// Sandbox enforcement check.
pub const EVENT_KIND_SANDBOX_EXECUTE: &str = "sandbox.execute";

/// Session created.
pub const EVENT_KIND_SESSION_CREATE: &str = "session.create";

/// Session destroyed.
pub const EVENT_KIND_SESSION_DESTROY: &str = "session.destroy";

/// Workspace created.
pub const EVENT_KIND_WORKSPACE_CREATE: &str = "workspace.create";

/// Workspace config loaded / merged.
pub const EVENT_KIND_WORKSPACE_CONFIG: &str = "workspace.config";

/// Tool registered in the ToolRegistry.
pub const EVENT_KIND_TOOL_REGISTER: &str = "tool.register";

/// Emit a structured tracing event on the `chain_event` target.
///
/// The daemon's tracing subscriber can filter on
/// `target == "chain_event"` and forward to `ChainManager::append`.
///
/// # Usage
///
/// ```rust,ignore
/// chain_event!("session", "session.create", { "key": key });
/// ```
#[macro_export]
macro_rules! chain_event {
    ($source:expr, $kind:expr, { $($key:tt : $val:expr),* $(,)? }) => {
        tracing::info!(
            target: "chain_event",
            source = $source,
            kind = $kind,
            $( $key = %$val, )*
            "chain"
        );
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_expected_values() {
        assert_eq!(EVENT_KIND_SANDBOX_EXECUTE, "sandbox.execute");
        assert_eq!(EVENT_KIND_SESSION_CREATE, "session.create");
        assert_eq!(EVENT_KIND_SESSION_DESTROY, "session.destroy");
        assert_eq!(EVENT_KIND_WORKSPACE_CREATE, "workspace.create");
        assert_eq!(EVENT_KIND_WORKSPACE_CONFIG, "workspace.config");
        assert_eq!(EVENT_KIND_TOOL_REGISTER, "tool.register");
    }

    #[test]
    fn macro_compiles() {
        // Just verify the macro expands without errors.
        chain_event!("test", "test.event", { "foo": "bar" });
    }
}
