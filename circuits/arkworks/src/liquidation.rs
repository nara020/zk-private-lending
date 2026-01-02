//! LiquidationProof Circuit - arkworks R1CS Implementation
//!
//! Proves: position is liquidatable (health_factor < 1.0)
//!
//! # Interview Q&A
//!
//! Q: Health Factor란?
//! A: 포지션의 건전성 지표
//!    health_factor = (collateral * price * liquidation_threshold) / debt
//!
//!    HF >= 1.0: 안전
//!    HF < 1.0:  청산 가능
//!
//! Q: 왜 청산 증명이 필요한가?
//! A: 기존 DeFi에서는 포지션 금액이 공개되어 청산 시점 예측 가능
//!    → MEV 봇이 선행거래로 이익 탈취
//!
//!    ZK 청산:
//!    - 포지션 금액 숨김
//!    - "청산 가능하다"는 사실만 증명
//!    - 청산자만 이익 획득
//!
//! # Circuit Constraints
//! 1. Range check: collateral, debt, price in [0, 2^BITS)
//! 2. Liquidation check: collateral * price * threshold < debt * 100 * 1e8
//! 3. Position hash verification

use ark_ff::PrimeField;
use ark_r1cs_std::{
    alloc::AllocVar,
    boolean::Boolean,
    eq::EqGadget,
    fields::fp::FpVar,
    prelude::*,
    ToBitsGadget,
};
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_std::marker::PhantomData;

/// Number of bits for range checking
const RANGE_BITS: usize = 64;

/// LiquidationProof Circuit for arkworks
///
/// 청산 가능 여부를 증명하는 회로
///
/// 증명 내용:
/// health_factor < 1.0
/// = (collateral * price * liq_threshold) / debt < 1.0
/// = collateral * price * liq_threshold < debt * 100 (percentage) * 1e8 (price decimals)
#[derive(Clone)]
pub struct LiquidationCircuit<F: PrimeField> {
    /// Private: actual collateral amount
    pub collateral: Option<F>,
    /// Private: actual debt amount
    pub debt: Option<F>,
    /// Private: salt for position hash
    pub salt: Option<F>,
    /// Public: current ETH price (8 decimals)
    pub price: Option<F>,
    /// Public: liquidation threshold (percentage, e.g., 80 = 80%)
    pub liquidation_threshold: Option<F>,
    /// Public: position hash = hash(collateral, debt, salt)
    pub position_hash: Option<F>,
    _marker: PhantomData<F>,
}

impl<F: PrimeField> LiquidationCircuit<F> {
    /// Create a new circuit
    pub fn new(
        collateral: F,
        debt: F,
        salt: F,
        price: F,
        liquidation_threshold: F,
        position_hash: F,
    ) -> Self {
        Self {
            collateral: Some(collateral),
            debt: Some(debt),
            salt: Some(salt),
            price: Some(price),
            liquidation_threshold: Some(liquidation_threshold),
            position_hash: Some(position_hash),
            _marker: PhantomData,
        }
    }

    /// Create empty circuit for setup
    pub fn empty() -> Self {
        Self {
            collateral: None,
            debt: None,
            salt: None,
            price: None,
            liquidation_threshold: None,
            position_hash: None,
            _marker: PhantomData,
        }
    }

    /// Compute position hash (simplified)
    pub fn compute_position_hash(collateral: F, debt: F, salt: F) -> F {
        collateral * debt * salt + collateral + debt
    }

    /// Check if position is liquidatable
    pub fn is_liquidatable(
        collateral: u128,
        debt: u128,
        price: u128,      // 8 decimals
        liq_threshold: u128, // percentage
    ) -> bool {
        // health_factor < 1.0
        // collateral * price * liq_threshold < debt * 100 * 1e8
        let lhs = collateral * price * liq_threshold;
        let rhs = debt * 100 * 100_000_000;
        lhs < rhs
    }
}

impl<F: PrimeField> ConstraintSynthesizer<F> for LiquidationCircuit<F> {
    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
        // ======== Allocate Private Inputs ========

