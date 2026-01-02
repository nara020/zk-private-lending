//! Common Types Module
//!
//! 애플리케이션 전반에서 사용되는 공통 타입 정의

use serde::{Deserialize, Serialize};

/// API 응답 래퍼
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// Ethereum 주소 타입
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthAddress(String);

impl EthAddress {
    pub fn new(addr: &str) -> Result<Self, String> {
        let addr = addr.to_lowercase();
        if addr.starts_with("0x") && addr.len() == 42 {
            Ok(Self(addr))
        } else {
            Err("Invalid Ethereum address format".to_string())
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 금액 타입 (오버플로우 방지)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Amount {
    pub value: u128,
    pub decimals: u8,
}

impl Amount {
    pub fn new(value: u128, decimals: u8) -> Self {
        Self { value, decimals }
    }

    /// ETH (18 decimals)
    pub fn eth(value: u128) -> Self {
        Self { value, decimals: 18 }
    }

    /// USDC (6 decimals)
    pub fn usdc(value: u128) -> Self {
        Self { value, decimals: 6 }
    }

    /// 사람이 읽기 쉬운 형태로 변환
    pub fn to_human_readable(&self) -> f64 {
        self.value as f64 / 10f64.powi(self.decimals as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eth_address_valid() {
        let addr = EthAddress::new("0x1234567890123456789012345678901234567890");
        assert!(addr.is_ok());
    }

    #[test]
    fn test_eth_address_invalid() {
        let addr = EthAddress::new("invalid");
        assert!(addr.is_err());
    }

    #[test]
    fn test_amount_human_readable() {
        let eth = Amount::eth(1_500_000_000_000_000_000); // 1.5 ETH
        assert!((eth.to_human_readable() - 1.5).abs() < 0.0001);

        let usdc = Amount::usdc(1_500_000); // 1.5 USDC
        assert!((usdc.to_human_readable() - 1.5).abs() < 0.0001);
    }
}
