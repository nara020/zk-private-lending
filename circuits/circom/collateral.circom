pragma circom 2.1.0;

include "node_modules/circomlib/circuits/comparators.circom";
include "node_modules/circomlib/circuits/poseidon.circom";

/*
 * CollateralProof Circuit - Circom DSL Implementation
 *
 * Same logic as Halo2 and arkworks versions, using Circom DSL.
 *
 * Circom vs Other Stacks:
 * ┌────────────┬─────────────┬──────────────┬────────────┐
 * │ Aspect     │ Circom      │ arkworks     │ Halo2      │
 * ├────────────┼─────────────┼──────────────┼────────────┤
 * │ Language   │ DSL         │ Rust         │ Rust       │
 * │ Learning   │ Easy        │ Medium       │ Hard       │
 * │ Flexibility│ Limited     │ High         │ Very High  │
 * │ Ecosystem  │ Large       │ Medium       │ Growing    │
 * │ Range Check│ LessThan    │ Bit decomp   │ Lookup     │
 * └────────────┴─────────────┴──────────────┴────────────┘
 *
 * Public Inputs:
 *   - threshold: Minimum required collateral
 *   - commitment: Poseidon(collateral, salt)
 *
 * Private Inputs:
 *   - collateral: Actual amount
 *   - salt: Random value for hiding
 */

template CollateralProof(BITS) {
    // ========== Signals ==========

    // Private inputs
    signal input collateral;
    signal input salt;

    // Public inputs
    signal input threshold;
    signal input commitment;

    // Output (for verification)
    signal output valid;

    // ========== Constraint 1: Range Check ==========
    // Ensure collateral is within valid range [0, 2^BITS)
    // Uses LessThan from circomlib (~BITS constraints)

    component collateral_range = LessThan(BITS);
    collateral_range.in[0] <== collateral;
    collateral_range.in[1] <== 1 << BITS;  // 2^BITS
    collateral_range.out === 1;

    component threshold_range = LessThan(BITS);
    threshold_range.in[0] <== threshold;
    threshold_range.in[1] <== 1 << BITS;
    threshold_range.out === 1;

    // ========== Constraint 2: Comparison ==========
    // Prove collateral >= threshold
    // Using GreaterEqThan component

    component comparison = GreaterEqThan(BITS);
    comparison.in[0] <== collateral;
    comparison.in[1] <== threshold;
    comparison.out === 1;

    // ========== Constraint 3: Commitment Verification ==========
    // commitment == Poseidon(collateral, salt)
    // Using Poseidon hash from circomlib

    component hasher = Poseidon(2);
    hasher.inputs[0] <== collateral;
    hasher.inputs[1] <== salt;

    // Verify computed hash matches public commitment
    commitment === hasher.out;

    // ========== Output ==========
    // Always 1 if all constraints pass
    valid <== 1;
}

// Main component with 64-bit values
component main {public [threshold, commitment]} = CollateralProof(64);

/*
 * Usage:
 *
 * 1. Compile:
 *    circom collateral.circom --r1cs --wasm --sym -o build
 *
 * 2. Create input.json:
 *    {
 *      "collateral": "1000",
 *      "salt": "12345",
 *      "threshold": "500",
 *      "commitment": "<poseidon_hash>"
 *    }
 *
 * 3. Generate witness:
 *    node build/collateral_js/generate_witness.js build/collateral_js/collateral.wasm input.json witness.wtns
 *
 * 4. Create proof:
 *    snarkjs groth16 prove collateral_final.zkey witness.wtns proof.json public.json
 *
 * 5. Verify:
 *    snarkjs groth16 verify verification_key.json public.json proof.json
 */
