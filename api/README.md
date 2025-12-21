# ZK Private Lending - Backend API

Rust + Axum 기반 백엔드 API 서버

## 📁 구조

```
api/
├── src/
│   ├── main.rs              # 서버 엔트리포인트
│   ├── lib.rs               # 라이브러리 모듈
│   ├── config.rs            # 환경 설정
│   ├── error.rs             # 에러 처리
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── health.rs        # GET /health
│   │   ├── proof.rs         # POST /proof/*
│   │   ├── commitment.rs    # POST /commitment/*
│   │   ├── position.rs      # GET /position/*
│   │   └── price.rs         # GET /price/*
│   ├── services/
│   │   ├── mod.rs
│   │   ├── zk_prover.rs     # ZK Proof 생성
│   │   └── price_oracle.rs  # 가격 조회
│   ├── db/
│   │   ├── mod.rs           # PostgreSQL 연동
│   │   ├── models.rs        # 데이터 모델
│   │   └── repository.rs    # 리포지토리 패턴
│   └── types/
│       └── mod.rs           # 공통 타입
├── migrations/
│   └── 001_initial.sql      # DB 스키마
├── Cargo.toml
└── README.md
```

## 🚀 시작하기

### 1. 환경 설정

```bash
# .env 파일 생성
cp .env.example .env

# 환경변수 수정
DATABASE_URL=postgres://user:password@localhost:5432/zk_lending
```

### 2. PostgreSQL 설정

```bash
# Docker로 PostgreSQL 실행
docker run -d \
  --name zk-lending-db \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=zk_lending \
  -p 5432:5432 \
  postgres:15
```

### 3. 빌드 및 실행

```bash
cd api

# 의존성 설치 및 빌드
cargo build

# 마이그레이션 실행 (sqlx-cli 필요)
cargo install sqlx-cli
sqlx migrate run

# 서버 실행
cargo run
```

### 4. 테스트

```bash
cargo test
```

## 📡 API Endpoints

### Health Check
```
GET /health

Response:
{
  "status": "healthy",
  "version": "0.1.0",
  "database": { "connected": true, "latency_ms": 1 },
  "timestamp": "2024-01-15T10:30:00Z"
}
```

### Proof Generation

```
POST /proof/collateral
Content-Type: application/json

{
  "collateral": "10000000000000000000",  // 10 ETH (wei)
  "threshold": "5000000000000000000",    // 5 ETH
  "salt": "12345678901234567890"
}

Response:
{
  "proof": { "a": [...], "b": [...], "c": [...] },
  "public_inputs": ["0x...", "0x..."],
  "commitment": "0x...",
  "generation_time_ms": 150
}
```

```
POST /proof/ltv
POST /proof/liquidation
```

### Commitment

```
POST /commitment/create
{
  "value": "10000000000000000000",
  "salt": "optional"  // 없으면 서버에서 생성
}

Response:
{
  "commitment": "0x...",
  "salt": "123..."
}
```

### Position

```
GET /position/0x1234...

Response:
{
  "address": "0x1234...",
  "has_deposit": true,
  "has_borrow": true,
  "borrowed_amount": "10000000000",
  "collateral_commitment": "0x...",
  "last_updated": "2024-01-15T10:30:00Z"
}
```

### Price

```
GET /price/eth

Response:
{
  "symbol": "ETH",
  "price_usd": "200000000000",
  "price_formatted": "$2000.00",
  "source": "chainlink",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

## ⚙️ 환경변수

| 변수 | 설명 | 기본값 |
|-----|------|--------|
| `PORT` | 서버 포트 | 3001 |
| `DATABASE_URL` | PostgreSQL 연결 문자열 | - |
| `PRICE_ORACLE_URL` | 가격 오라클 URL | http://localhost:3002 |
| `ETH_RPC_URL` | Ethereum RPC URL | http://localhost:8545 |
| `ENVIRONMENT` | 환경 (development/production) | development |

## 🔧 개발

### 로그 레벨 설정
```bash
RUST_LOG=debug cargo run
RUST_LOG=zk_lending_api=debug,tower_http=debug cargo run
```

### SQLx 오프라인 모드
```bash
# 쿼리 캐시 생성 (CI/CD용)
cargo sqlx prepare
```

## 📋 TODO

- [ ] 실제 Halo2 Prover 연동
- [ ] Chainlink Oracle 연동
- [ ] WebSocket 지원 (실시간 가격)
- [ ] Rate Limiting
- [ ] 인증/인가 (API Key)
