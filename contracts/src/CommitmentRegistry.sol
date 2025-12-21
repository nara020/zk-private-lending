// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {ICommitmentRegistry} from "./interfaces/ICommitmentRegistry.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

/// @title CommitmentRegistry
/// @notice Pedersen 커밋먼트 저장소 - 실제 금액을 숨기고 해시만 저장
/// @dev
/// == Pedersen Commitment란? ==
///
/// commitment = Hash(value, salt)
///
/// 예시:
///   실제 담보: 10 ETH
///   salt: 랜덤값 (사용자만 앎)
///   commitment: 0x7a8b... (해시값)
///
/// 특성:
///   - Hiding: commitment만 봐서는 10 ETH인지 알 수 없음
///   - Binding: 나중에 다른 값이라고 주장 불가능
///
/// == 사용 흐름 ==
///
/// 1. 사용자가 ETH 예치 시:
///    - 프론트엔드에서 salt 생성
///    - commitment = Poseidon(amount, salt) 계산
///    - commitment를 온체인에 저장
///
/// 2. 대출 요청 시:
///    - ZK Proof로 "commitment에 해당하는 담보가 충분함" 증명
///    - 실제 amount와 salt는 proof 생성에만 사용 (공개 안 함)
///
contract CommitmentRegistry is ICommitmentRegistry, Ownable {
    // ============ 상태 변수 ============

    /// @notice 사용자별 담보 커밋먼트
    mapping(address => bytes32) public collateralCommitments;

    /// @notice 사용자별 부채 커밋먼트
    mapping(address => bytes32) public debtCommitments;

    /// @notice 커밋먼트 존재 여부
    mapping(bytes32 => bool) public commitmentExists;

    /// @notice 커밋먼트 소유자
    mapping(bytes32 => address) public commitmentOwner;

    /// @notice 커밋먼트 타입
    mapping(bytes32 => CommitmentType) public commitmentTypes;

    /// @notice 사용된 nullifier (이중 사용 방지)
    mapping(bytes32 => bool) public nullifiers;

    /// @notice 권한 있는 컨트랙트 (LendingPool)
    mapping(address => bool) public authorizedCallers;

    // ============ 에러 ============

    error UnauthorizedCaller();
    error CommitmentAlreadyExists();
    error CommitmentDoesNotExist();
    error CommitmentAlreadyNullified();
    error NullifierAlreadyUsed();
    error ZeroCommitment();
    error NotCommitmentOwner();

    // ============ 수정자 ============

    modifier onlyAuthorized() {
        if (!authorizedCallers[msg.sender] && msg.sender != owner()) {
            revert UnauthorizedCaller();
        }
        _;
    }

    // ============ 생성자 ============

    constructor() Ownable(msg.sender) {}

    // ============ 관리 함수 ============

    /// @notice 권한 있는 caller 추가 (LendingPool 등)
    function setAuthorizedCaller(address caller, bool authorized) external onlyOwner {
        authorizedCallers[caller] = authorized;
    }

    // ============ 커밋먼트 등록 ============

    /// @notice 새 커밋먼트 등록
    /// @dev LendingPool에서 예치/대출 시 호출
    /// @param commitment Poseidon(amount, salt) 해시값
    /// @param commitmentType COLLATERAL 또는 DEBT
    /// @param user 커밋먼트 소유자 주소 (LendingPool에서 msg.sender 전달)
    ///
    /// Security Note:
    /// - tx.origin 대신 user 파라미터 사용 (보안 강화)
    /// - LendingPool이 msg.sender를 전달하므로 안전
    function registerCommitment(
        bytes32 commitment,
        CommitmentType commitmentType,
        address user
    ) external override onlyAuthorized {
        if (commitment == bytes32(0)) revert ZeroCommitment();
        if (commitmentExists[commitment]) revert CommitmentAlreadyExists();
        if (user == address(0)) revert ZeroCommitment(); // Zero address check

        commitmentExists[commitment] = true;
        commitmentOwner[commitment] = user;
        commitmentTypes[commitment] = commitmentType;

        if (commitmentType == CommitmentType.COLLATERAL) {
            collateralCommitments[user] = commitment;
        } else {
            debtCommitments[user] = commitment;
        }

        emit CommitmentRegistered(user, commitment, commitmentType, block.timestamp);
    }

    /// @notice 커밋먼트 업데이트 (예: 추가 예치, 부분 상환)
    /// @param oldCommitment 기존 커밋먼트
    /// @param newCommitment 새 커밋먼트
    /// @param nullifier 기존 커밋먼트 무효화용
    function updateCommitment(
        bytes32 oldCommitment,
        bytes32 newCommitment,
        bytes32 nullifier
    ) external onlyAuthorized {
        if (!commitmentExists[oldCommitment]) revert CommitmentDoesNotExist();
        if (nullifiers[nullifier]) revert NullifierAlreadyUsed();
        if (newCommitment == bytes32(0)) revert ZeroCommitment();

        address user = commitmentOwner[oldCommitment];
        CommitmentType cType = commitmentTypes[oldCommitment];

        // 기존 커밋먼트 무효화
        nullifiers[nullifier] = true;
        delete commitmentExists[oldCommitment];

        // 새 커밋먼트 등록
        commitmentExists[newCommitment] = true;
        commitmentOwner[newCommitment] = user;
        commitmentTypes[newCommitment] = cType;

        if (cType == CommitmentType.COLLATERAL) {
            collateralCommitments[user] = newCommitment;
        } else {
            debtCommitments[user] = newCommitment;
        }

        emit CommitmentNullified(user, oldCommitment, nullifier, block.timestamp);
        emit CommitmentRegistered(user, newCommitment, cType, block.timestamp);
    }

    // ============ 커밋먼트 무효화 ============

    /// @notice 커밋먼트 무효화 (출금, 상환 완료 시)
    function nullifyCommitment(
        bytes32 commitment,
        bytes32 nullifier
    ) external override onlyAuthorized {
        if (!commitmentExists[commitment]) revert CommitmentDoesNotExist();
        if (nullifiers[nullifier]) revert NullifierAlreadyUsed();

        address user = commitmentOwner[commitment];
        CommitmentType cType = commitmentTypes[commitment];

        nullifiers[nullifier] = true;
        delete commitmentExists[commitment];
        delete commitmentOwner[commitment];

        if (cType == CommitmentType.COLLATERAL) {
            delete collateralCommitments[user];
        } else {
            delete debtCommitments[user];
        }

        emit CommitmentNullified(user, commitment, nullifier, block.timestamp);
    }

    // ============ 조회 함수 ============

    /// @notice 커밋먼트 유효성 확인
    function isValidCommitment(bytes32 commitment) external view override returns (bool) {
        return commitmentExists[commitment];
    }

    /// @notice nullifier 사용 여부 확인
    function isNullifierUsed(bytes32 nullifier) external view override returns (bool) {
        return nullifiers[nullifier];
    }

    /// @notice 사용자의 커밋먼트 조회
    function getUserCommitments(
        address user
    ) external view override returns (bytes32 collateralCommitment, bytes32 debtCommitment) {
        return (collateralCommitments[user], debtCommitments[user]);
    }

    /// @notice 커밋먼트 소유자 확인
    function getCommitmentOwner(bytes32 commitment) external view returns (address) {
        return commitmentOwner[commitment];
    }

    /// @notice 커밋먼트 타입 확인
    function getCommitmentType(bytes32 commitment) external view returns (CommitmentType) {
        if (!commitmentExists[commitment]) revert CommitmentDoesNotExist();
        return commitmentTypes[commitment];
    }
}
