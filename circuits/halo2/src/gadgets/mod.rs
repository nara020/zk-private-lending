//! Reusable gadgets for ZK circuits
//!
//! - `RangeCheckChip`: Efficient range checks using lookup tables
//! - `ComparisonChip`: Greater-than-or-equal comparisons
//! - `CommitmentChip`: Pedersen/Poseidon commitment verification

pub mod comparison;
pub mod range_check;

pub use comparison::{ComparisonChip, ComparisonConfig, ComparisonInstruction};
pub use range_check::{RangeCheckChip, RangeCheckConfig, RangeCheckInstruction};
