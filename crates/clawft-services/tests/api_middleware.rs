//! Integration tests for the gateway middleware stack
//! (auth + CORS deny-by-default + per-IP rate limit + CSP).
//!
//! These exercise the assembled router from
//! [`clawft_services::api::build_router`] using a fully stubbed
//! [`ApiState`]. Behavior under test is the WEFT-99/100/101/298 contract.

#![cfg(feature = "api")]

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use clawft_services::api::{
    AgentAccess, AgentInfo, ApiState, BusAccess, ChannelAccess, ChannelStatusInfo, ConfigAccess,
    MemoryAccess, MemoryEntryInfo, SessionAccess, SessionDetail, SessionInfo, SkillAccess,
    SkillInfo, ToolInfo, ToolRegistryAccess, TtsProviderInfo, VoiceAccess, VoiceSettingsInfo,
    VoiceSettingsUpdate, VoiceStatusInfo, auth::TokenStore, broadcaster::TopicBroadcaster,
    build_router,
};
use tower::ServiceExt;

// ─── Stub access impls ──────────────────────────────────────────────────

struct StubTools;
impl ToolRegistryAccess for StubTools {
    fn list_tools(&self) -> Vec<ToolInfo> {
        vec![]
    }
    fn tool_schema(&self, _: &str) -> Option<serde_json::Value> {
        None
    }
}

struct StubSessions;
impl SessionAccess for StubSessions {
    fn list_sessions(&self) -> Vec<SessionInfo> {
        vec![]
    }
    fn get_session(&self, _: &str) -> Option<SessionDetail> {
        None
    }
    fn delete_session(&self, _: &str) -> bool {
        false
    }
}

struct StubAgents;
impl AgentAccess for StubAgents {
    fn list_agents(&self) -> Vec<AgentInfo> {
        vec![]
    }
    fn get_agent(&self, _: &str) -> Option<AgentInfo> {
        None
    }
}

struct StubBus;
impl BusAccess for StubBus {
    fn send_message(&self, _: &str, _: &str, _: &str) {}
}

struct StubSkills;
impl SkillAccess for StubSkills {
    fn list_skills(&self) -> Vec<SkillInfo> {
        vec![]
    }
    fn install_skill(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
    fn uninstall_skill(&self, _: &str) -> Result<(), String> {
        Ok(())
    }
}

struct StubMemory;
impl MemoryAccess for StubMemory {
    fn list_entries(&self) -> Vec<MemoryEntryInfo> {
        vec![]
    }
    fn search(&self, _: &str, _: usize) -> Vec<MemoryEntryInfo> {
        vec![]
    }
    fn store(&self, _: &str, _: &str, _: &str, _: &[String]) -> Result<MemoryEntryInfo, String> {
        Err("stub".into())
    }
    fn delete(&self, _: &str) -> bool {
        false
    }
}

struct StubConfig;
impl ConfigAccess for StubConfig {
    fn get_config(&self) -> serde_json::Value {
        serde_json::json!({})
    }
    fn save_config(&self, _: serde_json::Value) -> Result<(), String> {
        Ok(())
    }
}

struct StubChannels;
impl ChannelAccess for StubChannels {
    fn list_channels(&self) -> Vec<ChannelStatusInfo> {
        vec![]
    }
}

struct StubVoice;
impl VoiceAccess for StubVoice {
    fn get_status(&self) -> VoiceStatusInfo {
        VoiceStatusInfo {
            state: "idle".into(),
            talk_mode_active: false,
            wake_word_enabled: false,
        }
    }
    fn get_settings(&self) -> VoiceSettingsInfo {
        VoiceSettingsInfo {
            enabled: false,
            wake_word_enabled: false,
            language: "en".into(),
            echo_cancel: false,
            noise_suppression: false,
            push_to_talk: false,
        }
    }
    fn update_settings(&self, _: VoiceSettingsUpdate) -> Result<(), String> {
        Ok(())
    }
    fn get_tts_config(&self) -> TtsProviderInfo {
        TtsProviderInfo {
            provider: "browser".into(),
            model: "default".into(),
            voice: "default".into(),
            speed: 1.0,
            api_key: String::new(),
            api_base: None,
        }
    }
}

fn make_state() -> (ApiState, Arc<TokenStore>) {
    let auth = Arc::new(TokenStore::new());
    let state = ApiState {
        tools: Arc::new(StubTools),
        sessions: Arc::new(StubSessions),
        agents: Arc::new(StubAgents),
        bus: Arc::new(StubBus),
        auth: auth.clone(),
        skills: Arc::new(StubSkills),
        memory: Arc::new(StubMemory),
        config: Arc::new(StubConfig),
        channels: Arc::new(StubChannels),
        voice: Arc::new(StubVoice),
        broadcaster: Arc::new(TopicBroadcaster::new()),
        mcp: None,
    };
    (state, auth)
}

// ─── Tests ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn auth_middleware_rejects_no_bearer() {
    let (state, _auth) = make_state();
    let app = build_router(state, &[], None);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        resp.headers().get(header::WWW_AUTHENTICATE).unwrap(),
        "Bearer"
    );
}

