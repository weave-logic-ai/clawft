//! Windows path — cpal via WASAPI.
//!
//! Requirements:
//! - Microphone privacy enabled for desktop apps (Settings → Privacy →
//!   Microphone).
//! - No extra Cargo feature; WASAPI is the cpal default host on Windows.

pub fn host_backend() -> &'static str {
    "wasapi"
}

pub fn privacy_notes() -> &'static str {
    "Windows: cpal uses WASAPI. Enable Microphone access under Settings → \
     Privacy → Microphone for desktop apps."
}
