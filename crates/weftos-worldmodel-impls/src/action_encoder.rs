//! Action encoder: control intent → fixed-width [`Action`] code for `pred_φ`.

use weftos_worldmodel_core::{Action, WorldModelResult};

/// Width of [`Action::code`] (dense embedding for the predictor).
pub const ACTION_CODE_DIM: usize = 32;

/// Maps raw control / intervention payloads into a dense action code.
///
/// Core only stores [`Action`]; concrete DEMOCRITUS / servo semantics live
/// here. Production may later learn an action embedding network (candle path).
pub trait ActionEncoder {
    /// Encode arbitrary control bytes into an [`Action`].
    fn encode_bytes(&self, intent: &[u8]) -> WorldModelResult<Action>;

    /// Encode an already-dense code (length must be [`ACTION_CODE_DIM`]).
    fn encode_code(&self, code: &[f32]) -> WorldModelResult<Action> {
        if code.len() != ACTION_CODE_DIM {
            return Err(weftos_worldmodel_core::WorldModelError::Other(
                "action code dim mismatch",
            ));
        }
        let mut out = [0.0_f32; ACTION_CODE_DIM];
        out.copy_from_slice(code);
        Ok(Action { code: out })
    }
}

/// Always returns [`Action::null`] (no-op intervention).
#[derive(Debug, Default, Clone, Copy)]
pub struct NullActionEncoder;

impl ActionEncoder for NullActionEncoder {
    fn encode_bytes(&self, _intent: &[u8]) -> WorldModelResult<Action> {
        Ok(Action::null())
    }
}

/// Deterministic FNV-1a style hash into the action code (tests / mocks).
///
/// Not a learned embedding — stable for golden tests and mesh stubs.
#[derive(Debug, Default, Clone, Copy)]
pub struct HashActionEncoder;

impl ActionEncoder for HashActionEncoder {
    fn encode_bytes(&self, intent: &[u8]) -> WorldModelResult<Action> {
        Ok(Action {
            code: hash_to_f32_array::<ACTION_CODE_DIM>(intent),
        })
    }
}

/// Mix bytes into a fixed `[f32; N]` via FNV-1a + per-slot scramble.
pub(crate) fn hash_to_f32_array<const N: usize>(bytes: &[u8]) -> [f32; N] {
    // FNV-1a 64-bit offset basis / prime.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    let mut out = [0.0_f32; N];
    for (i, slot) in out.iter_mut().enumerate() {
        let mut x = h
            .wrapping_add((i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15))
            .wrapping_mul(0xbf58_476d_1ce4_e5b9);
        // Map u64 bits into (-1, 1) without libm (no_std friendly).
        let bits = (x >> 40) as u32; // 24 bits of mantissa-ish material
        let u = (bits as f32) / (1u32 << 24) as f32; // [0, 1)
        *slot = u * 2.0 - 1.0;
        // Advance state so consecutive slots differ even for empty input
        // only when bytes non-empty — empty still yields non-zero scramble.
        x = x.wrapping_mul(0x94d0_49bb_1331_11eb);
        h = x;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_is_zero_code() {
        let a = NullActionEncoder.encode_bytes(b"anything").unwrap();
        assert_eq!(a, Action::null());
    }

    #[test]
    fn hash_empty_still_defined() {
        let a = HashActionEncoder.encode_bytes(b"").unwrap();
        // Scramble of empty input is deterministic and finite.
        assert!(a.code.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn encode_code_rejects_wrong_len() {
        let err = NullActionEncoder.encode_code(&[0.0; 8]).unwrap_err();
        assert!(matches!(
            err,
            weftos_worldmodel_core::WorldModelError::Other(_)
        ));
    }
}
