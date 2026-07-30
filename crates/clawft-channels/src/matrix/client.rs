//! HTTP client for the Matrix Client-Server API (v3).
//!
//! Thin `reqwest` wrapper for the subset used by the channel adapter:
//! - `GET /_matrix/client/v3/sync` (long-poll)
//! - `POST /_matrix/client/v3/join/{roomIdOrAlias}`
//! - `PUT /_matrix/client/v3/rooms/{roomId}/send/m.room.message/{txnId}`

use std::collections::HashMap;
use std::time::Duration;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};

use clawft_plugin::error::PluginError;

// ---------------------------------------------------------------------------
// Request / response DTOs
// ---------------------------------------------------------------------------

/// Body for `PUT .../send/m.room.message/{txnId}`.
#[derive(Debug, Serialize)]
pub struct SendMessageBody<'a> {
    pub msgtype: &'a str,
    pub body: &'a str,
}

/// Response from a successful room send.
#[derive(Debug, Deserialize)]
pub struct SendMessageResponse {
    pub event_id: String,
}

/// Minimal `/sync` response. Only the fields the adapter needs are
/// decoded; unknown keys are ignored.
#[derive(Debug, Default, Deserialize)]
pub struct SyncResponse {
    pub next_batch: String,
    #[serde(default)]
    pub rooms: SyncRooms,
}

#[derive(Debug, Default, Deserialize)]
pub struct SyncRooms {
    #[serde(default)]
    pub join: HashMap<String, JoinedRoom>,
    #[serde(default)]
    pub invite: HashMap<String, InvitedRoom>,
}

#[derive(Debug, Default, Deserialize)]
pub struct JoinedRoom {
    #[serde(default)]
    pub timeline: Timeline,
}

#[derive(Debug, Default, Deserialize)]
pub struct Timeline {
    #[serde(default)]
    pub events: Vec<ClientEvent>,
}

#[derive(Debug, Default, Deserialize)]
pub struct InvitedRoom {
    /// Present on some servers; we only need the room id key from the map.
    #[serde(default)]
    pub invite_state: Option<serde_json::Value>,
}

