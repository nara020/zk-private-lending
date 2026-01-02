/**
 * PositionCard - 사용자 포지션 표시
 *
 * Interview Q&A:
 *
 * Q: ZK 렌딩에서 포지션 표시의 특수성은?
 * A: 담보 금액은 사용자 로컬에서만 알 수 있음
 *    - 온체인에는 commitment만 저장
 *    - 사용자가 salt를 알아야 실제 금액 확인 가능
 *    - UI에서는 로컬 스토리지에 저장된 정보 사용
 */

import { useQuery } from '@tanstack/react-query';
import { Shield, AlertTriangle, RefreshCw, TrendingUp } from 'lucide-react';
import { api } from '../services/api';

interface PositionCardProps {
  address: string;
}

export function PositionCard({ address }: PositionCardProps) {
  const { data: position, isLoading, refetch } = useQuery({
    queryKey: ['position', address],
    queryFn: () => api.getPosition(address),
    refetchInterval: 30000, // 30초마다 갱신
  });

  const { data: price } = useQuery({
    queryKey: ['ethPrice'],
    queryFn: () => api.getEthPrice(),
    refetchInterval: 60000, // 1분마다 갱신
  });

  const { data: poolStatus } = useQuery({
    queryKey: ['poolStatus'],
    queryFn: () => api.getPoolStatus(),
    refetchInterval: 30000, // 30초마다 갱신
  });

  // 로컬 스토리지에서 실제 담보 금액 가져오기 (사용자만 아는 정보)
  const localData = JSON.parse(localStorage.getItem(`position_${address}`) || '{}');
  const actualCollateral = localData.collateral || 0;
  const actualDebt = localData.debt || 0;
  const borrowTimestamp = localData.borrowTimestamp || 0;

  // 간단한 이자 계산 (5% APR 기준)
  const daysSinceBorrow = borrowTimestamp ? (Date.now() / 1000 - borrowTimestamp) / 86400 : 0;
  const estimatedInterest = actualDebt * 0.05 * (daysSinceBorrow / 365);
  const totalDebt = actualDebt + estimatedInterest;

  // Health Factor 계산
  const collateralValueUSD = actualCollateral * (price?.ethPrice || 0);
  const healthFactor = actualDebt > 0
    ? (collateralValueUSD * 0.8) / actualDebt
    : Infinity;

  const isWarning = healthFactor >= 1.0 && healthFactor < 1.5;
  const isDanger = healthFactor < 1.0;

  if (isLoading) {
    return (
      <div className="rounded-2xl border border-purple-800/30 bg-black/40 p-6 backdrop-blur-sm">
        <div className="animate-pulse space-y-4">
          <div className="h-4 w-1/3 rounded bg-purple-800/50" />
          <div className="h-8 w-2/3 rounded bg-purple-800/50" />
          <div className="h-4 w-1/2 rounded bg-purple-800/50" />
        </div>
      </div>
    );
  }

  return (
    <div className="rounded-2xl border border-purple-800/30 bg-black/40 p-6 backdrop-blur-sm">
      <div className="mb-4 flex items-center justify-between">
        <h2 className="flex items-center text-lg font-semibold text-white">
          <Shield className="mr-2 h-5 w-5 text-purple-400" />
          Your Position
        </h2>
        <button
          onClick={() => refetch()}
          className="rounded p-1 text-gray-400 hover:bg-purple-700/50 hover:text-white"
        >
          <RefreshCw className="h-4 w-4" />
        </button>
      </div>

      {/* Health Factor */}
      <div className={`mb-6 rounded-xl p-4 ${
        isDanger ? 'bg-red-900/30' :
        isWarning ? 'bg-yellow-900/30' :
        'bg-green-900/30'
      }`}>
        <p className="text-xs text-gray-400">Health Factor</p>
        <p className={`text-2xl font-bold ${
          isDanger ? 'text-red-400' :
          isWarning ? 'text-yellow-400' :
          'text-green-400'
        }`}>
          {healthFactor === Infinity ? '∞' : healthFactor.toFixed(2)}
        </p>
        {isDanger && (
          <p className="mt-1 flex items-center text-xs text-red-300">
            <AlertTriangle className="mr-1 h-3 w-3" />
            Liquidation risk!
          </p>
        )}
      </div>

      {/* Position Details */}
      <div className="space-y-4">
        <div className="flex justify-between">
          <span className="text-gray-400">Collateral</span>
          <div className="text-right">
            <p className="font-mono text-white">
              {actualCollateral.toFixed(4)} ETH
            </p>
            <p className="text-xs text-gray-500">
              ≈ ${collateralValueUSD.toFixed(2)}
            </p>
          </div>
        </div>

        <div className="flex justify-between">
          <span className="text-gray-400">Principal</span>
          <div className="text-right">
            <p className="font-mono text-white">
              {actualDebt.toFixed(2)} USDC
            </p>
          </div>
        </div>

        {estimatedInterest > 0 && (
          <div className="flex justify-between">
            <span className="text-gray-400">Accrued Interest</span>
            <div className="text-right">
              <p className="font-mono text-yellow-400">
                +{estimatedInterest.toFixed(2)} USDC
              </p>
            </div>
          </div>
        )}

        <div className="flex justify-between border-t border-purple-800/30 pt-2">
          <span className="text-gray-400">Total Debt</span>
          <div className="text-right">
            <p className="font-mono font-bold text-white">
              {totalDebt.toFixed(2)} USDC
            </p>
          </div>
        </div>

        <div className="flex justify-between">
          <span className="text-gray-400">LTV</span>
          <p className="font-mono text-white">
            {collateralValueUSD > 0
              ? ((actualDebt / collateralValueUSD) * 100).toFixed(1)
              : '0'}%
          </p>
        </div>

        <div className="flex justify-between">
          <span className="text-gray-400">Max LTV</span>
          <p className="font-mono text-purple-400">75%</p>
        </div>

        <div className="flex justify-between">
          <span className="text-gray-400">Liq. Threshold</span>
          <p className="font-mono text-purple-400">80%</p>
        </div>
      </div>

      {/* Interest Rate Info */}
      <div className="mt-4 rounded-lg border border-purple-800/30 bg-gradient-to-r from-purple-900/20 to-pink-900/20 p-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center text-sm text-gray-400">
            <TrendingUp className="mr-1 h-4 w-4 text-green-400" />
            Borrow APY
          </div>
          <p className="font-mono text-lg font-bold text-green-400">
            {poolStatus?.apy ? (poolStatus.apy / 100).toFixed(2) : '5.00'}%
          </p>
        </div>
        <div className="mt-2 flex items-center justify-between text-xs">
          <span className="text-gray-500">Utilization</span>
          <span className="text-gray-400">
            {poolStatus?.utilizationRate?.toFixed(1) || '0'}%
          </span>
        </div>
      </div>

      {/* Privacy Badge */}
      <div className="mt-6 rounded-lg border border-purple-800/30 bg-purple-900/20 p-3">
        <p className="flex items-center text-xs text-purple-300">
          <Shield className="mr-1 h-3 w-3" />
          Position size hidden on-chain
        </p>
        <p className="mt-1 text-xs text-gray-500">
          Commitment: {position?.collateralCommitment?.slice(0, 10)}...
        </p>
      </div>
    </div>
  );
}
