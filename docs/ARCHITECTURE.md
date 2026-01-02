# ZK Private Lending - System Architecture

## Overview

ZK Private Lending is a privacy-preserving DeFi lending protocol that uses Zero-Knowledge proofs to hide user position sizes while maintaining protocol security.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           ZK Private Lending                                 │
│                                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                   │
│  │   Frontend   │◄──►│   API/Prover │◄──►│  Blockchain  │                   │
│  │   (React)    │    │   (Rust)     │    │  (Ethereum)  │                   │
│  └──────────────┘    └──────────────┘    └──────────────┘                   │
│         │                   │                    │                          │
│         │                   │                    │                          │
│         ▼                   ▼                    ▼                          │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                   │
│  │  Local State │    │ Halo2/Circom │    │  Contracts   │                   │
│  │  (Secrets)   │    │  Circuits    │    │  (Verified)  │                   │
│  └──────────────┘    └──────────────┘    └──────────────┘                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Core Components

### 1. Smart Contracts (Solidity)

**ZKLendingPool.sol** - Main lending pool contract

```
┌─────────────────────────────────────────────────────────────────┐
│                      ZKLendingPool                               │
├─────────────────────────────────────────────────────────────────┤
│  State Variables:                                                │
│  ├─ totalCollateralETH (visible)                                │
│  ├─ totalBorrowedUSDC (visible)                                 │
│  ├─ hasDeposit[user] (visible)                                  │
│  ├─ borrowedAmount[user] (visible - can't hide USDC transfer)   │
│  └─ commitments (visible - but hides actual amounts)            │
├─────────────────────────────────────────────────────────────────┤
│  Core Functions:                                                 │
│  ├─ deposit(commitment) → Store commitment, receive ETH         │
│  ├─ borrow(amount, proof) → Verify ZK proof, send USDC          │
│  ├─ repay(amount, newCommitment) → Repay debt + interest        │
│  ├─ withdraw(amount, proof, nullifier) → Verify & send ETH      │
│  └─ liquidate(user, proof) → Verify undercollateralized         │
├─────────────────────────────────────────────────────────────────┤
│  Interest Model:                                                 │
│  ├─ Base Rate: 5% APR                                           │
│  ├─ Variable: Up to +20% based on utilization                   │
│  ├─ Optimal Utilization: 80%                                    │
│  └─ Interest Index: Ray precision (1e27)                        │
└─────────────────────────────────────────────────────────────────┘
```

**Supporting Contracts:**
- `CommitmentRegistry.sol` - Stores and manages commitment hashes
- `ZKVerifier.sol` - On-chain proof verification
- `MockUSDC.sol` - Test token (testnet)

### 2. ZK Circuits

Three circuit implementations using different proving systems:

#### Halo2 (PSE Fork)
```
circuits/halo2/
├── src/
│   ├── collateral.rs   # Prove knowledge of collateral amount
│   ├── ltv.rs          # Prove LTV ratio within bounds
│   ├── liquidation.rs  # Prove position is liquidatable
│   └── gadgets/
│       ├── poseidon.rs # Poseidon hash implementation
│       ├── comparison.rs # Less-than/greater-than gadgets
│       └── range_check.rs # Range verification
```

**Curve**: Pasta (Pallas/Vesta)
**Proof Size**: ~1.5KB
**Verification**: O(log n)

#### arkworks (Groth16)
```
circuits/arkworks/
├── src/
│   ├── circuits.rs     # All three circuits
│   └── constraints.rs  # R1CS constraints
```

**Curve**: BN254
**Proof Size**: ~128 bytes
**Verification**: 3 pairings

#### Circom (Groth16)
```
circuits/circom/
├── collateral.circom
├── ltv.circom
├── liquidation.circom
└── lib/
    └── poseidon.circom
```

### 3. API Server (Rust)

```
api/
├── src/
│   ├── main.rs         # Server entry
│   ├── routes/
│   │   ├── price.rs    # Price endpoints
│   │   ├── prove.rs    # Proof generation
│   │   └── position.rs # Position queries
│   └── services/
│       ├── prover.rs   # Proof generation service
│       └── oracle.rs   # Price oracle integration
└── openapi.yaml        # API specification
```

### 4. Frontend (React)

```
frontend/
├── src/
│   ├── components/
│   │   ├── DepositForm.tsx    # Deposit collateral
│   │   ├── BorrowForm.tsx     # Borrow with ZK proof
│   │   ├── RepayForm.tsx      # Repay with interest
│   │   ├── PositionCard.tsx   # Display position
│   │   └── PoolStats.tsx      # Pool statistics
│   ├── hooks/
│   │   └── useWallet.ts       # Wallet connection
│   └── services/
│       ├── api.ts             # API client
│       └── contracts.ts       # Contract interactions
```

---

## Data Flow

### Deposit Flow
```
User                    Frontend              API                 Blockchain
  │                        │                    │                      │
  │ 1. Enter amount        │                    │                      │
  │───────────────────────►│                    │                      │
  │                        │                    │                      │
  │                        │ 2. Generate salt   │                      │
  │                        │    locally         │                      │
  │                        │                    │                      │
  │                        │ 3. Compute         │                      │
  │                        │────────────────────►                      │
  │                        │    commitment      │                      │
  │                        │◄────────────────────                      │
  │                        │                    │                      │
  │                        │ 4. Send ETH +      │                      │
  │                        │────────────────────────────────────────────►
  │                        │    commitment      │                      │
  │                        │                    │                      │
  │                        │ 5. Store           │                      │
  │◄───────────────────────│    (salt, amount)  │                      │
  │    Save to localStorage│    locally         │                      │
```

