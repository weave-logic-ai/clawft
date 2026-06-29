//! Unit tests for [`super`] (`embedding_qwen3`). Split into a sibling file via
//! `#[path]` to keep `embedding_qwen3.rs` under the 500-line limit. The no-model
//! tests run on every build; `qwen3_live_inference_sanity` is `#[ignore]`d and
//! needs `--features onnx-embeddings` + the model bundle (ADR-059 1.4).

use super::*;

fn vec_magnitude(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

// -- last-token pooling math ------------------------------------------

#[test]
fn last_token_pool_picks_last_attended_row() {
    // 3 rows × 2 dims; mask marks row index 1 as the last attended one.
    let hidden = vec![1.0, 1.0, 2.0, 2.0, 9.0, 9.0];
    let mask = vec![1, 1, 0];
    let pooled = last_token_pool(&hidden, 3, 2, &mask);
    assert_eq!(pooled, vec![2.0, 2.0]);
}

#[test]
fn last_token_pool_all_attended_picks_final_row() {
    let hidden = vec![1.0, 1.0, 2.0, 2.0, 3.0, 3.0];
    let mask = vec![1, 1, 1];
    let pooled = last_token_pool(&hidden, 3, 2, &mask);
    assert_eq!(pooled, vec![3.0, 3.0]);
}

#[test]
fn last_token_pool_empty_mask_falls_back_to_final_row() {
    let hidden = vec![1.0, 1.0, 7.0, 7.0];
    let pooled = last_token_pool(&hidden, 2, 2, &[]);
    assert_eq!(pooled, vec![7.0, 7.0]);
}

// -- MRL slice + L2-norm ----------------------------------------------

#[test]
fn mrl_slices_to_512_and_normalizes() {
    let full: Vec<f32> = (0..1024).map(|i| (i as f32) + 1.0).collect();
    let out = mrl_slice_l2norm(&full, QWEN3_DIMS);
    assert_eq!(out.len(), QWEN3_DIMS, "must slice to 512 dims");
    let mag = vec_magnitude(&out);
    assert!((mag - 1.0).abs() < 1e-4, "magnitude = {mag}, expected ~1.0");
    // Slice-then-normalize: it must reflect the first 512 dims only.
    assert!(out[0] > 0.0 && out[511] > 0.0);
}

#[test]
fn mrl_zero_pads_short_input() {
    let short = vec![3.0, 4.0]; // |[3,4]| = 5
    let out = mrl_slice_l2norm(&short, QWEN3_DIMS);
    assert_eq!(out.len(), QWEN3_DIMS);
    assert!((out[0] - 0.6).abs() < 1e-4);
    assert!((out[1] - 0.8).abs() < 1e-4);
    assert!(out[2..].iter().all(|&x| x == 0.0));
}

// -- tokenizer/model-absent fallback ----------------------------------

#[tokio::test]
async fn absent_model_degrades_to_mock() {
    let p = Qwen3EmbeddingProvider::new("/nonexistent/model_fp16.onnx", "/nonexistent/tok.json");
    assert!(!p.is_runtime_available());
    assert_eq!(p.dimensions(), QWEN3_DIMS);
    assert!(p.model_name().starts_with(QWEN3_MODEL_NAME));
    assert!(p.model_name().contains("fallback"));

    let v = p.embed("hello world").await.unwrap();
    assert_eq!(v.len(), QWEN3_DIMS);
    // Deterministic via the Mock SHA-256 backend.
    let v2 = p.embed("hello world").await.unwrap();
    assert_eq!(v, v2);
}

#[tokio::test]
async fn fallback_query_embeds_without_model() {
    let p = Qwen3EmbeddingProvider::new("/nope.onnx", "/nope.json");
    let q = p
        .embed_query("what is the kernel boot order")
        .await
        .unwrap();
    assert_eq!(q.len(), QWEN3_DIMS);
}

#[tokio::test]
async fn fallback_batch_matches_individual() {
    let p = Qwen3EmbeddingProvider::new("/nope.onnx", "/nope.json");
    let batch = p.embed_batch(&["a", "b"]).await.unwrap();
    let a = p.embed("a").await.unwrap();
    assert_eq!(batch.len(), 2);
    assert_eq!(batch[0], a);
}

#[test]
fn query_instruction_format_is_official() {
    assert!(QUERY_INSTRUCTION.starts_with("Instruct: "));
    assert!(QUERY_INSTRUCTION.ends_with("\nQuery:"));
}

// -- Live decoder-ONNX test (task 1.4 — needs the model) --------------

#[cfg(feature = "onnx-embeddings")]
fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na = vec_magnitude(a);
    let nb = vec_magnitude(b);
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

/// Live decoder-ONNX inference (task 1.4). Requires `--features
/// onnx-embeddings` **and** the model bundle at
/// `.weftos/models/Qwen3-Embedding-0.6B/` (`model_fp16.onnx` +
/// `tokenizer.json`, ~1.2 GB — ADR-059 follow-up 1). Run with:
///
/// ```text
/// cargo test -p clawft-kernel --features onnx-embeddings -- --ignored qwen3_live
/// ```
///
/// Full lab cosine-parity (≥ 0.99 vs MLX) is validated against the probe set
/// in `~/llm/docs/models/results/embedlab.json` once the model is fetched.
#[cfg(feature = "onnx-embeddings")]
#[tokio::test]
#[ignore = "requires Qwen3 model bundle (~1.2 GB) — ADR-059 follow-up 1"]
async fn qwen3_live_inference_sanity() {
    let provider = Qwen3EmbeddingProvider::new(
        ".weftos/models/Qwen3-Embedding-0.6B/model_fp16.onnx",
        ".weftos/models/Qwen3-Embedding-0.6B/tokenizer.json",
    );
    assert!(
        provider.is_runtime_available(),
        "model bundle not loaded; fetch onnx-community/Qwen3-Embedding-0.6B-ONNX first"
    );

    // Document mode: 512-d, L2-normalized.
    let doc = provider
        .embed("The kernel boots the process table.")
        .await
        .unwrap();
    assert_eq!(doc.len(), QWEN3_DIMS);
    let mag = vec_magnitude(&doc);
    assert!((mag - 1.0).abs() < 1e-2, "doc not L2-normalized: {mag}");

    // Query mode (instruction-prefixed): 512-d.
    let q = provider
        .embed_query("how does the kernel boot")
        .await
        .unwrap();
    assert_eq!(q.len(), QWEN3_DIMS);

    // Relatedness sanity: related pair scores higher than an unrelated one.
    let a = provider.embed("process table allocation").await.unwrap();
    let b = provider
        .embed("allocating PIDs in the process table")
        .await
        .unwrap();
    let c = provider
        .embed("ocean acoustic sonar propagation")
        .await
        .unwrap();
    assert!(
        cosine(&a, &b) > cosine(&a, &c),
        "related cosine {} should exceed unrelated cosine {}",
        cosine(&a, &b),
        cosine(&a, &c)
    );
}
