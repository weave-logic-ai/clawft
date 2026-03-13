//! WebSocket handler for real-time events.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
};

use super::ApiState;

/// WebSocket upgrade handler.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(_state): State<ApiState>,
) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    // Send welcome message.
    let welcome = serde_json::json!({
        "type": "connected",
        "message": "ClawFT WebSocket connected"
    });
    if socket
        .send(Message::Text(welcome.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    // Main message loop.
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                if let Ok(cmd) = serde_json::from_str::<serde_json::Value>(&text) {
                    let cmd_type = cmd.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    match cmd_type {
                        "subscribe" => {
                            let topic =
                                cmd.get("topic").and_then(|v| v.as_str()).unwrap_or("*");
                            let ack = serde_json::json!({
                                "type": "subscribed",
                                "topic": topic
                            });
                            let _ = socket
                                .send(Message::Text(ack.to_string().into()))
                                .await;
                        }
                        "unsubscribe" => {
                            let topic =
                                cmd.get("topic").and_then(|v| v.as_str()).unwrap_or("*");
                            let ack = serde_json::json!({
                                "type": "unsubscribed",
                                "topic": topic
                            });
                            let _ = socket
                                .send(Message::Text(ack.to_string().into()))
                                .await;
                        }
                        "ping" => {
                            let pong = serde_json::json!({"type": "pong"});
                            let _ = socket
                                .send(Message::Text(pong.to_string().into()))
                                .await;
                        }
                        _ => {}
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
