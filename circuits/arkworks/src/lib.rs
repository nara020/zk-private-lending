//! arkworks R1CS Implementation
//!
//! CollateralProof circuit using arkworks for R1CS comparison with Halo2.
//!
//! # Key Differences from Halo2
//! - R1CS: Rank-1 Constraint System (a·b = c gates only)
//! - Range check: Bit decomposition (~16 constraints for 8-bit)
//! - Proving system: Groth16 (per-circuit trusted setup)

pub mod collateral;

pub use collateral::CollateralCircuit;
