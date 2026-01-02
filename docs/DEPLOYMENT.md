# ZK Private Lending - Deployment Guide

## Overview

This guide covers deploying the ZK Private Lending protocol to production.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Production Architecture                              │
│                                                                             │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐                  │
│  │   Vercel     │───►│   Railway    │───►│   Sepolia    │                  │
│  │  (Frontend)  │    │  (API+Rust)  │    │  (Contracts) │                  │
│  │              │    │              │    │              │                  │
│  │  React+Vite  │    │  Actix-web   │    │  Solidity    │                  │
│  └──────────────┘    └──────────────┘    └──────────────┘                  │
│         │                   │                    │                          │
│         └───────────────────┴────────────────────┘                          │
│                        HTTPS connections                                     │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Quick Start (Recommended)

### Option A: Monorepo Deployment (Easiest)

You can deploy the frontend directly from this monorepo using Vercel.

```bash
# 1. Install Vercel CLI
npm i -g vercel

# 2. Deploy frontend from monorepo root
cd zk-private-lending
vercel --cwd frontend
```

### Option B: Separate Repository (Alternative)

If you prefer a separate frontend repository:

```bash
# 1. Create new repo and copy frontend
mkdir zk-lending-frontend
cp -r frontend/* zk-lending-frontend/
cd zk-lending-frontend
git init && git add . && git commit -m "Initial frontend"

# 2. Deploy to Vercel
vercel
```

---

## Frontend Deployment (Vercel)

### Step 1: Connect Repository

