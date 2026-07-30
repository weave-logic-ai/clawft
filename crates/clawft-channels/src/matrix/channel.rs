//! Matrix channel adapter implementation.
//!
//! Real Client-Server API runtime behind the `matrix` feature flag:
//!
//! - `start()` long-polls `GET /_matrix/client/v3/sync` with a durable
//!   `since` token, auto-joins rooms from config (and optionally accepts
//!   invites), parses `m.room.message` events into
//!   [`MessagePayload`], and delivers them via the host.
//! - `send()` issues
//!   `PUT /_matrix/client/v3/rooms/{room}/send/m.room.message/{txn}`
//!   with a UUID transaction id and returns the real `event_id`.
//! - Transport errors use exponential backoff; cancellation is respected
//!   on every wait.
//!
//! Implements [`ChannelAdapter`] for Matrix messaging via the
//! client-server API. Supports threaded rooms and text payloads.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use clawft_plugin::error::PluginError;
use clawft_plugin::message::MessagePayload;
use clawft_plugin::traits::{ChannelAdapter, ChannelAdapterHost};

use super::client::{ClientEvent, MatrixClient, SyncResponse};
use super::types::MatrixAdapterConfig;

const SINCE_TOKEN_FILENAME: &str = "since_token";
const INITIAL_BACKOFF: Duration = Duration::from_millis(500);
const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// Matrix channel adapter using the client-server API.
///
/// Connects to a Matrix homeserver for sending and receiving messages.
/// Supports threaded conversations via Matrix room threads.
pub struct MatrixChannelAdapter {
    config: MatrixAdapterConfig,
    client: MatrixClient,
}

impl MatrixChannelAdapter {
    /// Create a new Matrix channel adapter.
    pub fn new(config: MatrixAdapterConfig) -> Self {
        let token = config.access_token.expose().to_string();
        // Local HTTP timeout must exceed the long-poll timeout.
        let http_timeout =
            Duration::from_millis(config.sync_timeout_ms.saturating_add(15_000).max(5_000));
        let client = MatrixClient::new(&config.homeserver_url, token, http_timeout);
        Self { config, client }
    }

    /// Check if a user ID is in the allow list.
    pub fn is_user_allowed(&self, user_id: &str) -> bool {
        if self.config.allowed_users.is_empty() {
            return true;
        }
        self.config.allowed_users.iter().any(|u| u == user_id)
    }

    fn since_token_path(&self) -> PathBuf {
        self.config.resolved_state_dir().join(SINCE_TOKEN_FILENAME)
    }

    /// Load the durable `since` token from disk, if present.
    pub fn load_since_token(&self) -> Option<String> {
        load_since_token(&self.since_token_path())
    }

    /// Persist the `since` token so the next process can resume.
    pub fn save_since_token(&self, token: &str) -> Result<(), PluginError> {
        save_since_token(&self.since_token_path(), token)
    }

    /// Auto-join every room listed in `auto_join_rooms`. Failures are
    /// logged but do not abort startup (the room may already be joined).
    async fn auto_join_configured_rooms(&self) {
        for room in &self.config.auto_join_rooms {
            match self.client.join_room(room).await {
                Ok(canonical) => {
                    info!(room = %room, canonical = %canonical, "matrix: auto-joined room");
                }
                Err(e) => {
                    // Already-joined returns M_FORBIDDEN / similar on some
                    // servers; log and continue so one bad room does not
                    // block the adapter.
                    warn!(room = %room, error = %e, "matrix: auto-join failed");
                }
            }
        }
    }

    /// Accept invites discovered in a `/sync` response when configured.
    async fn maybe_accept_invites(&self, sync: &SyncResponse) {
        if !self.config.auto_accept_invites {
            return;
        }
        for room_id in sync.rooms.invite.keys() {
            match self.client.join_room(room_id).await {
                Ok(_) => info!(room = %room_id, "matrix: accepted invite"),
                Err(e) => warn!(room = %room_id, error = %e, "matrix: invite accept failed"),
            }
        }
    }

