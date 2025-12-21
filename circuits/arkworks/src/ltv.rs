//! LTVProof Circuit - arkworks R1CS Implementation
//!
//! Proves: debt/collateral <= max_ltv without revealing actual amounts
//!
//! # Interview Q&A
//!
//! Q: 유한 필드에서 나눗셈을 어떻게 피하는가?
//! A: 부등식을 곱셈으로 변환
//!    debt/collateral <= max_ltv/100
//!    → debt * 100 <= collateral * max_ltv
//!
//! Q: R1CS에서 비교 연산의 비용은?
//! A: 비트 분해 필요 → ~64 constraints (64-bit 값)
//!    Halo2 lookup table은 1 constraint
//!
//! # Circuit Constraints
//! 1. Range check: debt in [0, 2^BITS)
//! 2. Range check: collateral in [0, 2^BITS)
//! 3. Comparison: debt * 100 <= collateral * max_ltv
//! 4. Commitment: debt_commitment == hash(debt, debt_salt)
//! 5. Commitment: collateral_commitment == hash(collateral, collateral_salt)

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

/// LTVProof Circuit for arkworks
///
/// # Design Decision
///
/// LTV(Loan-to-Value) = debt / collateral
///
/// 예: 담보 100 ETH, 대출 75 ETH → LTV = 75%
///
/// ZK로 증명: "LTV가 max_ltv 이하다" (실제 금액 숨김)
#[derive(Clone)]
pub struct LTVCircuit<F: PrimeField> {
    /// Private: actual collateral amount
    pub collateral: Option<F>,
    /// Private: collateral salt
    pub collateral_salt: Option<F>,
    /// Private: actual debt amount
    pub debt: Option<F>,
    /// Private: debt salt
    pub debt_salt: Option<F>,
    /// Public: maximum LTV (percentage, e.g., 75 = 75%)
    pub max_ltv: Option<F>,
    /// Public: collateral commitment
    pub collateral_commitment: Option<F>,
    /// Public: debt commitment
    pub debt_commitment: Option<F>,
    _marker: PhantomData<F>,
}

impl<F: PrimeField> LTVCircuit<F> {
    /// Create a new circuit
    pub fn new(
        collateral: F,
        collateral_salt: F,
        debt: F,
        debt_salt: F,
        max_ltv: F,
        collateral_commitment: F,
        debt_commitment: F,
    ) -> Self {
        Self {
            collateral: Some(collateral),
            collateral_salt: Some(collateral_salt),
            debt: Some(debt),
            debt_salt: Some(debt_salt),
            max_ltv: Some(max_ltv),
            collateral_commitment: Some(collateral_commitment),
            debt_commitment: Some(debt_commitment),
            _marker: PhantomData,
        }
    }

    /// Create empty circuit for setup
    pub fn empty() -> Self {
        Self {
            collateral: None,
            collateral_salt: None,
            debt: None,
            debt_salt: None,
            max_ltv: None,
            collateral_commitment: None,
            debt_commitment: None,
            _marker: PhantomData,
        }
    }

    /// Compute commitment (simplified hash)
    pub fn compute_commitment(value: F, salt: F) -> F {
        value * salt + value
    }
}

impl<F: PrimeField> ConstraintSynthesizer<F> for LTVCircuit<F> {
    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
        // ======== Allocate Private Inputs ========

