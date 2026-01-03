/**
 * DepositForm - ETH Collateral Deposit Form
 *
 * Handles ETH deposits with privacy-preserving commitments.
 * The actual deposit amount is hidden on-chain using cryptographic commitments.
 */

import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { parseEther } from 'ethers';
import { ArrowDown, Loader2, Lock } from 'lucide-react';
import toast from 'react-hot-toast';
import { useWallet } from '../hooks/useWallet';
import { api } from '../services/api';
import { contracts } from '../services/contracts';

export function DepositForm() {
  const [amount, setAmount] = useState('');
  const { address, balance, signer } = useWallet();
  const queryClient = useQueryClient();

  const depositMutation = useMutation({
    mutationFn: async () => {
      if (!signer || !address) throw new Error('Wallet not connected');

      const amountWei = parseEther(amount);

      // 1. 랜덤 salt 생성 (보안 중요!)
      const salt = crypto.getRandomValues(new BigUint64Array(1))[0];

      // 2. API에서 commitment 계산
      const { commitment } = await api.computeCommitment(
        amountWei.toString(),
        salt.toString()
      );

      // 3. 컨트랙트에 예치
      const tx = await contracts.deposit(signer, amountWei, commitment);
      await tx.wait();

      // 4. 로컬에 정보 저장 (사용자만 아는 정보)
      const existing = JSON.parse(localStorage.getItem(`position_${address}`) || '{}');
      localStorage.setItem(`position_${address}`, JSON.stringify({
        ...existing,
        collateral: parseFloat(amount),
        collateralSalt: salt.toString(),
        collateralCommitment: commitment,
      }));

      return { tx, commitment };
    },
    onSuccess: () => {
      toast.success('Deposit successful!');
      setAmount('');
      queryClient.invalidateQueries({ queryKey: ['position', address] });
    },
    onError: (error: Error) => {
      toast.error(error.message || 'Deposit failed');
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!amount || parseFloat(amount) <= 0) {
      toast.error('Enter a valid amount');
      return;
    }
    depositMutation.mutate();
  };

  const maxAmount = parseFloat(balance) - 0.01; // 가스비 여유

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Amount Input */}
      <div>
        <label className="mb-2 block text-sm text-gray-400">Deposit Amount</label>
        <div className="relative">
          <input
            type="number"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="0.0"
            step="0.01"
            min="0"
            max={maxAmount}
            className="w-full rounded-xl border border-purple-800/30 bg-purple-900/20 px-4 py-4 text-2xl text-white placeholder-gray-500 focus:border-purple-500 focus:outline-none"
          />
          <div className="absolute right-4 top-1/2 -translate-y-1/2">
            <div className="flex items-center space-x-2">
              <button
                type="button"
                onClick={() => setAmount(maxAmount.toFixed(4))}
                className="rounded bg-purple-600/30 px-2 py-1 text-xs text-purple-300 hover:bg-purple-600/50"
              >
                MAX
              </button>
              <span className="font-medium text-white">ETH</span>
            </div>
          </div>
        </div>
        <p className="mt-2 text-sm text-gray-500">
          Balance: {parseFloat(balance).toFixed(4)} ETH
        </p>
      </div>

      {/* Privacy Notice */}
      <div className="rounded-xl border border-purple-800/30 bg-purple-900/10 p-4">
        <div className="flex items-start space-x-3">
          <Lock className="mt-0.5 h-5 w-5 text-purple-400" />
          <div>
            <p className="font-medium text-purple-300">Privacy Protected</p>
            <p className="mt-1 text-sm text-gray-400">
              Your deposit amount will be hidden on-chain. Only a cryptographic
              commitment (hash) is stored publicly.
            </p>
          </div>
        </div>
      </div>

      {/* Arrow */}
      <div className="flex justify-center">
        <div className="rounded-lg bg-purple-800/30 p-2">
          <ArrowDown className="h-5 w-5 text-purple-400" />
        </div>
      </div>

      {/* Result Preview */}
      <div className="rounded-xl border border-purple-800/30 bg-black/30 p-4">
        <p className="text-sm text-gray-400">You will receive</p>
        <p className="text-xl font-bold text-white">
          Collateral Position
        </p>
        <p className="text-sm text-gray-500">
          Borrow up to 75% LTV against your collateral
        </p>
      </div>

      {/* Submit Button */}
      <button
        type="submit"
        disabled={depositMutation.isPending || !amount || parseFloat(amount) <= 0}
        className="w-full rounded-xl bg-gradient-to-r from-purple-600 to-pink-600 py-4 font-semibold text-white transition-all hover:from-purple-500 hover:to-pink-500 disabled:cursor-not-allowed disabled:opacity-50"
      >
        {depositMutation.isPending ? (
          <span className="flex items-center justify-center">
            <Loader2 className="mr-2 h-5 w-5 animate-spin" />
            Depositing...
          </span>
        ) : (
          'Deposit ETH'
        )}
      </button>
    </form>
  );
}