### Borrow Flow
```
User                    Frontend              API                 Blockchain
  │                        │                    │                      │
  │ 1. Enter amount        │                    │                      │
  │───────────────────────►│                    │                      │
  │                        │                    │                      │
  │                        │ 2. Request ZK      │                      │
  │                        │────────────────────►                      │
  │                        │    proof           │                      │
  │                        │                    │                      │
  │                        │    3. Generate     │                      │
  │                        │    Halo2/Groth16   │                      │
  │                        │    proof           │                      │
  │                        │◄────────────────────                      │
  │                        │                    │                      │
  │                        │ 4. Submit tx       │                      │
  │                        │────────────────────────────────────────────►
  │                        │    with proof      │                      │
  │                        │                    │                      │
  │                        │                    │  5. Verify proof     │
  │                        │                    │     on-chain         │
  │                        │◄──────────────────────────────────────────│
  │◄───────────────────────│                    │  6. Transfer USDC    │
```

---

## Privacy Model

### What's Hidden
- **Collateral Amount**: Only commitment visible on-chain
- **Salt**: Never leaves user's device
- **Position Ratio**: ZK proofs reveal nothing about actual values

### What's Visible
- **Borrow Amount**: USDC transfers are public (limitation)
- **Has Position**: Boolean flags are public
- **Total Pool Stats**: Aggregated totals are public

### Privacy Guarantees
```
┌────────────────────────────────────────────────────────────────┐
│                    Privacy Guarantee                           │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  commitment = Poseidon(amount, salt)                          │
│                                                                │
│  Given: commitment                                             │
│  Hidden: amount, salt                                          │
│                                                                │
│  Proof reveals: "I know (amount, salt) where:"                │
│    - Poseidon(amount, salt) = commitment                      │
│    - amount * price >= borrowAmount * (100/LTV)               │
│                                                                │
│  But NOT: actual amount or salt values                        │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

---

## Interest Rate Model

```
Rate = BaseRate + VariableRate × UtilizationFactor

Where:
  BaseRate = 5% (500 basis points)
  VariableRate = 20% (2000 basis points)

If utilization ≤ 80%:
  Rate = 5% + (utilization / 80%) × 20%

If utilization > 80%:
  Rate = 5% + 20% + ((utilization - 80%) / 20%) × 40%

┌──────────────────────────────────────────────────────────────┐
│  Interest Rate vs Utilization                                 │
│                                                               │
│  Rate (%)                                                     │
│    50│                                              ╭─────    │
│    40│                                          ╭───╯         │
│    30│                                      ╭───╯             │
│    25│                                  ╭───╯                 │
│    15│                          ╭───────╯                     │
│     5│──────────────────────────╯                             │
│      └──────────────────────────────────────────── Util (%)   │
│        0    20    40    60    80    100                       │
│                              ▲                                │
│                        Optimal (80%)                          │
└──────────────────────────────────────────────────────────────┘
```

---

## Security Model

### On-Chain Security
- **ReentrancyGuard**: All state-changing functions protected
- **SafeERC20**: Safe token transfers
- **Access Control**: Owner-only admin functions
- **ZK Verification**: All critical operations require valid proofs

### Off-Chain Security
- **Salt Generation**: Cryptographically secure random (browser crypto API)
- **Local Storage**: Secrets never leave the client
- **HTTPS**: All API communication encrypted
- **No Secret Storage**: API never stores user secrets

### Audit Status
- Slither: 44 findings analyzed (see SECURITY_AUDIT.md)
- Manual Review: Completed
- Formal Verification: Planned

---

## Deployment Architecture

### Production Setup
```
┌─────────────────────────────────────────────────────────────────┐
│                      Production Deployment                       │
│                                                                 │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐       │
│  │   CDN       │     │   Load      │     │   L2        │       │
│  │  (Static)   │────►│  Balancer   │────►│ (Optimism)  │       │
│  └─────────────┘     └─────────────┘     └─────────────┘       │
│         │                   │                    │              │
│         │                   │                    │              │
│         ▼                   ▼                    ▼              │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐       │
│  │  Frontend   │     │   API x3    │     │  Contracts  │       │
│  │   (Vercel)  │     │  (k8s pods) │     │  (Verified) │       │
│  └─────────────┘     └─────────────┘     └─────────────┘       │
│                             │                                   │
│                             │                                   │
│                             ▼                                   │
│                      ┌─────────────┐                           │
│                      │  PostgreSQL │                           │
│                      │   (Cache)   │                           │
│                      └─────────────┘                           │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Development Setup
```bash
# Start all services
docker-compose up -d

# Services:
# - api:3001       - Rust API server
# - postgres:5432  - Database
# - anvil:8545     - Local Ethereum node
# - frontend:5173  - Vite dev server
```

---

## Technology Stack

| Layer | Technology | Purpose |
|-------|------------|---------|
| Blockchain | Ethereum/L2 | Settlement layer |
| Smart Contracts | Solidity 0.8.20 | On-chain logic |
| ZK Circuits | Halo2/arkworks/Circom | Privacy proofs |
| API | Rust (Actix-web) | Proof generation |
| Frontend | React + TypeScript | User interface |
| Styling | TailwindCSS | UI components |
| State | React Query + Zustand | Client state |
| Testing | Hardhat + Foundry + Playwright | Full coverage |
| CI/CD | GitHub Actions | Automation |

---

## Future Improvements

### Phase 2
- [ ] Hide borrow amounts using commitment schemes
- [ ] Multi-collateral support (ETH, WETH, stETH)
- [ ] Recursive proofs for aggregation
- [ ] L2 deployment (Optimism, Arbitrum)

### Phase 3
- [ ] Client-side WASM proving
- [ ] Cross-chain lending
- [ ] Governance token
- [ ] Insurance fund

---

*Last updated: 2024-12-22*
