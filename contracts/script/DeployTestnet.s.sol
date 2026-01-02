// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console} from "forge-std/Script.sol";
import {ZKLendingPool} from "../src/ZKLendingPool.sol";
import {ZKVerifier} from "../src/ZKVerifier.sol";
import {CommitmentRegistry} from "../src/CommitmentRegistry.sol";
import {MockUSDC} from "../src/MockUSDC.sol";

/// @title Testnet Deployment Script
/// @notice Sepolia, Base Sepolia 테스트넷 배포 스크립트
/// @dev
/// == 사용법 ==
///
/// 1. 환경 변수 설정:
///    export PRIVATE_KEY=0x...
///    export SEPOLIA_RPC_URL=https://eth-sepolia.g.alchemy.com/v2/YOUR_KEY
///    export BASE_SEPOLIA_RPC_URL=https://base-sepolia.g.alchemy.com/v2/YOUR_KEY
///    export ETHERSCAN_API_KEY=YOUR_KEY
///    export BASESCAN_API_KEY=YOUR_KEY
///
/// 2. Sepolia 배포:
///    forge script script/DeployTestnet.s.sol:DeploySepolia \
///      --rpc-url $SEPOLIA_RPC_URL \
///      --private-key $PRIVATE_KEY \
///      --broadcast \
///      --verify \
///      -vvvv
///
/// 3. Base Sepolia 배포:
///    forge script script/DeployTestnet.s.sol:DeployBaseSepolia \
///      --rpc-url $BASE_SEPOLIA_RPC_URL \
///      --private-key $PRIVATE_KEY \
///      --broadcast \
///      --verify \
///      -vvvv
///
/// == Interview Q&A ==
///
/// Q: 테스트넷 배포 시 주의사항?
/// A: 1. 충분한 테스트넷 ETH 확보 (faucet)
///    2. 컨트랙트 검증 (verify) 필수
///    3. 배포 주소 기록 및 관리
///    4. 환경별 설정 분리
///
/// Q: 배포 후 검증 절차?
/// A: 1. 컨트랙트 소스 코드 검증
///    2. 권한 설정 확인
///    3. 기본 기능 테스트 (deposit/borrow)
///    4. 이벤트 발생 확인

/// @title Base Deployment Contract
/// @notice 공통 배포 로직
abstract contract BaseDeployment is Script {
    // 배포된 컨트랙트들
    ZKVerifier public verifier;
    CommitmentRegistry public registry;
    MockUSDC public usdc;
    ZKLendingPool public pool;

    // 초기 설정
    uint256 public constant INITIAL_ETH_PRICE = 2000_00000000; // $2000
    uint256 public constant INITIAL_LIQUIDITY = 100_000 * 1e6; // 100K USDC (테스트넷용)

    /// @notice 컨트랙트 배포
    function deployContracts() internal {
        console.log("==============================================");
        console.log("  ZK Private Lending - Testnet Deployment");
        console.log("==============================================");
        console.log("");
        console.log("Deployer:", msg.sender);
        console.log("Chain ID:", block.chainid);
        console.log("");

        // 1. ZKVerifier 배포
        console.log("[1/4] Deploying ZKVerifier...");
        verifier = new ZKVerifier();
        console.log("       Address:", address(verifier));

        // 2. CommitmentRegistry 배포
        console.log("[2/4] Deploying CommitmentRegistry...");
        registry = new CommitmentRegistry();
        console.log("       Address:", address(registry));

        // 3. MockUSDC 배포 (테스트넷용)
        console.log("[3/4] Deploying MockUSDC...");
        usdc = new MockUSDC();
        console.log("       Address:", address(usdc));

        // 4. ZKLendingPool 배포
        console.log("[4/4] Deploying ZKLendingPool...");
        pool = new ZKLendingPool(
            address(verifier),
            address(registry),
            address(usdc),
            INITIAL_ETH_PRICE
        );
        console.log("       Address:", address(pool));
    }

    /// @notice 권한 및 초기 설정
    function setupPermissions() internal {
        console.log("");
        console.log("Setting up permissions...");

        // CommitmentRegistry 권한 설정
        registry.setAuthorizedCaller(address(pool), true);
        console.log("  - LendingPool authorized on CommitmentRegistry");
    }

    /// @notice 초기 유동성 공급
    function supplyInitialLiquidity() internal {
        console.log("");
        console.log("Supplying initial liquidity...");

        usdc.approve(address(pool), INITIAL_LIQUIDITY);
        pool.supplyLiquidity(INITIAL_LIQUIDITY);
        console.log("  - Supplied", INITIAL_LIQUIDITY / 1e6, "USDC");
    }

    /// @notice 배포 결과 출력
    function printDeploymentSummary(string memory networkName) internal view {
        console.log("");
        console.log("==============================================");
        console.log("  Deployment Complete!");
        console.log("==============================================");
        console.log("");
        console.log("Network:", networkName);
        console.log("");
        console.log("Contract Addresses:");
        console.log("  ZKVerifier:        ", address(verifier));
        console.log("  CommitmentRegistry:", address(registry));
        console.log("  MockUSDC:          ", address(usdc));
        console.log("  ZKLendingPool:     ", address(pool));
        console.log("");
        console.log("Configuration:");
        console.log("  ETH Price:         $2,000");
        console.log("  Pool Liquidity:    100,000 USDC");
        console.log("  Max LTV:           75%");
        console.log("  Liquidation:       80%");
        console.log("  Base Interest:     5% APR");
        console.log("");
        console.log("Environment Variables (add to .env):");
        console.log("==============================================");
        printEnvVars();
        console.log("");
    }

    /// @notice 환경변수 형식 출력
    function printEnvVars() internal view {
        console.log("LENDING_POOL_ADDRESS=", address(pool));
        console.log("COMMITMENT_REGISTRY_ADDRESS=", address(registry));
        console.log("ZK_VERIFIER_ADDRESS=", address(verifier));
        console.log("USDC_ADDRESS=", address(usdc));
    }
}

