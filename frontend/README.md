# ZK Private Lending Frontend

React + TypeScript + Vite frontend for ZK-Private Lending.

## Tech Stack

| Technology | Purpose |
|------------|---------|
| React 18 | UI library with Concurrent Features |
| TypeScript | Type safety and better DX |
| Vite | Fast ESM-based dev server |
| Zustand | Lightweight state management (~1KB) |
| React Query | Server state management with caching |
| ethers.js v6 | Web3 interactions with native BigInt |

## Privacy Considerations

1. **Private Data**: salt and actual amounts stored locally only (localStorage)
2. **Commitment Display**: On-chain data shows only commitments; actual values restored locally
3. **Proof Generation UX**: ZK proof generation takes 5-30 seconds; show clear loading states

## 프로젝트 구조

```
frontend/
├── src/
│   ├── components/          # React 컴포넌트
│   │   ├── WalletConnect.tsx   # 지갑 연결 버튼
│   │   ├── PositionCard.tsx    # 포지션 표시
│   │   ├── DepositForm.tsx     # 담보 예치
│   │   ├── BorrowForm.tsx      # 대출 실행
│   │   └── RepayForm.tsx       # 상환 및 인출
│   ├── hooks/
│   │   └── useWallet.ts     # 지갑 상태 관리 (Zustand)
│   ├── services/
│   │   ├── api.ts           # 백엔드 API 클라이언트
│   │   └── contracts.ts     # 컨트랙트 상호작용
│   ├── App.tsx              # 메인 앱 컴포넌트
│   ├── main.tsx             # 엔트리 포인트
│   └── index.css            # Tailwind CSS
├── package.json
├── vite.config.ts
├── tailwind.config.js
├── tsconfig.json
└── README.md
```

## 설치 및 실행

```bash
# 의존성 설치
npm install

# 개발 서버 실행
npm run dev

# 프로덕션 빌드
npm run build

# 빌드 미리보기
npm run preview

# 타입 체크
npm run typecheck

# 린트
npm run lint
```

## 환경 변수

```bash
# .env 파일 생성
cp .env.example .env

# 필수 설정
VITE_API_URL=http://localhost:3001
VITE_LENDING_POOL_ADDRESS=0x...
VITE_USDC_ADDRESS=0x...
VITE_CHAIN_ID=31337
```

## 주요 기능 흐름

### 1. 담보 예치 (Deposit)

```
사용자 → 금액 입력
      → 랜덤 salt 생성 (crypto.getRandomValues)
      → API에서 commitment 계산
      → 컨트랙트 deposit 호출 (ETH + commitment)
      → 로컬에 salt 저장
```

### 2. 대출 (Borrow)

```
사용자 → 대출 금액 입력
      → 최대 대출 가능액 확인 (LTV 75%)
      → API에서 LTV 증명 생성
      → 컨트랙트 borrow 호출 (금액 + 증명)
      → USDC 수령
```

### 3. 상환 (Repay)

```
사용자 → 상환 금액 입력
      → USDC approve
      → 컨트랙트 repay 호출
      → 전액 상환 시 담보 인출 가능
```

## 보안 고려사항

### 민감 정보 처리

```typescript
// ❌ 나쁜 예: 민감 정보 노출
console.log('Salt:', salt);  // 절대 금지!

// ✅ 좋은 예: 로컬에만 저장
localStorage.setItem(`position_${address}`, JSON.stringify({
  collateralSalt: salt.toString(),  // 브라우저에만 저장
}));
```

### 트랜잭션 검증

```typescript
// 항상 예상 결과 확인
const tx = await contract.deposit(commitment, { value: amountWei });
const receipt = await tx.wait();

if (receipt.status !== 1) {
  throw new Error('Transaction failed');
}
```

## 개발 팁

### 1. MetaMask 테스트

```javascript
// 로컬 네트워크 추가
await window.ethereum.request({
  method: 'wallet_addEthereumChain',
  params: [{
    chainId: '0x7A69',  // 31337
    chainName: 'Localhost',
    rpcUrls: ['http://localhost:8545'],
  }],
});
```

### 2. 디버깅

```typescript
// React Query DevTools 활성화 (개발 모드)
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';

// 컴포넌트에 추가
<ReactQueryDevtools initialIsOpen={false} />
```

### 3. 타입 안전성

```typescript
// 컨트랙트 ABI에서 타입 생성
import { ZKLendingPool } from '../types/contracts';

// 타입 체크된 함수 호출
const contract: ZKLendingPool = new Contract(...);
await contract.deposit(commitment);  // 타입 검증됨
```

## 향후 개선 사항

1. **클라이언트 사이드 증명**
   - WASM 기반 증명 생성
   - salt가 서버로 전송되지 않음

2. **하드웨어 월렛 지원**
   - Ledger, Trezor 연동

3. **다중 네트워크 지원**
   - Mainnet, Arbitrum, Optimism

4. **포지션 백업/복구**
   - 암호화된 백업 파일 생성
   - QR 코드 백업
