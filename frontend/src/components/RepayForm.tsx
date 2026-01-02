/**
 * RepayForm - 대출 상환 폼
 *
 * Interview Q&A:
 *
 * Q: 상환 시 ZK 증명이 필요한가?
 * A: 상환은 증명 불필요
 *    - 상환은 사용자에게 유리한 행동
 *    - 부채 감소는 검증 없이 허용
 *    - 단, 상환 후 commitment 업데이트 필요
 *
 * Q: 부분 상환 vs 전액 상환?
 * A: 둘 다 지원
 *    - 부분: 원하는 만큼 상환
 *    - 전액: 모든 부채 + 이자 상환
 *    - 전액 상환 시 담보 인출 가능
 */

import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { parseUnits } from 'ethers';
import { ArrowDown, Loader2, CheckCircle, TrendingUp } from 'lucide-react';
import toast from 'react-hot-toast';
import { useWallet } from '../hooks/useWallet';
import { contracts } from '../services/contracts';

export function RepayForm() {
  const [amount, setAmount] = useState('');
  const { address, signer } = useWallet();
  const queryClient = useQueryClient();

  // 로컬 스토리지에서 부채 정보 가져오기
  const localData = JSON.parse(localStorage.getItem(`position_${address}`) || '{}');
  const principal = localData.debt || 0;
  const collateral = localData.collateral || 0;
  const borrowTimestamp = localData.borrowTimestamp || 0;

  // 간단한 이자 계산 (5% APR 기준)
  const daysSinceBorrow = borrowTimestamp ? (Date.now() / 1000 - borrowTimestamp) / 86400 : 0;
  const accruedInterest = principal * 0.05 * (daysSinceBorrow / 365);
  const currentDebt = principal + accruedInterest;

  const repayMutation = useMutation({
    mutationFn: async () => {
      if (!signer || !address) throw new Error('Wallet not connected');
      if (currentDebt <= 0) throw new Error('No debt to repay');

      const repayAmount = parseFloat(amount);
      if (repayAmount > currentDebt) {
        throw new Error('Amount exceeds current debt');
      }

      // USDC approve 및 repay 호출
      const amountWei = parseUnits(amount, 6); // USDC는 6 decimals

      // 1. USDC approve
      toast.loading('Approving USDC...', { id: 'approve' });
      await contracts.approveUSDC(signer, amountWei);
      toast.success('USDC approved!', { id: 'approve' });

      // 2. Repay
      const tx = await contracts.repay(signer, amountWei);
      await tx.wait();

      // 3. 로컬 정보 업데이트
      const newDebt = currentDebt - repayAmount;
      localStorage.setItem(`position_${address}`, JSON.stringify({
        ...localData,
        debt: newDebt,
      }));

      return { tx, newDebt };
    },
    onSuccess: (data) => {
      toast.success('Repayment successful!');
      setAmount('');
      queryClient.invalidateQueries({ queryKey: ['position', address] });

      if (data.newDebt <= 0) {
        toast.success('Congratulations! All debt repaid. You can now withdraw your collateral.', {
          duration: 5000,
        });
      }
    },
    onError: (error: Error) => {
      toast.dismiss('approve');
      toast.error(error.message || 'Repayment failed');
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!amount || parseFloat(amount) <= 0) {
      toast.error('Enter a valid amount');
      return;
    }
    if (parseFloat(amount) > currentDebt) {
      toast.error('Amount exceeds current debt');
      return;
    }
    repayMutation.mutate();
  };

  const isDisabled = repayMutation.isPending || !amount || parseFloat(amount) <= 0 || currentDebt <= 0;
  const remainingDebt = currentDebt - parseFloat(amount || '0');

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Current Position */}
      <div className="rounded-xl border border-purple-800/30 bg-purple-900/10 p-4">
        <div className="flex items-center justify-between">
          <span className="text-sm text-gray-400">Principal</span>
          <span className="font-mono text-white">${principal.toFixed(2)} USDC</span>
        </div>
        {accruedInterest > 0 && (
          <div className="mt-2 flex items-center justify-between">
            <span className="flex items-center text-sm text-gray-400">
              <TrendingUp className="mr-1 h-3 w-3 text-yellow-400" />
              Accrued Interest
            </span>
            <span className="font-mono text-yellow-400">+${accruedInterest.toFixed(2)}</span>
          </div>
        )}
        <div className="mt-2 flex items-center justify-between border-t border-purple-800/30 pt-2">
          <span className="text-sm font-medium text-gray-300">Total Debt</span>
          <span className="font-mono font-bold text-white">${currentDebt.toFixed(2)} USDC</span>
        </div>
        <div className="mt-2 flex items-center justify-between">
          <span className="text-sm text-gray-400">Collateral</span>
          <span className="font-mono text-white">{collateral.toFixed(4)} ETH</span>
        </div>
      </div>

      {/* Amount Input */}
      <div>
        <label className="mb-2 block text-sm text-gray-400">Repay Amount</label>
        <div className="relative">
          <input
            type="number"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="0.0"
            step="0.01"
            min="0"
            max={currentDebt}
            className="w-full rounded-xl border border-purple-800/30 bg-purple-900/20 px-4 py-4 text-2xl text-white placeholder-gray-500 focus:border-purple-500 focus:outline-none"
          />
          <div className="absolute right-4 top-1/2 -translate-y-1/2">
            <div className="flex items-center space-x-2">
              <button
                type="button"
                onClick={() => setAmount(currentDebt.toFixed(2))}
                className="rounded bg-purple-600/30 px-2 py-1 text-xs text-purple-300 hover:bg-purple-600/50"
                disabled={currentDebt <= 0}
              >
                MAX
              </button>
              <span className="font-medium text-white">USDC</span>
            </div>
          </div>
        </div>
        <p className="mt-2 text-sm text-gray-500">
          Outstanding: ${currentDebt.toFixed(2)} USDC
        </p>
      </div>

      {/* Full Repayment Notice */}
      {amount && parseFloat(amount) >= currentDebt && (
        <div className="rounded-xl border border-green-800/30 bg-green-900/10 p-4">
          <div className="flex items-start space-x-3">
            <CheckCircle className="mt-0.5 h-5 w-5 text-green-400" />
            <div>
              <p className="font-medium text-green-300">Full Repayment</p>
              <p className="mt-1 text-sm text-gray-400">
                After full repayment, you can withdraw all your collateral
                ({collateral.toFixed(4)} ETH).
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Arrow */}
      <div className="flex justify-center">
        <div className="rounded-lg bg-purple-800/30 p-2">
          <ArrowDown className="h-5 w-5 text-purple-400" />
        </div>
      </div>

      {/* Result Preview */}
      <div className="rounded-xl border border-purple-800/30 bg-black/30 p-4">
        <p className="text-sm text-gray-400">After repayment</p>
        <p className="text-xl font-bold text-white">
          ${Math.max(0, remainingDebt).toFixed(2)} USDC remaining
        </p>
        {remainingDebt <= 0 && (
          <p className="mt-1 text-sm text-green-400">
            Collateral unlocked for withdrawal
          </p>
        )}
      </div>

      {/* Submit Button */}
      <button
        type="submit"
        disabled={isDisabled}
        className="w-full rounded-xl bg-gradient-to-r from-purple-600 to-pink-600 py-4 font-semibold text-white transition-all hover:from-purple-500 hover:to-pink-500 disabled:cursor-not-allowed disabled:opacity-50"
      >
        {repayMutation.isPending ? (
          <span className="flex items-center justify-center">
            <Loader2 className="mr-2 h-5 w-5 animate-spin" />
            Repaying...
          </span>
        ) : currentDebt <= 0 ? (
          'No debt to repay'
        ) : (
          'Repay USDC'
        )}
      </button>

      {/* Withdraw Section */}
      {currentDebt <= 0 && collateral > 0 && (
        <WithdrawSection address={address!} collateral={collateral} signer={signer} />
      )}
    </form>
  );
}

// 담보 인출 섹션 (부채가 없을 때만 표시)
function WithdrawSection({ address, collateral, signer }: {
  address: string;
  collateral: number;
  signer: any;
}) {
  const queryClient = useQueryClient();

  const withdrawMutation = useMutation({
    mutationFn: async () => {
      if (!signer) throw new Error('Wallet not connected');

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

  return (
    <div className="mt-6 border-t border-purple-800/30 pt-6">
      <h3 className="mb-4 text-lg font-semibold text-white">Withdraw Collateral</h3>
      <p className="mb-4 text-sm text-gray-400">
        You have no outstanding debt. You can withdraw your full collateral.
      </p>
      <button
        onClick={() => withdrawMutation.mutate()}
        disabled={withdrawMutation.isPending}
        className="w-full rounded-xl border border-purple-500 bg-transparent py-3 font-semibold text-purple-400 transition-all hover:bg-purple-500/10 disabled:cursor-not-allowed disabled:opacity-50"
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
    </div>
  );
}
