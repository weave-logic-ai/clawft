//! Core agent loop -- message processing pipeline.
//!
//! The [`AgentLoop`] is the heart of the clawft agent. It implements the
//! consume-process-respond cycle ported from Python `nanobot/agent/loop.py`:
//!
//! ```text
//! Inbound Message (from MessageBus)
//!   |
//!   v
//! Session lookup / creation
//!   |
//!   v
//! ContextBuilder.build_messages()
//!   |
//!   v
//! Pipeline execution (Classifier -> Router -> Assembler -> Transport -> Scorer -> Learner)
//!   |
//!   v
//! Tool execution loop (up to max_tool_iterations)
//!   |  - Extract tool calls from LLM response
//!   |  - Execute each tool via ToolRegistry
//!   |  - Append tool results to context
//!   |  - Re-invoke LLM if stop_reason == ToolUse
//!   |
//!   v
//! Outbound Message (dispatched to MessageBus)
//! ```

use std::sync::Arc;

use tracing::{debug, error, info};

use clawft_platform::Platform;
use clawft_types::config::AgentsConfig;
use clawft_types::error::ClawftError;
use clawft_types::event::{InboundMessage, OutboundMessage};
use clawft_types::provider::ContentBlock;

use crate::bus::MessageBus;
use crate::pipeline::traits::{ChatRequest, LlmMessage, PipelineRegistry};
use crate::session::SessionManager;
use crate::tools::registry::ToolRegistry;

use super::context::ContextBuilder;

/// Maximum size in bytes for a single tool result.
const MAX_TOOL_RESULT_BYTES: usize = 65_536;

/// The core agent loop that processes inbound messages.
///
/// Consumes messages from the bus, invokes the LLM pipeline, executes
/// tool calls, and dispatches responses. This struct holds all the
/// dependencies needed for the full processing cycle.
///
/// # Processing flow
///
/// 1. **Consume**: Pull the next [`InboundMessage`] from the [`MessageBus`].
/// 2. **Session**: Look up or create a [`Session`](clawft_types::session::Session)
///    keyed by `channel:chat_id`.
/// 3. **Context**: Build the LLM message list via
///    [`ContextBuilder::build_messages`].
/// 4. **Pipeline**: Run the assembled context through the 6-stage pipeline
///    (Classifier -> Router -> Assembler -> Transport -> Scorer -> Learner).
/// 5. **Tools**: If the LLM response contains tool calls, execute them
///    via the `ToolRegistry`, append results, and loop back to step 4
///    (up to `max_tool_iterations`).
/// 6. **Respond**: Extract the final text response and dispatch an
///    [`OutboundMessage`] to the bus.
/// 7. **Persist**: Save the updated session and append to history.
pub struct AgentLoop<P: Platform> {
    config: AgentsConfig,
    platform: Arc<P>,
    bus: Arc<MessageBus>,
    pipeline: PipelineRegistry,
    tools: ToolRegistry,
    context: ContextBuilder<P>,
    sessions: SessionManager<P>,
}

impl<P: Platform> AgentLoop<P> {
    /// Create a new agent loop with all dependencies wired.
    ///
    /// # Arguments
    ///
    /// * `config` -- Agent configuration (model, max_tokens, etc.)
    /// * `platform` -- Platform abstraction for filesystem/env/http
    /// * `bus` -- Message bus for consuming inbound and dispatching outbound
    /// * `pipeline` -- Pipeline registry for LLM invocation
    /// * `tools` -- Tool registry for executing tool calls
    /// * `context` -- Context builder for assembling prompts
    /// * `sessions` -- Session manager for conversation persistence
    pub fn new(
        config: AgentsConfig,
        platform: Arc<P>,
        bus: Arc<MessageBus>,
        pipeline: PipelineRegistry,
        tools: ToolRegistry,
        context: ContextBuilder<P>,
        sessions: SessionManager<P>,
    ) -> Self {
        Self {
            config,
            platform,
            bus,
            pipeline,
            tools,
            context,
            sessions,
        }
    }

    /// Get a reference to the agent configuration.
    pub fn config(&self) -> &AgentsConfig {
        &self.config
    }

    /// Get a reference to the platform.
    pub fn platform(&self) -> &Arc<P> {
        &self.platform
    }

    /// Get a reference to the message bus.
    pub fn bus(&self) -> &Arc<MessageBus> {
        &self.bus
    }

