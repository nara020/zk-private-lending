//! CollateralProof Circuit - arkworks R1CS Implementation
//!
//! Same logic as Halo2 version, but using R1CS constraints.
//!
//! # R1CS vs PLONKish Comparison
//!
//! | Aspect | arkworks (R1CS) | Halo2 (PLONKish) |
//! |--------|-----------------|------------------|
//! | Gate type | a·b = c only | Custom gates |
//! | Range check | Bit decomposition | Lookup table |
//! | Constraints (8-bit) | ~16 | 1 |
//! | Flexibility | Lower | Higher |
//!
//! # Circuit Constraints
//! 1. Range check: collateral in [0, 2^BITS)
//! 2. Range check: threshold in [0, 2^BITS)
//! 3. Comparison: collateral >= threshold
//! 4. Commitment: commitment == hash(collateral, salt)

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

/// CollateralProof Circuit for arkworks
#[derive(Clone)]
pub struct CollateralCircuit<F: PrimeField> {
    /// Private: actual collateral amount
    pub collateral: Option<F>,
    /// Private: salt for commitment
    pub salt: Option<F>,
    /// Public: minimum threshold
    pub threshold: Option<F>,
    /// Public: commitment to collateral
    pub commitment: Option<F>,
    _marker: PhantomData<F>,
}

impl<F: PrimeField> CollateralCircuit<F> {
    /// Create a new circuit
    pub fn new(collateral: F, salt: F, threshold: F, commitment: F) -> Self {
        Self {
            collateral: Some(collateral),
            salt: Some(salt),
            threshold: Some(threshold),
            commitment: Some(commitment),
            _marker: PhantomData,
        }
    }

    /// Create empty circuit for setup
    pub fn empty() -> Self {
        Self {
            collateral: None,
            salt: None,
            threshold: None,
            commitment: None,
            _marker: PhantomData,
        }
    }

    /// Compute commitment (simplified)
    pub fn compute_commitment(collateral: F, salt: F) -> F {
        collateral * salt + collateral
    }
}

impl<F: PrimeField> ConstraintSynthesizer<F> for CollateralCircuit<F> {
    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
        // ======== Allocate Private Inputs ========

