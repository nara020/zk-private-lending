//! Commitment Endpoints
//!
//! Provides cryptographic commitment creation and verification using Poseidon hash.
//! Commitments hide values while allowing later verification with the original salt.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{AppState, error::ApiError};

// ============ Request/Response Types ============

/// Commitment 생성 요청
#[derive(Debug, Deserialize)]
pub struct CreateCommitmentRequest {
    /// 숨기고 싶은 값 (예: 담보 금액)
    pub value: String,
    /// 랜덤 salt (클라이언트에서 생성)
    /// 없으면 서버에서 생성
    pub salt: Option<String>,
}

/// Commitment 응답
#[derive(Debug, Serialize)]
pub struct CommitmentResponse {
    /// Poseidon hash 결과 (hex)
    pub commitment: String,
    /// 사용된 salt (클라이언트가 저장해야 함)
    pub salt: String,
    /// 값 확인용 (디버깅/개발 환경에서만)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_echo: Option<String>,
}

/// Commitment 검증 요청
#[derive(Debug, Deserialize)]
pub struct VerifyCommitmentRequest {
    /// 검증할 commitment
    pub commitment: String,
    /// 원본 값
    pub value: String,
    /// 사용된 salt
    pub salt: String,
}

/// 검증 결과
#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub valid: bool,
    pub message: String,
}

// ============ Handlers ============

/// POST /commitment/create
///
/// Pedersen commitment 생성
///
/// # Flow
///
/// 1. 입력 값 파싱
/// 2. Salt 생성 (없으면 랜덤 생성)
/// 3. Poseidon(value, salt) 계산
/// 4. commitment, salt 반환
///
/// # Security Warning
///
/// 이 API는 value를 서버에 전송함 → 완전한 프라이버시 아님
/// 실제 프로덕션에서는 클라이언트에서 commitment 계산 권장
pub async fn create_commitment(
    State(state): State<AppState>,
    Json(req): Json<CreateCommitmentRequest>,
) -> Result<Json<CommitmentResponse>, ApiError> {
    let value = req.value.parse::<u128>()
        .map_err(|_| ApiError::ValidationError("Invalid value".to_string()))?;

    // Salt: 제공되면 사용, 없으면 생성
    let salt = match req.salt {
        Some(s) => s.parse::<u128>()
            .map_err(|_| ApiError::ValidationError("Invalid salt".to_string()))?,
        None => rand::random::<u128>(),
    };

    // Poseidon hash 계산
    let commitment = state.zk_prover.compute_commitment(value, salt)
        .map_err(|e| ApiError::InternalError)?;

    // 개발 환경에서만 value 에코
    let value_echo = if !state.config.is_production() {
        Some(req.value.clone())
    } else {
        None
    };

    Ok(Json(CommitmentResponse {
        commitment: format!("0x{}", hex::encode(&commitment)),
        salt: salt.to_string(),
        value_echo,
    }))
}

/// POST /commitment/verify
///
/// Commitment 검증 (value + salt로 commitment 재계산)
///
/// # Use Case
///
/// 사용자가 "내가 10 ETH를 예치했다"고 주장할 때
/// → commitment = Poseidon(10 ETH, salt) 인지 확인
pub async fn verify_commitment(
    State(state): State<AppState>,
    Json(req): Json<VerifyCommitmentRequest>,
) -> Result<Json<VerifyResponse>, ApiError> {
    let value = req.value.parse::<u128>()
        .map_err(|_| ApiError::ValidationError("Invalid value".to_string()))?;

    let salt = req.salt.parse::<u128>()
        .map_err(|_| ApiError::ValidationError("Invalid salt".to_string()))?;

    // commitment 재계산
    let computed = state.zk_prover.compute_commitment(value, salt)
        .map_err(|e| ApiError::InternalError)?;

    let computed_hex = format!("0x{}", hex::encode(&computed));

    // 대소문자 무시하고 비교
    let valid = computed_hex.to_lowercase() == req.commitment.to_lowercase();

    Ok(Json(VerifyResponse {
        valid,
        message: if valid {
            "Commitment verified successfully".to_string()
        } else {
            "Commitment does not match".to_string()
        },
    }))
}
