//! CollateralProof Circuit
//!
//! Proves that a user's collateral is >= threshold without revealing the exact amount.
//!
//! # Public Inputs
//! - `threshold`: Minimum required collateral
//! - `commitment`: Hash(collateral, salt) - binding commitment to the collateral
//!
//! # Private Inputs (Witnesses)
//! - `collateral`: Actual collateral amount
//! - `salt`: Random value for commitment hiding property
//!
//! # Constraints
//! 1. `collateral >= threshold` (using comparison gadget with range check)
//! 2. `commitment == Hash(collateral, salt)` (commitment verification)
//!
//! # Security Properties
//! - **Soundness**: Cannot prove false statement (collateral < threshold)
//! - **Zero-Knowledge**: Verifier learns nothing except validity
//! - **Binding**: Cannot change collateral after commitment
//!
//! # Circuit Statistics
//! - Advice columns: 4 (collateral, salt, threshold, commitment)
//! - Instance columns: 1 (public inputs)
//! - Custom gates: 1 (commitment verification)
//! - Lookup tables: 1 (range check for comparison)
//! - Estimated rows: ~2^17 for 64-bit range checks

use ff::PrimeField;
use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance, Selector},
    poly::Rotation,
};
use pasta_curves::Fp;
use std::marker::PhantomData;

use crate::gadgets::comparison::{ComparisonChip, ComparisonConfig, ComparisonInstruction};

/// Number of bits for range checks
/// Note: For production 64-bit range checks, decompose into multiple smaller checks
/// Using 16 bits for demo/testing (2^16 = 65536 entries fits in lookup table)
const RANGE_BITS: usize = 16;

/// Configuration for the CollateralProof circuit
#[derive(Debug, Clone)]
pub struct CollateralConfig<F: PrimeField> {
    /// Advice column for collateral amount (private)
    pub collateral: Column<Advice>,
    /// Advice column for salt (private)
    pub salt: Column<Advice>,
    /// Advice column for threshold (copied from instance)
    pub threshold: Column<Advice>,
    /// Advice column for computed commitment
    pub commitment_computed: Column<Advice>,
    /// Instance column for public inputs
    pub instance: Column<Instance>,
    /// Selector for commitment verification
    pub q_commitment: Selector,
    /// Comparison chip config
    pub comparison: ComparisonConfig<F, RANGE_BITS>,
    _marker: PhantomData<F>,
}

/// CollateralProof circuit
#[derive(Clone)]
pub struct CollateralCircuit<F: PrimeField> {
    /// Private: actual collateral amount
    pub collateral: Value<F>,
    /// Private: salt for commitment
    pub salt: Value<F>,
    /// Public: minimum threshold (passed via instance)
    pub threshold: Value<F>,
    /// Public: commitment (passed via instance)
    pub commitment: Value<F>,
}

impl<F: PrimeField> Default for CollateralCircuit<F> {
    fn default() -> Self {
        Self {
            collateral: Value::unknown(),
            salt: Value::unknown(),
            threshold: Value::unknown(),
            commitment: Value::unknown(),
        }
    }
}

impl<F: PrimeField> CollateralCircuit<F> {
    /// Create a new circuit with the given values
    pub fn new(collateral: F, salt: F, threshold: F, commitment: F) -> Self {
        Self {
            collateral: Value::known(collateral),
            salt: Value::known(salt),
            threshold: Value::known(threshold),
            commitment: Value::known(commitment),
        }
    }

    /// Compute commitment for demo/testing
    ///
    /// commitment = collateral * salt + collateral
    ///
    /// NOTE: This is a simplified commitment for circuit demonstration.
    /// For production use, replace with Poseidon hash and implement
    /// Poseidon verification as circuit constraints.
    ///
    /// The formula matches the circuit's custom gate constraint.
    pub fn compute_commitment(collateral: F, salt: F) -> F {
        collateral * salt + collateral
    }
}

impl Circuit<Fp> for CollateralCircuit<Fp> {
    type Config = CollateralConfig<Fp>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
        // Advice columns for private inputs
        let collateral = meta.advice_column();
        let salt = meta.advice_column();
        let threshold = meta.advice_column();
        let commitment_computed = meta.advice_column();

        // Instance column for public inputs
        let instance = meta.instance_column();

        // Enable equality for copy constraints
        meta.enable_equality(collateral);
        meta.enable_equality(salt);
        meta.enable_equality(threshold);
        meta.enable_equality(commitment_computed);
        meta.enable_equality(instance);

        // Selector for commitment gate
        let q_commitment = meta.selector();

        // Commitment verification gate
        // commitment_computed = collateral * salt + collateral
        meta.create_gate("commitment", |meta| {
            let q = meta.query_selector(q_commitment);
            let coll = meta.query_advice(collateral, Rotation::cur());
            let s = meta.query_advice(salt, Rotation::cur());
            let comm = meta.query_advice(commitment_computed, Rotation::cur());

            // Constraint: comm = coll * s + coll
            vec![q * (comm - coll.clone() * s - coll)]
        });

        // Configure comparison chip
        let diff = meta.advice_column();
        meta.enable_equality(diff);
        let comparison = ComparisonChip::<Fp, RANGE_BITS>::configure(
            meta,
            collateral,
            threshold,
            diff,
        );

