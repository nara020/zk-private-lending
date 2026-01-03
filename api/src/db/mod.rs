//! Database Module
//!
//! PostgreSQL database layer using SQLx with connection pooling.
//! Provides compile-time query validation and type-safe database operations.

mod models;
mod repository;

pub use models::*;
use anyhow::Result;
use sqlx::{postgres::PgPoolOptions, PgPool};

/// 데이터베이스 연결 및 쿼리 담당
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// 데이터베이스 연결
    ///
    /// # Connection Pool Settings
    ///
    /// - max_connections: 10 (트래픽에 따라 조정)
    /// - min_connections: 1 (idle 시 최소 유지)
    /// - acquire_timeout: 3초 (커넥션 획득 대기)
    pub async fn connect(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(3))
            .connect(database_url)
            .await?;

        Ok(Self { pool })
    }

    /// 마이그레이션 실행
    pub async fn run_migrations(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await?;
        Ok(())
    }

    /// Health check
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 사용자 포지션 조회
    pub async fn get_position(&self, address: &str) -> Result<Option<Position>> {
        let position = sqlx::query_as::<_, Position>(
            r#"
            SELECT
                address,
                has_deposit,
                has_borrow,
                borrowed_amount,
                collateral_commitment,
                debt_commitment,
                updated_at
            FROM positions
            WHERE address = $1
            "#
        )
        .bind(address.to_lowercase())
        .fetch_optional(&self.pool)
        .await?;

        Ok(position)
    }

    /// 포지션 히스토리 조회 (페이지네이션)
    pub async fn get_position_history(
        &self,
        address: &str,
        page: u32,
        limit: u32,
    ) -> Result<(Vec<PositionEvent>, i64)> {
        let offset = page * limit;

        // 이벤트 조회
        let events = sqlx::query_as::<_, PositionEvent>(
            r#"
            SELECT
                event_type,
                amount,
                commitment,
                tx_hash,
                block_number,
                timestamp
            FROM position_events
            WHERE address = $1
            ORDER BY timestamp DESC
            LIMIT $2 OFFSET $3
            "#
        )
        .bind(address.to_lowercase())
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await?;

        // 전체 개수
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM position_events WHERE address = $1"
        )
        .bind(address.to_lowercase())
        .fetch_one(&self.pool)
        .await?;

        Ok((events, count.0))
    }

    /// Proof 생성 로그 저장
    pub async fn log_proof_generation(
        &self,
        proof_type: &str,
        generation_time_ms: u64,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO proof_logs (proof_type, generation_time_ms, created_at)
            VALUES ($1, $2, NOW())
            "#
        )
        .bind(proof_type)
        .bind(generation_time_ms as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 포지션 생성/업데이트 (upsert)
    pub async fn upsert_position(&self, position: &Position) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO positions (
                address, has_deposit, has_borrow, borrowed_amount,
                collateral_commitment, debt_commitment, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (address)
            DO UPDATE SET
                has_deposit = EXCLUDED.has_deposit,
                has_borrow = EXCLUDED.has_borrow,
                borrowed_amount = EXCLUDED.borrowed_amount,
                collateral_commitment = EXCLUDED.collateral_commitment,
                debt_commitment = EXCLUDED.debt_commitment,
                updated_at = NOW()
            "#
        )
        .bind(&position.address)
        .bind(position.has_deposit)
        .bind(position.has_borrow)
        .bind(position.borrowed_amount)
        .bind(&position.collateral_commitment)
        .bind(&position.debt_commitment)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 이벤트 저장
    pub async fn insert_event(&self, address: &str, event: &PositionEvent) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO position_events (
                address, event_type, amount, commitment, tx_hash, block_number, timestamp
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#
        )
        .bind(address.to_lowercase())
        .bind(&event.event_type)
        .bind(event.amount)
        .bind(&event.commitment)
        .bind(&event.tx_hash)
        .bind(event.block_number)
        .bind(event.timestamp)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