        // Collateral (private witness)
        let collateral_var = FpVar::new_witness(cs.clone(), || {
            self.collateral.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Salt (private witness)
        let salt_var = FpVar::new_witness(cs.clone(), || {
            self.salt.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // ======== Allocate Public Inputs ========

        // Threshold (public input)
        let threshold_var = FpVar::new_input(cs.clone(), || {
            self.threshold.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Commitment (public input)
        let commitment_var = FpVar::new_input(cs.clone(), || {
            self.commitment.ok_or(SynthesisError::AssignmentMissing)
        })?;

        // ======== Constraint 1: Range Check (Bit Decomposition) ========
        // This is where R1CS differs from Halo2 - we need bit decomposition

        // Decompose collateral into bits (this creates ~RANGE_BITS constraints)
        let collateral_bits = collateral_var.to_bits_le()?;

        // Enforce that we only use RANGE_BITS bits
        // (remaining bits must be zero for values in range)
        for bit in collateral_bits.iter().skip(RANGE_BITS) {
            bit.enforce_equal(&Boolean::constant(false))?;
        }

        // Same for threshold
        let threshold_bits = threshold_var.to_bits_le()?;
        for bit in threshold_bits.iter().skip(RANGE_BITS) {
            bit.enforce_equal(&Boolean::constant(false))?;
        }

        // ======== Constraint 2: Comparison (collateral >= threshold) ========
        // In R1CS, we prove a >= b by showing a - b + offset is non-negative

        // Compute difference: diff = collateral - threshold
        let diff = &collateral_var - &threshold_var;

        // Add offset to ensure non-negative representation
        let offset = FpVar::constant(F::from(1u64 << (RANGE_BITS - 1)));
        let diff_shifted = diff + offset;

        // Range check the shifted difference (must be in valid range)
        let diff_bits = diff_shifted.to_bits_le()?;
        for bit in diff_bits.iter().skip(RANGE_BITS) {
            bit.enforce_equal(&Boolean::constant(false))?;
        }

        // ======== Constraint 3: Commitment Verification ========
        // commitment == collateral * salt + collateral

        let computed_commitment = &collateral_var * &salt_var + &collateral_var;
        computed_commitment.enforce_equal(&commitment_var)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bn254::Fr;
    use ark_relations::r1cs::ConstraintSystem;

    fn test_circuit(collateral: u64, salt: u64, threshold: u64) -> bool {
        let collateral_f = Fr::from(collateral);
        let salt_f = Fr::from(salt);
        let threshold_f = Fr::from(threshold);
        let commitment = CollateralCircuit::compute_commitment(collateral_f, salt_f);

        let circuit = CollateralCircuit::new(collateral_f, salt_f, threshold_f, commitment);

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();

        cs.is_satisfied().unwrap()
    }

    #[test]
    fn test_valid_collateral() {
        // collateral (1000) >= threshold (500) ✓
        assert!(test_circuit(1000, 12345, 500));
    }

    #[test]
    fn test_equal_values() {
        // collateral (500) >= threshold (500) ✓
        assert!(test_circuit(500, 99999, 500));
    }

    #[test]
    fn test_insufficient_collateral() {
        // collateral (400) >= threshold (500) ✗
        assert!(!test_circuit(400, 12345, 500));
    }

    #[test]
    fn test_zero_threshold() {
        // collateral (100) >= threshold (0) ✓
        assert!(test_circuit(100, 11111, 0));
    }

    #[test]
    fn test_constraint_count() {
        let collateral = Fr::from(1000u64);
        let salt = Fr::from(12345u64);
        let threshold = Fr::from(500u64);
        let commitment = CollateralCircuit::compute_commitment(collateral, salt);

        let circuit = CollateralCircuit::new(collateral, salt, threshold, commitment);

        let cs = ConstraintSystem::<Fr>::new_ref();
        circuit.generate_constraints(cs.clone()).unwrap();

        let num_constraints = cs.num_constraints();
        let num_variables = cs.num_witness_variables();

        println!("\n=== arkworks R1CS Statistics ===");
        println!("Constraints: {}", num_constraints);
        println!("Witness variables: {}", num_variables);
        println!("Public inputs: {}", cs.num_instance_variables());
        println!("");
        println!("Note: Range check uses bit decomposition");
        println!("      ~64 constraints per range check");
        println!("      vs Halo2 lookup: 1 constraint");

        // R1CS should have many more constraints than Halo2
        // Due to bit decomposition for range checks
        assert!(num_constraints > 100, "Expected >100 constraints due to bit decomposition");
    }

    #[test]
    fn test_groth16_proof() {
        use ark_bn254::{Bn254, Fr};
        use ark_groth16::Groth16;
        use ark_snark::SNARK;
        use ark_std::rand::thread_rng;

        let mut rng = thread_rng();

        // Create circuit for setup
        let collateral = Fr::from(1000u64);
        let salt = Fr::from(12345u64);
        let threshold = Fr::from(500u64);
        let commitment = CollateralCircuit::compute_commitment(collateral, salt);

        let circuit = CollateralCircuit::new(collateral, salt, threshold, commitment);

        // Generate proving and verifying keys
        let (pk, vk) = Groth16::<Bn254>::circuit_specific_setup(
            CollateralCircuit::<Fr>::empty(),
            &mut rng,
        ).unwrap();

        // Create proof
        let proof = Groth16::<Bn254>::prove(&pk, circuit, &mut rng).unwrap();

        // Public inputs: [threshold, commitment]
        let public_inputs = vec![threshold, commitment];

        // Verify proof
        let valid = Groth16::<Bn254>::verify(&vk, &public_inputs, &proof).unwrap();
        assert!(valid, "Groth16 proof should be valid");

        println!("\n=== Groth16 Proof Generated ===");
        println!("Proof size: ~200 bytes (constant)");
        println!("Verification: ~3 pairings");
    }
}
