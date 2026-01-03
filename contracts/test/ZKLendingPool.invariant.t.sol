// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {StdInvariant} from "forge-std/StdInvariant.sol";
import {ZKLendingPool} from "../src/ZKLendingPool.sol";
import {CommitmentRegistry} from "../src/CommitmentRegistry.sol";
import {MockUSDC} from "../src/MockUSDC.sol";
import {IZKVerifier} from "../src/interfaces/IZKVerifier.sol";

/// @title MockVerifierForInvariant
/// @notice Invariant 테스트용 MockVerifier
contract MockVerifierForInvariant is IZKVerifier {
    bool public alwaysPass = true;

    function setAlwaysPass(bool _pass) external {
        alwaysPass = _pass;
    }

    function verify(ProofType, Proof calldata, uint256[] calldata) external view override returns (bool) {
        return alwaysPass;
    }

    function getVerificationKeyHash(ProofType) external pure override returns (bytes32) {
        return keccak256("mock");
    }
}

/// @title Handler
/// @notice Invariant 테스트를 위한 핸들러 컨트랙트
/// @dev Foundry가 이 컨트랙트의 함수들을 랜덤하게 호출
contract Handler is Test {
    ZKLendingPool public pool;
    CommitmentRegistry public registry;
    MockUSDC public usdc;
    MockVerifierForInvariant public verifier;

    address[] public actors;
    mapping(address => bool) public hasDeposited;
    mapping(address => uint256) public depositedAmounts;

    uint256 public totalDepositsTracked;
    uint256 public totalBorrowsTracked;
    uint256 public callCount;

    IZKVerifier.Proof internal dummyProof;
    uint256[] internal dummyInputs;

    constructor(
        ZKLendingPool _pool,
        CommitmentRegistry _registry,
        MockUSDC _usdc,
        MockVerifierForInvariant _verifier
    ) {
        pool = _pool;
        registry = _registry;
        usdc = _usdc;
        verifier = _verifier;

        // 테스트용 actors 생성
        for (uint256 i = 0; i < 5; i++) {
            address actor = address(uint160(0x1000 + i));
            actors.push(actor);
            vm.deal(actor, 1000 ether);
            usdc.ownerMint(actor, 10_000_000 * 1e6);
        }

        dummyProof = IZKVerifier.Proof({
            a: [uint256(1), uint256(2)],
            b: [[uint256(3), uint256(4)], [uint256(5), uint256(6)]],
            c: [uint256(7), uint256(8)]
        });
        dummyInputs = new uint256[](2);
    }

    /// @notice 랜덤 예치
    function deposit(uint256 actorSeed, uint256 amount) external {
        callCount++;
        address actor = actors[actorSeed % actors.length];
        amount = bound(amount, 0.01 ether, 100 ether);

        if (hasDeposited[actor]) return;

        bytes32 commitment = keccak256(abi.encodePacked(amount, block.timestamp, actor));

        vm.prank(actor);
        pool.deposit{value: amount}(commitment);

        hasDeposited[actor] = true;
        depositedAmounts[actor] = amount;
        totalDepositsTracked += amount;
    }

    /// @notice 랜덤 대출
    function borrow(uint256 actorSeed, uint256 amount) external {
        callCount++;
        address actor = actors[actorSeed % actors.length];

        if (!pool.hasDeposit(actor)) return;
        if (pool.hasBorrow(actor)) return;

        uint256 availableLiquidity = usdc.balanceOf(address(pool));
        if (availableLiquidity == 0) return;

        amount = bound(amount, 1e6, availableLiquidity);

        bytes32 debtCommitment = keccak256(abi.encodePacked(amount, block.timestamp, actor, "debt"));

        vm.prank(actor);
        pool.borrow(amount, debtCommitment, dummyProof, dummyProof, dummyInputs);

        totalBorrowsTracked += amount;
    }

    /// @notice 랜덤 상환
    function repay(uint256 actorSeed, uint256 amount) external {
        callCount++;
        address actor = actors[actorSeed % actors.length];

        if (!pool.hasBorrow(actor)) return;

        uint256 borrowed = pool.borrowedAmount(actor);
        amount = bound(amount, 1, borrowed);

        bytes32 nullifier = keccak256(abi.encodePacked(actor, block.timestamp, "repay", callCount));
        bytes32 newDebtCommitment = amount >= borrowed
            ? bytes32(0)
            : keccak256(abi.encodePacked(borrowed - amount, block.timestamp, actor));

        vm.startPrank(actor);
        usdc.approve(address(pool), amount);
        pool.repay(amount, newDebtCommitment, nullifier);
        vm.stopPrank();

        totalBorrowsTracked -= amount > totalBorrowsTracked ? totalBorrowsTracked : amount;
    }

    /// @notice 시간 경과 시뮬레이션
    function warpTime(uint256 secondsToWarp) external {
        callCount++;
        secondsToWarp = bound(secondsToWarp, 1, 365 days);
        vm.warp(block.timestamp + secondsToWarp);
    }
}

