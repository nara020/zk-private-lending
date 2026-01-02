/**
 * LiquidateForm - 청산 실행 폼
 *
 * ZK 청산의 핵심:
 * - 청산자는 대상 포지션의 실제 담보 금액을 모름
 * - ZK proof로 "담보 가치 < 부채 * 청산임계값" 증명
 * - 청산 보상: 담보의 일부 (보통 5-10%)
 */

import { useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { isAddress } from 'ethers';
import { Loader2, AlertOctagon, Search, Zap } from 'lucide-react';
import toast from 'react-hot-toast';
import { useWallet } from '../hooks/useWallet';
import { api } from '../services/api';
import { contracts } from '../services/contracts';

export function LiquidateForm() {
  const [targetAddress, setTargetAddress] = useState('');
  const [targetPosition, setTargetPosition] = useState<any>(null);
  const [isSearching, setIsSearching] = useState(false);
  const { address, signer } = useWallet();
  const queryClient = useQueryClient();

  // 대상 포지션 조회
  const handleSearch = async () => {
    if (!targetAddress || !isAddress(targetAddress)) {
      toast.error('Enter a valid Ethereum address');
      return;
    }
    if (targetAddress.toLowerCase() === address?.toLowerCase()) {
      toast.error('Cannot liquidate your own position');
      return;
    }

    setIsSearching(true);
    try {
      const position = await api.getPosition(targetAddress);
      const health = await api.getHealth(targetAddress);

      // 로컬 데모용: localStorage에서 가져오기
      const localData = JSON.parse(localStorage.getItem(`position_${targetAddress}`) || '{}');

      setTargetPosition({
        ...position,
        ...health,
        collateral: localData.collateral || 0,
        debt: localData.debt || 0,
      });
    } catch (error) {
      toast.error('Failed to fetch position');
      setTargetPosition(null);
    } finally {
      setIsSearching(false);
    }
  };

  const liquidateMutation = useMutation({
    mutationFn: async () => {
      if (!signer || !address) throw new Error('Wallet not connected');
      if (!targetPosition) throw new Error('No target position');

      // ZK Proof 생성 (실제로는 Halo2 proof)
      toast.loading('Generating ZK proof...', { id: 'proof' });

      const localData = JSON.parse(localStorage.getItem(`position_${targetAddress}`) || '{}');

      const proofData = await api.proveLiquidation({
        collateralAmount: localData.collateral?.toString() || '0',
        collateralSalt: localData.collateralSalt || '0',
        debtAmount: localData.debt?.toString() || '0',
        ethPrice: 2000 * 1e8, // Mock price
        liquidationThreshold: 80,
      });

      toast.success('Proof generated!', { id: 'proof' });

      // 청산 실행
      const tx = await contracts.liquidate(
        signer,
        targetAddress,
        proofData.proof,
        proofData.publicInputs
      );
      await tx.wait();

      return { tx };
    },
    onSuccess: () => {
      toast.success('Liquidation successful! You received the bonus.');
      setTargetAddress('');
      setTargetPosition(null);
      queryClient.invalidateQueries({ queryKey: ['position'] });
    },
    onError: (error: Error) => {
      toast.dismiss('proof');
      toast.error(error.message || 'Liquidation failed');
    },
  });

  const isLiquidatable = targetPosition?.isLiquidatable || targetPosition?.healthFactor < 1;

  return (
    <div className="space-y-6">
      {/* Info Banner */}
      <div className="rounded-xl border border-purple-800/30 bg-purple-900/10 p-4">
        <div className="flex items-start space-x-3">
          <AlertOctagon className="mt-0.5 h-5 w-5 text-purple-400" />
          <div>
            <h3 className="font-medium text-purple-300">ZK Liquidation</h3>
            <p className="mt-1 text-sm text-gray-400">
              Liquidate undercollateralized positions without knowing their exact collateral.
              ZK proofs verify eligibility while preserving privacy.
            </p>
          </div>
        </div>
      </div>

      {/* Search Section */}
      <div>
        <label className="mb-2 block text-sm text-gray-400">Target Address</label>
        <div className="flex space-x-2">
          <input
            type="text"
            value={targetAddress}
            onChange={(e) => setTargetAddress(e.target.value)}
            placeholder="0x..."
            className="flex-1 rounded-xl border border-purple-800/30 bg-purple-900/20 px-4 py-3 text-white placeholder-gray-500 focus:border-purple-500 focus:outline-none"
          />
          <button
            onClick={handleSearch}
            disabled={isSearching || !targetAddress}
            className="rounded-xl bg-purple-600 px-4 py-3 text-white transition-all hover:bg-purple-500 disabled:cursor-not-allowed disabled:opacity-50"
          >
            {isSearching ? (
              <Loader2 className="h-5 w-5 animate-spin" />
            ) : (
              <Search className="h-5 w-5" />
            )}
          </button>
        </div>
      </div>

      {/* Position Details */}
      {targetPosition && (
        <div className="space-y-4">
          <div className="rounded-xl border border-purple-800/30 bg-black/30 p-4">
            <h4 className="mb-3 text-sm font-medium text-gray-400">Position Details</h4>

            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-sm text-gray-400">Collateral</span>
                <span className="font-mono text-white">
                  {targetPosition.collateral?.toFixed(4) || '???'} ETH
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-gray-400">Debt</span>
                <span className="font-mono text-white">
                  ${targetPosition.debt?.toFixed(2) || '???'} USDC
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-gray-400">Health Factor</span>
                <span className={`font-mono ${
                  targetPosition.healthFactor >= 1 ? 'text-green-400' : 'text-red-400'
                }`}>
                  {targetPosition.healthFactor?.toFixed(2) || '???'}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-gray-400">Status</span>
                <span className={`rounded-full px-2 py-0.5 text-xs ${
                  isLiquidatable
                    ? 'bg-red-500/20 text-red-400'
                    : 'bg-green-500/20 text-green-400'
                }`}>
                  {isLiquidatable ? 'Liquidatable' : 'Healthy'}
                </span>
              </div>
            </div>
          </div>

          {/* Liquidation Reward Preview */}
          {isLiquidatable && (
            <div className="rounded-xl border border-orange-800/30 bg-orange-900/10 p-4">
              <h4 className="mb-2 text-sm font-medium text-orange-300">Liquidation Reward</h4>
              <p className="text-2xl font-bold text-white">
                ~{(targetPosition.collateral * 0.05).toFixed(4)} ETH
              </p>
              <p className="mt-1 text-xs text-gray-400">
                5% liquidation bonus
              </p>
            </div>
          )}

          {/* Liquidate Button */}
          <button
            onClick={() => liquidateMutation.mutate()}
            disabled={!isLiquidatable || liquidateMutation.isPending}
            className={`w-full rounded-xl py-4 font-semibold transition-all ${
              isLiquidatable
                ? 'bg-gradient-to-r from-red-600 to-orange-600 text-white hover:from-red-500 hover:to-orange-500'
                : 'cursor-not-allowed bg-gray-700 text-gray-400'
            } disabled:opacity-50`}
          >
            {liquidateMutation.isPending ? (
              <span className="flex items-center justify-center">
                <Loader2 className="mr-2 h-5 w-5 animate-spin" />
                Liquidating...
              </span>
            ) : isLiquidatable ? (
              <span className="flex items-center justify-center">
                <Zap className="mr-2 h-5 w-5" />
                Execute Liquidation
              </span>
            ) : (
              'Position is Healthy'
            )}
          </button>
        </div>
      )}

      {/* Empty State */}
      {!targetPosition && !isSearching && (
        <div className="flex flex-col items-center justify-center py-8 text-center">
          <div className="mb-4 rounded-full bg-purple-800/30 p-4">
            <Search className="h-8 w-8 text-purple-400" />
          </div>
          <p className="text-gray-400">
            Enter an address to check if their position is liquidatable
          </p>
        </div>
      )}
    </div>
  );
}
