//! Reusable gadgets for ZK circuits
//!
//! - `RangeCheckChip`: Efficient range checks using lookup tables
//! - `ComparisonChip`: Greater-than-or-equal comparisons
//! - `PoseidonChip`: Poseidon hash for secure commitments
//! - `SimpleCommitmentChip`: Simple commitment for testing

pub mod comparison;
pub mod poseidon;
pub mod range_check;

pub use comparison::{ComparisonChip, ComparisonConfig, ComparisonInstruction};
pub use poseidon::simple::{SimpleCommitmentChip, SimpleCommitmentConfig};
pub use poseidon::{PoseidonChip, PoseidonConfig};
pub use range_check::{RangeCheckChip, RangeCheckConfig, RangeCheckInstruction};
