/**
 * ZK Proof Generation Hook
 *
 * WASM-based ZK proof generation and management for browser-side proving.
 * Supports collateral proofs, LTV proofs, and liquidation proofs.
 */

import { useState, useCallback, useRef, useEffect } from 'react';

// ZK WASM 모듈 타입 정의
interface ZKModule {
  generate_collateral_proof: (
    collateral: bigint,
    salt: bigint,
    threshold: bigint,
    commitment: Uint8Array
  ) => Promise<ProofData>;

  generate_ltv_proof: (
    collateral: bigint,
    debt: bigint,
    salt: bigint,
    maxLtv: bigint,
    commitment: Uint8Array
  ) => Promise<ProofData>;

  generate_liquidation_proof: (
    collateral: bigint,
    debt: bigint,
    price: bigint,
    salt: bigint,
    commitment: Uint8Array
  ) => Promise<ProofData>;

  compute_commitment: (value: bigint, salt: bigint) => Uint8Array;

  verify_proof: (proofType: string, proof: ProofData) => boolean;
}

// 증명 데이터 구조
export interface ProofData {
  a: [string, string];
  b: [[string, string], [string, string]];
  c: [string, string];
  publicInputs: string[];
}

// 증명 생성 결과
export interface ProofResult {
  proof: ProofData;
  commitment: string;
  generationTime: number;
}

// 훅 상태
interface ZKProofState {
  isLoading: boolean;
  isInitialized: boolean;
  error: string | null;
  progress: number;
}

// 증명 타입
export type ProofType = 'collateral' | 'ltv' | 'liquidation';

/**
 * ZK 증명 생성 훅
 *
 * @example
 * ```tsx
 * const { generateCollateralProof, isLoading } = useZKProof();
 *
 * const handleDeposit = async () => {
 *   const result = await generateCollateralProof(
 *     BigInt(10 * 1e18), // 10 ETH
 *     BigInt(500 * 1e6)  // $500 threshold
 *   );
 *   console.log('Proof:', result.proof);
 * };
 * ```
 */