        let collateral_var = FpVar::new_witness(cs.clone(), || {
            self.collateral.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let collateral_salt_var = FpVar::new_witness(cs.clone(), || {
            self.collateral_salt.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let debt_var = FpVar::new_witness(cs.clone(), || {
            self.debt.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let debt_salt_var = FpVar::new_witness(cs.clone(), || {
            self.debt_salt.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // ======== Allocate Public Inputs ========

        let max_ltv_var = FpVar::new_input(cs.clone(), || {
            self.max_ltv.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let collateral_commitment_var = FpVar::new_input(cs.clone(), || {
            self.collateral_commitment.ok_or(SynthesisError::AssignmentMissing)
        })?;

        let debt_commitment_var = FpVar::new_input(cs.clone(), || {
            self.debt_commitment.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // ======== Constraint 1 & 2: Range Checks ========

        // Range check collateral
        let collateral_bits = collateral_var.to_bits_le()?;
        for bit in collateral_bits.iter().skip(RANGE_BITS) {
            bit.enforce_equal(&Boolean::constant(false))?;
        }

        // Range check debt
        let debt_bits = debt_var.to_bits_le()?;
        for bit in debt_bits.iter().skip(RANGE_BITS) {
            bit.enforce_equal(&Boolean::constant(false))?;
        }

        // ======== Constraint 3: LTV Check ========
        // debt * 100 <= collateral * max_ltv
        //
        // Rearranged: collateral * max_ltv - debt * 100 >= 0
        //
        // Add offset for non-negative check

        let hundred = FpVar::constant(F::from(100u64));
        let lhs = &debt_var * &hundred;                    // debt * 100
        let rhs = &collateral_var * &max_ltv_var;          // collateral * max_ltv

        // diff = rhs - lhs = collateral * max_ltv - debt * 100
        // If LTV is valid, diff >= 0
        let diff = &rhs - &lhs;

        // Add offset for range check (shift into positive range)
        let offset = FpVar::constant(F::from(1u128 << (RANGE_BITS - 1)));
        let diff_shifted = diff + offset;

        // Range check the shifted difference
        let diff_bits = diff_shifted.to_bits_le()?;
        for bit in diff_bits.iter().skip(RANGE_BITS) {
            bit.enforce_equal(&Boolean::constant(false))?;
        }

        // ======== Constraint 4 & 5: Commitment Verification ========

        // collateral_commitment == collateral * collateral_salt + collateral
        let computed_collateral_comm = &collateral_var * &collateral_salt_var + &collateral_var;
        computed_collateral_comm.enforce_equal(&collateral_commitment_var)?;

        // debt_commitment == debt * debt_salt + debt
        let computed_debt_comm = &debt_var * &debt_salt_var + &debt_var;
        computed_debt_comm.enforce_equal(&debt_commitment_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use ark_relations::r1cs::ConstraintSystem;

    fn test_ltv_circuit(collateral: u64, debt: u64, max_ltv: u64) -> bool {
        let collateral_f = Fr::from(collateral);
        let collateral_salt = Fr::from(12345u64);
        let debt_f = Fr::from(debt);
        let debt_salt = Fr::from(67890u64);
        let max_ltv_f = Fr::from(max_ltv);

        let collateral_commitment = LTVCircuit::compute_commitment(collateral_f, collateral_salt);
        let debt_commitment = LTVCircuit::compute_commitment(debt_f, debt_salt);

        let circuit = LTVCircuit::new(
            collateral_f,
            collateral_salt,
            debt_f,
            debt_salt,
            max_ltv_f,
            collateral_commitment,
            debt_commitment,
        );

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();
        cs.is_satisfied().unwrap()
    }

    #[test]
    fn test_valid_ltv() {
        // collateral=1000, debt=500 → LTV=50%, max_ltv=75% ✓
        assert!(test_ltv_circuit(1000, 500, 75));
    }

    #[test]
    fn test_exact_max_ltv() {
        // collateral=1000, debt=750 → LTV=75%, max_ltv=75% ✓
        assert!(test_ltv_circuit(1000, 750, 75));
    }

    #[test]
    fn test_ltv_exceeded() {
        // collateral=1000, debt=800 → LTV=80%, max_ltv=75% ✗
        assert!(!test_ltv_circuit(1000, 800, 75));
    }

    #[test]
    fn test_zero_debt() {
        // collateral=1000, debt=0 → LTV=0%, max_ltv=75% ✓
        assert!(test_ltv_circuit(1000, 0, 75));
    }

    #[test]
    fn test_high_ltv() {
        // collateral=1000, debt=900 → LTV=90%, max_ltv=90% ✓
        assert!(test_ltv_circuit(1000, 900, 90));
    }

    #[test]
    fn test_constraint_count() {
        let collateral = Fr::from(1000u64);
        let collateral_salt = Fr::from(12345u64);
        let debt = Fr::from(500u64);
        let debt_salt = Fr::from(67890u64);
        let max_ltv = Fr::from(75u64);

        let collateral_commitment = LTVCircuit::compute_commitment(collateral, collateral_salt);
        let debt_commitment = LTVCircuit::compute_commitment(debt, debt_salt);

        let circuit = LTVCircuit::new(
            collateral,
            collateral_salt,
            debt,
            debt_salt,
            max_ltv,
            collateral_commitment,
            debt_commitment,
        );

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();

        println!("\n=== LTV Circuit R1CS Statistics ===");
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

        let collateral = Fr::from(1000u64);
        let collateral_salt = Fr::from(12345u64);
        let debt = Fr::from(500u64);
        let debt_salt = Fr::from(67890u64);
        let max_ltv = Fr::from(75u64);

        let collateral_commitment = LTVCircuit::compute_commitment(collateral, collateral_salt);
        let debt_commitment = LTVCircuit::compute_commitment(debt, debt_salt);

        let circuit = LTVCircuit::new(
            collateral,
            collateral_salt,
            debt,
            debt_salt,
            max_ltv,
            collateral_commitment,
            debt_commitment,
        );

        // Setup
        let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(
            LTVCircuit::<Fr>::empty(),
            &mut rng,
        ).unwrap();

        // Prove
        let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng).unwrap();

        // Public inputs: [max_ltv, collateral_commitment, debt_commitment]
        let public_inputs = vec![max_ltv, collateral_commitment, debt_commitment];

        // Verify
        let valid = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof).unwrap();
        assert!(valid, "Groth16 LTV proof should be valid");

        println!("\n=== LTV Groth16 Proof Generated ===");
    }
}
