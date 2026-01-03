/**
 * API Service - Backend Communication
 *
 * REST API client for ZK Private Lending backend services.
 * Handles price feeds, commitment computation, ZK proof generation, and position queries.
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
   * Compute commitment hash on the server
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
   * Generate LTV proof for borrowing
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
   * Get position information (on-chain data)
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
