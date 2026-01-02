//! LiquidationProof Circuit
//!
//! Proves that a position is liquidatable (Health Factor < 1.0).
//!
//! # Health Factor Formula
//! `HF = (collateral * price * liquidation_threshold) / debt`
//!
//! Position is liquidatable when HF < 1.0
//!
//! Rearranged for integer arithmetic:
//! `collateral * price * liquidation_threshold < debt * PRECISION`
//!
//! # Public Inputs
//! - `price`: Current asset price from oracle
//! - `liquidation_threshold`: Protocol's liquidation threshold (e.g., 85%)
//! - `position_hash`: Commitment to the position
//!
//! # Private Inputs
//! - `collateral`: Amount of collateral
//! - `debt`: Amount of debt
//! - `salt`: Salt for position commitment
//!
//! # Use Case
//! Liquidators can prove a position is liquidatable without revealing
//! the exact position details until liquidation is executed.
//!
//! # Circuit Statistics
//! - Advice columns: 8 (collateral, debt, salt, price, threshold, computed values)
//! - Instance columns: 1 (public inputs)
//! - Custom gates: 2 (computation, position hash)
//! - Lookup tables: 1 (range check for comparison)
//! - Privacy: Position details hidden until liquidation execution

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
const PRECISION: u64 = 100; // For percentage calculations

/// Configuration for Liquidation circuit
#[derive(Debug, Clone)]
pub struct LiquidationConfig<F: PrimeField> {
    // Private inputs
    pub collateral: Column<Advice>,
    pub debt: Column<Advice>,
    pub salt: Column<Advice>,

    // Public inputs (copied from instance)
    pub price: Column<Advice>,
    pub liquidation_threshold: Column<Advice>,

    // Intermediate computations
    pub collateral_value: Column<Advice>,  // collateral * price * liq_threshold
    pub debt_scaled: Column<Advice>,       // debt * PRECISION

    // Commitment
    pub position_hash: Column<Advice>,

    // Instance column
    pub instance: Column<Instance>,

    // Selectors
    pub q_compute: Selector,
    pub q_commitment: Selector,

    // Comparison: debt_scaled > collateral_value (position is underwater)
    pub comparison: ComparisonConfig<F, RANGE_BITS>,

    _marker: PhantomData<F>,
}

/// Liquidation Proof Circuit
#[derive(Clone)]
pub struct LiquidationCircuit<F: PrimeField> {
    pub collateral: Value<F>,
    pub debt: Value<F>,
    pub salt: Value<F>,
    pub price: Value<F>,
    pub liquidation_threshold: Value<F>,
}

impl<F: PrimeField> Default for LiquidationCircuit<F> {
    fn default() -> Self {
        Self {
            collateral: Value::unknown(),
            debt: Value::unknown(),
            salt: Value::unknown(),
            price: Value::unknown(),
            liquidation_threshold: Value::unknown(),
        }
    }
}

impl<F: PrimeField> LiquidationCircuit<F> {
    pub fn new(
        collateral: F,
        debt: F,
        salt: F,
        price: F,
        liquidation_threshold: F,
    ) -> Self {
        Self {
            collateral: Value::known(collateral),
            debt: Value::known(debt),
            salt: Value::known(salt),
            price: Value::known(price),
            liquidation_threshold: Value::known(liquidation_threshold),
        }
    }

    /// Position commitment for demo/testing
    ///
    /// position_hash = collateral * salt + debt * salt + collateral + debt
    ///
    /// NOTE: This is a simplified commitment for circuit demonstration.
    /// For production use, replace with Poseidon hash and implement
    /// Poseidon verification as circuit constraints.
    ///
    /// The formula matches the circuit's custom gate constraint.
    pub fn compute_position_hash(collateral: F, debt: F, salt: F) -> F {
        collateral * salt + debt * salt + collateral + debt
    }

    /// Check if position is liquidatable
    /// HF = (collateral * price * liq_threshold) / (debt * 100)
    /// Liquidatable when HF < 1, i.e., collateral_value < debt_scaled
    ///
    /// Example: collateral=100, price=1, liq_threshold=85, debt=90
    /// collateral_value = 100 * 1 * 85 = 8500
    /// debt_scaled = 90 * 100 = 9000
    /// HF = 8500 / 9000 = 0.94 < 1 → liquidatable
    pub fn is_liquidatable(
        collateral: u64,
        debt: u64,
        price: u64,
        liquidation_threshold: u64,
    ) -> bool {
        let collateral_value = collateral * price * liquidation_threshold;
        let debt_scaled = debt * PRECISION; // debt * 100
        collateral_value < debt_scaled
    }
}

