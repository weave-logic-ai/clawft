//! 6-stage pipeline trait definitions and supporting types.
//!
//! This module defines the core abstractions for the clawft pipeline system.
//! Each stage is represented by a trait that can be implemented at different
//! capability levels (Level 0 = basic, Level 1 = adaptive, Level 2 = neural).
//!
//! The pipeline stages in order:
//! 1. **[`TaskClassifier`]** -- Classify the incoming request by task type
//! 2. **[`ModelRouter`]** -- Select the best provider/model for the task
//! 3. **[`ContextAssembler`]** -- Assemble context (system prompt, memory, history)
//! 4. **[`LlmTransport`]** -- Execute the LLM call via HTTP
//! 5. **[`QualityScorer`]** -- Score response quality
//! 6. **[`LearningBackend`]** -- Record the interaction for future learning

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use clawft_types::provider::LlmResponse;

// ── Request / message types ─────────────────────────────────────────────

/// A chat request entering the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// The conversation messages.
    pub messages: Vec<LlmMessage>,

    /// Tool definitions as JSON schemas.
    #[serde(default)]
    pub tools: Vec<serde_json::Value>,

    /// Explicit model override (if any).
    #[serde(default)]
    pub model: Option<String>,

    /// Maximum tokens in the response.
    #[serde(default)]
    pub max_tokens: Option<i32>,

    /// Sampling temperature.
    #[serde(default)]
    pub temperature: Option<f64>,
}

/// A single message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    /// The role of the message sender (e.g. "system", "user", "assistant", "tool").
    pub role: String,

    /// The text content of the message.
    pub content: String,

    /// For tool-result messages, the ID of the tool call this responds to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

// ── Classification types ────────────────────────────────────────────────

/// Task classification result produced by [`TaskClassifier`].
#[derive(Debug, Clone)]
pub struct TaskProfile {
    /// The detected task type.
    pub task_type: TaskType,

    /// Estimated complexity on a 0.0--1.0 scale.
    pub complexity: f32,

    /// Keywords that contributed to the classification.
    pub keywords: Vec<String>,
}

/// Types of tasks the classifier can identify.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    /// General conversation / chitchat.
    Chat,
    /// Writing new code.
    CodeGeneration,
    /// Reviewing existing code.
    CodeReview,
    /// Searching for information.
    Research,
    /// Creative writing (stories, poems, etc.).
    Creative,
    /// Analytical reasoning, summarization, explanation.
    Analysis,
    /// Explicit tool invocation.
    ToolUse,
    /// Could not determine the task type.
    Unknown,
}

// ── Routing types ───────────────────────────────────────────────────────

/// Model routing decision produced by [`ModelRouter`].
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// Provider name (e.g. "openai", "anthropic").
    pub provider: String,

    /// Model identifier (e.g. "gpt-4o", "claude-opus-4-5").
    pub model: String,

    /// Human-readable reason for the routing choice.
    pub reason: String,
}

/// Outcome of a response, used to update the router.
#[derive(Debug, Clone)]
pub struct ResponseOutcome {
    /// Whether the response was considered successful.
    pub success: bool,

    /// Quality assessment of the response.
    pub quality: QualityScore,

    /// End-to-end latency in milliseconds.
    pub latency_ms: u64,
}

// ── Quality types ───────────────────────────────────────────────────────

/// Quality assessment of a response.
#[derive(Debug, Clone)]
pub struct QualityScore {
    /// Overall quality score (0.0--1.0).
    pub overall: f32,

    /// Relevance to the original request (0.0--1.0).
    pub relevance: f32,

    /// Coherence and readability (0.0--1.0).
    pub coherence: f32,
}

// ── Context / transport types ───────────────────────────────────────────

/// Assembled context ready for transport to an LLM provider.
#[derive(Debug, Clone)]
pub struct AssembledContext {
    /// The final set of messages to send.
    pub messages: Vec<LlmMessage>,

    /// Estimated token count for the assembled context.
    pub token_estimate: usize,

    /// Whether the context was truncated to fit the budget.
    pub truncated: bool,
}

/// Request sent to the transport layer.
#[derive(Debug, Clone)]
pub struct TransportRequest {
    /// Provider name.
    pub provider: String,

    /// Model identifier.
    pub model: String,

    /// Messages to send.
    pub messages: Vec<LlmMessage>,

