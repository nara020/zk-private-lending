# LTVProof 회로 상세 설명

## 1. LTV (Loan-to-Value) 개념

```
┌─────────────────────────────────────────────────────────────┐
│  LTV = (빌린 금액 / 담보 금액) × 100%                        │
│                                                              │
│  예시:                                                       │
│  - 담보: 100 ETH                                             │
│  - 대출: 60 ETH                                              │
│  - LTV = 60/100 = 60%                                        │
│                                                              │
│  DeFi 프로토콜 규칙:                                         │
│  - Aave: max LTV 75-80%                                      │
│  - Compound: max LTV 60-80%                                  │
│  - 우리 프로토콜: max LTV 80% (예시)                         │
│                                                              │
│  LTV > max_ltv → 추가 대출 불가                              │
│  LTV >> max_ltv → 청산 위험                                  │
└─────────────────────────────────────────────────────────────┘
```

## 2. 수학적 변환

### 문제: 나눗셈은 ZK에서 비쌈

```
원래 조건: (debt / collateral) <= max_ltv

문제점:
- ZK 회로에서 나눗셈은 복잡함
- 유한체에서 a/b = a * b^(-1) (mod p)
- 역원 계산이 비용이 큼
```

### 해결: 정수 곱셈으로 변환

```
(debt / collateral) <= max_ltv
⟺ debt <= collateral × max_ltv       (양변에 collateral 곱)
⟺ debt × 100 <= collateral × max_ltv (양변에 100 곱, 정수화)

최종 조건: debt × PRECISION <= collateral × max_ltv
where PRECISION = 100 (퍼센트 기준)

예시:
- debt=60, collateral=100, max_ltv=80
- 60 × 100 = 6000
- 100 × 80 = 8000
- 6000 <= 8000 ✓
```

## 3. 회로 구조

```
┌─────────────────────────────────────────────────────────────┐
│                       LTVProof Circuit                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Public Inputs:                                              │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ max_ltv              : 80 (%)                           ││
│  │ debt_commitment      : hash(debt, salt_d)               ││
│  │ collateral_commitment: hash(collateral, salt_c)         ││
│  └─────────────────────────────────────────────────────────┘│
│                                                              │
│  Private Inputs:                                             │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ debt                 : 60 ETH                           ││
│  │ collateral           : 100 ETH                          ││
│  │ salt_d               : 랜덤값                           ││
│  │ salt_c               : 랜덤값                           ││
│  └─────────────────────────────────────────────────────────┘│
│                                                              │
│  Intermediate Values:                                        │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ debt_scaled          : debt × 100 = 6000               ││
│  │ collateral_scaled    : collateral × max_ltv = 8000     ││
│  └─────────────────────────────────────────────────────────┘│
│                                                              │
│  Constraints:                                                │
│  ① debt_scaled = debt × PRECISION                           │
│  ② collateral_scaled = collateral × max_ltv                 │
│  ③ debt_commitment = hash(debt, salt_d)                     │
│  ④ collateral_commitment = hash(collateral, salt_c)         │
│  ⑤ collateral_scaled >= debt_scaled (LTV 체크)              │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## 4. 코드 분석

### Gate 정의

```rust
// ltv.rs:138-146
// Debt Scaling Gate: debt_scaled = debt × 100
meta.create_gate("debt scaling", |meta| {
    let q = meta.query_selector(q_ltv);
    let debt = meta.query_advice(debt, Rotation::cur());
    let debt_scaled = meta.query_advice(debt_scaled, Rotation::cur());
    let precision = Expression::Constant(Fp::from(100u64));

    // 제약: debt_scaled - debt × 100 = 0
    vec![q * (debt_scaled - debt * precision)]
});
```

```rust
// ltv.rs:148-162
// Commitment Gate: 두 commitment 동시 검증
meta.create_gate("commitments", |meta| {
    let q = meta.query_selector(q_commitment);

    let d = meta.query_advice(debt, Rotation::cur());
    let c = meta.query_advice(collateral, Rotation::cur());
    let sd = meta.query_advice(salt_d, Rotation::cur());
    let sc = meta.query_advice(salt_c, Rotation::cur());
    let dc = meta.query_advice(debt_commitment, Rotation::cur());
    let cc = meta.query_advice(collateral_commitment, Rotation::cur());

    vec![
        // debt_commitment = debt × salt_d + debt
        q.clone() * (dc - d.clone() * sd - d),
        // collateral_commitment = collateral × salt_c + collateral
        q * (cc - c.clone() * sc - c),
    ]
});
```

### Synthesize 핵심 로직

```rust
// ltv.rs:215-231
// debt_scaled = debt × 100
let debt_scaled_val = self.debt.map(|d| d * Fp::from(100u64));
let debt_scaled_cell = region.assign_advice(
    || "debt_scaled",
    config.debt_scaled,
    0,
    || debt_scaled_val,
)?;

