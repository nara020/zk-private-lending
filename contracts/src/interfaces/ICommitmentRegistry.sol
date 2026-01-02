// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title ICommitmentRegistry
/// @notice Interface for managing Pedersen commitments
/// @dev Stores commitments that hide actual values while allowing ZK verification
interface ICommitmentRegistry {
    /// @notice Emitted when a new commitment is registered
    event CommitmentRegistered(
        address indexed user,
        bytes32 indexed commitment,
        CommitmentType commitmentType,
        uint256 timestamp
    );

    /// @notice Emitted when a commitment is nullified (spent/used)
    event CommitmentNullified(
        address indexed user,
        bytes32 indexed commitment,
        bytes32 nullifier,
        uint256 timestamp
    );

    /// @notice Types of commitments
    enum CommitmentType {
        COLLATERAL,  // Commitment to collateral amount
        DEBT         // Commitment to debt amount
    }

    /// @notice Register a new commitment
    /// @param commitment The Pedersen commitment hash
    /// @param commitmentType Type of the commitment
    /// @param user The user address who owns this commitment
    function registerCommitment(bytes32 commitment, CommitmentType commitmentType, address user) external;

    /// @notice Nullify a commitment (mark as spent)
    /// @param commitment The commitment to nullify
    /// @param nullifier The nullifier to prevent double-spending
    function nullifyCommitment(bytes32 commitment, bytes32 nullifier) external;

    /// @notice Update a commitment (e.g., for additional deposit, partial repayment)
    /// @param oldCommitment The existing commitment to update
    /// @param newCommitment The new commitment value
    /// @param nullifier The nullifier for the old commitment
    function updateCommitment(bytes32 oldCommitment, bytes32 newCommitment, bytes32 nullifier) external;

    /// @notice Check if a commitment exists and is valid
    /// @param commitment The commitment to check
    /// @return True if the commitment exists and is not nullified
    function isValidCommitment(bytes32 commitment) external view returns (bool);

    /// @notice Check if a nullifier has been used
    /// @param nullifier The nullifier to check
    /// @return True if the nullifier has been used
    function isNullifierUsed(bytes32 nullifier) external view returns (bool);

    /// @notice Get commitment details for a user
    /// @param user The user address
    /// @return collateralCommitment The user's collateral commitment
    /// @return debtCommitment The user's debt commitment
    function getUserCommitments(address user)
        external
        view
        returns (bytes32 collateralCommitment, bytes32 debtCommitment);
}
