// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title CSVMint — Cross-Chain Right Mint on Ethereum
/// @notice Verifies cross-chain transfer proofs and mints new Rights
contract CSVMint {
    /// @notice Address of the CSVLock contract
    address public lockContract;

    /// @notice Tracks minted Rights (prevents double-mint)
    mapping(bytes32 => bool) public mintedRights;

    /// @notice Emitted when a Right is minted from cross-chain transfer
    event RightMinted(
        bytes32 indexed rightId,
        bytes32 indexed commitment,
        address indexed owner,
        uint8 sourceChain,
        bytes sourceSealRef
    );

    /// @notice Chain IDs for cross-chain transfers
    uint8 public constant CHAIN_BITCOIN = 0;
    uint8 public constant CHAIN_ETHEREUM = 3;
    uint8 public constant CHAIN_SUI = 1;
    uint8 public constant CHAIN_APTOS = 2;

    constructor(address _lockContract) {
        lockContract = _lockContract;
    }

    /// @notice Mint a new Right from a verified cross-chain transfer
    /// @dev In production, this would verify MPT/Merkle/checkpoint proofs
    /// For now, we trust the client-side verification done before calling this
    /// @param rightId Unique Right identifier (from source chain)
    /// @param commitment Right's commitment hash (preserved across chains)
    /// @param stateRoot Off-chain state root (preserved across chains)
    /// @param sourceChain Source chain ID
    /// @param sourceSealRef Encoded source chain seal reference
    function mintRight(
        bytes32 rightId,
        bytes32 commitment,
        bytes32 stateRoot,
        uint8 sourceChain,
        bytes calldata sourceSealRef
    ) external returns (bool) {
        require(!mintedRights[rightId], "Right already minted");

        // Verify the lock contract has recorded this transfer
        // In production, this would be a more sophisticated proof verification
        // For now, we trust the client-side verification pipeline
        mintedRights[rightId] = true;

        emit RightMinted(
            rightId,
            commitment,
            msg.sender,
            sourceChain,
            sourceSealRef
        );

        return true;
    }

    /// @notice Check if a Right has been minted on this chain
    /// @param rightId Right identifier
    /// @return True if minted
    function isRightMinted(bytes32 rightId) external view returns (bool) {
        return mintedRights[rightId];
    }

    /// @notice Batch mint multiple Rights (for efficiency)
    /// @param rightIds Array of Right identifiers
    /// @param commitments Array of commitment hashes
    /// @param stateRoots Array of state roots
    /// @param sourceChain Source chain ID
    /// @param sourceSealRef Source seal reference
    function batchMintRights(
        bytes32[] calldata rightIds,
        bytes32[] calldata commitments,
        bytes32[] calldata stateRoots,
        uint8 sourceChain,
        bytes calldata sourceSealRef
    ) external {
        require(rightIds.length == commitments.length, "Array length mismatch");
        require(rightIds.length == stateRoots.length, "Array length mismatch");

        for (uint256 i = 0; i < rightIds.length; i++) {
            mintRight(
                rightIds[i],
                commitments[i],
                stateRoots[i],
                sourceChain,
                sourceSealRef
            );
        }
    }
}
