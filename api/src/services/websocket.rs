//! WebSocket Service
//!
//! Real-time data streaming via WebSocket.
//!
//! # Features
//! - Pool status live updates
//! - User position change notifications
//! - Price change alerts
//! - Liquidation warnings

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use serde::{Deserialize, Serialize};
use anyhow::Result;

/// WebSocket 메시지 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    /// Pool 상태 업데이트
    PoolStatus(PoolStatusUpdate),
    /// 사용자 포지션 업데이트
    PositionUpdate(PositionUpdate),
    /// 가격 업데이트
    PriceUpdate(PriceUpdate),
    /// 청산 경고
    LiquidationWarning(LiquidationWarning),
    /// 트랜잭션 상태
    TransactionStatus(TransactionStatusUpdate),
    /// 에러
    Error(WsError),
    /// 구독 확인
    Subscribed(SubscriptionConfirm),
    /// Heartbeat
    Ping,
    Pong,
}

/// Pool 상태 업데이트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatusUpdate {
    pub total_collateral_eth: String,
    pub total_borrowed_usdc: String,
    pub available_liquidity: String,
    pub utilization_rate: f64,
    pub current_interest_rate: f64,
    pub eth_price: String,
    pub timestamp: u64,
}

/// 포지션 업데이트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionUpdate {
    pub address: String,
    pub has_deposit: bool,
    pub has_borrow: bool,
    pub borrowed_amount: String,
    pub accrued_interest: String,
    pub total_debt: String,
    pub health_factor: Option<f64>,
    pub timestamp: u64,
}

/// 가격 업데이트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceUpdate {
    pub asset: String,
    pub price: String,
    pub change_24h: f64,
    pub timestamp: u64,
}

/// 청산 경고
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidationWarning {
    pub address: String,
    pub health_factor: f64,
    pub threshold: f64,
    pub message: String,
    pub urgency: LiquidationUrgency,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiquidationUrgency {
    Low,      // HF 1.2 ~ 1.5
    Medium,   // HF 1.0 ~ 1.2
    High,     // HF < 1.0
    Critical, // HF < 0.9
}

/// 트랜잭션 상태 업데이트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStatusUpdate {
    pub tx_hash: String,
    pub status: String,
    pub block_number: Option<u64>,
    pub confirmations: u64,
    pub timestamp: u64,
}

/// WebSocket 에러
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsError {
    pub code: i32,
    pub message: String,
}

/// 구독 확인
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionConfirm {
    pub channel: String,
    pub subscribed: bool,
}

/// 구독 채널
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Channel {
    /// 전체 Pool 상태
    PoolStatus,
    /// 특정 사용자 포지션
    UserPosition(String),
    /// 가격
    Prices,
    /// 모든 이벤트
    AllEvents,
}

impl std::fmt::Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Channel::PoolStatus => write!(f, "pool_status"),
            Channel::UserPosition(addr) => write!(f, "position:{}", addr),
            Channel::Prices => write!(f, "prices"),
            Channel::AllEvents => write!(f, "all_events"),
        }
    }
}

/// 연결 상태
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub id: String,
    pub connected_at: u64,
    pub subscriptions: Vec<Channel>,
    pub last_activity: u64,
}

/// WebSocket Hub
///
/// 모든 WebSocket 연결과 메시지 브로드캐스팅을 관리
///
/// # Architecture
/// ```text
/// ┌─────────────┐     ┌──────────────┐     ┌─────────────┐
/// │   Client 1  │────▶│              │────▶│  Channel 1  │
/// ├─────────────┤     │   WsHub      │     ├─────────────┤
/// │   Client 2  │────▶│  (Router)    │────▶│  Channel 2  │
/// ├─────────────┤     │              │     ├─────────────┤
/// │   Client 3  │────▶│              │────▶│  Channel 3  │
/// └─────────────┘     └──────────────┘     └─────────────┘
/// ```
pub struct WsHub {
    /// 브로드캐스트 채널 (Pool 상태)
    pool_tx: broadcast::Sender<WsMessage>,
    /// 브로드캐스트 채널 (가격)
    price_tx: broadcast::Sender<WsMessage>,
    /// 사용자별 개인 채널
    user_channels: Arc<RwLock<HashMap<String, broadcast::Sender<WsMessage>>>>,
    /// 연결 정보
    connections: Arc<RwLock<HashMap<String, ConnectionInfo>>>,
}

