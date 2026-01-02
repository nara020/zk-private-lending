// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console} from "forge-std/Script.sol";
import {ZKLendingPool} from "../src/ZKLendingPool.sol";
import {ZKVerifier} from "../src/ZKVerifier.sol";
import {CommitmentRegistry} from "../src/CommitmentRegistry.sol";
import {MockUSDC} from "../src/MockUSDC.sol";

/// @title Deploy Script
/// @notice ZK Private Lending 전체 시스템 배포
/// @dev
/// 사용법:
///   # 로컬 (Anvil)
///   forge script script/Deploy.s.sol --rpc-url http://localhost:8545 --broadcast
///
///   # Sepolia 테스트넷
///   forge script script/Deploy.s.sol --rpc-url $SEPOLIA_RPC_URL --private-key $PRIVATE_KEY --broadcast --verify
///
contract DeployScript is Script {
    // 배포된 컨트랙트 주소들
    ZKVerifier public verifier;
    CommitmentRegistry public registry;
    MockUSDC public usdc;
    ZKLendingPool public pool;

    // 초기 설정값
    uint256 public constant INITIAL_ETH_PRICE = 2000_00000000; // $2000 (8 decimals)
    uint256 public constant INITIAL_LIQUIDITY = 1_000_000 * 1e6; // 100만 USDC

    function run() public {
        uint256 deployerPrivateKey = vm.envOr("PRIVATE_KEY", uint256(0));

        // Anvil 기본 키 사용 (로컬 테스트용)
        if (deployerPrivateKey == 0) {
            deployerPrivateKey = 0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80;
        }

        vm.startBroadcast(deployerPrivateKey);

        console.log("===========================================");
        console.log("  ZK Private Lending - Deployment Start");
        console.log("===========================================");
        console.log("");

        // 1. ZKVerifier 배포
        console.log("1. Deploying ZKVerifier...");
        verifier = new ZKVerifier();
        console.log("   ZKVerifier deployed at:", address(verifier));

        // 2. CommitmentRegistry 배포
        console.log("2. Deploying CommitmentRegistry...");
        registry = new CommitmentRegistry();
        console.log("   CommitmentRegistry deployed at:", address(registry));

        // 3. MockUSDC 배포
        console.log("3. Deploying MockUSDC...");
        usdc = new MockUSDC();
        console.log("   MockUSDC deployed at:", address(usdc));

        // 4. ZKLendingPool 배포
        console.log("4. Deploying ZKLendingPool...");
        pool = new ZKLendingPool(
            address(verifier),
            address(registry),
            address(usdc),
            INITIAL_ETH_PRICE
        );
        console.log("   ZKLendingPool deployed at:", address(pool));

        // 5. 권한 설정
        console.log("5. Setting up permissions...");
        registry.setAuthorizedCaller(address(pool), true);
        console.log("   LendingPool authorized on CommitmentRegistry");

        // 6. 초기 유동성 공급
        console.log("6. Supplying initial liquidity...");
        usdc.approve(address(pool), INITIAL_LIQUIDITY);
        pool.supplyLiquidity(INITIAL_LIQUIDITY);
        console.log("   Supplied", INITIAL_LIQUIDITY / 1e6, "USDC to pool");

        vm.stopBroadcast();

        // 배포 요약
        console.log("");
        console.log("===========================================");
        console.log("  Deployment Complete!");
        console.log("===========================================");
        console.log("");
        console.log("Contract Addresses:");
        console.log("  ZKVerifier:        ", address(verifier));
        console.log("  CommitmentRegistry:", address(registry));
        console.log("  MockUSDC:          ", address(usdc));
        console.log("  ZKLendingPool:     ", address(pool));
        console.log("");
        console.log("Initial Settings:");
        console.log("  ETH Price:         $2000");
        console.log("  Pool Liquidity:    1,000,000 USDC");
        console.log("  Max LTV:           75%");
        console.log("  Liquidation:       80%");
        console.log("");
        console.log("Next Steps:");
        console.log("  1. Set Verification Keys: verifier.setVerificationKey(...)");
        console.log("  2. Test deposit: pool.deposit{value: 1 ether}(commitment)");
        console.log("  3. Generate ZK proofs in Rust backend");
        console.log("");
    }
}

/// @title 로컬 테스트용 배포 스크립트
contract DeployLocal is Script {
    function run() public {
        // Anvil 기본 계정 사용
        vm.startBroadcast();

        ZKVerifier verifier = new ZKVerifier();
        CommitmentRegistry registry = new CommitmentRegistry();
        MockUSDC usdc = new MockUSDC();

        ZKLendingPool pool = new ZKLendingPool(
            address(verifier),
            address(registry),
            address(usdc),
            2000_00000000 // $2000
        );

        registry.setAuthorizedCaller(address(pool), true);

        // 유동성 공급
        usdc.approve(address(pool), 1_000_000 * 1e6);
        pool.supplyLiquidity(1_000_000 * 1e6);

        vm.stopBroadcast();

        console.log("Pool:", address(pool));
        console.log("USDC:", address(usdc));
    }
}
