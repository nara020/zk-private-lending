pragma circom 2.1.0;

include "node_modules/circomlib/circuits/comparators.circom";
include "node_modules/circomlib/circuits/poseidon.circom";

/*
 * LTVProof Circuit - Circom DSL Implementation
 *
 * Proves: debt/collateral <= max_ltv without revealing amounts
 *
 * Interview Q&A:
 *
 * Q: Circom에서 나눗셈을 어떻게 피하는가?
 * A: 부등식 변환
 *    debt/collateral <= max_ltv/100
 *    → debt * 100 <= collateral * max_ltv
 *
 * Q: Circom의 장점은?
 * A: 빠른 프로토타이핑
 *    - DSL이라 Rust보다 배우기 쉬움
 *    - circomlib에 많은 템플릿 제공
 *    - snarkjs와 연동 편리
 *
 * Public Inputs:
 *   - max_ltv: Maximum LTV percentage (e.g., 75 = 75%)
 *   - collateral_commitment: Poseidon(collateral, collateral_salt)
 *   - debt_commitment: Poseidon(debt, debt_salt)
 *
 * Private Inputs:
 *   - collateral: Actual collateral amount
 *   - collateral_salt: Salt for collateral commitment
 *   - debt: Actual debt amount
 *   - debt_salt: Salt for debt commitment
 */

template LTVProof(BITS) {
    // ========== Signals ==========

    // Private inputs
    signal input collateral;
    signal input collateral_salt;
    signal input debt;
    signal input debt_salt;

    // Public inputs
    signal input max_ltv;
    signal input collateral_commitment;
    signal input debt_commitment;

    // Output
    signal output valid;

    // ========== Constraint 1 & 2: Range Checks ==========

    component collateral_range = LessThan(BITS);
    collateral_range.in[0] <== collateral;
    collateral_range.in[1] <== 1 << BITS;
    collateral_range.out === 1;

    component debt_range = LessThan(BITS);
    debt_range.in[0] <== debt;
    debt_range.in[1] <== 1 << BITS;
    debt_range.out === 1;

    // ========== Constraint 3: LTV Check ==========
    // debt * 100 <= collateral * max_ltv
    //
    // Using GreaterEqThan: collateral * max_ltv >= debt * 100

    signal debt_scaled;
    debt_scaled <== debt * 100;

    signal collateral_scaled;
    collateral_scaled <== collateral * max_ltv;

    component ltv_check = GreaterEqThan(BITS + 8);  // Extra bits for multiplication
    ltv_check.in[0] <== collateral_scaled;
    ltv_check.in[1] <== debt_scaled;
    ltv_check.out === 1;

    // ========== Constraint 4 & 5: Commitment Verification ==========

    // collateral_commitment == Poseidon(collateral, collateral_salt)
    component collateral_hasher = Poseidon(2);
    collateral_hasher.inputs[0] <== collateral;
    collateral_hasher.inputs[1] <== collateral_salt;
    collateral_commitment === collateral_hasher.out;

    // debt_commitment == Poseidon(debt, debt_salt)
    component debt_hasher = Poseidon(2);
    debt_hasher.inputs[0] <== debt;
    debt_hasher.inputs[1] <== debt_salt;
    debt_commitment === debt_hasher.out;

    // ========== Output ==========
    valid <== 1;
}

// Main component with 64-bit values
component main {public [max_ltv, collateral_commitment, debt_commitment]} = LTVProof(64);

/*
 * Usage:
 *
 * 1. Compile:
 *    circom ltv.circom --r1cs --wasm --sym -o build
 *
 * 2. Create input.json:
 *    {
 *      "collateral": "1000000000000000000",
 *      "collateral_salt": "12345",
 *      "debt": "500000000",
 *      "debt_salt": "67890",
 *      "max_ltv": "75",
 *      "collateral_commitment": "<poseidon_hash>",
 *      "debt_commitment": "<poseidon_hash>"
 *    }
 *
 * 3. Generate witness and proof (same as collateral.circom)
 */
