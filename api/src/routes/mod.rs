//! API Routes Module
//!
//! 모든 HTTP 엔드포인트 정의
//!
//! # Routes
//! - `/health` - 헬스 체크
//! - `/api/proof/*` - ZK 증명 생성
//! - `/api/commitment/*` - 커밋먼트 관리
//! - `/api/position/*` - 포지션 조회
//! - `/api/price/*` - 가격 정보
//! - `/ws` - WebSocket 실시간 데이터

pub mod health;
pub mod proof;
pub mod commitment;
pub mod position;
pub mod price;
pub mod ws;
