//! ECC RVF segment type definitions (WEFT-508).
//!
//! Upstream `rvf-types::SegmentType` covers vector/index/federation codes
//! (`0x00`–`0x36`). WeftOS ECC structures need **domain-local** discriminants
//! that do not collide with:
//!
//! - rvf-types (`≤ 0x36`)
//! - context-router research reservations (`0x40`–`0x42`)
//!
//! We allocate the **`0x50`–`0x5F`** block for ECC persistence payloads.
//! Each variant wraps a CBOR (preferred) or JSON body; this module defines
//! the type table + encode/decode helpers. Full migration of on-disk chain
//! blobs onto these segments is follow-up work (see migration notes).
//!
//! # Segment table
//!
//! | Code   | Name                 | Payload                                      |
//! |--------|----------------------|----------------------------------------------|
//! | `0x50` | `CausalEdge`         | [`CausalEdge`](crate::causal::CausalEdge)    |
//! | `0x51` | `CausalNode`         | [`CausalNode`](crate::causal::CausalNode)    |
//! | `0x52` | `Impulse`            | [`Impulse`](crate::impulse::Impulse)         |
//! | `0x53` | `CalibrationProfile` | JSON profile blob (calibration / democritus) |
//! | `0x54` | `CrossRef`           | [`CrossRef`](crate::crossref::CrossRef)      |
//! | `0x55` | `SpectralCheckpoint` | λ₂ / eigenvalue snapshot                     |
//! | `0x56` | `CausalGraphSnap`    | Whole-graph snapshot envelope                |
//!
//! # Migration plan
//!
//! 1. **Today**: ad-hoc JSON / ExoChain CBOR events remain authoritative.
//! 2. **Writers** may dual-write ECC segments alongside chain events.
//! 3. **Readers** prefer ECC segments when present; fall back to legacy.
//! 4. After one release cycle, stop writing legacy-only forms for new events.

use serde::{Deserialize, Serialize};

/// WeftOS ECC segment discriminants (RVF `type` byte, domain `0x50`–`0x5F`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EccSegmentType {
    /// Directed causal edge record.
    CausalEdge = 0x50,
    /// Causal graph node record.
    CausalNode = 0x51,
    /// Ephemeral impulse / causal event.
    Impulse = 0x52,
    /// Calibration or Democritus profile blob.
    CalibrationProfile = 0x53,
    /// Cross-tree reference.
    CrossRef = 0x54,
    /// Spectral analysis checkpoint (λ₂, eigenvalues).
    SpectralCheckpoint = 0x55,
    /// Full causal-graph snapshot envelope.
    CausalGraphSnap = 0x56,
}

impl EccSegmentType {
    /// Stable name for logs / docs.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CausalEdge => "ecc.causal_edge",
            Self::CausalNode => "ecc.causal_node",
            Self::Impulse => "ecc.impulse",
            Self::CalibrationProfile => "ecc.calibration_profile",
            Self::CrossRef => "ecc.cross_ref",
            Self::SpectralCheckpoint => "ecc.spectral_checkpoint",
            Self::CausalGraphSnap => "ecc.causal_graph_snap",
        }
    }

    /// All defined ECC segment types (for tests / docs generation).
    pub const ALL: &'static [EccSegmentType] = &[
        Self::CausalEdge,
        Self::CausalNode,
        Self::Impulse,
        Self::CalibrationProfile,
        Self::CrossRef,
        Self::SpectralCheckpoint,
        Self::CausalGraphSnap,
    ];
}

impl TryFrom<u8> for EccSegmentType {
    type Error = u8;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x50 => Ok(Self::CausalEdge),
            0x51 => Ok(Self::CausalNode),
            0x52 => Ok(Self::Impulse),
            0x53 => Ok(Self::CalibrationProfile),
            0x54 => Ok(Self::CrossRef),
            0x55 => Ok(Self::SpectralCheckpoint),
            0x56 => Ok(Self::CausalGraphSnap),
            other => Err(other),
        }
    }
}

/// Codec for ECC segment payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EccSegmentCodec {
    /// JSON (`serde_json`) — default for interoperability / tests.
    #[default]
    Json,
}

/// Framed ECC segment: type byte + codec + payload bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EccSegment {
    pub kind: EccSegmentType,
    pub codec: EccSegmentCodec,
    pub payload: Vec<u8>,
}

/// Encode a serializable payload as an [`EccSegment`].
pub fn encode_ecc_segment<T: Serialize>(
    kind: EccSegmentType,
    value: &T,
) -> Result<EccSegment, EccSegmentError> {
    let payload =
        serde_json::to_vec(value).map_err(|e| EccSegmentError::Encode(e.to_string()))?;
    Ok(EccSegment {
        kind,
        codec: EccSegmentCodec::Json,
        payload,
    })
}