    /// Tool definitions as JSON schemas.
    pub tools: Vec<serde_json::Value>,

    /// Maximum tokens in the response.
    pub max_tokens: Option<i32>,

    /// Sampling temperature.
    pub temperature: Option<f64>,
}

// ── Learning types ──────────────────────────────────────────────────────

/// A complete interaction trajectory for learning.
#[derive(Debug, Clone)]
pub struct Trajectory {
    /// The original request.
    pub request: ChatRequest,

    /// The routing decision that was made.
    pub routing: RoutingDecision,

    /// The LLM response.
    pub response: LlmResponse,

    /// The quality assessment of the response.
    pub quality: QualityScore,
}

/// Signal for the learning backend to adapt behavior.
#[derive(Debug, Clone)]
pub struct LearningSignal {
    /// Type of feedback (e.g. "thumbs_up", "thumbs_down", "correction").
    pub feedback_type: String,

    /// Numeric value of the signal (-1.0 to 1.0).
    pub value: f32,
}

// ── Pipeline traits ─────────────────────────────────────────────────────

/// Stage 1: Classify the incoming request to determine task type and complexity.
pub trait TaskClassifier: Send + Sync {
    /// Analyze the request and produce a task profile.
    fn classify(&self, request: &ChatRequest) -> TaskProfile;
}

/// Stage 2: Select the best provider and model for the classified task.
#[async_trait]
pub trait ModelRouter: Send + Sync {
    /// Choose a provider/model combination for the given request and profile.
    async fn route(&self, request: &ChatRequest, profile: &TaskProfile) -> RoutingDecision;

    /// Update internal state based on a routing outcome (for adaptive routers).
    fn update(&self, decision: &RoutingDecision, outcome: &ResponseOutcome);
}

/// Stage 3: Assemble the context (system prompt, memory, skills, history).
#[async_trait]
pub trait ContextAssembler: Send + Sync {
    /// Build the assembled context for the given request and task profile.
    async fn assemble(&self, request: &ChatRequest, profile: &TaskProfile) -> AssembledContext;
}

/// Stage 4: Execute the LLM call via HTTP transport.
#[async_trait]
pub trait LlmTransport: Send + Sync {
    /// Send a request to the LLM provider and return the response.
    async fn complete(&self, request: &TransportRequest) -> clawft_types::Result<LlmResponse>;
}

/// Stage 5: Score the quality of a response.
pub trait QualityScorer: Send + Sync {
    /// Assess the quality of the response relative to the original request.
    fn score(&self, request: &ChatRequest, response: &LlmResponse) -> QualityScore;
}

/// Stage 6: Record interactions and adapt behavior based on feedback.
pub trait LearningBackend: Send + Sync {
    /// Record a complete interaction trajectory.
    fn record(&self, trajectory: &Trajectory);

    /// Process a learning signal (e.g. user feedback).
    fn adapt(&self, signal: &LearningSignal);
}

// ── Pipeline & Registry ─────────────────────────────────────────────────

/// A complete pipeline wiring all 6 stages together.
pub struct Pipeline {
    /// Stage 1: task classifier.
    pub classifier: Arc<dyn TaskClassifier>,
    /// Stage 2: model router.
    pub router: Arc<dyn ModelRouter>,
    /// Stage 3: context assembler.
    pub assembler: Arc<dyn ContextAssembler>,
    /// Stage 4: LLM transport.
    pub transport: Arc<dyn LlmTransport>,
    /// Stage 5: quality scorer.
    pub scorer: Arc<dyn QualityScorer>,
    /// Stage 6: learning backend.
    pub learner: Arc<dyn LearningBackend>,
}

/// Registry that maps task types to specialized pipelines.
///
/// When a request arrives, the registry classifies it, looks up the
/// pipeline for that task type (falling back to the default), and
/// orchestrates the full 6-stage flow.
pub struct PipelineRegistry {
    pipelines: HashMap<TaskType, Pipeline>,
    default: Pipeline,
}

impl PipelineRegistry {
    /// Create a new registry with the given default pipeline.
    pub fn new(default: Pipeline) -> Self {
        Self {
            pipelines: HashMap::new(),
            default,
        }
    }

    /// Register a specialized pipeline for a specific task type.
    pub fn register(&mut self, task_type: TaskType, pipeline: Pipeline) {
        self.pipelines.insert(task_type, pipeline);
    }