export function useZKProof() {
  const [state, setState] = useState<ZKProofState>({
    isLoading: false,
    isInitialized: false,
    error: null,
    progress: 0,
  });

  const zkModuleRef = useRef<ZKModule | null>(null);
  const initPromiseRef = useRef<Promise<void> | null>(null);

  // WASM 모듈 초기화
  const initializeModule = useCallback(async () => {
    if (zkModuleRef.current) return;
    if (initPromiseRef.current) {
      await initPromiseRef.current;
      return;
    }

    initPromiseRef.current = (async () => {
      try {
        setState(s => ({ ...s, isLoading: true, progress: 10 }));

        // WASM module not available - using API-based proof generation instead
        // Client-side WASM proving is a future enhancement
        // For now, proofs are generated via backend API (see api.ts)
        console.info('ZK proofs will be generated via backend API');

        setState(s => ({ ...s, progress: 50 }));

        // Mark as initialized but without WASM module
        // Components should use api.generateCollateralProof() etc. instead
        zkModuleRef.current = null;

        setState(s => ({
          ...s,
          isLoading: false,
          isInitialized: true,
          progress: 100,
        }));
      } catch (err) {
        console.error('Failed to initialize ZK module:', err);
        setState(s => ({
          ...s,
          isLoading: false,
          error: 'ZK module initialization failed. Using API fallback.',
        }));
      }
    })();

    await initPromiseRef.current;
  }, []);

  // 컴포넌트 마운트 시 모듈 초기화
  useEffect(() => {
    initializeModule();
  }, [initializeModule]);

  // 암호학적으로 안전한 salt 생성
  const generateSalt = useCallback((): bigint => {
    const bytes = new Uint8Array(32);
    crypto.getRandomValues(bytes);

    // bytes를 BigInt로 변환
    let salt = BigInt(0);
    for (let i = 0; i < bytes.length; i++) {
      salt = (salt << BigInt(8)) | BigInt(bytes[i]);
    }

    return salt;
  }, []);

  // Commitment 계산
  const computeCommitment = useCallback(async (
    value: bigint,
    salt: bigint
  ): Promise<string> => {
    await initializeModule();

    if (!zkModuleRef.current) {
      throw new Error('ZK module not initialized');
    }

    const commitmentBytes = zkModuleRef.current.compute_commitment(value, salt);
    return '0x' + Array.from(commitmentBytes)
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');
  }, [initializeModule]);

  // Collateral Proof 생성
  const generateCollateralProof = useCallback(async (
    collateralWei: bigint,
    thresholdUsd: bigint,
    existingSalt?: bigint
  ): Promise<ProofResult> => {
    await initializeModule();

    if (!zkModuleRef.current) {
      throw new Error('ZK module not initialized');
    }

    setState(s => ({ ...s, isLoading: true, progress: 0, error: null }));

    try {
      const salt = existingSalt ?? generateSalt();

      setState(s => ({ ...s, progress: 20 }));

      // Commitment 계산
      const commitmentBytes = zkModuleRef.current.compute_commitment(collateralWei, salt);

      setState(s => ({ ...s, progress: 40 }));

      const startTime = performance.now();

      // 증명 생성
      const proof = await zkModuleRef.current.generate_collateral_proof(
        collateralWei,
        salt,
        thresholdUsd,
        commitmentBytes
      );

      const generationTime = performance.now() - startTime;

      setState(s => ({ ...s, progress: 100, isLoading: false }));

      const commitment = '0x' + Array.from(commitmentBytes)
        .map(b => b.toString(16).padStart(2, '0'))
        .join('');

      return {
        proof,
        commitment,
        generationTime,
      };
    } catch (err) {
      const error = err instanceof Error ? err.message : 'Proof generation failed';
      setState(s => ({ ...s, isLoading: false, error }));
      throw err;
    }
  }, [initializeModule, generateSalt]);

  // LTV Proof 생성
  const generateLTVProof = useCallback(async (
    collateralWei: bigint,
    debtUsdc: bigint,
    maxLtv: bigint,
    existingSalt?: bigint
  ): Promise<ProofResult> => {
    await initializeModule();

    if (!zkModuleRef.current) {
      throw new Error('ZK module not initialized');
    }

    setState(s => ({ ...s, isLoading: true, progress: 0, error: null }));

    try {
      const salt = existingSalt ?? generateSalt();

      const commitmentBytes = zkModuleRef.current.compute_commitment(collateralWei, salt);

      setState(s => ({ ...s, progress: 30 }));

      const startTime = performance.now();

      const proof = await zkModuleRef.current.generate_ltv_proof(
        collateralWei,
        debtUsdc,
        salt,
        maxLtv,
        commitmentBytes
      );

      const generationTime = performance.now() - startTime;

      setState(s => ({ ...s, progress: 100, isLoading: false }));

      const commitment = '0x' + Array.from(commitmentBytes)
        .map(b => b.toString(16).padStart(2, '0'))
        .join('');

      return {
        proof,
        commitment,
        generationTime,
      };
    } catch (err) {
      const error = err instanceof Error ? err.message : 'LTV proof generation failed';
      setState(s => ({ ...s, isLoading: false, error }));
      throw err;
    }
  }, [initializeModule, generateSalt]);

  // Liquidation Proof 생성
  const generateLiquidationProof = useCallback(async (
    collateralWei: bigint,
    debtUsdc: bigint,
    ethPriceUsd: bigint,
    existingSalt?: bigint
  ): Promise<ProofResult> => {
    await initializeModule();

    if (!zkModuleRef.current) {
      throw new Error('ZK module not initialized');
    }

    setState(s => ({ ...s, isLoading: true, progress: 0, error: null }));

    try {
      const salt = existingSalt ?? generateSalt();

      const commitmentBytes = zkModuleRef.current.compute_commitment(collateralWei, salt);

      setState(s => ({ ...s, progress: 30 }));

      const startTime = performance.now();

      const proof = await zkModuleRef.current.generate_liquidation_proof(
        collateralWei,
        debtUsdc,
        ethPriceUsd,
        salt,
        commitmentBytes
      );

      const generationTime = performance.now() - startTime;

      setState(s => ({ ...s, progress: 100, isLoading: false }));

      const commitment = '0x' + Array.from(commitmentBytes)
        .map(b => b.toString(16).padStart(2, '0'))
        .join('');

      return {
        proof,
        commitment,
        generationTime,
      };
    } catch (err) {
      const error = err instanceof Error ? err.message : 'Liquidation proof generation failed';
      setState(s => ({ ...s, isLoading: false, error }));
      throw err;
    }
  }, [initializeModule, generateSalt]);

  // 증명 검증 (로컬)
  const verifyProof = useCallback(async (
    proofType: ProofType,
    proof: ProofData
  ): Promise<boolean> => {
    await initializeModule();

    if (!zkModuleRef.current) {
      throw new Error('ZK module not initialized');
    }

    return zkModuleRef.current.verify_proof(proofType, proof);
  }, [initializeModule]);

  // Salt 저장 (localStorage - 실제로는 더 안전한 저장소 사용)
  const saveSalt = useCallback((key: string, salt: bigint) => {
    // 주의: localStorage는 보안에 취약함
    // 실제 프로덕션에서는 암호화된 저장소 또는 하드웨어 지갑 사용
    try {
      localStorage.setItem(`zk_salt_${key}`, salt.toString());
    } catch {
      console.warn('Failed to save salt to localStorage');
    }
  }, []);

  const loadSalt = useCallback((key: string): bigint | null => {
    try {
      const stored = localStorage.getItem(`zk_salt_${key}`);
      return stored ? BigInt(stored) : null;
    } catch {
      return null;
    }
  }, []);

  const clearSalt = useCallback((key: string) => {
    try {
      localStorage.removeItem(`zk_salt_${key}`);
    } catch {
      console.warn('Failed to clear salt from localStorage');
    }
  }, []);

  return {
    // 상태
    isLoading: state.isLoading,
    isInitialized: state.isInitialized,
    error: state.error,
    progress: state.progress,

    // 증명 생성
    generateCollateralProof,
    generateLTVProof,
    generateLiquidationProof,

    // 유틸리티
    computeCommitment,
    verifyProof,
    generateSalt,

    // Salt 관리
    saveSalt,
    loadSalt,
    clearSalt,

    // 초기화
    initializeModule,
  };
}

