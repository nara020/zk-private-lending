//! ZK-Private DeFi Lending Circuits
//!
//! Privacy-preserving lending protocol using Halo2 (PSE fork)
//!
//! # Circuits
//! - `CollateralProof`: Prove collateral >= threshold without revealing amount
//! - `LTVProof`: Prove LTV ratio within bounds
//! - `LiquidationProof`: Prove position is liquidatable (HF < 1.0)

pub mod collateral;
pub mod gadgets;
pub mod liquidation;
pub mod ltv;

pub use collateral::CollateralCircuit;
pub use liquidation::LiquidationCircuit;
pub use ltv::LTVCircuit;

// Re-export commonly used types
pub use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    pasta::Fp,
    plonk::{Circuit, ConstraintSystem, Error},
};