    /// Look up the pipeline for a task type, falling back to the default.
    pub fn get(&self, task_type: &TaskType) -> &Pipeline {
        self.pipelines.get(task_type).unwrap_or(&self.default)
    }

    /// Execute the full pipeline: classify -> route -> assemble -> transport -> score -> learn.
    pub async fn complete(&self, request: &ChatRequest) -> clawft_types::Result<LlmResponse> {
        // Stage 1: classify using the default pipeline's classifier
        let profile = self.default.classifier.classify(request);

        // Select the pipeline for this task type
        let pipeline = self.get(&profile.task_type);

        // Stage 2: route
        let routing = pipeline.router.route(request, &profile).await;

        // Stage 3: assemble context
        let context = pipeline.assembler.assemble(request, &profile).await;

        // Stage 4: transport
        let transport_request = TransportRequest {
            provider: routing.provider.clone(),
            model: routing.model.clone(),
            messages: context.messages,
            tools: request.tools.clone(),
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        };
        let response = pipeline.transport.complete(&transport_request).await?;

        // Stage 5: score
        let quality = pipeline.scorer.score(request, &response);

        // Stage 6: learn
        let trajectory = Trajectory {
            request: request.clone(),
            routing: routing.clone(),
            response: response.clone(),
            quality,
        };
        pipeline.learner.record(&trajectory);

        // Update the router with the outcome
        let outcome = ResponseOutcome {
            success: true,
            quality: trajectory.quality,
            latency_ms: 0, // Caller should measure actual latency
        };
        pipeline.router.update(&routing, &outcome);

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clawft_types::provider::{ContentBlock, StopReason, Usage};

    // ── Type construction tests ─────────────────────────────────────

    #[test]
    fn chat_request_construction() {
        let req = ChatRequest {
            messages: vec![LlmMessage {
                role: "user".into(),
                content: "hello".into(),
                tool_call_id: None,
            }],
            tools: vec![],
            model: Some("gpt-4o".into()),
            max_tokens: Some(1024),
            temperature: Some(0.7),
        };
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.model.as_deref(), Some("gpt-4o"));
    }

    #[test]
    fn llm_message_with_tool_call_id() {
        let msg = LlmMessage {
            role: "tool".into(),
            content: "result data".into(),
            tool_call_id: Some("call-123".into()),
        };
        assert_eq!(msg.tool_call_id.as_deref(), Some("call-123"));
    }

    #[test]
    fn task_profile_construction() {
        let profile = TaskProfile {
            task_type: TaskType::CodeGeneration,
            complexity: 0.7,
            keywords: vec!["implement".into(), "function".into()],
        };
        assert_eq!(profile.task_type, TaskType::CodeGeneration);
        assert!(profile.complexity > 0.5);
        assert_eq!(profile.keywords.len(), 2);
    }

    #[test]
    fn routing_decision_construction() {
        let decision = RoutingDecision {
            provider: "anthropic".into(),
            model: "claude-opus-4-5".into(),
            reason: "high complexity code task".into(),
        };
        assert_eq!(decision.provider, "anthropic");
    }

    #[test]
    fn quality_score_construction() {
        let score = QualityScore {
            overall: 0.9,
            relevance: 0.95,
            coherence: 0.85,
        };
        assert!(score.overall > 0.0 && score.overall <= 1.0);
    }

    #[test]
    fn assembled_context_construction() {
        let ctx = AssembledContext {
            messages: vec![LlmMessage {
                role: "system".into(),
                content: "You are a helpful assistant.".into(),
                tool_call_id: None,
            }],
            token_estimate: 50,
            truncated: false,
        };
        assert!(!ctx.truncated);
        assert_eq!(ctx.token_estimate, 50);
    }

    #[test]
    fn transport_request_construction() {
        let req = TransportRequest {
            provider: "openai".into(),
            model: "gpt-4o".into(),
            messages: vec![],
            tools: vec![],
            max_tokens: Some(2048),
            temperature: None,
        };
        assert_eq!(req.provider, "openai");
        assert!(req.temperature.is_none());
    }

    #[test]
    fn response_outcome_construction() {
        let score = QualityScore {
            overall: 0.8,
            relevance: 0.9,
            coherence: 0.7,
        };
        let outcome = ResponseOutcome {
            success: true,
            quality: score,
            latency_ms: 1500,
        };
        assert!(outcome.success);
        assert_eq!(outcome.latency_ms, 1500);
    }

