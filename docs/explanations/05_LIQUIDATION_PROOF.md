# LiquidationProof 회로 상세 설명

## 1. Health Factor 개념

```
┌─────────────────────────────────────────────────────────────┐
│  Health Factor (건전성 지표)                                 │
│                                                              │
│  HF = (담보가치 × 청산임계값) / 부채                         │
│                                                              │
│  HF = (collateral × price × liquidation_threshold) / debt   │
│                                                              │
│  해석:                                                       │
│  - HF > 1.0 : 건전한 포지션 (안전)                          │
│  - HF = 1.0 : 경계선 (위험)                                  │
│  - HF < 1.0 : 청산 가능 (underwater)                        │
│                                                              │
│  예시:                                                       │
│  - collateral = 100 ETH, price = $2000, liq_threshold = 85% │
│  - debt = $160,000                                           │
│  - HF = (100 × 2000 × 0.85) / 160000 = 1.0625 > 1.0 ✓       │
└─────────────────────────────────────────────────────────────┘
```

## 2. 청산 메커니즘

```
┌─────────────────────────────────────────────────────────────┐
│                     청산 시나리오                            │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  T=0 (정상 상태):                                            │
│  - collateral = 100 ETH                                      │
│  - ETH price = $2000                                         │
│  - debt = $160,000                                           │
│  - liquidation_threshold = 85%                               │
│  - HF = (100 × 2000 × 0.85) / 160000 = 1.0625               │
│  - 상태: 안전 ✓                                              │
│                                                              │
│  T=1 (가격 하락):                                            │
│  - ETH price = $1800 (10% 하락)                              │
│  - HF = (100 × 1800 × 0.85) / 160000 = 0.956                │
│  - 상태: 청산 가능! (HF < 1.0) ✗                             │
│                                                              │
│  청산 실행:                                                   │
│  - 청산자가 debt 일부 상환                                   │
│  - 담보 일부 + 보너스 수령                                   │
│  - 프로토콜 안정성 유지                                      │
└─────────────────────────────────────────────────────────────┘
```

## 3. ZK 청산의 의미

```
┌─────────────────────────────────────────────────────────────┐
│  전통적 청산 (투명):                                         │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ 문제점:                                                 ││
│  │ 1. 모든 포지션이 온체인에 공개                          ││
│  │ 2. MEV 봇이 큰 포지션 추적                              ││
│  │ 3. 청산 직전 포지션 front-running                       ││
│  │ 4. 청산 경쟁으로 가스비 급등                            ││
│  └─────────────────────────────────────────────────────────┘│
│                                                              │
│  ZK 청산 (프라이빗):                                         │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ 장점:                                                   ││
│  │ 1. 포지션 세부사항 비공개                               ││
│  │ 2. 청산자만 증명 가능 (포지션 알아야 함)                ││
│  │ 3. Front-running 방지                                   ││
│  │ 4. 공정한 청산 기회                                     ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

## 4. 회로 구조

```
┌─────────────────────────────────────────────────────────────┐
│                   LiquidationProof Circuit                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Public Inputs:                                              │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ price                : Oracle에서 가져온 현재 가격      ││
│  │ liquidation_threshold: 85 (%)                           ││
│  │ position_hash        : hash(collateral, debt, salt)     ││
│  └─────────────────────────────────────────────────────────┘│
│                                                              │
│  Private Inputs:                                             │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ collateral           : 100 ETH                          ││
│  │ debt                 : 160000 (USD 기준)                ││
│  │ salt                 : 랜덤값                           ││
│  └─────────────────────────────────────────────────────────┘│
│                                                              │
│  증명 내용:                                                  │
│  "이 포지션(position_hash)은 현재 가격에서 청산 가능하다"    │
│  (HF < 1.0)                                                  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## 5. 수학적 변환

```
청산 조건: HF < 1.0

HF = (collateral × price × liq_threshold) / debt < 1.0

나눗셈 제거:
collateral × price × liq_threshold < debt

정수화 (PRECISION = 100):
collateral × price × liq_threshold < debt × PRECISION × PRECISION

최종:
collateral_value < debt_scaled

where:
- collateral_value = collateral × price × liq_threshold
- debt_scaled = debt × 10000 (PRECISION²)
```

## 6. 코드 분석

### Computation Gate

```rust
// liquidation.rs (simplified)
meta.create_gate("liquidation computation", |meta| {
    let q = meta.query_selector(q_compute);

    let coll = meta.query_advice(collateral, Rotation::cur());
    let d = meta.query_advice(debt, Rotation::cur());
    let p = meta.query_advice(price, Rotation::cur());
    let lt = meta.query_advice(liquidation_threshold, Rotation::cur());
    let cv = meta.query_advice(collateral_value, Rotation::cur());
    let ds = meta.query_advice(debt_scaled, Rotation::cur());

    let precision_sq = Expression::Constant(Fp::from(10000u64));

    vec![
        // collateral_value = collateral × price × liq_threshold
        q.clone() * (cv - coll * p * lt),

        // debt_scaled = debt × PRECISION²
        q * (ds - d * precision_sq),
    ]
});
```

### Position Hash Gate

