//! Comparison Gadget for Greater-Than-Or-Equal
//!
//! Proves a >= b by showing (a - b) is in range [0, 2^BITS).
//!
//! # Strategy
//! 1. Compute diff = a - b (in finite field)
//! 2. Range check that diff is in [0, 2^BITS)
//! 3. If a >= b, diff is small and in range
//! 4. If a < b, diff wraps to p - (b - a) which is huge, failing range check
//!
//! # Important Constraint
//! Both a and b MUST be in range [0, 2^BITS) for this to work correctly.
//! If a and b can exceed 2^BITS, the caller must ensure they are range-checked
//! before using this comparison.
//!
//! # Example
//! ```ignore
//! // Prove collateral >= threshold
//! comparison_chip.gte(layouter, collateral, threshold)?;
//! ```

use ff::PrimeField;
use halo2_proofs::{
    circuit::{AssignedCell, Layouter},
    plonk::{Advice, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};
use std::marker::PhantomData;

use super::range_check::{RangeCheckChip, RangeCheckConfig, RangeCheckInstruction};

/// Configuration for comparison chip
#[derive(Debug, Clone)]
pub struct ComparisonConfig<F: PrimeField, const BITS: usize> {
    /// Advice column for operand a
    pub a: Column<Advice>,
    /// Advice column for operand b
    pub b: Column<Advice>,
    /// Advice column for the difference
    pub diff: Column<Advice>,
    /// Selector for the comparison gate
    pub q_cmp: Selector,
    /// Range check config for validating difference
    pub range_check: RangeCheckConfig<F, BITS>,
    _marker: PhantomData<F>,
}

/// Instructions for comparison operations
pub trait ComparisonInstruction<F: PrimeField> {
    /// Prove that a >= b
    fn gte(
        &self,
        layouter: impl Layouter<F>,
        a: AssignedCell<F, F>,
        b: AssignedCell<F, F>,
    ) -> Result<(), Error>;

    /// Prove that a > b (strictly greater)
    fn gt(
        &self,
        layouter: impl Layouter<F>,
        a: AssignedCell<F, F>,
        b: AssignedCell<F, F>,
    ) -> Result<(), Error>;
}

/// Comparison chip for >= and > operations
#[derive(Debug, Clone)]
pub struct ComparisonChip<F: PrimeField, const BITS: usize> {
    config: ComparisonConfig<F, BITS>,
}

impl<F: PrimeField, const BITS: usize> ComparisonChip<F, BITS> {
    /// Create a new comparison chip
    pub fn construct(config: ComparisonConfig<F, BITS>) -> Self {
        Self { config }
    }

    /// Configure the comparison chip
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        a: Column<Advice>,
        b: Column<Advice>,
        diff: Column<Advice>,
    ) -> ComparisonConfig<F, BITS> {
        let q_cmp = meta.selector();

        // Configure range check for the difference
        let range_check = RangeCheckChip::<F, BITS>::configure(meta, diff);

        // Custom gate: diff = a - b
        // In finite field: if a >= b, diff is small; if a < b, diff wraps to huge value
        meta.create_gate("comparison", |meta| {
            let q = meta.query_selector(q_cmp);
            let a = meta.query_advice(a, Rotation::cur());
            let b = meta.query_advice(b, Rotation::cur());
            let diff = meta.query_advice(diff, Rotation::cur());

            // Constraint: diff = a - b
            // Rearranged: diff - a + b = 0
            vec![q * (diff - a + b)]
        });

        ComparisonConfig {
            a,
            b,
            diff,
            q_cmp,
            range_check,
            _marker: PhantomData,
        }
    }

    /// Load the range check lookup table
    pub fn load_table(&self, layouter: impl Layouter<F>) -> Result<(), Error> {
        let range_chip = RangeCheckChip::<F, BITS>::construct(self.config.range_check.clone());
        range_chip.load_table(layouter)
    }
}

