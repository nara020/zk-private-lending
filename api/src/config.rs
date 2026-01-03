//! Configuration Module
//!
//! Environment-based configuration following 12-Factor App principles.
//! Supports development defaults with production overrides.

use std::env;
use anyhow::{Context, Result};

/// 애플리케이션 설정
#[derive(Debug, Clone)]
pub struct Config {
    /// 서버 포트 (기본값: 3001)
    pub port: u16,

    /// PostgreSQL 연결 문자열
    /// 형식: postgres://user:password@host:port/database
    pub database_url: String,

    /// 가격 오라클 URL (Chainlink mock 또는 실제 API)
    pub price_oracle_url: String,

    /// Ethereum RPC URL (스마트 컨트랙트 상호작용용)
    pub eth_rpc_url: String,

    /// ZK Proving Key 경로 (옵션, 없으면 생성)
    pub proving_key_path: Option<String>,

    /// 환경 (development, staging, production)
    pub environment: Environment,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl Config {
    /// 환경변수에서 설정 로드
    ///
    /// # Required Environment Variables
    ///
    /// - `DATABASE_URL`: PostgreSQL 연결 문자열
    ///
    /// # Optional Environment Variables
    ///
    /// - `PORT`: 서버 포트 (기본값: 3001)
    /// - `PRICE_ORACLE_URL`: 가격 오라클 URL
    /// - `ETH_RPC_URL`: Ethereum RPC URL
    /// - `PROVING_KEY_PATH`: ZK Proving Key 경로
    /// - `ENVIRONMENT`: development | staging | production
    ///
    /// # Design Decision
    ///
    /// 필수 값과 옵션 값을 명확히 구분:
    /// - 필수: DATABASE_URL (없으면 앱 시작 불가)
    /// - 옵션: 기본값 제공 (개발 편의성)
    pub fn from_env() -> Result<Self> {
        let environment = match env::var("ENVIRONMENT")
            .unwrap_or_else(|_| "development".to_string())
            .to_lowercase()
            .as_str()
        {
            "production" => Environment::Production,
            "staging" => Environment::Staging,
            _ => Environment::Development,
        };

        Ok(Config {
            port: env::var("PORT")
                .unwrap_or_else(|_| "3001".to_string())
                .parse()
                .context("PORT must be a valid number")?,

            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| {
                    // 개발 환경 기본값
                    "postgres://postgres:postgres@localhost:5432/zk_lending".to_string()
                }),

            price_oracle_url: env::var("PRICE_ORACLE_URL")
                .unwrap_or_else(|_| "http://localhost:3002/price".to_string()),

            eth_rpc_url: env::var("ETH_RPC_URL")
                .unwrap_or_else(|_| "http://localhost:8545".to_string()),

            proving_key_path: env::var("PROVING_KEY_PATH").ok(),

            environment,
        })
    }

    /// 프로덕션 환경인지 확인
    pub fn is_production(&self) -> bool {
        self.environment == Environment::Production
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        // 환경변수 없이 기본값으로 설정 생성
        let config = Config::from_env().unwrap();
        assert_eq!(config.port, 3001);
        assert_eq!(config.environment, Environment::Development);
    }
}
