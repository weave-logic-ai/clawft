//! ExoChain attestation encode / decode + append path (WEFT-533).
//!
//! Host builds (`feature = "std"`) serialize [`ObservationTuple`] to a
//! version-tagged JSON payload suitable for `ChainManager::append` or an
//! in-process [`MemoryChainSink`] used by the world-model service and tests.
//!
//! The world-model crates stay free of a hard `clawft-kernel` dependency;
//! the service (or daemon bridge) forwards payloads into the real ExoChain.

#![cfg(feature = "std")]

use serde::{Deserialize, Serialize};

use weftos_worldmodel_core::{
    Action, Latent, LatentVersion, ObservationTuple, ATTESTATION_SOURCE,
    EVENT_KIND_LEWM_FRAME_ATTESTATION, LATENT_DIM, LATENT_DIM_U16,
};

/// Wire payload for a single frame attestation on ExoChain.
///
/// Arrays are stored as `Vec<f32>` so serde_json round-trips cleanly without
/// const-generic array support. Decode validates width against the SIGReg
/// contract before reconstructing [`ObservationTuple`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttestationPayload {
    /// Schema major for this payload shape (independent of manifold major).
    pub schema_major: u16,
    /// Schema minor (additive fields only).
    pub schema_minor: u16,
    /// SIGReg manifold major (1 ⇒ 192-dim isotropic Gaussian).
    pub manifold_major: u16,
    /// SIGReg manifold minor.
    pub manifold_minor: u16,
    /// Declared latent width.
    pub latent_dim: u16,
    /// Dense action code `a_t`.
    pub a_t: Vec<f32>,
    /// Latent `z_t`.
    pub z_t: Vec<f32>,
    /// Observed `z_{t+1}`.
    pub z_tp1: Vec<f32>,
    /// Predicted `ẑ_{t+1}`.
    pub z_hat: Vec<f32>,
    /// VoE surprise scalar.
    pub surprise: f32,
    /// Observation timestamp (ms).
    pub timestamp_ms: u64,
    /// Frame sequence.
    pub frame_seq: u64,
}

/// Payload schema major for [`AttestationPayload`].
pub const ATTESTATION_SCHEMA_MAJOR: u16 = 1;
/// Payload schema minor.
pub const ATTESTATION_SCHEMA_MINOR: u16 = 0;

impl AttestationPayload {
    /// Encode an observational tuple into a wire payload.
    pub fn from_tuple(t: &ObservationTuple) -> Self {
        Self {
            schema_major: ATTESTATION_SCHEMA_MAJOR,
            schema_minor: ATTESTATION_SCHEMA_MINOR,
            manifold_major: t.manifold.major,
            manifold_minor: t.manifold.minor,
            latent_dim: t.latent_dim,
            a_t: t.a_t.code.to_vec(),
            z_t: t.z_t.to_vec(),
            z_tp1: t.z_tp1.to_vec(),
            z_hat: t.z_hat.to_vec(),
            surprise: t.surprise,
            timestamp_ms: t.timestamp_ms,
            frame_seq: t.frame_seq,
        }
    }

    /// Decode back to an [`ObservationTuple`], validating latent widths.
    pub fn to_tuple(&self) -> Result<ObservationTuple, AttestationError> {
        if self.latent_dim != LATENT_DIM_U16 {
            return Err(AttestationError::LatentDim {
                expected: LATENT_DIM_U16,
                got: self.latent_dim,
            });
        }
        let a_t = array_from_slice_32(&self.a_t)?;
        let z_t = array_from_slice_latent(&self.z_t)?;
        let z_tp1 = array_from_slice_latent(&self.z_tp1)?;
        let z_hat = array_from_slice_latent(&self.z_hat)?;
        Ok(ObservationTuple {
            a_t: Action { code: a_t },
            z_t,
            z_tp1,
            z_hat,
            surprise: self.surprise,
            manifold: LatentVersion {
                major: self.manifold_major,
                minor: self.manifold_minor,
            },
            latent_dim: self.latent_dim,
            timestamp_ms: self.timestamp_ms,
            frame_seq: self.frame_seq,
        })
    }

