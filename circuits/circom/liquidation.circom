pragma circom 2.1.0;

include "node_modules/circomlib/circuits/comparators.circom";
include "node_modules/circomlib/circuits/poseidon.circom";

/*
 * LiquidationProof Circuit - Circom DSL Implementation
 *
 * Proves: position is liquidatable (health_factor < 1.0)
 *
 * Public Inputs:
 *   - price: Current ETH/USD price (8 decimals)
 *   - liquidation_threshold: Threshold percentage (e.g., 80 = 80%)
 *   - position_hash: Poseidon(collateral, debt, salt)
 *
 * Private Inputs:
 *   - collateral: Actual collateral amount
 *   - debt: Actual debt amount
 *   - salt: Random value for hiding
 */

template LiquidationProof(BITS) {
    // ========== Signals ==========

    // Private inputs
    signal input collateral;
    signal input debt;
    signal input salt;

    // Public inputs
    signal input price;
    signal input liquidation_threshold;
    signal input position_hash;

    // Output
    signal output valid;

    // ========== Constraint 1: Range Checks ==========

    component collateral_range = LessThan(BITS);
    collateral_range.in[0] <== collateral;
    collateral_range.in[1] <== 1 << BITS;
    collateral_range.out === 1;

    component debt_range = LessThan(BITS);
    debt_range.in[0] <== debt;
    debt_range.in[1] <== 1 << BITS;
    debt_range.out === 1;

    // ========== Constraint 2: Liquidation Check ==========
    // health_factor < 1.0
    // = (collateral * price * liq_threshold) / (debt * 100 * 1e8) < 1.0
    // = collateral * price * liq_threshold < debt * 100 * 1e8
    //
    // For strict inequality (a < b), we use LessThan

    // lhs = collateral * price * liq_threshold
    signal temp1;
    temp1 <== collateral * price;

    signal lhs;
    lhs <== temp1 * liquidation_threshold;

    // rhs = debt * 100 * 1e8 (100 for percentage, 1e8 for price decimals)
    signal temp2;
    temp2 <== debt * 100;

    signal rhs;
    rhs <== temp2 * 100000000;  // 1e8

    // Prove: lhs < rhs (position is liquidatable)
    component liquidation_check = LessThan(BITS + 64);  // Extra bits for large multiplication
    liquidation_check.in[0] <== lhs;
    liquidation_check.in[1] <== rhs;
    liquidation_check.out === 1;

    // ========== Constraint 3: Position Hash Verification ==========
    // position_hash == Poseidon(collateral, debt, salt)

    component hasher = Poseidon(3);
    hasher.inputs[0] <== collateral;
    hasher.inputs[1] <== debt;
    hasher.inputs[2] <== salt;
    position_hash === hasher.out;

    // ========== Output ==========
    valid <== 1;
}

// Main component with 64-bit values
component main {public [price, liquidation_threshold, position_hash]} = LiquidationProof(64);

/*
 * Usage:
 *
 * 1. Compile:
 *    circom liquidation.circom --r1cs --wasm --sym -o build
 *
 * 2. Create input.json:
 *    {
 *      "collateral": "1000000000000000000",  // 1 ETH in wei
 *      "debt": "2000000000",                  // 2000 USDC (6 decimals)
 *      "salt": "99999",
 *      "price": "150000000000",               // $1500 (8 decimals)
 *      "liquidation_threshold": "80",         // 80%
 *      "position_hash": "<poseidon_hash>"
 *    }
 *
 * Example:
 *   collateral = 1 ETH, debt = 2000 USDC, price = $1500, threshold = 80%
 *   health = (1 * 1500e8 * 80) / (2000 * 100 * 1e8)
 *         = 12000e8 / 20000e8
 *         = 0.6 < 1.0
 *   → Liquidatable!
 *
 * 3. Generate witness and proof (same as collateral.circom)
 */
