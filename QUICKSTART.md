# 🚀 ZK Private Lending - Quick Start Guide

## 전체 실행 순서

```
1. Anvil (로컬 이더리움) 시작
2. 컨트랙트 배포
3. API 서버 시작
4. Frontend 시작
5. 브라우저에서 테스트
```

---

## 📋 사전 준비

### 필수 설치
```bash
# Rust (API, Circuits)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Foundry (Contracts)
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Node.js 18+ (Frontend)
# https://nodejs.org/ 에서 설치

# PostgreSQL (선택사항 - API)
# Docker로 실행 권장
```

---

## 🔷 Step 1: 로컬 블록체인 시작 (Anvil)

```bash
# 터미널 1 - Anvil 실행 (백그라운드)
anvil

# 출력 예시:
# Listening on 127.0.0.1:8545
# Available Accounts (10개 테스트 계정 생성됨)
# Private Keys (테스트용 - 절대 실제 자금 사용 금지!)
```

**Anvil 기본 정보:**
- RPC URL: `http://localhost:8545`
- Chain ID: `31337`
- 테스트 계정: 각각 10,000 ETH 보유

---

## 🔷 Step 2: 컨트랙트 배포

```bash
# 터미널 2 - 컨트랙트 디렉토리로 이동
cd contracts

# 의존성 설치
forge install

# 컨트랙트 빌드
forge build

# 로컬 배포
forge script script/Deploy.s.sol --rpc-url http://localhost:8545 --broadcast

# 출력에서 컨트랙트 주소 확인:
# ZKVerifier:         0x5FbDB2315678afecb367f032d93F642f64180aa3
# CommitmentRegistry: 0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512
# MockUSDC:           0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0
# ZKLendingPool:      0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9
```

---

## 🔷 Step 3: API 서버 시작

### 3-1. PostgreSQL 실행 (Docker)
```bash
# 터미널 3 - Docker Compose로 DB만 실행
docker run -d \
  --name zk-lending-db \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=zk_lending \
  -p 5432:5432 \
  postgres:16-alpine
```

### 3-2. API 서버 실행
```bash
# 터미널 4 - API 디렉토리
cd api

# 환경변수 설정
cp .env.example .env

# .env 파일 수정 (배포된 컨트랙트 주소 입력)
# DATABASE_URL=postgres://postgres:postgres@localhost:5432/zk_lending
# ETH_RPC_URL=http://localhost:8545
# PORT=3001

# API 빌드 및 실행
cargo run --release

# 출력:
# 🚀 Starting ZK Private Lending API Server
# 📋 Configuration loaded
# 🗄️  Database connected
# 📦 Migrations completed
# 🔐 ZK Prover initialized
# 🌐 Listening on http://0.0.0.0:3001
```

### Health Check
```bash
curl http://localhost:3001/health
# {"status":"ok","timestamp":"..."}
```

---

## 🔷 Step 4: Frontend 시작

```bash
# 터미널 5 - Frontend 디렉토리
cd frontend

# 의존성 설치
npm install

# 환경변수 설정
cp .env.example .env

# .env 수정 (배포된 컨트랙트 주소 입력)
# VITE_API_URL=http://localhost:3001
# VITE_LENDING_POOL_ADDRESS=0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9
# VITE_USDC_ADDRESS=0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0

# 개발 서버 시작
npm run dev

# 출력:
#   VITE v5.x.x  ready in xxx ms
#   ➜  Local:   http://localhost:5173/
```

---

## 🔷 Step 5: MetaMask 설정

### 5-1. 로컬 네트워크 추가
```
1. MetaMask → 네트워크 추가
2. 네트워크 이름: Anvil Local
3. RPC URL: http://localhost:8545
4. Chain ID: 31337
5. 통화 기호: ETH
```

### 5-2. 테스트 계정 Import
```
# Anvil 시작 시 출력된 Private Key 사용
# 예시 (첫 번째 계정):
Private Key: 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80

MetaMask → 계정 가져오기 → 위 Private Key 입력
```

---

## 🔷 Step 6: 테스트

### 브라우저에서
1. `http://localhost:5173` 접속
2. "Connect Wallet" 클릭
3. MetaMask 연결
4. Deposit 탭에서 ETH 예치
5. Borrow 탭에서 USDC 대출

### API 직접 테스트
```bash
# ETH 가격 조회
curl http://localhost:3001/price/eth

# Commitment 생성
curl -X POST http://localhost:3001/commitment/create \
  -H "Content-Type: application/json" \
  -d '{"value": "1000000000000000000", "salt": "12345"}'

# Collateral Proof 생성
curl -X POST http://localhost:3001/proof/collateral \
  -H "Content-Type: application/json" \
  -d '{
    "collateral": "1000000000000000000",
    "threshold": "500000000000000000",
    "salt": "12345"
  }'
```

---

## 🐳 Docker로 전체 실행 (권장)

```bash
# 루트 디렉토리에서
docker-compose up -d

# 서비스 확인
docker-compose ps

# 로그 확인
docker-compose logs -f api

# 종료
docker-compose down
```

---

## ⚠️ 트러블슈팅

### "MetaMask가 연결 안 됨"
```bash
# Anvil이 실행 중인지 확인
curl http://localhost:8545 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}'
```

### "Database connection failed"
```bash
# PostgreSQL 실행 확인
docker ps | grep postgres

# DB 재시작
docker restart zk-lending-db
```

### "Contract not found"
```bash
# 컨트랙트 재배포
cd contracts
forge script script/Deploy.s.sol --rpc-url http://localhost:8545 --broadcast

# .env에 새 주소 업데이트
```

### "Proof generation failed"
```bash
# API 로그 확인
cd api && RUST_LOG=debug cargo run

# 메모리 부족 시 k 값 줄이기
# .env에서 ZK_PROVER_K=14 (기본 17)
```

---

## 📊 현재 RPC 연동 상태

| 컴포넌트 | RPC 연결 | 상태 |
|---------|---------|------|
| API → Anvil | `ETH_RPC_URL` 환경변수 | ✅ 설정됨 |
| Frontend → API | `VITE_API_URL` 환경변수 | ✅ 설정됨 |
| Frontend → MetaMask | `window.ethereum` | ✅ 브라우저에서 |
| Contracts | 배포 시 `--rpc-url` | ✅ 설정됨 |

**현재 기본값:**
- Anvil: `http://localhost:8545`
- API: `http://localhost:3001`
- Frontend: `http://localhost:5173`

---

## 🔗 유용한 명령어

```bash
# Anvil 계정에 ETH 전송 (테스트)
cast send --private-key 0xac0974... --value 1ether 0x받을주소

# 컨트랙트 함수 호출
cast call 0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9 "ethPrice()" --rpc-url http://localhost:8545

# 트랜잭션 전송
cast send 0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9 "updatePrice(uint256)" 2500_00000000 \
  --private-key 0xac0974... --rpc-url http://localhost:8545
```
