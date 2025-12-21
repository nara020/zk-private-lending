/**
 * ZK Private Lending Backend API
 *
 * Provides:
 * - Commitment computation (Poseidon-like hash)
 * - Mock ZK proof generation (for demo)
 * - Position queries
 * - Price oracle
 */

import express from 'express';
import cors from 'cors';
import { ethers } from 'ethers';

const app = express();
const PORT = process.env.PORT || 4000;

// Middleware
app.use(cors());
app.use(express.json());

// In-memory storage for demo
const positions = new Map();
let ethPrice = 2000_00000000; // $2000 with 8 decimals

// Contract addresses (should match frontend .env)
const CONTRACTS = {
  lendingPool: process.env.LENDING_POOL_ADDRESS || '0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9',
  usdc: process.env.USDC_ADDRESS || '0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0',
};

/**
 * Compute commitment using keccak256 (simulating Poseidon for demo)
 * Real implementation should use Poseidon hash from circomlibjs
 */
function computeCommitment(amount, salt) {
  // For demo: use keccak256(amount || salt)
  // In production: use Poseidon hash for ZK-SNARK compatibility
  const packed = ethers.solidityPacked(
    ['uint256', 'uint256'],
    [BigInt(amount), BigInt(salt)]
  );
  return ethers.keccak256(packed);
}

/**
 * Generate mock proof structure (for demo/testing)
 * Real implementation should generate actual Groth16 proofs
 */
function generateMockProof() {
  // Mock Groth16 proof structure
  return {
    a: [
      '0x' + '1'.padStart(64, '0'),
      '0x' + '2'.padStart(64, '0'),
    ],
    b: [
      ['0x' + '3'.padStart(64, '0'), '0x' + '4'.padStart(64, '0')],
      ['0x' + '5'.padStart(64, '0'), '0x' + '6'.padStart(64, '0')],
    ],
    c: [
      '0x' + '7'.padStart(64, '0'),
      '0x' + '8'.padStart(64, '0'),
    ],
  };
}

// ========================
// API Endpoints
// ========================

/**
 * GET /api/price
 * Returns current ETH price
 */
app.get('/api/price', (req, res) => {
  res.json({
    ethPrice: ethPrice / 1e8, // Return as decimal (e.g., 2000.00)
    ethPriceRaw: ethPrice.toString(), // Raw with 8 decimals
    lastUpdated: new Date().toISOString(),
  });
});

/**
 * POST /api/price
 * Update ETH price (admin only in production)
 */
app.post('/api/price', (req, res) => {
  const { price } = req.body;
  if (price && price > 0) {
    ethPrice = Math.floor(price * 1e8); // Convert to 8 decimals
    res.json({ success: true, ethPrice: price });
  } else {
    res.status(400).json({ error: 'Invalid price' });
  }
});

/**
 * POST /api/compute-commitment
 * Compute commitment hash
 */
app.post('/api/compute-commitment', (req, res) => {
  try {
    const { amount, salt } = req.body;

    if (!amount || !salt) {
      return res.status(400).json({ error: 'Missing amount or salt' });
    }

    const commitment = computeCommitment(amount, salt);

    res.json({ commitment });
  } catch (error) {
    console.error('Commitment computation error:', error);
    res.status(500).json({ error: 'Failed to compute commitment' });
  }
});

/**
 * POST /api/prove/collateral
 * Generate collateral proof (mock for demo)
 */
app.post('/api/prove/collateral', (req, res) => {
  try {
    const { amount, salt, commitment } = req.body;

    if (!amount || !salt) {
      return res.status(400).json({ error: 'Missing required parameters' });
    }

    // Verify commitment matches
    const computedCommitment = computeCommitment(amount, salt);
    if (commitment && computedCommitment !== commitment) {
      return res.status(400).json({ error: 'Commitment mismatch' });
    }

    const proof = generateMockProof();
    const publicInputs = [
      commitment || computedCommitment,
      '0x' + BigInt(amount).toString(16).padStart(64, '0'),
    ];

    res.json({
      proof: JSON.stringify(proof),
      publicInputs,
    });
  } catch (error) {
    console.error('Collateral proof error:', error);
    res.status(500).json({ error: 'Failed to generate proof' });
  }
});

/**
 * POST /api/prove/ltv
 * Generate LTV proof (mock for demo)
 */
