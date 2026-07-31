//! RVF segment encode/decode for LeWM small models (WEFT-532).
//!
//! Wire framing (aligned with ECC helper style, independent of kernel):
//!
//! ```text
//! [type:u8][codec:u8][sensor_class:u8][version_tag:u64 LE]
//! [weight_digest:u64 LE][weight_len:u32 LE][weights...]
//! ```
//!
//! `type` is [`SmallModelKind`] (`0x60` transmit-gate, `0x61` aggregate,
//! `0x62` encode). Codec `0x02` = raw weight blob (default for stubs).

extern crate alloc;

use alloc::vec::Vec;

use weftos_worldmodel_core::{
    fnv1a64, ModelCheckpoint, SensorClass, SmallModelKind, TrainingSurfaceKind, WorldModelError,
    WorldModelResult, LEWM_MODEL_SEGMENT_CODEC_RAW,
};

/// Fixed header length before the weight blob.
pub const SEGMENT_HEADER_LEN: usize = 1 + 1 + 1 + 8 + 8 + 4; // 23

/// Framed LeWM model segment (RVF-deliverable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LewmModelSegment {
    /// Segment type = [`SmallModelKind`] discriminant.
    pub kind: SmallModelKind,
    /// Codec byte (`0x02` raw).
    pub codec: u8,
    /// Sensor class that owns this model.
    pub sensor_class: SensorClass,
    /// Model version tag.
    pub version_tag: u64,
    /// Digest of weights.
    pub weight_digest: u64,
    /// Opaque weight payload.
    pub weights: Vec<u8>,
}

impl LewmModelSegment {
    /// Build from a [`ModelCheckpoint`].
    pub fn from_checkpoint(cp: &ModelCheckpoint) -> Self {
        Self {
            kind: cp.kind,
            codec: LEWM_MODEL_SEGMENT_CODEC_RAW,
            sensor_class: cp.sensor_class,
            version_tag: cp.version_tag,
            weight_digest: cp.weight_digest,
            weights: cp.weights.clone(),
        }
    }

    /// Convert back to a checkpoint (surface defaults to offline edge).
    pub fn into_checkpoint(
        self,
        surface: TrainingSurfaceKind,
        target_tick: Option<u64>,
    ) -> ModelCheckpoint {
        ModelCheckpoint {
            sensor_class: self.sensor_class,
            kind: self.kind,
            version_tag: self.version_tag,
            surface,
            weights: self.weights,
            weight_digest: self.weight_digest,
            target_tick,
        }
    }

    /// Verify `weight_digest` matches payload.
    pub fn verify_digest(&self) -> bool {
        fnv1a64(&self.weights) == self.weight_digest
    }
}

/// Encode a checkpoint into a framed segment.
pub fn encode_model_segment(cp: &ModelCheckpoint) -> LewmModelSegment {
    LewmModelSegment::from_checkpoint(cp)
}

/// Decode segment into a checkpoint with the given surface / target tick.
pub fn decode_model_segment(
    segment: &LewmModelSegment,
    surface: TrainingSurfaceKind,
    target_tick: Option<u64>,
) -> WorldModelResult<ModelCheckpoint> {
    if !segment.verify_digest() {
        return Err(WorldModelError::SegmentCodec);
    }
    Ok(ModelCheckpoint {
        sensor_class: segment.sensor_class,
        kind: segment.kind,
        version_tag: segment.version_tag,
        surface,
        weights: segment.weights.clone(),
        weight_digest: segment.weight_digest,
        target_tick,
    })
}

/// Wire bytes: header + weights.
pub fn model_segment_to_wire(segment: &LewmModelSegment) -> Vec<u8> {
    let mut out = Vec::with_capacity(SEGMENT_HEADER_LEN + segment.weights.len());
    out.push(segment.kind as u8);
    out.push(segment.codec);
    out.push(segment.sensor_class as u8);
    out.extend_from_slice(&segment.version_tag.to_le_bytes());
    out.extend_from_slice(&segment.weight_digest.to_le_bytes());
    let len = segment.weights.len() as u32;
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&segment.weights);
    out
}

/// Parse wire bytes produced by [`model_segment_to_wire`].
pub fn model_segment_from_wire(bytes: &[u8]) -> WorldModelResult<LewmModelSegment> {
    if bytes.len() < SEGMENT_HEADER_LEN {
        return Err(WorldModelError::SegmentCodec);
    }
    let kind = SmallModelKind::try_from(bytes[0]).map_err(|_| WorldModelError::SegmentCodec)?;
    let codec = bytes[1];
    let sensor_class =
        SensorClass::try_from(bytes[2]).map_err(|_| WorldModelError::SegmentCodec)?;
    let version_tag = u64::from_le_bytes(bytes[3..11].try_into().unwrap());
    let weight_digest = u64::from_le_bytes(bytes[11..19].try_into().unwrap());
    let weight_len = u32::from_le_bytes(bytes[19..23].try_into().unwrap()) as usize;
    if bytes.len() < SEGMENT_HEADER_LEN + weight_len {
        return Err(WorldModelError::SegmentCodec);
    }
    let weights = bytes[SEGMENT_HEADER_LEN..SEGMENT_HEADER_LEN + weight_len].to_vec();
    let seg = LewmModelSegment {
        kind,
        codec,
        sensor_class,
        version_tag,
        weight_digest,
        weights,
    };
    if !seg.verify_digest() {
        return Err(WorldModelError::SegmentCodec);
    }
    Ok(seg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use weftos_worldmodel_core::{SensorClass, SmallModelKind, TrainingSurfaceKind};

    #[test]
    fn roundtrip_wire() {
        let cp = ModelCheckpoint::new(
            SensorClass::Lidar,
            SmallModelKind::TransmitGate,
            42,
            TrainingSurfaceKind::OfflineEdge,
            alloc::vec![9, 8, 7, 6],
            Some(100),
        );
        let seg = encode_model_segment(&cp);
        let wire = model_segment_to_wire(&seg);
        assert_eq!(wire[0], 0x60);
        let restored = model_segment_from_wire(&wire).expect("wire");
        assert_eq!(restored.version_tag, 42);
        assert_eq!(restored.sensor_class, SensorClass::Lidar);
        assert_eq!(restored.weights, cp.weights);
        let cp2 = decode_model_segment(&restored, TrainingSurfaceKind::OfflineEdge, Some(100))
            .expect("decode");
        assert_eq!(cp2.weight_digest, cp.weight_digest);
        assert_eq!(cp2.target_tick, Some(100));
    }

    #[test]
    fn bad_digest_rejected() {
        let mut seg = LewmModelSegment {
            kind: SmallModelKind::Encode,
            codec: LEWM_MODEL_SEGMENT_CODEC_RAW,
            sensor_class: SensorClass::Generic,
            version_tag: 1,
            weight_digest: 0xDEAD,
            weights: alloc::vec![1, 2, 3],
        };
        assert!(!seg.verify_digest());
        let wire = model_segment_to_wire(&seg);
        assert!(model_segment_from_wire(&wire).is_err());
        seg.weight_digest = fnv1a64(&seg.weights);
        let wire = model_segment_to_wire(&seg);
        assert!(model_segment_from_wire(&wire).is_ok());
    }

    #[test]
    fn truncated_wire_err() {
        assert!(model_segment_from_wire(&[0x60, 0x02]).is_err());
    }
}
