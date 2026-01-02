//! ZK Private Lending API Library
//!
//! # Overview
//!
//! 이 라이브러리는 ZK Private Lending 프로토콜의 백엔드 API를 제공합니다.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                         API                              │
//! │                                                          │
//! │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐    │
//! │  │ Routes  │  │Services │  │   DB    │  │  Types  │    │
//! │  └────┬────┘  └────┬────┘  └────┬────┘  └────┬────┘    │
//! │       │            │            │            │          │
//! │       └────────────┴────────────┴────────────┘          │
//! │                         │                                │
//! └─────────────────────────┼────────────────────────────────┘
//!                           │
//!                           ▼
//!                  ┌────────────────┐
//!                  │   Contracts    │
//!                  └────────────────┘
//! ```
//!
//! ## Modules
//!
//! - `config`: 환경 설정 관리
//! - `error`: 에러 타입 및 처리
//! - `routes`: HTTP 엔드포인트 핸들러
//! - `services`: 비즈니스 로직 (ZK Prover, Price Oracle)
//! - `db`: 데이터베이스 연동
//! - `types`: 공통 타입 정의
//!
//! ## Usage
//!
//! ```rust,ignore
//! use zk_lending_api::{config::Config, db::Database, services::ZKProver};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::from_env()?;
//!     let db = Database::connect(&config.database_url).await?;
//!     let prover = ZKProver::new()?;
//!
//!     // ... 서버 시작
//!     Ok(())
//! }
//! ```

use std::sync::Arc;

pub mod config;
pub mod error;
pub mod routes;
pub mod services;
pub mod db;
pub mod types;

// Re-exports for convenience
pub use config::Config;
pub use error::ApiError;
pub use db::Database;
pub use services::{ZKProver, PriceOracle};

/// 애플리케이션 전역 상태
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub zk_prover: Arc<ZKProver>,
    pub price_oracle: Arc<PriceOracle>,
    pub config: Arc<Config>,
}
