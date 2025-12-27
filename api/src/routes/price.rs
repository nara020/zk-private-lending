//! Price Oracle Endpoints
//!
//! # Interview Q&A
//!
//! Q: 가격 오라클은 왜 필요한가?
//! A: DeFi 렌딩에서 담보 가치 평가에 필수
//!    - 담보 ETH 10개 → $20,000 가치 (ETH = $2,000)
//!    - 이 가치를 기준으로 대출 한도 계산
//!    - 가격 변동 시 청산 여부 판단
//!
//! Q: 가격 오라클 공격은 어떻게 방어하는가?
//! A: 여러 방어 전략
//!    1. 다중 오라클 (Chainlink + Uniswap TWAP)
//!    2. 가격 변동 제한 (1블록에 10% 이상 변동 거부)
//!    3. 시간 가중 평균 (TWAP) 사용
//!    4. 이상치 필터링
//!
//!    현재 프로젝트: 단순 가격 피드 (테스트용)
//!    프로덕션: Chainlink 연동 필수
//!
//! Q: 프론트엔드와 컨트랙트 가격이 다르면?
//! A: 컨트랙트 가격이 진실 (Source of Truth)
//!    - 프론트는 예상 표시용
//!    - 실제 거래는 컨트랙트 가격으로 실행
//!    - 프론트에서 슬리피지 경고 표시

use axum::{extract::State, Json};
use serde::Serialize;

use crate::{AppState, error::ApiError, services::PriceData};

/// 가격 응답
#[derive(Debug, Serialize)]
pub struct PriceResponse {
    /// 토큰 심볼
    pub symbol: String,
    /// USD 가격 (8 decimals, 예: 200000000000 = $2000)
    pub price_usd: String,
    /// 사람이 읽기 쉬운 형태
    pub price_formatted: String,
    /// 가격 소스
    pub source: String,
    /// 업데이트 시간
    pub updated_at: String,
    /// 24시간 변동률 (%)
    pub change_24h: Option<f64>,
}

/// GET /price/eth
///
/// ETH/USD 가격 조회
pub async fn get_eth_price(
    State(state): State<AppState>,
) -> Result<Json<PriceResponse>, ApiError> {
    let price_data: PriceData = state.price_oracle.get_eth_price().await
        .map_err(|_: anyhow::Error| ApiError::ServiceUnavailable("Price Oracle".to_string()))?;

    let price_usd = price_data.price;
    let price_formatted = format!("${:.2}", price_usd as f64 / 100_000_000.0);

    Ok(Json(PriceResponse {
        symbol: "ETH".to_string(),
        price_usd: price_usd.to_string(),
        price_formatted,
        source: price_data.source,
        updated_at: price_data.updated_at.to_rfc3339(),
        change_24h: price_data.change_24h,
    }))
}
