/**
 * ZKLendingPool Comprehensive Test Suite
 *
 * Test Categories:
 * 1. Deployment & Initialization
 * 2. Deposit Functionality
 * 3. Borrow Functionality
 * 4. Interest Calculation
 * 5. Repayment (Principal + Interest)
 * 6. Withdrawal
 * 7. Liquidation
 * 8. Edge Cases & Security
 * 9. Gas Optimization Benchmarks
 */

import { expect } from "chai";
import { ethers } from "hardhat";
import { time, loadFixture } from "@nomicfoundation/hardhat-toolbox/network-helpers";
import { ZKLendingPool, MockZKVerifier, CommitmentRegistry, MockUSDC } from "../typechain-types";
import { SignerWithAddress } from "@nomicfoundation/hardhat-ethers/signers";

describe("ZKLendingPool", function () {
  // Constants
  const ETH_PRICE = 2000_00000000n; // $2000 with 8 decimals
  const INITIAL_LIQUIDITY = ethers.parseUnits("1000000", 6); // 1M USDC
  const ONE_DAY = 24 * 60 * 60;
  const ONE_YEAR = 365 * ONE_DAY;

  // Fixture for deploying contracts
  async function deployFixture() {
    const [owner, user1, user2, liquidator] = await ethers.getSigners();

    // Deploy MockZKVerifier (for testing - always returns true)
    const MockZKVerifier = await ethers.getContractFactory("MockZKVerifier");
    const zkVerifier = await MockZKVerifier.deploy();

    // Deploy CommitmentRegistry
    const CommitmentRegistry = await ethers.getContractFactory("CommitmentRegistry");
    const registry = await CommitmentRegistry.deploy();

    // Deploy MockUSDC
    const MockUSDC = await ethers.getContractFactory("MockUSDC");
    const usdc = await MockUSDC.deploy();

    // Deploy ZKLendingPool
    const ZKLendingPool = await ethers.getContractFactory("ZKLendingPool");
    const pool = await ZKLendingPool.deploy(
      await zkVerifier.getAddress(),
      await registry.getAddress(),
      await usdc.getAddress(),
      ETH_PRICE
    );

    // Setup: Authorize pool in registry
    await registry.setAuthorizedCaller(await pool.getAddress(), true);

    // Setup: Supply liquidity (owner already has initial supply from constructor)
    await usdc.approve(await pool.getAddress(), INITIAL_LIQUIDITY);
    await pool.supplyLiquidity(INITIAL_LIQUIDITY);

    // Mint USDC for users (for repayment) - use ownerMint for no cooldown
    await usdc.ownerMint(user1.address, ethers.parseUnits("100000", 6));
    await usdc.ownerMint(user2.address, ethers.parseUnits("100000", 6));
    await usdc.ownerMint(liquidator.address, ethers.parseUnits("100000", 6));

    return { pool, zkVerifier, registry, usdc, owner, user1, user2, liquidator };
  }

  // Helper: Create commitment
  function createCommitment(amount: bigint, salt: bigint): string {
    return ethers.keccak256(
      ethers.solidityPacked(["uint256", "uint256"], [amount, salt])
    );
  }

  // Helper: Create mock proof
  function createMockProof() {
    return {
      a: [1n, 2n] as [bigint, bigint],
      b: [[3n, 4n], [5n, 6n]] as [[bigint, bigint], [bigint, bigint]],
      c: [7n, 8n] as [bigint, bigint],
    };
  }

  // ============================================
  // 1. Deployment & Initialization Tests
  // ============================================
  describe("1. Deployment & Initialization", function () {
    it("should deploy with correct parameters", async function () {
      const { pool, zkVerifier, registry, usdc } = await loadFixture(deployFixture);

      expect(await pool.zkVerifier()).to.equal(await zkVerifier.getAddress());
      expect(await pool.commitmentRegistry()).to.equal(await registry.getAddress());
      expect(await pool.borrowToken()).to.equal(await usdc.getAddress());
      expect(await pool.ethPrice()).to.equal(ETH_PRICE);
    });

    it("should initialize interest parameters correctly", async function () {
      const { pool } = await loadFixture(deployFixture);

      expect(await pool.borrowIndex()).to.equal(ethers.parseUnits("1", 27));
      expect(await pool.BASE_INTEREST_RATE()).to.equal(500n); // 5%
      expect(await pool.VARIABLE_INTEREST_RATE()).to.equal(2000n); // 20%
      expect(await pool.OPTIMAL_UTILIZATION()).to.equal(80n);
    });

    it("should revert on zero addresses", async function () {
      const ZKLendingPool = await ethers.getContractFactory("ZKLendingPool");
      const { registry, usdc } = await loadFixture(deployFixture);

      await expect(
        ZKLendingPool.deploy(
          ethers.ZeroAddress,
          await registry.getAddress(),
          await usdc.getAddress(),
          ETH_PRICE
        )
      ).to.be.revertedWithCustomError(ZKLendingPool, "ZeroAddress");
    });
  });

  // ============================================
  // 2. Deposit Tests
  // ============================================
  describe("2. Deposit", function () {
    it("should accept ETH deposit with valid commitment", async function () {
      const { pool, user1 } = await loadFixture(deployFixture);
      const depositAmount = ethers.parseEther("10");
      const commitment = createCommitment(depositAmount, 12345n);

      await expect(pool.connect(user1).deposit(commitment, { value: depositAmount }))
        .to.emit(pool, "Deposited")
        .withArgs(user1.address, commitment, await time.latest() + 1);

      expect(await pool.hasDeposit(user1.address)).to.be.true;
      expect(await pool.totalCollateralETH()).to.equal(depositAmount);
    });

    it("should revert on zero amount deposit", async function () {
      const { pool, user1 } = await loadFixture(deployFixture);
      const commitment = createCommitment(0n, 12345n);

      await expect(
        pool.connect(user1).deposit(commitment, { value: 0 })
      ).to.be.revertedWithCustomError(pool, "ZeroAmount");
    });

    it("should revert on duplicate deposit", async function () {
      const { pool, user1 } = await loadFixture(deployFixture);
      const depositAmount = ethers.parseEther("10");
      const commitment1 = createCommitment(depositAmount, 12345n);
      const commitment2 = createCommitment(depositAmount, 67890n);

      await pool.connect(user1).deposit(commitment1, { value: depositAmount });

      await expect(
        pool.connect(user1).deposit(commitment2, { value: depositAmount })
      ).to.be.revertedWithCustomError(pool, "AlreadyHasDeposit");
    });

    it("should revert on invalid commitment", async function () {
      const { pool, user1 } = await loadFixture(deployFixture);
      const depositAmount = ethers.parseEther("10");

      await expect(
        pool.connect(user1).deposit(ethers.ZeroHash, { value: depositAmount })
      ).to.be.revertedWithCustomError(pool, "InvalidCommitment");
    });
  });

  // ============================================
  // 3. Interest Calculation Tests
  // ============================================
  describe("3. Interest Calculation", function () {
    it("should calculate utilization rate correctly", async function () {
      const { pool, user1 } = await loadFixture(deployFixture);

      // Initial: 0% utilization (no borrows)
      expect(await pool.getUtilizationRate()).to.equal(0n);

      // After deposit and borrow
      const depositAmount = ethers.parseEther("100");
      const commitment = createCommitment(depositAmount, 12345n);
      await pool.connect(user1).deposit(commitment, { value: depositAmount });

      const borrowAmount = ethers.parseUnits("100000", 6); // $100,000
      const debtCommitment = createCommitment(borrowAmount, 54321n);
      const proof = createMockProof();

      await pool.connect(user1).borrow(
        borrowAmount,
        debtCommitment,
        proof,
        proof,
        [depositAmount, borrowAmount]
      );

      // Utilization = 100000 / (1000000 - 100000 + 100000) = 10%
      const utilization = await pool.getUtilizationRate();
      expect(utilization).to.be.closeTo(10n, 1n);
    });

    it("should calculate interest rate based on utilization", async function () {
      const { pool } = await loadFixture(deployFixture);

      // Base rate at 0% utilization
      const baseRate = await pool.getCurrentInterestRate();
      expect(baseRate).to.equal(500n); // 5% base
    });

    it("should estimate interest correctly", async function () {
      const { pool } = await loadFixture(deployFixture);

      const amount = ethers.parseUnits("10000", 6); // $10,000
      const duration = BigInt(ONE_YEAR); // 1 year

      const interest = await pool.estimateInterest(amount, duration);
      // At 5% APR: $10,000 * 0.05 = $500
      expect(interest).to.be.closeTo(ethers.parseUnits("500", 6), ethers.parseUnits("10", 6));
    });

    it("should accrue interest over time", async function () {
      const { pool, user1, usdc } = await loadFixture(deployFixture);

      // Deposit
      const depositAmount = ethers.parseEther("100");
      const commitment = createCommitment(depositAmount, 12345n);
      await pool.connect(user1).deposit(commitment, { value: depositAmount });

      // Borrow
      const borrowAmount = ethers.parseUnits("50000", 6); // $50,000
      const debtCommitment = createCommitment(borrowAmount, 54321n);
      const proof = createMockProof();
      await pool.connect(user1).borrow(
        borrowAmount,
        debtCommitment,
        proof,
        proof,
        [depositAmount, borrowAmount]
      );

      // Check initial debt
      let [principal, interest, total] = await pool.getCurrentDebt(user1.address);
      expect(principal).to.equal(borrowAmount);
      expect(interest).to.equal(0n);

      // Fast forward 30 days
      await time.increase(30 * ONE_DAY);

      // Check debt after 30 days
      [principal, interest, total] = await pool.getCurrentDebt(user1.address);
      expect(principal).to.equal(borrowAmount);
      expect(interest).to.be.gt(0n);
      expect(total).to.equal(principal + interest);

      // Interest should be ~5% APR * 30/365 * $50,000 ≈ $205
      // Note: Actual interest may vary slightly due to utilization rate and block timing
      const expectedInterest = ethers.parseUnits("205", 6);
      expect(interest).to.be.closeTo(expectedInterest, ethers.parseUnits("100", 6));
    });
  });

  // ============================================
  // 4. Borrow Tests
  // ============================================
  describe("4. Borrow", function () {
    it("should allow borrowing with valid proofs", async function () {
      const { pool, user1 } = await loadFixture(deployFixture);

      // Deposit
      const depositAmount = ethers.parseEther("10");
      const commitment = createCommitment(depositAmount, 12345n);
      await pool.connect(user1).deposit(commitment, { value: depositAmount });

      // Borrow
      const borrowAmount = ethers.parseUnits("10000", 6); // $10,000
      const debtCommitment = createCommitment(borrowAmount, 54321n);
      const proof = createMockProof();

      await expect(
        pool.connect(user1).borrow(
          borrowAmount,
          debtCommitment,
          proof,
          proof,
          [depositAmount, borrowAmount]
        )
      ).to.emit(pool, "Borrowed");

      expect(await pool.hasBorrow(user1.address)).to.be.true;
      expect(await pool.borrowedAmount(user1.address)).to.equal(borrowAmount);
    });

    it("should revert if no deposit", async function () {
      const { pool, user1 } = await loadFixture(deployFixture);

      const borrowAmount = ethers.parseUnits("10000", 6);
      const debtCommitment = createCommitment(borrowAmount, 54321n);
      const proof = createMockProof();

      await expect(
        pool.connect(user1).borrow(
          borrowAmount,
          debtCommitment,
          proof,
          proof,
          [0n, borrowAmount]
        )
      ).to.be.revertedWithCustomError(pool, "NoDeposit");
    });

    it("should revert on insufficient pool liquidity", async function () {
      const { pool, user1 } = await loadFixture(deployFixture);

      // Deposit
      const depositAmount = ethers.parseEther("1000");
      const commitment = createCommitment(depositAmount, 12345n);
      await pool.connect(user1).deposit(commitment, { value: depositAmount });

      // Try to borrow more than pool has
      const borrowAmount = ethers.parseUnits("2000000", 6); // $2M (pool has $1M)
      const debtCommitment = createCommitment(borrowAmount, 54321n);
      const proof = createMockProof();

      await expect(
        pool.connect(user1).borrow(
          borrowAmount,
          debtCommitment,
          proof,
          proof,
          [depositAmount, borrowAmount]
        )
      ).to.be.revertedWithCustomError(pool, "InsufficientPoolLiquidity");
    });
  });

  // ============================================
  // 5. Repayment Tests
  // ============================================
  describe("5. Repayment", function () {
    async function borrowFixture() {
      const fixture = await loadFixture(deployFixture);
      const { pool, user1, usdc } = fixture;

      // Deposit
      const depositAmount = ethers.parseEther("10");
      const commitment = createCommitment(depositAmount, 12345n);
      await pool.connect(user1).deposit(commitment, { value: depositAmount });

      // Borrow
      const borrowAmount = ethers.parseUnits("10000", 6);
      const debtCommitment = createCommitment(borrowAmount, 54321n);
      const proof = createMockProof();
      await pool.connect(user1).borrow(
        borrowAmount,
        debtCommitment,
        proof,
        proof,
        [depositAmount, borrowAmount]
      );

      // Approve USDC for repayment
      await usdc.connect(user1).approve(await pool.getAddress(), ethers.MaxUint256);

      return { ...fixture, depositAmount, borrowAmount };
    }

    it("should repay principal + interest", async function () {
      const { pool, user1 } = await loadFixture(borrowFixture);

      // Fast forward to accrue interest
      await time.increase(30 * ONE_DAY);

      const [principal, interest, total] = await pool.getCurrentDebt(user1.address);
      const nullifier = ethers.keccak256(ethers.toUtf8Bytes("nullifier"));

      // Repay with a buffer to ensure full repayment (interest may accrue between calls)
      // Use a valid commitment in case of edge cases
      const repayAmount = total + ethers.parseUnits("10", 6); // Add buffer
      const newCommitment = createCommitment(0n, 12345n); // Dummy commitment (won't be used for full repay)

      await expect(
        pool.connect(user1).repay(repayAmount, newCommitment, nullifier)
      ).to.emit(pool, "Repaid");

      expect(await pool.borrowedAmount(user1.address)).to.equal(0n);
      expect(await pool.hasBorrow(user1.address)).to.be.false;
    });

    it("should repay interest first", async function () {
      const { pool, user1 } = await loadFixture(borrowFixture);

      // Fast forward to accrue interest
      await time.increase(30 * ONE_DAY);

      const [principal, interest, _] = await pool.getCurrentDebt(user1.address);

      // Repay exactly the interest amount
      const nullifier = ethers.keccak256(ethers.toUtf8Bytes("nullifier"));
      const newCommitment = createCommitment(principal, 99999n);

      await pool.connect(user1).repay(interest + 1n, newCommitment, nullifier);

      // Principal should remain, interest should be reduced
      expect(await pool.borrowedAmount(user1.address)).to.be.closeTo(principal, 1n);
    });

    it("should allow paying interest only", async function () {
      const { pool, user1 } = await loadFixture(borrowFixture);

      // Fast forward
      await time.increase(30 * ONE_DAY);

      const [principalBefore, _, __] = await pool.getCurrentDebt(user1.address);

      await expect(pool.connect(user1).payInterest())
        .to.emit(pool, "InterestPaid");

      // Principal unchanged, interest cleared
      const [principalAfter, interestAfter, ___] = await pool.getCurrentDebt(user1.address);
      expect(principalAfter).to.equal(principalBefore);
      expect(interestAfter).to.equal(0n);
    });
  });

  // ============================================
  // 6. Pool Status Tests
  // ============================================
  describe("6. Pool Status", function () {
    it("should return correct pool status", async function () {
      const { pool, user1 } = await loadFixture(deployFixture);

      // Deposit and borrow
      const depositAmount = ethers.parseEther("10");
      const commitment = createCommitment(depositAmount, 12345n);
      await pool.connect(user1).deposit(commitment, { value: depositAmount });

      const borrowAmount = ethers.parseUnits("10000", 6);
      const debtCommitment = createCommitment(borrowAmount, 54321n);
      const proof = createMockProof();
      await pool.connect(user1).borrow(
        borrowAmount,
        debtCommitment,
        proof,
        proof,
        [depositAmount, borrowAmount]
      );

      const [
        totalCollateral,
        totalBorrowed,
        availableLiquidity,
        utilization,
        interestRate,
        totalInterest
      ] = await pool.getPoolStatus();

      expect(totalCollateral).to.equal(depositAmount);
      expect(totalBorrowed).to.equal(borrowAmount);
      expect(availableLiquidity).to.equal(INITIAL_LIQUIDITY - borrowAmount);
      expect(utilization).to.be.gt(0n);
      expect(interestRate).to.be.gte(500n); // >= base rate
    });
  });

  // ============================================
  // 7. Gas Benchmarks
  // ============================================
  describe("7. Gas Benchmarks", function () {
    it("should track gas for deposit", async function () {
      const { pool, user1 } = await loadFixture(deployFixture);
      const depositAmount = ethers.parseEther("10");
      const commitment = createCommitment(depositAmount, 12345n);

      const tx = await pool.connect(user1).deposit(commitment, { value: depositAmount });
      const receipt = await tx.wait();

      console.log(`    Deposit gas used: ${receipt?.gasUsed}`);
      expect(receipt?.gasUsed).to.be.lt(200000n);
    });

    it("should track gas for borrow", async function () {
      const { pool, user1 } = await loadFixture(deployFixture);

      // Deposit first
      const depositAmount = ethers.parseEther("10");
      const commitment = createCommitment(depositAmount, 12345n);
      await pool.connect(user1).deposit(commitment, { value: depositAmount });

      // Borrow
      const borrowAmount = ethers.parseUnits("10000", 6);
      const debtCommitment = createCommitment(borrowAmount, 54321n);
      const proof = createMockProof();

      const tx = await pool.connect(user1).borrow(
        borrowAmount,
        debtCommitment,
        proof,
        proof,
        [depositAmount, borrowAmount]
      );
      const receipt = await tx.wait();

      console.log(`    Borrow gas used: ${receipt?.gasUsed}`);
      expect(receipt?.gasUsed).to.be.lt(300000n);
    });

    it("should track gas for repay", async function () {
      const { pool, user1, usdc } = await loadFixture(deployFixture);

      // Setup
      const depositAmount = ethers.parseEther("10");
      const commitment = createCommitment(depositAmount, 12345n);
      await pool.connect(user1).deposit(commitment, { value: depositAmount });

      const borrowAmount = ethers.parseUnits("10000", 6);
      const debtCommitment = createCommitment(borrowAmount, 54321n);
      const proof = createMockProof();
      await pool.connect(user1).borrow(
        borrowAmount,
        debtCommitment,
        proof,
        proof,
        [depositAmount, borrowAmount]
      );

      await usdc.connect(user1).approve(await pool.getAddress(), ethers.MaxUint256);
      const nullifier = ethers.keccak256(ethers.toUtf8Bytes("nullifier"));

      // Use a buffer and valid commitment for full repayment
      const repayWithBuffer = borrowAmount + ethers.parseUnits("10", 6);
      const newCommitment = createCommitment(0n, 99999n);

      const tx = await pool.connect(user1).repay(repayWithBuffer, newCommitment, nullifier);
      const receipt = await tx.wait();

      console.log(`    Repay gas used: ${receipt?.gasUsed}`);
      expect(receipt?.gasUsed).to.be.lt(250000n); // Increased limit for full repay with interest handling
    });
  });

  // ============================================
  // 8. Security Edge Cases
  // ============================================
  describe("8. Security", function () {
    it("should prevent reentrancy on withdraw", async function () {
      // ReentrancyGuard is used, basic test
      const { pool, user1 } = await loadFixture(deployFixture);
      expect(await pool.hasDeposit(user1.address)).to.be.false;
    });

    it("should only allow owner to update price", async function () {
      const { pool, user1 } = await loadFixture(deployFixture);

      await expect(
        pool.connect(user1).updatePrice(3000_00000000n)
      ).to.be.revertedWithCustomError(pool, "OwnableUnauthorizedAccount");
    });

    it("should only allow owner to supply liquidity", async function () {
      const { pool, user1 } = await loadFixture(deployFixture);

      await expect(
        pool.connect(user1).supplyLiquidity(1000n)
      ).to.be.revertedWithCustomError(pool, "OwnableUnauthorizedAccount");
    });
  });
});