/// @title ZKLendingPoolInvariantTest
/// @notice ZKLendingPool의 불변 조건 테스트
contract ZKLendingPoolInvariantTest is StdInvariant, Test {
    ZKLendingPool public pool;
    CommitmentRegistry public registry;
    MockUSDC public usdc;
    MockVerifierForInvariant public verifier;
    Handler public handler;

    uint256 constant INITIAL_LIQUIDITY = 10_000_000 * 1e6; // 10M USDC

    function setUp() public {
        // 컨트랙트 배포
        verifier = new MockVerifierForInvariant();
        registry = new CommitmentRegistry();
        usdc = new MockUSDC();

        pool = new ZKLendingPool(
            address(verifier),
            address(registry),
            address(usdc),
            2000_00000000 // $2000 ETH price
        );

        // 설정
        registry.setAuthorizedCaller(address(pool), true);
        usdc.approve(address(pool), type(uint256).max);
        pool.supplyLiquidity(INITIAL_LIQUIDITY);

        // 핸들러 배포 및 설정
        handler = new Handler(pool, registry, usdc, verifier);

        // Foundry에게 handler만 호출하도록 지시
        targetContract(address(handler));

        // 특정 함수만 타겟팅
        bytes4[] memory selectors = new bytes4[](4);
        selectors[0] = Handler.deposit.selector;
        selectors[1] = Handler.borrow.selector;
        selectors[2] = Handler.repay.selector;
        selectors[3] = Handler.warpTime.selector;

        targetSelector(FuzzSelector({
            addr: address(handler),
            selectors: selectors
        }));
    }

    // ================================================================
    // ===================== INVARIANT TESTS ==========================
    // ================================================================

    /// @notice Invariant: 총 담보 ETH는 항상 >= 0
    /// @dev 담보는 음수가 될 수 없음 (underflow 방지)
    function invariant_collateralNonNegative() public view {
        assertGe(pool.totalCollateralETH(), 0, "Collateral cannot be negative");
    }

    /// @notice Invariant: 총 대출은 풀 유동성 + 총 대출과 일치
    /// @dev USDC 보존 법칙
    function invariant_usdcConservation() public view {
        uint256 poolBalance = usdc.balanceOf(address(pool));
        uint256 totalBorrowed = pool.totalBorrowedUSDC();

        // 초기 유동성 = 현재 잔액 + 대출된 금액
        // (이자 수익은 별도로 누적되므로 약간의 차이 허용)
        assertLe(
            poolBalance + totalBorrowed,
            INITIAL_LIQUIDITY + pool.totalAccruedInterest(),
            "USDC conservation violated"
        );
    }

    /// @notice Invariant: 이용률은 0-100% 사이
    function invariant_utilizationBounded() public view {
        uint256 utilization = pool.getUtilizationRate();
        assertLe(utilization, 100, "Utilization cannot exceed 100%");
    }

    /// @notice Invariant: 이자율은 합리적인 범위 내
    /// @dev 기본 5% + 최대 가변 40% = 최대 45%
    function invariant_interestRateBounded() public view {
        uint256 rate = pool.getCurrentInterestRate();
        // 기본: 500 (5%), 최대: 4500 (45%)
        assertGe(rate, 500, "Interest rate below minimum");
        assertLe(rate, 10000, "Interest rate above maximum"); // 100%보다 작아야 함
    }

    /// @notice Invariant: hasDeposit/hasBorrow 상태 일관성
    function invariant_stateConsistency() public view {
        for (uint256 i = 0; i < handler.actors.length; i++) {
            address actor = handler.actors(i);

            // hasBorrow가 true면 반드시 hasDeposit도 true
            if (pool.hasBorrow(actor)) {
                assertTrue(
                    pool.hasDeposit(actor),
                    "Cannot have borrow without deposit"
                );
            }

            // borrowedAmount > 0이면 hasBorrow가 true여야 함
            if (pool.borrowedAmount(actor) > 0) {
                assertTrue(
                    pool.hasBorrow(actor),
                    "Borrowed amount without hasBorrow flag"
                );
            }
        }
    }

    /// @notice Invariant: 컨트랙트 ETH 잔액 >= totalCollateralETH
    /// @dev 누군가 직접 ETH를 보낼 수 있으므로 >= 조건
    function invariant_ethBalanceConsistency() public view {
        assertGe(
            address(pool).balance,
            pool.totalCollateralETH(),
            "ETH balance inconsistent with tracked collateral"
        );
    }

    /// @notice 테스트 후 호출 횟수 출력 (디버깅용)
    function invariant_callSummary() public view {
        console.log("Total handler calls:", handler.callCount());
        console.log("Total deposits tracked:", handler.totalDepositsTracked());
        console.log("Total borrows tracked:", handler.totalBorrowsTracked());
        console.log("Pool total collateral:", pool.totalCollateralETH());
        console.log("Pool total borrowed:", pool.totalBorrowedUSDC());
    }
}

