# ZK-Private DeFi Lending

> Privacy-preserving DeFi lending protocol using Zero-Knowledge Proofs

## Status: Complete (MVP)

A fully functional privacy-preserving DeFi lending protocol with:
- **3 ZK Stacks**: Halo2 (primary), arkworks, Circom
- **Smart Contracts**: Foundry-based with 35+ tests
- **Backend API**: Rust/Axum with real Halo2 proof generation
- **Frontend**: React + TypeScript + ethers.js v6
- **DevOps**: Docker Compose + GitHub Actions CI/CD

---

## Problem & Solution

### Problem: DeFi Transparency = Privacy Risk

```
Traditional DeFi (Aave, Compound):
┌─────────────────────────────────────────┐
│ User deposits 100 ETH as collateral     │
│ → Everyone sees: "0x123... has 100 ETH" │
│ → MEV bots track large positions        │
│ → Liquidation hunters front-run         │
└─────────────────────────────────────────┘
```

### Solution: ZK-Private Lending

```
ZK-Private Lending:
┌─────────────────────────────────────────┐
│ User deposits collateral privately      │
│ → Public: "User has sufficient funds"   │
│ → Private: Exact amount hidden          │
│ → Proof: ZK verification on-chain       │
└─────────────────────────────────────────┘
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Frontend (React + Vite + TS)                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │
│  │ Deposit  │  │  Borrow  │  │  Repay   │  │ Position │        │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘        │
└───────┼─────────────┼─────────────┼─────────────┼───────────────┘
        │             │             │             │
        ▼             ▼             ▼             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Backend API (Rust/Axum)                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ Proof Gen    │  │ Commitment   │  │ State Mgmt   │          │
│  │ Service      │  │ Manager      │  │              │          │
│  └──────┬───────┘  └──────────────┘  └──────────────┘          │
└─────────┼───────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────┐
│                       ZK Circuits (Halo2)                        │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐    │
│  │ CollateralProof│  │   LTVProof     │  │LiquidationProof│    │
│  │ collateral≥thr │  │ debt/coll≤max  │  │   HF < 1.0     │    │
│  └────────────────┘  └────────────────┘  └────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Smart Contracts (Solidity)                    │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐    │
│  │  ZKVerifier    │  │ Commitment     │  │ ZKLendingPool  │    │
│  │  (BN254)       │  │ Registry       │  │                │    │
│  └────────────────┘  └────────────────┘  └────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

---

## ZK Trinity Approach

Implementing the **same circuit logic** in three different ZK stacks:

| Stack | Role | Circuits | Why |
|-------|------|----------|-----|
| **Halo2** | Primary | 3 (Collateral, LTV, Liquidation) | Scroll, Polygon zkEVM standard |
| **arkworks** | Secondary | 1 (Collateral) | Low-level R1CS understanding |
| **Circom** | Secondary | 1 (Collateral) | DSL ecosystem familiarity |

### Why 3 Stacks?

```
면접관: "왜 Halo2를 선택했나요?"

나: "arkworks R1CS와 Circom DSL도 직접 구현해봤습니다.
     같은 CollateralProof 회로를 3개 스택으로 만들어서 비교했는데:

     - Range check: Halo2는 lookup 1개, R1CS는 16개 constraint
     - 개발 경험: Circom이 빠르지만 Halo2가 더 유연
     - 결론: L2 프로덕션엔 Halo2, PoC엔 Circom이 적합

     이 경험을 바탕으로 Halo2를 메인으로 선택했습니다."
```

---

## Core Circuits

### 1. CollateralProof (All 3 stacks)

**Purpose**: Prove `collateral >= threshold` without revealing exact amount

```
Public Inputs:
  - threshold: minimum required collateral
  - commitment: hash(collateral, salt)

Private Inputs:
  - collateral: actual collateral amount
  - salt: randomness for commitment

Constraints:
  1. collateral >= threshold (range check)
  2. commitment == hash(collateral, salt)
```

### 2. LTVProof (Halo2 only)

**Purpose**: Prove `(debt / collateral) <= max_ltv`

```
Public Inputs:
  - max_ltv: maximum allowed LTV (e.g., 80%)
  - debt_commitment
  - collateral_commitment

Private Inputs:
  - debt, collateral, salts

Constraints:
  1. debt * 100 <= collateral * max_ltv
  2. commitment validations
```

### 3. LiquidationProof (Halo2 only)

**Purpose**: Prove position is liquidatable (`health_factor < 1.0`)

```
Public Inputs:
  - price (from oracle)
  - liquidation_threshold

Private Inputs:
  - collateral, debt

Constraints:
  1. (collateral * price * liq_threshold) < debt
  2. commitment validations
```

---

## Development Plan

### Phase 1: ZK Circuits (3 weeks)

```
Week 1: Halo2 학습 & 환경 구축
├── Day 1-2: PSE Halo2 Book 학습
├── Day 3-4: Simple circuit 연습 (add, mul)
├── Day 5-6: Lookup table 연습
└── Day 7: CollateralProof 설계

Week 2: Halo2 회로 구현
├── Day 1-3: CollateralProof 구현 & 테스트
├── Day 4-5: LTVProof 구현
└── Day 6-7: LiquidationProof 구현

