//! Linux sysfs platform probe for host-local network / bluetooth adapters.
//!
//! # WEFT-420 — honest Linux-only constraint
//!
//! The `network` and `bluetooth` reference adapters (and the related
//! `rfkill` sensor) read kernel sysfs trees under `/sys/class/*`. That
//! ABI is **Linux-only**. macOS has no `/sys`; Windows does not either.
//! Full CoreWLAN / IOBluetooth / WinRT ports are unscheduled.
//!
//! This module is the single source of truth for:
//!
//! 1. Compile-time platform support ([`linux_sysfs_native`]).
//! 2. Stable cause codes for adapter-health + topic payloads when the
//!    host is not Linux ([`CAUSE_SYSFS_UNAVAILABLE`]).
//! 3. Human-readable reason strings for the health topic.
//!
//! Adapters still **compile and open** on non-Linux hosts so the tray
//! and Explorer keep a uniform subscription surface. They emit
//! `absent` / disabled placeholders and surface
//! [`AdapterHealthEvent::Error`](crate::health::AdapterHealthEvent) with
//! a `sysfs-unavailable` reason so consumers can distinguish "no WiFi
//! hardware" from "this OS cannot probe it."

use crate::adapter::SubId;
use crate::delta::StateDelta;
use crate::health::{AdapterHealthEvent, build_event_delta};

/// Stable cause code written into topic payloads and health reasons
/// when the host OS has no Linux sysfs.
///
/// Wire-stable: tray / Explorer match this string, not a free-form
/// message. Do not rename without a migration note.
pub const CAUSE_SYSFS_UNAVAILABLE: &str = "sysfs-unavailable";

/// Platform tag for Linux builds that use real `/sys/class/*`.
pub const PLATFORM_LINUX_SYSFS: &str = "linux-sysfs";

/// Platform tag when the adapter is running on a non-Linux host (or
/// when tests force the unsupported path).
pub const PLATFORM_UNSUPPORTED: &str = "unsupported";

/// Compile-time: does this build target Linux, where `/sys/class/*`
/// is the kernel ABI?
///
/// This is a **feature gate by target OS**, not a Cargo feature. Cargo
/// features cannot express "only on Linux"; `cfg(target_os = "linux")`
/// is the honest gate. Runtime existence of a particular class (e.g.
/// no `wlan0` on a wired-only Linux box) is separate — that still
/// yields plain `absent` without a platform cause.
#[inline]
pub const fn linux_sysfs_native() -> bool {
    cfg!(target_os = "linux")
}

/// Platform tag for the current build target.
#[inline]
pub fn platform_tag() -> &'static str {
    if linux_sysfs_native() {
        PLATFORM_LINUX_SYSFS
    } else {
        PLATFORM_UNSUPPORTED
    }
}

/// Human-readable reason for adapter-health when sysfs is unavailable.
///
/// Includes the adapter id and the stable cause code so log greps and
/// health UIs can filter without parsing free text.
pub fn unsupported_reason(adapter_id: &str) -> String {
    format!(
        "{adapter_id}: {CAUSE_SYSFS_UNAVAILABLE} — host-local probe \
         requires Linux sysfs (/sys/class/*); this build target is not \
         linux. macOS/Windows variants are unscheduled (WEFT-420). \
         Emitting absent/disabled placeholders."
    )
}

/// Build an adapter-health `error` delta for the non-Linux fallback.
///
/// Path: `substrate/meta/adapter/<adapter_id>/health`. Carries
/// [`AdapterHealthEvent::Error`] with [`CAUSE_SYSFS_UNAVAILABLE`] in
/// the reason string. `sub_id` is the live subscription when known.
pub fn sysfs_unavailable_health_delta(
    adapter_id: &str,
    topic: &str,
    sub_id: Option<SubId>,
) -> StateDelta {
    let reason = unsupported_reason(adapter_id);
    build_event_delta(
        adapter_id,
        AdapterHealthEvent::Error,
        topic,
        sub_id,
        Some(reason.as_str()),
    )
}

/// JSON fields shared by unsupported-platform payloads.
///
/// Callers merge these into their topic-specific object so every
/// sysfs-backed topic uses the same keys:
///
/// - `platform`: [`PLATFORM_UNSUPPORTED`]
/// - `cause`: [`CAUSE_SYSFS_UNAVAILABLE`]
pub fn unsupported_payload_fields() -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    m.insert(
        "platform".into(),
        serde_json::Value::String(PLATFORM_UNSUPPORTED.into()),
    );
    m.insert(
        "cause".into(),
        serde_json::Value::String(CAUSE_SYSFS_UNAVAILABLE.into()),
    );
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn platform_tag_matches_compile_time_cfg() {
        if cfg!(target_os = "linux") {
            assert!(linux_sysfs_native());
            assert_eq!(platform_tag(), PLATFORM_LINUX_SYSFS);
        } else {
            assert!(!linux_sysfs_native());
            assert_eq!(platform_tag(), PLATFORM_UNSUPPORTED);
        }
    }

    #[test]
    fn cause_code_is_stable_kebab() {
        assert_eq!(CAUSE_SYSFS_UNAVAILABLE, "sysfs-unavailable");
    }

    #[test]
    fn unsupported_reason_includes_adapter_and_cause() {
        let r = unsupported_reason("network");
        assert!(r.contains("network"), "{r}");
        assert!(r.contains(CAUSE_SYSFS_UNAVAILABLE), "{r}");
        assert!(r.contains("WEFT-420"), "{r}");
    }

    #[test]
    fn health_delta_is_error_with_sysfs_reason() {
        let d = sysfs_unavailable_health_delta(
            "bluetooth",
            "substrate/bluetooth",
            Some(SubId(7)),
        );
        match d {
            StateDelta::Replace { path, value } => {
                assert_eq!(path, "substrate/meta/adapter/bluetooth/health");
                assert_eq!(value["event"], "error");
                assert_eq!(value["adapter"], "bluetooth");
                assert_eq!(value["topic"], "substrate/bluetooth");
                assert_eq!(value["sub_id"], 7);
                let reason = value["reason"].as_str().unwrap_or("");
                assert!(reason.contains(CAUSE_SYSFS_UNAVAILABLE), "{reason}");
            }
            other => panic!("expected Replace, got {other:?}"),
        }
    }

    #[test]
    fn unsupported_payload_fields_are_stable() {
        let f = unsupported_payload_fields();
        assert_eq!(
            f.get("platform"),
            Some(&Value::String(PLATFORM_UNSUPPORTED.into()))
        );
        assert_eq!(
            f.get("cause"),
            Some(&Value::String(CAUSE_SYSFS_UNAVAILABLE.into()))
        );
    }
}
