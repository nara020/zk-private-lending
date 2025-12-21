# ZK Private Lending - 면접 대비 질문 모음

## 1. 프로젝트 개요 질문

### Q: 이 프로젝트를 한 문장으로 설명해주세요
**A:** "담보 금액을 공개하지 않고도 대출이 가능한 ZK 기반 프라이버시 보호 DeFi 렌딩 프로토콜입니다."

### Q: 왜 이 프로젝트를 만들었나요?
**A:**
1. **실제 문제**: 현재 DeFi 렌딩(Aave, Compound)은 모든 포지션이 온체인에 공개됨
   - 대형 홀더의 포지션 추적 가능
   - 청산 시점 예측 → MEV 봇에 의한 선행거래
   - 프라이버시 침해

2. **해결책**: ZK-SNARK로 "충분한 담보가 있다"는 것만 증명
   - 실제 금액은 공개하지 않음
   - 청산도 ZK proof로 실행

3. **학습 목적**:
   - ZK 기술 심층 이해 (Halo2, arkworks, Circom)
   - 풀스택 블록체인 개발 경험
   - 시스템 설계 역량 입증

---

## 2. ZK (영지식 증명) 질문

### Q: 영지식 증명이란 무엇인가요?
**A:** "어떤 사실이 참이라는 것을 증명하면서, 그 외의 정보는 전혀 공개하지 않는 암호학적 방법"

**예시:**
```
일반 증명: "내 담보는 10 ETH이고, 10 >= 5이므로 충분합니다"
            → 10 ETH라는 정보가 노출됨

ZK 증명:   "내 담보가 5 ETH 이상이라는 것을 증명합니다"
            → 실제 금액은 숨김, 충분하다는 사실만 검증
```

### Q: Halo2, arkworks, Circom의 차이점은?
**A:**

| 특성 | Halo2 (선택) | arkworks | Circom |
|-----|-------------|----------|--------|
| **패러다임** | PLONKish | R1CS | DSL → R1CS |
| **언어** | Rust | Rust | Circom DSL |
| **Range Check** | Lookup Table (1 constraint) | Bit Decomposition (~16) | LessThan template |
| **장점** | 효율적, L2 채택 (Scroll) | 학술적 검증 | 빠른 프로토타이핑 |
| **단점** | 학습 곡선 높음 | 제약 수 많음 | 최적화 한계 |

**선택 이유:**
- Halo2가 제약 수 최소화 → proof 생성 빠름
- Scroll, zkSync 등 L2에서 채택 → 실무 관련성
- 3가지 모두 구현 → 비교 분석 역량 입증

### Q: Groth16 vs PLONK 차이점은?
**A:**

| 특성 | Groth16 | PLONK |
|-----|---------|-------|
| **Trusted Setup** | 회로마다 필요 | Universal (1회) |
| **Proof 크기** | 128 bytes (가장 작음) | 384+ bytes |
| **검증 시간** | 빠름 | 약간 느림 |
| **유연성** | 고정 회로 | 동적 회로 가능 |

**이 프로젝트:** Solidity 검증은 Groth16 (EVM 호환), 회로는 Halo2/PLONK

### Q: Poseidon Hash를 사용하는 이유는?
**A:** ZK-friendly 해시 함수
- SHA256: ~25,000 constraints
- Poseidon: ~300 constraints
- **80배 이상 효율적** → proof 생성 시간 단축

---

## 3. 스마트 컨트랙트 질문

### Q: ZKVerifier 컨트랙트는 어떻게 작동하나요?
**A:**

```solidity
// Groth16 검증 공식
e(-A, B) · e(α, β) · e(L, γ) · e(C, δ) = 1

// 여기서:
// A, B, C = proof (증명자가 제출)
// α, β, γ, δ = verification key (trusted setup에서 생성)
// L = public input의 선형 조합
```

**EVM 프리컴파일 활용:**
- `0x06`: BN254 Point Addition (150 gas)
- `0x07`: BN254 Scalar Multiplication (6000 gas)
- `0x08`: BN254 Pairing (45000+ gas)

