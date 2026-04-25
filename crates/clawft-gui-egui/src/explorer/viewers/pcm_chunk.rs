//! `PcmChunkViewer` — renders a substrate value carrying base64
//! PCM audio without dragging a multi-KB string through egui's
//! text layout.
//!
//! The wire shape (matched against ESP32 firmware emissions):
//!
//! ```json
//! {
//!   "data":         "<base64>",     // s16le PCM samples
//!   "encoding":     "base64",
//!   "format":       "i16le",
//!   "sample_rate":  16000,
//!   "channels":     1,
//!   "samples":      8000,
//!   "start_ts_ms":  3807924
//! }
//! ```
//!
//! Why a dedicated viewer rather than letting JsonFallback handle
//! it: a single 500-ms 16-kHz mono i16le chunk is ~16 KB raw, ~21
//! KB once base64-encoded. JsonFallback would try to lay out the
//! `data` field as a single monospace galley and choke the render
//! thread. `paint_string` now hard-caps that path (see
//! `STR_INLINE_HARD_MAX`), but skipping it entirely on the hot
//! path is a clearer architecture: the shape is known, render the
//! parts you can read, summarise the bytes you can't.
//!
//! Priority **20** — wins decisively over JsonFallback (1) and
//! over generic shape-matchers; lower than ObjectType-Mesh's 20-as
//! priority because the two never overlap (Mesh matches a top-level
//! snapshot; PcmChunk matches a leaf payload).

use super::SubstrateViewer;
use serde_json::Value;

/// Priority for shape-match.
const PRIORITY: u32 = 20;

pub struct PcmChunkViewer;

impl SubstrateViewer for PcmChunkViewer {
    fn matches(value: &Value) -> u32 {
        let Some(obj) = value.as_object() else {
            return 0;
        };
        let has_data = obj.get("data").and_then(Value::as_str).is_some();
        let has_sr = obj.get("sample_rate").and_then(Value::as_u64).is_some();
        let format_known = obj
            .get("format")
            .and_then(Value::as_str)
            .map(|s| s == "i16le")
            .unwrap_or(false);
        let encoding_known = obj
            .get("encoding")
            .and_then(Value::as_str)
            .map(|s| s == "base64")
            .unwrap_or(false);
        if has_data && has_sr && format_known && encoding_known {
            PRIORITY
        } else {
            0
        }
    }

    fn paint(ui: &mut egui::Ui, path: &str, value: &Value) {
        let obj = match value.as_object() {
            Some(o) => o,
            None => return,
        };

        let sample_rate = obj.get("sample_rate").and_then(Value::as_u64).unwrap_or(0);
        let channels = obj.get("channels").and_then(Value::as_u64).unwrap_or(1);
        let samples = obj.get("samples").and_then(Value::as_u64).unwrap_or(0);
        let start_ts_ms = obj.get("start_ts_ms").and_then(Value::as_u64).unwrap_or(0);
        let data_len = obj
            .get("data")
            .and_then(Value::as_str)
            .map(|s| s.len())
            .unwrap_or(0);
        let chunk_ms = if sample_rate > 0 {
            samples * 1000 / sample_rate
        } else {
            0
        };

        ui.label(
            egui::RichText::new(format!("pcm_chunk · {path}"))
                .color(egui::Color32::from_rgb(160, 160, 170))
                .small(),
        );
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("format").strong());
            ui.monospace("i16le · base64");
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("sample_rate").strong());
            ui.monospace(format!("{sample_rate} Hz"));
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("channels").strong());
            ui.monospace(format!("{channels}"));
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("samples").strong());
            ui.monospace(format!("{samples} ({chunk_ms} ms)"));
        });
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("start_ts_ms").strong());
            ui.monospace(format!("{start_ts_ms}"));
        });
        ui.add_space(6.0);
        ui.separator();
        ui.label(
            egui::RichText::new(format!(
                "[encoded data — {data_len} bytes — not rendered inline]"
            ))
            .small()
            .italics()
            .color(egui::Color32::from_rgb(140, 140, 150)),
        );
        ui.label(
            egui::RichText::new(
                "Decoding + waveform plot deferred — would require \
                 per-frame base64 decode of ~16 KB. Click the \
                 substrate path to keep monitoring; subscribe for \
                 live consumption.",
            )
            .small()
            .color(egui::Color32::from_rgb(140, 140, 150)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture(data_len: usize) -> Value {
        json!({
            "data": "a".repeat(data_len),
            "encoding": "base64",
            "format": "i16le",
            "sample_rate": 16000,
            "channels": 1,
            "samples": 8000,
            "start_ts_ms": 1234,
        })
    }

    #[test]
    fn matches_full_shape() {
        let v = fixture(64);
        assert_eq!(PcmChunkViewer::matches(&v), PRIORITY);
    }

    #[test]
    fn matches_large_data_field() {
        // The exact size that locks up JsonFallback today — make sure
        // the dedicated viewer is the one that wins.
        let v = fixture(21336);
        assert_eq!(PcmChunkViewer::matches(&v), PRIORITY);
    }

    #[test]
    fn rejects_unknown_format() {
        let mut v = fixture(64);
        v["format"] = json!("opus");
        assert_eq!(PcmChunkViewer::matches(&v), 0);
    }

    #[test]
    fn rejects_unknown_encoding() {
        let mut v = fixture(64);
        v["encoding"] = json!("raw");
        assert_eq!(PcmChunkViewer::matches(&v), 0);
    }

    #[test]
    fn rejects_missing_data() {
        let v = json!({
            "encoding": "base64",
            "format": "i16le",
            "sample_rate": 16000,
        });
        assert_eq!(PcmChunkViewer::matches(&v), 0);
    }

    #[test]
    fn rejects_missing_sample_rate() {
        let v = json!({
            "data": "abc",
            "encoding": "base64",
            "format": "i16le",
        });
        assert_eq!(PcmChunkViewer::matches(&v), 0);
    }

    #[test]
    fn rejects_non_object() {
        assert_eq!(PcmChunkViewer::matches(&Value::Null), 0);
        assert_eq!(PcmChunkViewer::matches(&json!([1, 2, 3])), 0);
    }

    #[test]
    fn priority_beats_json_fallback() {
        assert!(PRIORITY > 1);
    }
}
