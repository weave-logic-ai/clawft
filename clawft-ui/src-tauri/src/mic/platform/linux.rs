//! Linux path — cpal via ALSA (and often PulseAudio/PipeWire through ALSA).
//!
//! Requirements:
//! - Dev headers for linking: `libasound2-dev` (Debian/Ubuntu) or
//!   `alsa-lib-devel` (Fedora/RHEL).
//! - Runtime: working ALSA/Pulse/PipeWire device; user in `audio` group if
//!   needed.
//! - No macOS-style Info.plist; sandboxed Flatpak/Snap builds may need
//!   explicit `--device=all` or Pulse socket permissions.

pub fn host_backend() -> &'static str {
    "alsa"
}

pub fn privacy_notes() -> &'static str {
    "Linux: cpal uses ALSA (Pulse/PipeWire via ALSA plugin). Build needs \
     libasound2-dev. Sandboxed packages may need device/socket permissions."
}