    /// Deliver all text `m.room.message` events from a sync response.
    async fn deliver_sync_events(
        &self,
        sync: &SyncResponse,
        host: &Arc<dyn ChannelAdapterHost>,
    ) -> Result<(), PluginError> {
        for (room_id, joined) in &sync.rooms.join {
            for event in &joined.timeline.events {
                if let Err(e) = self.deliver_event(room_id, event, host).await {
                    error!(
                        room = %room_id,
                        event_id = ?event.event_id,
                        error = %e,
                        "matrix: deliver failed"
                    );
                }
            }
        }
        Ok(())
    }

    async fn deliver_event(
        &self,
        room_id: &str,
        event: &ClientEvent,
        host: &Arc<dyn ChannelAdapterHost>,
    ) -> Result<(), PluginError> {
        if event.event_type != "m.room.message" {
            return Ok(());
        }

        // Skip our own messages to avoid echo loops.
        if !self.config.user_id.is_empty() && event.sender == self.config.user_id {
            debug!(
                event_id = ?event.event_id,
                "matrix: skipping own message"
            );
            return Ok(());
        }

        let msgtype = event.content.msgtype.as_deref().unwrap_or("");
        // Accept plain text (and notice/emote as text for agents).
        if !matches!(msgtype, "m.text" | "m.notice" | "m.emote" | "") {
            debug!(%msgtype, "matrix: skipping non-text msgtype");
            return Ok(());
        }

        let Some(ref body) = event.content.body else {
            debug!(event_id = ?event.event_id, "matrix: message without body");
            return Ok(());
        };

        if !self.is_user_allowed(&event.sender) {
            warn!(
                sender = %event.sender,
                room = %room_id,
                "matrix: message from disallowed user, ignoring"
            );
            return Ok(());
        }

        let mut metadata: HashMap<String, serde_json::Value> = HashMap::new();
        if let Some(ref eid) = event.event_id {
            metadata.insert("event_id".into(), serde_json::json!(eid));
        }
        metadata.insert("msgtype".into(), serde_json::json!(msgtype));
        metadata.insert("room_id".into(), serde_json::json!(room_id));
        if !self.config.allowed_users.is_empty()
            && self
                .config
                .allowed_users
                .iter()
                .any(|u| u == &event.sender)
        {
            metadata.insert("allow_from_match".into(), serde_json::Value::Bool(true));
        }

        host.deliver_inbound(
            "matrix",
            &event.sender,
            room_id,
            MessagePayload::text(body.clone()),
            metadata,
        )
        .await
    }
}

// ---------------------------------------------------------------------------
// Since-token persistence
// ---------------------------------------------------------------------------

fn load_since_token(path: &Path) -> Option<String> {
    match std::fs::read_to_string(path) {
        Ok(s) => {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            warn!(path = %path.display(), error = %e, "matrix: failed to read since token");
            None
        }
    }
}

fn save_since_token(path: &Path, token: &str) -> Result<(), PluginError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            PluginError::ExecutionFailed(format!(
                "matrix: create state dir {}: {e}",
                parent.display()
            ))
        })?;
    }
    // Write atomically via temp + rename when possible.
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, token.as_bytes()).map_err(|e| {
        PluginError::ExecutionFailed(format!("matrix: write since token {}: {e}", tmp.display()))
    })?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        // Fall back to direct write if rename fails (e.g. cross-device).
        std::fs::write(path, token.as_bytes()).map_err(|e2| {
            PluginError::ExecutionFailed(format!(
                "matrix: persist since token {}: rename={e}, write={e2}",
                path.display()
            ))
        })?;
        let _ = std::fs::remove_file(&tmp);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// ChannelAdapter
// ---------------------------------------------------------------------------

#[async_trait]
impl ChannelAdapter for MatrixChannelAdapter {
    fn name(&self) -> &str {
        "matrix"
    }

    fn display_name(&self) -> &str {
        "Matrix"
    }

    fn supports_threads(&self) -> bool {
        true
    }

    fn supports_media(&self) -> bool {
        true
    }

