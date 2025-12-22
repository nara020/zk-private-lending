/**
 * API Service - 백엔드 통신
 *
 * Interview Q&A:
 *
 * Q: 프론트엔드와 백엔드 통신 구조는?
 * A: REST API 사용
 *    - 가격 정보: GET /api/price
 *    - commitment 계산: POST /api/compute-commitment
 *    - ZK 증명 생성: POST /api/prove/*
 *    - 포지션 조회: GET /api/position/:address
 *
 * Q: ZK 증명은 왜 서버에서 생성하는가?
 * A: 1. 클라이언트 리소스 제한 (WASM 크기, 메모리)
 *    2. Proving Key 관리 용이
 *    3. 단, 민감 정보(salt)는 HTTPS로 안전하게 전송
 *    → 향후 클라이언트 사이드 증명도 가능
 */

const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3001';

interface PriceResponse {
  ethPrice: number;
  lastUpdated: string;
}

interface CommitmentResponse {
  commitment: string;
}

interface ProofResponse {
  proof: string;
  publicInputs: string[];
}

interface PositionResponse {
  collateralCommitment: string;
  debtCommitment: string;
  isActive: boolean;
}

interface PoolStatusResponse {
  totalCollateral: string;
  totalBorrowed: string;
  availableLiquidity: string;
  utilizationRate: number;
  interestRate: number;
  totalAccruedInterest: string;
  apy: number;
}

interface DebtInfoResponse {
  principal: string;
  interest: string;
  total: string;
}

interface LTVProofRequest {
  collateralAmount: string;
  collateralSalt: string;
  borrowAmount: string;
  ethPrice: string;
  maxLTV: string;
}

interface LiquidationProofRequest {
  collateralAmount: string;
  collateralSalt: string;
  debtAmount: string;
  ethPrice: string;
  liquidationThreshold: string;
}

async function fetchAPI<T>(
  endpoint: string,
  options?: RequestInit
): Promise<T> {
  const url = `${API_BASE_URL}${endpoint}`;

  const response = await fetch(url, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ message: 'Request failed' }));
    throw new Error(error.message || `HTTP error ${response.status}`);
  }

  return response.json();
}

export const api = {
  /**
   * ETH 가격 조회
   */
  getEthPrice: async (): Promise<PriceResponse> => {
    return fetchAPI<PriceResponse>('/api/price');
  },

  /**
   * Commitment 계산 (서버에서 해시 계산)
   *
   * Q: 왜 commitment를 서버에서 계산하는가?
   * A: Poseidon 해시가 클라이언트에서 무거울 수 있음
   *    하지만 salt는 클라이언트에서 생성 (보안)
   *    → 클라이언트에서 계산하는 옵션도 제공 가능
   */
  computeCommitment: async (
    amount: string,
    salt: string
  ): Promise<CommitmentResponse> => {
    return fetchAPI<CommitmentResponse>('/api/compute-commitment', {
      method: 'POST',
      body: JSON.stringify({ amount, salt }),
    });
  },

  /**
   * 담보 증명 생성
   */
  generateCollateralProof: async (
    amount: string,
    salt: string,
    commitment: string
  ): Promise<ProofResponse> => {
    return fetchAPI<ProofResponse>('/api/prove/collateral', {
      method: 'POST',
      body: JSON.stringify({ amount, salt, commitment }),
    });
  },

  /**
   * LTV 증명 생성
   *
   * Q: LTV 증명의 public inputs는?
   * A: 1. collateral_commitment (온체인에서 검증)
   *    2. max_ltv (75% = 75)
   *    3. borrow_amount_commitment (선택적)
   *    → 실제 담보액과 대출액은 비공개
   */
  generateLTVProof: async (params: LTVProofRequest): Promise<ProofResponse> => {
    return fetchAPI<ProofResponse>('/api/prove/ltv', {
      method: 'POST',
      body: JSON.stringify(params),
    });
  },

  /**
   * 청산 증명 생성
   */
  generateLiquidationProof: async (
    params: LiquidationProofRequest
  ): Promise<ProofResponse> => {
    return fetchAPI<ProofResponse>('/api/prove/liquidation', {
      method: 'POST',
      body: JSON.stringify(params),
    });
  },

  /**
   * 포지션 정보 조회 (온체인 데이터)
   *
   * Q: 왜 API에서 포지션을 조회하는가?
   * A: 온체인 commitment 정보 + 캐싱
   *    실제 금액은 로컬에만 있음
   */
  getPosition: async (address: string): Promise<PositionResponse> => {
    return fetchAPI<PositionResponse>(`/api/position/${address}`);
  },

  /**
   * 건강도 체크 (청산 위험 확인)
   */
  checkHealth: async (
    address: string
  ): Promise<{ healthFactor: number; isLiquidatable: boolean }> => {
    return fetchAPI<{ healthFactor: number; isLiquidatable: boolean }>(
      `/api/health/${address}`
    );
  },

  /**
   * 건강도 조회 (alias)
   */
  getHealth: async (
    address: string
  ): Promise<{ healthFactor: number; isLiquidatable: boolean }> => {
    return fetchAPI<{ healthFactor: number; isLiquidatable: boolean }>(
      `/api/health/${address}`
    );
  },

  /**
   * 풀 상태 조회 (이자율, 이용률 등)
   */
  getPoolStatus: async (): Promise<PoolStatusResponse> => {
    return fetchAPI<PoolStatusResponse>('/api/pool/status');
  },

  /**
   * 현재 APY 조회
   */
  getAPY: async (): Promise<{ apy: number }> => {
    return fetchAPI<{ apy: number }>('/api/pool/apy');
  },

  /**
   * 사용자 부채 정보 조회 (원금 + 이자)
   */
  getDebtInfo: async (address: string): Promise<DebtInfoResponse> => {
    return fetchAPI<DebtInfoResponse>(`/api/debt/${address}`);
  },

  /**
   * 청산 증명 요청 (내부용 alias)
   */
  proveLiquidation: async (
    params: {
      collateralAmount: string;
      collateralSalt: string;
      debtAmount: string;
      ethPrice: number;
      liquidationThreshold: number;
    }
  ): Promise<ProofResponse> => {
    return fetchAPI<ProofResponse>('/api/prove/liquidation', {
      method: 'POST',
      body: JSON.stringify({
        collateralAmount: params.collateralAmount,
        collateralSalt: params.collateralSalt,
        debtAmount: params.debtAmount,
        ethPrice: params.ethPrice.toString(),
        liquidationThreshold: params.liquidationThreshold.toString(),
      }),
    });
  },
};

// 클라이언트 사이드 commitment 계산 (선택적 사용)
// Poseidon 해시가 필요하므로 별도 라이브러리 필요
export async function computeCommitmentLocal(
  _amount: bigint,
  _salt: bigint
): Promise<string> {
  // TODO: circomlibjs 또는 snarkjs 사용하여 Poseidon 해시 계산
  // 현재는 서버 API 사용 권장
  throw new Error('Local commitment computation not implemented');
}