app.post('/api/prove/ltv', (req, res) => {
  try {
    const { collateralAmount, collateralSalt, borrowAmount, ethPrice: priceParam, maxLTV } = req.body;

    if (!collateralAmount || !borrowAmount) {
      return res.status(400).json({ error: 'Missing required parameters' });
    }

    // Calculate LTV and verify it's within bounds
    const collateralUSD = (BigInt(collateralAmount) * BigInt(priceParam || ethPrice)) / BigInt(1e18);
    const borrowUSD = BigInt(borrowAmount) / BigInt(1e6); // USDC has 6 decimals
    const ltv = (borrowUSD * 100n) / collateralUSD;

    if (ltv > BigInt(maxLTV || 75)) {
      return res.status(400).json({ error: 'LTV exceeds maximum' });
    }

    const commitment = computeCommitment(collateralAmount, collateralSalt);
    const proof = generateMockProof();
    const publicInputs = [
      commitment,
      '0x' + BigInt(maxLTV || 75).toString(16).padStart(64, '0'),
    ];

    res.json({
      proof: JSON.stringify(proof),
      publicInputs,
      ltv: Number(ltv),
    });
  } catch (error) {
    console.error('LTV proof error:', error);
    res.status(500).json({ error: 'Failed to generate proof' });
  }
});

/**
 * POST /api/prove/liquidation
 * Generate liquidation proof (mock for demo)
 */
app.post('/api/prove/liquidation', (req, res) => {
  try {
    const { collateralAmount, collateralSalt, debtAmount, ethPrice: priceParam, liquidationThreshold } = req.body;

    if (!collateralAmount || !debtAmount) {
      return res.status(400).json({ error: 'Missing required parameters' });
    }

    // Calculate health factor
    const collateralUSD = (BigInt(collateralAmount) * BigInt(priceParam || ethPrice)) / BigInt(1e18);
    const threshold = BigInt(liquidationThreshold || 80);
    const collateralWithThreshold = (collateralUSD * threshold) / 100n;
    const debtUSD = BigInt(debtAmount) / BigInt(1e6);

    const isLiquidatable = collateralWithThreshold < debtUSD;

    if (!isLiquidatable) {
      return res.status(400).json({ error: 'Position is not liquidatable' });
    }

    const commitment = computeCommitment(collateralAmount, collateralSalt);
    const proof = generateMockProof();
    const publicInputs = [
      commitment,
      '0x' + BigInt(priceParam || ethPrice).toString(16).padStart(64, '0'),
      '0x' + threshold.toString(16).padStart(64, '0'),
    ];

    res.json({
      proof: JSON.stringify(proof),
      publicInputs,
    });
  } catch (error) {
    console.error('Liquidation proof error:', error);
    res.status(500).json({ error: 'Failed to generate proof' });
  }
});

/**
 * GET /api/position/:address
 * Get position from blockchain
 */
app.get('/api/position/:address', async (req, res) => {
  try {
    const { address } = req.params;

    // For demo: return stored position or empty
    const position = positions.get(address.toLowerCase()) || {
      collateralCommitment: ethers.ZeroHash,
      debtCommitment: ethers.ZeroHash,
      isActive: false,
    };

    res.json(position);
  } catch (error) {
    console.error('Position query error:', error);
    res.status(500).json({ error: 'Failed to get position' });
  }
});

/**
 * GET /api/health/:address
 * Check position health
 */
app.get('/api/health/:address', (req, res) => {
  try {
    const { address } = req.params;

    // For demo: return healthy position
    // In production: calculate from on-chain data
    res.json({
      healthFactor: 1.5,
      isLiquidatable: false,
    });
  } catch (error) {
    console.error('Health check error:', error);
    res.status(500).json({ error: 'Failed to check health' });
  }
});

/**
 * GET /api/contracts
 * Get deployed contract addresses
 */
app.get('/api/contracts', (req, res) => {
  res.json(CONTRACTS);
});

/**
 * Health check
 */
app.get('/health', (req, res) => {
  res.json({ status: 'ok', timestamp: new Date().toISOString() });
});

// Start server
app.listen(PORT, () => {
  console.log(`
========================================
   ZK Private Lending API Server
========================================
Server running on: http://localhost:${PORT}
Available endpoints:
  GET  /api/price           - Get ETH price
  POST /api/compute-commitment - Compute commitment hash
  POST /api/prove/collateral  - Generate collateral proof
  POST /api/prove/ltv         - Generate LTV proof
  POST /api/prove/liquidation - Generate liquidation proof
  GET  /api/position/:address - Get position
  GET  /api/health/:address   - Check health factor
  GET  /api/contracts         - Get contract addresses
  GET  /health               - Health check
========================================
  `);
});
