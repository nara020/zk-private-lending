# ZK Stack Comparison: Halo2 vs arkworks vs Circom

> 동일한 CollateralProof 회로를 3개 스택으로 구현하여 비교 분석

## 1. Overview

| Aspect | Halo2 | arkworks | Circom |
|--------|-------|----------|--------|
| **Language** | Rust | Rust | DSL |
| **Arithmetization** | PLONKish | R1CS | R1CS |
| **Proving System** | PLONK variants | Groth16 | Groth16 |
| **Trusted Setup** | Universal (1x) | Per-circuit | Per-circuit |

## 2. Range Check Comparison (0-255)

### Halo2 (Lookup)
```rust
meta.lookup("range", |meta| {
    let value = meta.query_advice(col, Rotation::cur());
    vec![(value, table)]
});
// 1 constraint
```

### arkworks (Bit Decomposition)
```rust
let bits = UInt8::new_witness_vec(cs, &[value])?;
// ~16 constraints (8 bit allocation + 8 boolean checks)
```

### Circom (LessThan)
```circom
component lt = LessThan(8);
lt.in[0] <== value;
lt.in[1] <== 256;
lt.out === 1;
// ~8 constraints
```

## 3. Performance Comparison

| Metric | Halo2 | arkworks | Circom |
|--------|-------|----------|--------|
| **Constraint Count** | Lowest | Medium | Medium |
| **Prove Time** | Medium | Fast | Fast |
| **Verify Time** | Fast | Fast | Fast |
| **Proof Size** | ~1KB | ~200B | ~200B |

## 4. Developer Experience

### Halo2
- **Pros**: Efficient circuits, no per-circuit setup
- **Cons**: Steep learning curve, complex debugging
- **Best For**: L2 core development, complex circuits

### arkworks
- **Pros**: Low-level control, Rust ecosystem
- **Cons**: Verbose, requires ZK + Rust expertise
- **Best For**: Custom protocols, research

### Circom
- **Pros**: Fast prototyping, large library ecosystem
- **Cons**: Less flexible, DSL limitations
- **Best For**: Standard circuits, quick PoCs

## 5. When to Use What

| Scenario | Recommended |
|----------|-------------|
| L2 zkEVM development | Halo2 |
| Custom ZK protocol | arkworks |
| Quick prototype | Circom |
| Production DeFi | Halo2 or Circom |
| Learning ZK | Circom → arkworks → Halo2 |

## 6. Conclusion

이 프로젝트에서 3스택을 모두 사용하는 이유:
1. **직접 비교**: 같은 로직으로 각 스택의 장단점 체험
2. **채용 대응**: 어떤 ZK 포지션이든 지원 가능
3. **깊은 이해**: 패러다임 차이(R1CS vs PLONKish) 이해
