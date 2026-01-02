//! Repository Pattern Implementation
//!
//! # Interview Q&A
//!
//! Q: Repository 패턴이란?
//! A: 데이터 접근 로직을 추상화하는 패턴
//!
//!    장점:
//!    - 비즈니스 로직과 데이터 접근 분리
//!    - 테스트 시 Mock 구현 쉬움
//!    - DB 교체 시 영향 최소화
//!
//!    ```rust
//!    // Service 레이어
//!    let position = repository.find_by_address(&address).await?;
//!
//!    // Repository 인터페이스
//!    trait PositionRepository {
//!        async fn find_by_address(&self, addr: &str) -> Result<Option<Position>>;
//!        async fn save(&self, position: &Position) -> Result<()>;
//!    }
//!
//!    // PostgreSQL 구현
//!    impl PositionRepository for PgPositionRepository { ... }
//!
//!    // 테스트용 Mock
//!    impl PositionRepository for MockPositionRepository { ... }
//!    ```
//!
//! Q: 현재 코드에서 Repository 패턴을 사용하지 않은 이유는?
//! A: MVP 단계에서 오버엔지니어링 방지
//!    - 단일 DB (PostgreSQL)만 사용
//!    - 복잡한 추상화보다 직접 쿼리가 명확
//!    - 필요 시 리팩토링 가능
//!
//!    프로덕션에서는 trait 기반 추상화 권장

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
