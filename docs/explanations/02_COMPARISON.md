# Comparison 알고리즘 상세 설명

## 1. 문제 정의

```
목표: a >= b 임을 증명 (유한체에서)

문제점:
- 유한체(Finite Field)에는 "음수" 개념이 없음
- 모든 연산은 mod p로 수행
- 예: p=101에서 5-10 = 96 (mod 101), 음수가 아닌 96

해결책: Offset 기법
```

## 2. Offset 기법 알고리즘

### 핵심 아이디어

```
┌─────────────────────────────────────────────────────────────┐
│  관찰:                                                       │
│                                                              │
│  유한체에서 a - b의 결과:                                    │
│  - a >= b 이면: 결과가 "작은 양수" (0 ~ 2^63 범위)          │
│  - a < b 이면: 결과가 "매우 큰 양수" (p - 작은수 ≈ p)       │
│                                                              │
│  예시 (p = 101):                                             │
│  - 10 - 5 = 5 (작은 양수)                                   │
│  - 5 - 10 = 96 (매우 큰 양수, 실제론 -5)                    │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  해결책: Offset 더하기                                       │
│                                                              │
│  diff = a - b + offset                                       │
│  where offset = 2^(BITS-1)                                   │
│                                                              │
│  BITS=64일 때, offset = 2^63 ≈ 9.2 × 10^18                  │
└─────────────────────────────────────────────────────────────┘
```

### 동작 원리

```
Case 1: a >= b (성공해야 함)
─────────────────────────────
a = 100, b = 50
diff = 100 - 50 + 2^63
     = 50 + 2^63
     = 9223372036854775858

이 값이 [0, 2^64) 범위 내인가?
2^63 <= diff <= 2^63 + max_value
범위 내 ✓


Case 2: a < b (실패해야 함)
─────────────────────────────
a = 50, b = 100
diff = 50 - 100 + 2^63
     = -50 + 2^63

유한체에서 -50 = p - 50 (매우 큰 수)
따라서 diff = (p - 50) + 2^63 ≈ p (매우 큰 수)

이 값이 [0, 2^64) 범위 내인가?
p ≈ 2^254 >> 2^64
범위 초과 ✗
```

### 시각화

```
                    0                   2^63                  2^64                 p
                    │                    │                     │                   │
실제 수직선:        ├────────────────────┼─────────────────────┤                   │
                    │    유효 범위       │     유효 범위       │                   │
                    │   (a - b >= 0)     │    (a - b >= 0)     │                   │

a >= b일 때:        │         ●──────────┼──────────●          │                   │
diff = a-b+offset   │     [offset, offset+max]                 │                   │
                    │                    │                     │                   │

a < b일 때:         │                    │                     │    ●─────────────●│
diff = a-b+offset   │                    │                     │    [p-max, p]     │
                    │                    │                     │                   │

Range Check 범위:   │████████████████████████████████████████████│                   │
[0, 2^64)           │                    │                     │                   │

a >= b: 범위 내 ✓
a < b:  범위 밖 ✗
```

## 3. 코드 구현

### Halo2 버전

```rust
// comparison.rs:68-72
impl<F: PrimeField, const BITS: usize> ComparisonChip<F, BITS> {
    /// Offset: 2^(BITS-1)
    fn offset() -> F {
        F::from(1u64 << (BITS - 1))  // 2^63 for BITS=64
    }
}
```

```rust
// comparison.rs:91-102
// Custom Gate 정의
meta.create_gate("comparison", |meta| {
    let q = meta.query_selector(q_cmp);
    let a = meta.query_advice(a, Rotation::cur());
    let b = meta.query_advice(b, Rotation::cur());
    let diff = meta.query_advice(diff, Rotation::cur());
    let offset = Expression::Constant(Self::offset());

    // 제약: diff = a - b + offset
    // 수학적으로: diff - a + b - offset = 0
    vec![q * (diff - a + b - offset)]
});
```

