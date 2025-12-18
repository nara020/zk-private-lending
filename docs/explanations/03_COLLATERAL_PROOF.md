# CollateralProof 회로 상세 설명

## 1. 목적

```
┌─────────────────────────────────────────────────────────────┐
│                      DeFi 프라이버시 문제                    │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  기존 DeFi (Aave, Compound):                                │
│  - 사용자가 100 ETH 예치                                     │
│  - 온체인에 "0x123...가 100 ETH 보유" 공개                  │
│  - MEV 봇이 대형 포지션 추적                                 │
│  - 청산 헌터가 front-run                                     │
│                                                              │
│  ZK-Private Lending:                                         │
│  - 사용자가 담보 예치                                        │
│  - 온체인에 "담보 충분함" (O/X)만 공개                       │
│  - 정확한 금액은 비공개                                      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## 2. 회로 구조

### Input/Output 정의

```
┌─────────────────────────────────────────────────────────────┐
│                    CollateralProof Circuit                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Public Inputs (온체인 공개, 누구나 볼 수 있음):             │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ threshold    : 500 ETH (프로토콜이 요구하는 최소 담보) ││
│  │ commitment   : 0x1a2b... (담보 금액의 암호학적 해시)   ││
│  └─────────────────────────────────────────────────────────┘│
│                                                              │
│  Private Inputs (Prover만 알음, 온체인에 공개 안 됨):        │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ collateral   : 1000 ETH (실제 담보 금액)               ││
│  │ salt         : 0x9f8e... (commitment 생성용 랜덤값)    ││
│  └─────────────────────────────────────────────────────────┘│
│                                                              │
│  증명 내용:                                                  │
│  "내가 가진 collateral은 threshold 이상이고,                │
│   commitment은 내 collateral의 유효한 해시다"               │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### Constraint 구조

```
┌─────────────────────────────────────────────────────────────┐
│  Constraint 1: Commitment 검증                               │
│                                                              │
│  computed_commitment = hash(collateral, salt)                │
│  computed_commitment == public_commitment                    │
│                                                              │
│  의미: "내가 주장하는 담보와 public commitment이 일치"       │
│                                                              │
│  현재 구현 (단순화):                                         │
│  commitment = collateral * salt + collateral                 │
│                                                              │
│  프로덕션: Poseidon Hash 사용                                │
│  commitment = Poseidon(collateral, salt)                     │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  Constraint 2: 비교 검증                                     │
│                                                              │
│  collateral >= threshold                                     │
│                                                              │
│  구현: ComparisonChip 사용                                   │
│  - diff = collateral - threshold + offset                    │
│  - range_check(diff)                                         │
└─────────────────────────────────────────────────────────────┘
```

## 3. 코드 분석

### Circuit Config (회로 구조 정의)

```rust
// collateral.rs:37-54
pub struct CollateralConfig<F: PrimeField> {
    // Private witness columns (Prover만 할당)
    pub collateral: Column<Advice>,
    pub salt: Column<Advice>,

    // Public과 연결될 값들
    pub threshold: Column<Advice>,
    pub commitment_computed: Column<Advice>,

    // Public inputs
    pub instance: Column<Instance>,

    // Gate 활성화 플래그
    pub q_commitment: Selector,

    // 비교 로직
    pub comparison: ComparisonConfig<F, RANGE_BITS>,
}
```

### Configure (제약 조건 정의)

```rust
// collateral.rs:109-161
fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
    // 1. Column 생성
    let collateral = meta.advice_column();
    let salt = meta.advice_column();
    let threshold = meta.advice_column();
    let commitment_computed = meta.advice_column();
    let instance = meta.instance_column();

    // 2. Equality 활성화 (copy constraint용)
    meta.enable_equality(collateral);
    meta.enable_equality(instance);
    // ... 기타 columns

    // 3. Commitment Gate 정의
    let q_commitment = meta.selector();
    meta.create_gate("commitment", |meta| {
        let q = meta.query_selector(q_commitment);
        let coll = meta.query_advice(collateral, Rotation::cur());
        let s = meta.query_advice(salt, Rotation::cur());
        let comm = meta.query_advice(commitment_computed, Rotation::cur());

        // 제약: comm = coll * s + coll
        // 수학적: comm - coll * s - coll = 0
        vec![q * (comm - coll.clone() * s - coll)]
    });

    // 4. Comparison Chip 설정
    let diff = meta.advice_column();
    let comparison = ComparisonChip::configure(meta, collateral, threshold, diff);

    // ...
}
```

