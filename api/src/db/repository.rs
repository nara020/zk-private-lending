//! Repository Pattern Implementation
//!
//! Provides trait-based abstraction for data access operations.
//! Enables easy testing with mock implementations.

// 현재는 Database 구조체에 직접 구현
// 향후 Repository trait로 분리 가능

use async_trait::async_trait;
use anyhow::Result;

use super::models::{Position, PositionEvent};

/// Position Repository 인터페이스 (향후 확장용)
#[async_trait]
pub trait PositionRepository: Send + Sync {
    async fn find_by_address(&self, address: &str) -> Result<Option<Position>>;
    async fn save(&self, position: &Position) -> Result<()>;
    async fn find_events(
        &self,
        address: &str,
        page: u32,
        limit: u32,
    ) -> Result<(Vec<PositionEvent>, i64)>;
}

// PostgreSQL 구현은 db/mod.rs의 Database 구조체에 있음
// 테스트용 Mock 구현:

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;
    use chrono::Utc;

    pub struct MockPositionRepository {
        positions: RwLock<HashMap<String, Position>>,
    }

    impl MockPositionRepository {
        pub fn new() -> Self {
            Self {
                positions: RwLock::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl PositionRepository for MockPositionRepository {
        async fn find_by_address(&self, address: &str) -> Result<Option<Position>> {
            let positions = self.positions.read().unwrap();
            Ok(positions.get(address).cloned())
        }

        async fn save(&self, position: &Position) -> Result<()> {
            let mut positions = self.positions.write().unwrap();
            positions.insert(position.address.clone(), position.clone());
            Ok(())
        }

        async fn find_events(
            &self,
            _address: &str,
            _page: u32,
            _limit: u32,
        ) -> Result<(Vec<PositionEvent>, i64)> {
            // Mock: 빈 히스토리 반환
            Ok((vec![], 0))
        }
    }
}
