//! Database Models
//!
//! Data models for blockchain event indexing and position tracking.
//! Stores commitments (hashes) for privacy while indexing public on-chain data.

use chrono::{DateTime, Utc};
use sqlx::FromRow;

/// 사용자 포지션
#[derive(Debug, Clone, FromRow)]
pub struct Position {
    /// Ethereum 주소 (lowercase)
    pub address: String,

    /// 담보 예치 여부
    pub has_deposit: bool,

    /// 대출 여부
    pub has_borrow: bool,

    /// 대출 금액 (USDC, 6 decimals)
    /// 온체인에서 공개되므로 저장
    pub borrowed_amount: Option<i64>,

    /// 담보 commitment (Poseidon hash)
    /// 실제 담보 금액은 숨겨짐!
    pub collateral_commitment: Option<String>,

    /// 부채 commitment
    pub debt_commitment: Option<String>,

    /// 마지막 업데이트 시간
    pub updated_at: DateTime<Utc>,
}

/// 포지션 이벤트 (히스토리)
#[derive(Debug, Clone, FromRow)]
pub struct PositionEvent {
    /// 이벤트 타입
    /// - deposit: 담보 예치
    /// - borrow: 대출
    /// - repay: 상환
    /// - withdraw: 출금
    /// - liquidate: 청산
    pub event_type: String,

    /// 금액 (해당되는 경우)
    /// deposit/withdraw: ETH (wei)
    /// borrow/repay: USDC (6 decimals)
    pub amount: Option<i64>,

    /// 관련 commitment
    pub commitment: Option<String>,

    /// 트랜잭션 해시
    pub tx_hash: String,

    /// 블록 번호
    pub block_number: i64,

    /// 이벤트 발생 시간
    pub timestamp: DateTime<Utc>,
}

/// Proof 생성 로그 (분석용)
#[derive(Debug, Clone, FromRow)]
pub struct ProofLog {
    pub id: i64,
    pub proof_type: String,
    pub generation_time_ms: i64,
    pub created_at: DateTime<Utc>,
}
