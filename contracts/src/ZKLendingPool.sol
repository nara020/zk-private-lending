// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import {ReentrancyGuard} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

import {IZKVerifier} from "./interfaces/IZKVerifier.sol";
import {ICommitmentRegistry} from "./interfaces/ICommitmentRegistry.sol";

/// @title ZKLendingPool
/// @notice ZK 프라이버시 보호 렌딩 풀 - 담보 금액을 숨기고 대출
/// @dev
/// ╔══════════════════════════════════════════════════════════════════╗
/// ║                    ZK Private Lending Flow                       ║
/// ╠══════════════════════════════════════════════════════════════════╣
/// ║                                                                  ║
/// ║  1. DEPOSIT (예치)                                               ║
/// ║     User → ETH + commitment → Pool                               ║
/// ║     - 실제 ETH는 컨트랙트가 보관                                  ║
/// ║     - commitment = Hash(amount, salt) 만 공개                    ║
/// ║                                                                  ║
/// ║  2. BORROW (대출)                                                ║
/// ║     User → ZK Proof ("담보 충분!") → Pool → USDC                 ║
/// ║     - CollateralProof: 담보 >= threshold 증명                    ║
/// ║     - LTVProof: debt/collateral <= maxLTV 증명                   ║
/// ║     - 실제 담보 금액은 절대 공개 안 됨!                           ║
/// ║                                                                  ║
/// ║  3. REPAY (상환)                                                 ║
/// ║     User → USDC → Pool → Update commitment                       ║
/// ║     - 부채 상환하면 debt commitment 업데이트                      ║
/// ║                                                                  ║
/// ║  4. WITHDRAW (출금)                                              ║
/// ║     User → ZK Proof + nullifier → ETH                            ║
/// ║     - 부채 없으면 담보 회수 가능                                  ║
/// ║                                                                  ║
/// ║  5. LIQUIDATE (청산)                                             ║
/// ║     Liquidator → LiquidationProof → Liquidation                  ║
/// ║     - health_factor < 1 증명하면 청산 가능                        ║
/// ║                                                                  ║
/// ╚══════════════════════════════════════════════════════════════════╝
///
contract ZKLendingPool is ReentrancyGuard, Ownable {
    using SafeERC20 for IERC20;

    // ============ 상수 ============

    /// @notice 최대 LTV (Loan-to-Value) - 75%
    uint256 public constant MAX_LTV = 75;

    /// @notice 청산 임계값 - 80%
    uint256 public constant LIQUIDATION_THRESHOLD = 80;

    /// @notice 청산 보너스 - 5%
    uint256 public constant LIQUIDATION_BONUS = 5;

    /// @notice 기준 단위 (퍼센트 계산용)
    uint256 public constant PERCENTAGE_BASE = 100;

    // ============ 상태 변수 ============

    /// @notice ZK 검증기 컨트랙트
    IZKVerifier public immutable zkVerifier;

    /// @notice 커밋먼트 저장소 컨트랙트
    ICommitmentRegistry public immutable commitmentRegistry;

    /// @notice 대출 토큰 (USDC)
    IERC20 public immutable borrowToken;

    /// @notice ETH/USD 가격 (8 decimals, 예: 2000_00000000 = $2000)
    uint256 public ethPrice;

    /// @notice 풀에 예치된 총 ETH (프라이버시 보호를 위해 개별 금액은 숨김)
    uint256 public totalCollateralETH;

    /// @notice 총 대출 USDC
    uint256 public totalBorrowedUSDC;

    /// @notice 사용자별 예치 여부 (금액은 숨김)
    mapping(address => bool) public hasDeposit;

    /// @notice 사용자별 대출 여부
    mapping(address => bool) public hasBorrow;

    /// @notice 사용자별 대출 금액 (이건 숨기기 어려움 - USDC 전송이 공개되므로)
    /// @dev 향후 개선: 대출 금액도 commitment로 숨기기
    mapping(address => uint256) public borrowedAmount;

    // ============ 이벤트 ============

    event Deposited(address indexed user, bytes32 commitment, uint256 timestamp);
    event Borrowed(address indexed user, uint256 amount, bytes32 debtCommitment, uint256 timestamp);
    event Repaid(address indexed user, uint256 amount, uint256 timestamp);
    event Withdrawn(address indexed user, bytes32 nullifier, uint256 timestamp);
    event Liquidated(
        address indexed user,
        address indexed liquidator,
        uint256 debtRepaid,
        uint256 collateralSeized,
        uint256 timestamp
    );
    event PriceUpdated(uint256 oldPrice, uint256 newPrice);

    // ============ 에러 ============

    error InvalidProof();
    error InsufficientCollateral();
    error ExceedsMaxLTV();
    error NoDeposit();
    error NoBorrow();
    error AlreadyHasDeposit();
    error NotLiquidatable();
    error InsufficientPoolLiquidity();
    error ZeroAmount();
    error InvalidCommitment();
    error ZeroAddress();

    // ============ 생성자 ============

    /// @dev Zero address 체크 포함
    constructor(
        address _zkVerifier,
        address _commitmentRegistry,
        address _borrowToken,
        uint256 _initialEthPrice
    ) Ownable(msg.sender) {
        // Security: Zero address validation
        if (_zkVerifier == address(0)) revert ZeroAddress();
        if (_commitmentRegistry == address(0)) revert ZeroAddress();
        if (_borrowToken == address(0)) revert ZeroAddress();
        if (_initialEthPrice == 0) revert ZeroAmount();

        zkVerifier = IZKVerifier(_zkVerifier);
        commitmentRegistry = ICommitmentRegistry(_commitmentRegistry);
        borrowToken = IERC20(_borrowToken);
        ethPrice = _initialEthPrice;
    }

    // ============ 관리 함수 ============

    /// @notice ETH 가격 업데이트 (실제로는 Chainlink Oracle 사용)
    /// @param newPrice 새 가격 (8 decimals)
    function updatePrice(uint256 newPrice) external onlyOwner {
        uint256 oldPrice = ethPrice;
        ethPrice = newPrice;
        emit PriceUpdated(oldPrice, newPrice);
    }

    /// @notice 풀에 USDC 유동성 공급 (테스트용)
    function supplyLiquidity(uint256 amount) external onlyOwner {
        borrowToken.safeTransferFrom(msg.sender, address(this), amount);
    }

    // ============ 핵심 함수 ============

    /// @notice ETH 담보 예치
    /// @param commitment Poseidon(amount, salt) 해시값
    /// @dev 실제 금액은 commitment에 숨겨짐
    function deposit(bytes32 commitment) external payable nonReentrant {
        if (msg.value == 0) revert ZeroAmount();
        if (hasDeposit[msg.sender]) revert AlreadyHasDeposit();
        if (commitment == bytes32(0)) revert InvalidCommitment();

        // 커밋먼트 등록 (실제 금액은 숨김)
        // Security: msg.sender를 명시적으로 전달 (tx.origin 사용 안 함)
        commitmentRegistry.registerCommitment(
            commitment,
            ICommitmentRegistry.CommitmentType.COLLATERAL,
            msg.sender
        );

        hasDeposit[msg.sender] = true;
        totalCollateralETH += msg.value;

        emit Deposited(msg.sender, commitment, block.timestamp);
    }

    /// @notice USDC 대출
    /// @param amount 대출할 USDC 양 (6 decimals)
    /// @param debtCommitment 부채 commitment
    /// @param collateralProof 담보 충분 증명
    /// @param ltvProof LTV 비율 증명
    /// @param publicInputs ZK 증명의 public inputs
    function borrow(
        uint256 amount,
        bytes32 debtCommitment,
        IZKVerifier.Proof calldata collateralProof,
        IZKVerifier.Proof calldata ltvProof,
        uint256[] calldata publicInputs
    ) external nonReentrant {
        if (amount == 0) revert ZeroAmount();
        if (!hasDeposit[msg.sender]) revert NoDeposit();
        if (borrowToken.balanceOf(address(this)) < amount) revert InsufficientPoolLiquidity();

        // 1. CollateralProof 검증: 담보 >= 필요 담보량
        bool collateralValid = zkVerifier.verify(
            IZKVerifier.ProofType.COLLATERAL,
            collateralProof,
            publicInputs
        );
        if (!collateralValid) revert InvalidProof();

        // 2. LTVProof 검증: debt/collateral <= MAX_LTV
        bool ltvValid = zkVerifier.verify(IZKVerifier.ProofType.LTV, ltvProof, publicInputs);
        if (!ltvValid) revert ExceedsMaxLTV();

        // 3. 부채 commitment 등록
        // Security: msg.sender를 명시적으로 전달 (tx.origin 사용 안 함)
        commitmentRegistry.registerCommitment(
            debtCommitment,
            ICommitmentRegistry.CommitmentType.DEBT,
            msg.sender
        );

        // 4. 상태 업데이트
        hasBorrow[msg.sender] = true;
        borrowedAmount[msg.sender] += amount;
        totalBorrowedUSDC += amount;

        // 5. USDC 전송
        borrowToken.safeTransfer(msg.sender, amount);

        emit Borrowed(msg.sender, amount, debtCommitment, block.timestamp);
    }

    /// @notice USDC 상환
    /// @param amount 상환할 양
    /// @param newDebtCommitment 업데이트된 부채 commitment (0이면 전액 상환)
    /// @param nullifier 기존 commitment 무효화용
    function repay(
        uint256 amount,
        bytes32 newDebtCommitment,
        bytes32 nullifier
    ) external nonReentrant {
        if (amount == 0) revert ZeroAmount();
        if (!hasBorrow[msg.sender]) revert NoBorrow();

        uint256 currentDebt = borrowedAmount[msg.sender];
        uint256 repayAmount = amount > currentDebt ? currentDebt : amount;

        // USDC 받기
        borrowToken.safeTransferFrom(msg.sender, address(this), repayAmount);

        // 상태 업데이트
        borrowedAmount[msg.sender] -= repayAmount;
        totalBorrowedUSDC -= repayAmount;

        // commitment 업데이트
        (bytes32 collateralComm, bytes32 debtComm) = commitmentRegistry.getUserCommitments(
            msg.sender
        );

        if (borrowedAmount[msg.sender] == 0) {
            // 전액 상환: commitment 삭제
            hasBorrow[msg.sender] = false;
            commitmentRegistry.nullifyCommitment(debtComm, nullifier);
        } else {
            // 부분 상환: commitment 업데이트
            commitmentRegistry.updateCommitment(debtComm, newDebtCommitment, nullifier);
        }

        emit Repaid(msg.sender, repayAmount, block.timestamp);
    }

    /// @notice 담보 출금
    /// @param amount 출금할 ETH 양
    /// @param nullifier commitment 무효화용
    /// @param withdrawProof 출금 가능 증명 (부채 없음 또는 충분한 담보 유지)
    /// @param publicInputs 증명의 public inputs
    function withdraw(
        uint256 amount,
        bytes32 nullifier,
        IZKVerifier.Proof calldata withdrawProof,
        uint256[] calldata publicInputs
    ) external nonReentrant {
        if (amount == 0) revert ZeroAmount();
        if (!hasDeposit[msg.sender]) revert NoDeposit();

        // 부채가 있으면 LTV 체크 필요
        if (hasBorrow[msg.sender]) {
            // LTV proof 검증 (출금 후에도 LTV 유지되는지)
            bool valid = zkVerifier.verify(
                IZKVerifier.ProofType.LTV,
                withdrawProof,
                publicInputs
            );
            if (!valid) revert ExceedsMaxLTV();
        }

        // commitment 처리
        (bytes32 collateralComm, ) = commitmentRegistry.getUserCommitments(msg.sender);

        // 전액 출금이면 commitment 삭제, 아니면 업데이트
        // (단순화를 위해 전액 출금만 지원 - 부채 없을 때)
        if (!hasBorrow[msg.sender]) {
            commitmentRegistry.nullifyCommitment(collateralComm, nullifier);
            hasDeposit[msg.sender] = false;
        }

        totalCollateralETH -= amount;

        // ETH 전송
        (bool success, ) = msg.sender.call{value: amount}("");
        require(success, "ETH transfer failed");

        emit Withdrawn(msg.sender, nullifier, block.timestamp);
    }

    /// @notice 청산
    /// @param user 청산 대상 사용자
    /// @param liquidationProof 청산 가능 증명 (health_factor < 1)
    /// @param publicInputs 증명의 public inputs
    function liquidate(
        address user,
        IZKVerifier.Proof calldata liquidationProof,
        uint256[] calldata publicInputs
    ) external nonReentrant {
        if (!hasBorrow[user]) revert NoBorrow();

        // LiquidationProof 검증: health_factor < 1
        bool valid = zkVerifier.verify(
            IZKVerifier.ProofType.LIQUIDATION,
            liquidationProof,
            publicInputs
        );
        if (!valid) revert NotLiquidatable();

        uint256 debtToRepay = borrowedAmount[user];

        // 청산자가 부채 대신 갚기
        borrowToken.safeTransferFrom(msg.sender, address(this), debtToRepay);

        // 담보 계산 (USD 가치 기준 + 청산 보너스)
        // collateralValue = debtToRepay * (100 + LIQUIDATION_BONUS) / 100
        uint256 collateralValueUSD = (debtToRepay * (PERCENTAGE_BASE + LIQUIDATION_BONUS)) /
            PERCENTAGE_BASE;

        // ETH로 변환 (가격은 8 decimals, USDC는 6 decimals)
        // collateralETH = collateralValueUSD * 1e18 / ethPrice * 1e6 / 1e8
        uint256 collateralETH = (collateralValueUSD * 1e20) / ethPrice;

        // 상태 업데이트
        borrowedAmount[user] = 0;
        totalBorrowedUSDC -= debtToRepay;
        hasBorrow[user] = false;

        // commitment 정리
        (, bytes32 debtComm) = commitmentRegistry.getUserCommitments(user);
        bytes32 nullifier = keccak256(abi.encodePacked(user, block.timestamp, "liquidation"));
        commitmentRegistry.nullifyCommitment(debtComm, nullifier);

        // 담보에서 차감
        if (collateralETH > totalCollateralETH) {
            collateralETH = totalCollateralETH;
        }
        totalCollateralETH -= collateralETH;

        // 청산자에게 담보 전송
        (bool success, ) = msg.sender.call{value: collateralETH}("");
        require(success, "ETH transfer failed");

        emit Liquidated(user, msg.sender, debtToRepay, collateralETH, block.timestamp);
    }

    // ============ 조회 함수 ============

    /// @notice 사용자 포지션 조회
    function getUserPosition(
        address user
    )
        external
        view
        returns (
            bool _hasDeposit,
            bool _hasBorrow,
            uint256 _borrowedAmount,
            bytes32 collateralCommitment,
            bytes32 debtCommitment
        )
    {
        (collateralCommitment, debtCommitment) = commitmentRegistry.getUserCommitments(user);
        return (hasDeposit[user], hasBorrow[user], borrowedAmount[user], collateralCommitment, debtCommitment);
    }

    /// @notice 풀 상태 조회
    function getPoolStatus()
        external
        view
        returns (uint256 _totalCollateralETH, uint256 _totalBorrowedUSDC, uint256 availableLiquidity)
    {
        return (totalCollateralETH, totalBorrowedUSDC, borrowToken.balanceOf(address(this)));
    }

    /// @notice ETH 수신 가능
    receive() external payable {}
}