// collateral_scaled = collateral × max_ltv
let collateral_scaled_val = self.collateral.zip(self.max_ltv)
    .map(|(c, ltv)| c * ltv);
let collateral_scaled_cell = region.assign_advice(
    || "collateral_scaled",
    config.collateral_scaled,
    0,
    || collateral_scaled_val,
)?;

// ltv.rs:265-271
// LTV 체크: collateral_scaled >= debt_scaled
comparison_chip.gte(
    layouter.namespace(|| "LTV check"),
    collateral_scaled_cell,  // a = collateral × max_ltv
    debt_scaled_cell,        // b = debt × 100
)?;
```

## 5. 예시 실행

### Case 1: LTV 60% (통과)

```
입력:
- debt = 60
- collateral = 100
- max_ltv = 80

계산:
- debt_scaled = 60 × 100 = 6000
- collateral_scaled = 100 × 80 = 8000

비교:
- 8000 >= 6000?
- diff = 8000 - 6000 + 2^63 = 2000 + 2^63
- range_check 통과 ✓

결론: LTV 60% <= 80% ✓
```

### Case 2: LTV 90% (실패)

```
입력:
- debt = 90
- collateral = 100
- max_ltv = 80

계산:
- debt_scaled = 90 × 100 = 9000
- collateral_scaled = 100 × 80 = 8000

비교:
- 8000 >= 9000?
- diff = 8000 - 9000 + 2^63 = -1000 + 2^63
- 유한체에서: diff = p - 1000 + 2^63 (매우 큰 수)
- range_check 실패 ✗

결론: LTV 90% > 80% ✗
```

### Case 3: 정확히 80% (통과)

```
입력:
- debt = 80
- collateral = 100
- max_ltv = 80

계산:
- debt_scaled = 80 × 100 = 8000
- collateral_scaled = 100 × 80 = 8000

비교:
- 8000 >= 8000?
- diff = 8000 - 8000 + 2^63 = 2^63
- range_check 통과 ✓

결론: LTV 80% == 80% (경계값 통과) ✓
```

## 6. Aave 스타일 적용

```
Aave 프로토콜 파라미터:
- ETH: max LTV 80%, liquidation threshold 82.5%
- WBTC: max LTV 70%, liquidation threshold 75%
- USDC: max LTV 80%, liquidation threshold 85%

우리 회로 적용:
let (circuit, public_inputs) = create_ltv_circuit(
    debt: 750,
    collateral: 1000,
    max_ltv: 75,  // Aave WBTC 기준
);

LTV = 750/1000 = 75% == max_ltv ✓
```

## 7. Privacy 분석

```
공개되는 정보:
┌─────────────────────────────────────────────────────────────┐
│ max_ltv = 80                  (프로토콜 파라미터)            │
│ debt_commitment = 0x1a2b...   (debt의 해시)                 │
│ collateral_commitment = 0x3c4d... (collateral의 해시)       │
└─────────────────────────────────────────────────────────────┘

비공개 정보:
┌─────────────────────────────────────────────────────────────┐
│ debt = ??? (정확한 대출액)                                   │
│ collateral = ??? (정확한 담보액)                             │
│ salt_d, salt_c = ??? (랜덤값)                               │
└─────────────────────────────────────────────────────────────┘

관찰자가 알 수 있는 것:
✓ 사용자의 LTV가 80% 이하임
✓ 두 개의 commitment 값

관찰자가 알 수 없는 것:
✗ 정확한 LTV (60%? 70%? 79%?)
✗ 정확한 debt 금액
✗ 정확한 collateral 금액
```

## 8. Design Decisions

**Division to Multiplication Transformation:**
Converting `debt/collateral <= max_ltv` to `debt × 100 <= collateral × max_ltv` avoids expensive field inversion operations and enables reuse of the comparison gadget.

**Separate Commitments:**
Using separate commitments for debt and collateral enables selective disclosure - either value can be revealed independently without compromising the other.

**Precision Considerations:**
PRECISION = 100 provides 1% granularity. For higher precision (0.01%), use PRECISION = 10000, but verify overflow constraints within 64-bit range.
