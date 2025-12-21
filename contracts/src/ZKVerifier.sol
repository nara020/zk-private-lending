// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IZKVerifier} from "./interfaces/IZKVerifier.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

/// @title ZKVerifier
/// @notice Groth16 증명 검증기 - BN254 곡선 사용
/// @dev EVM 프리컴파일 활용: ecAdd(0x06), ecMul(0x07), ecPairing(0x08)
///
/// == Groth16 검증 공식 ==
/// e(A, B) = e(α, β) · e(L, γ) · e(C, δ)
///
/// 여기서:
/// - A, B, C: proof (증명자가 제출)
/// - α, β, γ, δ: verification key (신뢰 설정에서 생성)
/// - L: public input들의 선형 조합
///
/// == BN254 곡선 파라미터 ==
/// - 소수 필드: p = 21888242871839275222246405745257275088696311157297823662689037894645226208583
/// - 그룹 차수: r = 21888242871839275222246405745257275088548364400416034343698204186575808495617
///
contract ZKVerifier is IZKVerifier, Ownable {
    // ============ 상수 ============

    /// @notice BN254 소수 필드 모듈러스
    uint256 internal constant PRIME_Q =
        21888242871839275222246405745257275088696311157297823662689037894645226208583;

    /// @notice 스칼라 필드 모듈러스 (그룹 차수)
    uint256 internal constant SCALAR_FIELD =
        21888242871839275222246405745257275088548364400416034343698204186575808495617;

    // ============ 상태 변수 ============

    /// @notice 각 증명 타입별 Verification Key
    /// @dev vk[proofType] = [alpha, beta, gamma, delta, ic[0], ic[1], ...]
    mapping(ProofType => uint256[]) public verificationKeys;

    /// @notice VK 해시 (무결성 검증용)
    mapping(ProofType => bytes32) public vkHashes;

    /// @notice VK 설정 여부
    mapping(ProofType => bool) public isVKSet;

    // ============ 이벤트 ============

    event VerificationKeySet(ProofType indexed proofType, bytes32 vkHash);
    event ProofVerified(ProofType indexed proofType, address indexed verifier, bool success);

    // ============ 에러 ============

    error InvalidProof();
    error InvalidPublicInputs();
    error VKNotSet();
    error InvalidVKLength();
    error PointNotOnCurve();
    error PairingFailed();

    // ============ 생성자 ============

    constructor() Ownable(msg.sender) {}

    // ============ VK 설정 (Owner Only) ============

    /// @notice Verification Key 설정
    /// @dev VK 구조: [alpha.x, alpha.y, beta.x1, beta.x2, beta.y1, beta.y2,
    ///               gamma.x1, gamma.x2, gamma.y1, gamma.y2,
    ///               delta.x1, delta.x2, delta.y1, delta.y2,
    ///               ic[0].x, ic[0].y, ic[1].x, ic[1].y, ...]
    /// @param proofType 증명 타입
    /// @param vk Verification Key 배열
    function setVerificationKey(ProofType proofType, uint256[] calldata vk) external onlyOwner {
        // VK 최소 길이: alpha(2) + beta(4) + gamma(4) + delta(4) + ic[0](2) + ic[1](2) = 18
        if (vk.length < 18) revert InvalidVKLength();

        // IC 포인트 개수 = (전체 길이 - 14) / 2
        // public input 개수 = IC 개수 - 1
        uint256 icCount = (vk.length - 14) / 2;
        if (vk.length != 14 + icCount * 2) revert InvalidVKLength();

        verificationKeys[proofType] = vk;
        vkHashes[proofType] = keccak256(abi.encodePacked(vk));
        isVKSet[proofType] = true;

        emit VerificationKeySet(proofType, vkHashes[proofType]);
    }

    // ============ 증명 검증 ============

    /// @notice ZK 증명 검증
    /// @param proofType 증명 타입
    /// @param proof Groth16 증명 (A, B, C 포인트)
    /// @param publicInputs 공개 입력값들
    /// @return 검증 성공 여부
    function verify(
        ProofType proofType,
        Proof calldata proof,
        uint256[] calldata publicInputs
    ) external view override returns (bool) {
        if (!isVKSet[proofType]) revert VKNotSet();

        uint256[] storage vk = verificationKeys[proofType];
        uint256 icCount = (vk.length - 14) / 2;

        // public input 개수 검증
        if (publicInputs.length != icCount - 1) revert InvalidPublicInputs();

        // public input 범위 검증
        for (uint256 i = 0; i < publicInputs.length; i++) {
            if (publicInputs[i] >= SCALAR_FIELD) revert InvalidPublicInputs();
        }

        // 1. L = ic[0] + Σ(publicInput[i] * ic[i+1]) 계산
        uint256[2] memory L = _computeLinearCombination(vk, publicInputs, icCount);

        // 2. Pairing 검증: e(-A, B) * e(α, β) * e(L, γ) * e(C, δ) == 1
        bool success = _verifyPairing(proof, vk, L);

        // Note: emit removed for view compatibility
        // Event emission happens in LendingPool for transparency

        return success;
    }

    /// @notice VK 해시 조회
    function getVerificationKeyHash(ProofType proofType) external view override returns (bytes32) {
        return vkHashes[proofType];
    }

    // ============ 내부 함수 ============

    /// @notice Linear combination 계산: L = ic[0] + Σ(input[i] * ic[i+1])
    function _computeLinearCombination(
        uint256[] storage vk,
        uint256[] calldata inputs,
        uint256 icCount
    ) internal view returns (uint256[2] memory) {
        // L = ic[0]
        uint256[2] memory L;
        L[0] = vk[14]; // ic[0].x
        L[1] = vk[15]; // ic[0].y

        // L += Σ(input[i] * ic[i+1])
        for (uint256 i = 0; i < inputs.length; i++) {
            uint256[2] memory icPoint;
            icPoint[0] = vk[16 + i * 2]; // ic[i+1].x
            icPoint[1] = vk[17 + i * 2]; // ic[i+1].y

            // scalar multiplication: input[i] * ic[i+1]
            uint256[2] memory product = _ecMul(icPoint, inputs[i]);

            // point addition: L += product
            L = _ecAdd(L, product);
        }

        return L;
    }

    /// @notice Pairing 검증
    /// @dev e(-A, B) * e(α, β) * e(L, γ) * e(C, δ) == 1
    function _verifyPairing(
        Proof calldata proof,
        uint256[] storage vk,
        uint256[2] memory L
    ) internal view returns (bool) {
        // 입력 배열 구성 (4개의 pairing)
        uint256[24] memory input;

        // Pairing 1: e(-A, B)
        // -A = (A.x, PRIME_Q - A.y)
        input[0] = proof.a[0];
        input[1] = PRIME_Q - (proof.a[1] % PRIME_Q);
        input[2] = proof.b[0][0];
        input[3] = proof.b[0][1];
        input[4] = proof.b[1][0];
        input[5] = proof.b[1][1];

        // Pairing 2: e(α, β)
        input[6] = vk[0]; // alpha.x
        input[7] = vk[1]; // alpha.y
        input[8] = vk[2]; // beta.x1
        input[9] = vk[3]; // beta.x2
        input[10] = vk[4]; // beta.y1
        input[11] = vk[5]; // beta.y2

        // Pairing 3: e(L, γ)
        input[12] = L[0];
        input[13] = L[1];
        input[14] = vk[6]; // gamma.x1
        input[15] = vk[7]; // gamma.x2
        input[16] = vk[8]; // gamma.y1
        input[17] = vk[9]; // gamma.y2

        // Pairing 4: e(C, δ)
        input[18] = proof.c[0];
        input[19] = proof.c[1];
        input[20] = vk[10]; // delta.x1
        input[21] = vk[11]; // delta.x2
        input[22] = vk[12]; // delta.y1
        input[23] = vk[13]; // delta.y2

        // Pairing precompile 호출 (0x08)
        uint256[1] memory result;
        bool success;

        assembly {
            // staticcall(gas, address, input_offset, input_size, output_offset, output_size)
            success := staticcall(sub(gas(), 2000), 0x08, input, 768, result, 32)
        }

        if (!success) revert PairingFailed();

        return result[0] == 1;
    }

    /// @notice EC Point Addition (precompile 0x06)
    function _ecAdd(
        uint256[2] memory p1,
        uint256[2] memory p2
    ) internal view returns (uint256[2] memory result) {
        uint256[4] memory input;
        input[0] = p1[0];
        input[1] = p1[1];
        input[2] = p2[0];
        input[3] = p2[1];

        bool success;
        assembly {
            success := staticcall(sub(gas(), 2000), 0x06, input, 128, result, 64)
        }
        require(success, "ecAdd failed");
    }

    /// @notice EC Scalar Multiplication (precompile 0x07)
    function _ecMul(
        uint256[2] memory p,
        uint256 s
    ) internal view returns (uint256[2] memory result) {
        uint256[3] memory input;
        input[0] = p[0];
        input[1] = p[1];
        input[2] = s;

        bool success;
        assembly {
            success := staticcall(sub(gas(), 2000), 0x07, input, 96, result, 64)
        }
        require(success, "ecMul failed");
    }
}