```rust
// Position commitment: hash(collateral, debt, salt)
meta.create_gate("position hash", |meta| {
    let q = meta.query_selector(q_commitment);

    let coll = meta.query_advice(collateral, Rotation::cur());
    let d = meta.query_advice(debt, Rotation::cur());
    let s = meta.query_advice(salt, Rotation::cur());
    let hash = meta.query_advice(position_hash, Rotation::cur());

    // Simplified hash: coll * s + debt * s + coll + debt
    vec![q * (hash - coll.clone() * s.clone() - d.clone() * s - coll - d)]
});
```

### Liquidation Check (핵심!)

```rust
// 청산 조건: debt_scaled > collateral_value (HF < 1.0)
comparison_chip.gt(
    layouter.namespace(|| "liquidation check"),
    debt_scaled_cell,        // a = debt × PRECISION²
    collateral_value_cell,   // b = collateral × price × liq_threshold
)?;
```

**주의**: `gte`가 아닌 `gt` (strictly greater) 사용!
- HF < 1.0 이어야 청산 가능
- HF = 1.0은 청산 불가 (경계선)

## 7. 예시 실행

### Case 1: Underwater Position (청산 가능)

```
입력:
- collateral = 100
- debt = 90 (간단한 예시)
- price = 1
- liquidation_threshold = 85

계산:
- collateral_value = 100 × 1 × 85 = 8500
- debt_scaled = 90 × 100 × 100 = 900000

비교 (gt: debt_scaled > collateral_value):
- 900000 > 8500?

잠깐, 계산 다시:
PRECISION = 100이므로
- collateral_value = 100 × 1 × 85 = 8500
- debt_scaled = 90 × 10000 = 900000

비교:
- 900000 > 8500 ✓
- 청산 가능!

실제 HF:
HF = 8500 / 9000 = 0.94 < 1.0 ✓
```

### Case 2: Healthy Position (청산 불가)

```
입력:
- collateral = 100
- debt = 50
- price = 100
- liquidation_threshold = 85

계산:
- collateral_value = 100 × 100 × 85 = 850000
- debt_scaled = 50 × 10000 = 500000

비교:
- 500000 > 850000?
- 500000 < 850000 ✗
- 청산 불가!

실제 HF:
HF = 850000 / 500000 = 1.7 > 1.0 ✓ (안전)
```

### Case 3: Price Drop Scenario

```
T=0: 안전한 포지션
- collateral = 100 ETH
- debt = 70 (scaled)
- price = 100
- liq_threshold = 85
- collateral_value = 100 × 100 × 85 = 850000
- debt_scaled = 70 × 10000 = 700000
- HF = 850000 / 700000 = 1.21 > 1.0 ✓

T=1: 가격 20% 하락
- price = 80
- collateral_value = 100 × 80 × 85 = 680000
- debt_scaled = 700000 (변화 없음)
- HF = 680000 / 700000 = 0.97 < 1.0 ✗

청산 가능!
```

## 8. Privacy 분석

```
청산자가 알아야 하는 것:
┌─────────────────────────────────────────────────────────────┐
│ ✓ collateral 정확한 양                                      │
│ ✓ debt 정확한 양                                            │
│ ✓ salt (position_hash 계산용)                               │
└─────────────────────────────────────────────────────────────┘

온체인에 공개되는 것:
┌─────────────────────────────────────────────────────────────┐
│ ✓ price (Oracle 데이터)                                     │
│ ✓ liquidation_threshold (프로토콜 파라미터)                  │
│ ✓ position_hash (포지션 식별자)                              │
│ ✓ ZK proof (청산 유효성)                                     │
└─────────────────────────────────────────────────────────────┘

공격자가 알 수 없는 것:
┌─────────────────────────────────────────────────────────────┐
│ ✗ 정확한 담보량                                              │
│ ✗ 정확한 부채량                                              │
│ ✗ 정확한 HF (단지 "< 1.0"만 알 수 있음)                     │
│ ✗ 얼마나 underwater인지                                      │
└─────────────────────────────────────────────────────────────┘
```

## 9. 청산 플로우

```
┌─────────────────────────────────────────────────────────────┐
│  1. 청산자가 포지션 발견                                     │
│     - 오프체인에서 모니터링                                  │
│     - 또는 포지션 소유자에게서 정보 획득                     │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. ZK Proof 생성                                            │
│     - LiquidationCircuit 인스턴스 생성                       │
│     - Private inputs: collateral, debt, salt                │
│     - Prove 실행 → proof 생성                               │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. 온체인 청산 실행                                         │
│     - liquidate(position_hash, proof) 호출                  │
│     - Verifier가 proof 검증                                 │
│     - 유효하면 청산 진행                                     │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  4. 청산 완료                                                │
│     - 청산자: debt 상환, collateral + 보너스 수령           │
│     - 포지션 소유자: 남은 담보 회수                          │
│     - 프로토콜: 안정성 유지                                  │
└─────────────────────────────────────────────────────────────┘
```

## 10. Implementation Details

**Strictly Greater (gt) vs Greater-or-Equal (gte):**
The circuit uses `gt` (strictly greater) because liquidation requires HF < 1.0, not HF <= 1.0. A position exactly at HF = 1.0 is still considered safe.

**Liquidator Information Access:**
In a production system, liquidators would obtain position information through:
- Off-chain monitoring with partial information leakage
- Keeper systems where position owners voluntarily share data
- Incentive mechanisms for timely liquidations

**Oracle Trust Assumption:**
The circuit assumes the oracle price is correct. Production deployments should use:
- Chainlink or similar decentralized oracles
- Time-weighted average prices (TWAP)
- Multi-oracle aggregation for robustness