impl<F: PrimeField, const BITS: usize> ComparisonInstruction<F> for ComparisonChip<F, BITS> {
    fn gte(
        &self,
        mut layouter: impl Layouter<F>,
        a: AssignedCell<F, F>,
        b: AssignedCell<F, F>,
    ) -> Result<(), Error> {
        let diff_cell = layouter.assign_region(
            || "comparison: a >= b",
            |mut region| {
                // Enable the comparison selector
                self.config.q_cmp.enable(&mut region, 0)?;

                // Copy a and b to this region
                a.copy_advice(|| "a", &mut region, self.config.a, 0)?;
                b.copy_advice(|| "b", &mut region, self.config.b, 0)?;

                // Compute and assign diff = a - b
                // In finite field: if a >= b, diff is small (in range)
                // If a < b, diff = p - (b - a) is huge (fails range check)
                let diff_value = a.value().zip(b.value()).map(|(a, b)| {
                    *a - *b
                });

                region.assign_advice(|| "diff", self.config.diff, 0, || diff_value)
            },
        )?;

        // Range check the difference
        let range_chip = RangeCheckChip::<F, BITS>::construct(self.config.range_check.clone());
        range_chip.check(
            layouter.namespace(|| "range check diff"),
            diff_cell,
            BITS,
        )?;

        Ok(())
    }

    fn gt(
        &self,
        mut layouter: impl Layouter<F>,
        a: AssignedCell<F, F>,
        b: AssignedCell<F, F>,
    ) -> Result<(), Error> {
        // For a > b, we prove a >= b + 1
        // This is equivalent to checking a - b - 1 >= 0
        let b_plus_one = layouter.assign_region(
            || "b + 1",
            |mut region| {
                let b_val = b.value().map(|b| *b + F::ONE);
                region.assign_advice(|| "b + 1", self.config.b, 0, || b_val)
            },
        )?;

        self.gte(layouter.namespace(|| "a > b"), a, b_plus_one)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::{
        circuit::{SimpleFloorPlanner, Value},
        dev::MockProver,
        plonk::Circuit,
    };
    use pasta_curves::Fp;

    #[derive(Clone)]
    struct ComparisonTestCircuit<const BITS: usize> {
        a: Value<Fp>,
        b: Value<Fp>,
    }

    impl<const BITS: usize> Default for ComparisonTestCircuit<BITS> {
        fn default() -> Self {
            Self {
                a: Value::unknown(),
                b: Value::unknown(),
            }
        }
    }

    impl<const BITS: usize> Circuit<Fp> for ComparisonTestCircuit<BITS> {
        type Config = ComparisonConfig<Fp, BITS>;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
            let a = meta.advice_column();
            let b = meta.advice_column();
            let diff = meta.advice_column();

            meta.enable_equality(a);
            meta.enable_equality(b);
            meta.enable_equality(diff);

            ComparisonChip::<Fp, BITS>::configure(meta, a, b, diff)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<Fp>,
        ) -> Result<(), Error> {
            let chip = ComparisonChip::<Fp, BITS>::construct(config.clone());

            // Load lookup table
            chip.load_table(layouter.namespace(|| "load table"))?;

            // Assign a and b
            let (a_cell, b_cell) = layouter.assign_region(
                || "assign inputs",
                |mut region| {
                    let a = region.assign_advice(|| "a", config.a, 0, || self.a)?;
                    let b = region.assign_advice(|| "b", config.b, 0, || self.b)?;
                    Ok((a, b))
                },
            )?;

            // Prove a >= b
            chip.gte(layouter.namespace(|| "a >= b"), a_cell, b_cell)?;

            Ok(())
        }
    }

    #[test]
    fn test_comparison_gte_valid() {
        let k = 10;
        const BITS: usize = 8;

        // Test cases where a >= b
        let test_cases = vec![
            (100u64, 50u64),   // a > b
            (100, 100),       // a == b
            (255, 0),         // max >= min
            (1, 1),           // equal
        ];

        for (a, b) in test_cases {
            let circuit = ComparisonTestCircuit::<BITS> {
                a: Value::known(Fp::from(a)),
                b: Value::known(Fp::from(b)),
            };

            let prover = MockProver::run(k, &circuit, vec![]).unwrap();
            assert_eq!(
                prover.verify(),
                Ok(()),
                "Failed for a={}, b={}",
                a,
                b
            );
        }
    }

    #[test]
    fn test_comparison_gte_invalid() {
        let k = 10;
        const BITS: usize = 8;

        // Test case where a < b (should fail)
        let circuit = ComparisonTestCircuit::<BITS> {
            a: Value::known(Fp::from(50u64)),
            b: Value::known(Fp::from(100u64)),
        };

        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert!(prover.verify().is_err(), "Should fail when a < b");
    }
}
