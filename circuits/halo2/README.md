# Halo2 Circuits (Primary)

> PSE halo2_proofs 0.3 기반 ZK 회로 구현

## 📁 구조

```
halo2/
├── src/
│   ├── lib.rs              # 모듈 exports
│   ├── collateral.rs       # CollateralProof 회로
│   ├── ltv.rs              # LTVProof 회로
│   ├── liquidation.rs      # LiquidationProof 회로
│   ├── error.rs            # 에러 타입 및 검증
│   ├── tests.rs            # 통합 테스트
│   └── gadgets/
│       ├── mod.rs
│       ├── range_check.rs  # Lookup table 기반 범위 검증
│       ├── comparison.rs   # 대소 비교 게이트
│       └── poseidon.rs     # Poseidon 해시
├── benches/
│   └── circuit_benchmarks.rs
├── Cargo.toml
└── README.md
```

## 🔧 회로 목록

| 회로 | 용도 | Constraints | Public Inputs |
|-----|------|-------------|---------------|
| **CollateralProof** | 담보 >= threshold | ~50 | threshold, commitment |
| **LTVProof** | debt/collateral <= max_ltv | ~80 | max_ltv, coll_comm, debt_comm |
| **LiquidationProof** | health_factor < 1.0 | ~100 | price, liq_threshold, position_hash |

## 🎯 핵심 개념

### PLONKish Arithmetization

Halo2는 PLONKish 연산 체계를 사용합니다:

```
┌─────────────────────────────────────────────────────────────┐
│                    PLONKish Table                            │
├───────────┬───────────┬───────────┬───────────┬─────────────┤
│ Selector  │  Advice   │  Advice   │  Fixed    │  Instance   │
│ (gates)   │ (private) │ (private) │ (const)   │ (public)    │
├───────────┼───────────┼───────────┼───────────┼─────────────┤
│    1      │   a_0     │   b_0     │   c_0     │   x_0       │
│    0      │   a_1     │   b_1     │   c_1     │   x_1       │
│    1      │   a_2     │   b_2     │   c_2     │             │
│   ...     │   ...     │   ...     │   ...     │   ...       │
└───────────┴───────────┴───────────┴───────────┴─────────────┘
```

### Column Types

```rust
// Instance: 공개 입력 (검증자가 알아야 함)
let threshold = meta.instance_column();

// Advice: 비밀 증인 (증명자만 앎)
let collateral = meta.advice_column();

// Fixed: 상수 (lookup table 등)
let lookup_table = meta.fixed_column();

// Selector: 게이트 활성화 플래그
let s_comparison = meta.selector();
```

### Lookup Tables (핵심 최적화)

R1CS와의 가장 큰 차이점:

```
R1CS (arkworks):
  - Range check: 비트 분해 필요
  - 64비트 값 → ~64 constraints

PLONKish (Halo2):
  - Range check: Lookup table 사용
  - 64비트 값 → 1 constraint (!)

결과: 16~64배 효율적
```

### Custom Gates

```rust
meta.create_gate("comparison", |meta| {
    let s = meta.query_selector(s_comparison);
    let a = meta.query_advice(col_a, Rotation::cur());
    let b = meta.query_advice(col_b, Rotation::cur());
    let diff = meta.query_advice(col_diff, Rotation::cur());
    let offset = meta.query_fixed(col_offset, Rotation::cur());

    // diff = a - b + offset
    // offset makes negative values positive (for range check)
    vec![s * (a - b + offset - diff)]
});
```

## 📊 Gadgets

### 1. RangeCheckChip

Lookup table을 사용한 효율적인 범위 검증:

```rust
// 8비트 lookup table: [0, 1, 2, ..., 255]
// 값이 테이블에 있으면 유효

pub fn range_check(&self, value: Value<Assigned<F>>) -> Result<(), Error> {
    // 1 constraint로 범위 검증!
    self.config.table.lookup(value)
}
```