    /// Serialize to JSON bytes for chain storage.
    pub fn to_json_bytes(&self) -> Result<Vec<u8>, AttestationError> {
        serde_json::to_vec(self).map_err(|e| AttestationError::Serialize(e.to_string()))
    }

    /// Deserialize from JSON bytes.
    pub fn from_json_bytes(bytes: &[u8]) -> Result<Self, AttestationError> {
        serde_json::from_slice(bytes).map_err(|e| AttestationError::Deserialize(e.to_string()))
    }
}

fn array_from_slice_32(v: &[f32]) -> Result<[f32; 32], AttestationError> {
    if v.len() != 32 {
        return Err(AttestationError::ActionDim {
            expected: 32,
            got: v.len(),
        });
    }
    let mut out = [0.0f32; 32];
    out.copy_from_slice(v);
    Ok(out)
}

fn array_from_slice_latent(v: &[f32]) -> Result<Latent, AttestationError> {
    if v.len() != LATENT_DIM {
        return Err(AttestationError::LatentVecDim {
            expected: LATENT_DIM,
            got: v.len(),
        });
    }
    let mut out = [0.0f32; LATENT_DIM];
    out.copy_from_slice(v);
    Ok(out)
}

/// Errors from attestation encode / decode / append.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationError {
    /// Latent dim field mismatch.
    LatentDim {
        /// Expected SIGReg width.
        expected: u16,
        /// Width present on the payload.
        got: u16,
    },
    /// Latent vector length mismatch.
    LatentVecDim {
        /// Expected vector length ([`weftos_worldmodel_core::LATENT_DIM`]).
        expected: usize,
        /// Actual vector length.
        got: usize,
    },
    /// Action code length mismatch.
    ActionDim {
        /// Expected action code length (32).
        expected: usize,
        /// Actual length.
        got: usize,
    },
    /// JSON serialize failure.
    Serialize(String),
    /// JSON deserialize failure.
    Deserialize(String),
    /// Chain sink rejected the append.
    Sink(String),
}

impl core::fmt::Display for AttestationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::LatentDim { expected, got } => {
                write!(f, "latent_dim mismatch: expected {expected}, got {got}")
            }
            Self::LatentVecDim { expected, got } => {
                write!(f, "latent vector len: expected {expected}, got {got}")
            }
            Self::ActionDim { expected, got } => {
                write!(f, "action code len: expected {expected}, got {got}")
            }
            Self::Serialize(s) => write!(f, "serialize: {s}"),
            Self::Deserialize(s) => write!(f, "deserialize: {s}"),
            Self::Sink(s) => write!(f, "chain sink: {s}"),
        }
    }
}

impl std::error::Error for AttestationError {}

/// One chain entry produced by an attestation append.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainEntry {
    /// Monotonic sequence assigned by the sink.
    pub seq: u64,
    /// Event kind (`lewm.frame.attestation`).
    pub kind: String,
    /// Source subsystem.
    pub source: String,
    /// Serialized JSON payload bytes.
    pub payload: Vec<u8>,
}

/// Minimal append-only sink used by the world-model service and unit tests.
///
/// Production hosts implement this over `ChainManager::append`; the service
/// defaults to [`MemoryChainSink`] so single-node boots need no kernel.
pub trait ChainSink {
    /// Append a pre-encoded attestation payload; returns assigned sequence.
    fn append_attestation(&mut self, payload: &AttestationPayload) -> Result<u64, AttestationError>;
}

/// In-memory ExoChain stand-in for service smoke tests and offline replay.
#[derive(Debug, Default, Clone)]
pub struct MemoryChainSink {
    /// Appended entries in order.
    pub entries: Vec<ChainEntry>,
    next_seq: u64,
}