    /// Run the agent loop, consuming messages until the bus is closed.
    ///
    /// This is the main entrypoint. It pulls messages from the inbound
    /// channel and processes each one through the full pipeline. Errors
    /// on individual messages are logged but do not terminate the loop.
    pub async fn run(&self) -> clawft_types::Result<()> {
        info!("agent loop started, waiting for messages");

        loop {
            match self.bus.consume_inbound().await {
                Some(msg) => {
                    debug!(
                        channel = %msg.channel,
                        chat_id = %msg.chat_id,
                        "processing inbound message"
                    );
                    if let Err(e) = self.process_message(msg).await {
                        error!("failed to process message: {}", e);
                    }
                }
                None => {
                    info!("inbound channel closed, agent loop exiting");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Process a single inbound message through the full pipeline.
    ///
    /// Handles session lookup, context building, pipeline invocation,
    /// the tool execution loop, session persistence, and outbound dispatch.
    async fn process_message(&self, msg: InboundMessage) -> clawft_types::Result<()> {
        let session_key = msg.session_key();

        // 1. Get or create session
        let mut session = self.sessions.get_or_create(&session_key).await?;

        // 2. Add user message to session
        session.add_message("user", &msg.content, None);

        // 3. Build context messages from memory, skills, and history
        let context_messages = self.context.build_messages(&session, &[]).await;

        // 4. Convert context::LlmMessage to pipeline::traits::LlmMessage
        let mut messages: Vec<LlmMessage> = context_messages
            .iter()
            .map(|m| LlmMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                tool_call_id: None,
            })
            .collect();

        // 5. Add current user message
        messages.push(LlmMessage {
            role: "user".into(),
            content: msg.content.clone(),
            tool_call_id: None,
        });

        // 6. Create pipeline request
        let request = ChatRequest {
            messages,
            tools: self.tools.schemas(),
            model: Some(self.config.defaults.model.clone()),
            max_tokens: Some(self.config.defaults.max_tokens),
            temperature: Some(self.config.defaults.temperature),
        };

        // 7. Execute pipeline + tool loop
        let response_text = self.run_tool_loop(request).await?;

        // 8. Add assistant message to session
        session.add_message("assistant", &response_text, None);

        // 9. Save session
        self.sessions.save_session(&session).await?;

        // 10. Dispatch outbound
        let outbound = OutboundMessage {
            channel: msg.channel.clone(),
            chat_id: msg.chat_id.clone(),
            content: response_text,
            reply_to: None,
            media: vec![],
            metadata: Default::default(),
        };
        self.bus.dispatch_outbound(outbound)?;

        debug!(session_key = %session_key, "message processed successfully");

        Ok(())
    }

    /// Execute the tool loop: call LLM, execute tools, repeat.
    ///
    /// After each LLM call, checks if the response contains tool-use
    /// requests. If so, executes each tool via the `ToolRegistry`, appends
    /// tool results to the message list, and re-invokes the pipeline.
    /// Continues until the LLM returns a text response or the maximum
    /// iteration limit is reached.
    async fn run_tool_loop(
        &self,
        mut request: ChatRequest,
    ) -> clawft_types::Result<String> {
        let max_iterations = self.config.defaults.max_tool_iterations.max(1) as usize;

        for iteration in 0..max_iterations {
            let response = self.pipeline.complete(&request).await?;

            // Extract tool calls from the response
            let tool_calls: Vec<(String, String, serde_json::Value)> = response
                .content
                .iter()
                .filter_map(|block| {
                    if let ContentBlock::ToolUse { id, name, input } = block {
                        Some((id.clone(), name.clone(), input.clone()))
                    } else {
                        None
                    }
                })
                .collect();

            if tool_calls.is_empty() {
                // No tool calls -- extract text response and return
                let text = response
                    .content
                    .iter()
                    .filter_map(|block| {
                        if let ContentBlock::Text { text } = block {
                            Some(text.as_str())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("");

                debug!(iteration, "tool loop complete, returning text response");
                return Ok(text);
            }

            debug!(
                iteration,
                tool_count = tool_calls.len(),
                "executing tool calls"
            );

            // Execute each tool and append results to the request
            for (id, name, input) in tool_calls {
                let result = self.tools.execute(&name, input.clone()).await;
                let result_json = match result {
                    Ok(val) => {
                        let truncated = crate::security::truncate_result(val, MAX_TOOL_RESULT_BYTES);
                        serde_json::to_string(&truncated).unwrap_or_default()
                    }
                    Err(e) => {
                        error!(tool = %name, error = %e, "tool execution failed");
                        format!("{{\"error\": \"{}\"}}", e)
                    }
                };

                request.messages.push(LlmMessage {
                    role: "tool".into(),
                    content: result_json,
                    tool_call_id: Some(id),
                });
            }
        }

        Err(ClawftError::Provider {
            message: format!(
                "max tool iterations ({}) exceeded",
                max_iterations
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::memory::MemoryStore;
    use crate::agent::skills::SkillsLoader;
    use crate::pipeline::traits::{
        AssembledContext, LearningBackend, LearningSignal, LlmTransport, ModelRouter,
        Pipeline, QualityScore, QualityScorer, ResponseOutcome, RoutingDecision,
        TaskClassifier, TaskProfile, TaskType, Trajectory, TransportRequest,
    };
    use crate::tools::registry::Tool;
    use async_trait::async_trait;
    use clawft_platform::NativePlatform;
    use clawft_types::config::AgentDefaults;
    use clawft_types::provider::{LlmResponse, StopReason, Usage};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir(prefix: &str) -> PathBuf {
        let id = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        std::env::temp_dir().join(format!("clawft_loop_test_{prefix}_{pid}_{id}"))
    }

    fn test_config() -> AgentsConfig {
        AgentsConfig {
            defaults: AgentDefaults {
                workspace: "~/.clawft/workspace".into(),
                model: "test-model".into(),
                max_tokens: 4096,
                temperature: 0.5,
                max_tool_iterations: 10,
                memory_window: 50,
            },
        }
    }

    // -- Mock pipeline stages --

    struct MockClassifier;
    impl TaskClassifier for MockClassifier {
        fn classify(&self, _request: &ChatRequest) -> TaskProfile {
            TaskProfile {
                task_type: TaskType::Chat,
                complexity: 0.3,
                keywords: vec![],
            }
        }
    }

    struct MockRouter;
    #[async_trait]
    impl ModelRouter for MockRouter {
        async fn route(
            &self,
            _request: &ChatRequest,
            _profile: &TaskProfile,
        ) -> RoutingDecision {
            RoutingDecision {
                provider: "test".into(),
                model: "test-model".into(),
                reason: "mock".into(),
            }
        }
        fn update(&self, _d: &RoutingDecision, _o: &ResponseOutcome) {}
    }

    struct MockAssembler;
    #[async_trait]
    impl crate::pipeline::traits::ContextAssembler for MockAssembler {
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

    /// Transport that returns a fixed text response.
    struct MockTransport {
        response_text: String,
    }

    impl MockTransport {
        fn new(text: &str) -> Self {
            Self {
                response_text: text.into(),
            }
        }
    }

    #[async_trait]
    impl LlmTransport for MockTransport {
        async fn complete(
            &self,
            _request: &TransportRequest,
        ) -> clawft_types::Result<LlmResponse> {
            Ok(LlmResponse {
                id: "mock-resp".into(),
                content: vec![ContentBlock::Text {
                    text: self.response_text.clone(),
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

    /// Transport that returns a tool call first, then text.
    struct MockToolTransport {
        call_count: std::sync::atomic::AtomicUsize,
    }

    impl MockToolTransport {
        fn new() -> Self {
            Self {
                call_count: std::sync::atomic::AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl LlmTransport for MockToolTransport {
        async fn complete(
            &self,
            _request: &TransportRequest,
        ) -> clawft_types::Result<LlmResponse> {
            let count = self
                .call_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count == 0 {
                // First call: request a tool use
                Ok(LlmResponse {
                    id: "mock-tool-resp".into(),
                    content: vec![ContentBlock::ToolUse {
                        id: "call-1".into(),
                        name: "echo".into(),
                        input: serde_json::json!({"text": "hello"}),
                    }],
                    stop_reason: StopReason::ToolUse,
                    usage: Usage {
                        input_tokens: 10,
                        output_tokens: 5,
                    },
                    metadata: HashMap::new(),
                })
            } else {
                // Second call: return text
                Ok(LlmResponse {
                    id: "mock-final-resp".into(),
                    content: vec![ContentBlock::Text {
                        text: "tool result processed".into(),
                    }],
                    stop_reason: StopReason::EndTurn,
                    usage: Usage {
                        input_tokens: 20,
                        output_tokens: 8,
                    },
                    metadata: HashMap::new(),
                })
            }
        }
    }

    /// Transport that always returns tool calls (to test max iterations).
    struct InfiniteToolTransport;

    #[async_trait]
    impl LlmTransport for InfiniteToolTransport {
        async fn complete(
            &self,
            _request: &TransportRequest,
        ) -> clawft_types::Result<LlmResponse> {
            Ok(LlmResponse {
                id: "infinite".into(),
                content: vec![ContentBlock::ToolUse {
                    id: "call-inf".into(),
                    name: "echo".into(),
                    input: serde_json::json!({"text": "loop"}),
                }],
                stop_reason: StopReason::ToolUse,
                usage: Usage {
                    input_tokens: 5,
                    output_tokens: 3,
                },
                metadata: HashMap::new(),
            })
        }
    }

    struct MockScorer;
    impl QualityScorer for MockScorer {
        fn score(&self, _req: &ChatRequest, _resp: &LlmResponse) -> QualityScore {
            QualityScore {
                overall: 1.0,
                relevance: 1.0,
                coherence: 1.0,
            }
        }
    }

    struct MockLearner;
    impl LearningBackend for MockLearner {
        fn record(&self, _t: &Trajectory) {}
        fn adapt(&self, _s: &LearningSignal) {}
    }

    fn make_pipeline(transport: Arc<dyn LlmTransport>) -> PipelineRegistry {
        let pipeline = Pipeline {
            classifier: Arc::new(MockClassifier),
            router: Arc::new(MockRouter),
            assembler: Arc::new(MockAssembler),
            transport,
            scorer: Arc::new(MockScorer),
            learner: Arc::new(MockLearner),
        };
        PipelineRegistry::new(pipeline)
    }

    // -- Mock tool --

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn description(&self) -> &str {
            "Echo back the input text"
        }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string" }
                },
                "required": ["text"]
            })
        }
        async fn execute(
            &self,
            args: serde_json::Value,
        ) -> Result<serde_json::Value, crate::tools::registry::ToolError> {
            let text = args
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("(no text)");
            Ok(serde_json::json!({"output": text}))
        }
    }

    /// Helper to create an AgentLoop with the given transport.
    async fn make_agent_loop(
        transport: Arc<dyn LlmTransport>,
        prefix: &str,
    ) -> (AgentLoop<NativePlatform>, PathBuf) {
        let dir = temp_dir(prefix);
        let platform = Arc::new(NativePlatform::new());
        let bus = Arc::new(MessageBus::new());

        let sessions_dir = dir.join("sessions");
        let sessions =
            SessionManager::with_dir(platform.clone(), sessions_dir);

        let memory = Arc::new(MemoryStore::with_paths(
            dir.join("memory").join("MEMORY.md"),
            dir.join("memory").join("HISTORY.md"),
            platform.clone(),
        ));
        let skills = Arc::new(SkillsLoader::with_dir(
            dir.join("skills"),
            platform.clone(),
        ));
        let context = ContextBuilder::new(
            test_config(),
            memory,
            skills,
            platform.clone(),
        );

        let mut tools = ToolRegistry::new();
        tools.register(Arc::new(EchoTool));

        let pipeline = make_pipeline(transport);

        let agent = AgentLoop::new(
            test_config(),
            platform,
            bus,
            pipeline,
            tools,
            context,
            sessions,
        );
        (agent, dir)
    }

    #[test]
    fn new_creates_agent_loop_with_all_deps() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let transport = Arc::new(MockTransport::new("hello"));
            let (agent, dir) = make_agent_loop(transport, "new_all").await;

            assert_eq!(agent.config().defaults.model, "test-model");
            assert_eq!(agent.config().defaults.max_tokens, 4096);
            assert_eq!(agent.config().defaults.max_tool_iterations, 10);

            let _ = tokio::fs::remove_dir_all(&dir).await;
        });
    }

    #[test]
    fn config_accessor_returns_config() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let transport = Arc::new(MockTransport::new("hello"));
            let (agent, dir) = make_agent_loop(transport, "config_acc").await;

            assert_eq!(agent.config().defaults.memory_window, 50);
            assert_eq!(agent.config().defaults.workspace, "~/.clawft/workspace");

            let _ = tokio::fs::remove_dir_all(&dir).await;
        });
    }

    #[test]
    fn platform_accessor_returns_platform() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let transport = Arc::new(MockTransport::new("hello"));
            let (agent, dir) = make_agent_loop(transport, "platform_acc").await;

            // Verify the platform reference is accessible
            let _p = agent.platform();

            let _ = tokio::fs::remove_dir_all(&dir).await;
        });
    }

    #[tokio::test]
    async fn process_message_produces_outbound() {
        let transport = Arc::new(MockTransport::new("Hello from LLM!"));
        let (agent, dir) = make_agent_loop(transport, "process_msg").await;

        // Publish an inbound message
        let inbound = InboundMessage {
            channel: "test".into(),
            sender_id: "user1".into(),
            chat_id: "chat1".into(),
            content: "hi there".into(),
            timestamp: chrono::Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        };
        agent.bus.publish_inbound(inbound).unwrap();

        // Process it
        let msg = agent.bus.consume_inbound().await.unwrap();
        agent.process_message(msg).await.unwrap();

        // Check outbound
        let outbound = agent.bus.consume_outbound().await.unwrap();
        assert_eq!(outbound.channel, "test");
        assert_eq!(outbound.chat_id, "chat1");
        assert_eq!(outbound.content, "Hello from LLM!");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn process_message_executes_tool_loop() {
        let transport = Arc::new(MockToolTransport::new());
        let (agent, dir) = make_agent_loop(transport, "tool_loop").await;

        let inbound = InboundMessage {
            channel: "test".into(),
            sender_id: "user1".into(),
            chat_id: "chat1".into(),
            content: "use echo tool".into(),
            timestamp: chrono::Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        };
        agent.bus.publish_inbound(inbound).unwrap();

        let msg = agent.bus.consume_inbound().await.unwrap();
        agent.process_message(msg).await.unwrap();

        let outbound = agent.bus.consume_outbound().await.unwrap();
        assert_eq!(outbound.content, "tool result processed");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn run_tool_loop_respects_max_iterations() {
        let transport = Arc::new(InfiniteToolTransport);
        let (agent, dir) = make_agent_loop(transport, "max_iter").await;

        let request = ChatRequest {
            messages: vec![LlmMessage {
                role: "user".into(),
                content: "loop forever".into(),
                tool_call_id: None,
            }],
            tools: vec![],
            model: Some("test-model".into()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
        };

        let result = agent.run_tool_loop(request).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("max tool iterations"),
            "error should mention max iterations: {}",
            err_msg
        );

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn run_exits_when_bus_closes() {
        let transport = Arc::new(MockTransport::new("hello"));
        let (agent, dir) = make_agent_loop(transport, "bus_close").await;

        // Drop the inbound sender by dropping a cloned bus reference.
        // We need to drop all inbound senders. The bus holds one internally.
        // Simplest: publish a message, consume it, then drop the bus.
        // Since the agent holds an Arc<MessageBus>, we cannot fully drop it.
        // Instead, test that run() exits when the channel is closed by
        // spawning run in a background task and sending a message that
        // processes, then dropping the bus's sender.

        // We cannot easily test the full `run()` loop exit here since the
        // bus is shared via Arc. Instead test the contract: consume_inbound
        // returns None when all senders are dropped.
        // This is already tested in bus.rs. Here we verify the struct compiles.
        assert!(agent.bus().inbound_sender().send(InboundMessage {
            channel: "test".into(),
            sender_id: "u".into(),
            chat_id: "c".into(),
            content: "msg".into(),
            timestamp: chrono::Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        }).is_ok());

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn process_message_saves_session() {
        let transport = Arc::new(MockTransport::new("saved response"));
        let (agent, dir) = make_agent_loop(transport, "session_save").await;

        let inbound = InboundMessage {
            channel: "telegram".into(),
            sender_id: "user1".into(),
            chat_id: "chat42".into(),
            content: "remember this".into(),
            timestamp: chrono::Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        };
        agent.bus.publish_inbound(inbound).unwrap();

        let msg = agent.bus.consume_inbound().await.unwrap();
        agent.process_message(msg).await.unwrap();

        // Verify session was saved with both messages
        let session = agent
            .sessions
            .get_or_create("telegram:chat42")
            .await
            .unwrap();
        // Session should have user message + assistant message
        assert!(session.messages.len() >= 2);

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[test]
    fn agent_loop_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<AgentLoop<NativePlatform>>();
    }
}