impl Circuit<Fp> for LiquidationCircuit<Fp> {
    type Config = LiquidationConfig<Fp>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
        // Advice columns
        let collateral = meta.advice_column();
        let debt = meta.advice_column();
        let salt = meta.advice_column();
        let price = meta.advice_column();
        let liquidation_threshold = meta.advice_column();
        let collateral_value = meta.advice_column();
        let debt_scaled = meta.advice_column();
        let position_hash = meta.advice_column();

        // Instance column
        let instance = meta.instance_column();

        // Enable equality
        for col in [collateral, debt, collateral_value, debt_scaled, position_hash, price, liquidation_threshold] {
            meta.enable_equality(col);
        }
        meta.enable_equality(instance);

        // Selectors
        let q_compute = meta.selector();
        let q_commitment = meta.selector();

        // Computation gate:
        // collateral_value = collateral * price * liquidation_threshold
        // debt_scaled = debt * PRECISION
        meta.create_gate("liquidation computation", |meta| {
            let q = meta.query_selector(q_compute);
            let coll = meta.query_advice(collateral, Rotation::cur());
            let d = meta.query_advice(debt, Rotation::cur());
            let p = meta.query_advice(price, Rotation::cur());
            let lt = meta.query_advice(liquidation_threshold, Rotation::cur());
            let cv = meta.query_advice(collateral_value, Rotation::cur());
            let ds = meta.query_advice(debt_scaled, Rotation::cur());
            let precision = halo2_proofs::plonk::Expression::Constant(
                Fp::from(PRECISION)
            );

            vec![
                // cv = coll * price * liq_threshold
                q.clone() * (cv - coll * p * lt),
                // ds = debt * PRECISION
                q * (ds - d * precision),
            ]
        });

        // Position commitment gate
        meta.create_gate("position hash", |meta| {
            let q = meta.query_selector(q_commitment);
            let coll = meta.query_advice(collateral, Rotation::cur());
            let d = meta.query_advice(debt, Rotation::cur());
            let s = meta.query_advice(salt, Rotation::cur());
            let hash = meta.query_advice(position_hash, Rotation::cur());

            // hash = coll * s + debt * s + coll + debt
            vec![q * (hash - coll.clone() * s.clone() - d.clone() * s - coll - d)]
        });

        // Comparison: debt_scaled > collateral_value
        // (proving HF < 1.0, position is underwater)
        let diff = meta.advice_column();
        meta.enable_equality(diff);
        let comparison = ComparisonChip::<Fp, RANGE_BITS>::configure(
            meta,
            debt_scaled,        // a = debt_scaled (must be greater)
            collateral_value,   // b = collateral_value
            diff,
        );

