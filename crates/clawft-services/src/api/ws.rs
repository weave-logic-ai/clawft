//! WebSocket handler for real-time topic-based events.
//!
//! Clients connect to `/ws` and can subscribe to named topics (e.g.
//! `"agents"`, `"sessions:abc123"`). When a producer publishes to a topic
//! via [`TopicBroadcaster`], the message is forwarded to all connected
//! clients subscribed to that topic.
//!
//! # Protocol
//!
//! Clients send JSON commands:
//!
//! - `{"type":"subscribe","topic":"<name>"}` -- subscribe to a topic
//! - `{"type":"unsubscribe","topic":"<name>"}` -- unsubscribe from a topic
//! - `{"type":"ping"}` -- keepalive; server responds with `{"type":"pong"}`
//!
//! The server sends JSON events:
//!
//! - `{"type":"connected","message":"..."}` -- on initial connection
//! - `{"type":"subscribed","topic":"<name>"}` -- ack after subscribe
//! - `{"type":"unsubscribed","topic":"<name>"}` -- ack after unsubscribe
//! - `{"type":"event","topic":"<name>","data":{...}}` -- broadcast event
//! - `{"type":"pong"}` -- keepalive response

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use super::ApiState;

/// WebSocket upgrade handler.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<ApiState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle a single WebSocket connection.
///
/// Splits the socket into a sender and receiver. The sender is wrapped in
/// `Arc<Mutex<_>>` so that subscription forwarding tasks can write to it
/// concurrently. Each subscription spawns a task that reads from the
/// broadcast receiver and forwards events to the client.
async fn handle_socket(socket: WebSocket, state: ApiState) {
    let (ws_sender, mut ws_receiver) = socket.split();

    // Wrap sender in Arc<Mutex> so subscription tasks can send messages.
    let sender = Arc::new(Mutex::new(ws_sender));

    // Track active subscription forwarding tasks per topic.
    let mut subscriptions: HashMap<String, JoinHandle<()>> = HashMap::new();

    // Send welcome message.
    {
        let welcome = serde_json::json!({
            "type": "connected",
            "message": "ClawFT WebSocket connected"
        });
        let mut s = sender.lock().await;
        if s.send(Message::Text(welcome.to_string().into()))
            .await
            .is_err()
        {
            return;
        }
    }

    // Main message loop -- process client commands.
    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            Message::Text(text) => {
                let cmd = match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let cmd_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("");

                match cmd_type {
                    "subscribe" => {
                        let topic = cmd
                            .get("topic")
                            .and_then(|v| v.as_str())
                            .unwrap_or("*")
                            .to_string();

                        // Don't double-subscribe; just re-ack.
                        if subscriptions.contains_key(&topic) {
                            let ack =
                                serde_json::json!({"type": "subscribed", "topic": &topic});
                            let mut s = sender.lock().await;
                            let _ =
                                s.send(Message::Text(ack.to_string().into())).await;
                            continue;
                        }

                        // Subscribe to the broadcast channel.
                        let mut rx = state.broadcaster.subscribe(&topic).await;
                        let sender_clone = sender.clone();
                        let topic_clone = topic.clone();

                        // Spawn a forwarding task that reads from the broadcast
                        // channel and writes events to this client's WebSocket.
                        let handle = tokio::spawn(async move {
                            loop {
                                match rx.recv().await {
                                    Ok(msg) => {
                                        // Parse the message as JSON, falling
                                        // back to a plain string value.
                                        let data = serde_json::from_str::<
                                            serde_json::Value,
                                        >(
                                            &msg
                                        )
                                        .unwrap_or(serde_json::Value::String(msg));

                                        let event = serde_json::json!({
                                            "type": "event",
                                            "topic": &topic_clone,
                                            "data": data
                                        });
                                        let mut s = sender_clone.lock().await;
                                        if s.send(Message::Text(
                                            event.to_string().into(),
                                        ))
                                        .await
                                        .is_err()
                                        {
                                            break; // Client disconnected.
                                        }
                                    }
                                    Err(
                                        tokio::sync::broadcast::error::RecvError::Lagged(
                                            _,
                                        ),
                                    ) => {
                                        // Slow consumer; skip missed messages.
                                        continue;
                                    }
                                    Err(_) => {
                                        // Channel closed.
                                        break;
                                    }
                                }
                            }
                        });

                        subscriptions.insert(topic.clone(), handle);

                        // Send subscription acknowledgement.
                        let ack =
                            serde_json::json!({"type": "subscribed", "topic": &topic});
                        let mut s = sender.lock().await;
                        let _ = s.send(Message::Text(ack.to_string().into())).await;
                    }

                    "unsubscribe" => {
                        let topic = cmd
                            .get("topic")
                            .and_then(|v| v.as_str())
                            .unwrap_or("*");

                        // Abort the forwarding task if it exists.
                        if let Some(handle) = subscriptions.remove(topic) {
                            handle.abort();
                        }

                        let ack =
                            serde_json::json!({"type": "unsubscribed", "topic": topic});
                        let mut s = sender.lock().await;
                        let _ = s.send(Message::Text(ack.to_string().into())).await;
                    }

                    "ping" => {
                        let pong = serde_json::json!({"type": "pong"});
                        let mut s = sender.lock().await;
                        let _ = s.send(Message::Text(pong.to_string().into())).await;
                    }

                    _ => {}
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    // Cleanup: abort all subscription forwarding tasks.
    for (_, handle) in subscriptions {
        handle.abort();
    }
}
