//! ZK Proof Generation Endpoints
//!
//! Provides REST API endpoints for generating ZK proofs (collateral, LTV, liquidation).
//! Proofs are generated server-side using Halo2 circuits.

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{AppState, error::ApiError, services::ProofResult};

// ============ Request/Response Types ============

/// 담보 증명 요청
#[derive(Debug, Deserialize)]
pub struct CollateralProofRequest {
    /// 실제 담보 금액 (wei 단위)
    pub collateral: String,
    /// 최소 담보 임계값 (wei 단위)
    pub threshold: String,
    /// 랜덤 salt (commitment 생성에 사용됨)
    pub salt: String,
}

/// LTV 증명 요청
#[derive(Debug, Deserialize)]
pub struct LtvProofRequest {
    /// 담보 금액 (wei)
    pub collateral: String,
    /// 부채 금액 (6 decimals, USDC)
    pub debt: String,
    /// 최대 허용 LTV (%)
    pub max_ltv: u64,
    /// 담보 salt
    pub collateral_salt: String,
    /// 부채 salt
    pub debt_salt: String,
}

/// 청산 증명 요청
#[derive(Debug, Deserialize)]
pub struct LiquidationProofRequest {
    /// 담보 금액
    pub collateral: String,
    /// 부채 금액
    pub debt: String,
    /// ETH 가격 (8 decimals)
    pub price: String,
    /// 청산 임계값 (%)
    pub liquidation_threshold: u64,
    /// salt
    pub salt: String,
}

/// Proof 응답
#[derive(Debug, Serialize)]
pub struct ProofResponse {
    /// Groth16 proof (hex encoded)
    pub proof: ProofData,
    /// Public inputs (hex encoded)
    pub public_inputs: Vec<String>,
    /// Commitment 값
    pub commitment: String,
    /// 증명 생성 시간 (ms)
    pub generation_time_ms: u64,
}

#[derive(Debug, Serialize)]
pub struct ProofData {
    /// G1 point A
    pub a: [String; 2],
    /// G2 point B
    pub b: [[String; 2]; 2],
    /// G1 point C
    pub c: [String; 2],
}

// ============ Handlers ============

/// POST /proof/collateral
///
/// 담보 충분 증명 생성
///
/// # Flow
///
/// 1. 입력 검증 (collateral >= threshold 확인)
/// 2. Poseidon hash로 commitment 계산
/// 3. Halo2 회로로 ZK proof 생성
/// 4. Proof를 Groth16 형식으로 변환 (Solidity 호환)
/// 5. 응답 반환
///
/// # Security Note
///
/// - collateral과 salt는 private input → proof에 포함되지 않음
/// - threshold와 commitment만 public input
pub async fn generate_collateral_proof(
    State(state): State<AppState>,
    Json(req): Json<CollateralProofRequest>,
) -> Result<Json<ProofResponse>, ApiError> {
    tracing::info!("Generating collateral proof");
    let start = std::time::Instant::now();

    // 입력 파싱
    let collateral = parse_u128(&req.collateral)?;
    let threshold = parse_u128(&req.threshold)?;
    let salt = parse_u128(&req.salt)?;

    // 사전 검증 (proof 생성 전에 실패 케이스 빠르게 반환)
    if collateral < threshold {
        return Err(ApiError::ValidationError(
            "Collateral is less than threshold".to_string()
        ));
    }

    // Proof 생성
    let proof_result: ProofResult = state.zk_prover
        .generate_collateral_proof(collateral, threshold, salt)
        .await
        .map_err(|e: anyhow::Error| ApiError::ProofGenerationFailed(e.to_string()))?;

    let generation_time = start.elapsed().as_millis() as u64;
    tracing::info!("Collateral proof generated in {}ms", generation_time);

    // DB에 로그 저장 (optional)
    if let Err(e) = state.db.log_proof_generation("collateral", generation_time).await {
        tracing::warn!("Failed to log proof generation: {:?}", e);
    }

    Ok(Json(ProofResponse {
        proof: proof_result.proof,
        public_inputs: proof_result.public_inputs,
        commitment: proof_result.commitment,
        generation_time_ms: generation_time,
    }))
}

/// POST /proof/ltv
///
/// LTV 비율 증명 생성
pub async fn generate_ltv_proof(
    State(state): State<AppState>,
    Json(req): Json<LtvProofRequest>,
) -> Result<Json<ProofResponse>, ApiError> {
    tracing::info!("Generating LTV proof");
    let start = std::time::Instant::now();

    let collateral = parse_u128(&req.collateral)?;
    let debt = parse_u128(&req.debt)?;
    let max_ltv = req.max_ltv;
    let collateral_salt = parse_u128(&req.collateral_salt)?;
    let debt_salt = parse_u128(&req.debt_salt)?;

    // LTV 검증: debt * 100 <= collateral * max_ltv
    // 유한 필드에서 나눗셈을 피하기 위해 곱셈으로 변환
    if debt * 100 > collateral * max_ltv as u128 {
        return Err(ApiError::ValidationError(
            format!("LTV exceeds maximum: {}%", max_ltv)
        ));
    }

    let proof_result: ProofResult = state.zk_prover
        .generate_ltv_proof(collateral, debt, max_ltv, collateral_salt, debt_salt)
        .await
        .map_err(|e: anyhow::Error| ApiError::ProofGenerationFailed(e.to_string()))?;

    let generation_time = start.elapsed().as_millis() as u64;
    tracing::info!("LTV proof generated in {}ms", generation_time);

    Ok(Json(ProofResponse {
        proof: proof_result.proof,
        public_inputs: proof_result.public_inputs,
        commitment: proof_result.commitment,
        generation_time_ms: generation_time,
    }))
}

/// POST /proof/liquidation
///
/// 청산 가능 증명 생성
pub async fn generate_liquidation_proof(
    State(state): State<AppState>,
    Json(req): Json<LiquidationProofRequest>,
) -> Result<Json<ProofResponse>, ApiError> {
    tracing::info!("Generating liquidation proof");
    let start = std::time::Instant::now();

    let collateral = parse_u128(&req.collateral)?;
    let debt = parse_u128(&req.debt)?;
    let price = parse_u128(&req.price)?;
    let liquidation_threshold = req.liquidation_threshold;
    let salt = parse_u128(&req.salt)?;

    // 청산 조건 검증: collateral * price * liq_threshold < debt * 100
    // health_factor < 1.0 이면 청산 가능
    let collateral_value = collateral * price * liquidation_threshold as u128;
    let debt_value = debt * 100 * 100_000_000; // price는 8 decimals

    if collateral_value >= debt_value {
        return Err(ApiError::ValidationError(
            "Position is not liquidatable (health factor >= 1.0)".to_string()
        ));
    }

    let proof_result: ProofResult = state.zk_prover
        .generate_liquidation_proof(collateral, debt, price, liquidation_threshold, salt)
        .await
        .map_err(|e: anyhow::Error| ApiError::ProofGenerationFailed(e.to_string()))?;

    let generation_time = start.elapsed().as_millis() as u64;
    tracing::info!("Liquidation proof generated in {}ms", generation_time);

    Ok(Json(ProofResponse {
        proof: proof_result.proof,
        public_inputs: proof_result.public_inputs,
        commitment: proof_result.commitment,
        generation_time_ms: generation_time,
    }))
}

// ============ Helpers ============

fn parse_u128(s: &str) -> Result<u128, ApiError> {
    s.parse::<u128>()
        .map_err(|_| ApiError::ValidationError(format!("Invalid number: {}", s)))
}
