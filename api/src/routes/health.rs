//! Health Check Endpoint
//!
//! Provides deep health check including database connectivity status
//! for load balancer and Kubernetes probe integration.

use axum::{extract::State, Json};
use serde::Serialize;

use crate::AppState;

/// Health check 응답
#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub database: DatabaseStatus,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct DatabaseStatus {
    pub connected: bool,
    pub latency_ms: Option<u64>,
}

/// GET /health
///
/// 서버 및 의존성 상태 확인
pub async fn health_check(
    State(state): State<AppState>,
) -> Json<HealthResponse> {
    // DB 연결 테스트
    let db_start = std::time::Instant::now();
    let db_status = match state.db.health_check().await {
        Ok(_) => DatabaseStatus {
            connected: true,
            latency_ms: Some(db_start.elapsed().as_millis() as u64),
        },
        Err(_) => DatabaseStatus {
            connected: false,
            latency_ms: None,
        },
    };

    Json(HealthResponse {
        status: if db_status.connected { "healthy" } else { "degraded" }.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database: db_status,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
