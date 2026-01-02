//! Database Models
//!
//! # Interview Q&A
//!
//! Q: 왜 블록체인 데이터를 DB에 중복 저장하는가?
//! A: "인덱싱" 패턴 - 블록체인 조회의 한계 극복
//!
//!    블록체인 직접 조회 문제:
//!    - 느림 (매번 노드 RPC 호출)
//!    - 복잡한 쿼리 불가 (JOIN, 집계 등)
//!    - 히스토리 조회 어려움 (이벤트 스캔)
//!
//!    DB 인덱싱 장점:
//!    - 빠른 조회 (인덱스 활용)
//!    - SQL 파워 (복잡한 분석 가능)
//!    - 캐시 역할
//!
//!    주의: 블록체인이 진실의 원천 (Source of Truth)
//!          DB는 읽기 최적화용 캐시
//!
//! Q: borrowed_amount는 왜 DB에 저장하는가? 이것도 숨겨야 하지 않나?
//! A: 현재 설계의 한계
//!
//!    문제: USDC 전송은 온체인에서 공개됨
//!    → pool.borrow(1000 USDC) 호출 시 1000이 노출
//!
//!    개선 방안:
//!    1. 대출 금액도 commitment로 숨기기
//!    2. 고정 금액 단위 (100 USDC씩만 대출)
//!    3. Mixer 활용 (대출 금액 섞기)
//!
//!    현재는 MVP로 담보 금액만 숨김

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