### 2. ComparisonChip

유한 필드에서 대소 비교:

```rust
// 문제: 유한 필드에서 a - b가 음수면?
//       → 매우 큰 양수가 됨 (modular arithmetic)

// 해결: Offset 기법
//       diff = a - b + OFFSET
//       OFFSET = 2^32
//
//       a >= b: diff ∈ [OFFSET, OFFSET + MAX]
//       a < b:  diff ∈ [0, OFFSET)
//
//       → diff의 범위로 비교 결과 판단
```

### 3. PoseidonChip

ZK-friendly 해시:

```rust
// Poseidon vs SHA256
// SHA256: ~25,000 constraints
// Poseidon: ~300 constraints
// → 80배 이상 효율적!

pub fn hash(&self, inputs: [Value<Assigned<F>>; 2]) -> Value<Assigned<F>> {
    // halo2_gadgets::poseidon 사용
    self.poseidon.hash(inputs)
}
```

## 🧪 테스트

```bash
# 전체 테스트
cargo test

# 상세 출력
cargo test -- --nocapture

# 특정 테스트
cargo test test_collateral_proof

# 벤치마크
cargo bench
```

### 테스트 구조

```rust
#[test]
fn test_collateral_proof_valid() {
    // 1. 테스트 값 설정
    let collateral = 1000u64;
    let threshold = 500u64;
    let salt = 12345u64;

    // 2. Commitment 계산
    let commitment = poseidon_hash(collateral, salt);

    // 3. 회로 생성
    let circuit = CollateralCircuit::new(
        collateral, threshold, salt, commitment
    );

    // 4. MockProver로 검증
    let prover = MockProver::run(K, &circuit, vec![
        vec![threshold.into(), commitment]
    ]).unwrap();

    // 5. 결과 확인
    assert_eq!(prover.verify(), Ok(()));
}
```

## 📈 성능 비교

| 메트릭 | Halo2 | arkworks | Circom |
|-------|-------|----------|--------|
| **CollateralProof Constraints** | ~50 | ~200 | ~150 |
| **Range Check (64-bit)** | 1 | ~64 | ~64 |
| **Proof Size** | 384 bytes | 128 bytes | 128 bytes |
| **Proving Time** | ~1s | ~2s | ~1.5s |
| **Verification Time** | ~5ms | ~3ms | ~3ms |

## FAQ

### Why Halo2?

1. **Efficiency**: Lookup table enables single-constraint range check
2. **Flexibility**: Custom gates for complex logic
3. **Industry Standard**: Used by Scroll, zkSync, and other L2s

### Trusted Setup?

Halo2 uses "Universal Setup":
- No per-circuit setup required
- KZG commitment-based SRS (Structured Reference String)
- One-time Powers of Tau ceremony

### PlonK vs Groth16

| Aspect | PlonK | Groth16 |
|--------|-------|---------|
| Setup | Universal (once) | Per-circuit |
| Proof Size | 384+ bytes | 128 bytes |
| Verification | Slightly slower | Fastest |
| Flexibility | High | Low |

### Circuit Optimization Techniques

1. **Lookup Usage**: Use lookup instead of bit decomposition for range checks
2. **Custom Gates**: Express complex logic in single gate
3. **Column Reuse**: Share columns across regions
4. **Minimize Rotations**: Adjacent row access is most efficient

## 🔗 참고 자료

- [PSE Halo2 Book](https://zcash.github.io/halo2/)
- [Scroll Halo2 Implementation](https://github.com/scroll-tech/halo2)
- [Halo2 Awesome](https://github.com/zcash/halo2-awesome)
- [ZK Learning Resources](https://learn.0xparc.org/)

## 🚀 다음 단계

1. **Proving Key Export**: 프로덕션용 key 생성
2. **Batch Proving**: 여러 proof 한번에 생성
3. **GPU 가속**: CUDA로 MSM 최적화
4. **Audit**: 보안 감사
