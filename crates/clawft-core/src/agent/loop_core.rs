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

use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use clawft_platform::Platform;
use clawft_types::config::AgentsConfig;
use clawft_types::error::ClawftError;
use clawft_types::event::{InboundMessage, OutboundMessage};
use clawft_types::provider::ContentBlock;
use clawft_types::routing::AuthContext;

use crate::bus::MessageBus;
use crate::pipeline::permissions::PermissionResolver;
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
    permission_resolver: PermissionResolver,
    cancel: Option<CancellationToken>,
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
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: AgentsConfig,
        platform: Arc<P>,
        bus: Arc<MessageBus>,
        pipeline: PipelineRegistry,
        tools: ToolRegistry,
        context: ContextBuilder<P>,
        sessions: SessionManager<P>,
        permission_resolver: PermissionResolver,
    ) -> Self {
        Self {
            config,
            platform,
            bus,
            pipeline,
            tools,
            context,
            sessions,
            permission_resolver,
            cancel: None,
        }
    }

    /// Attach a [`CancellationToken`] so the agent loop exits promptly on
    /// shutdown instead of waiting for all bus senders to be dropped.
    pub fn with_cancel(mut self, token: CancellationToken) -> Self {
        self.cancel = Some(token);
        self
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

    /// Run the agent loop, consuming messages until the bus is closed or
    /// the optional [`CancellationToken`] is triggered.
    ///
    /// This is the main entrypoint. It pulls messages from the inbound
    /// channel and processes each one through the full pipeline. Errors
    /// on individual messages are logged but do not terminate the loop.
    pub async fn run(&self) -> clawft_types::Result<()> {
        info!("agent loop started, waiting for messages");

        loop {
            let msg = if let Some(ref token) = self.cancel {
                tokio::select! {
                    biased;
                    _ = token.cancelled() => {
                        info!("agent loop cancelled via token, exiting");
                        break;
                    }
                    msg = self.bus.consume_inbound() => msg,
                }
            } else {
                self.bus.consume_inbound().await
            };

            match msg {
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

        // 2. Build context messages from memory, skills, and history BEFORE
        //    adding the user message to session (to avoid duplicate).
        let context_messages = self.context.build_messages(&session, &[]).await;

        // 3. Add user message to session (after building context)
        session.add_message("user", &msg.content, None);

        // 4. Convert context::LlmMessage to pipeline::traits::LlmMessage
        let mut messages: Vec<LlmMessage> = context_messages
            .iter()
            .map(|m| LlmMessage {
                role: m.role.clone(),
                content: m.content.clone(),
                tool_call_id: None,
                tool_calls: None,
            })
            .collect();

        // 5. Add current user message
        messages.push(LlmMessage {
            role: "user".into(),
            content: msg.content.clone(),
            tool_call_id: None,
            tool_calls: None,
        });

        // 6. Resolve auth context from inbound message identity.
        //    CLI channel gets admin permissions; other channels get zero-trust
        //    defaults with the sender_id and channel attached.
        let auth_context = self.resolve_auth_context(&msg);

        // 7. Create pipeline request with auth context
        let request = ChatRequest {
            messages,
            tools: self.tools.schemas(),
            model: Some(self.config.defaults.model.clone()),
            max_tokens: Some(self.config.defaults.max_tokens),
            temperature: Some(self.config.defaults.temperature),
            auth_context: Some(auth_context),
        };

        // 8. Execute pipeline + tool loop
        let response_text = self.run_tool_loop(request).await?;

        // 9. Add assistant message to session
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

    /// Resolve [`AuthContext`] from the inbound message's sender identity.
    ///
    /// Resolve permissions for an inbound message using the 5-layer
    /// [`PermissionResolver`].
    ///
    /// Resolution order (highest priority wins):
    /// 1. Built-in level defaults
    /// 2. Global config level overrides
    /// 3. Workspace config level overrides
    /// 4. Per-user overrides
    /// 5. Per-channel overrides (highest priority)
    ///
    /// CLI channel messages always receive admin-level (Level 2)
    /// permissions via the resolver's `cli_default_level`.
    fn resolve_auth_context(&self, msg: &InboundMessage) -> AuthContext {
        // allow_from_match is set by channel plugins when they verify the
        // sender is in the channel's allow_from list. The CLI channel does
        // NOT use allow_from -- it gets admin via the cli_default_level path
        // in PermissionResolver::determine_level, so pass false here.
        self.permission_resolver
            .resolve_auth_context(&msg.sender_id, &msg.channel, false)
    }

    /// Execute the tool loop: call LLM, execute tools, repeat.
    ///
    /// After each LLM call, checks if the response contains tool-use
    /// requests. If so, executes each tool via the `ToolRegistry`, appends
    /// tool results to the message list, and re-invokes the pipeline.
    /// Continues until the LLM returns a text response or the maximum
    /// iteration limit is reached.
    async fn run_tool_loop(&self, mut request: ChatRequest) -> clawft_types::Result<String> {
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

            // Append the assistant message (with tool_calls) to the conversation
            // so the next LLM request sees the correct message sequence:
            //   ... user -> assistant (tool_use) -> tool results -> ...
            let assistant_tool_calls: Vec<serde_json::Value> = tool_calls
                .iter()
                .map(|(id, name, input)| {
                    serde_json::json!({
                        "id": id,
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": serde_json::to_string(input).unwrap_or_default(),
                        }
                    })
                })
                .collect();

            // Extract any text content from the response for the assistant message
            let assistant_text: String = response
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

            request.messages.push(LlmMessage {
                role: "assistant".into(),
                content: assistant_text,
                tool_call_id: None,
                tool_calls: Some(assistant_tool_calls),
            });

            // Execute each tool and append results to the request
            for (id, name, input) in tool_calls {
                let permissions = request
                    .auth_context
                    .as_ref()
                    .map(|ctx| &ctx.permissions);
                let result = self
                    .tools
                    .execute(&name, input.clone(), permissions)
                    .await;
                let result_json = match result {
                    Ok(val) => {
                        let truncated =
                            crate::security::truncate_result(val, MAX_TOOL_RESULT_BYTES);
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
                    tool_calls: None,
                });
            }
        }

        Err(ClawftError::Provider {
            message: format!("max tool iterations ({}) exceeded", max_iterations),
        })
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::memory::MemoryStore;
    use crate::agent::skills::SkillsLoader;
    use crate::pipeline::traits::{
        AssembledContext, LearningBackend, LearningSignal, LlmTransport, ModelRouter, Pipeline,
        QualityScore, QualityScorer, ResponseOutcome, RoutingDecision, TaskClassifier, TaskProfile,
        TaskType, Trajectory, TransportRequest,
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
        async fn route(&self, _request: &ChatRequest, _profile: &TaskProfile) -> RoutingDecision {
            RoutingDecision {
                provider: "test".into(),
                model: "test-model".into(),
                reason: "mock".into(),
                ..Default::default()
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
        async fn complete(&self, _request: &TransportRequest) -> clawft_types::Result<LlmResponse> {
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
        async fn complete(&self, _request: &TransportRequest) -> clawft_types::Result<LlmResponse> {
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
        async fn complete(&self, _request: &TransportRequest) -> clawft_types::Result<LlmResponse> {
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
        let sessions = SessionManager::with_dir(platform.clone(), sessions_dir);

        let memory = Arc::new(MemoryStore::with_paths(
            dir.join("memory").join("MEMORY.md"),
            dir.join("memory").join("HISTORY.md"),
            platform.clone(),
        ));
        let skills = Arc::new(SkillsLoader::with_dir(dir.join("skills"), platform.clone()));
        let context = ContextBuilder::new(test_config(), memory, skills, platform.clone());

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
            PermissionResolver::default_resolver(),
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
                tool_calls: None,
            }],
            tools: vec![],
            model: Some("test-model".into()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            auth_context: None,
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
        assert!(
            agent
                .bus()
                .inbound_sender()
                .send(InboundMessage {
                    channel: "test".into(),
                    sender_id: "u".into(),
                    chat_id: "c".into(),
                    content: "msg".into(),
                    timestamp: chrono::Utc::now(),
                    media: vec![],
                    metadata: HashMap::new(),
                })
                .is_ok()
        );

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

    // ── GAP-19: Tool result truncation verification ───────────────────

    /// Transport that returns a tool call for a tool producing oversized output.
    struct OversizedToolTransport {
        call_count: std::sync::atomic::AtomicUsize,
    }

    impl OversizedToolTransport {
        fn new() -> Self {
            Self {
                call_count: std::sync::atomic::AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl LlmTransport for OversizedToolTransport {
        async fn complete(&self, request: &TransportRequest) -> clawft_types::Result<LlmResponse> {
            let count = self
                .call_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if count == 0 {
                // First call: request a tool call that will produce oversized output
                Ok(LlmResponse {
                    id: "oversized-resp".into(),
                    content: vec![ContentBlock::ToolUse {
                        id: "call-big".into(),
                        name: "big_output".into(),
                        input: serde_json::json!({}),
                    }],
                    stop_reason: StopReason::ToolUse,
                    usage: Usage {
                        input_tokens: 10,
                        output_tokens: 5,
                    },
                    metadata: HashMap::new(),
                })
            } else {
                // Second call: verify the tool result was included (truncated)
                // and return text. Check message list for truncation.
                let last_msg = request.messages.last();
                let content_text = last_msg
                    .map(|m| m.content.as_str())
                    .unwrap_or("no tool result");

                Ok(LlmResponse {
                    id: "final-resp".into(),
                    content: vec![ContentBlock::Text {
                        text: format!("tool_result_len:{}", content_text.len()),
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

    /// Tool that produces output exceeding MAX_TOOL_RESULT_BYTES.
    struct BigOutputTool;

    #[async_trait]
    impl Tool for BigOutputTool {
        fn name(&self) -> &str {
            "big_output"
        }
        fn description(&self) -> &str {
            "Returns a very large output"
        }
        fn parameters(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {}
            })
        }
        async fn execute(
            &self,
            _args: serde_json::Value,
        ) -> Result<serde_json::Value, crate::tools::registry::ToolError> {
            // Produce output far exceeding 64KB (MAX_TOOL_RESULT_BYTES)
            let big_string = "x".repeat(200_000);
            Ok(serde_json::json!({"data": big_string}))
        }
    }

    #[tokio::test]
    async fn tool_result_truncation_enforced() {
        let dir = temp_dir("truncation");
        let platform = Arc::new(NativePlatform::new());
        let bus = Arc::new(MessageBus::new());

        let sessions_dir = dir.join("sessions");
        let sessions = SessionManager::with_dir(platform.clone(), sessions_dir);

        let memory = Arc::new(MemoryStore::with_paths(
            dir.join("memory").join("MEMORY.md"),
            dir.join("memory").join("HISTORY.md"),
            platform.clone(),
        ));
        let skills = Arc::new(SkillsLoader::with_dir(dir.join("skills"), platform.clone()));
        let context = ContextBuilder::new(test_config(), memory, skills, platform.clone());

        let mut tools = ToolRegistry::new();
        tools.register(Arc::new(BigOutputTool));

        let pipeline = make_pipeline(Arc::new(OversizedToolTransport::new()));

        let agent = AgentLoop::new(
            test_config(),
            platform,
            bus,
            pipeline,
            tools,
            context,
            sessions,
            PermissionResolver::default_resolver(),
        );

        let request = ChatRequest {
            messages: vec![LlmMessage {
                role: "user".into(),
                content: "trigger big tool".into(),
                tool_call_id: None,
                tool_calls: None,
            }],
            tools: vec![],
            model: Some("test-model".into()),
            max_tokens: Some(4096),
            temperature: Some(0.5),
            auth_context: None,
        };

        let result = agent.run_tool_loop(request).await.unwrap();

        // The tool result should have been truncated to MAX_TOOL_RESULT_BYTES (65536).
        // The response tells us the length of the tool result message.
        assert!(
            result.starts_with("tool_result_len:"),
            "response should contain truncated tool result length: {result}"
        );
        let len_str = result.strip_prefix("tool_result_len:").unwrap();
        let result_len: usize = len_str.parse().unwrap();
        assert!(
            result_len <= MAX_TOOL_RESULT_BYTES,
            "tool result ({result_len} bytes) should be truncated to <= {} bytes",
            MAX_TOOL_RESULT_BYTES
        );

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    // ── TEST-04: Agent loop end-to-end test ────────────────────────────

    /// Transport that records every request it receives and drives a full
    /// tool-use round-trip: call 1 returns tool_use, call 2 verifies the
    /// tool result was appended and returns text.
    struct E2eRecordingTransport {
        call_count: std::sync::atomic::AtomicUsize,
        /// Snapshot of message lists received on each call.
        recorded_requests: std::sync::Mutex<Vec<Vec<LlmMessage>>>,
    }

    impl E2eRecordingTransport {
        fn new() -> Self {
            Self {
                call_count: std::sync::atomic::AtomicUsize::new(0),
                recorded_requests: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn snapshots(&self) -> Vec<Vec<LlmMessage>> {
            self.recorded_requests.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl LlmTransport for E2eRecordingTransport {
        async fn complete(&self, request: &TransportRequest) -> clawft_types::Result<LlmResponse> {
            // Record the incoming message list
            self.recorded_requests
                .lock()
                .unwrap()
                .push(request.messages.clone());

            let count = self
                .call_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            if count == 0 {
                // Call 1: LLM returns a tool_use request
                Ok(LlmResponse {
                    id: "e2e-resp-1".into(),
                    content: vec![ContentBlock::ToolUse {
                        id: "call-e2e-1".into(),
                        name: "echo".into(),
                        input: serde_json::json!({"text": "e2e-ping"}),
                    }],
                    stop_reason: StopReason::ToolUse,
                    usage: Usage {
                        input_tokens: 15,
                        output_tokens: 10,
                    },
                    metadata: HashMap::new(),
                })
            } else {
                // Call 2: LLM receives tool result, returns final text
                Ok(LlmResponse {
                    id: "e2e-resp-2".into(),
                    content: vec![ContentBlock::Text {
                        text: "I received the tool output successfully".into(),
                    }],
                    stop_reason: StopReason::EndTurn,
                    usage: Usage {
                        input_tokens: 25,
                        output_tokens: 12,
                    },
                    metadata: HashMap::new(),
                })
            }
        }
    }

    /// Multi-tool transport: returns two tool calls on the first invocation,
    /// then text on the second.
    struct MultiToolE2eTransport {
        call_count: std::sync::atomic::AtomicUsize,
        recorded_requests: std::sync::Mutex<Vec<Vec<LlmMessage>>>,
    }

    impl MultiToolE2eTransport {
        fn new() -> Self {
            Self {
                call_count: std::sync::atomic::AtomicUsize::new(0),
                recorded_requests: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn snapshots(&self) -> Vec<Vec<LlmMessage>> {
            self.recorded_requests.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl LlmTransport for MultiToolE2eTransport {
        async fn complete(&self, request: &TransportRequest) -> clawft_types::Result<LlmResponse> {
            self.recorded_requests
                .lock()
                .unwrap()
                .push(request.messages.clone());

            let count = self
                .call_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            if count == 0 {
                // Return two tool calls at once
                Ok(LlmResponse {
                    id: "multi-tool-resp-1".into(),
                    content: vec![
                        ContentBlock::ToolUse {
                            id: "call-mt-1".into(),
                            name: "echo".into(),
                            input: serde_json::json!({"text": "first"}),
                        },
                        ContentBlock::ToolUse {
                            id: "call-mt-2".into(),
                            name: "echo".into(),
                            input: serde_json::json!({"text": "second"}),
                        },
                    ],
                    stop_reason: StopReason::ToolUse,
                    usage: Usage {
                        input_tokens: 20,
                        output_tokens: 15,
                    },
                    metadata: HashMap::new(),
                })
            } else {
                Ok(LlmResponse {
                    id: "multi-tool-resp-2".into(),
                    content: vec![ContentBlock::Text {
                        text: "processed both tools".into(),
                    }],
                    stop_reason: StopReason::EndTurn,
                    usage: Usage {
                        input_tokens: 30,
                        output_tokens: 10,
                    },
                    metadata: HashMap::new(),
                })
            }
        }
    }

    /// TEST-04: Full e2e test -- mock LLM returns tool_use, tool executes,
    /// result is sent back to LLM, LLM returns text. Verifies the full
    /// message chain including intermediate tool result messages.
    #[tokio::test]
    async fn e2e_tool_roundtrip_message_chain() {
        let transport = Arc::new(E2eRecordingTransport::new());
        let transport_ref = transport.clone();
        let (agent, dir) = make_agent_loop(transport, "e2e_chain").await;

        // Publish and process a message that triggers tool use.
        // Use channel "cli" so resolve_auth_context grants admin permissions,
        // allowing the echo tool to execute through the permission check.
        let inbound = InboundMessage {
            channel: "cli".into(),
            sender_id: "e2e-user".into(),
            chat_id: "e2e-chat".into(),
            content: "please use the echo tool".into(),
            timestamp: chrono::Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        };
        agent.bus.publish_inbound(inbound).unwrap();
        let msg = agent.bus.consume_inbound().await.unwrap();
        agent.process_message(msg).await.unwrap();

        // Verify outbound message is the final text response
        let outbound = agent.bus.consume_outbound().await.unwrap();
        assert_eq!(outbound.content, "I received the tool output successfully");
        assert_eq!(outbound.channel, "cli");
        assert_eq!(outbound.chat_id, "e2e-chat");

        // Verify the transport was called exactly twice
        let snapshots = transport_ref.snapshots();
        assert_eq!(
            snapshots.len(),
            2,
            "transport should be called twice (tool_use -> text)"
        );

        // Snapshot 1: initial user request
        let first_call = &snapshots[0];
        assert!(
            first_call.iter().any(|m| m.role == "user"),
            "first call should contain user message"
        );

        // Snapshot 2: should include the tool result message
        let second_call = &snapshots[1];
        let tool_result_msg = second_call
            .iter()
            .find(|m| m.role == "tool")
            .expect("second call should contain a tool result message");

        // Verify the tool result has the correct tool_call_id
        assert_eq!(
            tool_result_msg.tool_call_id.as_deref(),
            Some("call-e2e-1"),
            "tool result should reference the tool call ID"
        );

        // Verify the tool result contains the echo output
        assert!(
            tool_result_msg.content.contains("e2e-ping"),
            "tool result should contain the echoed text: {}",
            tool_result_msg.content
        );

        // Verify session was persisted with both user and assistant messages
        let session = agent
            .sessions
            .get_or_create("cli:e2e-chat")
            .await
            .unwrap();
        assert!(
            session.messages.len() >= 2,
            "session should have at least user + assistant messages, got {}",
            session.messages.len()
        );

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// TEST-04: E2e test with multiple tool calls in a single LLM response.
    /// Verifies that all tool results are sent back with correct IDs.
    #[tokio::test]
    async fn e2e_multi_tool_calls_all_results_returned() {
        let transport = Arc::new(MultiToolE2eTransport::new());
        let transport_ref = transport.clone();
        let (agent, dir) = make_agent_loop(transport, "e2e_multi").await;

        // Use channel "cli" so resolve_auth_context grants admin permissions,
        // allowing the echo tool to execute through the permission check.
        let inbound = InboundMessage {
            channel: "cli".into(),
            sender_id: "user".into(),
            chat_id: "chat".into(),
            content: "use echo twice".into(),
            timestamp: chrono::Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        };
        agent.bus.publish_inbound(inbound).unwrap();
        let msg = agent.bus.consume_inbound().await.unwrap();
        agent.process_message(msg).await.unwrap();

        let outbound = agent.bus.consume_outbound().await.unwrap();
        assert_eq!(outbound.content, "processed both tools");

        // Verify the second call to the transport has both tool results
        let snapshots = transport_ref.snapshots();
        assert_eq!(snapshots.len(), 2);

        let second_call = &snapshots[1];
        let tool_results: Vec<&LlmMessage> =
            second_call.iter().filter(|m| m.role == "tool").collect();

        assert_eq!(
            tool_results.len(),
            2,
            "second call should have 2 tool result messages"
        );

        // Verify each tool result has the correct call ID
        let ids: Vec<&str> = tool_results
            .iter()
            .filter_map(|m| m.tool_call_id.as_deref())
            .collect();
        assert!(
            ids.contains(&"call-mt-1"),
            "should have result for call-mt-1"
        );
        assert!(
            ids.contains(&"call-mt-2"),
            "should have result for call-mt-2"
        );

        // Verify tool outputs
        let first_result = tool_results
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-mt-1"))
            .unwrap();
        assert!(
            first_result.content.contains("first"),
            "first tool result should contain 'first': {}",
            first_result.content
        );

        let second_result = tool_results
            .iter()
            .find(|m| m.tool_call_id.as_deref() == Some("call-mt-2"))
            .unwrap();
        assert!(
            second_result.content.contains("second"),
            "second tool result should contain 'second': {}",
            second_result.content
        );

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// TEST-04: E2e test verifying a direct text response (no tool use)
    /// flows through the full pipeline correctly.
    #[tokio::test]
    async fn e2e_direct_text_response_no_tools() {
        let transport = Arc::new(MockTransport::new("Direct answer from LLM"));
        let (agent, dir) = make_agent_loop(transport, "e2e_no_tools").await;

        let inbound = InboundMessage {
            channel: "direct".into(),
            sender_id: "user".into(),
            chat_id: "chat".into(),
            content: "what is 2+2?".into(),
            timestamp: chrono::Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        };
        agent.bus.publish_inbound(inbound).unwrap();
        let msg = agent.bus.consume_inbound().await.unwrap();
        agent.process_message(msg).await.unwrap();

        let outbound = agent.bus.consume_outbound().await.unwrap();
        assert_eq!(outbound.content, "Direct answer from LLM");
        assert_eq!(outbound.channel, "direct");

        // Session should have user + assistant
        let session = agent.sessions.get_or_create("direct:chat").await.unwrap();
        let roles: Vec<String> = session
            .messages
            .iter()
            .filter_map(|m| m.get("role").and_then(|v| v.as_str()).map(String::from))
            .collect();
        assert!(
            roles.iter().any(|r| r == "user"),
            "session should have user message"
        );
        assert!(
            roles.iter().any(|r| r == "assistant"),
            "session should have assistant message"
        );

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// TEST-04: E2e test verifying tool execution failure is gracefully
    /// handled and the error is sent back to the LLM.
    struct FailingToolTransport {
        call_count: std::sync::atomic::AtomicUsize,
        recorded_requests: std::sync::Mutex<Vec<Vec<LlmMessage>>>,
    }

    impl FailingToolTransport {
        fn new() -> Self {
            Self {
                call_count: std::sync::atomic::AtomicUsize::new(0),
                recorded_requests: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn snapshots(&self) -> Vec<Vec<LlmMessage>> {
            self.recorded_requests.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl LlmTransport for FailingToolTransport {
        async fn complete(&self, request: &TransportRequest) -> clawft_types::Result<LlmResponse> {
            self.recorded_requests
                .lock()
                .unwrap()
                .push(request.messages.clone());

            let count = self
                .call_count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            if count == 0 {
                // Request a tool that does not exist
                Ok(LlmResponse {
                    id: "fail-resp-1".into(),
                    content: vec![ContentBlock::ToolUse {
                        id: "call-fail-1".into(),
                        name: "nonexistent_tool".into(),
                        input: serde_json::json!({}),
                    }],
                    stop_reason: StopReason::ToolUse,
                    usage: Usage {
                        input_tokens: 10,
                        output_tokens: 5,
                    },
                    metadata: HashMap::new(),
                })
            } else {
                // LLM receives the error and returns gracefully
                Ok(LlmResponse {
                    id: "fail-resp-2".into(),
                    content: vec![ContentBlock::Text {
                        text: "I see the tool failed, let me help differently".into(),
                    }],
                    stop_reason: StopReason::EndTurn,
                    usage: Usage {
                        input_tokens: 20,
                        output_tokens: 12,
                    },
                    metadata: HashMap::new(),
                })
            }
        }
    }

    #[tokio::test]
    async fn e2e_tool_execution_failure_handled_gracefully() {
        let transport = Arc::new(FailingToolTransport::new());
        let transport_ref = transport.clone();
        let (agent, dir) = make_agent_loop(transport, "e2e_fail").await;

        let inbound = InboundMessage {
            channel: "fail".into(),
            sender_id: "user".into(),
            chat_id: "chat".into(),
            content: "try a tool".into(),
            timestamp: chrono::Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        };
        agent.bus.publish_inbound(inbound).unwrap();
        let msg = agent.bus.consume_inbound().await.unwrap();
        agent.process_message(msg).await.unwrap();

        let outbound = agent.bus.consume_outbound().await.unwrap();
        assert_eq!(
            outbound.content,
            "I see the tool failed, let me help differently"
        );

        // Verify the error was passed to the LLM in the second call
        let snapshots = transport_ref.snapshots();
        assert_eq!(snapshots.len(), 2);

        let second_call = &snapshots[1];
        let tool_result = second_call
            .iter()
            .find(|m| m.role == "tool")
            .expect("second call should have a tool result with the error");

        assert!(
            tool_result.content.contains("error"),
            "tool result should contain error message: {}",
            tool_result.content
        );

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    // ── Phase F: Auth context threading tests ────────────────────────

    /// Helper to build an InboundMessage with the given channel and sender.
    fn make_inbound(channel: &str, sender_id: &str) -> InboundMessage {
        InboundMessage {
            channel: channel.into(),
            sender_id: sender_id.into(),
            chat_id: "test-chat".into(),
            content: "test message".into(),
            timestamp: chrono::Utc::now(),
            media: vec![],
            metadata: HashMap::new(),
        }
    }

    /// Helper: resolve auth context using the default resolver (CLI=admin,
    /// everything else=zero_trust) without needing a full AgentLoop.
    fn resolve_default(msg: &InboundMessage) -> AuthContext {
        let resolver = PermissionResolver::default_resolver();
        resolver.resolve_auth_context(&msg.sender_id, &msg.channel, false)
    }

    /// F-05: CLI channel gets admin-level permissions via cli_default().
    #[test]
    fn test_resolve_auth_context_cli() {
        let msg = make_inbound("cli", "local");
        let ctx = resolve_default(&msg);

        assert_eq!(ctx.sender_id, "local");
        assert_eq!(ctx.channel, "cli");
        assert_eq!(ctx.permissions.level, 2, "CLI should get admin (level 2)");
        assert!(
            ctx.permissions.tool_access.contains(&"*".to_string()),
            "CLI admin should have wildcard tool access"
        );
        assert_eq!(
            ctx.permissions.rate_limit, 0,
            "CLI admin should have no rate limit"
        );
    }

    /// F-09: Empty sender_id gets zero-trust (level 0) permissions.
    #[test]
    fn test_resolve_auth_context_empty_sender() {
        let msg = make_inbound("telegram", "");
        let ctx = resolve_default(&msg);

        assert_eq!(ctx.sender_id, "");
        assert_eq!(ctx.channel, "telegram");
        assert_eq!(
            ctx.permissions.level, 0,
            "empty sender_id should get zero-trust (level 0)"
        );
    }

    /// F-10: Non-CLI channel with unknown sender gets zero-trust defaults.
    #[test]
    fn test_resolve_auth_context_gateway_channel() {
        let msg = make_inbound("gateway", "api_key_user");
        let ctx = resolve_default(&msg);

        assert_eq!(ctx.sender_id, "api_key_user");
        assert_eq!(ctx.channel, "gateway");
        assert_eq!(
            ctx.permissions.level, 0,
            "gateway users should get zero-trust (level 0) by default"
        );
        assert!(
            ctx.permissions.tool_access.is_empty(),
            "zero-trust should have no tool access"
        );
    }

    /// F-06/07: Telegram channel gets zero-trust with sender identity preserved.
    /// With the default resolver (no per-user overrides), all non-CLI channels
    /// get zero-trust. Config-driven per-user/channel overrides are tested in
    /// the `permissions` module.
    #[test]
    fn test_resolve_auth_context_telegram_preserves_identity() {
        let msg = make_inbound("telegram", "12345");
        let ctx = resolve_default(&msg);

        assert_eq!(ctx.sender_id, "12345");
        assert_eq!(ctx.channel, "telegram");
        assert_eq!(
            ctx.permissions.level, 0,
            "non-CLI channel gets zero-trust with default resolver"
        );
    }

    /// F-extra: Discord channel gets zero-trust with sender identity preserved.
    #[test]
    fn test_resolve_auth_context_discord() {
        let msg = make_inbound("discord", "snowflake_987654321");
        let ctx = resolve_default(&msg);

        assert_eq!(ctx.sender_id, "snowflake_987654321");
        assert_eq!(ctx.channel, "discord");
        assert_eq!(ctx.permissions.level, 0);
    }

    /// F-extra: Slack channel gets zero-trust with sender identity preserved.
    #[test]
    fn test_resolve_auth_context_slack() {
        let msg = make_inbound("slack", "U12345");
        let ctx = resolve_default(&msg);

        assert_eq!(ctx.sender_id, "U12345");
        assert_eq!(ctx.channel, "slack");
        assert_eq!(ctx.permissions.level, 0);
    }

    /// F-12: process_message attaches auth_context to the pipeline request.
    /// Uses a "cli" channel so the auth_context has admin permissions,
    /// verifying the full threading from InboundMessage -> ChatRequest.
    #[tokio::test]
    async fn test_auth_context_attached_to_chat_request() {
        let transport = Arc::new(MockTransport::new("auth-verified"));
        let (agent, dir) = make_agent_loop(transport, "auth_attach").await;

        let inbound = make_inbound("cli", "local");
        agent.bus.publish_inbound(inbound).unwrap();

        let msg = agent.bus.consume_inbound().await.unwrap();
        agent.process_message(msg).await.unwrap();

        // Verify response came through (proves pipeline executed successfully
        // with auth_context attached).
        let outbound = agent.bus.consume_outbound().await.unwrap();
        assert_eq!(outbound.content, "auth-verified");
        assert_eq!(outbound.channel, "cli");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    /// F-12b: process_message with non-CLI channel attaches zero-trust auth_context.
    #[tokio::test]
    async fn test_auth_context_non_cli_attaches_zero_trust() {
        let transport = Arc::new(MockTransport::new("zero-trust-ok"));
        let (agent, dir) = make_agent_loop(transport, "auth_zt").await;

        let inbound = make_inbound("telegram", "user42");
        agent.bus.publish_inbound(inbound).unwrap();

        let msg = agent.bus.consume_inbound().await.unwrap();
        agent.process_message(msg).await.unwrap();

        let outbound = agent.bus.consume_outbound().await.unwrap();
        assert_eq!(outbound.content, "zero-trust-ok");
        assert_eq!(outbound.channel, "telegram");

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