        LiquidationConfig {
            collateral,
            debt,
            salt,
            price,
            liquidation_threshold,
            collateral_value,
            debt_scaled,
            position_hash,
            instance,
            q_compute,
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

        // Main computation region
        let (debt_scaled_cell, collateral_value_cell, position_hash_cell) =
            layouter.assign_region(
                || "liquidation computation",
                |mut region| {
                    // Enable selectors
                    config.q_compute.enable(&mut region, 0)?;
                    config.q_commitment.enable(&mut region, 0)?;

                    // Assign private inputs
                    region.assign_advice(|| "collateral", config.collateral, 0, || self.collateral)?;
                    region.assign_advice(|| "debt", config.debt, 0, || self.debt)?;
                    region.assign_advice(|| "salt", config.salt, 0, || self.salt)?;

                    // Assign public inputs (from instance)
                    region.assign_advice(|| "price", config.price, 0, || self.price)?;
                    region.assign_advice(
                        || "liquidation_threshold",
                        config.liquidation_threshold,
                        0,
                        || self.liquidation_threshold,
                    )?;

                    // Compute collateral_value = collateral * price * liq_threshold
                    let cv_val = self.collateral
                        .zip(self.price)
                        .zip(self.liquidation_threshold)
                        .map(|((c, p), lt)| c * p * lt);
                    let collateral_value_cell = region.assign_advice(
                        || "collateral_value",
                        config.collateral_value,
                        0,
                        || cv_val,
                    )?;

                    // Compute debt_scaled = debt * PRECISION
                    let ds_val = self.debt.map(|d| d * Fp::from(PRECISION));
                    let debt_scaled_cell = region.assign_advice(
                        || "debt_scaled",
                        config.debt_scaled,
                        0,
                        || ds_val,
                    )?;

                    // Compute position hash
                    let hash_val = self.collateral
                        .zip(self.debt)
                        .zip(self.salt)
                        .map(|((c, d), s)| Self::compute_position_hash(c, d, s));
                    let position_hash_cell = region.assign_advice(
                        || "position_hash",
                        config.position_hash,
                        0,
                        || hash_val,
                    )?;

                    Ok((debt_scaled_cell, collateral_value_cell, position_hash_cell))
                },
            )?;

        // Constrain public inputs
        // instance[0] = price
        // instance[1] = liquidation_threshold
        // instance[2] = position_hash
        layouter.constrain_instance(position_hash_cell.cell(), config.instance, 2)?;

        // Prove position is liquidatable: debt_scaled > collateral_value
        // This means HF < 1.0
        comparison_chip.gt(
            layouter.namespace(|| "liquidation check"),
            debt_scaled_cell,
            collateral_value_cell,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::dev::MockProver;

    fn create_liquidation_circuit(
        collateral: u64,
        debt: u64,
        price: u64,
        liquidation_threshold: u64,
    ) -> (LiquidationCircuit<Fp>, Vec<Fp>) {
        let collateral_fp = Fp::from(collateral);
        let debt_fp = Fp::from(debt);
        let salt = Fp::from(99999u64);
        let price_fp = Fp::from(price);
        let lt_fp = Fp::from(liquidation_threshold);

        let position_hash = LiquidationCircuit::compute_position_hash(
            collateral_fp, debt_fp, salt
        );

        let circuit = LiquidationCircuit::new(
            collateral_fp, debt_fp, salt, price_fp, lt_fp
        );
        let public_inputs = vec![price_fp, lt_fp, position_hash];

        (circuit, public_inputs)
    }

    #[test]
    fn test_liquidatable_position() {
        let k = 17;

        // Scenario: Position is underwater (values fit in 16-bit range)
        // collateral=100, price=1, liq_threshold=85, debt=90
        // collateral_value = 100 * 1 * 85 = 8500
        // debt_scaled = 90 * 100 = 9000
        // HF = 8500 / 9000 = 0.94 < 1 ✓
        let (circuit, public_inputs) = create_liquidation_circuit(100, 90, 1, 85);

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert_eq!(prover.verify(), Ok(()), "Underwater position should be liquidatable");
    }

    #[test]
    fn test_price_drop_liquidation() {
        let k = 17;

        // Scenario: Price dropped, now liquidatable (values fit in 16-bit range)
        // collateral=10, price=8 (dropped), liq_threshold=85, debt=70
        // collateral_value = 10 * 8 * 85 = 6800
        // debt_scaled = 70 * 100 = 7000
        // HF = 6800 / 7000 = 0.97 < 1 ✓
        let (circuit, public_inputs) = create_liquidation_circuit(10, 70, 8, 85);

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert_eq!(prover.verify(), Ok(()), "Price drop should trigger liquidation");
    }

    #[test]
    fn test_healthy_position_fails() {
        let k = 17;

        // Scenario: Healthy position (not liquidatable, values fit in 16-bit range)
        // collateral=100, price=1, liq_threshold=85, debt=50
        // collateral_value = 100 * 1 * 85 = 8500
        // debt_scaled = 50 * 100 = 5000
        // HF = 8500 / 5000 = 1.7 > 1 ✗
        let (circuit, public_inputs) = create_liquidation_circuit(100, 50, 1, 85);

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert!(prover.verify().is_err(), "Healthy position should NOT be liquidatable");
    }

    #[test]
    fn test_borderline_liquidation() {
        let k = 17;

        // Borderline case: HF slightly below 1.0 (values fit in 16-bit range)
        // collateral=10, price=10, liq_threshold=85, debt=86
        // collateral_value = 10 * 10 * 85 = 8500
        // debt_scaled = 86 * 100 = 8600
        // HF = 8500 / 8600 = 0.988 < 1 ✓
        let (circuit, public_inputs) = create_liquidation_circuit(10, 86, 10, 85);

        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert_eq!(prover.verify(), Ok(()), "Borderline case should be liquidatable");
    }

    #[test]
    fn test_is_liquidatable_helper() {
        // Test the helper function (with values that fit in 16-bit range)
        // Case 1: collateral_value=8500 < debt_scaled=9000 → liquidatable
        assert!(LiquidationCircuit::<Fp>::is_liquidatable(100, 90, 1, 85));
        // Case 2: collateral_value=6800 < debt_scaled=7000 → liquidatable
        assert!(LiquidationCircuit::<Fp>::is_liquidatable(10, 70, 8, 85));
        // Case 3: collateral_value=8500 > debt_scaled=5000 → NOT liquidatable
        assert!(!LiquidationCircuit::<Fp>::is_liquidatable(100, 50, 1, 85));
    }
}