### Synthesize (값 할당 및 실행)

```rust
// collateral.rs:163-227
fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<Fp>)
    -> Result<(), Error>
{
    // 1. Lookup Table 로드 (Range Check용)
    let comparison_chip = ComparisonChip::construct(config.comparison.clone());
    comparison_chip.load_table(layouter.namespace(|| "load range table"))?;

    // 2. Private/Public 값 할당
    let (collateral_cell, threshold_cell, commitment_cell) = layouter.assign_region(
        || "assign inputs",
        |mut region| {
            // Commitment gate 활성화
            config.q_commitment.enable(&mut region, 0)?;

            // Private: collateral
            let collateral_cell = region.assign_advice(
                || "collateral",
                config.collateral,
                0,
                || self.collateral,  // Value<Fp>
            )?;

            // Private: salt
            region.assign_advice(|| "salt", config.salt, 0, || self.salt)?;

            // Public: threshold
            let threshold_cell = region.assign_advice(
                || "threshold", config.threshold, 0, || self.threshold
            )?;

            // Computed: commitment = collateral * salt + collateral
            let commitment_value = self.collateral.zip(self.salt).map(|(c, s)| {
                Self::compute_commitment(c, s)
            });
            let commitment_cell = region.assign_advice(
                || "commitment", config.commitment_computed, 0, || commitment_value
            )?;

            Ok((collateral_cell, threshold_cell, commitment_cell))
        },
    )?;

    // 3. Public Inputs와 연결 (Copy Constraint)
    layouter.constrain_instance(threshold_cell.cell(), config.instance, 0)?;
    layouter.constrain_instance(commitment_cell.cell(), config.instance, 1)?;

    // 4. 비교 증명: collateral >= threshold
    comparison_chip.gte(
        layouter.namespace(|| "collateral >= threshold"),
        collateral_cell,
        threshold_cell,
    )?;

    Ok(())
}
```

## 4. 실행 흐름

```
┌─────────────────────────────────────────────────────────────┐
│  Step 1: Prover가 Circuit 인스턴스 생성                      │
│                                                              │
│  let circuit = CollateralCircuit::new(                       │
│      collateral: 1000,  // Private                           │
│      salt: 12345,       // Private                           │
│      threshold: 500,    // Public                            │
│      commitment: hash(1000, 12345),  // Public               │
│  );                                                          │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  Step 2: Prove 실행                                          │
│                                                              │
│  1. load_table(): [0, 1, ..., 2^64-1] 테이블 로드           │
│                                                              │
│  2. assign_region(): 값 할당                                 │
│     - collateral = 1000                                      │
│     - salt = 12345                                           │
│     - threshold = 500                                        │
│     - commitment = 1000 * 12345 + 1000 = 12346000           │
│                                                              │
│  3. q_commitment gate 체크:                                  │
│     commitment - collateral * salt - collateral = 0?        │
│     12346000 - 1000 * 12345 - 1000 = 0 ✓                    │
│                                                              │
│  4. constrain_instance():                                    │
│     threshold_cell == instance[0] (500)                      │
│     commitment_cell == instance[1] (12346000)                │
│                                                              │
│  5. comparison.gte():                                        │
│     collateral >= threshold?                                 │
│     diff = 1000 - 500 + 2^63 = 9223372036854776308          │
│     range_check(diff) → diff < 2^64 ✓                       │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  Step 3: Proof 생성                                          │
│                                                              │
│  모든 constraint 만족 → Proof π 생성                         │
│  Public: [threshold=500, commitment=12346000]                │
│  Proof: π (약 1KB)                                           │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  Step 4: Verification (온체인)                               │
│                                                              │
│  Verifier.verify(                                            │
│      vk,  // Verification Key                                │
│      public_inputs: [500, 12346000],                         │
│      proof: π                                                │
│  ) → true/false                                              │
│                                                              │
│  Verifier가 알 수 있는 것:                                    │
│  ✓ 누군가 500 이상의 담보를 가짐                             │
│  ✓ 그 담보의 commitment는 12346000                           │
│                                                              │
│  Verifier가 알 수 없는 것:                                    │
│  ✗ 정확한 담보 금액 (1000)                                   │
│  ✗ salt 값 (12345)                                           │
└─────────────────────────────────────────────────────────────┘
```