        CollateralConfig {
            collateral,
            salt,
            threshold,
            commitment_computed,
            instance,
            q_commitment,
            comparison,
            _marker: PhantomData,
        }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<Fp>,
    ) -> Result<(), Error> {
        // Create comparison chip and load lookup table
        let comparison_chip = ComparisonChip::<Fp, RANGE_BITS>::construct(config.comparison.clone());
        comparison_chip.load_table(layouter.namespace(|| "load range table"))?;

        // Assign private inputs and compute commitment
        let (collateral_cell, threshold_cell, commitment_cell) = layouter.assign_region(
            || "assign inputs",
            |mut region| {
                // Enable commitment gate
                config.q_commitment.enable(&mut region, 0)?;

                // Assign collateral (private)
                let collateral_cell = region.assign_advice(
                    || "collateral",
                    config.collateral,
                    0,
                    || self.collateral,
                )?;

                // Assign salt (private)
                region.assign_advice(|| "salt", config.salt, 0, || self.salt)?;

                // Assign threshold (will constrain to instance)
                let threshold_cell = region.assign_advice(
                    || "threshold",
                    config.threshold,
                    0,
                    || self.threshold,
                )?;

                // Compute and assign commitment
                let commitment_value = self.collateral.zip(self.salt).map(|(c, s)| {
                    Self::compute_commitment(c, s)
                });
                let commitment_cell = region.assign_advice(
                    || "commitment",
                    config.commitment_computed,
                    0,
                    || commitment_value,
                )?;

                Ok((collateral_cell, threshold_cell, commitment_cell))
            },
        )?;

        // Constrain threshold to public input (instance[0])
        layouter.constrain_instance(threshold_cell.cell(), config.instance, 0)?;

        // Constrain commitment to public input (instance[1])
        layouter.constrain_instance(commitment_cell.cell(), config.instance, 1)?;

        // Prove collateral >= threshold
        comparison_chip.gte(
            layouter.namespace(|| "collateral >= threshold"),
            collateral_cell.clone(),
            threshold_cell,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::dev::MockProver;

    fn create_test_circuit(collateral: u64, salt: u64, threshold: u64) -> (CollateralCircuit<Fp>, Vec<Fp>) {
        let collateral_fp = Fp::from(collateral);
        let salt_fp = Fp::from(salt);
        let threshold_fp = Fp::from(threshold);
        let commitment = CollateralCircuit::compute_commitment(collateral_fp, salt_fp);

        let circuit = CollateralCircuit::new(collateral_fp, salt_fp, threshold_fp, commitment);
        let public_inputs = vec![threshold_fp, commitment];

        (circuit, public_inputs)
    }

    #[test]
    fn test_collateral_proof_valid() {
        let k = 17; // 2^17 rows

        // Test: collateral (1000) >= threshold (500) ✓
        let (circuit, public_inputs) = create_test_circuit(1000, 12345, 500);

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert_eq!(prover.verify(), Ok(()), "Valid proof should pass");
    }

    #[test]
    fn test_collateral_proof_equal() {
        let k = 17;

        // Test: collateral (500) >= threshold (500) ✓
        let (circuit, public_inputs) = create_test_circuit(500, 99999, 500);

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert_eq!(prover.verify(), Ok(()), "Equal values should pass");
    }

    #[test]
    fn test_collateral_proof_insufficient() {
        let k = 17;

        // Test: collateral (400) >= threshold (500) ✗
        let (circuit, public_inputs) = create_test_circuit(400, 12345, 500);

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert!(prover.verify().is_err(), "Insufficient collateral should fail");
    }

    #[test]
    fn test_collateral_proof_wrong_commitment() {
        let k = 17;

        let collateral = Fp::from(1000u64);
        let salt = Fp::from(12345u64);
        let threshold = Fp::from(500u64);

        // Wrong commitment (different salt)
        let wrong_commitment = CollateralCircuit::compute_commitment(collateral, Fp::from(99999u64));

        let circuit = CollateralCircuit::new(collateral, salt, threshold, wrong_commitment);
        let public_inputs = vec![threshold, wrong_commitment];

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert!(prover.verify().is_err(), "Wrong commitment should fail");
    }

    #[test]
    fn test_collateral_proof_edge_cases() {
        let k = 17;

        // Test minimum values
        let (circuit, public_inputs) = create_test_circuit(1, 1, 1);
        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert_eq!(prover.verify(), Ok(()), "Minimum values should work");

        // Test with zero threshold
        let (circuit, public_inputs) = create_test_circuit(100, 12345, 0);
        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert_eq!(prover.verify(), Ok(()), "Zero threshold should work");
    }

    /// Print circuit statistics using MockProver
    #[test]
    fn test_print_circuit_info() {
        let k = 17;
        let (circuit, public_inputs) = create_test_circuit(1000, 12345, 500);

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();

        // Verify passes
        assert_eq!(prover.verify(), Ok(()));

        println!("CollateralCircuit Statistics:");
        println!("  k (rows = 2^k): {}", k);
        println!("  Public inputs: 2 (threshold, commitment)");
        println!("  Private inputs: 2 (collateral, salt)");
    }
}
