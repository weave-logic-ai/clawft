//! Audio playback (speaker output) via cpal.

/// Audio playback configuration.
#[derive(Debug, Clone)]
pub struct PlaybackConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub device_name: Option<String>,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            device_name: None,
        }
    }
}

/// Speaker audio playback stream.
///
/// Wraps cpal output stream. Currently a stub.
pub struct AudioPlayback {
    config: PlaybackConfig,
    active: bool,
}

impl AudioPlayback {
    pub fn new(config: PlaybackConfig) -> Self {
        Self { config, active: false }
    }

    pub fn start(&mut self) -> Result<(), String> {
        self.active = true;
        tracing::info!("Audio playback started (stub)");
        Ok(())
    }

    pub fn stop(&mut self) {
        self.active = false;
        tracing::info!("Audio playback stopped");
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn config(&self) -> &PlaybackConfig {
        &self.config
    }
}
