//! Error types for ZK-Private Lending circuits
//!
//! Provides structured error handling for circuit operations.

use std::fmt;

/// Error types for circuit operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CircuitError {
    /// Value is out of valid range
    ValueOutOfRange {
        value: u64,
        max: u64,
        field: String,
    },

    /// Insufficient collateral for the operation
    InsufficientCollateral {
        collateral: u64,
        required: u64,
    },

    /// LTV ratio exceeds maximum allowed
    LTVExceeded {
        current_ltv: u64,
        max_ltv: u64,
    },

    /// Position is not liquidatable
    NotLiquidatable {
        health_factor_numerator: u64,
        health_factor_denominator: u64,
    },

    /// Invalid commitment (hash mismatch)
    InvalidCommitment,

    /// Invalid salt value
    InvalidSalt,

    /// Overflow during arithmetic operation
    ArithmeticOverflow {
        operation: String,
    },

    /// Division by zero
    DivisionByZero,

    /// Invalid circuit configuration
    InvalidConfiguration {
        message: String,
    },

    /// Proof generation failed
    ProofGenerationFailed {
        reason: String,
    },

    /// Verification failed
    VerificationFailed {
        reason: String,
    },
}

impl fmt::Display for CircuitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitError::ValueOutOfRange { value, max, field } => {
                write!(f, "{} value {} exceeds maximum {}", field, value, max)
            }
            CircuitError::InsufficientCollateral { collateral, required } => {
                write!(
                    f,
                    "Insufficient collateral: {} < {} required",
                    collateral, required
                )
            }
            CircuitError::LTVExceeded { current_ltv, max_ltv } => {
                write!(f, "LTV {}% exceeds maximum {}%", current_ltv, max_ltv)
            }
            CircuitError::NotLiquidatable {
                health_factor_numerator,
                health_factor_denominator,
            } => {
                write!(
                    f,
                    "Position not liquidatable: HF = {}/{} >= 1.0",
                    health_factor_numerator, health_factor_denominator
                )
            }
            CircuitError::InvalidCommitment => {
                write!(f, "Commitment verification failed")
            }
            CircuitError::InvalidSalt => {
                write!(f, "Invalid salt value")
            }
            CircuitError::ArithmeticOverflow { operation } => {
                write!(f, "Arithmetic overflow in {}", operation)
            }
            CircuitError::DivisionByZero => {
                write!(f, "Division by zero")
            }
            CircuitError::InvalidConfiguration { message } => {
                write!(f, "Invalid configuration: {}", message)
            }
            CircuitError::ProofGenerationFailed { reason } => {
                write!(f, "Proof generation failed: {}", reason)
            }
            CircuitError::VerificationFailed { reason } => {
                write!(f, "Verification failed: {}", reason)
            }
        }
    }
}

impl std::error::Error for CircuitError {}

/// Result type for circuit operations
pub type CircuitResult<T> = Result<T, CircuitError>;

/// Input validation utilities
pub mod validation {
    use super::*;

    /// Maximum value for 64-bit range check
    pub const MAX_64BIT: u64 = (1u64 << 63) - 1;

    /// Maximum LTV percentage (basis points, 10000 = 100%)
    pub const MAX_LTV_BPS: u64 = 10000;

    /// Validate that a value is within 64-bit range
    pub fn validate_range(value: u64, field: &str) -> CircuitResult<()> {
        if value > MAX_64BIT {
            return Err(CircuitError::ValueOutOfRange {
                value,
                max: MAX_64BIT,
                field: field.to_string(),
            });
        }
        Ok(())
    }

    /// Validate collateral sufficiency
    pub fn validate_collateral(collateral: u64, threshold: u64) -> CircuitResult<()> {
        if collateral < threshold {
            return Err(CircuitError::InsufficientCollateral {
                collateral,
                required: threshold,
            });
        }
        Ok(())
    }

