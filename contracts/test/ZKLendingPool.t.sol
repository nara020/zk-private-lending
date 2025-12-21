// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Test, console} from "forge-std/Test.sol";
import {ZKLendingPool} from "../src/ZKLendingPool.sol";
import {ZKVerifier} from "../src/ZKVerifier.sol";
import {CommitmentRegistry} from "../src/CommitmentRegistry.sol";
import {MockUSDC} from "../src/MockUSDC.sol";
import {IZKVerifier} from "../src/interfaces/IZKVerifier.sol";

/// @title MockVerifier - 테스트용 ZK 검증기
/// @notice 테스트에서 ZK 검증 결과를 제어할 수 있는 모의 검증기
/// @dev
/// == Interview Q&A ==
/// Q: 왜 MockVerifier가 필요한가?
/// A: 실제 ZK proof 생성은 복잡하고 시간이 오래 걸림
///    - Groth16 proof 생성: ~2초 (Rust에서)
///    - Circuit 컴파일, setup 필요
///    - 테스트에서는 검증 로직만 테스트하면 됨
///
/// Q: 이 방식의 한계는?
/// A: 실제 ZK 검증이 제대로 작동하는지는 별도 검증 필요
///    - Integration test에서 실제 proof 사용
///    - E2E test에서 전체 플로우 검증
contract MockVerifier is IZKVerifier {
    // 증명 타입별 검증 결과 제어
    mapping(ProofType => bool) public shouldPass;

    // 기본값: 모든 증명 통과
    constructor() {
        shouldPass[ProofType.COLLATERAL] = true;
        shouldPass[ProofType.LTV] = true;
        shouldPass[ProofType.LIQUIDATION] = true;
    }

    /// @notice 특정 증명 타입의 검증 결과 설정
    function setVerificationResult(ProofType proofType, bool result) external {
        shouldPass[proofType] = result;
    }

    /// @notice 모든 증명 통과하도록 설정
    function passAll() external {
        shouldPass[ProofType.COLLATERAL] = true;
        shouldPass[ProofType.LTV] = true;
        shouldPass[ProofType.LIQUIDATION] = true;
    }

    /// @notice 모든 증명 실패하도록 설정
    function failAll() external {
        shouldPass[ProofType.COLLATERAL] = false;
        shouldPass[ProofType.LTV] = false;
        shouldPass[ProofType.LIQUIDATION] = false;
    }

    function verify(
        ProofType proofType,
        Proof calldata,
        uint256[] calldata
    ) external view override returns (bool) {
        return shouldPass[proofType];
    }

    function getVerificationKeyHash(ProofType) external pure override returns (bytes32) {
        return keccak256("mock_vk");
    }
}

