//! Physical-sensor adapter trait layer.
//!
//! Every physical device WeftOS integrates (microphone, camera, radar,
//! speaker, geiger tube, load cell, GPS, IMU, …) wraps its raw sample
//! stream in an [`OntologyAdapter`] emitting `substrate/sensor/<kind>/*`
//! topics. The extension trait [`PhysicalSensorAdapter`] adds the
//! hardware-specific metadata those adapters share: bus / interface,
//! units, range, calibration, and — critically — a declared
//! [`Characterization`] level.
//!
//! ## The spectrometer principle (see
//! [feedback memory][gcm])
//!
//! [gcm]: ../../../.claude/projects/-home-aepod-dev-clawft/memory/feedback_spectrometer_principle.md
//!
//! A geiger counter CAN'T identify isotopes — it can only count events
//! ("ionisation is happening"). A spectrometer CAN characterize what's
//! there. Every adapter must honestly declare which side of that line
//! its data source sits on, so the UI doesn't overpromise ("chip green
//! = all good" is dishonest if the underlying signal can only tell
//! presence).
//!
//! | Level | Meaning | Example |
//! |-------|---------|---------|
//! | [`Characterization::Presence`] | Binary "something is happening" | PIR motion sensor, geiger tube, limit switch |
//! | [`Characterization::Rate`] | A scalar rate / magnitude, no structure | Mic RMS level, CPM, anemometer RPM |
//! | [`Characterization::Enumerated`] | One of N discrete states | Rfkill soft/hard/off, HC-SR04 near/mid/far |
//! | [`Characterization::Spectral`] | Structured distribution over a basis | Mic FFT bins, colour sensor RGB triplet, thermal image |
//! | [`Characterization::Identifying`] | Names / classifies the underlying phenomenon | NFC UID, fingerprint match, object-detection label |
//!
//! `Rate` data can't honestly render as "everything is fine / something
//! is wrong" without a threshold — chips driven by rate-only data
//! should show the number, not a colour verdict. `Presence` data
//! should render as binary. `Enumerated` / `Spectral` / `Identifying`
//! unlock richer rendering options.
//!
//! ## What this module is not
//!
//! - Not a device registry. The adapter *owns* its interface
//!   declaration; there is no central "plug-and-play" DB mapping
//!   topic → hardware. That's future work (ADR-020+) with its own
//!   resolution protocol.
//! - Not a calibration database. Each adapter carries its own
//!   factory-default calibration; per-unit tuning is out of scope
//!   for this preview.

use async_trait::async_trait;

use crate::adapter::OntologyAdapter;

/// Epistemic resolution of a physical sensor — what question its
/// output can honestly answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Characterization {
    /// Binary — "is something happening?"
    Presence,
    /// Scalar rate or magnitude — "how much is happening?" — but with
    /// no structure that distinguishes sources from each other.
    Rate,
    /// One of a small set of discrete states.
    Enumerated,
    /// Structured distribution over a basis — frequency bins, colour
    /// channels, spatial grids.
    Spectral,
    /// Names the phenomenon — classification, match, UID.
    Identifying,
}

impl Characterization {
    /// Short stable identifier suitable for serialising into a
    /// substrate value, tray tooltip, etc.
    pub fn as_str(self) -> &'static str {
        match self {
            Characterization::Presence => "presence",
            Characterization::Rate => "rate",
            Characterization::Enumerated => "enumerated",
            Characterization::Spectral => "spectral",
            Characterization::Identifying => "identifying",
        }
    }
}

/// Physical interface a sensor hangs off of. Deliberately coarse —
/// a USB-UVC webcam and an I²C camera module both reduce to
/// something the OS can open; finer-grained bus/pin declarations
/// belong to the per-adapter configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SensorInterface {
    /// Single GPIO pin (ADC / digital pulse / PWM).
    Gpio {
        /// Pin / channel index.
        pin: u32,
    },
    /// I²C device at `address` on `bus`.
    I2c {
        /// 7-bit I²C address.
        address: u8,
        /// Bus index (0, 1, …).
        bus: u8,
    },
    /// SPI chip-select pin on `bus`.
    Spi {
        /// Chip-select GPIO pin.
        cs_pin: u32,
        /// SPI bus index.
        bus: u8,
    },
    /// UART / serial port.
    Uart {
        /// Port index.
        port: u8,
        /// Baud rate.
        baud: u32,
    },
    /// I²S digital audio.
    I2s {
        /// Bit-clock pin.
        bck: u32,
        /// Word-select / LR-clock pin.
        ws: u32,
        /// Data pin.
        data: u32,
    },
    /// USB device referenced by vendor/product or by a path.
    Usb {
        /// `/dev/videoN`, `/dev/input/eventN`, etc.
        device_path: String,
    },
    /// OS-provided CoreAudio / ALSA / WASAPI / CPAL handle — the
    /// adapter just asks the host for a stream; the pins don't
    /// surface.
    HostAudio {
        /// Capture or playback.
        direction: AudioDirection,
    },
    /// File-backed fake source — used by tests and by the preview
    /// stub adapters before real hardware is wired. Path identifies
    /// a raw byte stream in whatever format the adapter expects.
    FileBacked {
        /// Path to the backing file.
        path: std::path::PathBuf,
    },
}

/// Direction of a host-audio stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioDirection {
    /// Capture (microphone → host).
    Capture,
    /// Playback (host → speaker).
    Playback,
}

/// Per-sensor calibration — factory-default values for converting a
/// raw reading into a real-world unit. Enough for this preview;
/// per-unit tuning + a calibration store is future work.
#[derive(Debug, Clone)]
pub struct SensorCalibration {
    /// Multiplicative factor: `real = scale * raw + offset`.
    pub scale: f64,
    /// Additive offset (applied after scaling).
    pub offset: f64,
    /// Human-readable reference for where the numbers came from
    /// (e.g. `"SBM-20 tube: 1 CPM ≈ 0.0057 µSv/h"`). Optional.
    pub reference: Option<String>,
}

impl Default for SensorCalibration {
    fn default() -> Self {
        Self {
            scale: 1.0,
            offset: 0.0,
            reference: None,
        }
    }
}

/// Physical-sensor-flavoured extension of [`OntologyAdapter`].
///
/// Adapters implementing this trait plug into the substrate exactly
/// like any other ontology adapter — but also expose the
/// hardware-specific metadata a tray / admin UI needs to render
/// them honestly (model name, unit, characterization level).
#[async_trait]
pub trait PhysicalSensorAdapter: OntologyAdapter {
    /// Human-readable model name — e.g. `"INMP441 I²S MEMS mic"`,
    /// `"OV2640 2MP camera"`, `"MightyOhm SBM-20 geiger"`.
    fn model(&self) -> &'static str;

    /// Physical interface declaration.
    fn interface(&self) -> SensorInterface;

    /// Unit of the primary reading — `"dBFS"`, `"CPM"`, `"°C"`,
    /// `"lux"`, `"mm"`, `"RGB"`. Opaque string so exotic units don't
    /// need a trait-level enum.
    fn unit(&self) -> &'static str;

    /// Valid reading range (for calibration display / clamping).
    /// `(lo, hi)`; may be `(f64::NEG_INFINITY, f64::INFINITY)` for
    /// open-ended sensors.
    fn range(&self) -> (f64, f64);

    /// Factory-default calibration.
    fn calibration(&self) -> SensorCalibration;

    /// What can this sensor honestly tell us? See [`Characterization`].
    fn characterization(&self) -> Characterization;
}