1. Go to [vercel.com](https://vercel.com)
2. Click "New Project"
3. Import your GitHub repository: `nara020/zk-private-lending`
4. Configure as follows:

### Step 2: Configure Build Settings

| Setting | Value |
|---------|-------|
| **Framework Preset** | Vite |
| **Root Directory** | `frontend` |
| **Build Command** | `npm run build` |
| **Output Directory** | `dist` |
| **Install Command** | `npm ci` |

### Step 3: Environment Variables

Add these environment variables in Vercel Dashboard:

| Variable | Value | Description |
|----------|-------|-------------|
| `VITE_API_URL` | `https://your-api.railway.app` | Backend API URL |
| `VITE_CHAIN_ID` | `11155111` | Sepolia testnet |
| `VITE_CONTRACT_ADDRESS` | `0x...` | Deployed contract address |

### Step 4: Deploy

Click "Deploy" and wait for the build to complete.

**Custom Domain (Optional):**
```
Settings → Domains → Add Domain → zk-lending.yourdomain.com
```

---

## Backend Deployment (Railway)

### Step 1: Create Railway Project

1. Go to [railway.app](https://railway.app)
2. Click "New Project" → "Deploy from GitHub repo"
3. Select your repository

### Step 2: Configure Service

```bash
# Railway will auto-detect the Dockerfile
# Set the root directory to: api
```

### Step 3: Environment Variables

| Variable | Value |
|----------|-------|
| `DATABASE_URL` | Railway auto-generates |
| `HOST` | `0.0.0.0` |
| `PORT` | `3000` |
| `RUST_LOG` | `info,zk_lending_api=debug` |
| `ZK_PROVER_K` | `17` |

### Step 4: Add PostgreSQL

1. Click "New" → "Database" → "PostgreSQL"
2. Railway auto-connects the database

### Step 5: Generate Domain

1. Go to Settings → Networking → Generate Domain
2. Copy the URL (e.g., `zk-lending-api.railway.app`)
3. Use this as `VITE_API_URL` in Vercel

---

## Smart Contract Deployment (Sepolia)

### Prerequisites

```bash
# Install Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

### Step 1: Configure Environment

```bash
cd contracts

# Create .env file
cat > .env << 'EOF'
SEPOLIA_RPC_URL=https://sepolia.infura.io/v3/YOUR_INFURA_KEY
PRIVATE_KEY=0xYOUR_PRIVATE_KEY
ETHERSCAN_API_KEY=YOUR_ETHERSCAN_KEY
EOF
```

### Step 2: Deploy Contracts

```bash
# Load environment
source .env

# Deploy to Sepolia
forge script script/Deploy.s.sol:DeployScript \
  --rpc-url $SEPOLIA_RPC_URL \
  --private-key $PRIVATE_KEY \
  --broadcast \
  --verify \
  -vvv
```

### Step 3: Verify Contracts (If not auto-verified)

```bash
forge verify-contract \
  --chain-id 11155111 \
  --constructor-args $(cast abi-encode "constructor(address,address)" $USDC_ADDRESS $VERIFIER_ADDRESS) \
  $CONTRACT_ADDRESS \
  src/ZKLendingPool.sol:ZKLendingPool
```

### Step 4: Update Frontend

Copy the deployed contract addresses to Vercel environment variables.

---

## Alternative Deployment Options

### API Alternatives

| Platform | Pros | Cons |
|----------|------|------|
| **Railway** | Easy setup, auto-scaling | Paid after free tier |
| **Fly.io** | Great for Rust, edge computing | More complex setup |
| **Render** | Simple, free tier | Cold starts |
| **AWS ECS** | Production-grade | Complex, expensive |

### Fly.io Setup

```bash
# Install flyctl
curl -L https://fly.io/install.sh | sh

# Deploy
cd api
fly launch
fly secrets set DATABASE_URL=postgres://...
fly deploy
```

### Render Setup

1. Connect GitHub repository
2. Create new Web Service
3. Set:
   - Root Directory: `api`
   - Build Command: `cargo build --release`
   - Start Command: `./target/release/zk-lending-api`

---

## Demo Mode (No Backend)

For portfolio demos without a running backend:

### Option 1: Mock API

Create a mock API using Vercel Edge Functions:

```typescript
// frontend/api/price.ts (Vercel Edge Function)
export const config = { runtime: 'edge' };

export default function handler() {
  return Response.json({
    ethPrice: 2500,
    lastUpdated: new Date().toISOString(),
  });
}
```

### Option 2: Static Demo Data

Update `api.ts` to use fallback data:

```typescript
const DEMO_MODE = !import.meta.env.VITE_API_URL;

export const api = {
  getEthPrice: async () => {
    if (DEMO_MODE) {
      return { ethPrice: 2500, lastUpdated: new Date().toISOString() };
    }
    return fetchAPI('/api/price');
  },
  // ... other methods with fallbacks
};
```

---

## CI/CD Integration

### GitHub Actions (Automatic)

The CI pipeline already handles:
- Rust API tests
- Halo2 circuit tests
- Solidity contract tests
- E2E tests

### Vercel Integration

Vercel auto-deploys on push to main:

```yaml
# .github/workflows/vercel.yml (optional, for preview deployments)
name: Vercel Preview

on:
  pull_request:
    paths:
      - 'frontend/**'

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: amondnet/vercel-action@v25
        with:
          vercel-token: ${{ secrets.VERCEL_TOKEN }}
          vercel-org-id: ${{ secrets.VERCEL_ORG_ID }}
          vercel-project-id: ${{ secrets.VERCEL_PROJECT_ID }}
          working-directory: frontend
```

---

## Post-Deployment Checklist

### Frontend Verification

- [ ] Site loads at custom domain
- [ ] Wallet connection works
- [ ] Price feed displays correctly
- [ ] Forms render without errors
- [ ] Console has no errors

### Backend Verification

- [ ] Health endpoint returns 200: `curl https://api.yourdomain.com/health`
- [ ] Price API works: `curl https://api.yourdomain.com/api/price`
- [ ] Database connected (check logs)

### Contract Verification

- [ ] Contracts verified on Etherscan
- [ ] Test transactions succeed
- [ ] Events emit correctly

---

## Troubleshooting

### Common Issues

**1. CORS Errors**

Add CORS headers to your API:
```rust
// api/src/main.rs
.wrap(Cors::default()
    .allowed_origin("https://your-frontend.vercel.app")
    .allowed_methods(vec!["GET", "POST"])
    .allowed_headers(vec![header::CONTENT_TYPE])
)
```

**2. Environment Variables Not Loading**

Ensure variables start with `VITE_` for Vite to expose them:
```
VITE_API_URL=https://...  ✓
API_URL=https://...       ✗ (won't be available in browser)
```

**3. Build Fails on Vercel**

Check Node.js version:
```json
// frontend/package.json
{
  "engines": {
    "node": ">=18.0.0"
  }
}
```

**4. Contract Deployment Fails**

Check gas and nonce:
```bash
# Reset nonce if stuck
cast nonce $YOUR_ADDRESS --rpc-url $SEPOLIA_RPC_URL
```

---

## Cost Estimation

| Service | Free Tier | Paid Estimate |
|---------|-----------|---------------|
| Vercel (Frontend) | 100GB bandwidth | ~$20/mo |
| Railway (API) | 500 hours/mo | ~$10/mo |
| Sepolia (Contracts) | Free testnet | $0 |
| Infura (RPC) | 100k req/day | ~$50/mo |

**Total Estimated Cost**: $0 (free tier) to ~$80/mo (production)

---

## Security Considerations

### Production Security Checklist

- [ ] HTTPS only (enforced by Vercel/Railway)
- [ ] Environment variables secured
- [ ] Private keys never in code
- [ ] Rate limiting enabled on API
- [ ] CORS properly configured
- [ ] Contract admin keys secured (multisig recommended)

### Environment Variable Security

```bash
# Never commit these files
echo ".env" >> .gitignore
echo ".env.local" >> .gitignore
echo ".env.production" >> .gitignore
```

---

*Last updated: 2024-12-22*
