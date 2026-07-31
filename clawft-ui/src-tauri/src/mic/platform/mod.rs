//! Per-platform notes and helpers for native mic capture (WEFT-228).
//!
//! Capture itself is driven by **cpal** (CoreAudio on macOS, ALSA/Pulse on
//! Linux, WASAPI on Windows). Platform modules document host requirements and
//! expose a small host-id string for the UI / diagnostics.

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
mod other {
    pub fn host_backend() -> &'static str {
        "cpal-unknown"
    }

    pub fn privacy_notes() -> &'static str {
        "Native mic capture is untested on this OS. cpal may or may not find a host."
    }
}

/// Short identifier for the audio host in use on this OS.
pub fn host_backend() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        macos::host_backend()
    }
    #[cfg(target_os = "linux")]
    {
        linux::host_backend()
    }
    #[cfg(target_os = "windows")]
    {
        windows::host_backend()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        other::host_backend()
    }
}

/// Human-readable notes about permissions / packages needed on this platform.
pub fn privacy_notes() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        macos::privacy_notes()
    }
    #[cfg(target_os = "linux")]
    {
        linux::privacy_notes()
    }
    #[cfg(target_os = "windows")]
    {
        windows::privacy_notes()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        other::privacy_notes()
    }
}

/// OS name reported to the UI (`std::env::consts::OS`).
pub fn target_os() -> &'static str {
    std::env::consts::OS
}
