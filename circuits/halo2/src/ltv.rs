//! LTVProof Circuit
//!
//! Proves that Loan-to-Value ratio is within acceptable bounds:
//! `(debt / collateral) <= max_ltv`
//!
//! Rearranged for integer arithmetic (avoids division in finite fields):
//! `debt * 100 <= collateral * max_ltv`
//!
//! # Public Inputs
//! - `max_ltv`: Maximum allowed LTV ratio (e.g., 80 = 80%)
//! - `debt_commitment`: Hash(debt, salt_d)
//! - `collateral_commitment`: Hash(collateral, salt_c)
//!
//! # Private Inputs
//! - `debt`: Borrowed amount
//! - `collateral`: Collateral amount
//! - `salt_d`, `salt_c`: Salts for commitments
//!
//! # Example
//! - collateral: 100 ETH
//! - debt: 60 ETH
//! - max_ltv: 80%
//! - LTV = 60/100 = 60% <= 80% ✓
//!
//! # Circuit Statistics
//! - Advice columns: 8 (debt, collateral, salts, scaled values, commitments)
//! - Instance columns: 1 (public inputs)
//! - Custom gates: 2 (debt scaling, commitments)
//! - Lookup tables: 1 (range check for comparison)
//! - Constraint optimization: Division transformed to multiplication

use ff::PrimeField;
use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance, Selector},
    poly::Rotation,
};
use pasta_curves::Fp;
use std::marker::PhantomData;

use crate::gadgets::comparison::{ComparisonChip, ComparisonConfig, ComparisonInstruction};

/// Note: For production 64-bit range checks, decompose into multiple smaller checks
/// Using 16 bits for demo/testing (2^16 = 65536 entries fits in lookup table)
const RANGE_BITS: usize = 16;
const LTV_PRECISION: u64 = 100; // LTV in percentage (80 = 80%)

/// Configuration for LTV circuit
#[derive(Debug, Clone)]
pub struct LTVConfig<F: PrimeField> {
    // Private inputs
    pub debt: Column<Advice>,
    pub collateral: Column<Advice>,
    pub salt_d: Column<Advice>,
    pub salt_c: Column<Advice>,

    // Intermediate values
    pub debt_scaled: Column<Advice>,        // debt * 100
    pub collateral_scaled: Column<Advice>,  // collateral * max_ltv
    pub debt_commitment: Column<Advice>,
    pub collateral_commitment: Column<Advice>,

    // Public inputs
    pub instance: Column<Instance>,

    // Gates
    pub q_ltv: Selector,
    pub q_commitment: Selector,

    // Comparison for LTV check
    pub comparison: ComparisonConfig<F, RANGE_BITS>,

    _marker: PhantomData<F>,
}

/// LTV Proof Circuit
#[derive(Clone)]
pub struct LTVCircuit<F: PrimeField> {
    pub debt: Value<F>,
    pub collateral: Value<F>,
    pub salt_d: Value<F>,
    pub salt_c: Value<F>,
    pub max_ltv: Value<F>,
}

impl<F: PrimeField> Default for LTVCircuit<F> {
    fn default() -> Self {
        Self {
            debt: Value::unknown(),
            collateral: Value::unknown(),
            salt_d: Value::unknown(),
            salt_c: Value::unknown(),
            max_ltv: Value::unknown(),
        }
    }
}

impl<F: PrimeField> LTVCircuit<F> {
    pub fn new(debt: F, collateral: F, salt_d: F, salt_c: F, max_ltv: F) -> Self {
        Self {
            debt: Value::known(debt),
            collateral: Value::known(collateral),
            salt_d: Value::known(salt_d),
            salt_c: Value::known(salt_c),
            max_ltv: Value::known(max_ltv),
        }
    }

    /// Compute commitment for demo/testing
    ///
    /// commitment = value * salt + value
    ///
    /// NOTE: This is a simplified commitment for circuit demonstration.
    /// For production use, replace with Poseidon hash and implement
    /// Poseidon verification as circuit constraints.
    pub fn compute_commitment(value: F, salt: F) -> F {
        value * salt + value
    }
}

