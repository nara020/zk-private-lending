//! ZK Private Lending API Server
//!
//! # Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        Client (Frontend)                     │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Axum Web Server                         │
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │                      Routes Layer                        ││
//! │  │  /health  /proof/*  /commitment/*  /position/*          ││
//! │  └─────────────────────────────────────────────────────────┘│
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │                    Services Layer                        ││
//! │  │  ZKProver    CommitmentService    PriceOracle           ││
//! │  └─────────────────────────────────────────────────────────┘│
//! │  ┌─────────────────────────────────────────────────────────┐│
//! │  │                    Data Layer                            ││
//! │  │  PostgreSQL Repository    Redis Cache (future)          ││
//! │  └─────────────────────────────────────────────────────────┘│
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    Smart Contracts (Ethereum)                │
//! │  ZKVerifier    CommitmentRegistry    ZKLendingPool          │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Interview Q&A
//!
//! Q: 왜 layered architecture를 선택했는가?
//! A: 1. 테스트 용이성 - 각 레이어를 독립적으로 테스트 가능
//!    2. 관심사 분리 - 라우팅, 비즈니스 로직, 데이터 접근 분리
//!    3. 확장성 - 새로운 엔드포인트 추가 시 기존 코드 영향 최소화
//!
//! Q: Axum의 State 공유 방식은?
//! A: Arc<AppState>를 사용하여 여러 핸들러에서 안전하게 공유
//!    - DB 커넥션 풀: Arc<Pool<Postgres>>
//!    - ZK Prover: Arc<Mutex<ZKProver>> (상태 있는 경우)
//!    - Config: Arc<Config> (읽기 전용)

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod routes;
mod services;
mod db;
mod types;

use config::Config;
use db::Database;
use services::{ZKProver, PriceOracle};

/// 애플리케이션 전역 상태
///
/// # Design Decision
///
/// Arc를 사용하는 이유:
/// - Axum 핸들러는 각 요청마다 클론됨
/// - Arc는 참조 카운팅으로 저렴한 클론 제공
/// - 내부 데이터는 불변이거나 Mutex로 보호
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub zk_prover: Arc<ZKProver>,
    pub price_oracle: Arc<PriceOracle>,
    pub config: Arc<Config>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 환경변수 로드
    dotenvy::dotenv().ok();

    // 로깅 초기화
    // RUST_LOG=debug,sqlx=warn 형태로 레벨 제어 가능
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "zk_lending_api=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 Starting ZK Private Lending API Server");

    // 설정 로드
    let config = Config::from_env()?;
    tracing::info!("📋 Configuration loaded");

    // 데이터베이스 연결
    let db = Database::connect(&config.database_url).await?;
    tracing::info!("🗄️  Database connected");

    // 마이그레이션 실행
    db.run_migrations().await?;
    tracing::info!("📦 Migrations completed");

    // 서비스 초기화
    let zk_prover = ZKProver::new()?;
    tracing::info!("🔐 ZK Prover initialized");

    let price_oracle = PriceOracle::new(&config.price_oracle_url);
    tracing::info!("💰 Price Oracle connected");

    // 앱 상태 구성
    let state = AppState {
        db: Arc::new(db),
        zk_prover: Arc::new(zk_prover),
        price_oracle: Arc::new(price_oracle),
        config: Arc::new(config.clone()),
    };

    // 라우터 구성
    let app = create_router(state);

    // 서버 시작
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("🌐 Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// 라우터 생성
///
/// # Route Structure
///
/// ```text
/// GET  /health              - 서버 상태 확인
///
/// POST /proof/collateral    - 담보 충분 증명 생성
/// POST /proof/ltv           - LTV 비율 증명 생성
/// POST /proof/liquidation   - 청산 가능 증명 생성
///
/// POST /commitment/create   - 커밋먼트 계산
/// POST /commitment/verify   - 커밋먼트 검증
///
/// GET  /position/:address   - 사용자 포지션 조회
/// GET  /position/:address/history - 포지션 히스토리
/// ```
fn create_router(state: AppState) -> Router {
    // CORS 설정
    // 프로덕션에서는 특정 도메인만 허용
    // 개발 환경에서는 localhost 허용
    use tower_http::cors::AllowOrigin;

    let cors = if state.config.is_production() {
        // 프로덕션: 특정 도메인만 허용 (환경변수로 설정)
        let allowed_origins = std::env::var("ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "https://yourdomain.com".to_string());
        let origins: Vec<_> = allowed_origins
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
            .allow_headers([axum::http::header::CONTENT_TYPE])
    } else {
        // 개발: localhost 허용
        CorsLayer::new()
            .allow_origin([
                "http://localhost:5173".parse().unwrap(),  // Vite dev server
                "http://localhost:3000".parse().unwrap(),  // Alternative
                "http://127.0.0.1:5173".parse().unwrap(),
            ])
            .allow_methods(Any)
            .allow_headers(Any)
    };

    Router::new()
        // Health check
        .route("/health", get(routes::health::health_check))

        // Proof generation
        .route("/proof/collateral", post(routes::proof::generate_collateral_proof))
        .route("/proof/ltv", post(routes::proof::generate_ltv_proof))
        .route("/proof/liquidation", post(routes::proof::generate_liquidation_proof))

        // Commitment
        .route("/commitment/create", post(routes::commitment::create_commitment))
        .route("/commitment/verify", post(routes::commitment::verify_commitment))

        // Position
        .route("/position/:address", get(routes::position::get_position))
        .route("/position/:address/history", get(routes::position::get_position_history))

        // Price
        .route("/price/eth", get(routes::price::get_eth_price))

        // 미들웨어
        .layer(TraceLayer::new_for_http())
        .layer(cors)

        // 상태 주입
        .with_state(state)
}
