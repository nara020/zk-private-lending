//! WebSocket Routes
//!
//! 실시간 데이터 스트리밍 WebSocket 엔드포인트
//!
//! # Endpoints
//! - `GET /ws` - WebSocket 연결

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::services::{WsHub, WsMessage};

/// WebSocket 업그레이드 핸들러
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(hub): State<Arc<WsHub>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, hub))
}

/// WebSocket 연결 처리
async fn handle_socket(socket: WebSocket, hub: Arc<WsHub>) {
    let (mut sender, mut receiver) = socket.split();

    // Pool 상태 구독
    let mut pool_rx = hub.subscribe_pool_status();
    let mut price_rx = hub.subscribe_prices();

    // 연결 ID 생성
    let conn_id = uuid::Uuid::new_v4().to_string();

    // 연결 등록
    hub.register_connection(
        conn_id.clone(),
        crate::services::websocket::ConnectionInfo {
            id: conn_id.clone(),
            connected_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            subscriptions: vec![],
            last_activity: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        },
    )
    .await;

    // 수신 태스크
    let hub_clone = hub.clone();
    let conn_id_clone = conn_id.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                        handle_client_message(&hub_clone, &conn_id_clone, client_msg).await;
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // 송신 태스크
    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Pool 상태 업데이트
                Ok(msg) = pool_rx.recv() => {
                    if let Ok(json) = serde_json::to_string(&msg) {
                        if sender.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                }

                // 가격 업데이트
                Ok(msg) = price_rx.recv() => {
                    if let Ok(json) = serde_json::to_string(&msg) {
                        if sender.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    // 연결이 종료될 때까지 대기
    tokio::select! {
        _ = recv_task => {}
        _ = send_task => {}
    }

    // 연결 해제
    hub.unregister_connection(&conn_id).await;
}

/// 클라이언트 메시지 타입
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "action")]
enum ClientMessage {
    Subscribe { channel: String },
    Unsubscribe { channel: String },
    Ping,
}

/// 클라이언트 메시지 처리
async fn handle_client_message(hub: &WsHub, conn_id: &str, msg: ClientMessage) {
    match msg {
        ClientMessage::Subscribe { channel } => {
            tracing::info!("Connection {} subscribed to {}", conn_id, channel);
            // 구독 처리
        }
        ClientMessage::Unsubscribe { channel } => {
            tracing::info!("Connection {} unsubscribed from {}", conn_id, channel);
            // 구독 취소 처리
        }
        ClientMessage::Ping => {
            // Pong 응답은 자동 처리
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_message_deserialize() {
        let json = r#"{"action":"Subscribe","channel":"pool_status"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();

        if let ClientMessage::Subscribe { channel } = msg {
            assert_eq!(channel, "pool_status");
        } else {
            panic!("Expected Subscribe");
        }
    }
}
