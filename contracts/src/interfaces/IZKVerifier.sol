// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title IZKVerifier
/// @notice Interface for ZK proof verification
/// @dev Supports multiple proof types for the ZK lending protocol
interface IZKVerifier {
    /// @notice Proof types supported by the verifier
    enum ProofType {
        COLLATERAL,   // Proves: collateral >= threshold
        LTV,          // Proves: debt/collateral <= max_ltv
        LIQUIDATION   // Proves: health_factor < 1.0
    }

    /// @notice Groth16 proof structure
    /// @param a G1 point (2 elements)
    /// @param b G2 point (2x2 elements)
    /// @param c G1 point (2 elements)
    struct Proof {
        uint256[2] a;
        uint256[2][2] b;
        uint256[2] c;
    }

    /// @notice Verify a ZK proof
    /// @param proofType The type of proof being verified
    /// @param proof The Groth16 proof
    /// @param publicInputs The public inputs for verification
    /// @return True if the proof is valid
    function verify(
        ProofType proofType,
        Proof calldata proof,
        uint256[] calldata publicInputs
    ) external view returns (bool);

    /// @notice Get the verification key hash for a proof type
    /// @param proofType The proof type
    /// @return The keccak256 hash of the verification key
    function getVerificationKeyHash(ProofType proofType) external view returns (bytes32);
}
