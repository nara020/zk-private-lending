/**
 * ZK Private Lending - Main Application
 *
 * Privacy-first DeFi lending protocol frontend built with React, TypeScript,
 * and Zustand for state management. Integrates with ZK proof generation
 * for private collateral verification.
 */

import { useState } from 'react';
import { WalletConnect } from './components/WalletConnect';
import { PositionCard } from './components/PositionCard';
import { DepositForm } from './components/DepositForm';
import { BorrowForm } from './components/BorrowForm';
import { RepayForm } from './components/RepayForm';
import { WithdrawForm } from './components/WithdrawForm';
import { LiquidateForm } from './components/LiquidateForm';
import { useWallet } from './hooks/useWallet';

type Tab = 'deposit' | 'borrow' | 'repay' | 'withdraw' | 'liquidate';

function App() {
  const { address, isConnected } = useWallet();
  const [activeTab, setActiveTab] = useState<Tab>('deposit');

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-purple-900 to-slate-900">
      {/* Header */}
      <header className="border-b border-purple-800/30 bg-black/20 backdrop-blur-sm">
        <div className="container mx-auto px-4 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              <div className="h-8 w-8 rounded-lg bg-gradient-to-br from-purple-500 to-pink-500" />
              <h1 className="text-xl font-bold text-white">ZK Private Lending</h1>
              <span className="rounded-full bg-purple-500/20 px-2 py-0.5 text-xs text-purple-300">
                Beta
              </span>
            </div>
            <WalletConnect />
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="container mx-auto px-4 py-8">
        {!isConnected ? (
          <div className="flex flex-col items-center justify-center py-20">
            <div className="mb-8 h-24 w-24 rounded-full bg-gradient-to-br from-purple-500/20 to-pink-500/20 p-6">
              <div className="h-full w-full rounded-full bg-gradient-to-br from-purple-500 to-pink-500" />
            </div>
            <h2 className="mb-4 text-3xl font-bold text-white">
              Privacy-First DeFi Lending
            </h2>
            <p className="mb-8 max-w-md text-center text-gray-400">
              Deposit collateral and borrow assets without revealing your position size.
              Powered by zero-knowledge proofs.
            </p>
            <WalletConnect />
          </div>
        ) : (
          <div className="grid gap-8 lg:grid-cols-3">
            {/* Position Overview */}
            <div className="lg:col-span-1">
              <PositionCard address={address!} />
            </div>

            {/* Action Panel */}
            <div className="lg:col-span-2">
              <div className="rounded-2xl border border-purple-800/30 bg-black/40 backdrop-blur-sm">
                {/* Tabs */}
                <div className="flex border-b border-purple-800/30">
                  {(['deposit', 'borrow', 'repay', 'withdraw', 'liquidate'] as Tab[]).map((tab) => (
                    <button
                      key={tab}
                      onClick={() => setActiveTab(tab)}
                      className={`flex-1 px-4 py-3 text-sm font-medium transition-colors ${
                        activeTab === tab
                          ? 'border-b-2 border-purple-500 text-purple-400'
                          : 'text-gray-400 hover:text-gray-300'
                      }`}
                    >
                      {tab.charAt(0).toUpperCase() + tab.slice(1)}
                    </button>
                  ))}
                </div>

                {/* Tab Content */}
                <div className="p-6">
                  {activeTab === 'deposit' && <DepositForm />}
                  {activeTab === 'borrow' && <BorrowForm />}
                  {activeTab === 'repay' && <RepayForm />}
                  {activeTab === 'withdraw' && <WithdrawForm />}
                  {activeTab === 'liquidate' && <LiquidateForm />}
                </div>
              </div>

              {/* Info Card */}
              <div className="mt-6 rounded-xl border border-purple-800/30 bg-purple-900/20 p-4">
                <h3 className="mb-2 flex items-center text-sm font-medium text-purple-300">
                  <span className="mr-2">🔐</span>
                  Privacy Guarantee
                </h3>
                <p className="text-sm text-gray-400">
                  Your collateral amount is never revealed on-chain. Only cryptographic
                  commitments are stored, and ZK proofs verify your eligibility without
                  exposing actual values.
                </p>
              </div>
            </div>
          </div>
        )}
      </main>

      {/* Footer */}
      <footer className="border-t border-purple-800/30 bg-black/20 py-6">
        <div className="container mx-auto px-4 text-center text-sm text-gray-500">
          <p>ZK Private Lending Protocol - Powered by Halo2 ZK-SNARKs</p>
          <p className="mt-1">
            <a href="#" className="text-purple-400 hover:text-purple-300">
              Documentation
            </a>
            {' · '}
            <a href="#" className="text-purple-400 hover:text-purple-300">
              GitHub
            </a>
            {' · '}
            <a href="#" className="text-purple-400 hover:text-purple-300">
              Audit Report
            </a>
          </p>
        </div>
      </footer>
    </div>
  );
}

export default App;
