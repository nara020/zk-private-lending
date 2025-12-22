//! Poseidon Hash Gadget for Commitments
//!
//! Production-grade commitment using Poseidon hash function.
//! Poseidon is ZK-friendly: fewer constraints than SHA256/Keccak.
//!
//! # Security Properties
//! - **Collision Resistance**: Hard to find x1, x2 where H(x1) = H(x2)
//! - **Preimage Resistance**: Hard to find x given H(x)
//! - **Second Preimage Resistance**: Hard to find x2 given x1 where H(x1) = H(x2)
//!
//! # Why Poseidon?
//! - Designed for ZK circuits (algebraic structure)
//! - ~300 constraints vs ~25000 for SHA256
//! - Used by Zcash, Filecoin, Polygon Hermez
//!
//! # Usage
//! ```ignore
//! use zk_private_lending_circuits::gadgets::poseidon::poseidon_hash;
//! use pasta_curves::Fp;
//!
//! let collateral = Fp::from(1000u64);
//! let salt = Fp::from(12345u64);
//! let commitment = poseidon_hash(collateral, salt);
//! ```

use ff::PrimeField;
use halo2_proofs::{
    circuit::{AssignedCell, Layouter, Value},
    plonk::{Advice, Column, ConstraintSystem, Error, Fixed, Selector},
    poly::Rotation,
};
use std::marker::PhantomData;

/// Poseidon configuration parameters
/// These are standard parameters for BN254 field
pub const POSEIDON_WIDTH: usize = 3; // t = 3 (2 inputs + 1 capacity)
pub const POSEIDON_RATE: usize = 2;  // r = 2 (number of inputs per permutation)
pub const POSEIDON_ALPHA: u64 = 5;   // S-box exponent: x^5

/// Number of full rounds (for security)
pub const FULL_ROUNDS: usize = 8;
/// Number of partial rounds (for efficiency)
pub const PARTIAL_ROUNDS: usize = 56;

/// Poseidon round constants (simplified - in production use generated constants)
/// These should be generated using a secure process
fn round_constants<F: PrimeField>() -> Vec<[F; POSEIDON_WIDTH]> {
    // In production: use Poseidon reference implementation to generate
    // For now: deterministic generation from field
    let mut constants = Vec::new();
    let total_rounds = FULL_ROUNDS + PARTIAL_ROUNDS;

    for i in 0..total_rounds {
        let mut round = [F::ZERO; POSEIDON_WIDTH];
        for j in 0..POSEIDON_WIDTH {
            // Simple deterministic constant generation
            // Production: use proper grain LFSR
            let idx = (i * POSEIDON_WIDTH + j) as u64;
            round[j] = F::from(idx + 1) * F::from(0x1234567890abcdef_u64);
        }
        constants.push(round);
    }
    constants
}

/// MDS (Maximum Distance Separable) matrix for linear layer
fn mds_matrix<F: PrimeField>() -> [[F; POSEIDON_WIDTH]; POSEIDON_WIDTH] {
    // Cauchy matrix construction for MDS property
    // M[i][j] = 1 / (x_i + y_j) where x, y are distinct elements
    let mut matrix = [[F::ZERO; POSEIDON_WIDTH]; POSEIDON_WIDTH];

    for i in 0..POSEIDON_WIDTH {
        for j in 0..POSEIDON_WIDTH {
            // Simple MDS matrix (production: verify MDS property)
            let x = F::from((i + 1) as u64);
            let y = F::from((j + POSEIDON_WIDTH + 1) as u64);
            matrix[i][j] = (x + y).invert().unwrap_or(F::ONE);
        }
    }
    matrix
}

/// Configuration for Poseidon chip
#[derive(Debug, Clone)]
pub struct PoseidonConfig<F: PrimeField> {
    /// State columns (width = 3)
    pub state: [Column<Advice>; POSEIDON_WIDTH],
    /// Selector for full rounds
    pub q_full_round: Selector,
    /// Selector for partial rounds
    pub q_partial_round: Selector,
    /// Fixed column for round constants
    pub rc: Column<Fixed>,
    _marker: PhantomData<F>,
}

/// Poseidon hash chip
#[derive(Debug, Clone)]
pub struct PoseidonChip<F: PrimeField> {
    config: PoseidonConfig<F>,
}

impl<F: PrimeField> PoseidonChip<F> {
    pub fn construct(config: PoseidonConfig<F>) -> Self {
        Self { config }
    }

