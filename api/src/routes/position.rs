//! Position Endpoints
//!
//! # Interview Q&A
//!
//! Q: 왜 포지션 데이터를 DB에 저장하는가? 블록체인에 있지 않은가?
//! A: 블록체인 데이터 조회의 한계
//!
//!    블록체인에서 직접 조회:
//!    ❌ 느림 (매번 RPC 호출)
//!    ❌ 히스토리 조회 어려움 (이벤트 스캔 필요)
//!    ❌ 복잡한 쿼리 불가능
//!
//!    DB 인덱싱:
//!    ✅ 빠른 조회 (인덱스 활용)
//!    ✅ 히스토리 쿼리 간편
//!    ✅ 복잡한 분석 가능
//!
//!    → "이벤트 소싱" 패턴: 블록체인 이벤트를 DB에 동기화
//!
//! Q: 블록체인과 DB 데이터 불일치는 어떻게 처리하는가?
//! A: "Eventually Consistent" 접근
//!    1. 블록체인이 진실의 원천 (Source of Truth)
//!    2. DB는 읽기 최적화용 캐시
//!    3. 불일치 감지 시 재동기화
//!    4. 중요한 작업은 블록체인에서 직접 검증

use axum::{
    extract::{Path, State, Query},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{AppState, error::ApiError, db::PositionEvent as DbPositionEvent};

// ============ Request/Response Types ============

/// 포지션 조회 응답
#[derive(Debug, Serialize)]
pub struct PositionResponse {
    pub address: String,
    /// 담보 예치 여부 (금액은 숨김)
    pub has_deposit: bool,
    /// 대출 여부
    pub has_borrow: bool,
    /// 대출 금액 (USDC, 6 decimals) - 온체인에서 공개됨
    pub borrowed_amount: Option<String>,
    /// 담보 commitment (금액 숨김)
    pub collateral_commitment: Option<String>,
    /// 부채 commitment
    pub debt_commitment: Option<String>,
    /// 마지막 업데이트 시간
    pub last_updated: String,
}

/// 히스토리 쿼리 파라미터
#[derive(Debug, Deserialize)]
pub struct HistoryQuery {
    /// 페이지 (0부터 시작)
    pub page: Option<u32>,
    /// 페이지 크기 (기본 20, 최대 100)
    pub limit: Option<u32>,
}

/// 히스토리 응답
#[derive(Debug, Serialize)]
pub struct HistoryResponse {
    pub address: String,
    pub events: Vec<PositionEvent>,
    pub pagination: Pagination,
}

#[derive(Debug, Serialize)]
pub struct PositionEvent {
    pub event_type: String,  // deposit, borrow, repay, withdraw, liquidate
    pub amount: Option<String>,
    pub commitment: Option<String>,
    pub tx_hash: String,
    pub block_number: u64,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
pub struct Pagination {
    pub page: u32,
    pub limit: u32,
    pub total: u64,
    pub has_next: bool,
}

// ============ Handlers ============

/// GET /position/:address
///
/// 사용자 포지션 조회
///
/// # Response
///
/// ```json
/// {
///   "address": "0x...",
///   "has_deposit": true,
///   "has_borrow": true,
///   "borrowed_amount": "10000000000",  // 10,000 USDC
///   "collateral_commitment": "0x7a8b...",  // 금액은 숨김!
///   "debt_commitment": "0x9c2d...",
///   "last_updated": "2024-01-15T10:30:00Z"
/// }
/// ```
pub async fn get_position(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<Json<PositionResponse>, ApiError> {
    // 주소 형식 검증
    if !is_valid_ethereum_address(&address) {
        return Err(ApiError::ValidationError("Invalid Ethereum address".to_string()));
    }

    // DB에서 조회
    let position = state.db.get_position(&address).await?;

    match position {
        Some(pos) => Ok(Json(PositionResponse {
            address: address.clone(),
            has_deposit: pos.has_deposit,
            has_borrow: pos.has_borrow,
            borrowed_amount: pos.borrowed_amount.map(|a: i64| a.to_string()),
            collateral_commitment: pos.collateral_commitment,
            debt_commitment: pos.debt_commitment,
            last_updated: pos.updated_at.to_rfc3339(),
        })),
        None => {
            // DB에 없으면 새 사용자 (예치 없음)
            Ok(Json(PositionResponse {
                address,
                has_deposit: false,
                has_borrow: false,
                borrowed_amount: None,
                collateral_commitment: None,
                debt_commitment: None,
                last_updated: chrono::Utc::now().to_rfc3339(),
            }))
        }
    }
}

/// GET /position/:address/history
///
/// 포지션 히스토리 조회 (페이지네이션)
pub async fn get_position_history(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<HistoryResponse>, ApiError> {
    if !is_valid_ethereum_address(&address) {
        return Err(ApiError::ValidationError("Invalid Ethereum address".to_string()));
    }

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(20).min(100); // 최대 100개

    // DB에서 이벤트 조회
    let (events, total): (Vec<DbPositionEvent>, i64) = state.db.get_position_history(&address, page, limit).await?;

    let has_next = ((page + 1) * limit) < total as u32;

    Ok(Json(HistoryResponse {
        address,
        events: events.into_iter().map(|e| PositionEvent {
            event_type: e.event_type,
            amount: e.amount.map(|a: i64| a.to_string()),
            commitment: e.commitment,
            tx_hash: e.tx_hash,
            block_number: e.block_number as u64,
            timestamp: e.timestamp.to_rfc3339(),
        }).collect(),
        pagination: Pagination {
            page,
            limit,
            total: total as u64,
            has_next,
        },
    }))
}

// ============ Helpers ============

fn is_valid_ethereum_address(addr: &str) -> bool {
    // 0x로 시작하고 40자리 hex
    addr.starts_with("0x") && addr.len() == 42 && addr[2..].chars().all(|c| c.is_ascii_hexdigit())
}
