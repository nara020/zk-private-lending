-- Initial Migration: ZK Private Lending Database Schema
--
-- Interview Q&A:
--
-- Q: 왜 positions 테이블에 실제 담보 금액을 저장하지 않는가?
-- A: 프라이버시 보호가 프로젝트의 핵심!
--    - 온체인에도 commitment(해시)만 저장
--    - DB에도 동일하게 commitment만 저장
--    - 실제 금액은 사용자 클라이언트에만 존재
--
-- Q: 인덱스 전략은?
-- A: 조회 패턴 기반 최적화
--    - positions(address): 사용자별 조회 O(1)
--    - position_events(address, timestamp): 히스토리 범위 조회
--    - proof_logs(created_at): 분석용 시계열 조회

-- ============ Positions Table ============
-- 사용자 포지션 (현재 상태)

CREATE TABLE IF NOT EXISTS positions (
    -- 기본 키: Ethereum 주소 (lowercase)
    address VARCHAR(42) PRIMARY KEY,

    -- 예치/대출 상태
    has_deposit BOOLEAN NOT NULL DEFAULT FALSE,
    has_borrow BOOLEAN NOT NULL DEFAULT FALSE,

    -- 대출 금액 (USDC, 6 decimals)
    -- 온체인에서 공개되므로 저장
    -- NULL = 대출 없음
    borrowed_amount BIGINT,

    -- Commitment (Poseidon hash)
    -- 실제 금액은 숨겨짐!
    collateral_commitment VARCHAR(66),  -- 0x + 64 hex chars
    debt_commitment VARCHAR(66),

    -- 메타데이터
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 인덱스: 예치/대출 상태별 조회 (분석용)
CREATE INDEX idx_positions_has_deposit ON positions(has_deposit);
CREATE INDEX idx_positions_has_borrow ON positions(has_borrow);

-- ============ Position Events Table ============
-- 포지션 히스토리 (이벤트 소싱)

CREATE TABLE IF NOT EXISTS position_events (
    id BIGSERIAL PRIMARY KEY,

    -- 사용자 주소
    address VARCHAR(42) NOT NULL,

    -- 이벤트 타입
    -- deposit, borrow, repay, withdraw, liquidate
    event_type VARCHAR(20) NOT NULL,

    -- 금액 (해당되는 경우)
    amount BIGINT,

    -- 관련 commitment
    commitment VARCHAR(66),

    -- 블록체인 정보
    tx_hash VARCHAR(66) NOT NULL,
    block_number BIGINT NOT NULL,

    -- 이벤트 시간 (블록 타임스탬프)
    timestamp TIMESTAMPTZ NOT NULL,

    -- 레코드 생성 시간
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 인덱스: 사용자별 히스토리 조회
CREATE INDEX idx_events_address ON position_events(address);
-- 복합 인덱스: 사용자별 시간순 조회 (페이지네이션)
CREATE INDEX idx_events_address_timestamp ON position_events(address, timestamp DESC);
-- 인덱스: 이벤트 타입별 분석
CREATE INDEX idx_events_type ON position_events(event_type);

-- ============ Proof Logs Table ============
-- ZK Proof 생성 로그 (성능 분석용)

CREATE TABLE IF NOT EXISTS proof_logs (
    id BIGSERIAL PRIMARY KEY,

    -- 증명 타입: collateral, ltv, liquidation
    proof_type VARCHAR(20) NOT NULL,

    -- 생성 시간 (밀리초)
    generation_time_ms BIGINT NOT NULL,

    -- 생성 시간
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 인덱스: 시간순 조회 (성능 트렌드 분석)
CREATE INDEX idx_proof_logs_created ON proof_logs(created_at);

-- ============ Comments ============

COMMENT ON TABLE positions IS '사용자 포지션 - 담보 예치 및 대출 상태';
COMMENT ON COLUMN positions.collateral_commitment IS 'Poseidon(담보금액, salt) - 실제 금액 숨김';
COMMENT ON COLUMN positions.borrowed_amount IS 'USDC 대출 금액 (온체인 공개되므로 저장)';

COMMENT ON TABLE position_events IS '포지션 이벤트 히스토리 - 블록체인 이벤트 인덱싱';
COMMENT ON COLUMN position_events.tx_hash IS 'Ethereum 트랜잭션 해시';

COMMENT ON TABLE proof_logs IS 'ZK Proof 생성 성능 로그 - 분석 및 최적화용';