/**
 * Proof를 컨트랙트 호출 형식으로 변환
 */
export function formatProofForContract(proof: ProofData): {
  a: [bigint, bigint];
  b: [[bigint, bigint], [bigint, bigint]];
  c: [bigint, bigint];
} {
  return {
    a: [BigInt(proof.a[0]), BigInt(proof.a[1])],
    b: [
      [BigInt(proof.b[0][0]), BigInt(proof.b[0][1])],
      [BigInt(proof.b[1][0]), BigInt(proof.b[1][1])],
    ],
    c: [BigInt(proof.c[0]), BigInt(proof.c[1])],
  };
}

/**
 * ETH를 Wei로 변환
 */
export function ethToWei(eth: number | string): bigint {
  const ethStr = typeof eth === 'number' ? eth.toString() : eth;
  const [whole, decimal = ''] = ethStr.split('.');
  const paddedDecimal = decimal.padEnd(18, '0').slice(0, 18);
  return BigInt(whole + paddedDecimal);
}

/**
 * USDC를 기본 단위로 변환 (6 decimals)
 */
export function usdcToBase(usdc: number | string): bigint {
  const usdcStr = typeof usdc === 'number' ? usdc.toString() : usdc;
  const [whole, decimal = ''] = usdcStr.split('.');
  const paddedDecimal = decimal.padEnd(6, '0').slice(0, 6);
  return BigInt(whole + paddedDecimal);
}

export default useZKProof;
