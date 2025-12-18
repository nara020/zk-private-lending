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

## 각 문서 구성

```
모든 문서는 동일한 구조:

1. 문제 정의
   - 무엇을 증명하려는가?
   - 왜 어려운가?

2. 알고리즘 설명
   - 핵심 아이디어
   - 수학적 원리
   - 시각화 다이어그램

3. 코드 분석
   - 실제 Rust 코드
   - 라인별 설명

4. 예시 실행
   - 구체적인 숫자로 trace
   - 성공/실패 케이스

5. 면접 대비 Q&A
   - 예상 질문과 답변
```

## Halo2 vs arkworks 비교 요약

| 측면 | Halo2 (PLONKish) | arkworks (R1CS) |
|------|------------------|-----------------|
| **Range Check** | Lookup 1개 | Bit decomposition ~64개 |
| **Custom Gates** | 자유롭게 정의 | a·b=c 형태만 |
| **Setup** | Universal (1회) | Per-circuit |
| **Proof Size** | ~1KB | ~200B |
| **Learning Curve** | 높음 | 중간 |
| **L2 채택** | Scroll, Polygon | 일부 |

## 면접 핵심 키워드

```
1. PLONKish Arithmetization
   - Advice, Instance, Fixed columns
   - Selector, Custom Gate, Lookup Table

2. R1CS
   - Rank-1 Constraint System
   - a·b = c 형태의 제약
   - Bit decomposition

3. Commitment Scheme
   - Hiding (정보 숨김)
   - Binding (값 고정)

4. Soundness
   - 거짓 증명 불가능

5. Zero-Knowledge
   - Verifier가 추가 정보 획득 불가
```

## 프로덕션 체크리스트

- [ ] Poseidon Hash로 commitment 교체
- [ ] 암호학적 난수 생성기 (salt)
- [ ] Overflow 검증 (큰 값 처리)
- [ ] Circuit 최적화 (constraint 수 감소)
- [ ] Formal verification
- [ ] 보안 감사 (audit)
