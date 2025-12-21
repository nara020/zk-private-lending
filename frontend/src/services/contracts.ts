/**
 * Contract Service - 스마트 컨트랙트 상호작용
 *
 * Interview Q&A:
 *
 * Q: ethers v6의 주요 변경점은?
 * A: 1. BigNumber → bigint (네이티브)
 *    2. Provider/Signer 분리 강화
 *    3. Contract 인터페이스 개선
 *    4. 더 나은 TypeScript 지원
 *
 * Q: 컨트랙트 인터랙션 에러 처리는?
 * A: 1. 가스 추정 실패 → 사용자에게 명확한 에러
 *    2. revert → 컨트랙트 에러 메시지 파싱
 *    3. 트랜잭션 실패 → receipt 확인 후 처리
 */

import {
  Contract,
  JsonRpcSigner,
  formatUnits,
} from 'ethers';

// 컨트랙트 주소 (환경변수에서 로드)
const LENDING_POOL_ADDRESS = import.meta.env.VITE_LENDING_POOL_ADDRESS || '0x0000000000000000000000000000000000000000';
const USDC_ADDRESS = import.meta.env.VITE_USDC_ADDRESS || '0x0000000000000000000000000000000000000000';

// ABI (필요한 함수만 포함)
const LENDING_POOL_ABI = [
  // Deposit
  'function deposit(bytes32 commitment) external payable',

  // Borrow
  'function borrow(uint256 amount, bytes calldata proof, bytes32[] calldata publicInputs) external',

  // Repay
  'function repay(uint256 amount) external',

  // Withdraw
  'function withdraw() external',

  // Liquidate
  'function liquidate(address user, bytes calldata proof, bytes32[] calldata publicInputs) external',

  // View functions
  'function positions(address user) external view returns (bytes32 collateralCommitment, bytes32 debtCommitment, bool isActive)',
  'function getPosition(address user) external view returns (tuple(bytes32 collateralCommitment, bytes32 debtCommitment, bool isActive))',

  // Events
  'event Deposited(address indexed user, bytes32 commitment)',
  'event Borrowed(address indexed user, uint256 amount)',
  'event Repaid(address indexed user, uint256 amount)',
  'event Withdrawn(address indexed user)',
  'event Liquidated(address indexed user, address indexed liquidator)',
];

const ERC20_ABI = [
  'function approve(address spender, uint256 amount) external returns (bool)',
  'function allowance(address owner, address spender) external view returns (uint256)',
  'function balanceOf(address account) external view returns (uint256)',
];

function getLendingPoolContract(signer: JsonRpcSigner): Contract {
  return new Contract(LENDING_POOL_ADDRESS, LENDING_POOL_ABI, signer);
}

function getUSDCContract(signer: JsonRpcSigner): Contract {
  return new Contract(USDC_ADDRESS, ERC20_ABI, signer);
}

