# Range Check 알고리즘 상세 설명

## 1. 문제 정의

```
목표: value가 [0, 2^BITS) 범위 내인지 증명
예시: BITS=8이면 [0, 256) 범위, 즉 0~255

왜 필요한가?
- 유한체(Finite Field)에서는 음수가 없음
- 큰 수가 wrap-around 되어 음수처럼 동작
- 범위 체크 없이는 overflow 공격 가능
```

## 2. Halo2 Lookup Table 방식

### 알고리즘 원리

```
┌─────────────────────────────────────────────────────────────┐
│  Step 1: 테이블 사전 생성                                    │
│                                                              │
│  Table T = [0, 1, 2, 3, ..., 2^BITS - 1]                    │
│                                                              │
│  예시 (BITS=8):                                              │
│  T = [0, 1, 2, 3, ..., 255]                                 │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  Step 2: Lookup Argument                                     │
│                                                              │
│  제약조건: "value ∈ T"                                       │
│                                                              │
│  - value=100 → T에 존재 → 통과 ✓                            │
│  - value=256 → T에 미존재 → 실패 ✗                          │
│  - value=(-1 mod p) → T에 미존재 → 실패 ✗                   │
└─────────────────────────────────────────────────────────────┘
```

### 코드 구현

```rust
// range_check.rs:62-76

pub fn configure(meta: &mut ConstraintSystem<F>, value: Column<Advice>)
    -> RangeCheckConfig<F, BITS>
{
    let q_lookup = meta.complex_selector();  // 활성화 플래그
    let table = meta.lookup_table_column();  // 테이블 컬럼

    // 핵심: Lookup Argument 정의
    meta.lookup("range check", |meta| {
        let q = meta.query_selector(q_lookup);
        let v = meta.query_advice(value, Rotation::cur());

        // (q * v, table)의 의미:
        // - q=1이면 v가 table에 반드시 존재해야 함
        // - q=0이면 검사 안 함 (0*v = 0, 0은 항상 테이블에 있음)
        vec![(q * v, table)]
    });

    // ...
}
```

### 테이블 로드

```rust
// range_check.rs:86-104

pub fn load_table(&self, mut layouter: impl Layouter<F>) -> Result<(), Error> {
    let table_size = 1 << BITS;  // 2^BITS

    layouter.assign_table(
        || "range check table",
        |mut table| {
            // 0부터 2^BITS-1까지 모든 값을 테이블에 로드
            for i in 0..table_size {
                table.assign_cell(
                    || format!("table[{}]", i),
                    self.config.table,
                    i,
                    || Value::known(F::from(i as u64)),
                )?;
            }
            Ok(())
        },
    )
}
```

### 장점

| 특성 | 값 |
|------|-----|
| Constraint 수 | **1개** (상수) |
| 테이블 크기 | 2^BITS rows |
| 검증 복잡도 | O(1) |
| 메모리 | O(2^BITS) |

## 3. R1CS Bit Decomposition 방식 (arkworks)

### 알고리즘 원리

```
┌─────────────────────────────────────────────────────────────┐
│  Step 1: 비트 분해                                           │
│                                                              │
│  value = b₀ + b₁·2 + b₂·4 + ... + b_{n-1}·2^{n-1}          │
│                                                              │
│  예시: value = 100 (BITS=8)                                  │
│  100 = 0 + 0·2 + 1·4 + 0·8 + 0·16 + 1·32 + 1·64 + 0·128    │
│      = [0, 0, 1, 0, 0, 1, 1, 0] (little-endian)            │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  Step 2: Boolean 제약                                        │
│                                                              │
│  각 비트 bᵢ에 대해: bᵢ · (1 - bᵢ) = 0                       │
│                                                              │
│  - bᵢ = 0이면: 0 · 1 = 0 ✓                                  │
│  - bᵢ = 1이면: 1 · 0 = 0 ✓                                  │
│  - bᵢ = 2이면: 2 · (-1) ≠ 0 ✗                               │
└─────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  Step 3: 재조합 검증                                         │
│                                                              │
│  value == Σ bᵢ · 2^i                                        │
│                                                              │
│  이 제약으로 value가 BITS 비트 내에 있음을 보장             │
└─────────────────────────────────────────────────────────────┘
```

### 코드 구현

```rust
// arkworks/collateral.rs:104-120

// 비트 분해 (각 비트에 대해 Boolean 제약 자동 생성)
let collateral_bits = collateral_var.to_bits_le()?;

// 상위 비트가 0인지 확인 (RANGE_BITS 이상의 비트는 0이어야 함)
for bit in collateral_bits.iter().skip(RANGE_BITS) {
    bit.enforce_equal(&Boolean::constant(false))?;
}
```

### 제약 수 분석

```
64-bit 값 기준:

비트 분해:        ~64 constraints (각 비트 Boolean 제약)
재조합 검증:      ~64 constraints (선형 조합)
─────────────────────────────
총합:             ~128 constraints

vs Halo2:         1 constraint (lookup)
```

## 4. 두 방식 비교

### Constraint 효율성

```
┌────────────────┬─────────────┬───────────────┐
│  Bit Width     │  R1CS       │  Halo2        │
├────────────────┼─────────────┼───────────────┤
│  8-bit         │  ~16        │  1            │
│  16-bit        │  ~32        │  1            │
│  32-bit        │  ~64        │  1            │
│  64-bit        │  ~128       │  1            │
│  256-bit       │  ~512       │  1            │
└────────────────┴─────────────┴───────────────┘

결론: Halo2는 bit width에 관계없이 O(1)
     R1CS는 O(n) where n = bit width
```

### 트레이드오프

| 특성 | Halo2 Lookup | R1CS Bit Decomposition |
|------|--------------|------------------------|
| Constraint 수 | O(1) | O(n) |
| 테이블 메모리 | O(2^n) | O(1) |
| Setup 복잡도 | Universal | Per-circuit |
| 증명 크기 | ~1KB | ~200B |
| 적합한 경우 | 큰 범위, 반복 사용 | 작은 범위, 일회성 |

## 5. 보안 고려사항

### Overflow 공격 방지

```
공격 시나리오:
- collateral = -1 (mod p) = p - 1 (매우 큰 수)
- 범위 체크 없으면 "충분한 담보"로 통과할 수 있음

방지:
- Range check로 collateral이 합리적 범위 내인지 확인
- 64-bit 범위 = 최대 ~18 quintillion (현실적 자산 범위)
```

### Field 크기 고려

```rust
// BN254 field size: ~254 bits
// 우리 값: 최대 64 bits
// 충분한 여유 있음 (overflow 불가)

const RANGE_BITS: usize = 64;  // 64-bit 범위
```

## 6. Implementation Notes

The lookup table approach is particularly efficient for repeated range checks on the same bit width, making it ideal for DeFi applications where multiple values need validation within the same circuit.
