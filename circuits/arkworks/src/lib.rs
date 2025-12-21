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
//!
//! # Interview Q&A
//!
//! Q: 왜 arkworks와 Halo2 둘 다 구현했는가?
//! A: ZK 기술의 깊은 이해를 증명하기 위해
//!    - arkworks (R1CS): 학술적 기반, Groth16 최적
//!    - Halo2 (PLONKish): 실무 표준, L2 채택
//!    - 두 패러다임의 차이점 직접 비교

pub mod collateral;
pub mod ltv;
pub mod liquidation;

pub use collateral::CollateralCircuit;
pub use ltv::LTVCircuit;
pub use liquidation::LiquidationCircuit;