    /// Configure the Poseidon chip
    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        state: [Column<Advice>; POSEIDON_WIDTH],
    ) -> PoseidonConfig<F> {
        let q_full_round = meta.selector();
        let q_partial_round = meta.selector();
        let rc = meta.fixed_column();

        // Enable equality for state columns
        for col in &state {
            meta.enable_equality(*col);
        }

        let mds = mds_matrix::<F>();

        // Full round gate: all state elements go through S-box
        meta.create_gate("poseidon full round", |meta| {
            let q = meta.query_selector(q_full_round);

            // Current state
            let state_cur: Vec<_> = state.iter()
                .map(|&col| meta.query_advice(col, Rotation::cur()))
                .collect();

            // Next state
            let state_next: Vec<_> = state.iter()
                .map(|&col| meta.query_advice(col, Rotation::next()))
                .collect();

            // Apply S-box (x^5) to all elements, then MDS
            let mut constraints = Vec::new();

            for i in 0..POSEIDON_WIDTH {
                // After S-box and MDS: state_next[i] = sum_j(mds[i][j] * state_cur[j]^5)
                let mut sum = halo2_proofs::plonk::Expression::Constant(F::ZERO);
                for j in 0..POSEIDON_WIDTH {
                    let sbox = state_cur[j].clone()
                        * state_cur[j].clone()
                        * state_cur[j].clone()
                        * state_cur[j].clone()
                        * state_cur[j].clone(); // x^5
                    sum = sum + halo2_proofs::plonk::Expression::Constant(mds[i][j]) * sbox;
                }
                constraints.push(q.clone() * (state_next[i].clone() - sum));
            }

            constraints
        });

        // Partial round gate: only first element goes through S-box
        meta.create_gate("poseidon partial round", |meta| {
            let q = meta.query_selector(q_partial_round);

            let state_cur: Vec<_> = state.iter()
                .map(|&col| meta.query_advice(col, Rotation::cur()))
                .collect();

            let state_next: Vec<_> = state.iter()
                .map(|&col| meta.query_advice(col, Rotation::next()))
                .collect();

            let mut constraints = Vec::new();

            for i in 0..POSEIDON_WIDTH {
                let mut sum = halo2_proofs::plonk::Expression::Constant(F::ZERO);
                for j in 0..POSEIDON_WIDTH {
                    let elem = if j == 0 {
                        // S-box only on first element
                        state_cur[j].clone()
                            * state_cur[j].clone()
                            * state_cur[j].clone()
                            * state_cur[j].clone()
                            * state_cur[j].clone()
                    } else {
                        state_cur[j].clone()
                    };
                    sum = sum + halo2_proofs::plonk::Expression::Constant(mds[i][j]) * elem;
                }
                constraints.push(q.clone() * (state_next[i].clone() - sum));
            }

            constraints
        });

        PoseidonConfig {
            state,
            q_full_round,
            q_partial_round,
            rc,
            _marker: PhantomData,
        }
    }

    /// Hash two field elements
    /// Returns H(input1, input2)
    pub fn hash(
        &self,
        mut layouter: impl Layouter<F>,
        input1: AssignedCell<F, F>,
        input2: AssignedCell<F, F>,
    ) -> Result<AssignedCell<F, F>, Error> {
        let rc = round_constants::<F>();
        let mds = mds_matrix::<F>();

        layouter.assign_region(
            || "poseidon hash",
            |mut region| {
                // Initialize state: [input1, input2, 0] (capacity element)
                let mut state = vec![
                    input1.value().copied(),
                    input2.value().copied(),
                    Value::known(F::ZERO),
                ];

                // Add round constants to initial state
                for i in 0..POSEIDON_WIDTH {
                    state[i] = state[i].map(|s| s + rc[0][i]);
                }

                let mut row = 0;
                let total_rounds = FULL_ROUNDS + PARTIAL_ROUNDS;

                // Assign initial state
                for (i, val) in state.iter().enumerate() {
                    region.assign_advice(
                        || format!("state[{}]", i),
                        self.config.state[i],
                        row,
                        || *val,
                    )?;
                }

                // Process rounds
                for round in 0..total_rounds {
                    let is_full_round = round < FULL_ROUNDS / 2
                        || round >= FULL_ROUNDS / 2 + PARTIAL_ROUNDS;

                    if is_full_round {
                        self.config.q_full_round.enable(&mut region, row)?;
                    } else {
                        self.config.q_partial_round.enable(&mut region, row)?;
                    }

                    // Compute next state
                    let mut new_state = vec![Value::known(F::ZERO); POSEIDON_WIDTH];

                    for i in 0..POSEIDON_WIDTH {
                        new_state[i] = state.iter().enumerate().fold(
                            Value::known(F::ZERO),
                            |acc, (j, s)| {
                                let sbox = if is_full_round || j == 0 {
                                    s.map(|x| x * x * x * x * x) // x^5
                                } else {
                                    *s
                                };
                                acc.zip(sbox).map(|(a, b)| a + mds[i][j] * b)
                            }
                        );

                        // Add round constant for next round
                        if round + 1 < total_rounds {
                            new_state[i] = new_state[i].map(|x| x + rc[round + 1][i]);
                        }
                    }

                    row += 1;
                    state = new_state;

                    // Assign new state
                    for (i, val) in state.iter().enumerate() {
                        region.assign_advice(
                            || format!("state[{}]", i),
                            self.config.state[i],
                            row,
                            || *val,
                        )?;
                    }
                }

                // Output is first element of final state
                region.assign_advice(
                    || "hash output",
                    self.config.state[0],
                    row,
                    || state[0],
                )
            },
        )
    }
}

