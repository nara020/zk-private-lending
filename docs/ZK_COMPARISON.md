# ZK Stack Comparison: Halo2 vs arkworks vs Circom

> 동일한 CollateralProof 회로를 3개 스택으로 구현하여 비교 분석

## 1. Overview

| Aspect | Halo2 | arkworks | Circom |
|--------|-------|----------|--------|
| **Language** | Rust | Rust | DSL |
| **Arithmetization** | PLONKish | R1CS | R1CS |
| **Proving System** | PLONK variants | Groth16 | Groth16 |
| **Trusted Setup** | Universal (1x) | Per-circuit | Per-circuit |
| **Learning Curve** | Steep | Moderate | Easy |

## 2. Range Check Implementation Comparison

### Halo2 (Lookup Table) - 1 Constraint

```rust
// Single lookup constraint
meta.lookup("range", |meta| {
    let value = meta.query_advice(col, Rotation::cur());
    vec![(value, table)]  // Value must exist in pre-computed table
});
```

**How it works:**
- Pre-load table with values [0, 1, 2, ..., 255]
- Single constraint: "value exists in table"
- O(1) constraints regardless of bit width

### arkworks (Bit Decomposition) - ~64 Constraints

```rust
// Decompose into bits and verify
let bits = value.to_bits_le()?;  // Creates 64 Boolean constraints
for bit in bits.iter().skip(RANGE_BITS) {
    bit.enforce_equal(&Boolean::constant(false))?;  // High bits must be 0
}
```

**How it works:**
- Decompose value into individual bits
- Each bit requires 1 constraint (b * (1-b) = 0)
- O(n) constraints where n = bit width

### Circom (LessThan) - ~64 Constraints

```circom
component range = LessThan(64);
range.in[0] <== value;
range.in[1] <== 1 << 64;  // 2^64
range.out === 1;
```

**How it works:**
- Uses bit decomposition internally
- LessThan template from circomlib
- Similar constraint count to arkworks

## 3. Comparison Gate Implementation

### Halo2 - Custom Gate + Lookup

```rust
// Custom gate: diff = a - b + offset
meta.create_gate("comparison", |meta| {
    let diff = meta.query_advice(diff, Rotation::cur());
    let a = meta.query_advice(a, Rotation::cur());
    let b = meta.query_advice(b, Rotation::cur());
    vec![q * (diff - a + b - offset)]
});

// Then lookup to verify diff is in valid range
```

### arkworks - Bit Decomposition

```rust
let diff = &a - &b;
let offset = FpVar::constant(F::from(1u64 << (BITS - 1)));
let diff_shifted = diff + offset;
let diff_bits = diff_shifted.to_bits_le()?;
// Verify high bits are zero
```

### Circom - GreaterEqThan Template

```circom
component cmp = GreaterEqThan(64);
cmp.in[0] <== a;
cmp.in[1] <== b;
cmp.out === 1;
```

## 4. Performance Comparison

| Metric | Halo2 | arkworks | Circom |
|--------|-------|----------|--------|
| **Constraints (CollateralProof)** | ~100 | ~500 | ~500 |
| **Prove Time** | Medium | Fast | Fast |
| **Verify Time** | Fast | Very Fast | Very Fast |
| **Proof Size** | ~1KB | ~200B | ~200B |
| **Setup** | Universal | Per-circuit | Per-circuit |

## 5. Code Complexity Comparison

### Circuit Definition

**Halo2** (Most Verbose):
```rust
impl Circuit<Fp> for CollateralCircuit<Fp> {
    type Config = CollateralConfig<Fp>;
    type FloorPlanner = SimpleFloorPlanner;

    fn configure(meta: &mut ConstraintSystem<Fp>) -> Self::Config {
        // Define columns
        let value = meta.advice_column();
        // Define gates
        meta.create_gate("custom", |meta| { ... });
        // Define lookups
        meta.lookup("range", |meta| { ... });
    }

    fn synthesize(&self, config, layouter) -> Result<(), Error> {
        // Assign values to regions
        layouter.assign_region(|| "main", |region| { ... })?;
    }
}
```

**arkworks** (Medium):
```rust
impl ConstraintSynthesizer<F> for CollateralCircuit<F> {
    fn generate_constraints(self, cs: ConstraintSystemRef<F>) -> Result<(), SynthesisError> {
        let value = FpVar::new_witness(cs.clone(), || ...)?;
        value.enforce_equal(&expected)?;
    }
}
```

**Circom** (Most Concise):
```circom
template CollateralProof(BITS) {
    signal input collateral;
    signal input threshold;

    component cmp = GreaterEqThan(BITS);
    cmp.in[0] <== collateral;
    cmp.in[1] <== threshold;
    cmp.out === 1;
}
```

## 6. When to Use What

| Scenario | Recommended | Reason |
|----------|-------------|--------|
| L2 zkEVM (Scroll, Polygon) | **Halo2** | Native support, efficient recursion |
| Quick PoC / Hackathon | **Circom** | Fast development, large library |
| Custom Protocol Research | **arkworks** | Low-level control, academic standard |
| Production DeFi | **Halo2** or **Circom** | Battle-tested |
| Learning ZK | Circom → arkworks → Halo2 | Progressive complexity |

## 7. Interview Response

```
면접관: "왜 Halo2를 메인으로 선택했나요?"

나: "세 가지 스택을 직접 비교해봤습니다.

1. Range Check 효율성:
   - Halo2 lookup: 1개 constraint
   - arkworks/Circom bit decomposition: 64개 constraints
   - 64배 차이가 납니다.

2. 개발 경험:
   - Circom이 가장 빠르고 쉽지만, 복잡한 로직에 한계
   - arkworks는 R1CS 패러다임 깊이 이해에 좋음
   - Halo2는 learning curve 높지만 가장 유연

3. 산업 표준:
   - Scroll, Polygon zkEVM이 Halo2 사용
   - L2 코어 개발 준비를 위해 Halo2 선택

결론: PoC는 Circom, 프로덕션은 Halo2가 적합합니다."
```

## 8. Implementation Details

### This Project

| Circuit | Halo2 | arkworks | Circom |
|---------|-------|----------|--------|
| CollateralProof | ✅ Full | ✅ Full | ✅ Full |
| LTVProof | ✅ Full | - | - |
| LiquidationProof | ✅ Full | - | - |

### Key Files

```
circuits/
├── halo2/
│   ├── src/
│   │   ├── collateral.rs    # CollateralProof
│   │   ├── ltv.rs           # LTVProof
│   │   ├── liquidation.rs   # LiquidationProof
│   │   └── gadgets/         # Reusable components
│   └── benches/
│
├── arkworks/
│   └── src/
│       └── collateral.rs    # R1CS comparison
│
└── circom/
    └── collateral.circom    # DSL implementation
```

## 9. Conclusion

이 프로젝트에서 3스택을 모두 사용하는 이유:

1. **직접 비교**: 같은 로직으로 각 스택의 장단점 체험
2. **채용 대응**: 어떤 ZK 포지션이든 지원 가능
3. **깊은 이해**: 패러다임 차이(R1CS vs PLONKish) 이해
4. **실무 준비**: L2 코어 개발 (Halo2) + 빠른 프로토타이핑 (Circom)