## 5. 보안 속성

### Soundness (건전성)

```
"거짓 증명은 불가능하다"

공격 시도: collateral = 400 (threshold 미달)인데 통과하려 함

문제:
- comparison.gte(400, 500)
- diff = 400 - 500 + 2^63
- 유한체에서: diff = p - 100 + 2^63 (매우 큰 수)
- range_check 실패 ✗

결론: threshold 미만이면 증명 불가
```

### Zero-Knowledge (영지식)

```
"Verifier는 담보 금액을 알 수 없다"

증명에서 공개되는 것:
- threshold (이미 알려진 파라미터)
- commitment (해시값)
- proof (ZK proof)

공개되지 않는 것:
- collateral 실제값
- salt 값

Commitment의 Hiding 속성:
- commitment = hash(collateral, salt)
- salt가 랜덤이면 commitment에서 collateral 역산 불가
- 같은 collateral도 salt에 따라 다른 commitment
```

### Binding (구속성)

```
"한번 commit하면 값 변경 불가"

commitment = hash(collateral, salt)

공격 시도:
- commitment C로 증명 생성
- 나중에 다른 collateral'로 같은 C 만들려 함

Hash의 Binding 속성:
- Collision resistance: hash(a, s1) = hash(b, s2) 찾기 어려움
- 같은 commitment에 대해 다른 값 증명 불가
```

## 6. 테스트 케이스

```rust
// 유효한 케이스
#[test]
fn test_collateral_proof_valid() {
    // collateral (1000) >= threshold (500) ✓
    let (circuit, public_inputs) = create_test_circuit(1000, 12345, 500);
    let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
    assert_eq!(prover.verify(), Ok(()));
}

// 경계값
#[test]
fn test_collateral_proof_equal() {
    // collateral (500) >= threshold (500) ✓
    // 정확히 같을 때도 통과해야 함
}

// 실패 케이스
#[test]
fn test_collateral_proof_insufficient() {
    // collateral (400) >= threshold (500) ✗
    // 미달이면 증명 실패해야 함
}

// Commitment 불일치
#[test]
fn test_collateral_proof_wrong_commitment() {
    // commitment가 실제 (collateral, salt)와 안 맞으면 실패
}
```

## 7. 프로덕션 고려사항

### 현재 구현의 한계

```
1. Commitment 함수가 단순함
   현재: commitment = collateral * salt + collateral
   문제: 암호학적으로 안전하지 않을 수 있음
   해결: Poseidon Hash 사용

2. Salt 생성
   현재: 테스트용 하드코딩
   문제: 예측 가능한 salt는 privacy 훼손
   해결: 암호학적 난수 생성기 사용

3. 64-bit 범위
   현재: 64-bit collateral 값
   확인: 실제 자산 범위에 충분한가? (최대 ~18 ETH * 10^18)
```

### 필요한 개선

```rust
// 프로덕션 Commitment (Poseidon 사용)
pub fn compute_commitment(collateral: F, salt: F) -> F {
    use halo2_gadgets::poseidon::{Pow5Chip, Pow5Config, Hash};

    // Poseidon parameters for security
    let params = Poseidon::new::<R_F, R_P, T, RATE>();
    params.hash(&[collateral, salt])
}
```

## 8. Key Concepts

**Commitment Properties:**
- **Binding**: Once committed, the collateral value cannot be changed without invalidating the proof
- **Hiding**: The commitment reveals nothing about the actual collateral amount

**Salt Purpose:**
The salt ensures that identical collateral amounts produce different commitments, preventing rainbow table attacks where an attacker could precompute hashes for all possible values.
