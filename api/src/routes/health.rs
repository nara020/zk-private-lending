//! Health Check Endpoint
//!
//! # Interview Q&A
//!
//! Q: Health check 엔드포인트는 왜 필요한가?
//! A: 3가지 용도
//!    1. 로드밸런서 헬스체크 (ALB, nginx)
//!    2. Kubernetes liveness/readiness probe
//!    3. 모니터링 시스템 연동 (Prometheus, Datadog)
//!
//! Q: DB 연결 상태도 체크하는 이유는?
//! A: "깊은 헬스체크"(deep health check) 패턴
//!    - 단순 200 OK: 프로세스 살아있음
//!    - DB/Redis 체크: 실제 서비스 가능 상태
//!    - 외부 의존성 장애 시 트래픽 차단 가능

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