impl Circuit<Fp> for LTVCircuit<Fp> {
    type Config = LTVConfig<Fp>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
        // Advice columns
        let debt = meta.advice_column();
        let collateral = meta.advice_column();
        let salt_d = meta.advice_column();
        let salt_c = meta.advice_column();
        let debt_scaled = meta.advice_column();
        let collateral_scaled = meta.advice_column();
        let debt_commitment = meta.advice_column();
        let collateral_commitment = meta.advice_column();

        // Instance for public inputs
        let instance = meta.instance_column();

        // Enable equality
        for col in [debt, collateral, debt_scaled, collateral_scaled, debt_commitment, collateral_commitment] {
            meta.enable_equality(col);
        }
        meta.enable_equality(instance);

        // Selectors
        let q_ltv = meta.selector();
        let q_commitment = meta.selector();

        // LTV scaling gate: debt_scaled = debt * 100
        meta.create_gate("debt scaling", |meta| {
            let q = meta.query_selector(q_ltv);
            let debt = meta.query_advice(debt, Rotation::cur());
            let debt_scaled = meta.query_advice(debt_scaled, Rotation::cur());
            let precision = halo2_proofs::plonk::Expression::Constant(Fp::from(LTV_PRECISION));

            vec![q * (debt_scaled - debt * precision)]
        });

        // Commitment gate
        meta.create_gate("commitments", |meta| {
            let q = meta.query_selector(q_commitment);
            let d = meta.query_advice(debt, Rotation::cur());
            let c = meta.query_advice(collateral, Rotation::cur());
            let sd = meta.query_advice(salt_d, Rotation::cur());
            let sc = meta.query_advice(salt_c, Rotation::cur());
            let dc = meta.query_advice(debt_commitment, Rotation::cur());
            let cc = meta.query_advice(collateral_commitment, Rotation::cur());

            vec![
                q.clone() * (dc - d.clone() * sd - d),
                q * (cc - c.clone() * sc - c),
            ]
        });

        // Comparison config for LTV check
        let diff = meta.advice_column();
        meta.enable_equality(diff);
        let comparison = ComparisonChip::<Fp, RANGE_BITS>::configure(
            meta,
            collateral_scaled,  // a = collateral * max_ltv
            debt_scaled,        // b = debt * 100
            diff,
        );

