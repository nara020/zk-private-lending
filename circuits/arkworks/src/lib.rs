//! arkworks R1CS Implementation
//!
//! ZK circuits using arkworks for R1CS comparison with Halo2.
//!
//! # Key Differences from Halo2
//! - R1CS: Rank-1 Constraint System (a·b = c gates only)
//! - Range check: Bit decomposition (~64 constraints for 64-bit)
//! - Proving system: Groth16 (per-circuit trusted setup)
//!
//! # Available Circuits
//!
//! | Circuit | Purpose | Constraints |
//! |---------|---------|-------------|
//! | CollateralCircuit | collateral >= threshold | ~200 |
//! | LTVCircuit | debt/collateral <= max_ltv | ~300 |
//! | LiquidationCircuit | health_factor < 1.0 | ~350 |

pub mod collateral;
pub mod ltv;
pub mod liquidation;

pub use collateral::CollateralCircuit;
pub use ltv::LTVCircuit;
pub use liquidation::LiquidationCircuit;
