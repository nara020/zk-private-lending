# ZK-Private Lending 알고리즘 설명서

## 목차

| # | 파일 | 주제 | 핵심 내용 |
|---|------|------|----------|
| 01 | [01_RANGE_CHECK.md](./01_RANGE_CHECK.md) | Range Check | Halo2 Lookup vs R1CS Bit Decomposition |
| 02 | [02_COMPARISON.md](./02_COMPARISON.md) | 비교 연산 | Offset 기법으로 유한체에서 >= 증명 |
| 03 | [03_COLLATERAL_PROOF.md](./03_COLLATERAL_PROOF.md) | 담보 증명 | collateral >= threshold + commitment |
| 04 | [04_LTV_PROOF.md](./04_LTV_PROOF.md) | LTV 증명 | debt/collateral <= max_ltv |
| 05 | [05_LIQUIDATION_PROOF.md](./05_LIQUIDATION_PROOF.md) | 청산 증명 | Health Factor < 1.0 |

## 학습 순서 추천

```
1. Range Check (01) - 기본 빌딩 블록
   └── Lookup Table의 원리 이해

2. Comparison (02) - Range Check 활용
   └── Offset 기법 이해

3. Collateral Proof (03) - 첫 번째 실제 회로
   └── 전체 회로 구조 이해

4. LTV Proof (04) - 응용 회로
   └── 나눗셈 → 곱셈 변환

5. Liquidation Proof (05) - 고급 회로
   └── 실제 DeFi 시나리오
```

## Document Structure

Each document follows a consistent structure:

1. **Problem Definition** - What is being proved and why it's challenging
2. **Algorithm Explanation** - Core ideas, mathematical principles, visualizations
3. **Code Analysis** - Actual Rust implementation with line-by-line explanation
4. **Example Execution** - Concrete numerical traces with success/failure cases
5. **Implementation Notes** - Design decisions and production considerations

## Halo2 vs arkworks 비교 요약

| 측면 | Halo2 (PLONKish) | arkworks (R1CS) |
|------|------------------|-----------------|
| **Range Check** | Lookup 1개 | Bit decomposition ~64개 |
| **Custom Gates** | 자유롭게 정의 | a·b=c 형태만 |
| **Setup** | Universal (1회) | Per-circuit |
| **Proof Size** | ~1KB | ~200B |
| **Learning Curve** | 높음 | 중간 |
| **L2 채택** | Scroll, Polygon | 일부 |

## Key Technical Concepts

| Concept | Description |
|---------|-------------|
| PLONKish Arithmetization | Advice, Instance, Fixed columns; Selector, Custom Gate, Lookup Table |
| R1CS | Rank-1 Constraint System; a·b = c constraints; Bit decomposition |
| Commitment Scheme | Hiding (information concealment) + Binding (value locking) |
| Soundness | False proofs are computationally infeasible |
| Zero-Knowledge | Verifier learns nothing beyond statement validity |

## Production Checklist

- [ ] Poseidon Hash로 commitment 교체
- [ ] 암호학적 난수 생성기 (salt)
- [ ] Overflow 검증 (큰 값 처리)
- [ ] Circuit 최적화 (constraint 수 감소)
- [ ] Formal verification
- [ ] 보안 감사 (audit)
