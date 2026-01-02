# ZK-Private DeFi Lending

> Privacy-preserving DeFi lending protocol using Zero-Knowledge Proofs

[![CI](https://github.com/nara020/zk-private-lending/actions/workflows/ci.yml/badge.svg)](https://github.com/nara020/zk-private-lending/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

ZK-Private Lending is a privacy-preserving DeFi lending protocol that allows users to prove their collateral sufficiency without revealing exact amounts. Built with a **multi-stack ZK approach** for maximum flexibility and compatibility.

**Key Features:**
- **Privacy-First**: Collateral amounts remain hidden while proving sufficiency
- **Multi-Stack ZK**: Halo2 (primary), arkworks, and Circom implementations
- **Production-Ready**: Full-stack implementation with smart contracts, API, and frontend
- **EVM Compatible**: Optimized for L2 deployment (Scroll, Polygon zkEVM, Base)

---

## Problem Statement

Traditional DeFi lending protocols expose user positions publicly:

```
Traditional DeFi (Aave, Compound):
┌─────────────────────────────────────────┐
│ User deposits 100 ETH as collateral     │
│ → Everyone sees: "0x123... has 100 ETH" │
│ → MEV bots track large positions        │
│ → Liquidation hunters front-run         │
└─────────────────────────────────────────┘
```

**ZK-Private Lending Solution:**

```
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

## ZK Multi-Stack Implementation

This project implements the **same circuit logic** across three ZK frameworks for comparative analysis:

| Stack | Circuits | Use Case |
|-------|----------|----------|
| **Halo2 (PSE)** | Collateral, LTV, Liquidation | Production - L2 standard |
| **arkworks** | Collateral | R1CS/Groth16 reference |
| **Circom** | Collateral | DSL rapid prototyping |

### Comparison Insights

| Aspect | Halo2 | arkworks | Circom |
|--------|-------|----------|--------|
| Range Check (8-bit) | 1 lookup | ~16 constraints | ~16 constraints |
| Custom Logic | Custom gates | Manual R1CS | Templates |
| Debugging | MockProver | Limited | Limited |
| L2 Adoption | Scroll, Polygon | Research | Legacy |

---

## Core Circuits

### 1. CollateralProof

Proves `collateral >= threshold` without revealing exact amount.

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

### 2. LTVProof

Proves `(debt / collateral) <= max_ltv` for borrowing eligibility.

```
Public Inputs:
  - max_ltv: maximum allowed LTV (e.g., 80%)
  - debt_commitment, collateral_commitment

Private Inputs:
  - debt, collateral, salts

Constraints:
  1. debt * 100 <= collateral * max_ltv
  2. commitment validations
```

### 3. LiquidationProof

Proves position is liquidatable when `health_factor < 1.0`.

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

## Quick Start

### Prerequisites

- Rust 1.75+
- Node.js 18+
- Foundry
- Docker (optional)

### Installation

```bash
# Clone repository
git clone https://github.com/nara020/zk-private-lending.git
cd zk-private-lending

# Install dependencies
cd circuits/halo2 && cargo build --release
cd ../../contracts && forge install
cd ../frontend && npm install
cd ../api && cargo build --release
```

### Running with Docker

```bash
docker-compose up -d
```

### Running Tests

```bash
# ZK Circuits
cd circuits/halo2 && cargo test

# Smart Contracts
cd contracts && forge test

# API
cd api && cargo test
```

---

## Project Structure

```
zk-private-lending/
├── circuits/
│   ├── halo2/           # Primary ZK implementation
│   ├── arkworks/        # R1CS reference implementation
│   └── circom/          # DSL implementation
├── contracts/           # Solidity smart contracts
│   ├── src/
│   │   ├── ZKVerifier.sol
│   │   ├── CommitmentRegistry.sol
│   │   └── ZKLendingPool.sol
│   └── test/
├── api/                 # Rust backend
│   └── src/
│       ├── routes/
│       └── services/
├── frontend/            # React + TypeScript
│   └── src/
│       ├── components/
│       └── hooks/
├── docs/               # Documentation
└── docker-compose.yml
```

---

## Tech Stack

| Layer | Technology | Version |
|-------|------------|---------|
| **ZK (Primary)** | Halo2 (PSE) | 0.3.0 |
| **ZK (Secondary)** | arkworks | 0.4.2 |
| **ZK (Secondary)** | Circom + snarkjs | 2.1.0 |
| **Curve** | BN254 (EIP-196/197) | - |
| **Contracts** | Solidity + Foundry | 0.8.20 |
| **Backend** | Rust + Axum | 1.75+ |
| **Frontend** | React + Vite + TypeScript | 18.x |
| **Web3** | ethers.js v6 | 6.x |

---

## Technical Decisions

### Why BN254?

EVM precompiles (EIP-196, EIP-197) provide efficient on-chain verification:

```
Gas costs:
- ecAdd:     150 gas
- ecMul:     6,000 gas
- ecPairing: 34,000 * k + 45,000 gas

Groth16 verification: ~200K gas (~$0.01 on L2)
```

### Commitment Scheme

Pedersen-style commitment for hiding collateral:

```
commitment = hash(collateral || salt)

Properties:
- Hiding: Can't determine collateral from commitment
- Binding: Can't find different collateral for same commitment
```

---

## Documentation

- [Architecture Overview](docs/ARCHITECTURE.md)
- [ZK Stack Comparison](docs/ZK_COMPARISON.md)
- [API Reference](docs/API.md)
- [Circuit Explanations](docs/explanations/)

---

## Contributing

Contributions are welcome! Please read our contributing guidelines and submit PRs.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

---

## References

- [PSE Halo2 Book](https://zcash.github.io/halo2/)
- [arkworks Documentation](https://arkworks.rs/)
- [Circom Documentation](https://docs.circom.io/)
- [Aave Protocol](https://aave.com/) - DeFi lending reference

---

## License

MIT License - see [LICENSE](LICENSE) for details.