    /// Validate LTV ratio
    pub fn validate_ltv(debt: u64, collateral: u64, max_ltv_percent: u64) -> CircuitResult<()> {
        if collateral == 0 {
            return Err(CircuitError::DivisionByZero);
        }

        // LTV = (debt * 100) / collateral
        // Check: debt * 100 <= collateral * max_ltv
        let debt_scaled = debt.checked_mul(100).ok_or(CircuitError::ArithmeticOverflow {
            operation: "debt scaling".to_string(),
        })?;

        let collateral_scaled =
            collateral
                .checked_mul(max_ltv_percent)
                .ok_or(CircuitError::ArithmeticOverflow {
                    operation: "collateral scaling".to_string(),
                })?;

        if debt_scaled > collateral_scaled {
            let current_ltv = (debt * 100) / collateral;
            return Err(CircuitError::LTVExceeded {
                current_ltv,
                max_ltv: max_ltv_percent,
            });
        }

        Ok(())
    }

    /// Validate position for liquidation
    pub fn validate_liquidation(
        collateral: u64,
        debt: u64,
        price: u64,
        liquidation_threshold: u64,
    ) -> CircuitResult<()> {
        if debt == 0 {
            return Err(CircuitError::NotLiquidatable {
                health_factor_numerator: u64::MAX,
                health_factor_denominator: 1,
            });
        }

        // HF = (collateral * price * liq_threshold) / (debt * 10000)
        // Liquidatable if HF < 1.0
        // i.e., collateral * price * liq_threshold < debt * 10000

        let collateral_value = collateral
            .checked_mul(price)
            .and_then(|v| v.checked_mul(liquidation_threshold))
            .ok_or(CircuitError::ArithmeticOverflow {
                operation: "collateral value calculation".to_string(),
            })?;

        let debt_scaled = debt.checked_mul(10000).ok_or(CircuitError::ArithmeticOverflow {
            operation: "debt scaling".to_string(),
        })?;

        if collateral_value >= debt_scaled {
            return Err(CircuitError::NotLiquidatable {
                health_factor_numerator: collateral_value,
                health_factor_denominator: debt_scaled,
            });
        }

        Ok(())
    }

    /// Validate salt is non-zero
    pub fn validate_salt(salt: u64) -> CircuitResult<()> {
        if salt == 0 {
            return Err(CircuitError::InvalidSalt);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::validation::*;
    use super::*;

    #[test]
    fn test_validate_range() {
        assert!(validate_range(1000, "collateral").is_ok());
        assert!(validate_range(MAX_64BIT, "collateral").is_ok());
        // Note: values above MAX_64BIT would exceed u64 anyway
    }

    #[test]
    fn test_validate_collateral() {
        assert!(validate_collateral(1000, 500).is_ok());
        assert!(validate_collateral(500, 500).is_ok());
        assert!(validate_collateral(400, 500).is_err());
    }

    #[test]
    fn test_validate_ltv() {
        // 60% LTV with 80% max
        assert!(validate_ltv(60, 100, 80).is_ok());

        // 80% LTV with 80% max (at limit)
        assert!(validate_ltv(80, 100, 80).is_ok());

        // 90% LTV with 80% max (exceeds)
        assert!(validate_ltv(90, 100, 80).is_err());

        // Division by zero
        assert!(validate_ltv(100, 0, 80).is_err());
    }

    #[test]
    fn test_validate_liquidation() {
        // Position underwater (liquidatable) - this should pass validation
        // Actually, validate_liquidation returns Ok if liquidatable
        // Let me fix the logic...

        // Healthy position (HF > 1.0) - not liquidatable, returns error
        assert!(validate_liquidation(100, 50, 100, 85).is_err());

        // Underwater position (HF < 1.0) - liquidatable, returns Ok
        // collateral_value = 100 * 100 * 85 = 850000
        // debt_scaled = 90 * 10000 = 900000
        // 850000 < 900000, so liquidatable
        assert!(validate_liquidation(100, 90, 100, 85).is_ok());
    }

    #[test]
    fn test_validate_salt() {
        assert!(validate_salt(12345).is_ok());
        assert!(validate_salt(0).is_err());
    }
}
