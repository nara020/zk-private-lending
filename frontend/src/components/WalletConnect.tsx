/**
 * WalletConnect Component - 지갑 연결 버튼
 */

import { useWallet } from '../hooks/useWallet';
import { Wallet, LogOut, Loader2 } from 'lucide-react';
import toast from 'react-hot-toast';

// 로컬 Hardhat 네트워크 설정
const LOCALHOST_CHAIN_ID = import.meta.env.VITE_CHAIN_ID || '31337';
const LOCALHOST_NETWORK = {
  chainId: `0x${parseInt(LOCALHOST_CHAIN_ID).toString(16)}`,
  chainName: import.meta.env.VITE_NETWORK_NAME || 'Localhost 8545',
  nativeCurrency: {
    name: 'Ethereum',
    symbol: 'ETH',
    decimals: 18,
  },
  rpcUrls: [import.meta.env.VITE_RPC_URL || 'http://127.0.0.1:8545'],
};

// 네트워크 전환/추가 함수
async function switchToLocalNetwork(): Promise<boolean> {
  if (!window.ethereum) return false;

  try {
    // 먼저 네트워크 전환 시도
    await window.ethereum.request({
      method: 'wallet_switchEthereumChain',
      params: [{ chainId: LOCALHOST_NETWORK.chainId }],
    });
    return true;
  } catch (switchError: any) {
    // 네트워크가 없으면 추가 시도
    if (switchError.code === 4902) {
      // MetaMask에서 localhost는 수동 추가 필요
      // 사용자에게 안내 메시지 표시
      alert(`MetaMask에서 네트워크를 수동으로 추가해주세요:

1. MetaMask 열기 → 네트워크 선택 → "네트워크 추가"
2. 다음 정보 입력:
   - 네트워크 이름: Localhost 8545
   - RPC URL: http://127.0.0.1:8545
   - 체인 ID: 31337
   - 통화 기호: ETH

추가 후 다시 연결해주세요.`);
      return false;
    }
    // 사용자가 거부한 경우
    if (switchError.code === 4001) {
      return false;
    }
    console.error('Failed to switch network:', switchError);
    return false;
  }
}

export function WalletConnect() {
  const { address, isConnected, isConnecting, balance, chainId, connect, disconnect } = useWallet();

  const expectedChainId = parseInt(LOCALHOST_CHAIN_ID);
  const isWrongNetwork = isConnected && chainId !== expectedChainId;

  const handleConnect = async () => {
    // MetaMask 설치 확인
    if (typeof window.ethereum === 'undefined') {
      toast.error('MetaMask를 설치해주세요!', {
        duration: 5000,
        icon: '🦊',
      });
      window.open('https://metamask.io/download/', '_blank');
      return;
    }

    try {
      // 먼저 연결 시도 (네트워크는 나중에 확인)
      await connect();
      toast.success('지갑이 연결되었습니다!', { icon: '✅' });
    } catch (error: any) {
      console.error('Wallet connection error:', error);

      // 사용자가 연결 거부한 경우
      if (error.code === 4001) {
        toast.error('연결이 거부되었습니다');
      } else {
        toast.error(error.message || '지갑 연결에 실패했습니다');
      }
    }
  };

  const handleSwitchNetwork = async () => {
    const switched = await switchToLocalNetwork();
    if (switched) {
      toast.success('네트워크가 전환되었습니다!');
      window.location.reload();
    } else {
      toast.error('네트워크 전환 실패');
    }
  };

  if (isConnecting) {
    return (
      <button
        disabled
        className="flex items-center space-x-2 rounded-lg bg-purple-600/50 px-4 py-2 text-white"
      >
        <Loader2 className="h-4 w-4 animate-spin" />
        <span>Connecting...</span>
      </button>
    );
  }

  // 잘못된 네트워크 경고
  if (isWrongNetwork) {
    return (
      <button
        onClick={handleSwitchNetwork}
        className="flex items-center space-x-2 rounded-lg bg-orange-600 px-4 py-2 font-medium text-white transition-all hover:bg-orange-500"
      >
        <span>⚠️ Switch to Localhost</span>
      </button>
    );
  }

  if (isConnected && address) {
    return (
      <div className="flex items-center space-x-3">
        <div className="rounded-lg bg-purple-900/50 px-3 py-1.5">
          <p className="text-xs text-gray-400">Balance</p>
          <p className="font-mono text-sm text-white">
            {parseFloat(balance).toFixed(4)} ETH
          </p>
        </div>
        <div className="flex items-center space-x-2 rounded-lg bg-purple-800/30 px-3 py-2">
          <div className="h-2 w-2 rounded-full bg-green-500" />
          <span className="font-mono text-sm text-white">
            {address.slice(0, 6)}...{address.slice(-4)}
          </span>
          <button
            onClick={disconnect}
            className="ml-2 rounded p-1 text-gray-400 hover:bg-purple-700/50 hover:text-white"
          >
            <LogOut className="h-4 w-4" />
          </button>
        </div>
      </div>
    );
  }

  return (
    <button
      onClick={handleConnect}
      className="flex items-center space-x-2 rounded-lg bg-gradient-to-r from-purple-600 to-pink-600 px-4 py-2 font-medium text-white transition-all hover:from-purple-500 hover:to-pink-500"
    >
      <Wallet className="h-4 w-4" />
      <span>Connect Wallet</span>
    </button>
  );
}
