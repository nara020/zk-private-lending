import { ethers } from "hardhat";
import * as fs from "fs";
import * as path from "path";

async function main() {
  const [deployer] = await ethers.getSigners();

  console.log("Deploying contracts with account:", deployer.address);
  console.log("Account balance:", ethers.formatEther(await ethers.provider.getBalance(deployer.address)));

  // 1. Deploy ZKVerifier
  console.log("\n1. Deploying ZKVerifier...");
  const ZKVerifier = await ethers.getContractFactory("ZKVerifier");
  const zkVerifier = await ZKVerifier.deploy();
  await zkVerifier.waitForDeployment();
  const zkVerifierAddress = await zkVerifier.getAddress();
  console.log("   ZKVerifier deployed to:", zkVerifierAddress);

  // 2. Deploy CommitmentRegistry
  console.log("\n2. Deploying CommitmentRegistry...");
  const CommitmentRegistry = await ethers.getContractFactory("CommitmentRegistry");
  const commitmentRegistry = await CommitmentRegistry.deploy();
  await commitmentRegistry.waitForDeployment();
  const commitmentRegistryAddress = await commitmentRegistry.getAddress();
  console.log("   CommitmentRegistry deployed to:", commitmentRegistryAddress);

  // 3. Deploy MockUSDC
  console.log("\n3. Deploying MockUSDC...");
  const MockUSDC = await ethers.getContractFactory("MockUSDC");
  const mockUSDC = await MockUSDC.deploy();
  await mockUSDC.waitForDeployment();
  const mockUSDCAddress = await mockUSDC.getAddress();
  console.log("   MockUSDC deployed to:", mockUSDCAddress);

  // 4. Deploy ZKLendingPool
  console.log("\n4. Deploying ZKLendingPool...");
  const initialEthPrice = 2000_00000000n; // $2000 with 8 decimals
  const ZKLendingPool = await ethers.getContractFactory("ZKLendingPool");
  const lendingPool = await ZKLendingPool.deploy(
    zkVerifierAddress,
    commitmentRegistryAddress,
    mockUSDCAddress,
    initialEthPrice
  );
  await lendingPool.waitForDeployment();
  const lendingPoolAddress = await lendingPool.getAddress();
  console.log("   ZKLendingPool deployed to:", lendingPoolAddress);

  // 5. Configure CommitmentRegistry
  console.log("\n5. Configuring CommitmentRegistry...");
  await commitmentRegistry.setAuthorizedCaller(lendingPoolAddress, true);
  console.log("   LendingPool authorized in CommitmentRegistry");

  // 6. Supply initial liquidity to LendingPool
  console.log("\n6. Supplying initial liquidity...");
  const initialLiquidity = 1_000_000n * 10n ** 6n; // 1M USDC
  await mockUSDC.approve(lendingPoolAddress, initialLiquidity);
  await lendingPool.supplyLiquidity(initialLiquidity);
  console.log("   Supplied 1,000,000 USDC to LendingPool");

  // Summary
  console.log("\n========================================");
  console.log("           DEPLOYMENT COMPLETE          ");
  console.log("========================================");
  console.log("ZKVerifier:          ", zkVerifierAddress);
  console.log("CommitmentRegistry:  ", commitmentRegistryAddress);
  console.log("MockUSDC:            ", mockUSDCAddress);
  console.log("ZKLendingPool:       ", lendingPoolAddress);
  console.log("========================================");

  // Save addresses to file for frontend
  const deploymentInfo = {
    network: "localhost",
    chainId: 31337,
    contracts: {
      ZKVerifier: zkVerifierAddress,
      CommitmentRegistry: commitmentRegistryAddress,
      MockUSDC: mockUSDCAddress,
      ZKLendingPool: lendingPoolAddress,
    },
    timestamp: new Date().toISOString(),
  };

  const outputPath = path.join(__dirname, "..", "deployments.json");
  fs.writeFileSync(outputPath, JSON.stringify(deploymentInfo, null, 2));
  console.log("\nDeployment info saved to:", outputPath);

  // Generate frontend .env content
  const envContent = `# Contract Addresses (Local Hardhat Network)
VITE_LENDING_POOL_ADDRESS=${lendingPoolAddress}
VITE_USDC_ADDRESS=${mockUSDCAddress}
VITE_CHAIN_ID=31337
VITE_NETWORK_NAME=Localhost
VITE_RPC_URL=http://127.0.0.1:8545
VITE_API_URL=http://localhost:3001
`;

  const frontendEnvPath = path.join(__dirname, "..", "..", "frontend", ".env.local");
  fs.writeFileSync(frontendEnvPath, envContent);
  console.log("Frontend .env.local saved to:", frontendEnvPath);
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
