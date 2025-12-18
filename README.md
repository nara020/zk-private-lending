# ZK-Private DeFi Lending

> Privacy-preserving DeFi lending protocol using Zero-Knowledge Proofs

## Status: In Development

This project implements a privacy-preserving DeFi lending protocol where users can prove their collateral is sufficient without revealing the exact amount.

## ZK Trinity Approach

Implementing the **same circuit logic** in three different ZK stacks for comprehensive comparison:

| Stack | Role | Why |
|-------|------|-----|
| **Halo2** | Primary | Used by Scroll, Polygon zkEVM - L2 production standard |
| **arkworks** | Secondary | Low-level Rust library for deep understanding |
| **Circom** | Secondary | DSL for rapid prototyping, large ecosystem |

## Core Circuits

### 1. CollateralProof (All 3 stacks)
Proves: `collateral >= threshold` without revealing exact collateral amount

### 2. LTVProof (Halo2 only)
Proves: `(debt / collateral) <= max_ltv` for loan-to-value ratio validation

### 3. LiquidationProof (Halo2 only)
Proves: `health_factor < 1.0` for liquidation eligibility

## Project Structure

```
zk-private-lending/
├── circuits/
│   ├── halo2/           # Primary - 3 circuits
│   │   ├── collateral.rs
│   │   ├── ltv.rs
│   │   └── liquidation.rs
│   ├── arkworks/        # Secondary - 1 circuit
│   │   └── collateral.rs
│   └── circom/          # Secondary - 1 circuit
│       └── collateral.circom
├── contracts/           # Solidity verifiers
├── api/                 # Rust backend
├── frontend/            # Next.js dashboard
└── docs/
    └── ZK_COMPARISON.md # Stack comparison analysis
```

## Tech Stack

- **ZK**: Halo2 (PSE), arkworks 0.4, Circom 2.1
- **Curve**: BN254 (EVM precompile compatible)
- **Smart Contracts**: Solidity, Foundry
- **Backend**: Rust, Axum
- **Frontend**: Next.js, TypeScript

## Key Features

### PLONKish vs R1CS Comparison

| Feature | Halo2 (PLONKish) | arkworks (R1CS) |
|---------|------------------|-----------------|
| Arithmetization | Custom gates, lookup tables | a·b = c multiplication gates |
| Range Check | 1 lookup | ~16 constraints for 8-bit |
| Flexibility | High (custom polynomials) | Medium (standard gates) |
| Learning Curve | Steep | Moderate |

### On-chain Verification

- BN254 curve with EVM precompiles (EIP-196, EIP-197)
- Groth16 verification: ~200K gas
- L2 cost: ~$0.01 per verification

## Development Timeline

- [x] Architecture design
- [x] ZK stack selection & comparison
- [ ] Halo2 circuit implementation
- [ ] arkworks circuit implementation
- [ ] Circom circuit implementation
- [ ] Solidity verifiers
- [ ] Backend integration
- [ ] Frontend dashboard

## Why This Project?

1. **Practical Privacy**: MEV protection, institutional requirements
2. **ZK Mastery**: Deep understanding through multi-stack comparison
3. **Career Growth**: Coverage for any ZK job posting (Halo2 OR Circom OR arkworks)

## Author

**Jinhyeok Kim** - Blockchain Engineer
- Prior ZK experience with arkworks (ccSNARK implementation)
- Currently expanding to Halo2 and Circom

## License

MIT