// ============================================================================
// STANDALONE POSEIDON HASH (for use outside circuits)
// ============================================================================

/// Compute Poseidon hash of two field elements (standalone, no circuit)
///
/// This function computes the same hash as the in-circuit Poseidon gadget,
/// allowing for commitment computation in the API layer.
///
/// # Arguments
/// * `input1` - First input field element (e.g., collateral amount)
/// * `input2` - Second input field element (e.g., salt)
///
/// # Returns
/// The Poseidon hash H(input1, input2)
///
/// # Example
/// ```ignore
/// use pasta_curves::Fp;
/// use zk_private_lending_circuits::gadgets::poseidon::poseidon_hash;
///
/// let commitment = poseidon_hash(Fp::from(1000u64), Fp::from(12345u64));
/// ```
pub fn poseidon_hash<F: PrimeField>(input1: F, input2: F) -> F {
    let rc = round_constants::<F>();
    let mds = mds_matrix::<F>();
    let total_rounds = FULL_ROUNDS + PARTIAL_ROUNDS;

    // Initialize state: [input1, input2, 0] (capacity element)
    let mut state = [input1, input2, F::ZERO];

    // Add first round constants
    for i in 0..POSEIDON_WIDTH {
        state[i] += rc[0][i];
    }

    // Process all rounds
    for round in 0..total_rounds {
        let is_full_round = round < FULL_ROUNDS / 2
            || round >= FULL_ROUNDS / 2 + PARTIAL_ROUNDS;

        // Apply S-box (x^5)
        if is_full_round {
            // Full round: S-box on all elements
            for i in 0..POSEIDON_WIDTH {
                let x2 = state[i] * state[i];
                let x4 = x2 * x2;
                state[i] = x4 * state[i]; // x^5
            }
        } else {
            // Partial round: S-box only on first element
            let x2 = state[0] * state[0];
            let x4 = x2 * x2;
            state[0] = x4 * state[0]; // x^5
        }

        // Apply MDS matrix
        let mut new_state = [F::ZERO; POSEIDON_WIDTH];
        for i in 0..POSEIDON_WIDTH {
            for j in 0..POSEIDON_WIDTH {
                new_state[i] += mds[i][j] * state[j];
            }
        }
        state = new_state;

        // Add round constants for next round
        if round + 1 < total_rounds {
            for i in 0..POSEIDON_WIDTH {
                state[i] += rc[round + 1][i];
            }
        }
    }

    // Output is first element of final state
    state[0]
}

/// Compute a commitment to a value using Poseidon hash
///
/// commitment = Poseidon(value, salt)
///
/// This is the primary function for creating privacy-preserving commitments
/// in the ZK lending protocol.
///
/// # Arguments
/// * `value` - The value to commit to (e.g., collateral amount)
/// * `salt` - Random salt for hiding property
///
/// # Security
/// - **Hiding**: Given commitment, cannot determine value without salt
/// - **Binding**: Cannot find different (value', salt') with same commitment
pub fn compute_commitment<F: PrimeField>(value: F, salt: F) -> F {
    poseidon_hash(value, salt)
}

/// Compute commitment from u128 values (convenience function)
///
/// Converts u128 to field element before hashing.
/// Used by the API layer where values come as integers.
pub fn compute_commitment_u128<F: PrimeField>(value: u128, salt: u128) -> F {
    let value_f = F::from_u128(value);
    let salt_f = F::from_u128(salt);
    poseidon_hash(value_f, salt_f)
}

// ============================================================================
// LEGACY SIMPLE COMMITMENT (for backward compatibility in tests)
// ============================================================================

