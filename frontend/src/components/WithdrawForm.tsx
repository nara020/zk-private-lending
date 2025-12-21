/**
 * WithdrawForm - 담보 출금 폼
 *
 * 출금 조건:
 * 1. 부채가 0이어야 함
 * 2. 활성 포지션이 있어야 함
 */

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { Loader2, Unlock, AlertTriangle } from 'lucide-react';
import toast from 'react-hot-toast';
import { useWallet } from '../hooks/useWallet';
import { contracts } from '../services/contracts';

export function WithdrawForm() {
  const { address, signer } = useWallet();
  const queryClient = useQueryClient();

  // 로컬 스토리지에서 포지션 정보 가져오기
  const localData = JSON.parse(localStorage.getItem(`position_${address}`) || '{}');
  const collateral = localData.collateral || 0;
  const debt = localData.debt || 0;
  const hasPosition = collateral > 0;
  const canWithdraw = hasPosition && debt <= 0;

  const withdrawMutation = useMutation({
    mutationFn: async () => {
      if (!signer) throw new Error('Wallet not connected');
      if (!canWithdraw) throw new Error('Cannot withdraw with active debt');

      const tx = await contracts.withdraw(signer);
      await tx.wait();

      // 로컬 정보 초기화
      localStorage.removeItem(`position_${address}`);

      return { tx };
    },
    onSuccess: () => {
      toast.success('Withdrawal successful! Collateral returned.');
      queryClient.invalidateQueries({ queryKey: ['position', address] });
    },
    onError: (error: Error) => {
      toast.error(error.message || 'Withdrawal failed');
    },
  });

  // 포지션이 없는 경우
  if (!hasPosition) {
    return (
      <div className="flex flex-col items-center justify-center py-12">
        <div className="mb-4 rounded-full bg-gray-800/50 p-4">
          <Unlock className="h-8 w-8 text-gray-500" />
        </div>
        <h3 className="mb-2 text-lg font-medium text-white">No Position</h3>
        <p className="text-center text-sm text-gray-400">
          You don't have any collateral deposited.
          <br />
          Deposit ETH first to create a position.
        </p>
      </div>
    );
  }

  // 부채가 있는 경우
  if (debt > 0) {
    return (
      <div className="space-y-6">
        <div className="rounded-xl border border-orange-800/30 bg-orange-900/10 p-6">
          <div className="flex items-start space-x-4">
            <div className="rounded-full bg-orange-500/20 p-2">
              <AlertTriangle className="h-6 w-6 text-orange-400" />
            </div>
            <div>
              <h3 className="text-lg font-medium text-orange-300">Active Debt</h3>
              <p className="mt-2 text-sm text-gray-400">
                You must repay all debt before withdrawing collateral.
              </p>
            </div>
          </div>
        </div>

        <div className="rounded-xl border border-purple-800/30 bg-purple-900/10 p-4">
          <div className="flex items-center justify-between">
            <span className="text-sm text-gray-400">Collateral</span>
            <span className="font-mono text-white">{collateral.toFixed(4)} ETH</span>
          </div>
          <div className="mt-2 flex items-center justify-between">
            <span className="text-sm text-gray-400">Outstanding Debt</span>
            <span className="font-mono text-red-400">${debt.toFixed(2)} USDC</span>
          </div>
        </div>

        <button
          disabled
          className="w-full cursor-not-allowed rounded-xl bg-gray-700 py-4 font-semibold text-gray-400"
        >
          Repay Debt First
        </button>
      </div>
    );
  }

  // 출금 가능한 경우
  return (
    <div className="space-y-6">
      <div className="rounded-xl border border-green-800/30 bg-green-900/10 p-6">
        <div className="flex items-start space-x-4">
          <div className="rounded-full bg-green-500/20 p-2">
            <Unlock className="h-6 w-6 text-green-400" />
          </div>
          <div>
            <h3 className="text-lg font-medium text-green-300">Ready to Withdraw</h3>
            <p className="mt-2 text-sm text-gray-400">
              You have no outstanding debt. Your collateral is unlocked and ready for withdrawal.
            </p>
          </div>
        </div>
      </div>

      <div className="rounded-xl border border-purple-800/30 bg-black/30 p-6">
        <p className="text-sm text-gray-400">Available to withdraw</p>
        <p className="mt-2 text-3xl font-bold text-white">{collateral.toFixed(4)} ETH</p>
        <p className="mt-1 text-sm text-gray-500">
          ≈ ${(collateral * 2000).toFixed(2)} USD
        </p>
      </div>

      <button
        onClick={() => withdrawMutation.mutate()}
        disabled={withdrawMutation.isPending}
        className="w-full rounded-xl bg-gradient-to-r from-green-600 to-emerald-600 py-4 font-semibold text-white transition-all hover:from-green-500 hover:to-emerald-500 disabled:cursor-not-allowed disabled:opacity-50"
      >
        {withdrawMutation.isPending ? (
          <span className="flex items-center justify-center">
            <Loader2 className="mr-2 h-5 w-5 animate-spin" />
            Withdrawing...
          </span>
        ) : (
          `Withdraw ${collateral.toFixed(4)} ETH`
        )}
      </button>

      <p className="text-center text-xs text-gray-500">
        Your position will be closed after withdrawal
      </p>
    </div>
  );
}