    async fn start(
        &self,
        host: Arc<dyn ChannelAdapterHost>,
        cancel: CancellationToken,
    ) -> Result<(), PluginError> {
        info!(
            homeserver = %self.config.homeserver_url,
            user = %self.config.user_id,
            rooms = self.config.auto_join_rooms.len(),
            "matrix channel adapter starting"
        );

        if self.config.homeserver_url.is_empty() {
            return Err(PluginError::LoadFailed(
                "matrix adapter: homeserver_url is required".into(),
            ));
        }
        if self.config.access_token.is_empty() {
            return Err(PluginError::LoadFailed(
                "matrix adapter: access_token is required".into(),
            ));
        }
        if self.config.user_id.is_empty() {
            return Err(PluginError::LoadFailed(
                "matrix adapter: user_id is required".into(),
            ));
        }

        // Best-effort auto-join before entering the long-poll loop.
        self.auto_join_configured_rooms().await;

        let mut since = self.load_since_token();
        if let Some(ref token) = since {
            info!(
                token_prefix = %token.chars().take(12).collect::<String>(),
                "matrix: resumed since token from disk"
            );
        }

        let timeout_ms = self.config.sync_timeout_ms;
        let mut backoff = INITIAL_BACKOFF;

        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("matrix channel adapter shutting down");
                    return Ok(());
                }
                result = self.client.sync(since.as_deref(), timeout_ms) => {
                    match result {
                        Ok(sync) => {
                            backoff = INITIAL_BACKOFF;

                            self.maybe_accept_invites(&sync).await;

                            if let Err(e) = self.deliver_sync_events(&sync, &host).await {
                                error!(error = %e, "matrix: batch deliver error");
                            }

                            // Persist next_batch for resume across restarts.
                            if !sync.next_batch.is_empty() {
                                if let Err(e) = self.save_since_token(&sync.next_batch) {
                                    warn!(error = %e, "matrix: failed to persist since token");
                                }
                                since = Some(sync.next_batch);
                            }
                        }
                        Err(e) => {
                            error!(error = %e, backoff_ms = backoff.as_millis(), "matrix: /sync failed");
                            tokio::select! {
                                _ = cancel.cancelled() => {
                                    info!("matrix channel adapter shutting down");
                                    return Ok(());
                                }
                                _ = tokio::time::sleep(backoff) => {}
                            }
                            backoff = (backoff * 2).min(MAX_BACKOFF);
                        }
                    }
                }
            }
        }
    }

    async fn send(&self, target: &str, payload: &MessagePayload) -> Result<String, PluginError> {
        let content = payload.as_text().ok_or_else(|| {
            PluginError::ExecutionFailed("matrix: only text payloads supported currently".into())
        })?;

        if self.config.homeserver_url.is_empty() {
            return Err(PluginError::ExecutionFailed(
                "matrix: homeserver_url not configured".into(),
            ));
        }
        if target.is_empty() {
            return Err(PluginError::ExecutionFailed(
                "matrix: send target (room id) is empty".into(),
            ));
        }
        if self.config.access_token.is_empty() {
            return Err(PluginError::ExecutionFailed(
                "matrix: access_token not configured".into(),
            ));
        }

        let txn_id = uuid::Uuid::new_v4().simple().to_string();
        self.client.send_text(target, content, &txn_id).await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use serde_json::json;
    use tokio::sync::Mutex;
    use wiremock::matchers::{header, method, path, path_regex, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_config(homeserver: &str) -> MatrixAdapterConfig {
        MatrixAdapterConfig {
            homeserver_url: homeserver.into(),
            access_token: "syt_test_token".into(),
            user_id: "@bot:matrix.test.org".into(),
            sync_timeout_ms: 0,
            ..Default::default()
        }
    }

    struct MockHost {
        messages: Mutex<Vec<(String, String, String, MessagePayload)>>,
    }

    impl MockHost {
        fn new() -> Self {
            Self {
                messages: Mutex::new(vec![]),
            }
        }
    }

    #[async_trait]
    impl ChannelAdapterHost for MockHost {
        async fn deliver_inbound(
            &self,
            channel: &str,
            sender_id: &str,
            chat_id: &str,
            payload: MessagePayload,
            _metadata: HashMap<String, serde_json::Value>,
        ) -> Result<(), PluginError> {
            self.messages.lock().await.push((
                channel.into(),
                sender_id.into(),
                chat_id.into(),
                payload,
            ));
            Ok(())
        }
    }

    // -- unit: metadata / filtering --

    #[test]
    fn name_is_matrix() {
        let adapter = MatrixChannelAdapter::new(make_config("https://matrix.test.org"));
        assert_eq!(adapter.name(), "matrix");
    }

    #[test]
    fn display_name() {
        let adapter = MatrixChannelAdapter::new(make_config("https://matrix.test.org"));
        assert_eq!(adapter.display_name(), "Matrix");
    }

    #[test]
    fn supports_threads_and_media() {
        let adapter = MatrixChannelAdapter::new(make_config("https://matrix.test.org"));
        assert!(adapter.supports_threads());
        assert!(adapter.supports_media());
    }

    #[test]
    fn user_filtering() {
        let mut config = make_config("https://matrix.test.org");
        config.allowed_users = vec!["@admin:matrix.org".into()];
        let adapter = MatrixChannelAdapter::new(config);

        assert!(adapter.is_user_allowed("@admin:matrix.org"));
        assert!(!adapter.is_user_allowed("@stranger:matrix.org"));
    }

    #[test]
    fn empty_allow_list_allows_all() {
        let adapter = MatrixChannelAdapter::new(make_config("https://matrix.test.org"));
        assert!(adapter.is_user_allowed("@anyone:anywhere.org"));
    }

    #[test]
    fn since_token_roundtrip_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_config("https://matrix.test.org");
        config.state_dir = Some(dir.path().to_path_buf());
        let adapter = MatrixChannelAdapter::new(config);

        assert!(adapter.load_since_token().is_none());
        adapter.save_since_token("s123_NEXT").unwrap();
        assert_eq!(adapter.load_since_token().as_deref(), Some("s123_NEXT"));
    }

    // -- validation --

    #[tokio::test]
    async fn start_validates_homeserver_url() {
        let mut config = make_config("https://matrix.test.org");
        config.homeserver_url = String::new();
        let adapter = MatrixChannelAdapter::new(config);

        let host = Arc::new(MockHost::new());
        let cancel = CancellationToken::new();
        let result = adapter.start(host, cancel).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("homeserver_url"));
    }

    #[tokio::test]
    async fn start_validates_access_token() {
        let mut config = make_config("https://matrix.test.org");
        config.access_token = "".into();
        let adapter = MatrixChannelAdapter::new(config);

        let host = Arc::new(MockHost::new());
        let cancel = CancellationToken::new();
        let result = adapter.start(host, cancel).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("access_token"));
    }

    #[tokio::test]
    async fn start_validates_user_id() {
        let mut config = make_config("https://matrix.test.org");
        config.user_id = String::new();
        let adapter = MatrixChannelAdapter::new(config);

        let host = Arc::new(MockHost::new());
        let cancel = CancellationToken::new();
        let result = adapter.start(host, cancel).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("user_id"));
    }

    #[tokio::test]
    async fn send_non_text_fails() {
        let adapter = MatrixChannelAdapter::new(make_config("https://matrix.test.org"));
        let payload = MessagePayload::structured(serde_json::json!({}));
        let result = adapter.send("!room1:matrix.test.org", &payload).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn send_empty_target_fails() {
        let adapter = MatrixChannelAdapter::new(make_config("https://matrix.test.org"));
        let result = adapter.send("", &MessagePayload::text("hi")).await;
        assert!(result.is_err());
    }

    // -- wiremock: send --

    #[tokio::test]
    async fn send_puts_m_room_message_and_returns_event_id() {
        let server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path_regex(
                r"^/_matrix/client/v3/rooms/.*/send/m\.room\.message/.*",
            ))
            .and(header("authorization", "Bearer syt_test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "event_id": "$real_event_abc"
            })))
            .mount(&server)
            .await;

        let adapter = MatrixChannelAdapter::new(make_config(&server.uri()));
        let id = adapter
            .send("!room1:matrix.test.org", &MessagePayload::text("Hello Matrix"))
            .await
            .unwrap();
        assert_eq!(id, "$real_event_abc");
        // Must not be a fabricated `$timestamp` from the old stub.
        assert!(!id
            .trim_start_matches('$')
            .chars()
            .all(|c| c.is_ascii_digit()));
    }

    #[tokio::test]
    async fn send_propagates_http_error() {
        let server = MockServer::start().await;
        Mock::given(method("PUT"))
            .and(path_regex(r"^/_matrix/client/v3/rooms/.*"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .mount(&server)
            .await;

        let adapter = MatrixChannelAdapter::new(make_config(&server.uri()));
        let err = adapter
            .send("!room1:matrix.test.org", &MessagePayload::text("x"))
            .await
            .unwrap_err()
            .to_string();
        assert!(err.contains("403"), "expected 403 in error: {err}");
    }

    // -- wiremock: join --

    #[tokio::test]
    async fn auto_join_posts_to_join_endpoint() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path_regex(r"^/_matrix/client/v3/join/.*"))
            .and(header("authorization", "Bearer syt_test_token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "room_id": "!canonical:matrix.test.org"
            })))
            .expect(1)
            .mount(&server)
            .await;

        // /sync returns empty and is cancelled after first successful cycle.
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "next_batch": "s1",
                "rooms": { "join": {}, "invite": {} }
            })))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let mut config = make_config(&server.uri());
        config.auto_join_rooms = vec!["!room1:matrix.test.org".into()];
        config.state_dir = Some(dir.path().to_path_buf());
        let adapter = MatrixChannelAdapter::new(config);

        let host = Arc::new(MockHost::new());
        let cancel = CancellationToken::new();
        let cancel_c = cancel.clone();
        let handle = tokio::spawn(async move { adapter.start(host, cancel_c).await });

        // Give the join + one sync cycle time to complete.
        tokio::time::sleep(Duration::from_millis(150)).await;
        cancel.cancel();
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "start failed: {result:?}");
    }

    // -- wiremock: sync + deliver --

    #[tokio::test]
    async fn sync_loop_delivers_m_room_message_and_persists_since() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();

        // First /sync returns timeline events once; subsequent polls are empty
        // (mirrors a real homeserver advancing past `next_batch`).
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .and(query_param("timeout", "0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "next_batch": "s999_persisted",
                "rooms": {
                    "join": {
                        "!room1:matrix.test.org": {
                            "timeline": {
                                "events": [
                                    {
                                        "type": "m.room.message",
                                        "sender": "@alice:matrix.test.org",
                                        "event_id": "$evt1",
                                        "content": {
                                            "msgtype": "m.text",
                                            "body": "Hello bot"
                                        }
                                    },
                                    {
                                        "type": "m.room.message",
                                        "sender": "@bot:matrix.test.org",
                                        "event_id": "$own",
                                        "content": {
                                            "msgtype": "m.text",
                                            "body": "should be skipped"
                                        }
                                    }
                                ]
                            }
                        }
                    },
                    "invite": {}
                }
            })))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "next_batch": "s999_persisted",
                "rooms": { "join": {}, "invite": {} }
            })))
            .mount(&server)
            .await;

        let mut config = make_config(&server.uri());
        config.state_dir = Some(dir.path().to_path_buf());
        let adapter = MatrixChannelAdapter::new(config);

        let host = Arc::new(MockHost::new());
        let cancel = CancellationToken::new();
        let cancel_c = cancel.clone();
        let host_c = Arc::clone(&host);
        let handle = tokio::spawn(async move { adapter.start(host_c, cancel_c).await });

        // Wait until a message is delivered.
        for _ in 0..50 {
            if !host.messages.lock().await.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        // One more tick so the since token is written.
        tokio::time::sleep(Duration::from_millis(30)).await;
        cancel.cancel();
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "start failed: {result:?}");

        let msgs = host.messages.lock().await;
        assert_eq!(msgs.len(), 1, "expected only alice's message, got {msgs:?}");
        assert_eq!(msgs[0].0, "matrix");
        assert_eq!(msgs[0].1, "@alice:matrix.test.org");
        assert_eq!(msgs[0].2, "!room1:matrix.test.org");
        assert_eq!(msgs[0].3.as_text(), Some("Hello bot"));

        // Since token persisted.
        let token_path = dir.path().join(SINCE_TOKEN_FILENAME);
        let persisted = std::fs::read_to_string(&token_path).unwrap();
        assert_eq!(persisted.trim(), "s999_persisted");
    }

    #[tokio::test]
    async fn sync_loop_respects_cancellation_during_backoff() {
        let server = MockServer::start().await;

        // Always fail so the loop enters backoff sleep.
        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .respond_with(ResponseTemplate::new(500).set_body_string("boom"))
            .mount(&server)
            .await;

        let dir = tempfile::tempdir().unwrap();
        let mut config = make_config(&server.uri());
        config.state_dir = Some(dir.path().to_path_buf());
        let adapter = MatrixChannelAdapter::new(config);

        let host = Arc::new(MockHost::new());
        let cancel = CancellationToken::new();
        let cancel_c = cancel.clone();
        let handle = tokio::spawn(async move { adapter.start(host, cancel_c).await });

        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel.cancel();
        let result = tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("start did not exit on cancel")
            .unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn auto_accept_invite_joins_room() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "next_batch": "s2",
                "rooms": {
                    "join": {},
                    "invite": {
                        "!invited:matrix.test.org": {
                            "invite_state": { "events": [] }
                        }
                    }
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path_regex(r"^/_matrix/client/v3/join/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "room_id": "!invited:matrix.test.org"
            })))
            .expect(1..)
            .mount(&server)
            .await;

        let mut config = make_config(&server.uri());
        config.auto_accept_invites = true;
        config.state_dir = Some(dir.path().to_path_buf());
        let adapter = MatrixChannelAdapter::new(config);

        let host = Arc::new(MockHost::new());
        let cancel = CancellationToken::new();
        let cancel_c = cancel.clone();
        let handle = tokio::spawn(async move { adapter.start(host, cancel_c).await });

        tokio::time::sleep(Duration::from_millis(200)).await;
        cancel.cancel();
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn start_shuts_down_on_cancel_with_empty_sync() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({
                        "next_batch": "s0",
                        "rooms": { "join": {}, "invite": {} }
                    }))
                    // Delay so we can cancel while in-flight or between polls.
                    .set_delay(Duration::from_millis(30)),
            )
            .mount(&server)
            .await;

        let mut config = make_config(&server.uri());
        config.state_dir = Some(dir.path().to_path_buf());
        let adapter = MatrixChannelAdapter::new(config);

        let host = Arc::new(MockHost::new());
        let cancel = CancellationToken::new();
        let cancel_c = cancel.clone();
        let handle = tokio::spawn(async move { adapter.start(host, cancel_c).await });

        tokio::time::sleep(Duration::from_millis(20)).await;
        cancel.cancel();
        let result = tokio::time::timeout(Duration::from_secs(2), handle)
            .await
            .expect("did not shut down")
            .unwrap();
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn disallowed_user_messages_are_dropped() {
        let server = MockServer::start().await;
        let dir = tempfile::tempdir().unwrap();

        Mock::given(method("GET"))
            .and(path("/_matrix/client/v3/sync"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "next_batch": "s3",
                "rooms": {
                    "join": {
                        "!r:s": {
                            "timeline": {
                                "events": [{
                                    "type": "m.room.message",
                                    "sender": "@stranger:matrix.test.org",
                                    "event_id": "$x",
                                    "content": { "msgtype": "m.text", "body": "nope" }
                                }]
                            }
                        }
                    },
                    "invite": {}
                }
            })))
            .mount(&server)
            .await;

        let mut config = make_config(&server.uri());
        config.allowed_users = vec!["@admin:matrix.test.org".into()];
        config.state_dir = Some(dir.path().to_path_buf());
        let adapter = MatrixChannelAdapter::new(config);

        let host = Arc::new(MockHost::new());
        let cancel = CancellationToken::new();
        let cancel_c = cancel.clone();
        let host_c = Arc::clone(&host);
        let handle = tokio::spawn(async move { adapter.start(host_c, cancel_c).await });

        tokio::time::sleep(Duration::from_millis(150)).await;
        cancel.cancel();
        let _ = handle.await;

        assert!(
            host.messages.lock().await.is_empty(),
            "disallowed user should not deliver"
        );
    }
}
