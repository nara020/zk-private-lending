//! Price Oracle Endpoints
//!
//! Provides ETH/USD price data for collateral valuation and liquidation checks.

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
