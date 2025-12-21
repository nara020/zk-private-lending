//! WASM bindings for ZK Private Lending circuits
//!
//! This module provides JavaScript/TypeScript bindings for the Halo2 circuits.
//!
//! ## Usage in JavaScript
//! ```javascript
//! import init, { generate_collateral_proof, verify_proof } from 'zk-private-lending-circuits';
//!
//! await init();
//! const proof = generate_collateral_proof(amount, salt, commitment);
//! const isValid = verify_proof(proof, publicInputs);
//! ```

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use crate::circuits::{CollateralCircuit, LTVCircuit, LiquidationCircuit};

/// Initialize WASM module with panic hook for better error messages
#[cfg(feature = "wasm")]
#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

/// Generate a collateral proof
///
/// # Arguments
/// * `amount` - Collateral amount as string (to handle large numbers)
/// * `salt` - Random salt as hex string
/// * `commitment` - Expected commitment as hex string
///
/// # Returns
/// Proof bytes as Uint8Array
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn generate_collateral_proof(
    amount: &str,
    salt: &str,
    commitment: &str,
) -> Result<Vec<u8>, JsError> {
    use pasta_curves::Fp;
    use std::str::FromStr;

    let amount_value = Fp::from_str(amount)
        .map_err(|_| JsError::new("Invalid amount"))?;
    let salt_value = Fp::from_str(salt)
        .map_err(|_| JsError::new("Invalid salt"))?;
    let commitment_value = Fp::from_str(commitment)
        .map_err(|_| JsError::new("Invalid commitment"))?;

    // Create circuit and generate proof
    let circuit = CollateralCircuit::new(amount_value, salt_value, commitment_value);

    // Note: Actual proof generation requires setup params
    // This is a placeholder - in production, load params from file
    let proof_bytes = circuit.to_proof_bytes()
        .map_err(|e| JsError::new(&format!("Proof generation failed: {:?}", e)))?;

    Ok(proof_bytes)
}

/// Generate an LTV proof
///
/// # Arguments
/// * `collateral_amount` - Collateral amount as string
/// * `collateral_salt` - Collateral salt as hex string
/// * `borrow_amount` - Borrow amount as string
/// * `eth_price` - ETH price in USD (8 decimals) as string
/// * `max_ltv` - Maximum LTV ratio (e.g., "75" for 75%)
///
/// # Returns
/// Proof bytes as Uint8Array
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn generate_ltv_proof(
    collateral_amount: &str,
    collateral_salt: &str,
    borrow_amount: &str,
    eth_price: &str,
    max_ltv: &str,
) -> Result<Vec<u8>, JsError> {
    use pasta_curves::Fp;
    use std::str::FromStr;

    let collateral = Fp::from_str(collateral_amount)
        .map_err(|_| JsError::new("Invalid collateral amount"))?;
    let salt = Fp::from_str(collateral_salt)
        .map_err(|_| JsError::new("Invalid salt"))?;
    let borrow = Fp::from_str(borrow_amount)
        .map_err(|_| JsError::new("Invalid borrow amount"))?;
    let price = Fp::from_str(eth_price)
        .map_err(|_| JsError::new("Invalid ETH price"))?;
    let ltv = Fp::from_str(max_ltv)
        .map_err(|_| JsError::new("Invalid max LTV"))?;

    let circuit = LTVCircuit::new(collateral, salt, borrow, price, ltv);

    let proof_bytes = circuit.to_proof_bytes()
        .map_err(|e| JsError::new(&format!("Proof generation failed: {:?}", e)))?;

    Ok(proof_bytes)
}

/// Generate a liquidation proof
///
/// # Arguments
/// * `collateral_amount` - Collateral amount as string
/// * `collateral_salt` - Collateral salt as hex string
/// * `debt_amount` - Total debt amount as string
/// * `eth_price` - ETH price in USD (8 decimals) as string
/// * `liquidation_threshold` - Liquidation threshold (e.g., "80" for 80%)
///
/// # Returns
/// Proof bytes as Uint8Array
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn generate_liquidation_proof(
    collateral_amount: &str,
    collateral_salt: &str,
    debt_amount: &str,
    eth_price: &str,
    liquidation_threshold: &str,
) -> Result<Vec<u8>, JsError> {
    use pasta_curves::Fp;
    use std::str::FromStr;

    let collateral = Fp::from_str(collateral_amount)
        .map_err(|_| JsError::new("Invalid collateral amount"))?;
    let salt = Fp::from_str(collateral_salt)
        .map_err(|_| JsError::new("Invalid salt"))?;
    let debt = Fp::from_str(debt_amount)
        .map_err(|_| JsError::new("Invalid debt amount"))?;
    let price = Fp::from_str(eth_price)
        .map_err(|_| JsError::new("Invalid ETH price"))?;
    let threshold = Fp::from_str(liquidation_threshold)
        .map_err(|_| JsError::new("Invalid liquidation threshold"))?;

    let circuit = LiquidationCircuit::new(collateral, salt, debt, price, threshold);

    let proof_bytes = circuit.to_proof_bytes()
        .map_err(|e| JsError::new(&format!("Proof generation failed: {:?}", e)))?;

    Ok(proof_bytes)
}

/// Compute Poseidon hash commitment
///
/// # Arguments
/// * `amount` - Amount as string
/// * `salt` - Random salt as hex string
///
/// # Returns
/// Commitment hash as hex string
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn compute_commitment(amount: &str, salt: &str) -> Result<String, JsError> {
    use pasta_curves::Fp;
    use std::str::FromStr;
    use crate::gadgets::poseidon::PoseidonHash;

    let amount_value = Fp::from_str(amount)
        .map_err(|_| JsError::new("Invalid amount"))?;
    let salt_value = Fp::from_str(salt)
        .map_err(|_| JsError::new("Invalid salt"))?;

    let commitment = PoseidonHash::hash(&[amount_value, salt_value]);

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
