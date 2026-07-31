//! macOS (CoreAudio via cpal) path — primary target for WEFT-228.
//!
//! Requirements:
//! - `NSMicrophoneUsageDescription` in the app Info.plist (set in
//!   `tauri.conf.json` → `bundle.macOS.infoPlist`) so the system privacy
//!   prompt can appear the first time capture opens.
//! - User must grant Microphone access under System Settings → Privacy &
//!   Security → Microphone for the ClawFT app.

pub fn host_backend() -> &'static str {
    "coreaudio"
}

pub fn privacy_notes() -> &'static str {
    "macOS: cpal uses CoreAudio. Grant Microphone access in System Settings \
     → Privacy & Security → Microphone. Bundle includes NSMicrophoneUsageDescription."
}
