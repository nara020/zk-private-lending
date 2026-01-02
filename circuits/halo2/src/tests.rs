//! Comprehensive Integration Tests
//!
//! Production-grade test coverage for ZK-Private Lending circuits.
//! Includes validation integration, edge cases, and security tests.

#[cfg(test)]
mod integration_tests {
    use crate::collateral::CollateralCircuit;
    use crate::error::validation::{
        validate_collateral, validate_ltv, validate_liquidation, validate_range, validate_salt,
    };
    use crate::gadgets::poseidon::simple::SimpleCommitmentChip;
    use crate::liquidation::LiquidationCircuit;
    use crate::ltv::LTVCircuit;
    use halo2_proofs::dev::MockProver;
    use pasta_curves::Fp;

    // =============================================================
    // Validation Integration Tests
    // =============================================================

    mod validation_tests {
        use super::*;

        #[test]
        fn test_collateral_validation_before_proof() {
            // Valid case
            assert!(validate_collateral(1000, 500).is_ok());

            // Invalid case - should fail validation before circuit
            let result = validate_collateral(400, 500);
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("Insufficient collateral"));
        }

        #[test]
        fn test_ltv_validation_before_proof() {
            // Valid: 60% LTV with 80% max
            assert!(validate_ltv(60, 100, 80).is_ok());

            // Invalid: 90% LTV with 80% max
            let result = validate_ltv(90, 100, 80);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("LTV"));
        }

        #[test]
        fn test_liquidation_validation_before_proof() {
            // Valid liquidation case (HF < 1.0)
            assert!(validate_liquidation(100, 90, 100, 85).is_ok());

            // Invalid: healthy position (HF > 1.0)
            let result = validate_liquidation(100, 50, 100, 85);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not liquidatable"));
        }

        #[test]
        fn test_salt_validation() {
            assert!(validate_salt(12345).is_ok());
            assert!(validate_salt(1).is_ok());

            let result = validate_salt(0);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("salt"));
        }

        #[test]
        fn test_range_validation() {
            assert!(validate_range(0, "value").is_ok());
            assert!(validate_range(1000000, "value").is_ok());
            // Note: MAX_64BIT = (1u64 << 63) - 1, so values up to that are valid
        }
    }

    // =============================================================
    // Circuit + Validation Integration Tests
    // =============================================================

    mod circuit_validation_integration {
        use super::*;

        #[test]
        fn test_validated_collateral_proof() {
            let k = 17;
            let collateral = 1000u64;
            let threshold = 500u64;
            let salt = 12345u64;

            // Step 1: Validate inputs
            assert!(validate_collateral(collateral, threshold).is_ok());
            assert!(validate_salt(salt).is_ok());

            // Step 2: Create and verify circuit
            let collateral_fp = Fp::from(collateral);
            let salt_fp = Fp::from(salt);
            let threshold_fp = Fp::from(threshold);
            let commitment = CollateralCircuit::compute_commitment(collateral_fp, salt_fp);

            let circuit =
                CollateralCircuit::new(collateral_fp, salt_fp, threshold_fp, commitment);
            let public_inputs = vec![threshold_fp, commitment];

            let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
            assert_eq!(prover.verify(), Ok(()));
        }

        #[test]
        fn test_validated_ltv_proof() {
            let k = 17;
            let debt = 60u64;
            let collateral = 100u64;
            let max_ltv = 80u64;

            // Step 1: Validate
            assert!(validate_ltv(debt, collateral, max_ltv).is_ok());

            // Step 2: Create circuit
            let debt_fp = Fp::from(debt);
            let collateral_fp = Fp::from(collateral);
            let max_ltv_fp = Fp::from(max_ltv);
            let salt_d = Fp::from(11111u64);
            let salt_c = Fp::from(22222u64);

            let debt_commitment = LTVCircuit::compute_commitment(debt_fp, salt_d);
            let collateral_commitment = LTVCircuit::compute_commitment(collateral_fp, salt_c);

            let circuit = LTVCircuit::new(debt_fp, collateral_fp, salt_d, salt_c, max_ltv_fp);
            let public_inputs = vec![max_ltv_fp, debt_commitment, collateral_commitment];

            let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
            assert_eq!(prover.verify(), Ok(()));
        }
    }

    // =============================================================
    // Edge Case Tests
    // =============================================================

    mod edge_cases {
        use super::*;

        #[test]
        fn test_collateral_exactly_at_threshold() {
            let k = 17;

            // collateral == threshold (boundary case)
            let collateral = Fp::from(500u64);
            let salt = Fp::from(99999u64);
            let threshold = Fp::from(500u64);
            let commitment = CollateralCircuit::compute_commitment(collateral, salt);

            let circuit = CollateralCircuit::new(collateral, salt, threshold, commitment);
            let public_inputs = vec![threshold, commitment];

            let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
            assert_eq!(prover.verify(), Ok(()), "Equal values should pass");
        }

        #[test]
        fn test_ltv_exactly_at_limit() {
            let k = 17;

            // debt=80, collateral=100, max_ltv=80% -> exactly at limit
            let debt_fp = Fp::from(80u64);
            let collateral_fp = Fp::from(100u64);
            let max_ltv_fp = Fp::from(80u64);
            let salt_d = Fp::from(11111u64);
            let salt_c = Fp::from(22222u64);

            let debt_commitment = LTVCircuit::compute_commitment(debt_fp, salt_d);
            let collateral_commitment = LTVCircuit::compute_commitment(collateral_fp, salt_c);

            let circuit = LTVCircuit::new(debt_fp, collateral_fp, salt_d, salt_c, max_ltv_fp);
            let public_inputs = vec![max_ltv_fp, debt_commitment, collateral_commitment];

            let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
            assert_eq!(prover.verify(), Ok(()), "Exactly at limit should pass");
        }

        #[test]
        fn test_zero_debt_ltv() {
            let k = 17;

            // Zero debt should always pass LTV check
            let debt_fp = Fp::from(0u64);
            let collateral_fp = Fp::from(100u64);
            let max_ltv_fp = Fp::from(80u64);
            let salt_d = Fp::from(11111u64);
            let salt_c = Fp::from(22222u64);

            let debt_commitment = LTVCircuit::compute_commitment(debt_fp, salt_d);
            let collateral_commitment = LTVCircuit::compute_commitment(collateral_fp, salt_c);

            let circuit = LTVCircuit::new(debt_fp, collateral_fp, salt_d, salt_c, max_ltv_fp);
            let public_inputs = vec![max_ltv_fp, debt_commitment, collateral_commitment];

            let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
            assert_eq!(prover.verify(), Ok(()));
        }

        #[test]
        fn test_minimum_values() {
            let k = 17;

            // Minimum non-zero values
            let collateral = Fp::from(1u64);
            let salt = Fp::from(1u64);
            let threshold = Fp::from(1u64);
            let commitment = CollateralCircuit::compute_commitment(collateral, salt);

            let circuit = CollateralCircuit::new(collateral, salt, threshold, commitment);
            let public_inputs = vec![threshold, commitment];

            let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
            assert_eq!(prover.verify(), Ok(()));
        }

        #[test]
        fn test_large_values() {
            let k = 17;

            // Values within 16-bit range (max 65535 for current range check)
            // Note: For production with 64-bit support, decompose into multiple range checks
            let collateral = Fp::from(60000u64);
            let salt = Fp::from(12345u64);
            let threshold = Fp::from(30000u64);
            let commitment = CollateralCircuit::compute_commitment(collateral, salt);

            let circuit = CollateralCircuit::new(collateral, salt, threshold, commitment);
            let public_inputs = vec![threshold, commitment];

            let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
            assert_eq!(prover.verify(), Ok(()));
        }
    }

    // =============================================================
    // Security Tests (Negative Tests)
    // =============================================================

    mod security_tests {
        use super::*;

        #[test]
        fn test_cannot_prove_insufficient_collateral() {
            let k = 17;

            // Attempt to prove collateral=400 >= threshold=500 (FALSE)
            let collateral = Fp::from(400u64);
            let salt = Fp::from(12345u64);
            let threshold = Fp::from(500u64);
            let commitment = CollateralCircuit::compute_commitment(collateral, salt);

            let circuit = CollateralCircuit::new(collateral, salt, threshold, commitment);
            let public_inputs = vec![threshold, commitment];

            let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
            assert!(
                prover.verify().is_err(),
                "Should NOT be able to prove false statement"
            );
        }

        #[test]
        fn test_cannot_fake_commitment() {
            let k = 17;

            let collateral = Fp::from(1000u64);
            let salt = Fp::from(12345u64);
            let threshold = Fp::from(500u64);

            // Fake commitment with different salt
            let fake_commitment =
                CollateralCircuit::compute_commitment(collateral, Fp::from(99999u64));

            let circuit = CollateralCircuit::new(collateral, salt, threshold, fake_commitment);
            let public_inputs = vec![threshold, fake_commitment];

            let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
            assert!(
                prover.verify().is_err(),
                "Fake commitment should fail verification"
            );
        }

        #[test]
        fn test_cannot_exceed_ltv() {
            let k = 17;

            // debt=90, collateral=100, max_ltv=80%
            // LTV = 90% > 80% (SHOULD FAIL)
            let debt_fp = Fp::from(90u64);
            let collateral_fp = Fp::from(100u64);
            let max_ltv_fp = Fp::from(80u64);
            let salt_d = Fp::from(11111u64);
            let salt_c = Fp::from(22222u64);

            let debt_commitment = LTVCircuit::compute_commitment(debt_fp, salt_d);
            let collateral_commitment = LTVCircuit::compute_commitment(collateral_fp, salt_c);

            let circuit = LTVCircuit::new(debt_fp, collateral_fp, salt_d, salt_c, max_ltv_fp);
            let public_inputs = vec![max_ltv_fp, debt_commitment, collateral_commitment];

            let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
            assert!(
                prover.verify().is_err(),
                "Should NOT be able to exceed LTV limit"
            );
        }

        #[test]
        fn test_cannot_liquidate_healthy_position() {
            let k = 17;

            // Healthy position: HF > 1.0
            // collateral=100, price=100, liq_threshold=85, debt=50
            // HF = (100 * 100 * 85) / (50 * 10000) = 850000 / 500000 = 1.7 > 1
            let collateral_fp = Fp::from(100u64);
            let debt_fp = Fp::from(50u64);
            let salt = Fp::from(99999u64);
            let price_fp = Fp::from(100u64);
            let lt_fp = Fp::from(85u64);

            let position_hash =
                LiquidationCircuit::compute_position_hash(collateral_fp, debt_fp, salt);

            let circuit =
                LiquidationCircuit::new(collateral_fp, debt_fp, salt, price_fp, lt_fp);
            let public_inputs = vec![price_fp, lt_fp, position_hash];

            let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
            assert!(
                prover.verify().is_err(),
                "Healthy position should NOT be liquidatable"
            );
        }
    }

    // =============================================================
    // Commitment Function Tests
    // =============================================================

    mod commitment_tests {
        use super::*;

        #[test]
        fn test_simple_commitment_deterministic() {
            let a = Fp::from(1000u64);
            let b = Fp::from(12345u64);

            let c1 = SimpleCommitmentChip::<Fp>::compute_commitment(a, b);
            let c2 = SimpleCommitmentChip::<Fp>::compute_commitment(a, b);

            assert_eq!(c1, c2, "Same inputs should produce same commitment");
        }

        #[test]
        fn test_simple_commitment_different_inputs() {
            let a1 = Fp::from(1000u64);
            let a2 = Fp::from(1001u64);
            let b = Fp::from(12345u64);

            let c1 = SimpleCommitmentChip::<Fp>::compute_commitment(a1, b);
            let c2 = SimpleCommitmentChip::<Fp>::compute_commitment(a2, b);

            assert_ne!(c1, c2, "Different inputs should produce different commitments");
        }

        #[test]
        fn test_commitment_formula() {
            let a = Fp::from(100u64);
            let b = Fp::from(200u64);

            // commitment = a * b + a + b
            let expected = a * b + a + b;
            let actual = SimpleCommitmentChip::<Fp>::compute_commitment(a, b);

            assert_eq!(actual, expected);
        }
    }

    // =============================================================
    // DeFi Scenario Tests
    // =============================================================

    mod defi_scenarios {
        use super::*;

        #[test]
        fn test_aave_style_lending() {
            let k = 17;

            // Aave typical parameters:
            // - Max LTV: 75%
            // - Liquidation threshold: 85%

            // User deposits 1000 ETH, borrows 700 ETH worth
            // LTV = 700/1000 = 70% < 75% max
            let debt_fp = Fp::from(700u64);
            let collateral_fp = Fp::from(1000u64);
            let max_ltv_fp = Fp::from(75u64);
            let salt_d = Fp::from(11111u64);
            let salt_c = Fp::from(22222u64);

            let debt_commitment = LTVCircuit::compute_commitment(debt_fp, salt_d);
            let collateral_commitment = LTVCircuit::compute_commitment(collateral_fp, salt_c);

            let circuit = LTVCircuit::new(debt_fp, collateral_fp, salt_d, salt_c, max_ltv_fp);
            let public_inputs = vec![max_ltv_fp, debt_commitment, collateral_commitment];

            let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
            assert_eq!(prover.verify(), Ok(()), "Aave-style 70% LTV should pass");
        }

        #[test]
        fn test_price_crash_liquidation() {
            let k = 17;

            // Scenario: Price drops, position becomes liquidatable
            // Using values that fit in 16-bit range
            // collateral=10, price=8 (crashed), debt=70, liq_threshold=85
            // collateral_value = 10 * 8 * 85 = 6800
            // debt_scaled = 70 * 100 = 7000
            // HF = 6800 / 7000 = 0.97 < 1 ✓

            let collateral_fp = Fp::from(10u64);
            let debt_fp = Fp::from(70u64);
            let salt = Fp::from(99999u64);
            let price_fp = Fp::from(8u64); // Price crashed
            let lt_fp = Fp::from(85u64);

            let position_hash =
                LiquidationCircuit::compute_position_hash(collateral_fp, debt_fp, salt);

            let circuit =
                LiquidationCircuit::new(collateral_fp, debt_fp, salt, price_fp, lt_fp);
            let public_inputs = vec![price_fp, lt_fp, position_hash];

            let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
            assert_eq!(
                prover.verify(),
                Ok(()),
                "Price crash should trigger liquidation"
            );
        }

        #[test]
        fn test_compound_style_overcollateralized() {
            let k = 17;

            // Compound style: heavily overcollateralized
            // collateral=1000, debt=200
            // LTV = 20% << 80% max
            let debt_fp = Fp::from(200u64);
            let collateral_fp = Fp::from(1000u64);
            let max_ltv_fp = Fp::from(80u64);
            let salt_d = Fp::from(11111u64);
            let salt_c = Fp::from(22222u64);

            let debt_commitment = LTVCircuit::compute_commitment(debt_fp, salt_d);
            let collateral_commitment = LTVCircuit::compute_commitment(collateral_fp, salt_c);

            let circuit = LTVCircuit::new(debt_fp, collateral_fp, salt_d, salt_c, max_ltv_fp);
            let public_inputs = vec![max_ltv_fp, debt_commitment, collateral_commitment];

            let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
            assert_eq!(prover.verify(), Ok(()));
        }
    }
}