export const contracts = {
  /**
   * ETH 담보 예치
   *
   * Q: deposit 트랜잭션에 포함되는 정보는?
   * A: 1. value: 예치할 ETH 금액 (공개)
   *    2. commitment: Hash(amount, salt) (공개)
   *    → 온체인에서는 commitment만으로는 금액 알 수 없음
   *    → 하지만 ETH 전송량은 공개됨 (한계점)
   */
  deposit: async (
    signer: JsonRpcSigner,
    amountWei: bigint,
    commitment: string
  ) => {
    const contract = getLendingPoolContract(signer);

    // 가스 추정
    const gasEstimate = await contract.deposit.estimateGas(commitment, {
      value: amountWei,
    });

    // 10% 여유
    const gasLimit = (gasEstimate * 110n) / 100n;

    const tx = await contract.deposit(commitment, {
      value: amountWei,
      gasLimit,
    });

    return tx;
  },

  /**
   * USDC 대출
   *
   * Q: borrow에서 proof는 무엇을 증명하는가?
   * A: LTV 조건 만족 증명
   *    - "담보 가치 >= 대출액 / LTV"
   *    - 실제 담보액 공개 없이 조건만 증명
   */
  borrow: async (
    signer: JsonRpcSigner,
    amountWei: bigint,
    proof: string,
    publicInputs: string[]
  ) => {
    const contract = getLendingPoolContract(signer);

    // bytes32[] 형식으로 변환
    const publicInputsBytes32 = publicInputs.map(input => {
      // 이미 0x로 시작하면 그대로, 아니면 패딩
      if (input.startsWith('0x')) {
        return input.padEnd(66, '0').slice(0, 66);
      }
      return '0x' + input.padStart(64, '0');
    });

    const gasEstimate = await contract.borrow.estimateGas(
      amountWei,
      proof,
      publicInputsBytes32
    );

    const gasLimit = (gasEstimate * 110n) / 100n;

    const tx = await contract.borrow(amountWei, proof, publicInputsBytes32, {
      gasLimit,
    });

    return tx;
  },

  /**
   * 대출 상환
   */
  repay: async (signer: JsonRpcSigner, amountWei: bigint) => {
    const contract = getLendingPoolContract(signer);

    const gasEstimate = await contract.repay.estimateGas(amountWei);
    const gasLimit = (gasEstimate * 110n) / 100n;

    const tx = await contract.repay(amountWei, { gasLimit });
    return tx;
  },

  /**
   * USDC approve
   */
  approveUSDC: async (signer: JsonRpcSigner, amountWei: bigint) => {
    const usdc = getUSDCContract(signer);

    // 현재 allowance 확인
    const address = await signer.getAddress();
    const currentAllowance = await usdc.allowance(address, LENDING_POOL_ADDRESS);

    if (currentAllowance >= amountWei) {
      return null; // 이미 충분한 allowance
    }

    const tx = await usdc.approve(LENDING_POOL_ADDRESS, amountWei);
    await tx.wait();
    return tx;
  },

  /**
   * 담보 인출
   */
  withdraw: async (signer: JsonRpcSigner) => {
    const contract = getLendingPoolContract(signer);

    const gasEstimate = await contract.withdraw.estimateGas();
    const gasLimit = (gasEstimate * 110n) / 100n;

    const tx = await contract.withdraw({ gasLimit });
    return tx;
  },

  /**
   * 청산 실행
   *
   * Q: 청산자는 어떻게 청산 가능 여부를 아는가?
   * A: 1. 온체인: 사용자 포지션의 commitment만 공개
   *    2. 오프체인: 오라클에서 가격 정보 + 청산 조건 공개
   *    3. 청산자가 증명 생성하여 청산 시도
   *    → 실제 담보액 모르고도 청산 가능 여부 판단
   */
  liquidate: async (
    signer: JsonRpcSigner,
    userAddress: string,
    proof: string,
    publicInputs: string[]
  ) => {
    const contract = getLendingPoolContract(signer);

    const publicInputsBytes32 = publicInputs.map(input => {
      if (input.startsWith('0x')) {
        return input.padEnd(66, '0').slice(0, 66);
      }
      return '0x' + input.padStart(64, '0');
    });

    const gasEstimate = await contract.liquidate.estimateGas(
      userAddress,
      proof,
      publicInputsBytes32
    );

    const gasLimit = (gasEstimate * 110n) / 100n;

    const tx = await contract.liquidate(userAddress, proof, publicInputsBytes32, {
      gasLimit,
    });

    return tx;
  },

  /**
   * 포지션 조회
   */
  getPosition: async (signer: JsonRpcSigner, userAddress: string) => {
    const contract = getLendingPoolContract(signer);
    const position = await contract.positions(userAddress);

    return {
      collateralCommitment: position[0] as string,
      debtCommitment: position[1] as string,
      isActive: position[2] as boolean,
    };
  },

  /**
   * USDC 잔액 조회
   */
  getUSDCBalance: async (signer: JsonRpcSigner, address: string) => {
    const usdc = getUSDCContract(signer);
    const balance = await usdc.balanceOf(address);
    return formatUnits(balance, 6);
  },
};

// 에러 메시지 파싱 헬퍼
export function parseContractError(error: any): string {
  // ethers v6 에러 구조
  if (error.reason) {
    return error.reason;
  }

  if (error.message) {
    // revert 메시지 추출
    const match = error.message.match(/reason="([^"]+)"/);
    if (match) {
      return match[1];
    }

    // 일반적인 에러 메시지
    if (error.message.includes('insufficient funds')) {
      return 'Insufficient ETH balance';
    }
    if (error.message.includes('user rejected')) {
      return 'Transaction rejected by user';
    }
  }

  return 'Transaction failed';
}
