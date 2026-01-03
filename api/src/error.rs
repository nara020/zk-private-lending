//! Error Handling Module
//!
//! Provides type-safe error handling with proper HTTP status code mapping.
//! Uses thiserror for domain errors and integrates with tracing for structured logging.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

/// API 에러 타입
///
/// # Design Decision
///
/// 각 에러 variant는 적절한 HTTP 상태 코드에 매핑됨
/// - 클라이언트 에러: 4xx (잘못된 요청, 인증 실패 등)
/// - 서버 에러: 5xx (내부 오류)
///
/// 민감한 내부 정보는 클라이언트에 노출하지 않음
#[derive(Debug, Error)]
pub enum ApiError {
    // ============ 400 Bad Request ============
    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Validation failed: {0}")]
    ValidationError(String),

    // ============ 401 Unauthorized ============
    #[error("Authentication required")]
    Unauthorized,

    // ============ 404 Not Found ============
    #[error("Resource not found: {0}")]
    NotFound(String),

    // ============ 422 Unprocessable Entity ============
    #[error("Invalid commitment: {0}")]
    InvalidCommitment(String),

    #[error("Proof generation failed: {0}")]
    ProofGenerationFailed(String),

    // ============ 500 Internal Server Error ============
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Internal server error")]
    InternalError,

    // ============ 503 Service Unavailable ============
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
}

/// API 에러 응답 구조
#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message, details) = match &self {
            // 4xx 클라이언트 에러
            ApiError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                "BAD_REQUEST",
                msg.clone(),
                None,
            ),
            ApiError::ValidationError(msg) => (
                StatusCode::BAD_REQUEST,
                "VALIDATION_ERROR",
                "Validation failed".to_string(),
                Some(msg.clone()),
            ),
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "UNAUTHORIZED",
                "Authentication required".to_string(),
                None,
            ),
            ApiError::NotFound(resource) => (
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                format!("{} not found", resource),
                None,
            ),
            ApiError::InvalidCommitment(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_COMMITMENT",
                "Invalid commitment".to_string(),
                Some(msg.clone()),
            ),
            ApiError::ProofGenerationFailed(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "PROOF_GENERATION_FAILED",
                "Failed to generate ZK proof".to_string(),
                Some(msg.clone()),
            ),

            // 5xx 서버 에러
            ApiError::DatabaseError(_) => {
                // 내부 에러는 클라이언트에 상세 정보 노출 안 함
                tracing::error!("Database error: {:?}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "Database error occurred".to_string(),
                    None,
                )
            }
            ApiError::InternalError => {
                tracing::error!("Internal error: {:?}", self);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".to_string(),
                    None,
                )
            }
            ApiError::ServiceUnavailable(service) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "SERVICE_UNAVAILABLE",
                format!("{} is currently unavailable", service),
                None,
            ),
        };

        let body = ErrorResponse {
            error: message,
            code: code.to_string(),
            details,
        };

        (status, Json(body)).into_response()
    }
}

/// SQLx 에러를 ApiError로 변환
impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        tracing::error!("SQLx error: {:?}", err);
        ApiError::DatabaseError(err.to_string())
    }
}

/// anyhow 에러를 ApiError로 변환
impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!("Anyhow error: {:?}", err);
        ApiError::InternalError
    }
}