impl WsHub {
    /// 새 WsHub 생성
    pub fn new() -> Self {
        let (pool_tx, _) = broadcast::channel(1000);
        let (price_tx, _) = broadcast::channel(1000);

        Self {
            pool_tx,
            price_tx,
            user_channels: Arc::new(RwLock::new(HashMap::new())),
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Pool 상태 브로드캐스트
    pub async fn broadcast_pool_status(&self, update: PoolStatusUpdate) {
        let _ = self.pool_tx.send(WsMessage::PoolStatus(update));
    }

    /// 가격 브로드캐스트
    pub async fn broadcast_price(&self, update: PriceUpdate) {
        let _ = self.price_tx.send(WsMessage::PriceUpdate(update));
    }

    /// 특정 사용자에게 포지션 업데이트 전송
    pub async fn send_position_update(&self, address: &str, update: PositionUpdate) {
        let channels = self.user_channels.read().await;
        if let Some(tx) = channels.get(address) {
            let _ = tx.send(WsMessage::PositionUpdate(update));
        }
    }

    /// 청산 경고 전송
    pub async fn send_liquidation_warning(&self, address: &str, warning: LiquidationWarning) {
        let channels = self.user_channels.read().await;
        if let Some(tx) = channels.get(address) {
            let _ = tx.send(WsMessage::LiquidationWarning(warning));
        }
    }

    /// 트랜잭션 상태 업데이트
    pub async fn send_tx_status(&self, address: &str, update: TransactionStatusUpdate) {
        let channels = self.user_channels.read().await;
        if let Some(tx) = channels.get(address) {
            let _ = tx.send(WsMessage::TransactionStatus(update));
        }
    }

    /// Pool 상태 구독
    pub fn subscribe_pool_status(&self) -> broadcast::Receiver<WsMessage> {
        self.pool_tx.subscribe()
    }

    /// 가격 구독
    pub fn subscribe_prices(&self) -> broadcast::Receiver<WsMessage> {
        self.price_tx.subscribe()
    }

    /// 사용자 채널 구독
    pub async fn subscribe_user(&self, address: &str) -> broadcast::Receiver<WsMessage> {
        let mut channels = self.user_channels.write().await;

        let tx = channels.entry(address.to_string()).or_insert_with(|| {
            let (tx, _) = broadcast::channel(100);
            tx
        });

        tx.subscribe()
    }

    /// 연결 등록
    pub async fn register_connection(&self, id: String, info: ConnectionInfo) {
        let mut conns = self.connections.write().await;
        conns.insert(id, info);
    }

    /// 연결 해제
    pub async fn unregister_connection(&self, id: &str) {
        let mut conns = self.connections.write().await;
        conns.remove(id);
    }

    /// 활성 연결 수
    pub async fn active_connections(&self) -> usize {
        let conns = self.connections.read().await;
        conns.len()
    }

    /// 모든 연결에 메시지 브로드캐스트
    pub async fn broadcast_all(&self, message: WsMessage) {
        let _ = self.pool_tx.send(message.clone());
        let _ = self.price_tx.send(message.clone());

        let channels = self.user_channels.read().await;
        for tx in channels.values() {
            let _ = tx.send(message.clone());
        }
    }
}

impl Default for WsHub {
    fn default() -> Self {
        Self::new()
    }
}

/// WebSocket 클라이언트 메시지 (수신)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ClientMessage {
    /// 채널 구독
    Subscribe { channel: String },
    /// 구독 취소
    Unsubscribe { channel: String },
    /// Ping (keepalive)
    Ping,
}

/// 클라이언트 메시지 파싱
pub fn parse_client_message(data: &str) -> Result<ClientMessage> {
    serde_json::from_str(data).map_err(Into::into)
}

/// 서버 메시지 직렬화
pub fn serialize_message(msg: &WsMessage) -> Result<String> {
    serde_json::to_string(msg).map_err(Into::into)
}

/// Health Factor 계산 유틸리티
pub fn calculate_health_factor(
    collateral_value_usd: f64,
    debt_usd: f64,
    liquidation_threshold: f64,
) -> f64 {
    if debt_usd == 0.0 {
        return f64::MAX;
    }
    (collateral_value_usd * liquidation_threshold / 100.0) / debt_usd
}

/// 청산 긴급도 판단
pub fn get_liquidation_urgency(health_factor: f64) -> LiquidationUrgency {
    if health_factor < 0.9 {
        LiquidationUrgency::Critical
    } else if health_factor < 1.0 {
        LiquidationUrgency::High
    } else if health_factor < 1.2 {
        LiquidationUrgency::Medium
    } else {
        LiquidationUrgency::Low
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ws_hub_creation() {
        let hub = WsHub::new();
        assert_eq!(hub.active_connections().await, 0);
    }

    #[tokio::test]
    async fn test_pool_status_broadcast() {
        let hub = WsHub::new();
        let mut rx = hub.subscribe_pool_status();

        let update = PoolStatusUpdate {
            total_collateral_eth: "100.0".to_string(),
            total_borrowed_usdc: "50000.0".to_string(),
            available_liquidity: "950000.0".to_string(),
            utilization_rate: 5.0,
            current_interest_rate: 5.0,
            eth_price: "2000.0".to_string(),
            timestamp: 1234567890,
        };

        hub.broadcast_pool_status(update.clone()).await;

        if let Ok(WsMessage::PoolStatus(received)) = rx.recv().await {
            assert_eq!(received.utilization_rate, 5.0);
        } else {
            panic!("Expected PoolStatus message");
        }
    }

    #[tokio::test]
    async fn test_user_subscription() {
        let hub = WsHub::new();
        let address = "0x1234";

        let mut rx = hub.subscribe_user(address).await;

        let update = PositionUpdate {
            address: address.to_string(),
            has_deposit: true,
            has_borrow: false,
            borrowed_amount: "0".to_string(),
            accrued_interest: "0".to_string(),
            total_debt: "0".to_string(),
            health_factor: None,
            timestamp: 1234567890,
        };

        hub.send_position_update(address, update.clone()).await;

        if let Ok(WsMessage::PositionUpdate(received)) = rx.recv().await {
            assert_eq!(received.address, address);
            assert!(received.has_deposit);
        } else {
            panic!("Expected PositionUpdate message");
        }
    }

    #[test]
    fn test_health_factor_calculation() {
        // $10000 담보, $5000 부채, 80% 청산 임계값
        let hf = calculate_health_factor(10000.0, 5000.0, 80.0);
        assert!((hf - 1.6).abs() < 0.001);

        // 부채 없으면 무한대
        let hf_no_debt = calculate_health_factor(10000.0, 0.0, 80.0);
        assert!(hf_no_debt > 1000000.0);
    }

    #[test]
    fn test_liquidation_urgency() {
        assert!(matches!(get_liquidation_urgency(0.8), LiquidationUrgency::Critical));
        assert!(matches!(get_liquidation_urgency(0.95), LiquidationUrgency::High));
        assert!(matches!(get_liquidation_urgency(1.1), LiquidationUrgency::Medium));
        assert!(matches!(get_liquidation_urgency(1.5), LiquidationUrgency::Low));
    }

    #[test]
    fn test_message_serialization() {
        let msg = WsMessage::Ping;
        let json = serialize_message(&msg).unwrap();
        assert!(json.contains("Ping"));
    }

    #[test]
    fn test_client_message_parsing() {
        let json = r#"{"action":"Subscribe","channel":"pool_status"}"#;
        let msg = parse_client_message(json).unwrap();

        if let ClientMessage::Subscribe { channel } = msg {
            assert_eq!(channel, "pool_status");
        } else {
            panic!("Expected Subscribe message");
        }
    }
}
