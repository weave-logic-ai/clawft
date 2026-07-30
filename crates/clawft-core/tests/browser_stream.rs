//! WEFT-390 — browser streaming chat through the pipeline.
//!
//! Host-side unit tests for the non-`Send` stream callback path
//! (`StreamCallback` without `Send`, `LlmProvider::complete_stream` with
//! a dyn-FnMut sink, and `PipelineRegistry::complete_stream`).
//!
//! Run with:
//! ```text
//! cargo test -p clawft-core --no-default-features --features browser --test browser_stream
//! ```
//!
//! Required-features keeps native-only test modules out of this binary.

#![cfg(feature = "browser")]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use clawft_core::pipeline::assembler::TokenBudgetAssembler;
use clawft_core::pipeline::classifier::KeywordClassifier;
use clawft_core::pipeline::learner::NoopLearner;
use clawft_core::pipeline::router::StaticRouter;
use clawft_core::pipeline::scorer::NoopScorer;
use clawft_core::pipeline::traits::{
    ChatRequest, LlmMessage, LlmTransport, Pipeline, PipelineRegistry, StreamCallback,
    TransportRequest,
};
use clawft_core::pipeline::transport::{LlmProvider, OpenAiCompatTransport};

/// Mock provider that streams canned text deltas via the browser
/// callback signature (WEFT-390).
struct StreamingMock {
    chunks: Vec<&'static str>,
}

// SAFETY: test-only; never crosses threads.
unsafe impl Send for StreamingMock {}
unsafe impl Sync for StreamingMock {}

#[async_trait(?Send)]
impl LlmProvider for StreamingMock {
    async fn complete(
        &self,
        _model: &str,
        _messages: &[serde_json::Value],
        _tools: &[serde_json::Value],
        _max_tokens: Option<i32>,
        _temperature: Option<f64>,
        _tool_choice: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "id": "mock",
            "model": "mock",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": self.chunks.join("")},
                "finish_reason": "stop"
            }]
        }))
    }

    async fn complete_stream(
        &self,
        model: &str,
        _messages: &[serde_json::Value],
        _tools: &[serde_json::Value],
        _max_tokens: Option<i32>,
        _temperature: Option<f64>,
        _tool_choice: Option<&serde_json::Value>,
        mut on_delta: Box<dyn for<'a> FnMut(&'a str) -> bool>,
    ) -> Result<serde_json::Value, String> {
        let mut full = String::new();
        for c in &self.chunks {
            full.push_str(c);
            if !on_delta(c) {
                break;
            }
        }
        Ok(serde_json::json!({
            "id": "stream-mock",
            "model": model,
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": full},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 1,
                "completion_tokens": self.chunks.len(),
                "total_tokens": 1 + self.chunks.len()
            }
        }))
    }
}

fn transport_request() -> TransportRequest {
    TransportRequest {
        provider: "mock".into(),
        model: "mock-model".into(),
        messages: vec![LlmMessage {
            role: "user".into(),
            content: "hi".into(),
            tool_call_id: None,
            tool_calls: None,
        }],
        tools: vec![],
        max_tokens: None,
        temperature: None,
        tool_choice: None,
    }
}

#[tokio::test]
async fn browser_transport_streams_chunks_via_callback() {
    let provider = Arc::new(StreamingMock {
        chunks: vec!["Hello", " ", "stream", "!"],
    });
    let transport = OpenAiCompatTransport::with_provider(provider);

    let collected = Arc::new(Mutex::new(Vec::new()));
    let c = Arc::clone(&collected);
    let callback: StreamCallback = Box::new(move |delta| {
        c.lock().unwrap().push(delta.to_string());
        true
    });

    let response = transport
        .complete_stream(&transport_request(), callback)
        .await
        .expect("stream ok");

    let chunks = collected.lock().unwrap().clone();
    assert_eq!(chunks, vec!["Hello", " ", "stream", "!"]);
    assert_eq!(response.id, "stream-mock");
    match &response.content[..] {
        [clawft_types::provider::ContentBlock::Text { text }] => {
            assert_eq!(text, "Hello stream!");
        }
        other => panic!("unexpected content: {other:?}"),
    }
}

#[tokio::test]
async fn browser_transport_callback_can_abort() {
    let provider = Arc::new(StreamingMock {
        chunks: vec!["a", "b", "c", "d"],
    });
    let transport = OpenAiCompatTransport::with_provider(provider);

    let collected = Arc::new(Mutex::new(Vec::new()));
    let c = Arc::clone(&collected);
    let callback: StreamCallback = Box::new(move |delta| {
        let mut v = c.lock().unwrap();
        v.push(delta.to_string());
        v.len() < 2
    });

    let _ = transport
        .complete_stream(&transport_request(), callback)
        .await
        .expect("partial stream ok");

    let chunks = collected.lock().unwrap().clone();
    assert_eq!(chunks, vec!["a", "b"]);
}

#[tokio::test]
async fn pipeline_registry_complete_stream_with_streaming_transport() {
    let provider = Arc::new(StreamingMock {
        chunks: vec!["pipe", "-", "stream"],
    });
    let transport = Arc::new(OpenAiCompatTransport::with_provider(provider));

    let pipeline = Pipeline {
        classifier: Arc::new(KeywordClassifier::new()),
        router: Arc::new(StaticRouter::new("mock".into(), "mock-model".into())),
        assembler: Arc::new(TokenBudgetAssembler::new(4096)),
        transport,
        scorer: Arc::new(NoopScorer::new()),
        learner: Arc::new(NoopLearner::new()),
    };
    let registry = PipelineRegistry::new(pipeline);

    let collected = Arc::new(Mutex::new(Vec::new()));
    let c = Arc::clone(&collected);
    let callback: StreamCallback = Box::new(move |d| {
        c.lock().unwrap().push(d.to_string());
        true
    });

    let req = ChatRequest {
        messages: vec![LlmMessage {
            role: "user".into(),
            content: "stream me".into(),
            tool_call_id: None,
            tool_calls: None,
        }],
        tools: vec![],
        model: None,
        max_tokens: None,
        temperature: None,
        auth_context: None,
        complexity_boost: 0.0,
        tool_choice: None,
    };

    let response = registry
        .complete_stream(&req, callback)
        .await
        .expect("pipeline stream");

    let chunks = collected.lock().unwrap().clone();
    assert_eq!(chunks, vec!["pipe", "-", "stream"]);
    assert!(response.content.iter().any(
        |b| matches!(b, clawft_types::provider::ContentBlock::Text { text } if text.contains("pipe"))
    ));
}
