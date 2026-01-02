//! ZK-Private DeFi Lending Circuits
//!
//! Privacy-preserving lending protocol using Halo2 (PSE fork)
//!
//! # Circuits
//! - `CollateralProof`: Prove collateral >= threshold without revealing amount
//! - `LTVProof`: Prove LTV ratio within bounds
//! - `LiquidationProof`: Prove position is liquidatable (HF < 1.0)
//!
//! # Features
//! - Production-grade Poseidon hash for commitments
//! - Efficient range checks using lookup tables
//! - Comprehensive error handling and validation
//!
//! # Example
//! ```ignore
//! use zk_private_lending_circuits::{CollateralCircuit, Fp};
//!
//! let collateral = Fp::from(1000u64);
//! let salt = Fp::from(12345u64);
//! let threshold = Fp::from(500u64);
//! let commitment = CollateralCircuit::compute_commitment(collateral, salt);
//!
//! let circuit = CollateralCircuit::new(collateral, salt, threshold, commitment);
//! ```

pub mod collateral;
pub mod error;
pub mod gadgets;
pub mod liquidation;
pub mod ltv;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(test)]
mod tests;

// Circuit exports
pub use collateral::CollateralCircuit;
pub use liquidation::LiquidationCircuit;
pub use ltv::LTVCircuit;

// Error handling
pub use error::{CircuitError, CircuitResult};
pub use error::validation;

// Gadget exports
pub use gadgets::{
    ComparisonChip, ComparisonConfig,
    RangeCheckChip, RangeCheckConfig,
    SimpleCommitmentChip, SimpleCommitmentConfig,
    PoseidonChip, PoseidonConfig,
};

// Re-export commonly used types from halo2
pub use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    plonk::{Circuit, ConstraintSystem, Error},
};

// Re-export Pasta curves
pub use pasta_curves::Fp;
