/**
 * Contract Service - Smart Contract Interactions
 *
 * Handles all interactions with the ZK Lending Pool and USDC contracts.
 * Built with ethers.js v6 for type-safe contract calls.
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
   * Deposit ETH collateral with privacy commitment
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
   * Borrow USDC with ZK proof of LTV compliance
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
   * Execute liquidation with ZK proof
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