### Q: Commitment가 뭔가요?
**A:**
```
commitment = Poseidon(value, salt)

예시:
- value = 10 ETH
- salt = random()  ← 사용자만 앎
- commitment = 0x7a8b...

특성:
- Hiding: commitment만 봐서는 10 ETH인지 알 수 없음
- Binding: 나중에 5 ETH라고 주장 불가능
```

### Q: Reentrancy 공격 방어는?
**A:** `ReentrancyGuard` 사용
```solidity
function withdraw() external nonReentrant {
    // nonReentrant modifier가 재진입 방지
    (bool success, ) = msg.sender.call{value: amount}("");
}
```

**추가 방어:**
- Checks-Effects-Interactions 패턴
- 상태 변경 먼저, 외부 호출 나중에

---

## 4. 백엔드 질문

### Q: 왜 Rust + Axum을 선택했나요?
**A:**
1. **ZK 연동**: Halo2가 Rust로 작성됨 → 직접 호출 가능
2. **성능**: Proof 생성은 CPU 집약적 → Rust의 zero-cost abstraction
3. **안전성**: 메모리 안전성 보장
4. **비동기**: tokio 기반 Axum은 높은 동시성 처리

**대안 비교:**
- Node.js: WASM 변환 필요, 성능 오버헤드
- Go: ZK 라이브러리 생태계 부족
- Python: 성능 문제

### Q: Proof 생성을 백엔드에서 하는 이유는?
**A:**

**백엔드 생성 (현재):**
- ✅ 사용자 디바이스 성능과 무관
- ✅ Proving Key 안전 관리
- ❌ 서버에 private input 전송

**클라이언트 생성 (대안):**
- ✅ 완전한 프라이버시
- ❌ WASM 빌드 필요, 모바일에서 느림

**개선 계획:** 클라이언트 WASM 옵션 제공

### Q: 가격 오라클 공격은 어떻게 방어하나요?
**A:**
1. **다중 소스**: Chainlink + Uniswap TWAP
2. **TWAP**: 시간 가중 평균으로 순간 조작 방어
3. **변동 제한**: 1블록에 10% 이상 변동 거부
4. **서킷 브레이커**: 이상 감지 시 거래 중단

---

## 5. 시스템 설계 질문

### Q: 전체 아키텍처를 설명해주세요
**A:**
```
┌─────────────────┐     ┌─────────────────┐
│    Frontend     │────▶│   Backend API   │
│   (Next.js)     │     │   (Rust/Axum)   │
└────────┬────────┘     └────────┬────────┘
         │                       │
         │                       ▼
         │              ┌─────────────────┐
         │              │   PostgreSQL    │
         │              │   (Position DB) │
         │              └─────────────────┘
         │                       │
         ▼                       ▼
┌─────────────────────────────────────────┐
│           Ethereum Blockchain            │
│  ┌─────────┐ ┌─────────┐ ┌───────────┐ │
│  │Verifier │ │Registry │ │LendingPool│ │
│  └─────────┘ └─────────┘ └───────────┘ │
└─────────────────────────────────────────┘
```

### Q: 블록체인과 DB 데이터 불일치는 어떻게 처리하나요?
**A:** "Eventually Consistent" 패턴
1. **블록체인 = Source of Truth**
2. **DB = 읽기 캐시** (빠른 조회용)
3. **불일치 감지 시**: 블록체인에서 재동기화
4. **중요 작업**: 항상 블록체인에서 직접 검증

### Q: 확장성은 어떻게 확보하나요?
**A:**
1. **읽기 확장**: PostgreSQL 읽기 레플리카
2. **쓰기 확장**: 샤딩 (주소 기반)
3. **Proof 생성**: 별도 워커 서비스 분리
4. **캐싱**: Redis로 가격 데이터 캐시

---

## 6. 문제 해결 경험

### Q: 개발 중 가장 어려웠던 문제는?
**A:** "유한 필드에서 비교 연산 구현"

**문제:**
```
ZK 회로에서: a >= b 를 어떻게 증명?
유한 필드에서는 음수가 없음
a - b가 음수면? → 엄청 큰 양수가 됨
```

