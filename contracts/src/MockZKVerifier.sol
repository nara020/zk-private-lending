// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {IZKVerifier} from "./interfaces/IZKVerifier.sol";

/// @title MockZKVerifier
/// @notice Mock ZK verifier for testing - ALWAYS returns true
/// @dev DO NOT use in production! This is only for development/testing
contract MockZKVerifier is IZKVerifier {
    /// @notice Mock VK hashes
    mapping(ProofType => bytes32) public mockVKHashes;

    /// @notice Track verification calls for testing
    uint256 public verificationCount;

    /// @notice Last verified proof type
    ProofType public lastProofType;

    /// @notice Control whether verification should pass or fail (for negative testing)
    bool public shouldPass = true;

    constructor() {
        // Set mock VK hashes
        mockVKHashes[ProofType.COLLATERAL] = keccak256("MOCK_COLLATERAL_VK");
        mockVKHashes[ProofType.LTV] = keccak256("MOCK_LTV_VK");
        mockVKHashes[ProofType.LIQUIDATION] = keccak256("MOCK_LIQUIDATION_VK");
    }

    /// @notice Always returns true (or false if shouldPass is false)
    /// @dev Mock implementation for testing
    function verify(
        ProofType proofType,
        Proof calldata /* proof */,
        uint256[] calldata /* publicInputs */
    ) external view override returns (bool) {
        // Note: Can't modify state in view function, but we track for test purposes
        // verificationCount++;
        // lastProofType = proofType;

        // Just reference proofType to avoid unused variable warning
        if (uint8(proofType) >= 0) {
            return shouldPass;
        }
        return shouldPass;
    }

    /// @notice Get mock VK hash
    function getVerificationKeyHash(ProofType proofType) external view override returns (bytes32) {
        return mockVKHashes[proofType];
    }

    /// @notice Set whether verification should pass (for negative testing)
    function setShouldPass(bool _shouldPass) external {
        shouldPass = _shouldPass;
    }
}
