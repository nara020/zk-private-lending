//! Price Oracle Service
//!
//! # Interview Q&A
//!
//! Q: DeFi에서 가격 오라클의 중요성은?
//! A: 모든 DeFi 프로토콜의 핵심 인프라
//!
//!    사용 사례:
//!    - 렌딩: 담보 가치 평가, 청산 트리거
//!    - DEX: 가격 발견, 슬리피지 계산
//!    - 파생상품: 결제 가격 결정
//!
//!    위험성:
//!    - 오라클 조작 → 잘못된 청산 → 자금 탈취
//!    - 2020년 Harvest Finance: $34M 손실 (오라클 조작)
//!    - 2022년 Mango Markets: $114M 손실
//!
//! Q: Chainlink가 업계 표준인 이유는?
//! A: 분산화 + 신뢰성
//!    1. 다수의 독립적인 노드 운영자
//!    2. 다중 데이터 소스 집계
//!    3. 이상치 필터링 알고리즘
//!    4. 경제적 인센티브 (스테이킹)
//!    5. 검증된 트랙레코드
//!
//! Q: TWAP(Time-Weighted Average Price)란?
//! A: 시간 가중 평균 가격
//!
//!    공식: TWAP = Σ(price_i * time_i) / Σ(time_i)
//!
//!    장점:
//!    - 순간적인 가격 조작에 강함
//!    - 플래시론 공격 방어
//!
//!    단점:
//!    - 가격 반영 지연
//!    - 급격한 변동 시 부정확

use anyhow::Result;
use chrono::{DateTime, Utc};

/// 가격 데이터
pub struct PriceData {
    pub price: u128,           // 8 decimals
    pub source: String,
    pub updated_at: DateTime<Utc>,
    pub change_24h: Option<f64>,
}

/// 가격 오라클 서비스
///
/// # Implementation Options
///
/// 1. Mock (현재): 고정 가격 반환 (테스트용)
/// 2. Chainlink: 온체인 데이터 조회
/// 3. External API: CoinGecko, Binance 등
/// 4. Hybrid: 다중 소스 집계
pub struct PriceOracle {
    oracle_url: String,
    /// 캐시된 가격 (빈번한 요청 최적화)
    cached_price: std::sync::RwLock<Option<CachedPrice>>,
}

struct CachedPrice {
    data: PriceData,
    cached_at: std::time::Instant,
}

impl PriceOracle {
    /// 캐시 유효 시간 (초)
    const CACHE_TTL_SECS: u64 = 60;

    pub fn new(oracle_url: &str) -> Self {
        Self {
            oracle_url: oracle_url.to_string(),
            cached_price: std::sync::RwLock::new(None),
        }
    }

    /// ETH/USD 가격 조회
    ///
    /// # Caching Strategy
    ///
    /// 가격 데이터는 60초 캐시
    /// - 이유: 외부 API 호출 비용 절감
    /// - 트레이드오프: 최신 가격이 아닐 수 있음
    /// - 개선: WebSocket으로 실시간 업데이트
    pub async fn get_eth_price(&self) -> Result<PriceData> {
        // 캐시 확인
        {
            let cache = self.cached_price.read().unwrap();
            if let Some(cached) = cache.as_ref() {
                if cached.cached_at.elapsed().as_secs() < Self::CACHE_TTL_SECS {
                    return Ok(PriceData {
                        price: cached.data.price,
                        source: cached.data.source.clone(),
                        updated_at: cached.data.updated_at,
                        change_24h: cached.data.change_24h,
                    });
                }
            }
        }

        // 새로 조회
        let price_data = self.fetch_price().await?;

        // 캐시 업데이트
        {
            let mut cache = self.cached_price.write().unwrap();
            *cache = Some(CachedPrice {
                data: PriceData {
                    price: price_data.price,
                    source: price_data.source.clone(),
                    updated_at: price_data.updated_at,
                    change_24h: price_data.change_24h,
                },
                cached_at: std::time::Instant::now(),
            });
        }

        Ok(price_data)
    }

    /// 실제 가격 fetch (외부 API 또는 온체인)
    async fn fetch_price(&self) -> Result<PriceData> {
        // TODO: 실제 구현
        //
        // Option 1: Chainlink (온체인)
        // let provider = Provider::<Http>::try_from(&self.oracle_url)?;
        // let aggregator = ChainlinkAggregator::new(ETH_USD_FEED, provider);
        // let (_, answer, _, updated_at, _) = aggregator.latestRoundData().call().await?;
        //
        // Option 2: External API
        // let resp = reqwest::get(&format!("{}/price/eth", self.oracle_url)).await?;
        // let data: ApiResponse = resp.json().await?;

        // Mock 구현 (테스트용)
        Ok(PriceData {
            price: 2000_00000000,  // $2000.00 (8 decimals)
            source: "mock".to_string(),
            updated_at: Utc::now(),
            change_24h: Some(-2.5),  // -2.5%
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_eth_price() {
        let oracle = PriceOracle::new("http://mock");
        let price = oracle.get_eth_price().await.unwrap();

        assert_eq!(price.price, 2000_00000000);
        assert_eq!(price.source, "mock");
    }

    #[tokio::test]
    async fn test_price_caching() {
        let oracle = PriceOracle::new("http://mock");

        // 첫 번째 호출
        let p1 = oracle.get_eth_price().await.unwrap();
        let t1 = p1.updated_at;

        // 두 번째 호출 (캐시에서)
        let p2 = oracle.get_eth_price().await.unwrap();
        let t2 = p2.updated_at;

        // 캐시된 값이므로 시간이 같아야 함
        assert_eq!(t1, t2);
    }
}