/// A client event from a room timeline.
#[derive(Debug, Clone, Deserialize)]
pub struct ClientEvent {
    #[serde(rename = "type", default)]
    pub event_type: String,
    #[serde(default)]
    pub sender: String,
    #[serde(default)]
    pub event_id: Option<String>,
    #[serde(default)]
    pub content: EventContent,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EventContent {
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub msgtype: Option<String>,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Matrix Client-Server HTTP client.
pub struct MatrixClient {
    http: Client,
    homeserver_url: String,
    access_token: String,
}

impl MatrixClient {
    /// Build a client. `http_timeout` should exceed the `/sync` long-poll
    /// timeout so the request is not aborted early by the local client.
    pub fn new(
        homeserver_url: impl Into<String>,
        access_token: impl Into<String>,
        http_timeout: Duration,
    ) -> Self {
        let http = Client::builder()
            .timeout(http_timeout)
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            http,
            homeserver_url: homeserver_url.into().trim_end_matches('/').to_string(),
            access_token: access_token.into(),
        }
    }

    /// Homeserver base URL (no trailing slash).
    pub fn homeserver_url(&self) -> &str {
        &self.homeserver_url
    }

    /// `GET /_matrix/client/v3/sync` with optional `since` token and
    /// long-poll `timeout` in milliseconds.
    pub async fn sync(
        &self,
        since: Option<&str>,
        timeout_ms: u64,
    ) -> Result<SyncResponse, PluginError> {
        let mut url = format!(
            "{}/_matrix/client/v3/sync?timeout={}",
            self.homeserver_url, timeout_ms
        );
        if let Some(token) = since {
            if !token.is_empty() {
                url.push_str("&since=");
                url.push_str(&encode_query_value(token));
            }
        }

        trace!(%url, "matrix: GET /sync");

        let resp = self
            .http
            .get(&url)
            .bearer_auth(&self.access_token)
            .send()
            .await
            .map_err(|e| {
                PluginError::ExecutionFailed(format!("matrix: /sync request failed: {e}"))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(PluginError::ExecutionFailed(format!(
                "matrix: /sync HTTP {status}: {text}"
            )));
        }

        let body: SyncResponse = resp.json().await.map_err(|e| {
            PluginError::ExecutionFailed(format!("matrix: /sync decode failed: {e}"))
        })?;

        debug!(
            next_batch = %body.next_batch,
            joined = body.rooms.join.len(),
            invited = body.rooms.invite.len(),
            "matrix: /sync ok"
        );
        Ok(body)
    }

    /// `POST /_matrix/client/v3/join/{roomIdOrAlias}`.
    ///
    /// Returns the canonical room id on success (or the request id if the
    /// server omits `room_id` in the response).
    pub async fn join_room(&self, room_id_or_alias: &str) -> Result<String, PluginError> {
        let encoded = encode_path_segment(room_id_or_alias);
        let url = format!(
            "{}/_matrix/client/v3/join/{}",
            self.homeserver_url, encoded
        );

        debug!(room = %room_id_or_alias, "matrix: joining room");

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| {
                PluginError::ExecutionFailed(format!("matrix: join request failed: {e}"))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(PluginError::ExecutionFailed(format!(
                "matrix: join HTTP {status}: {text}"
            )));
        }

        #[derive(Deserialize)]
        struct JoinResp {
            #[serde(default)]
            room_id: Option<String>,
        }

        let body: JoinResp = resp.json().await.unwrap_or(JoinResp { room_id: None });
        Ok(body
            .room_id
            .unwrap_or_else(|| room_id_or_alias.to_string()))
    }

    /// `PUT /_matrix/client/v3/rooms/{roomId}/send/m.room.message/{txnId}`.
    ///
    /// Returns the server-assigned `event_id`.
    pub async fn send_text(
        &self,
        room_id: &str,
        body: &str,
        txn_id: &str,
    ) -> Result<String, PluginError> {
        let room_enc = encode_path_segment(room_id);
        let txn_enc = encode_path_segment(txn_id);
        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
            self.homeserver_url, room_enc, txn_enc
        );

        let payload = SendMessageBody {
            msgtype: "m.text",
            body,
        };

        debug!(
            room = %room_id,
            txn_id = %txn_id,
            content_len = body.len(),
            "matrix: sending m.room.message"
        );

        let resp = self
            .http
            .put(&url)
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| {
                PluginError::ExecutionFailed(format!("matrix: send request failed: {e}"))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(PluginError::ExecutionFailed(format!(
                "matrix: send HTTP {status}: {text}"
            )));
        }

        let parsed: SendMessageResponse = resp.json().await.map_err(|e| {
            PluginError::ExecutionFailed(format!("matrix: send response decode failed: {e}"))
        })?;
        Ok(parsed.event_id)
    }
}

// ---------------------------------------------------------------------------
// Encoding helpers (no extra crate)
// ---------------------------------------------------------------------------

/// Encode a path segment per RFC 3986 (unreserved chars left alone).
pub fn encode_path_segment(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push('%');
                out.push(hex_digit(b >> 4));
                out.push(hex_digit(b & 0x0f));
            }
        }
    }
    out
}

/// Encode a query-string value (same unreserved set as path segment).
fn encode_query_value(s: &str) -> String {
    encode_path_segment(s)
}

fn hex_digit(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'A' + (n - 10)) as char,
        _ => '0',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_room_id_encodes_bang_and_colon() {
        let enc = encode_path_segment("!abc:matrix.org");
        assert_eq!(enc, "%21abc%3Amatrix.org");
    }

    #[test]
    fn encode_leaves_unreserved() {
        assert_eq!(encode_path_segment("hello-world_1.0"), "hello-world_1.0");
    }

    #[test]
    fn homeserver_strips_trailing_slash() {
        let c = MatrixClient::new("https://matrix.org/", "tok", Duration::from_secs(5));
        assert_eq!(c.homeserver_url(), "https://matrix.org");
    }
}