    #[test]
    fn learning_signal_construction() {
        let signal = LearningSignal {
            feedback_type: "thumbs_up".into(),
            value: 1.0,
        };
        assert_eq!(signal.feedback_type, "thumbs_up");
        assert!((signal.value - 1.0).abs() < f32::EPSILON);
    }

    // ── Serde roundtrip tests ───────────────────────────────────────

    #[test]
    fn chat_request_serde_roundtrip() {
        let req = ChatRequest {
            messages: vec![
                LlmMessage {
                    role: "system".into(),
                    content: "You are helpful.".into(),
                    tool_call_id: None,
                },
                LlmMessage {
                    role: "user".into(),
                    content: "Write a function".into(),
                    tool_call_id: None,
                },
            ],
            tools: vec![serde_json::json!({"type": "function", "name": "web_search"})],
            model: Some("gpt-4o".into()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
        };
        let json = serde_json::to_string(&req).unwrap();
        let restored: ChatRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.messages.len(), 2);
        assert_eq!(restored.messages[0].role, "system");
        assert_eq!(restored.model.as_deref(), Some("gpt-4o"));
        assert_eq!(restored.max_tokens, Some(4096));
        assert_eq!(restored.tools.len(), 1);
    }

    #[test]
    fn llm_message_serde_roundtrip() {
        let msg = LlmMessage {
            role: "tool".into(),
            content: "search results".into(),
            tool_call_id: Some("tc-42".into()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let restored: LlmMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.role, "tool");
        assert_eq!(restored.tool_call_id.as_deref(), Some("tc-42"));
    }

    #[test]
    fn llm_message_serde_skips_none_tool_call_id() {
        let msg = LlmMessage {
            role: "user".into(),
            content: "hello".into(),
            tool_call_id: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("tool_call_id"));
    }

    #[test]
    fn task_type_serde_roundtrip() {
        let types = [
            TaskType::Chat,
            TaskType::CodeGeneration,
            TaskType::CodeReview,
            TaskType::Research,
            TaskType::Creative,
            TaskType::Analysis,
            TaskType::ToolUse,
            TaskType::Unknown,
        ];
        for tt in &types {
            let json = serde_json::to_string(tt).unwrap();
            let restored: TaskType = serde_json::from_str(&json).unwrap();
            assert_eq!(&restored, tt);
        }
    }

    #[test]
    fn task_type_equality_and_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(TaskType::CodeGeneration);
        set.insert(TaskType::CodeGeneration);
        assert_eq!(set.len(), 1);
        set.insert(TaskType::Chat);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn chat_request_with_defaults_deserializes() {
        let json = r#"{"messages": [{"role": "user", "content": "hi"}]}"#;
        let req: ChatRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.messages.len(), 1);
        assert!(req.tools.is_empty());
        assert!(req.model.is_none());
        assert!(req.max_tokens.is_none());
        assert!(req.temperature.is_none());
    }

    // ── Pipeline registry tests ─────────────────────────────────────

    /// Minimal classifier for testing.
    struct TestClassifier {
        task_type: TaskType,
    }

    impl TaskClassifier for TestClassifier {
        fn classify(&self, _request: &ChatRequest) -> TaskProfile {
            TaskProfile {
                task_type: self.task_type.clone(),
                complexity: 0.5,
                keywords: vec![],
            }
        }
    }

    /// Minimal router for testing.
    struct TestRouter {
        provider: String,
        model: String,
    }

    #[async_trait]
    impl ModelRouter for TestRouter {
        async fn route(&self, _request: &ChatRequest, _profile: &TaskProfile) -> RoutingDecision {
            RoutingDecision {
                provider: self.provider.clone(),
                model: self.model.clone(),
                reason: "test".into(),
            }
        }

        fn update(&self, _decision: &RoutingDecision, _outcome: &ResponseOutcome) {}
    }

    /// Minimal assembler for testing.
    struct TestAssembler;

    #[async_trait]
    impl ContextAssembler for TestAssembler {
        async fn assemble(
            &self,
            request: &ChatRequest,
            _profile: &TaskProfile,
        ) -> AssembledContext {
            AssembledContext {
                messages: request.messages.clone(),
                token_estimate: 100,
                truncated: false,
            }
        }
    }

    /// Minimal transport that returns a canned response.
    struct TestTransport;