/// Decode payload bytes into `T`.
pub fn decode_ecc_segment<T: for<'de> Deserialize<'de>>(
    segment: &EccSegment,
) -> Result<T, EccSegmentError> {
    match segment.codec {
        EccSegmentCodec::Json => serde_json::from_slice(&segment.payload)
            .map_err(|e| EccSegmentError::Decode(e.to_string())),
    }
}

/// Wire bytes: `[type:u8][codec:u8][payload...]`.
pub fn segment_to_wire(segment: &EccSegment) -> Vec<u8> {
    let mut out = Vec::with_capacity(2 + segment.payload.len());
    out.push(segment.kind as u8);
    out.push(match segment.codec {
        EccSegmentCodec::Json => 0x01,
    });
    out.extend_from_slice(&segment.payload);
    out
}

/// Parse wire bytes produced by [`segment_to_wire`].
pub fn segment_from_wire(bytes: &[u8]) -> Result<EccSegment, EccSegmentError> {
    if bytes.len() < 2 {
        return Err(EccSegmentError::Truncated);
    }
    let kind = EccSegmentType::try_from(bytes[0]).map_err(EccSegmentError::UnknownType)?;
    let codec = match bytes[1] {
        0x01 => EccSegmentCodec::Json,
        other => return Err(EccSegmentError::UnknownCodec(other)),
    };
    Ok(EccSegment {
        kind,
        codec,
        payload: bytes[2..].to_vec(),
    })
}

/// Errors for ECC segment codec.
#[non_exhaustive]
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum EccSegmentError {
    #[error("encode failed: {0}")]
    Encode(String),
    #[error("decode failed: {0}")]
    Decode(String),
    #[error("truncated segment")]
    Truncated,
    #[error("unknown ECC segment type byte: 0x{0:02x}")]
    UnknownType(u8),
    #[error("unknown codec byte: 0x{0:02x}")]
    UnknownCodec(u8),
}

/// Spectral checkpoint payload (λ₂ snapshot).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpectralCheckpoint {
    pub lambda_2: f64,
    pub node_count: u64,
    pub edge_count: u64,
    pub timestamp: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Calibration profile payload (opaque JSON-friendly map for now).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalibrationProfileSegment {
    pub profile_id: String,
    pub version: u32,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::causal::{CausalEdge, CausalEdgeType};

    #[test]
    fn discriminants_stable() {
        assert_eq!(EccSegmentType::CausalEdge as u8, 0x50);
        assert_eq!(EccSegmentType::CausalGraphSnap as u8, 0x56);
        for &t in EccSegmentType::ALL {
            assert_eq!(EccSegmentType::try_from(t as u8), Ok(t));
        }
    }

    #[test]
    fn unknown_type_err() {
        assert_eq!(EccSegmentType::try_from(0x12), Err(0x12));
        assert_eq!(EccSegmentType::try_from(0x40), Err(0x40));
    }

    #[test]
    fn causal_edge_roundtrip() {
        let edge = CausalEdge {
            source: 1,
            target: 2,
            edge_type: CausalEdgeType::Causes,
            weight: 0.9,
            timestamp: 10,
            chain_seq: 3,
            source_universal_id: [1u8; 32],
            target_universal_id: [2u8; 32],
        };
        let seg = encode_ecc_segment(EccSegmentType::CausalEdge, &edge).unwrap();
        assert_eq!(seg.kind, EccSegmentType::CausalEdge);
        let wire = segment_to_wire(&seg);
        let restored_seg = segment_from_wire(&wire).unwrap();
        let restored: CausalEdge = decode_ecc_segment(&restored_seg).unwrap();
        assert_eq!(restored.source, 1);
        assert_eq!(restored.target, 2);
        assert_eq!(restored.edge_type, CausalEdgeType::Causes);
        assert!((restored.weight - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn spectral_checkpoint_roundtrip() {
        let cp = SpectralCheckpoint {
            lambda_2: 0.42,
            node_count: 10,
            edge_count: 20,
            timestamp: 99,
            notes: Some("tick".into()),
        };
        let seg = encode_ecc_segment(EccSegmentType::SpectralCheckpoint, &cp).unwrap();
        let cp2: SpectralCheckpoint = decode_ecc_segment(&seg).unwrap();
        assert_eq!(cp2, cp);
    }

    #[test]
    fn calibration_profile_roundtrip() {
        let p = CalibrationProfileSegment {
            profile_id: "democritus-default".into(),
            version: 1,
            params: serde_json::json!({"tick_us": 1000}),
        };
        let seg = encode_ecc_segment(EccSegmentType::CalibrationProfile, &p).unwrap();
        let p2: CalibrationProfileSegment = decode_ecc_segment(&seg).unwrap();
        assert_eq!(p2.profile_id, "democritus-default");
        assert_eq!(p2.version, 1);
    }

    #[test]
    fn wire_truncated() {
        assert_eq!(segment_from_wire(&[0x50]), Err(EccSegmentError::Truncated));
    }
}
