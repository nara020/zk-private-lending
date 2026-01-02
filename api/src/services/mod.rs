//! Services Module
//!
//! 비즈니스 로직을 담당하는 서비스 레이어
//!
//! # Services
//! - `ZKProver`: ZK 증명 생성 서비스
//! - `PriceOracle`: 가격 정보 서비스
//! - `BlockchainService`: 블록체인 상호작용
//! - `WsHub`: WebSocket 실시간 데이터

mod zk_prover;
mod price_oracle;
mod blockchain;
mod websocket;

pub use zk_prover::{ZKProver, ProofResult};
pub use price_oracle::{PriceOracle, PriceData};
pub use blockchain::{BlockchainService, BlockchainConfig, PoolStatus, UserPosition, TransactionRequest};
pub use websocket::{WsHub, WsMessage, PoolStatusUpdate, PositionUpdate, PriceUpdate, LiquidationWarning};