```rust
// comparison.rs:121-156
fn gte(&self, mut layouter, a: AssignedCell, b: AssignedCell) -> Result<(), Error> {
    // 1. diff 계산 및 할당
    let diff_cell = layouter.assign_region(
        || "comparison: a >= b",
        |mut region| {
            self.config.q_cmp.enable(&mut region, 0)?;

            a.copy_advice(|| "a", &mut region, self.config.a, 0)?;
            b.copy_advice(|| "b", &mut region, self.config.b, 0)?;

            let diff_value = a.value().zip(b.value()).map(|(a, b)| {
                *a - *b + Self::offset()
            });

            region.assign_advice(|| "diff", self.config.diff, 0, || diff_value)
        },
    )?;

    // 2. diff에 대한 Range Check
    let range_chip = RangeCheckChip::construct(self.config.range_check.clone());
    range_chip.check(layouter, diff_cell, BITS)?;

    Ok(())
}
```

### arkworks 버전

```rust
// arkworks/collateral.rs:122-136

// Compute difference
let diff = &collateral_var - &threshold_var;

// Add offset
let offset = FpVar::constant(F::from(1u64 << (RANGE_BITS - 1)));
let diff_shifted = diff + offset;

// Range check via bit decomposition
let diff_bits = diff_shifted.to_bits_le()?;
for bit in diff_bits.iter().skip(RANGE_BITS) {
    bit.enforce_equal(&Boolean::constant(false))?;
}
```

## 4. Strictly Greater (a > b)

```rust
// comparison.rs:158-175
fn gt(&self, mut layouter, a: AssignedCell, b: AssignedCell) -> Result<(), Error> {
    // a > b  ⟺  a >= b + 1
    let b_plus_one = layouter.assign_region(
        || "b + 1",
        |mut region| {
            let b_val = b.value().map(|b| *b + F::ONE);
            region.assign_advice(|| "b + 1", self.config.b, 0, || b_val)
        },
    )?;

    // a >= (b + 1)을 증명
    self.gte(layouter, a, b_plus_one)
}
```

```
a > b를 증명하려면:
1. b' = b + 1 계산
2. a >= b' 증명
3. a >= b + 1 이면 a > b
```

## 5. 수학적 증명

### 정리: Offset 기법의 Soundness

```
Claim: diff = a - b + 2^(n-1)이 [0, 2^n) 범위 내
       ⟺ a >= b (0 ≤ a, b < 2^(n-1) 가정)

Proof:
(→) diff ∈ [0, 2^n) 가정
    diff = a - b + 2^(n-1)
    0 ≤ a - b + 2^(n-1) < 2^n
    -2^(n-1) ≤ a - b < 2^(n-1)

    a, b가 [0, 2^(n-1)) 범위이므로:
    a - b의 범위는 (-2^(n-1), 2^(n-1))

    diff가 범위 내이면:
    a - b ≥ -2^(n-1) (항상 성립)
    a - b < 2^(n-1) (항상 성립)

    하지만 더 구체적으로:
    diff ≥ 0 ⟹ a - b ≥ -2^(n-1) (항상 참)
    diff < 2^n ⟹ a - b < 2^(n-1) (항상 참)

    핵심: diff ∈ [2^(n-1), 2^n) 이면 a - b ∈ [0, 2^(n-1))
         즉, a ≥ b

(←) a ≥ b 가정 (0 ≤ a - b < 2^(n-1))
    diff = a - b + 2^(n-1)
    diff ∈ [2^(n-1), 2^n)
    따라서 diff ∈ [0, 2^n) ✓
```

## 6. Edge Cases

### Case 1: a = b (같을 때)

```
a = 100, b = 100
diff = 100 - 100 + 2^63 = 2^63

2^63 < 2^64 이므로 범위 내 ✓
a >= b 증명됨 ✓
```

### Case 2: a = 0, b = 0

```
a = 0, b = 0
diff = 0 - 0 + 2^63 = 2^63

범위 내 ✓
```

### Case 3: Maximum 값

```
BITS = 64, max_value = 2^63 - 1

a = 2^63 - 1 (최대값)
b = 0
diff = (2^63 - 1) - 0 + 2^63 = 2^64 - 1

2^64 - 1 < 2^64 이므로 범위 내 ✓
```

## 7. Design Rationale

The offset value of 2^(BITS-1) is chosen for symmetry: it maps the possible difference range (-2^63, 2^63) to exactly (0, 2^64), enabling a single range check to validate the comparison.