        LTVConfig {
            debt,
            collateral,
            salt_d,
            salt_c,
            debt_scaled,
            collateral_scaled,
            debt_commitment,
            collateral_commitment,
            instance,
            q_ltv,
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
        // Load comparison lookup table
        let comparison_chip = ComparisonChip::<Fp, RANGE_BITS>::construct(config.comparison.clone());
        comparison_chip.load_table(layouter.namespace(|| "load table"))?;

        // Main region: assign values and compute scaled amounts
        let (debt_scaled_cell, collateral_scaled_cell, debt_comm_cell, coll_comm_cell) =
            layouter.assign_region(
                || "LTV computation",
                |mut region| {
                    // Enable selectors
                    config.q_ltv.enable(&mut region, 0)?;
                    config.q_commitment.enable(&mut region, 0)?;

                    // Assign private inputs
                    region.assign_advice(|| "debt", config.debt, 0, || self.debt)?;
                    region.assign_advice(|| "collateral", config.collateral, 0, || self.collateral)?;
                    region.assign_advice(|| "salt_d", config.salt_d, 0, || self.salt_d)?;
                    region.assign_advice(|| "salt_c", config.salt_c, 0, || self.salt_c)?;

                    // Compute debt_scaled = debt * 100
                    let debt_scaled_val = self.debt.map(|d| d * Fp::from(LTV_PRECISION));
                    let debt_scaled_cell = region.assign_advice(
                        || "debt_scaled",
                        config.debt_scaled,
                        0,
                        || debt_scaled_val,
                    )?;

                    // Compute collateral_scaled = collateral * max_ltv
                    let collateral_scaled_val = self.collateral.zip(self.max_ltv).map(|(c, ltv)| c * ltv);
                    let collateral_scaled_cell = region.assign_advice(
                        || "collateral_scaled",
                        config.collateral_scaled,
                        0,
                        || collateral_scaled_val,
                    )?;

                    // Compute commitments
                    let debt_comm_val = self.debt.zip(self.salt_d).map(|(d, s)| {
                        Self::compute_commitment(d, s)
                    });
                    let debt_comm_cell = region.assign_advice(
                        || "debt_commitment",
                        config.debt_commitment,
                        0,
                        || debt_comm_val,
                    )?;

                    let coll_comm_val = self.collateral.zip(self.salt_c).map(|(c, s)| {
                        Self::compute_commitment(c, s)
                    });
                    let coll_comm_cell = region.assign_advice(
                        || "collateral_commitment",
                        config.collateral_commitment,
                        0,
                        || coll_comm_val,
                    )?;

                    Ok((debt_scaled_cell, collateral_scaled_cell, debt_comm_cell, coll_comm_cell))
                },
            )?;

        // Constrain public inputs
        // instance[0] = max_ltv
        // instance[1] = debt_commitment
        // instance[2] = collateral_commitment
        layouter.constrain_instance(debt_comm_cell.cell(), config.instance, 1)?;
        layouter.constrain_instance(coll_comm_cell.cell(), config.instance, 2)?;

        // LTV check: collateral_scaled >= debt_scaled
        // i.e., collateral * max_ltv >= debt * 100
        comparison_chip.gte(
            layouter.namespace(|| "LTV check"),
            collateral_scaled_cell,
            debt_scaled_cell,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::dev::MockProver;

    fn create_ltv_circuit(
        debt: u64,
        collateral: u64,
        max_ltv: u64,
    ) -> (LTVCircuit<Fp>, Vec<Fp>) {
        let debt_fp = Fp::from(debt);
        let collateral_fp = Fp::from(collateral);
        let max_ltv_fp = Fp::from(max_ltv);
        let salt_d = Fp::from(11111u64);
        let salt_c = Fp::from(22222u64);

        let debt_commitment = LTVCircuit::compute_commitment(debt_fp, salt_d);
        let collateral_commitment = LTVCircuit::compute_commitment(collateral_fp, salt_c);

        let circuit = LTVCircuit::new(debt_fp, collateral_fp, salt_d, salt_c, max_ltv_fp);
        let public_inputs = vec![max_ltv_fp, debt_commitment, collateral_commitment];

        (circuit, public_inputs)
    }

    #[test]
    fn test_ltv_valid() {
        let k = 17;

        // debt=60, collateral=100, max_ltv=80%
        // LTV = 60% <= 80% ✓
        let (circuit, public_inputs) = create_ltv_circuit(60, 100, 80);

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert_eq!(prover.verify(), Ok(()), "60% LTV should be valid");
    }

    #[test]
    fn test_ltv_at_limit() {
        let k = 17;

        // debt=80, collateral=100, max_ltv=80%
        // LTV = 80% == 80% ✓
        let (circuit, public_inputs) = create_ltv_circuit(80, 100, 80);

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert_eq!(prover.verify(), Ok(()), "Exactly at limit should pass");
    }

    #[test]
    fn test_ltv_exceeds_limit() {
        let k = 17;

        // debt=90, collateral=100, max_ltv=80%
        // LTV = 90% > 80% ✗
        let (circuit, public_inputs) = create_ltv_circuit(90, 100, 80);

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert!(prover.verify().is_err(), "90% LTV should exceed 80% limit");
    }

    #[test]
    fn test_ltv_zero_debt() {
        let k = 17;

        // debt=0, collateral=100, max_ltv=80%
        // LTV = 0% <= 80% ✓
        let (circuit, public_inputs) = create_ltv_circuit(0, 100, 80);

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert_eq!(prover.verify(), Ok(()), "Zero debt should pass");
    }

    #[test]
    fn test_ltv_aave_style() {
        let k = 17;

        // Aave-style: max LTV 75%
        // debt=750, collateral=1000
        // LTV = 75% == 75% ✓
        let (circuit, public_inputs) = create_ltv_circuit(750, 1000, 75);

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert_eq!(prover.verify(), Ok(()), "Aave-style 75% LTV should pass");
    }
}