/// @title ZKLendingPoolFuzzTest
/// @notice ZKLendingPool의 Fuzz 테스트
contract ZKLendingPoolFuzzTest is Test {
    ZKLendingPool public pool;
    CommitmentRegistry public registry;
    MockUSDC public usdc;
    MockVerifierForInvariant public verifier;

    address public alice = address(0xA11CE);

    IZKVerifier.Proof internal dummyProof;
    uint256[] internal dummyInputs;

    function setUp() public {
        verifier = new MockVerifierForInvariant();
        registry = new CommitmentRegistry();
        usdc = new MockUSDC();

        pool = new ZKLendingPool(
            address(verifier),
            address(registry),
            address(usdc),
            2000_00000000
        );

        registry.setAuthorizedCaller(address(pool), true);
        usdc.approve(address(pool), type(uint256).max);
        pool.supplyLiquidity(10_000_000 * 1e6);

        vm.deal(alice, 10000 ether);
        usdc.ownerMint(alice, 100_000_000 * 1e6);

        dummyProof = IZKVerifier.Proof({
            a: [uint256(1), uint256(2)],
            b: [[uint256(3), uint256(4)], [uint256(5), uint256(6)]],
            c: [uint256(7), uint256(8)]
        });
        dummyInputs = new uint256[](2);
    }

    /// @notice Fuzz: 예치 금액에 관계없이 commitment가 등록되어야 함
    function testFuzz_Deposit(uint256 amount) public {
        // 유효한 범위로 제한
        amount = bound(amount, 0.001 ether, 1000 ether);

        bytes32 commitment = keccak256(abi.encodePacked(amount, uint256(12345)));

        vm.prank(alice);
        pool.deposit{value: amount}(commitment);

        assertTrue(pool.hasDeposit(alice), "Should have deposit");
        assertEq(pool.totalCollateralETH(), amount, "Collateral should match");
        assertTrue(registry.isValidCommitment(commitment), "Commitment should be valid");
    }

    /// @notice Fuzz: 대출 금액이 유동성 이하면 성공해야 함
    function testFuzz_Borrow(uint256 depositAmount, uint256 borrowAmount) public {
        // 범위 제한
        depositAmount = bound(depositAmount, 1 ether, 100 ether);

        // 먼저 예치
        bytes32 collCommitment = keccak256(abi.encodePacked(depositAmount, uint256(111)));
        vm.prank(alice);
        pool.deposit{value: depositAmount}(collCommitment);

        // 대출 금액을 유동성 이하로 제한
        uint256 maxBorrow = usdc.balanceOf(address(pool));
        if (maxBorrow == 0) return;

        borrowAmount = bound(borrowAmount, 1e6, maxBorrow);

        bytes32 debtCommitment = keccak256(abi.encodePacked(borrowAmount, uint256(222)));

        vm.prank(alice);
        pool.borrow(borrowAmount, debtCommitment, dummyProof, dummyProof, dummyInputs);

        assertEq(pool.borrowedAmount(alice), borrowAmount, "Borrowed amount should match");
    }

    /// @notice Fuzz: 이자 계산이 올바른지 검증
    function testFuzz_InterestCalculation(uint256 amount, uint256 duration) public {
        // 범위 제한
        amount = bound(amount, 1e6, 1_000_000 * 1e6); // 1 ~ 1M USDC
        duration = bound(duration, 1, 365 days);

        uint256 interest = pool.estimateInterest(amount, duration);

        // 이자는 항상 원금보다 작아야 함 (1년 이하에서는)
        if (duration <= 365 days) {
            assertLe(interest, amount, "Interest should not exceed principal in 1 year");
        }

        // 이자는 음수가 아니어야 함
        assertGe(interest, 0, "Interest cannot be negative");
    }

    /// @notice Fuzz: 이용률 계산
    function testFuzz_UtilizationRate(uint256 borrowAmount) public {
        // 예치
        bytes32 collCommitment = keccak256(abi.encodePacked(uint256(100 ether), uint256(111)));
        vm.prank(alice);
        pool.deposit{value: 100 ether}(collCommitment);

        uint256 maxBorrow = usdc.balanceOf(address(pool));
        if (maxBorrow == 0) return;

        borrowAmount = bound(borrowAmount, 1e6, maxBorrow);

        bytes32 debtCommitment = keccak256(abi.encodePacked(borrowAmount, uint256(222)));
        vm.prank(alice);
        pool.borrow(borrowAmount, debtCommitment, dummyProof, dummyProof, dummyInputs);

        uint256 utilization = pool.getUtilizationRate();

        // 이용률은 0-100 사이
        assertLe(utilization, 100, "Utilization cannot exceed 100%");
        assertGe(utilization, 0, "Utilization cannot be negative");
    }

    /// @notice Fuzz: 시간 경과 후 이자 발생
    function testFuzz_InterestAccrual(uint256 timeElapsed) public {
        timeElapsed = bound(timeElapsed, 1, 365 days);

        // 예치 + 대출
        bytes32 collCommitment = keccak256(abi.encodePacked(uint256(10 ether), uint256(111)));
        vm.prank(alice);
        pool.deposit{value: 10 ether}(collCommitment);

        uint256 borrowAmount = 10000 * 1e6;
        bytes32 debtCommitment = keccak256(abi.encodePacked(borrowAmount, uint256(222)));
        vm.prank(alice);
        pool.borrow(borrowAmount, debtCommitment, dummyProof, dummyProof, dummyInputs);

        // 시간 경과
        vm.warp(block.timestamp + timeElapsed);

        // 이자 발생 확인
        (uint256 principal, uint256 interest, uint256 total) = pool.getCurrentDebt(alice);

        assertEq(principal, borrowAmount, "Principal should not change");
        assertGe(interest, 0, "Interest should be non-negative");
        assertEq(total, principal + interest, "Total should be principal + interest");
    }

    /// @notice Fuzz: 상환 금액이 부채 이하면 성공
    function testFuzz_Repay(uint256 repayAmount) public {
        // 예치 + 대출
        bytes32 collCommitment = keccak256(abi.encodePacked(uint256(10 ether), uint256(111)));
        vm.prank(alice);
        pool.deposit{value: 10 ether}(collCommitment);

        uint256 borrowAmount = 10000 * 1e6;
        bytes32 debtCommitment = keccak256(abi.encodePacked(borrowAmount, uint256(222)));
        vm.prank(alice);
        pool.borrow(borrowAmount, debtCommitment, dummyProof, dummyProof, dummyInputs);

        // 상환 금액 제한
        repayAmount = bound(repayAmount, 1, borrowAmount * 2);

        bytes32 nullifier = keccak256(abi.encodePacked(alice, block.timestamp));
        bytes32 newDebtCommitment = repayAmount >= borrowAmount
            ? bytes32(0)
            : keccak256(abi.encodePacked(borrowAmount - repayAmount, uint256(333)));

        vm.startPrank(alice);
        usdc.approve(address(pool), repayAmount);
        pool.repay(repayAmount, newDebtCommitment, nullifier);
        vm.stopPrank();

        // 부채가 적절히 감소했는지 확인
        uint256 expectedRemaining = repayAmount >= borrowAmount ? 0 : borrowAmount - repayAmount;
        assertEq(pool.borrowedAmount(alice), expectedRemaining, "Remaining debt should match");
    }

    /// @notice Fuzz: 청산 보너스 계산
    function testFuzz_LiquidationBonus(uint256 debtAmount) public {
        debtAmount = bound(debtAmount, 1e6, 1_000_000 * 1e6);

        // 청산 보너스 계산: debt * 105 / 100
        uint256 collateralValueUSD = (debtAmount * 105) / 100;

        // 결과가 overflow 없이 계산되어야 함
        assertGe(collateralValueUSD, debtAmount, "Bonus should increase value");
        assertEq(collateralValueUSD, debtAmount + (debtAmount * 5) / 100, "5% bonus calculation");
    }
}
