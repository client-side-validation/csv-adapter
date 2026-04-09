// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title CSVLock — Cross-Chain Right Lock on Ethereum
/// @notice Registers nullifiers and emits lock events for cross-chain Right transfers
contract CSVLock {
    /// @notice Tracks consumed nullifiers (seal single-use)
    mapping(bytes32 => bool) public usedSeals;

    /// @notice Emitted when a Right is locked for cross-chain transfer
    event CrossChainLock(
        bytes32 indexed rightId,
        bytes32 indexed commitment,
        address indexed owner,
        uint8 destinationChain,
        bytes destinationOwner,
        bytes32 sourceTxHash
    );

    /// @notice Emitted when a Right is consumed (nullifier registered)
    event SealUsed(bytes32 indexed sealId, bytes32 commitment);

    /// @notice Lock a Right for cross-chain transfer
    /// @param rightId Unique Right identifier
    /// @param commitment Right's commitment hash
    /// @param destinationChain Target chain ID
    /// @param destinationOwner Encoded destination owner address
    function lockRight(
        bytes32 rightId,
        bytes32 commitment,
        uint8 destinationChain,
        bytes calldata destinationOwner
    ) external {
        require(!usedSeals[rightId], "Right already consumed");
        usedSeals[rightId] = true;

        emit CrossChainLock(
            rightId,
            commitment,
            msg.sender,
            destinationChain,
            destinationOwner,
            blockhash(block.number - 1)
        );

        emit SealUsed(rightId, commitment);
    }

    /// @notice Register a nullifier (consume seal without cross-chain transfer)
    /// @param sealId Seal identifier
    /// @param commitment Commitment hash
    function markSealUsed(bytes32 sealId, bytes32 commitment) external {
        require(!usedSeals[sealId], "Seal already consumed");
        usedSeals[sealId] = true;
        emit SealUsed(sealId, commitment);
    }

    /// @notice Check if a seal/Right has been consumed
    /// @param sealId Seal or Right identifier
    /// @return True if consumed
    function isSealUsed(bytes32 sealId) external view returns (bool) {
        return usedSeals[sealId];
    }
}