Week 3: arkworks + Circom + 비교 문서
├── Day 1-2: arkworks CollateralProof
├── Day 3-4: Circom CollateralProof
├── Day 5: 3스택 비교 분석 문서
└── Day 6-7: 테스트 & 벤치마크
```

### Phase 2: Smart Contracts (1 week)

```
├── ZKVerifier.sol: BN254 pairing verification
├── CommitmentRegistry.sol: commitment 저장/조회
└── ZKLendingPool.sol: 메인 lending 로직
```

### Phase 3: Backend (1 week)

```
├── Proof generation service
├── Commitment management
└── API endpoints (deposit, borrow, repay)
```

### Phase 4: Frontend (1 week)

```
├── Wallet connection (wagmi)
├── Deposit/Borrow/Repay UI
└── Position dashboard
```

---

## Project Structure

```
zk-private-lending/
├── circuits/
│   ├── halo2/                 # Primary - 3 circuits
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── collateral.rs  # CollateralProof
│   │   │   ├── ltv.rs         # LTVProof
│   │   │   ├── liquidation.rs # LiquidationProof
│   │   │   └── gadgets/       # Reusable components
│   │   └── tests/
│   │
│   ├── arkworks/              # Secondary - 1 circuit
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── collateral.rs
│   │
│   └── circom/                # Secondary - 1 circuit
│       ├── collateral.circom
│       ├── package.json
│       └── scripts/
│
├── contracts/                  # Solidity
│   ├── foundry.toml
│   ├── src/
│   │   ├── ZKVerifier.sol
│   │   ├── CommitmentRegistry.sol
│   │   └── ZKLendingPool.sol
│   └── test/
│
├── api/                        # Rust backend
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── routes/
│       └── services/
│
├── frontend/                   # React + Vite
│   ├── package.json
│   ├── src/
│   │   ├── components/        # UI components
│   │   ├── hooks/             # Custom hooks (useWallet)
│   │   └── services/          # API & contract services
│   └── vite.config.ts
│
├── .github/
│   └── workflows/
│       └── ci.yml              # GitHub Actions CI/CD
│
├── docker-compose.yml          # Full stack orchestration
│
└── docs/
    ├── ARCHITECTURE.md
    ├── ZK_COMPARISON.md
    └── API.md
```

---

## Tech Stack

| Layer | Technology | Version |
|-------|------------|---------|
| **ZK (Primary)** | Halo2 (PSE) | 0.3.0 |
| **ZK (Secondary)** | arkworks | 0.4.2 |
| **ZK (Secondary)** | Circom + snarkjs | 2.1.0 |
| **Curve** | BN254 | - |
| **Contracts** | Solidity + Foundry | 0.8.20 |
| **Backend** | Rust + Axum | 1.75+ |
| **Frontend** | React + Vite + TypeScript | 18.x |
| **Web3** | ethers.js + Zustand | 6.x |

---

## Key Technical Decisions

### Why BN254?

```solidity
// EVM precompiles (EIP-196, EIP-197)
// Gas costs:
ecAdd:     150 gas
ecMul:     6,000 gas
ecPairing: 34,000 * k + 45,000 gas

// Groth16 verification: ~200K gas
// L2 (Base, Arbitrum): ~$0.01
```

### Why Halo2 over Circom for Production?

| Aspect | Halo2 | Circom |
|--------|-------|--------|
| Range Check (8-bit) | 1 lookup | ~16 constraints |
| Custom Logic | Custom gates | Limited to templates |
| Debugging | MockProver (detailed) | Less detailed |
| L2 Adoption | Scroll, Polygon | Older projects |

### Commitment Scheme

Using Pedersen commitment for hiding collateral:

```
commitment = hash(collateral || salt)

Properties:
- Hiding: Can't determine collateral from commitment
- Binding: Can't find different collateral for same commitment
```

---

## Development Timeline

- [x] Architecture design
- [x] ZK stack selection & comparison analysis
- [x] Project structure setup
- [x] **Phase 1**: Halo2 circuits (Collateral, LTV, Liquidation)
- [x] **Phase 1**: arkworks circuits (Collateral, LTV, Liquidation)
- [x] **Phase 1**: Circom circuits (Collateral, LTV, Liquidation)
- [x] **Phase 2**: Solidity contracts & comprehensive tests
- [x] **Phase 3**: Backend API with real Halo2 integration
- [x] **Phase 4**: React frontend (Vite + TypeScript)
- [x] **Phase 5**: Docker & CI/CD setup

---

## Why This Project?

### 1. Practical Problem
- MEV attacks on large DeFi positions
- Institutional privacy requirements
- Regulatory compliance (selective disclosure)

### 2. Technical Depth
- Multi-stack ZK implementation
- PLONKish vs R1CS paradigm comparison
- Full-stack blockchain development

### 3. Career Growth
- Any ZK job posting covered (Halo2 OR Circom OR arkworks)
- L2 core development ready (Scroll, Polygon)
- DeFi domain expertise

---

## References

- [PSE Halo2 Book](https://zcash.github.io/halo2/)
- [arkworks Documentation](https://arkworks.rs/)
- [Circom Documentation](https://docs.circom.io/)
- [Aave Protocol](https://aave.com/) - DeFi lending reference

---

## Author

**Jinhyeok Kim** - Blockchain Engineer

- Prior ZK experience: arkworks-based ccSNARK implementation
- Hyperledger Besu 26x TPS optimization
- IEEE ICBTA 2024 paper (ZKP for EU DPP compliance)

## License

MIT