**해결:**
```
Offset 기법 사용:
diff = a - b + OFFSET
where OFFSET = 2^32

만약 a >= b: diff는 [OFFSET, OFFSET + MAX_VALUE] 범위
만약 a < b:  diff는 [0, OFFSET) 범위

→ diff가 특정 범위에 있는지 lookup table로 검증
```

### Q: 성능 최적화 경험은?
**A:** "Range Check 최적화"

**Before (비트 분해):**
```rust
// 8비트 값 검증 = 8개 constraint
for i in 0..8 {
    let bit = (value >> i) & 1;
    enforce!(bit * (bit - 1) == 0);  // bit은 0 또는 1
}
```

**After (Lookup Table):**
```rust
// 1개 constraint로 해결
lookup_table.contains(value)  // [0, 1, 2, ..., 255]
```

**결과:** 16배 constraint 감소 → proof 생성 시간 단축

---

## 7. 향후 계획 / 개선점

### Q: 현재 프로젝트의 한계점은?
**A:**
1. **대출 금액 노출**: USDC 전송이 온체인에 공개됨
   - 개선: 대출도 commitment로 숨기기

2. **중앙화된 Proof 생성**: 서버가 private input을 알게 됨
   - 개선: 클라이언트 WASM prover

3. **단일 담보 자산**: ETH만 지원
   - 개선: ERC20, NFT 담보 지원

### Q: 프로덕션 배포를 위해 필요한 것은?
**A:**
1. **보안 감사**: ZK 회로 + 스마트 컨트랙트
2. **Trusted Setup**: MPC 기반 Powers of Tau
3. **Chainlink 연동**: 실제 가격 피드
4. **프론트엔드**: 사용자 인터페이스
5. **테스트넷 배포**: Sepolia에서 테스트

---

## 8. 기술 심화 질문

### Q: BN254 곡선을 사용하는 이유는?
**A:** EVM 프리컴파일 지원
- 0x06, 0x07, 0x08 프리컴파일이 BN254 연산
- 가스 효율적 (pairing ~45000 gas)
- 대안(BLS12-381)은 프리컴파일 없음 → 비쌈

### Q: Trusted Setup의 위험성은?
**A:**
- "Toxic Waste" 문제: setup에 사용된 랜덤값을 아는 사람은 가짜 proof 생성 가능
- **해결책**: MPC (Multi-Party Computation)
  - 여러 참가자가 순차적으로 기여
  - 한 명이라도 정직하면 안전
  - Zcash Powers of Tau: 87명 참가

### Q: MEV 공격 방어는?
**A:** ZK로 본질적 방어
- **기존 DeFi**: 청산 시점 예측 가능 → MEV 봇 선행거래
- **ZK Lending**: 포지션 금액 숨김 → 청산 시점 예측 불가
- 추가 방어: Flashbots, private mempool

---

## 9. 협업 / 소프트 스킬

### Q: 코드 품질을 어떻게 유지했나요?
**A:**
1. **문서화**: 모든 함수에 Interview Q&A 포함
2. **테스트**: 단위 테스트 + 통합 테스트
3. **타입 안전**: Rust의 타입 시스템 활용
4. **에러 처리**: thiserror + anyhow 조합

### Q: 이 프로젝트에서 가장 자랑스러운 부분은?
**A:**
1. **3가지 ZK 스택 비교 구현**: 깊은 이해 입증
2. **풀스택 구현**: 회로 → 컨트랙트 → 백엔드
3. **상세한 문서화**: 면접에서 바로 설명 가능
4. **실제 문제 해결**: DeFi 프라이버시 문제 해결

---

## 핵심 키워드 정리

| 분야 | 핵심 키워드 |
|-----|-----------|
| **ZK** | Halo2, PLONKish, Lookup Table, Poseidon, Commitment |
| **암호학** | BN254, Groth16, Trusted Setup, Pairing |
| **스마트 컨트랙트** | EVM Precompile, ReentrancyGuard, Commitment Scheme |
| **백엔드** | Rust, Axum, tokio, SQLx, async |
| **아키텍처** | Layered Architecture, Event Sourcing, Eventually Consistent |
| **DeFi** | LTV, Liquidation, Oracle, MEV |