/// @title Sepolia Deployment
contract DeploySepolia is BaseDeployment {
    function run() public {
        uint256 deployerKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerKey);

        deployContracts();
        setupPermissions();
        supplyInitialLiquidity();

        vm.stopBroadcast();

        printDeploymentSummary("Sepolia Testnet (Chain ID: 11155111)");
    }
}

/// @title Base Sepolia Deployment
contract DeployBaseSepolia is BaseDeployment {
    function run() public {
        uint256 deployerKey = vm.envUint("PRIVATE_KEY");
        vm.startBroadcast(deployerKey);

        deployContracts();
        setupPermissions();
        supplyInitialLiquidity();

        vm.stopBroadcast();

        printDeploymentSummary("Base Sepolia (Chain ID: 84532)");
    }
}

/// @title Dry Run (시뮬레이션)
/// @notice 실제 배포 전 테스트
contract DeployDryRun is BaseDeployment {
    function run() public {
        // 시뮬레이션 모드 (broadcast 없음)
        console.log("DRY RUN MODE - No actual deployment");
        console.log("==============================================");

        // 가상 주소로 컨트랙트 생성 (실제 배포 안 함)
        vm.startPrank(address(0xDEAD));

        deployContracts();
        setupPermissions();
        supplyInitialLiquidity();

        vm.stopPrank();

        printDeploymentSummary("Dry Run (Simulation)");

        // 예상 가스 비용 출력
        console.log("Estimated Gas Costs:");
        console.log("  ZKVerifier:         ~2,000,000 gas");
        console.log("  CommitmentRegistry: ~800,000 gas");
        console.log("  MockUSDC:           ~1,500,000 gas");
        console.log("  ZKLendingPool:      ~3,500,000 gas");
        console.log("  Total:              ~8,000,000 gas");
        console.log("");
        console.log("At 30 Gwei: ~0.24 ETH");
        console.log("At 50 Gwei: ~0.40 ETH");
    }
}

/// @title 배포 검증 스크립트
contract VerifyDeployment is Script {
    function run() public view {
        // 환경변수에서 배포된 주소 읽기
        address poolAddr = vm.envAddress("LENDING_POOL_ADDRESS");
        address registryAddr = vm.envAddress("COMMITMENT_REGISTRY_ADDRESS");
        address verifierAddr = vm.envAddress("ZK_VERIFIER_ADDRESS");
        address usdcAddr = vm.envAddress("USDC_ADDRESS");

        console.log("Verifying deployment...");
        console.log("");

        // 1. 컨트랙트 코드 존재 확인
        require(poolAddr.code.length > 0, "LendingPool not deployed");
        require(registryAddr.code.length > 0, "Registry not deployed");
        require(verifierAddr.code.length > 0, "Verifier not deployed");
        require(usdcAddr.code.length > 0, "USDC not deployed");
        console.log("[OK] All contracts deployed");

        // 2. LendingPool 설정 확인
        ZKLendingPool pool = ZKLendingPool(payable(poolAddr));
        require(pool.MAX_LTV() == 75, "Invalid MAX_LTV");
        require(pool.LIQUIDATION_THRESHOLD() == 80, "Invalid LIQUIDATION_THRESHOLD");
        console.log("[OK] LendingPool configuration valid");

        // 3. 권한 설정 확인
        CommitmentRegistry registry = CommitmentRegistry(registryAddr);
        require(registry.authorizedCallers(poolAddr), "Pool not authorized");
        console.log("[OK] Pool authorized on Registry");

        // 4. 유동성 확인
        MockUSDC usdc = MockUSDC(usdcAddr);
        uint256 poolBalance = usdc.balanceOf(poolAddr);
        console.log("Pool USDC balance:", poolBalance / 1e6, "USDC");

        console.log("");
        console.log("Verification complete! All checks passed.");
    }
}