/// @title ZKLendingPool 테스트
/// @notice 핵심 기능 테스트: 예치, 대출, 상환, 출금, 청산
/// @dev
/// == 테스트 구조 ==
/// 1. 기본 기능 테스트 (Deposit, Pool Status)
/// 2. 대출 플로우 테스트 (Borrow)
/// 3. 상환 플로우 테스트 (Repay)
/// 4. 출금 플로우 테스트 (Withdraw)
/// 5. 청산 플로우 테스트 (Liquidate)
/// 6. 엣지 케이스 및 보안 테스트
///
/// == Interview Q&A ==
/// Q: DeFi 프로토콜 테스트에서 중요한 점은?
/// A: 1. Edge cases: 경계값, 0값, 최대값
///    2. Reentrancy: 재진입 공격 방어
///    3. State consistency: 상태 일관성
///    4. Economic attacks: 가격 조작, 플래시론
///    5. Access control: 권한 검증
contract ZKLendingPoolTest is Test {
    // ============ 컨트랙트 ============
    ZKLendingPool public pool;
    MockVerifier public mockVerifier;
    CommitmentRegistry public registry;
    MockUSDC public usdc;

    // ============ 테스트 계정 ============
    address public owner = address(this);
    address public alice = address(0xA11CE);
    address public bob = address(0xB0B);
    address public liquidator = address(0x11001D);

    // ============ 상수 ============
    uint256 public constant INITIAL_ETH_PRICE = 2000_00000000; // $2000 (8 decimals)
    uint256 public constant DEPOSIT_AMOUNT = 10 ether;
    uint256 public constant BORROW_AMOUNT = 10000 * 1e6; // 10,000 USDC
    uint256 public constant POOL_LIQUIDITY = 1_000_000 * 1e6; // 1M USDC

    // ============ 테스트용 더미 데이터 ============
    IZKVerifier.Proof internal dummyProof;
    uint256[] internal dummyPublicInputs;

    // ============ 설정 ============

    function setUp() public {
        // 컨트랙트 배포 (MockVerifier 사용)
        mockVerifier = new MockVerifier();
        registry = new CommitmentRegistry();
        usdc = new MockUSDC();

        pool = new ZKLendingPool(
            address(mockVerifier),
            address(registry),
            address(usdc),
            INITIAL_ETH_PRICE
        );

        // 권한 설정
        registry.setAuthorizedCaller(address(pool), true);

        // 풀에 USDC 유동성 공급
        usdc.approve(address(pool), type(uint256).max);
        pool.supplyLiquidity(POOL_LIQUIDITY);

        // 테스트 계정에 ETH/USDC 지급
        vm.deal(alice, 100 ether);
        vm.deal(bob, 100 ether);
        vm.deal(liquidator, 100 ether);
        usdc.ownerMint(alice, 100000 * 1e6);
        usdc.ownerMint(bob, 100000 * 1e6);
        usdc.ownerMint(liquidator, 100000 * 1e6);

        // 더미 proof 설정 (MockVerifier는 내용 무시)
        dummyProof = IZKVerifier.Proof({
            a: [uint256(1), uint256(2)],
            b: [[uint256(3), uint256(4)], [uint256(5), uint256(6)]],
            c: [uint256(7), uint256(8)]
        });
        dummyPublicInputs = new uint256[](2);
        dummyPublicInputs[0] = 100;
        dummyPublicInputs[1] = 200;
    }

    // ============ Helper Functions ============

    /// @notice Alice가 담보 예치하는 헬퍼 함수
    function _depositAsAlice() internal returns (bytes32 commitment) {
        commitment = keccak256(abi.encodePacked(DEPOSIT_AMOUNT, uint256(12345)));
        vm.prank(alice);
        pool.deposit{value: DEPOSIT_AMOUNT}(commitment);
    }

    /// @notice Alice가 대출받는 헬퍼 함수
    function _borrowAsAlice(uint256 amount) internal returns (bytes32 debtCommitment) {
        debtCommitment = keccak256(abi.encodePacked(amount, uint256(67890)));
        vm.prank(alice);
        pool.borrow(
            amount,
            debtCommitment,
            dummyProof,
            dummyProof,
            dummyPublicInputs
        );
    }

    // ====================================================================
    // ======================== DEPOSIT TESTS =============================
    // ====================================================================

    function test_Deposit() public {
        bytes32 commitment = keccak256(abi.encodePacked(DEPOSIT_AMOUNT, uint256(12345)));

        vm.prank(alice);
        pool.deposit{value: DEPOSIT_AMOUNT}(commitment);

        assertTrue(pool.hasDeposit(alice), "Should have deposit");
        assertEq(pool.totalCollateralETH(), DEPOSIT_AMOUNT, "Total collateral should match");
    }

    function test_Deposit_RevertZeroAmount() public {
        bytes32 commitment = keccak256(abi.encodePacked(uint256(0), uint256(12345)));

        vm.prank(alice);
        vm.expectRevert(ZKLendingPool.ZeroAmount.selector);
        pool.deposit{value: 0}(commitment);
    }

    function test_Deposit_RevertAlreadyHasDeposit() public {
        bytes32 commitment1 = keccak256(abi.encodePacked(DEPOSIT_AMOUNT, uint256(12345)));
        bytes32 commitment2 = keccak256(abi.encodePacked(DEPOSIT_AMOUNT, uint256(67890)));

        vm.startPrank(alice);
        pool.deposit{value: DEPOSIT_AMOUNT}(commitment1);

        vm.expectRevert(ZKLendingPool.AlreadyHasDeposit.selector);
        pool.deposit{value: DEPOSIT_AMOUNT}(commitment2);
        vm.stopPrank();
    }

    function test_Deposit_RevertInvalidCommitment() public {
        vm.prank(alice);
        vm.expectRevert(ZKLendingPool.InvalidCommitment.selector);
        pool.deposit{value: DEPOSIT_AMOUNT}(bytes32(0));
    }

    // ============ Commitment 테스트 ============

    function test_CommitmentRegistered() public {
        bytes32 commitment = keccak256(abi.encodePacked(DEPOSIT_AMOUNT, uint256(12345)));

        vm.prank(alice);
        pool.deposit{value: DEPOSIT_AMOUNT}(commitment);

        assertTrue(registry.isValidCommitment(commitment), "Commitment should be valid");

        (bytes32 collComm, ) = registry.getUserCommitments(alice);
        assertEq(collComm, commitment, "User commitment should match");
    }

    // ====================================================================
    // ========================= BORROW TESTS =============================
    // ====================================================================

    /// @notice 정상적인 대출 플로우 테스트
    /// @dev
    /// Interview Q&A:
    /// Q: 대출 시 검증되는 항목들은?
    /// A: 1. CollateralProof - 담보가 충분한지
    ///    2. LTVProof - 부채/담보 비율이 MAX_LTV 이하인지
    ///    실제 담보 금액은 ZK proof로만 검증, 공개되지 않음!
    function test_Borrow_Success() public {
        // 1. 담보 예치
        _depositAsAlice();

        // 2. 대출
        bytes32 debtCommitment = _borrowAsAlice(BORROW_AMOUNT);

        // 3. 상태 확인
        assertTrue(pool.hasBorrow(alice), "Should have borrow");
        assertEq(pool.borrowedAmount(alice), BORROW_AMOUNT, "Borrowed amount should match");
        assertEq(usdc.balanceOf(alice), 100000 * 1e6 + BORROW_AMOUNT, "USDC balance should increase");
        assertEq(pool.totalBorrowedUSDC(), BORROW_AMOUNT, "Total borrowed should match");

        // 4. Commitment 확인
        (, bytes32 debtComm) = registry.getUserCommitments(alice);
        assertEq(debtComm, debtCommitment, "Debt commitment should be registered");
    }

    /// @notice 예치 없이 대출 시도 - 실패해야 함
    function test_Borrow_RevertNoDeposit() public {
        bytes32 debtCommitment = keccak256(abi.encodePacked(BORROW_AMOUNT, uint256(67890)));

        vm.prank(alice);
        vm.expectRevert(ZKLendingPool.NoDeposit.selector);
        pool.borrow(
            BORROW_AMOUNT,
            debtCommitment,
            dummyProof,
            dummyProof,
            dummyPublicInputs
        );
    }

    /// @notice 0 금액 대출 시도 - 실패해야 함
    function test_Borrow_RevertZeroAmount() public {
        _depositAsAlice();

        bytes32 debtCommitment = keccak256(abi.encodePacked(uint256(0), uint256(67890)));

        vm.prank(alice);
        vm.expectRevert(ZKLendingPool.ZeroAmount.selector);
        pool.borrow(
            0,
            debtCommitment,
            dummyProof,
            dummyProof,
            dummyPublicInputs
        );
    }

    /// @notice 풀 유동성 부족 시 대출 - 실패해야 함
    function test_Borrow_RevertInsufficientLiquidity() public {
        _depositAsAlice();

        // 풀 유동성보다 많은 금액 대출 시도
        uint256 tooMuch = POOL_LIQUIDITY + 1;
        bytes32 debtCommitment = keccak256(abi.encodePacked(tooMuch, uint256(67890)));

        vm.prank(alice);
        vm.expectRevert(ZKLendingPool.InsufficientPoolLiquidity.selector);
        pool.borrow(
            tooMuch,
            debtCommitment,
            dummyProof,
            dummyProof,
            dummyPublicInputs
        );
    }

    /// @notice CollateralProof 실패 시 대출 - 실패해야 함
    /// @dev
    /// Interview Q&A:
    /// Q: CollateralProof가 실패하면?
    /// A: InvalidProof 에러 발생
    ///    실제 상황: 담보가 threshold보다 적을 때
    function test_Borrow_RevertInvalidCollateralProof() public {
        _depositAsAlice();

        // CollateralProof 실패하도록 설정
        mockVerifier.setVerificationResult(IZKVerifier.ProofType.COLLATERAL, false);

        bytes32 debtCommitment = keccak256(abi.encodePacked(BORROW_AMOUNT, uint256(67890)));

        vm.prank(alice);
        vm.expectRevert(ZKLendingPool.InvalidProof.selector);
        pool.borrow(
            BORROW_AMOUNT,
            debtCommitment,
            dummyProof,
            dummyProof,
            dummyPublicInputs
        );
    }

    /// @notice LTVProof 실패 시 대출 - 실패해야 함
    /// @dev
    /// Interview Q&A:
    /// Q: LTV(Loan-to-Value)란?
    /// A: 담보 대비 부채 비율
    ///    LTV = debt / collateral_value
    ///    MAX_LTV = 75%면, $100 담보로 최대 $75까지 대출 가능
    function test_Borrow_RevertInvalidLTVProof() public {
        _depositAsAlice();

        // LTVProof 실패하도록 설정 (LTV 초과)
        mockVerifier.setVerificationResult(IZKVerifier.ProofType.LTV, false);

        bytes32 debtCommitment = keccak256(abi.encodePacked(BORROW_AMOUNT, uint256(67890)));

        vm.prank(alice);
        vm.expectRevert(ZKLendingPool.ExceedsMaxLTV.selector);
        pool.borrow(
            BORROW_AMOUNT,
            debtCommitment,
            dummyProof,
            dummyProof,
            dummyPublicInputs
        );
    }

    /// @notice 여러 번 대출 테스트 (추가 대출)
    function test_Borrow_MultipleBorrows() public {
        _depositAsAlice();

        // 첫 번째 대출
        bytes32 debtCommitment1 = keccak256(abi.encodePacked(BORROW_AMOUNT, uint256(111)));
        vm.prank(alice);
        pool.borrow(
            BORROW_AMOUNT,
            debtCommitment1,
            dummyProof,
            dummyProof,
            dummyPublicInputs
        );

        // 두 번째 대출 (추가 대출)
        bytes32 debtCommitment2 = keccak256(abi.encodePacked(BORROW_AMOUNT, uint256(222)));
        vm.prank(alice);
        pool.borrow(
            BORROW_AMOUNT,
            debtCommitment2,
            dummyProof,
            dummyProof,
            dummyPublicInputs
        );

        // 총 대출 금액 확인
        assertEq(pool.borrowedAmount(alice), BORROW_AMOUNT * 2, "Should have total borrowed");
        assertEq(pool.totalBorrowedUSDC(), BORROW_AMOUNT * 2, "Pool total should match");
    }

    // ====================================================================
    // ========================== REPAY TESTS =============================
    // ====================================================================

    /// @notice 전액 상환 테스트
    /// @dev
    /// Interview Q&A:
    /// Q: 상환 시 commitment는 어떻게 처리?
    /// A: nullify - 사용된 commitment를 무효화
    ///    이중 사용 방지 (double spending prevention)
    function test_Repay_Full() public {
        // Setup: 예치 + 대출
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        uint256 aliceBalanceBefore = usdc.balanceOf(alice);

        // USDC approve
        vm.startPrank(alice);
        usdc.approve(address(pool), BORROW_AMOUNT);

        // 전액 상환
        bytes32 nullifier = keccak256(abi.encodePacked(alice, block.timestamp, "repay"));
        pool.repay(BORROW_AMOUNT, bytes32(0), nullifier);
        vm.stopPrank();

        // 상태 확인
        assertFalse(pool.hasBorrow(alice), "Should not have borrow after full repay");
        assertEq(pool.borrowedAmount(alice), 0, "Borrowed amount should be 0");
        assertEq(pool.totalBorrowedUSDC(), 0, "Total borrowed should be 0");
        assertEq(usdc.balanceOf(alice), aliceBalanceBefore - BORROW_AMOUNT, "USDC should decrease");
    }

    /// @notice 부분 상환 테스트
    function test_Repay_Partial() public {
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        uint256 repayAmount = BORROW_AMOUNT / 2;
        bytes32 newDebtCommitment = keccak256(abi.encodePacked(BORROW_AMOUNT - repayAmount, uint256(99999)));

        vm.startPrank(alice);
        usdc.approve(address(pool), repayAmount);

        bytes32 nullifier = keccak256(abi.encodePacked(alice, block.timestamp, "partial_repay"));
        pool.repay(repayAmount, newDebtCommitment, nullifier);
        vm.stopPrank();

        // 상태 확인
        assertTrue(pool.hasBorrow(alice), "Should still have borrow");
        assertEq(pool.borrowedAmount(alice), BORROW_AMOUNT - repayAmount, "Remaining debt should match");
    }

    /// @notice 대출 없이 상환 시도 - 실패해야 함
    function test_Repay_RevertNoBorrow() public {
        _depositAsAlice();
        // 대출 안 받음

        vm.startPrank(alice);
        usdc.approve(address(pool), BORROW_AMOUNT);

        bytes32 nullifier = keccak256(abi.encodePacked(alice, block.timestamp));
        vm.expectRevert(ZKLendingPool.NoBorrow.selector);
        pool.repay(BORROW_AMOUNT, bytes32(0), nullifier);
        vm.stopPrank();
    }

    /// @notice 0 금액 상환 - 실패해야 함
    function test_Repay_RevertZeroAmount() public {
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        vm.startPrank(alice);
        bytes32 nullifier = keccak256(abi.encodePacked(alice, block.timestamp));
        vm.expectRevert(ZKLendingPool.ZeroAmount.selector);
        pool.repay(0, bytes32(0), nullifier);
        vm.stopPrank();
    }

    /// @notice 부채보다 많이 상환해도 부채만큼만 처리
    function test_Repay_ExcessAmount() public {
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        uint256 excessAmount = BORROW_AMOUNT * 2;

        vm.startPrank(alice);
        usdc.approve(address(pool), excessAmount);

        bytes32 nullifier = keccak256(abi.encodePacked(alice, block.timestamp));
        pool.repay(excessAmount, bytes32(0), nullifier);
        vm.stopPrank();

        // 부채만큼만 상환됨
        assertFalse(pool.hasBorrow(alice), "Should not have borrow");
        assertEq(pool.borrowedAmount(alice), 0, "Borrowed amount should be 0");
    }

    // ====================================================================
    // ======================== WITHDRAW TESTS ============================
    // ====================================================================

    /// @notice 부채 없이 출금 테스트
    function test_Withdraw_NoBorrow() public {
        _depositAsAlice();

        uint256 aliceBalanceBefore = alice.balance;
        bytes32 nullifier = keccak256(abi.encodePacked(alice, block.timestamp, "withdraw"));

        vm.prank(alice);
        pool.withdraw(DEPOSIT_AMOUNT, nullifier, dummyProof, dummyPublicInputs);

        // 상태 확인
        assertFalse(pool.hasDeposit(alice), "Should not have deposit after withdraw");
        assertEq(pool.totalCollateralETH(), 0, "Total collateral should be 0");
        assertEq(alice.balance, aliceBalanceBefore + DEPOSIT_AMOUNT, "ETH should be returned");
    }

    /// @notice 부채 있을 때 출금 - LTV 유지 필요
    function test_Withdraw_WithBorrow() public {
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        // 부분 출금 시도 (LTV proof 필요)
        uint256 withdrawAmount = 1 ether;
        bytes32 nullifier = keccak256(abi.encodePacked(alice, block.timestamp, "partial_withdraw"));

        vm.prank(alice);
        pool.withdraw(withdrawAmount, nullifier, dummyProof, dummyPublicInputs);

        // 출금 성공 (MockVerifier가 통과시킴)
        assertEq(pool.totalCollateralETH(), DEPOSIT_AMOUNT - withdrawAmount, "Collateral should decrease");
    }

    /// @notice 부채 있을 때 출금 - LTV 초과 시 실패
    function test_Withdraw_RevertExceedsLTV() public {
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        // LTV proof 실패하도록 설정
        mockVerifier.setVerificationResult(IZKVerifier.ProofType.LTV, false);

        bytes32 nullifier = keccak256(abi.encodePacked(alice, block.timestamp, "withdraw"));

        vm.prank(alice);
        vm.expectRevert(ZKLendingPool.ExceedsMaxLTV.selector);
        pool.withdraw(DEPOSIT_AMOUNT, nullifier, dummyProof, dummyPublicInputs);
    }

    /// @notice 예치 없이 출금 - 실패해야 함
    function test_Withdraw_RevertNoDeposit() public {
        bytes32 nullifier = keccak256(abi.encodePacked(alice, block.timestamp));

        vm.prank(alice);
        vm.expectRevert(ZKLendingPool.NoDeposit.selector);
        pool.withdraw(1 ether, nullifier, dummyProof, dummyPublicInputs);
    }

    /// @notice 0 금액 출금 - 실패해야 함
    function test_Withdraw_RevertZeroAmount() public {
        _depositAsAlice();

        bytes32 nullifier = keccak256(abi.encodePacked(alice, block.timestamp));

        vm.prank(alice);
        vm.expectRevert(ZKLendingPool.ZeroAmount.selector);
        pool.withdraw(0, nullifier, dummyProof, dummyPublicInputs);
    }

    // ====================================================================
    // ======================= LIQUIDATE TESTS ============================
    // ====================================================================

    /// @notice 정상 청산 테스트
    /// @dev
    /// Interview Q&A:
    /// Q: 청산은 언제 발생하는가?
    /// A: Health Factor < 1.0 일 때
    ///    health = (collateral * price * liq_threshold) / (debt * 100)
    ///    가격 하락 또는 부채 증가 시 청산 가능 상태가 됨
    ///
    /// Q: 청산자의 인센티브는?
    /// A: LIQUIDATION_BONUS (5%)
    ///    부채를 대신 갚고, 담보를 5% 보너스 받아 획득
    function test_Liquidate_Success() public {
        // Setup: Alice가 예치 + 대출
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        uint256 liquidatorBalanceBefore = liquidator.balance;
        uint256 debtToRepay = pool.borrowedAmount(alice);

        // Liquidator가 USDC approve
        vm.startPrank(liquidator);
        usdc.approve(address(pool), debtToRepay);

        // 청산 실행
        pool.liquidate(alice, dummyProof, dummyPublicInputs);
        vm.stopPrank();

        // 상태 확인
        assertFalse(pool.hasBorrow(alice), "Alice should not have borrow after liquidation");
        assertEq(pool.borrowedAmount(alice), 0, "Alice borrowed amount should be 0");

        // 청산자가 담보 + 보너스 받았는지 확인
        uint256 expectedCollateralUSD = (debtToRepay * 105) / 100; // 5% bonus
        uint256 expectedCollateralETH = (expectedCollateralUSD * 1e20) / INITIAL_ETH_PRICE;
        assertTrue(liquidator.balance > liquidatorBalanceBefore, "Liquidator should receive ETH");
    }

    /// @notice 대출 없는 사용자 청산 - 실패해야 함
    function test_Liquidate_RevertNoBorrow() public {
        _depositAsAlice();
        // Alice는 대출 안 받음

        vm.startPrank(liquidator);
        usdc.approve(address(pool), BORROW_AMOUNT);

        vm.expectRevert(ZKLendingPool.NoBorrow.selector);
        pool.liquidate(alice, dummyProof, dummyPublicInputs);
        vm.stopPrank();
    }

    /// @notice 청산 불가능한 포지션 청산 시도 - 실패해야 함
    /// @dev
    /// Interview Q&A:
    /// Q: ZK 청산 증명이 필요한 이유는?
    /// A: MEV 방어!
    ///    - 청산 시점이 공개되면 MEV 봇이 선행매매
    ///    - ZK proof로 "청산 가능"만 증명 → 정확한 포지션 예측 불가
    function test_Liquidate_RevertNotLiquidatable() public {
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        // LiquidationProof 실패하도록 설정 (건강한 포지션)
        mockVerifier.setVerificationResult(IZKVerifier.ProofType.LIQUIDATION, false);

        vm.startPrank(liquidator);
        usdc.approve(address(pool), BORROW_AMOUNT);

        vm.expectRevert(ZKLendingPool.NotLiquidatable.selector);
        pool.liquidate(alice, dummyProof, dummyPublicInputs);
        vm.stopPrank();
    }

    /// @notice 청산 보너스 계산 테스트
    function test_Liquidate_BonusCalculation() public {
        // Setup: Bob이 예치 + 대출
        bytes32 bobCommitment = keccak256(abi.encodePacked(DEPOSIT_AMOUNT, uint256(11111)));
        vm.prank(bob);
        pool.deposit{value: DEPOSIT_AMOUNT}(bobCommitment);

        bytes32 bobDebtCommitment = keccak256(abi.encodePacked(BORROW_AMOUNT, uint256(22222)));
        vm.prank(bob);
        pool.borrow(
            BORROW_AMOUNT,
            bobDebtCommitment,
            dummyProof,
            dummyProof,
            dummyPublicInputs
        );

        uint256 poolCollateralBefore = pool.totalCollateralETH();
        uint256 debtToRepay = pool.borrowedAmount(bob);

        // 청산 실행
        vm.startPrank(liquidator);
        usdc.approve(address(pool), debtToRepay);
        pool.liquidate(bob, dummyProof, dummyPublicInputs);
        vm.stopPrank();

        // 청산으로 감소한 담보 = 부채 * 105% (USD 기준) / ETH 가격
        uint256 collateralSeizedUSD = (debtToRepay * 105) / 100;
        uint256 expectedSeizedETH = (collateralSeizedUSD * 1e20) / INITIAL_ETH_PRICE;

        // 풀의 총 담보가 적절히 감소했는지 확인
        assertEq(
            pool.totalCollateralETH(),
            poolCollateralBefore - expectedSeizedETH,
            "Pool collateral should decrease by seized amount"
        );
    }

    /// @notice 여러 청산자 경쟁 시나리오
    function test_Liquidate_FirstComeFirstServe() public {
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        address liquidator2 = address(0x2222);
        vm.deal(liquidator2, 100 ether);
        usdc.ownerMint(liquidator2, 100000 * 1e6);

        // 첫 번째 청산자가 먼저 청산
        vm.startPrank(liquidator);
        usdc.approve(address(pool), BORROW_AMOUNT);
        pool.liquidate(alice, dummyProof, dummyPublicInputs);
        vm.stopPrank();

        // 두 번째 청산자는 실패 (이미 청산됨)
        vm.startPrank(liquidator2);
        usdc.approve(address(pool), BORROW_AMOUNT);
        vm.expectRevert(ZKLendingPool.NoBorrow.selector);
        pool.liquidate(alice, dummyProof, dummyPublicInputs);
        vm.stopPrank();
    }

    // ====================================================================
    // ====================== INTEREST TESTS ==============================
    // ====================================================================

    /// @notice 이자율 상수 확인 테스트
    function test_InterestRateConstants() public view {
        assertEq(pool.INTEREST_RATE_BASE(), 10000, "Interest rate base should be 10000");
        assertEq(pool.BASE_INTEREST_RATE(), 500, "Base rate should be 5%");
        assertEq(pool.VARIABLE_INTEREST_RATE(), 2000, "Variable rate should be 20%");
        assertEq(pool.OPTIMAL_UTILIZATION(), 80, "Optimal utilization should be 80%");
    }

    /// @notice 이용률 계산 테스트
    function test_UtilizationRate() public {
        // 대출 없을 때: 0%
        assertEq(pool.getUtilizationRate(), 0, "Utilization should be 0 with no borrows");

        // 대출 후 이용률 계산
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        uint256 utilization = pool.getUtilizationRate();
        assertGt(utilization, 0, "Utilization should be > 0 after borrow");
    }

    /// @notice 이자율 계산 테스트
    function test_CurrentInterestRate() public view {
        // 대출 없을 때 기본 이자율
        uint256 rate = pool.getCurrentInterestRate();
        assertEq(rate, 500, "Base interest rate should be 5%");
    }

    /// @notice 이자 예상 계산 테스트
    function test_EstimateInterest() public view {
        uint256 amount = 10000 * 1e6; // $10,000
        uint256 oneYear = 365 days;

        uint256 interest = pool.estimateInterest(amount, oneYear);
        // 5% APR: $10,000 * 0.05 = $500
        assertApproxEqRel(interest, 500 * 1e6, 0.1e18, "Interest should be ~$500");
    }

    /// @notice 시간 경과 후 이자 발생 테스트
    function test_InterestAccrual() public {
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        // 초기 부채 확인
        (uint256 principal, uint256 interestBefore, ) = pool.getCurrentDebt(alice);
        assertEq(principal, BORROW_AMOUNT, "Principal should match borrow amount");
        assertEq(interestBefore, 0, "Initial interest should be 0");

        // 30일 경과
        vm.warp(block.timestamp + 30 days);

        // 이자 발생 확인
        (, uint256 interestAfter, uint256 total) = pool.getCurrentDebt(alice);
        assertGt(interestAfter, 0, "Interest should accrue over time");
        assertEq(total, principal + interestAfter, "Total should be principal + interest");
    }

    /// @notice 이자 먼저 상환 테스트
    function test_InterestPaidFirst() public {
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        // 30일 경과
        vm.warp(block.timestamp + 30 days);

        (uint256 principal, uint256 interest, ) = pool.getCurrentDebt(alice);
        require(interest > 0, "Interest should have accrued");

        // 이자만큼만 상환
        vm.startPrank(alice);
        usdc.approve(address(pool), interest);
        bytes32 newDebtCommitment = keccak256(abi.encodePacked(principal, uint256(77777)));
        pool.repay(interest, newDebtCommitment, keccak256("interest_repay"));
        vm.stopPrank();

        // 원금은 그대로
        assertEq(pool.borrowedAmount(alice), principal, "Principal should remain unchanged");
    }

    /// @notice payInterest 함수 테스트
    function test_PayInterest() public {
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        // 30일 경과
        vm.warp(block.timestamp + 30 days);

        (, uint256 interestBefore, ) = pool.getCurrentDebt(alice);
        require(interestBefore > 0, "Interest should have accrued");

        // payInterest 호출
        vm.startPrank(alice);
        usdc.approve(address(pool), interestBefore * 2); // 충분한 양 승인
        pool.payInterest();
        vm.stopPrank();

        // 이자 청산 확인
        (, uint256 interestAfter, ) = pool.getCurrentDebt(alice);
        assertEq(interestAfter, 0, "Interest should be cleared after payment");
    }

    /// @notice APY 조회 테스트
    function test_GetAPY() public view {
        uint256 apy = pool.getAPY();
        assertGe(apy, 500, "APY should be at least base rate");
    }

    // ====================================================================
    // ====================== POOL STATUS TESTS ===========================
    // ====================================================================

    function test_GetPoolStatus() public {
        _depositAsAlice();

        (
            uint256 totalColl,
            uint256 totalBorrow,
            uint256 available,
            uint256 utilization,
            uint256 interestRate,
            uint256 totalInterest
        ) = pool.getPoolStatus();

        assertEq(totalColl, DEPOSIT_AMOUNT, "Total collateral should match");
        assertEq(totalBorrow, 0, "Total borrow should be 0");
        assertEq(available, POOL_LIQUIDITY, "Available liquidity should match");
        assertEq(utilization, 0, "Utilization should be 0 with no borrows");
        assertGe(interestRate, 500, "Interest rate should be at least base rate");
        assertEq(totalInterest, 0, "Total interest should be 0");
    }

    function test_GetPoolStatus_AfterBorrow() public {
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        (
            uint256 totalColl,
            uint256 totalBorrow,
            uint256 available,
            uint256 utilization,
            uint256 interestRate,
            uint256 totalInterest
        ) = pool.getPoolStatus();

        assertEq(totalColl, DEPOSIT_AMOUNT, "Total collateral should match");
        assertEq(totalBorrow, BORROW_AMOUNT, "Total borrow should match");
        assertEq(available, POOL_LIQUIDITY - BORROW_AMOUNT, "Available should decrease");
        assertGt(utilization, 0, "Utilization should be > 0");
        assertGe(interestRate, 500, "Interest rate should be at least base rate");
    }

    // ====================================================================
    // ====================== PRICE UPDATE TESTS ==========================
    // ====================================================================

    function test_UpdatePrice() public {
        uint256 newPrice = 2500_00000000; // $2500
        pool.updatePrice(newPrice);
        assertEq(pool.ethPrice(), newPrice, "Price should be updated");
    }

    function test_UpdatePrice_RevertNotOwner() public {
        vm.prank(alice);
        vm.expectRevert();
        pool.updatePrice(2500_00000000);
    }

    // ====================================================================
    // ==================== USER POSITION TESTS ===========================
    // ====================================================================

    function test_GetUserPosition() public {
        bytes32 commitment = _depositAsAlice();

        (
            bool _hasDeposit,
            bool _hasBorrow,
            uint256 _borrowedAmount,
            bytes32 collComm,
            bytes32 debtComm
        ) = pool.getUserPosition(alice);

        assertTrue(_hasDeposit, "Should have deposit");
        assertFalse(_hasBorrow, "Should not have borrow");
        assertEq(_borrowedAmount, 0, "Borrowed amount should be 0");
        assertEq(collComm, commitment, "Collateral commitment should match");
        assertEq(debtComm, bytes32(0), "Debt commitment should be empty");
    }

    function test_GetUserPosition_AfterBorrow() public {
        _depositAsAlice();
        bytes32 debtCommitment = _borrowAsAlice(BORROW_AMOUNT);

        (
            bool _hasDeposit,
            bool _hasBorrow,
            uint256 _borrowedAmount,
            ,
            bytes32 debtComm
        ) = pool.getUserPosition(alice);

        assertTrue(_hasDeposit, "Should have deposit");
        assertTrue(_hasBorrow, "Should have borrow");
        assertEq(_borrowedAmount, BORROW_AMOUNT, "Borrowed amount should match");
        assertEq(debtComm, debtCommitment, "Debt commitment should match");
    }

    // ====================================================================
    // ==================== MULTIPLE USERS TESTS ==========================
    // ====================================================================

    function test_MultipleDeposits() public {
        bytes32 aliceCommitment = keccak256(abi.encodePacked(DEPOSIT_AMOUNT, uint256(111)));
        bytes32 bobCommitment = keccak256(abi.encodePacked(DEPOSIT_AMOUNT * 2, uint256(222)));

        vm.prank(alice);
        pool.deposit{value: DEPOSIT_AMOUNT}(aliceCommitment);

        vm.prank(bob);
        pool.deposit{value: DEPOSIT_AMOUNT * 2}(bobCommitment);

        assertEq(pool.totalCollateralETH(), DEPOSIT_AMOUNT * 3, "Total should be sum");
        assertTrue(pool.hasDeposit(alice), "Alice should have deposit");
        assertTrue(pool.hasDeposit(bob), "Bob should have deposit");
    }

    function test_MultipleUsersBorrow() public {
        // Alice 예치 + 대출
        _depositAsAlice();
        _borrowAsAlice(BORROW_AMOUNT);

        // Bob 예치 + 대출
        bytes32 bobCommitment = keccak256(abi.encodePacked(DEPOSIT_AMOUNT * 2, uint256(333)));
        vm.prank(bob);
        pool.deposit{value: DEPOSIT_AMOUNT * 2}(bobCommitment);

        bytes32 bobDebtCommitment = keccak256(abi.encodePacked(BORROW_AMOUNT * 2, uint256(444)));
        vm.prank(bob);
        pool.borrow(
            BORROW_AMOUNT * 2,
            bobDebtCommitment,
            dummyProof,
            dummyProof,
            dummyPublicInputs
        );

        // 풀 상태 확인
        assertEq(pool.totalBorrowedUSDC(), BORROW_AMOUNT * 3, "Total borrowed should be sum");
        assertEq(pool.borrowedAmount(alice), BORROW_AMOUNT, "Alice borrowed should match");
        assertEq(pool.borrowedAmount(bob), BORROW_AMOUNT * 2, "Bob borrowed should match");
    }

    // ====================================================================
    // ====================== PRIVACY TESTS ===============================
    // ====================================================================

    /// @notice 핵심: 동일한 금액도 다른 salt면 다른 commitment
    /// @dev
    /// Interview Q&A:
    /// Q: Commitment 방식의 프라이버시는?
    /// A: commitment = Hash(amount, salt)
    ///    - salt가 비밀이므로 amount 추론 불가
    ///    - 같은 금액이라도 salt가 다르면 다른 commitment
    ///    - 단방향 해시로 역산 불가
    function test_PrivacyProperty() public {
        uint256 amount = 10 ether;
        uint256 salt1 = 12345;
        uint256 salt2 = 67890;

        // 같은 금액, 다른 salt → 다른 commitment
        bytes32 comm1 = keccak256(abi.encodePacked(amount, salt1));
        bytes32 comm2 = keccak256(abi.encodePacked(amount, salt2));

        assertTrue(comm1 != comm2, "Different salts should produce different commitments");

        // commitment만 봐서는 금액을 알 수 없음 (단방향 해시)
        // 이것이 ZK Private Lending의 핵심!
    }

    /// @notice 다른 금액도 commitment만 봐서는 구분 불가
    function test_PrivacyProperty_DifferentAmounts() public {
        uint256 amount1 = 10 ether;
        uint256 amount2 = 100 ether;
        uint256 salt = 12345;

        bytes32 comm1 = keccak256(abi.encodePacked(amount1, salt));
        bytes32 comm2 = keccak256(abi.encodePacked(amount2, salt));

        // commitment 자체는 32바이트 해시로, 금액 정보를 드러내지 않음
        // (물론 같은 salt를 쓰면 안 됨 - 실제로는 각각 다른 salt 사용)
        assertTrue(comm1 != comm2, "Different amounts should produce different commitments");
    }

    // ====================================================================
    // ====================== EDGE CASES ==================================
    // ====================================================================

    function test_ReceiveETH() public {
        // 풀이 직접 ETH를 받을 수 있는지 확인
        vm.prank(alice);
        (bool success, ) = address(pool).call{value: 1 ether}("");
        assertTrue(success, "Pool should receive ETH");
    }

    /// @notice 풀 유동성 정확히 소진
    function test_Borrow_ExactLiquidity() public {
        _depositAsAlice();

        // 정확히 풀 유동성만큼 대출
        bytes32 debtCommitment = keccak256(abi.encodePacked(POOL_LIQUIDITY, uint256(99999)));
        vm.prank(alice);
        pool.borrow(
            POOL_LIQUIDITY,
            debtCommitment,
            dummyProof,
            dummyProof,
            dummyPublicInputs
        );

        assertEq(pool.totalBorrowedUSDC(), POOL_LIQUIDITY, "Should borrow exact liquidity");

        (, , uint256 available, , , ) = pool.getPoolStatus();
        assertEq(available, 0, "Available should be 0");
    }

    /// @notice 전체 플로우 통합 테스트
    /// @dev 예치 → 대출 → 부분상환 → 전액상환 → 출금
    function test_FullFlow_Integration() public {
        // 1. 예치
        bytes32 collCommitment = _depositAsAlice();
        assertTrue(pool.hasDeposit(alice), "Step 1: Should have deposit");

        // 2. 대출
        bytes32 debtCommitment = _borrowAsAlice(BORROW_AMOUNT);
        assertTrue(pool.hasBorrow(alice), "Step 2: Should have borrow");

        // 3. 부분 상환
        uint256 partialRepay = BORROW_AMOUNT / 2;
        bytes32 newDebtCommitment = keccak256(abi.encodePacked(BORROW_AMOUNT - partialRepay, uint256(11111)));

        vm.startPrank(alice);
        usdc.approve(address(pool), partialRepay);
        pool.repay(partialRepay, newDebtCommitment, keccak256("partial"));
        vm.stopPrank();

        assertEq(pool.borrowedAmount(alice), BORROW_AMOUNT - partialRepay, "Step 3: Partial repay");

        // 4. 전액 상환
        vm.startPrank(alice);
        usdc.approve(address(pool), BORROW_AMOUNT - partialRepay);
        pool.repay(BORROW_AMOUNT - partialRepay, bytes32(0), keccak256("full"));
        vm.stopPrank();

        assertFalse(pool.hasBorrow(alice), "Step 4: Should not have borrow");

        // 5. 출금
        vm.prank(alice);
        pool.withdraw(DEPOSIT_AMOUNT, keccak256("withdraw"), dummyProof, dummyPublicInputs);
        assertFalse(pool.hasDeposit(alice), "Step 5: Should not have deposit");
    }
}

