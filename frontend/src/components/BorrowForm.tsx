/**
 * BorrowForm - USDC 대출 폼
 *
 * Interview Q&A:
 *
 * Q: ZK 렌딩에서 대출 프로세스는?
 * A: 1. LTV 증명 생성 (로컬)
 *    2. 증명과 함께 borrow 호출
 *    3. 컨트랙트가 증명 검증
 *    4. 검증 통과 시 USDC 전송
 *
 * Q: 왜 LTV 증명이 필요한가?
 * A: 담보 금액이 숨겨져 있으므로
 *    "담보 >= 대출 * (1/LTV)"를 증명해야 함
 *    실제 금액 노출 없이 자격만 증명
 */

import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { parseUnits } from 'ethers';
import { ArrowDown, Loader2, Shield, AlertTriangle, TrendingUp } from 'lucide-react';
import toast from 'react-hot-toast';
import { useWallet } from '../hooks/useWallet';
import { api } from '../services/api';
import { contracts } from '../services/contracts';

export function BorrowForm() {
  const [amount, setAmount] = useState('');
  const { address, signer } = useWallet();
  const queryClient = useQueryClient();

  // ETH 가격 조회
  const { data: priceData } = useQuery({
    queryKey: ['ethPrice'],
    queryFn: () => api.getEthPrice(),
    refetchInterval: 60000,
  });

  // 풀 상태 조회 (이자율 정보)
  const { data: poolStatus } = useQuery({
    queryKey: ['poolStatus'],
    queryFn: () => api.getPoolStatus(),
    refetchInterval: 30000,
  });

  // 로컬 스토리지에서 담보 정보 가져오기
  const localData = JSON.parse(localStorage.getItem(`position_${address}`) || '{}');
  const collateral = localData.collateral || 0;
  const collateralSalt = localData.collateralSalt || '0';
  const currentDebt = localData.debt || 0;

  // 최대 대출 가능 금액 계산 (75% LTV)
  const collateralValueUSD = collateral * (priceData?.ethPrice || 0);
  const maxBorrow = collateralValueUSD * 0.75 - currentDebt;

  const borrowMutation = useMutation({
    mutationFn: async () => {
      if (!signer || !address) throw new Error('Wallet not connected');
      if (collateral <= 0) throw new Error('No collateral deposited');

      const borrowAmount = parseFloat(amount);
      if (borrowAmount > maxBorrow) {
        throw new Error('Exceeds maximum borrow amount');
      }

      // 1. LTV 증명 생성 요청
      toast.loading('Generating ZK proof...', { id: 'proof' });

      const { proof, publicInputs } = await api.generateLTVProof({
        collateralAmount: collateral.toString(),
        collateralSalt: collateralSalt,
        borrowAmount: borrowAmount.toString(),
        ethPrice: priceData?.ethPrice.toString() || '0',
        maxLTV: '75',
      });

      toast.success('Proof generated!', { id: 'proof' });

      // 2. 컨트랙트에 대출 요청
      const amountWei = parseUnits(amount, 6); // USDC는 6 decimals
      const tx = await contracts.borrow(signer, amountWei, proof, publicInputs);
      await tx.wait();

      // 3. 로컬 정보 업데이트
      localStorage.setItem(`position_${address}`, JSON.stringify({
        ...localData,
        debt: currentDebt + borrowAmount,
      }));

      return { tx };
    },
    onSuccess: () => {
      toast.success('Borrow successful!');
      setAmount('');
      queryClient.invalidateQueries({ queryKey: ['position', address] });
    },
    onError: (error: Error) => {
      toast.dismiss('proof');
      toast.error(error.message || 'Borrow failed');
    },
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!amount || parseFloat(amount) <= 0) {
      toast.error('Enter a valid amount');
      return;
    }
    if (parseFloat(amount) > maxBorrow) {
      toast.error('Amount exceeds maximum borrowable');
      return;
    }
    borrowMutation.mutate();
  };

  const isDisabled = borrowMutation.isPending || !amount || parseFloat(amount) <= 0 || maxBorrow <= 0;

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Collateral Info */}
      <div className="rounded-xl border border-purple-800/30 bg-purple-900/10 p-4">
        <div className="flex items-center justify-between">
          <span className="text-sm text-gray-400">Your Collateral</span>
          <span className="font-mono text-white">{collateral.toFixed(4)} ETH</span>
        </div>
        <div className="mt-2 flex items-center justify-between">
          <span className="text-sm text-gray-400">Collateral Value</span>
          <span className="font-mono text-white">${collateralValueUSD.toFixed(2)}</span>
        </div>
        <div className="mt-2 flex items-center justify-between">
          <span className="text-sm text-gray-400">Current Debt</span>
          <span className="font-mono text-white">${currentDebt.toFixed(2)}</span>
        </div>
      </div>

      {/* Amount Input */}
      <div>
        <label className="mb-2 block text-sm text-gray-400">Borrow Amount</label>
        <div className="relative">
          <input
            type="number"
            value={amount}
            onChange={(e) => setAmount(e.target.value)}
            placeholder="0.0"
            step="0.01"
            min="0"
            max={maxBorrow}
            className="w-full rounded-xl border border-purple-800/30 bg-purple-900/20 px-4 py-4 text-2xl text-white placeholder-gray-500 focus:border-purple-500 focus:outline-none"
          />
          <div className="absolute right-4 top-1/2 -translate-y-1/2">
            <div className="flex items-center space-x-2">
              <button
                type="button"
                onClick={() => setAmount(Math.max(0, maxBorrow).toFixed(2))}
                className="rounded bg-purple-600/30 px-2 py-1 text-xs text-purple-300 hover:bg-purple-600/50"
                disabled={maxBorrow <= 0}
              >
                MAX
              </button>
              <span className="font-medium text-white">USDC</span>
            </div>
          </div>
        </div>
        <p className="mt-2 text-sm text-gray-500">
          Max borrowable: ${Math.max(0, maxBorrow).toFixed(2)} USDC
        </p>
      </div>

      {/* Warning if near max */}
      {amount && parseFloat(amount) > maxBorrow * 0.9 && (
        <div className="rounded-xl border border-yellow-800/30 bg-yellow-900/10 p-4">
          <div className="flex items-start space-x-3">
            <AlertTriangle className="mt-0.5 h-5 w-5 text-yellow-400" />
            <div>
              <p className="font-medium text-yellow-300">High LTV Warning</p>
              <p className="mt-1 text-sm text-gray-400">
                Borrowing near maximum increases liquidation risk. Consider
                borrowing less or adding more collateral.
              </p>
            </div>
          </div>
        </div>
      )}

      {/* ZK Proof Info */}
      <div className="rounded-xl border border-purple-800/30 bg-purple-900/10 p-4">
        <div className="flex items-start space-x-3">
          <Shield className="mt-0.5 h-5 w-5 text-purple-400" />
          <div>
            <p className="font-medium text-purple-300">Zero-Knowledge Proof</p>
            <p className="mt-1 text-sm text-gray-400">
              A ZK proof will be generated to verify your collateral ratio
              without revealing your actual position size.
            </p>
          </div>
        </div>
      </div>

      {/* Interest Rate Info */}
      <div className="rounded-xl border border-purple-800/30 bg-gradient-to-r from-purple-900/20 to-pink-900/20 p-4">
        <div className="flex items-center justify-between">
          <div className="flex items-center text-sm text-gray-400">
            <TrendingUp className="mr-2 h-4 w-4 text-green-400" />
            Current Borrow APY
          </div>
          <p className="font-mono text-lg font-bold text-green-400">
            {poolStatus?.apy ? (poolStatus.apy / 100).toFixed(2) : '5.00'}%
          </p>
        </div>
        <div className="mt-3 flex items-center justify-between text-xs">
          <span className="text-gray-500">Pool Utilization</span>
          <span className="text-gray-400">
            {poolStatus?.utilizationRate?.toFixed(1) || '0'}%
          </span>
        </div>
        <div className="mt-2 flex items-center justify-between text-xs">
          <span className="text-gray-500">Available Liquidity</span>
          <span className="text-gray-400">
            ${poolStatus?.availableLiquidity
              ? (parseFloat(poolStatus.availableLiquidity) / 1e6).toLocaleString()
              : '0'} USDC
          </span>
        </div>
        {amount && parseFloat(amount) > 0 && (
          <div className="mt-3 border-t border-purple-800/30 pt-3">
            <div className="flex items-center justify-between text-sm">
              <span className="text-gray-400">Estimated Daily Interest</span>
              <span className="font-mono text-yellow-400">
                ~${((parseFloat(amount) * (poolStatus?.apy ? poolStatus.apy / 100 : 5) / 100) / 365).toFixed(4)}
              </span>
            </div>
            <div className="mt-1 flex items-center justify-between text-sm">
              <span className="text-gray-400">Estimated Monthly Interest</span>
              <span className="font-mono text-yellow-400">
                ~${((parseFloat(amount) * (poolStatus?.apy ? poolStatus.apy / 100 : 5) / 100) / 12).toFixed(2)}
              </span>
            </div>
          </div>
        )}
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
          {amount || '0'} USDC
        </p>
        <p className="text-sm text-gray-500">
          New LTV: {collateralValueUSD > 0
            ? (((currentDebt + parseFloat(amount || '0')) / collateralValueUSD) * 100).toFixed(1)
            : '0'}%
        </p>
      </div>

      {/* Submit Button */}
      <button
        type="submit"
        disabled={isDisabled}
        className="w-full rounded-xl bg-gradient-to-r from-purple-600 to-pink-600 py-4 font-semibold text-white transition-all hover:from-purple-500 hover:to-pink-500 disabled:cursor-not-allowed disabled:opacity-50"
      >
        {borrowMutation.isPending ? (
          <span className="flex items-center justify-center">
            <Loader2 className="mr-2 h-5 w-5 animate-spin" />
            Borrowing...
          </span>
        ) : maxBorrow <= 0 ? (
          'Deposit collateral first'
        ) : (
          'Borrow USDC'
        )}
      </button>
    </form>
  );
}