    #[async_trait]
    impl LlmTransport for TestTransport {
        async fn complete(
            &self,
            _request: &TransportRequest,
        ) -> clawft_types::Result<LlmResponse> {
            Ok(LlmResponse {
                id: "test-resp".into(),
                content: vec![ContentBlock::Text {
                    text: "Hello from test transport".into(),
                }],
                stop_reason: StopReason::EndTurn,
                usage: Usage {
                    input_tokens: 10,
                    output_tokens: 5,
                },
                metadata: HashMap::new(),
            })
        }
    }

    /// Minimal scorer for testing.
    struct TestScorer;

    impl QualityScorer for TestScorer {
        fn score(&self, _request: &ChatRequest, _response: &LlmResponse) -> QualityScore {
            QualityScore {
                overall: 1.0,
                relevance: 1.0,
                coherence: 1.0,
            }
        }
    }

    /// Minimal learner for testing.
    struct TestLearner;

    impl LearningBackend for TestLearner {
        fn record(&self, _trajectory: &Trajectory) {}
        fn adapt(&self, _signal: &LearningSignal) {}
    }

    fn make_test_pipeline(task_type: TaskType, provider: &str, model: &str) -> Pipeline {
        Pipeline {
            classifier: Arc::new(TestClassifier { task_type }),
            router: Arc::new(TestRouter {
                provider: provider.into(),
                model: model.into(),
            }),
            assembler: Arc::new(TestAssembler),
            transport: Arc::new(TestTransport),
            scorer: Arc::new(TestScorer),
            learner: Arc::new(TestLearner),
        }
    }

    #[test]
    fn pipeline_registry_new() {
        let registry = PipelineRegistry::new(make_test_pipeline(
            TaskType::Chat,
            "openai",
            "gpt-4o",
        ));
        // Default pipeline should be returned for any task type
        let pipeline = registry.get(&TaskType::CodeGeneration);
        // We cannot easily assert identity, but we can verify it does not panic
        let _ = pipeline.classifier.classify(&ChatRequest {
            messages: vec![],
            tools: vec![],
            model: None,
            max_tokens: None,
            temperature: None,
        });
    }

    #[test]
    fn pipeline_registry_register_and_get() {
        let mut registry = PipelineRegistry::new(make_test_pipeline(
            TaskType::Chat,
            "openai",
            "gpt-4o",
        ));
        registry.register(
            TaskType::CodeGeneration,
            make_test_pipeline(TaskType::CodeGeneration, "anthropic", "claude-opus-4-5"),
        );

        // Registered type should return the specialized pipeline
        let _code_pipeline = registry.get(&TaskType::CodeGeneration);
        // Unregistered type should return the default
        let _default_pipeline = registry.get(&TaskType::Research);
    }

    #[tokio::test]
    async fn pipeline_registry_complete_orchestrates_all_stages() {
        let registry = PipelineRegistry::new(make_test_pipeline(
            TaskType::Chat,
            "openai",
            "gpt-4o",
        ));

        let request = ChatRequest {
            messages: vec![LlmMessage {
                role: "user".into(),
                content: "hello".into(),
                tool_call_id: None,
            }],
            tools: vec![],
            model: None,
            max_tokens: None,
            temperature: None,
        };

        let response = registry.complete(&request).await.unwrap();
        assert_eq!(response.id, "test-resp");
        assert_eq!(response.stop_reason, StopReason::EndTurn);
        match &response.content[0] {
            ContentBlock::Text { text } => {
                assert_eq!(text, "Hello from test transport");
            }
            _ => panic!("expected text block"),
        }
    }

    #[tokio::test]
    async fn pipeline_registry_complete_uses_specialized_pipeline() {
        let mut registry = PipelineRegistry::new(make_test_pipeline(
            TaskType::CodeGeneration,
            "default-provider",
            "default-model",
        ));
        registry.register(
            TaskType::CodeGeneration,
            make_test_pipeline(TaskType::CodeGeneration, "anthropic", "claude-opus-4-5"),
        );

        let request = ChatRequest {
            messages: vec![LlmMessage {
                role: "user".into(),
                content: "write code".into(),
                tool_call_id: None,
            }],
            tools: vec![],
            model: None,
            max_tokens: None,
            temperature: None,
        };

        // The classifier returns CodeGeneration, so the specialized pipeline is used.
        let response = registry.complete(&request).await.unwrap();
        assert_eq!(response.id, "test-resp");
    }
}