/// @title MockVerifier 단위 테스트
contract MockVerifierTest is Test {
    MockVerifier public mockVerifier;
    IZKVerifier.Proof internal dummyProof;
    uint256[] internal dummyInputs;

    function setUp() public {
        mockVerifier = new MockVerifier();
        dummyProof = IZKVerifier.Proof({
            a: [uint256(1), uint256(2)],
            b: [[uint256(3), uint256(4)], [uint256(5), uint256(6)]],
            c: [uint256(7), uint256(8)]
        });
        dummyInputs = new uint256[](1);
    }

    function test_MockVerifier_DefaultPass() public view {
        assertTrue(
            mockVerifier.verify(IZKVerifier.ProofType.COLLATERAL, dummyProof, dummyInputs),
            "Should pass by default"
        );
        assertTrue(
            mockVerifier.verify(IZKVerifier.ProofType.LTV, dummyProof, dummyInputs),
            "Should pass by default"
        );
        assertTrue(
            mockVerifier.verify(IZKVerifier.ProofType.LIQUIDATION, dummyProof, dummyInputs),
            "Should pass by default"
        );
    }

    function test_MockVerifier_SetResult() public {
        mockVerifier.setVerificationResult(IZKVerifier.ProofType.COLLATERAL, false);

        assertFalse(
            mockVerifier.verify(IZKVerifier.ProofType.COLLATERAL, dummyProof, dummyInputs),
            "Should fail after setting"
        );
        assertTrue(
            mockVerifier.verify(IZKVerifier.ProofType.LTV, dummyProof, dummyInputs),
            "LTV should still pass"
        );
    }

    function test_MockVerifier_FailAll() public {
        mockVerifier.failAll();

        assertFalse(mockVerifier.verify(IZKVerifier.ProofType.COLLATERAL, dummyProof, dummyInputs));
        assertFalse(mockVerifier.verify(IZKVerifier.ProofType.LTV, dummyProof, dummyInputs));
        assertFalse(mockVerifier.verify(IZKVerifier.ProofType.LIQUIDATION, dummyProof, dummyInputs));
    }

    function test_MockVerifier_PassAll() public {
        mockVerifier.failAll();
        mockVerifier.passAll();

        assertTrue(mockVerifier.verify(IZKVerifier.ProofType.COLLATERAL, dummyProof, dummyInputs));
        assertTrue(mockVerifier.verify(IZKVerifier.ProofType.LTV, dummyProof, dummyInputs));
        assertTrue(mockVerifier.verify(IZKVerifier.ProofType.LIQUIDATION, dummyProof, dummyInputs));
    }
}