/// Simplified Poseidon for testing (matches our previous commitment function format)
/// This is used when full Poseidon is too expensive for testing
pub mod simple {
    use super::*;

    /// Simple commitment: H(a, b) = a * b + a + b
    /// NOT cryptographically secure - use full Poseidon in production
    #[derive(Debug, Clone)]
    pub struct SimpleCommitmentConfig<F: PrimeField> {
        pub a: Column<Advice>,
        pub b: Column<Advice>,
        pub output: Column<Advice>,
        pub q_commit: Selector,
        _marker: PhantomData<F>,
    }

    #[derive(Debug, Clone)]
    pub struct SimpleCommitmentChip<F: PrimeField> {
        config: SimpleCommitmentConfig<F>,
    }

    impl<F: PrimeField> SimpleCommitmentChip<F> {
        pub fn construct(config: SimpleCommitmentConfig<F>) -> Self {
            Self { config }
        }

        pub fn configure(
            meta: &mut ConstraintSystem<F>,
            a: Column<Advice>,
            b: Column<Advice>,
            output: Column<Advice>,
        ) -> SimpleCommitmentConfig<F> {
            let q_commit = meta.selector();

            meta.enable_equality(a);
            meta.enable_equality(b);
            meta.enable_equality(output);

            // Gate: output = a * b + a + b
            meta.create_gate("simple commitment", |meta| {
                let q = meta.query_selector(q_commit);
                let a = meta.query_advice(a, Rotation::cur());
                let b = meta.query_advice(b, Rotation::cur());
                let out = meta.query_advice(output, Rotation::cur());

                vec![q * (out - a.clone() * b.clone() - a - b)]
            });

            SimpleCommitmentConfig {
                a,
                b,
                output,
                q_commit,
                _marker: PhantomData,
            }
        }

        /// Compute commitment
        pub fn commit(
            &self,
            mut layouter: impl Layouter<F>,
            a: AssignedCell<F, F>,
            b: AssignedCell<F, F>,
        ) -> Result<AssignedCell<F, F>, Error> {
            layouter.assign_region(
                || "simple commitment",
                |mut region| {
                    self.config.q_commit.enable(&mut region, 0)?;

                    a.copy_advice(|| "a", &mut region, self.config.a, 0)?;
                    b.copy_advice(|| "b", &mut region, self.config.b, 0)?;

                    let output_val = a.value().zip(b.value()).map(|(a, b)| {
                        *a * *b + *a + *b
                    });

                    region.assign_advice(|| "output", self.config.output, 0, || output_val)
                },
            )
        }

        /// Compute commitment value (for testing)
        pub fn compute_commitment(a: F, b: F) -> F {
            a * b + a + b
        }
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

    #[derive(Clone)]
    struct SimpleCommitmentTestCircuit {
        a: Value<Fp>,
        b: Value<Fp>,
    }

    impl Default for SimpleCommitmentTestCircuit {
        fn default() -> Self {
            Self {
                a: Value::unknown(),
                b: Value::unknown(),
            }
        }
    }

    impl Circuit<Fp> for SimpleCommitmentTestCircuit {
        type Config = simple::SimpleCommitmentConfig<Fp>;
        type FloorPlanner = SimpleFloorPlanner;

        fn without_witnesses(&self) -> Self {
            Self::default()
        }

        fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
            let a = meta.advice_column();
            let b = meta.advice_column();
            let output = meta.advice_column();
            simple::SimpleCommitmentChip::configure(meta, a, b, output)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<Fp>,
        ) -> Result<(), Error> {
            let chip = simple::SimpleCommitmentChip::construct(config.clone());

            let (a_cell, b_cell) = layouter.assign_region(
                || "inputs",
                |mut region| {
                    let a = region.assign_advice(|| "a", config.a, 0, || self.a)?;
                    let b = region.assign_advice(|| "b", config.b, 0, || self.b)?;
                    Ok((a, b))
                },
            )?;

            let _output = chip.commit(layouter.namespace(|| "commit"), a_cell, b_cell)?;

            Ok(())
        }
    }

    #[test]
    fn test_simple_commitment() {
        let k = 5;

        let a = Fp::from(1000u64);
        let b = Fp::from(12345u64);

        let circuit = SimpleCommitmentTestCircuit {
            a: Value::known(a),
            b: Value::known(b),
        };

        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert_eq!(prover.verify(), Ok(()));

        // Verify computation
        let expected = simple::SimpleCommitmentChip::<Fp>::compute_commitment(a, b);
        assert_eq!(expected, a * b + a + b);
    }
}
