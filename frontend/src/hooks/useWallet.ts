/**
 * useWallet Hook - Wallet Connection Management
 *
 * Zustand-based state management for Web3 wallet connections.
 * Handles MetaMask integration, account changes, and network switching.
 */

import { create } from 'zustand';
import { BrowserProvider, JsonRpcSigner, formatEther } from 'ethers';

interface WalletState {
  address: string | null;
  isConnected: boolean;
  isConnecting: boolean;
  chainId: number | null;
  balance: string;
  provider: BrowserProvider | null;
  signer: JsonRpcSigner | null;

  connect: () => Promise<void>;
  disconnect: () => void;
  refreshBalance: () => Promise<void>;
}

export const useWallet = create<WalletState>((set, get) => ({
  address: null,
  isConnected: false,
  isConnecting: false,
  chainId: null,
  balance: '0',
  provider: null,
  signer: null,

  connect: async () => {
    if (typeof window.ethereum === 'undefined') {
      throw new Error('MetaMask not installed');
    }

    set({ isConnecting: true });

    try {
      const provider = new BrowserProvider(window.ethereum);
      const accounts = await provider.send('eth_requestAccounts', []);
      const address = accounts[0];
      const signer = await provider.getSigner();
      const network = await provider.getNetwork();
      const balance = await provider.getBalance(address);

      set({
        address,
        isConnected: true,
        isConnecting: false,
        chainId: Number(network.chainId),
        balance: formatEther(balance),
        provider,
        signer,
      });

      // Listen for account changes
      window.ethereum.on('accountsChanged', (accounts: string[]) => {
        if (accounts.length === 0) {
          get().disconnect();
        } else {
          set({ address: accounts[0] });
          get().refreshBalance();
        }
      });

      // Listen for chain changes
      window.ethereum.on('chainChanged', () => {
        window.location.reload();
      });
    } catch (error) {
      set({ isConnecting: false });
      throw error;
    }
  },

  disconnect: () => {
    set({
      address: null,
      isConnected: false,
      isConnecting: false,
      chainId: null,
      balance: '0',
      provider: null,
      signer: null,
    });
  },

  refreshBalance: async () => {
    const { provider, address } = get();
    if (!provider || !address) return;

    const balance = await provider.getBalance(address);
    set({ balance: formatEther(balance) });
  },
}));

// Type augmentation for window.ethereum
declare global {
  interface Window {
    ethereum?: {
      request: (args: { method: string; params?: unknown[] }) => Promise<unknown>;
      on: (event: string, callback: (accounts: string[]) => void) => void;
      removeListener: (event: string, callback: (accounts: string[]) => void) => void;
    };
  }
}