#[tokio::test]
async fn auth_middleware_accepts_valid_token() {
    let (state, auth) = make_state();
    let token = auth.generate_token(3600).unwrap();
    let app = build_router(state, &[], None);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/agents")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn auth_middleware_allows_health_without_token() {
    let (state, _auth) = make_state();
    let app = build_router(state, &[], None);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn auth_middleware_allows_token_endpoint_without_token() {
    let (state, _auth) = make_state();
    let app = build_router(state, &[], None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

/// WEFT-570: `POST /api/auth/revoke` must require a valid Bearer
/// (it is NOT in the public-paths allowlist) and, on success, the
/// token cannot be reused for any subsequent protected request.
#[tokio::test]
async fn auth_revoke_invalidates_bearer() {
    let (state, auth) = make_state();
    let token = auth.generate_token(3600).unwrap();
    let app = build_router(state, &[], None);

    // Sanity: token works against a protected route.
    let probe = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/agents")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(probe.status(), StatusCode::OK);

    // Revoke must succeed (204 No Content) and require the bearer.
    let revoke = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/revoke")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoke.status(), StatusCode::NO_CONTENT);

    // Subsequent request with the same bearer is now 401.
    let after = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/agents")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(after.status(), StatusCode::UNAUTHORIZED);
}

/// WEFT-570: revoke without a Bearer must be rejected by the auth
/// middleware itself (401), not silently 204'd. The endpoint is NOT
/// public — the caller must already prove they hold the token they're
/// asking us to revoke.
#[tokio::test]
async fn auth_revoke_rejects_anonymous_caller() {
    let (state, _auth) = make_state();
    let app = build_router(state, &[], None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/auth/revoke")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn cors_denies_unconfigured_origin() {
    let (state, _auth) = make_state();
    let app = build_router(state, &[], None);

    // Preflight from a non-localhost origin.
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/health")
                .header(header::ORIGIN, "https://evil.example.com")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // CORS layer should refuse to add Access-Control-Allow-Origin
    // for an unallowed origin.
    assert!(
        resp.headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none()
    );
}

#[tokio::test]
async fn cors_allows_localhost_default() {
    let (state, _auth) = make_state();
    let app = build_router(state, &[], None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/health")
                .header(header::ORIGIN, "http://localhost:5173")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let allow = resp
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .expect("Access-Control-Allow-Origin header should be present");
    assert_eq!(allow, "http://localhost:5173");
}

#[tokio::test]
async fn rate_limit_429_after_general_quota() {
    let (state, _auth) = make_state();
    let app = build_router(state, &[], None);

    // /api/health is exempt from rate limiting (k8s probes, etc.) so use
    // /api/agents which is rate-limited but cheap. We need a valid token
    // to pass the auth gate first.
    let token = _auth.generate_token(3600).unwrap();

    // 60 requests should pass…
    for i in 0..60 {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "request {i} unexpectedly throttled"
        );
    }
    // …and the 61st should be throttled.
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/agents")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn csp_header_present_on_health() {
    let (state, _auth) = make_state();
    let app = build_router(state, &[], None);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let csp = resp
        .headers()
        .get("content-security-policy")
        .expect("CSP header missing on /api/health");
    let v = csp.to_str().unwrap();
    assert!(v.contains("default-src 'self'"));
    assert!(v.contains("frame-ancestors 'none'"));
}

#[tokio::test]
async fn csp_header_present_on_unauthorized() {
    let (state, _auth) = make_state();
    let app = build_router(state, &[], None);

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/agents")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    assert!(
        resp.headers().get("content-security-policy").is_some(),
        "CSP header must accompany 401 responses too"
    );
}

// ─── /mcp endpoint (MCP-over-HTTP, WEFT: grok-voice integration) ────────

mod mcp_endpoint {
    use super::*;
    use async_trait::async_trait;
    use clawft_services::api::mcp_http::McpEndpoint;
    use clawft_services::mcp::ToolDefinition;
    use clawft_services::mcp::composite::CompositeToolProvider;
    use clawft_services::mcp::provider::{CallToolResult, ToolError, ToolProvider};
    use http_body_util::BodyExt;
    use serde_json::{Value, json};

    struct EchoProvider;

    #[async_trait]
    impl ToolProvider for EchoProvider {
        fn namespace(&self) -> &str {
            "echo"
        }
        fn list_tools(&self) -> Vec<ToolDefinition> {
            vec![ToolDefinition {
                name: "say".into(),
                description: "Echoes text".into(),
                input_schema: json!({
                    "type": "object",
                    "properties": { "text": { "type": "string" } }
                }),
            }]
        }
        async fn call_tool(&self, name: &str, args: Value) -> Result<CallToolResult, ToolError> {
            match name {
                "say" => Ok(CallToolResult::text(
                    args.get("text").and_then(|v| v.as_str()).unwrap_or(""),
                )),
                other => Err(ToolError::NotFound(other.into())),
            }
        }
    }

    const TOKEN: &str = "test-mcp-token";

    fn mcp_state() -> ApiState {
        let (mut state, _) = make_state();
        let mut provider = CompositeToolProvider::new();
        provider.register(Box::new(EchoProvider));
        let endpoint =
            McpEndpoint::new(provider, Vec::new(), TOKEN.into()).expect("non-empty token");
        state.mcp = Some(Arc::new(endpoint));
        state
    }

    fn mcp_request(auth: Option<&str>, body: Value) -> Request<Body> {
        let mut builder = Request::builder()
            .method(Method::POST)
            .uri("/mcp")
            .header(header::CONTENT_TYPE, "application/json");
        if let Some(token) = auth {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        builder.body(Body::from(body.to_string())).unwrap()
    }

    async fn body_json(resp: axum::response::Response) -> Value {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[test]
    fn empty_token_is_rejected_at_construction() {
        let result = McpEndpoint::new(CompositeToolProvider::new(), Vec::new(), "  ".into());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn disabled_endpoint_returns_404() {
        let (state, _) = make_state(); // state.mcp == None
        let app = build_router(state, &[], None);
        let resp = app
            .oneshot(mcp_request(Some(TOKEN), json!({"jsonrpc":"2.0","id":1,"method":"tools/list"})))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn missing_bearer_is_401() {
        let app = build_router(mcp_state(), &[], None);
        let resp = app
            .oneshot(mcp_request(None, json!({"jsonrpc":"2.0","id":1,"method":"tools/list"})))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn wrong_bearer_is_401() {
        let app = build_router(mcp_state(), &[], None);
        let resp = app
            .oneshot(mcp_request(
                Some("wrong"),
                json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn initialize_answers_with_server_info() {
        let app = build_router(mcp_state(), &[], None);
        let resp = app
            .oneshot(mcp_request(
                Some(TOKEN),
                json!({
                    "jsonrpc": "2.0", "id": 1, "method": "initialize",
                    "params": {
                        "protocolVersion": "2025-06-18",
                        "capabilities": {},
                        "clientInfo": { "name": "grok", "version": "1.0" }
                    }
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["id"], 1);
        assert_eq!(body["result"]["serverInfo"]["name"], "clawft");
    }

    #[tokio::test]
    async fn tools_list_works_without_prior_initialize() {
        // Stateless Streamable HTTP: each POST may be a fresh connection,
        // so tools/* must not require an initialize on the same socket.
        let app = build_router(mcp_state(), &[], None);
        let resp = app
            .oneshot(mcp_request(
                Some(TOKEN),
                json!({"jsonrpc":"2.0","id":7,"method":"tools/list"}),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let tools = body["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "echo__say");
    }

    #[tokio::test]
    async fn tools_call_round_trips() {
        let app = build_router(mcp_state(), &[], None);
        let resp = app
            .oneshot(mcp_request(
                Some(TOKEN),
                json!({
                    "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                    "params": { "name": "echo__say", "arguments": { "text": "hello grok" } }
                }),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        assert_eq!(body["result"]["content"][0]["text"], "hello grok");
    }

    #[tokio::test]
    async fn notification_returns_202_no_body() {
        let app = build_router(mcp_state(), &[], None);
        let resp = app
            .oneshot(mcp_request(
                Some(TOKEN),
                json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn batch_returns_array_of_responses() {
        let app = build_router(mcp_state(), &[], None);
        let resp = app
            .oneshot(mcp_request(
                Some(TOKEN),
                json!([
                    {"jsonrpc":"2.0","id":1,"method":"tools/list"},
                    {"jsonrpc":"2.0","method":"notifications/initialized"},
                    {"jsonrpc":"2.0","id":2,"method":"tools/call",
                     "params": {"name": "echo__say", "arguments": {"text": "b"}}}
                ]),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_json(resp).await;
        let arr = body.as_array().unwrap();
        assert_eq!(arr.len(), 2); // notification produces no response
    }

    #[tokio::test]
    async fn get_method_is_405() {
        let app = build_router(mcp_state(), &[], None);
        let resp = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/mcp")
                    .header(header::AUTHORIZATION, format!("Bearer {TOKEN}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