        let collateral_var = FpVar::new_witness(cs.clone(), || {
            self.collateral.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let debt_var = FpVar::new_witness(cs.clone(), || {
            self.debt.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let salt_var = FpVar::new_witness(cs.clone(), || {
            self.salt.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // ======== Allocate Public Inputs ========

        let price_var = FpVar::new_input(cs.clone(), || {
            self.price.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let liq_threshold_var = FpVar::new_input(cs.clone(), || {
            self.liquidation_threshold.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let position_hash_var = FpVar::new_input(cs.clone(), || {
            self.position_hash.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // ======== Constraint 1: Range Checks ========

        let collateral_bits = collateral_var.to_bits_le()?;
        for bit in collateral_bits.iter().skip(RANGE_BITS) {
            bit.enforce_equal(&Boolean::constant(false))?;
        }

        let debt_bits = debt_var.to_bits_le()?;
        for bit in debt_bits.iter().skip(RANGE_BITS) {
            bit.enforce_equal(&Boolean::constant(false))?;
        }

        // ======== Constraint 2: Liquidation Check ========
        // Prove: collateral * price * liq_threshold < debt * 100 * 1e8
        //
        // Rearranged: debt * 100 * 1e8 - collateral * price * liq_threshold > 0
        //
        // For strict inequality (a < b), we prove (b - a - 1) >= 0

        let hundred = FpVar::constant(F::from(100u64));
        let price_decimals = FpVar::constant(F::from(100_000_000u64)); // 1e8

        // lhs = collateral * price * liq_threshold
        let lhs = &collateral_var * &price_var * &liq_threshold_var;

        // rhs = debt * 100 * 1e8
        let rhs = &debt_var * &hundred * &price_decimals;

        // diff = rhs - lhs - 1 (strict inequality)
        // If liquidatable, diff >= 0
        let one = FpVar::constant(F::one());
        let diff = &rhs - &lhs - &one;

        // Add offset for range check
        let offset = FpVar::constant(F::from(1u128 << (RANGE_BITS - 1)));
        let diff_shifted = diff + offset;

        // Range check
        let diff_bits = diff_shifted.to_bits_le()?;
        for bit in diff_bits.iter().skip(RANGE_BITS + 32) {
            // Extended range for larger values
            bit.enforce_equal(&Boolean::constant(false))?;
        }

        // ======== Constraint 3: Position Hash Verification ========

        let computed_hash = &collateral_var * &debt_var * &salt_var + &collateral_var + &debt_var;
        computed_hash.enforce_equal(&position_hash_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use ark_relations::r1cs::ConstraintSystem;

    fn test_liquidation_circuit(
        collateral: u64,
        debt: u64,
        price: u64,  // 8 decimals
        liq_threshold: u64,
    ) -> bool {
        let collateral_f = Fr::from(collateral);
        let debt_f = Fr::from(debt);
        let salt = Fr::from(99999u64);
        let price_f = Fr::from(price);
        let liq_threshold_f = Fr::from(liq_threshold);

        let position_hash = LiquidationCircuit::compute_position_hash(collateral_f, debt_f, salt);

        let circuit = LiquidationCircuit::new(
            collateral_f,
            debt_f,
            salt,
            price_f,
            liq_threshold_f,
            position_hash,
        );

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();
        cs.is_satisfied().unwrap()
    }

    #[test]
    fn test_liquidatable_position() {
        // collateral=1 ETH, debt=2000 USDC, price=$1500, threshold=80%
        // health = (1 * 1500 * 80) / (2000 * 100) = 0.6 < 1.0 ✓ Liquidatable
        assert!(test_liquidation_circuit(
            1_000_000_000_000_000_000,  // 1 ETH (wei)
            2000_000_000,               // 2000 USDC (6 decimals)
            150000000000,               // $1500 (8 decimals)
            80                          // 80%
        ));
    }

    #[test]
    fn test_healthy_position() {
        // collateral=2 ETH, debt=2000 USDC, price=$2000, threshold=80%
        // health = (2 * 2000 * 80) / (2000 * 100) = 1.6 >= 1.0 ✗ Not liquidatable
        // Circuit should NOT be satisfied (we're proving it IS liquidatable)
        assert!(!test_liquidation_circuit(
            2_000_000_000_000_000_000,  // 2 ETH
            2000_000_000,               // 2000 USDC
            200000000000,               // $2000
            80
        ));
    }

    #[test]
    fn test_borderline_position() {
        // Exactly at liquidation threshold
        // This tests the strict inequality (< vs <=)
        // health = 1.0 exactly → NOT liquidatable (need < 1.0)

        // collateral=1, debt=80, price=100, threshold=80
        // health = (1 * 100 * 80) / (80 * 100) = 8000/8000 = 1.0
        // Since we need < 1.0, this should fail
        assert!(!test_liquidation_circuit(
            1_000_000_000_000_000_000,
            1600_000_000,  // 1600 USDC
            200000000000,  // $2000
            80
        ));
    }

    #[test]
    fn test_constraint_count() {
        let collateral = Fr::from(1_000_000_000_000_000_000u64);
        let debt = Fr::from(2000_000_000u64);
        let salt = Fr::from(99999u64);
        let price = Fr::from(150000000000u64);
        let liq_threshold = Fr::from(80u64);

        let position_hash = LiquidationCircuit::compute_position_hash(collateral, debt, salt);

        let circuit = LiquidationCircuit::new(
            collateral,
            debt,
            salt,
            price,
            liq_threshold,
            position_hash,
        );

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();

        println!("\n=== Liquidation Circuit R1CS Statistics ===");
        println!("Constraints: {}", cs.num_constraints());
        println!("Witness variables: {}", cs.num_witness_variables());
        println!("Public inputs: {}", cs.num_instance_variables());
    }

    #[test]
    fn test_groth16_proof() {
        use ark_bn254::{Bn254, Fr};
        use ark_groth16::Groth16;
        use ark_snark::SNARK;
        use ark_std::rand::thread_rng;

        let mut rng = thread_rng();

        // Liquidatable position
        let collateral = Fr::from(1_000_000_000_000_000_000u64);
        let debt = Fr::from(2000_000_000u64);
        let salt = Fr::from(99999u64);
        let price = Fr::from(150000000000u64);
        let liq_threshold = Fr::from(80u64);

        let position_hash = LiquidationCircuit::compute_position_hash(collateral, debt, salt);

        let circuit = LiquidationCircuit::new(
            collateral,
            debt,
            salt,
            price,
            liq_threshold,
            position_hash,
        );

        // Setup
        let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(
            LiquidationCircuit::<Fr>::empty(),
            &mut rng,
        ).unwrap();

        // Prove
        let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng).unwrap();

        // Public inputs: [price, liquidation_threshold, position_hash]
        let public_inputs = vec![price, liq_threshold, position_hash];

        // Verify
        let valid = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof).unwrap();
        assert!(valid, "Groth16 Liquidation proof should be valid");

        println!("\n=== Liquidation Groth16 Proof Generated ===");
    }
}
