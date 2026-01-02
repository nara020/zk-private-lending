//! Range Check Gadget using Lookup Tables
//!
//! Efficiently proves value is within [0, 2^BITS) using Halo2 lookup tables.
//! This is much more efficient than R1CS bit decomposition:
//! - Halo2 lookup: 1 constraint
//! - R1CS bit decomposition: ~BITS constraints
//!
//! # Example
//! ```ignore
//! // Check that value is in range [0, 256)
//! range_check_chip.check(layouter, value, 8)?;
//! ```

use ff::PrimeField;
use halo2_proofs::{
    circuit::{AssignedCell, Layouter, Value},
    plonk::{Advice, Column, ConstraintSystem, Error, Selector, TableColumn},
    poly::Rotation,
};
use std::marker::PhantomData;

/// Configuration for the range check chip
#[derive(Debug, Clone)]
pub struct RangeCheckConfig<F: PrimeField, const BITS: usize> {
    /// Advice column for the value to check
    pub value: Column<Advice>,
    /// Selector to enable the lookup
    pub q_lookup: Selector,
    /// Table column containing valid range values
    pub table: TableColumn,
    _marker: PhantomData<F>,
}

/// Instructions for the range check chip
pub trait RangeCheckInstruction<F: PrimeField> {
    /// Check that value is within [0, 2^bits)
    fn check(
        &self,
        layouter: impl Layouter<F>,
        value: AssignedCell<F, F>,
        bits: usize,
    ) -> Result<(), Error>;
}

/// Range check chip using lookup tables
#[derive(Debug, Clone)]
pub struct RangeCheckChip<F: PrimeField, const BITS: usize> {
    config: RangeCheckConfig<F, BITS>,
}

impl<F: PrimeField, const BITS: usize> RangeCheckChip<F, BITS> {
    /// Create a new range check chip
    pub fn construct(config: RangeCheckConfig<F, BITS>) -> Self {
        Self { config }
    }

    /// Configure the range check chip
    ///
    /// This sets up:
    /// 1. A lookup table with values [0, 2^BITS)
    /// 2. A lookup argument that checks value is in the table
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        value: Column<Advice>,
    ) -> RangeCheckConfig<F, BITS> {
        let q_lookup = meta.complex_selector();
        let table = meta.lookup_table_column();

        // Configure the lookup: value must exist in table
        meta.lookup("range check", |meta| {
            let q = meta.query_selector(q_lookup);
            let v = meta.query_advice(value, Rotation::cur());

            // When q_lookup is enabled, v must be in the table
            vec![(q * v, table)]
        });

        RangeCheckConfig {
            value,
            q_lookup,
            table,
            _marker: PhantomData,
        }
    }

    /// Load the lookup table with values [0, 2^BITS)
    pub fn load_table(&self, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        let table_size = 1 << BITS; // 2^BITS

        layouter.assign_table(
            || "range check table",
            |mut table| {
                for i in 0..table_size {
                    table.assign_cell(
                        || format!("table[{}]", i),
                        self.config.table,
                        i,
                        || Value::known(F::from(i as u64)),
                    )?;
                }
                Ok(())
            },
        )
    }
}

impl<F: PrimeField, const BITS: usize> RangeCheckInstruction<F> for RangeCheckChip<F, BITS> {
    fn check(
        &self,
        mut layouter: impl Layouter<F>,
        value: AssignedCell<F, F>,
        _bits: usize,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "range check",
            |mut region| {
                // Enable the lookup selector
                self.config.q_lookup.enable(&mut region, 0)?;

                // Copy the value to this region
                value.copy_advice(|| "value", &mut region, self.config.value, 0)?;

                Ok(())
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::{
        circuit::SimpleFloorPlanner,
        dev::MockProver,
        plonk::Circuit,
    };
    use pasta_curves::Fp;

    #[derive(Default, Clone)]
    struct RangeCheckTestCircuit<const BITS: usize> {
        value: Value<Fp>,
    }

    impl<const BITS: usize> Circuit<Fp> for RangeCheckTestCircuit<BITS> {
        type Config = RangeCheckConfig<Fp, BITS>;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
            let value = meta.advice_column();
            meta.enable_equality(value);
            RangeCheckChip::<Fp, BITS>::configure(meta, value)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<Fp>,
        ) -> Result<(), Error> {
            let chip = RangeCheckChip::<Fp, BITS>::construct(config.clone());

            // Load the lookup table
            chip.load_table(layouter.namespace(|| "load table"))?;

            // Assign the value
            let value_cell = layouter.assign_region(
                || "assign value",
                |mut region| {
                    region.assign_advice(|| "value", config.value, 0, || self.value)
                },
            )?;

            // Check the range
            chip.check(layouter.namespace(|| "range check"), value_cell, BITS)?;

            Ok(())
        }
    }

    #[test]
    fn test_range_check_valid() {
        let k = 9; // 2^9 = 512 rows
        const BITS: usize = 8; // Range [0, 256)

        // Test valid values
        for value in [0u64, 1, 127, 255] {
            let circuit = RangeCheckTestCircuit::<BITS> {
                value: Value::known(Fp::from(value)),
            };

            let prover = MockProver::run(k, &circuit, vec![]).unwrap();
            assert_eq!(prover.verify(), Ok(()), "Failed for value {}", value);
        }
    }

    #[test]
    fn test_range_check_invalid() {
        let k = 9;
        const BITS: usize = 8;

        // Value 256 is out of range [0, 256)
        let circuit = RangeCheckTestCircuit::<BITS> {
            value: Value::known(Fp::from(256u64)),
        };

        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert!(prover.verify().is_err(), "Should fail for value 256");
    }
}