impl MemoryChainSink {
    /// Empty chain starting at sequence 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of attested frames.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True when no frames have been attested.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Decode entry `i` back to an [`ObservationTuple`].
    pub fn decode_tuple(&self, i: usize) -> Result<ObservationTuple, AttestationError> {
        let entry = self
            .entries
            .get(i)
            .ok_or_else(|| AttestationError::Sink(format!("missing entry {i}")))?;
        let payload = AttestationPayload::from_json_bytes(&entry.payload)?;
        payload.to_tuple()
    }
}

impl ChainSink for MemoryChainSink {
    fn append_attestation(&mut self, payload: &AttestationPayload) -> Result<u64, AttestationError> {
        let bytes = payload.to_json_bytes()?;
        let seq = self.next_seq;
        self.next_seq = self.next_seq.saturating_add(1);
        self.entries.push(ChainEntry {
            seq,
            kind: EVENT_KIND_LEWM_FRAME_ATTESTATION.to_string(),
            source: ATTESTATION_SOURCE.to_string(),
            payload: bytes,
        });
        Ok(seq)
    }
}

/// Attest one observational tuple onto a [`ChainSink`].
///
/// Validates the version-tagged SIGReg manifold reference before append.
pub fn attest_frame<S: ChainSink>(
    sink: &mut S,
    tuple: &ObservationTuple,
) -> Result<u64, AttestationError> {
    if !tuple.manifold_matches_local() {
        return Err(AttestationError::LatentDim {
            expected: LATENT_DIM_U16,
            got: tuple.latent_dim,
        });
    }
    let payload = AttestationPayload::from_tuple(tuple);
    sink.append_attestation(&payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use weftos_worldmodel_core::{surprise_voe, zero_latent};

    #[test]
    fn payload_roundtrip_preserves_tuple() {
        let mut z_t = zero_latent();
        z_t[0] = 0.5;
        let mut z_hat = zero_latent();
        z_hat[0] = 0.7;
        let mut z_tp1 = zero_latent();
        z_tp1[0] = 0.6;
        let mut a = Action::null();
        a.code[0] = 1.0;

        let t = ObservationTuple::new(a, z_t, z_hat, z_tp1, 99, 7);
        assert!((t.surprise - surprise_voe(&z_hat, &z_tp1)).abs() < 1e-6);

        let payload = AttestationPayload::from_tuple(&t);
        assert_eq!(payload.manifold_major, 1);
        assert_eq!(payload.latent_dim, 192);
        let bytes = payload.to_json_bytes().expect("json");
        let decoded = AttestationPayload::from_json_bytes(&bytes)
            .expect("decode")
            .to_tuple()
            .expect("tuple");
        assert_eq!(decoded.frame_seq, 7);
        assert_eq!(decoded.timestamp_ms, 99);
        assert_eq!(decoded.a_t.code[0], 1.0);
        assert!((decoded.z_t[0] - 0.5).abs() < 1e-6);
        assert!((decoded.surprise - t.surprise).abs() < 1e-6);
        assert!(decoded.manifold_matches_local());
    }

    #[test]
    fn memory_chain_append_and_decode() {
        let mut sink = MemoryChainSink::new();
        let t = ObservationTuple::new(
            Action::null(),
            zero_latent(),
            zero_latent(),
            zero_latent(),
            1,
            0,
        );
        let seq = attest_frame(&mut sink, &t).expect("attest");
        assert_eq!(seq, 0);
        assert_eq!(sink.len(), 1);
        assert_eq!(sink.entries[0].kind, EVENT_KIND_LEWM_FRAME_ATTESTATION);
        assert_eq!(sink.entries[0].source, ATTESTATION_SOURCE);
        let back = sink.decode_tuple(0).expect("decode");
        assert_eq!(back.frame_seq, 0);
        assert_eq!(back.surprise, 0.0);
    }

    #[test]
    fn rejects_wrong_latent_dim_on_payload() {
        let mut payload = AttestationPayload::from_tuple(&ObservationTuple::new(
            Action::null(),
            zero_latent(),
            zero_latent(),
            zero_latent(),
            0,
            0,
        ));
        payload.latent_dim = 64;
        assert!(matches!(
            payload.to_tuple(),
            Err(AttestationError::LatentDim { got: 64, .. })
        ));
    }
}
