//! WASM bindings for ZK Private Lending circuits
//!
//! This module provides JavaScript/TypeScript bindings for the Halo2 circuits.
//!
//! ## Usage in JavaScript
//! ```javascript
//! import init, { compute_commitment, get_circuit_info } from 'zk-private-lending-circuits';
//!
//! await init();
//! const commitment = compute_commitment(amount, salt);
//! console.log(get_circuit_info());
//! ```
//!
//! Note: Full proof generation requires proving keys which are not included in WASM.
//! For production, use the API server for proof generation.

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use crate::{CollateralCircuit, LTVCircuit, LiquidationCircuit};

/// Initialize WASM module with panic hook for better error messages
#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

/// Validate collateral circuit inputs
///
/// # Arguments
/// * `amount` - Collateral amount as string (to handle large numbers)
/// * `salt` - Random salt as string
/// * `threshold` - Minimum required collateral as string
///
/// # Returns
/// True if inputs are valid and would produce a valid proof
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn validate_collateral_inputs(
    amount: &str,
    salt: &str,
    threshold: &str,
) -> Result<bool, JsError> {
    let amount: u64 = amount.parse()
        .map_err(|_| JsError::new("Invalid amount"))?;
    let _salt: u64 = salt.parse()
        .map_err(|_| JsError::new("Invalid salt"))?;
    let threshold: u64 = threshold.parse()
        .map_err(|_| JsError::new("Invalid threshold"))?;

    // Validate that collateral >= threshold
    Ok(amount >= threshold)
}

/// Validate LTV circuit inputs
///
/// # Arguments
/// * `collateral_amount` - Collateral amount as string
/// * `borrow_amount` - Borrow amount as string
/// * `max_ltv` - Maximum LTV ratio (e.g., "75" for 75%)
///
/// # Returns
/// True if LTV is within bounds
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn validate_ltv_inputs(
    collateral_amount: &str,
    borrow_amount: &str,
    max_ltv: &str,
) -> Result<bool, JsError> {
    let collateral: u64 = collateral_amount.parse()
        .map_err(|_| JsError::new("Invalid collateral amount"))?;
    let borrow: u64 = borrow_amount.parse()
        .map_err(|_| JsError::new("Invalid borrow amount"))?;
    let max_ltv: u64 = max_ltv.parse()
        .map_err(|_| JsError::new("Invalid max LTV"))?;

    // Check: borrow * 100 <= collateral * max_ltv
    // This avoids division and works with integers
    let borrow_scaled = borrow * 100;
    let collateral_scaled = collateral * max_ltv;

    Ok(borrow_scaled <= collateral_scaled)
}

/// Check if a position is liquidatable
///
/// # Arguments
/// * `collateral_amount` - Collateral amount as string
/// * `debt_amount` - Total debt amount as string
/// * `eth_price` - ETH price (scaled) as string
/// * `liquidation_threshold` - Liquidation threshold (e.g., "85" for 85%)
///
/// # Returns
/// True if position is liquidatable (health factor < 1.0)
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn is_liquidatable(
    collateral_amount: &str,
    debt_amount: &str,
    eth_price: &str,
    liquidation_threshold: &str,
) -> Result<bool, JsError> {
    let collateral: u64 = collateral_amount.parse()
        .map_err(|_| JsError::new("Invalid collateral amount"))?;
    let debt: u64 = debt_amount.parse()
        .map_err(|_| JsError::new("Invalid debt amount"))?;
    let price: u64 = eth_price.parse()
        .map_err(|_| JsError::new("Invalid ETH price"))?;
    let threshold: u64 = liquidation_threshold.parse()
        .map_err(|_| JsError::new("Invalid liquidation threshold"))?;

    // Use the circuit's helper function
    Ok(LiquidationCircuit::<pasta_curves::Fp>::is_liquidatable(
        collateral, debt, price, threshold
    ))
}

/// Compute commitment hash
///
/// Uses the same commitment formula as the circuits:
/// commitment = amount * salt + amount
///
/// Note: This is a simplified commitment for demonstration.
/// Production would use Poseidon hash.
///
/// # Arguments
/// * `amount` - Amount as string
/// * `salt` - Random salt as string
///
/// # Returns
/// Commitment value as string
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn compute_commitment(amount: &str, salt: &str) -> Result<String, JsError> {
    use pasta_curves::Fp;

    let amount_value: u64 = amount.parse()
        .map_err(|_| JsError::new("Invalid amount"))?;
    let salt_value: u64 = salt.parse()
        .map_err(|_| JsError::new("Invalid salt"))?;

    let amount_fp = Fp::from(amount_value);
    let salt_fp = Fp::from(salt_value);

    // Use the same formula as CollateralCircuit::compute_commitment
    let commitment = CollateralCircuit::compute_commitment(amount_fp, salt_fp);

    Ok(format!("{:?}", commitment))
}

/// Get circuit parameters (for debugging/info)
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn get_circuit_info() -> JsValue {
    use serde_json::json;

    let info = json!({
        "name": "ZK Private Lending Circuits",
        "version": env!("CARGO_PKG_VERSION"),
        "circuits": {
            "collateral": {
                "description": "Proves knowledge of collateral amount matching commitment",
                "public_inputs": ["commitment"],
                "private_inputs": ["amount", "salt"]
            },
            "ltv": {
                "description": "Proves LTV ratio is within bounds without revealing amounts",
                "public_inputs": ["commitment", "max_ltv"],
                "private_inputs": ["collateral", "salt", "borrow_amount", "eth_price"]
            },
            "liquidation": {
                "description": "Proves position is liquidatable (health_factor < 1)",
                "public_inputs": ["commitment", "threshold"],
                "private_inputs": ["collateral", "salt", "debt", "eth_price"]
            }
        },
        "curve": "Pasta (Pallas/Vesta)",
        "proof_system": "Halo2 (PSE fork)"
    });

    JsValue::from_str(&info.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_wasm_bindings_compile() {
        // Placeholder test to ensure WASM bindings compile
        assert!(true);
    }
}
