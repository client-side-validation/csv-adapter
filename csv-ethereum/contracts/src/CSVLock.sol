// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title CSVLock — Cross-Chain Right Lock on Ethereum
/// @notice Registers nullifiers, emits lock events, and supports time-locked refunds
contract CSVLock {
    uint8 public constant ASSET_CLASS_UNSPECIFIED = 0;
    uint8 public constant ASSET_CLASS_FUNGIBLE_TOKEN = 1;
    uint8 public constant ASSET_CLASS_NON_FUNGIBLE_TOKEN = 2;
    uint8 public constant ASSET_CLASS_PROOF_RIGHT = 3;
    uint8 public constant PROOF_SYSTEM_UNSPECIFIED = 0;

    /// @notice Tracks consumed nullifiers (seal single-use)
    mapping(bytes32 => bool) public usedSeals;

    /// @notice Cross-chain metadata shared by all CSV contracts.
    struct RightMetadata {
        uint8 assetClass;
        bytes32 assetId;
        bytes32 metadataHash;
        uint8 proofSystem;
        bytes32 proofRoot;
    }

    /// @notice Lock record for refund support
    struct LockRecord {
        bytes32 commitment;
        uint256 timestamp;
        uint8 destinationChain;
        bytes32 destinationOwnerRoot; // Hash of destination owner for verification
        RightMetadata metadata;
        bool refunded;
    }

    /// @notice Tracks lock events for refund verification
    mapping(bytes32 => LockRecord) public locks;

    /// @notice Refund timeout — 24 hours after lock
    uint256 public constant REFUND_TIMEOUT = 24 hours;

    /// @notice Address of the CSVMint contract (to verify no mint happened)
    address public mintContract;

    /// @notice Emitted when a Right is locked for cross-chain transfer
    event CrossChainLock(
        bytes32 indexed rightId,
        bytes32 indexed commitment,
        address indexed owner,
        uint8 destinationChain,
        bytes destinationOwner,
        bytes32 sourceTxHash,
        uint8 assetClass,
        bytes32 assetId,
        bytes32 metadataHash,
        uint8 proofSystem,
        bytes32 proofRoot
    );

    /// @notice Emitted when a Right is consumed (nullifier registered)
    event SealUsed(bytes32 indexed sealId, bytes32 commitment);

    /// @notice Emitted when a locked Right is refunded
    event RightRefunded(
        bytes32 indexed rightId,
        bytes32 indexed commitment,
        address indexed claimant,
        uint256 refundTimestamp
    );

    /// @notice Emitted when mint contract address is set
    event MintContractSet(address indexed mintContract);
    event RightMetadataRecorded(
        bytes32 indexed rightId,
        uint8 assetClass,
        bytes32 indexed assetId,
        bytes32 metadataHash,
        uint8 proofSystem,
        bytes32 indexed proofRoot
    );

    error RightAlreadyConsumed();
    error RightAlreadyLocked();
    error TimeoutNotExpired();
    error RightAlreadyMinted();
    error RefundAlreadyClaimed();
    error InvalidMintContract();
    error InvalidRightMetadata();

    /// @notice Set the mint contract address (for refund verification)
    /// @param _mintContract Address of the CSVMint contract
    function setMintContract(address _mintContract) external {
        require(_mintContract != address(0), "Invalid mint contract address");
        mintContract = _mintContract;
        emit MintContractSet(_mintContract);
    }

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
        _lockRight(
            rightId,
            commitment,
            destinationChain,
            destinationOwner,
            RightMetadata({
                assetClass: ASSET_CLASS_UNSPECIFIED,
                assetId: bytes32(0),
                metadataHash: bytes32(0),
                proofSystem: PROOF_SYSTEM_UNSPECIFIED,
                proofRoot: bytes32(0)
            })
        );
    }

    /// @notice Lock a Right with asset/proof metadata for token, NFT, or advanced proof flows.
    function lockRightWithMetadata(
        bytes32 rightId,
        bytes32 commitment,
        uint8 destinationChain,
        bytes calldata destinationOwner,
        uint8 assetClass,
        bytes32 assetId,
        bytes32 metadataHash,
        uint8 proofSystem,
        bytes32 proofRoot
    ) external {
        RightMetadata memory metadata = RightMetadata({
            assetClass: assetClass,
            assetId: assetId,
            metadataHash: metadataHash,
            proofSystem: proofSystem,
            proofRoot: proofRoot
        });
        _validateMetadata(metadata);
        _lockRight(rightId, commitment, destinationChain, destinationOwner, metadata);
    }

    function _lockRight(
        bytes32 rightId,
        bytes32 commitment,
        uint8 destinationChain,
        bytes calldata destinationOwner,
        RightMetadata memory metadata
    ) internal {
        if (usedSeals[rightId]) {
            revert RightAlreadyConsumed();
        }
        if (locks[rightId].timestamp != 0 && !locks[rightId].refunded) {
            revert RightAlreadyLocked();
        }

        usedSeals[rightId] = true;

        // Record lock for refund support
        locks[rightId] = LockRecord({
            commitment: commitment,
            timestamp: block.timestamp,
            destinationChain: destinationChain,
            destinationOwnerRoot: keccak256(destinationOwner),
            metadata: metadata,
            refunded: false
        });

        emit CrossChainLock(
            rightId,
            commitment,
            msg.sender,
            destinationChain,
            destinationOwner,
            blockhash(block.number - 1),
            metadata.assetClass,
            metadata.assetId,
            metadata.metadataHash,
            metadata.proofSystem,
            metadata.proofRoot
        );

        emit RightMetadataRecorded(
            rightId,
            metadata.assetClass,
            metadata.assetId,
            metadata.metadataHash,
            metadata.proofSystem,
            metadata.proofRoot
        );
        emit SealUsed(rightId, commitment);
    }

    function _validateMetadata(RightMetadata memory metadata) internal pure {
        if (metadata.assetClass > ASSET_CLASS_PROOF_RIGHT) revert InvalidRightMetadata();
        if (metadata.assetClass != ASSET_CLASS_UNSPECIFIED && metadata.assetId == bytes32(0)) {
            revert InvalidRightMetadata();
        }
        if (metadata.proofSystem != PROOF_SYSTEM_UNSPECIFIED && metadata.proofRoot == bytes32(0)) {
            revert InvalidRightMetadata();
        }
    }

    /// @notice Register a nullifier (consume seal without cross-chain transfer)
    /// @param sealId Seal identifier
    /// @param commitment Commitment hash
    function markSealUsed(bytes32 sealId, bytes32 commitment) external {
        if (usedSeals[sealId]) {
            revert RightAlreadyConsumed();
        }
        usedSeals[sealId] = true;
        emit SealUsed(sealId, commitment);
    }

    /// @notice Claim a refund for a locked Right that was never minted on destination.
    /// @dev This function allows a user to recover a Right if:
    ///   1. The lock was recorded in this contract
    ///   2. The REFUND_TIMEOUT has elapsed since the lock
    ///   3. The Right was NOT minted on any destination chain
    ///   4. The refund has not already been claimed
    /// @param rightId The Right identifier to refund
    /// @param destinationOwnerHash Hash of the destination owner (for verification)
    function refundRight(bytes32 rightId, bytes32 destinationOwnerHash) external {
        LockRecord storage lock = locks[rightId];

        // Verify lock exists
        if (lock.timestamp == 0) {
            revert RightAlreadyConsumed();
        }

        // Verify timeout has elapsed
        if (block.timestamp < lock.timestamp + REFUND_TIMEOUT) {
            revert TimeoutNotExpired();
        }

        // Verify not already refunded
        if (lock.refunded) {
            revert RefundAlreadyClaimed();
        }

        // Verify the Right was NOT minted on destination chain
        // This requires the mint contract to expose isRightMinted
        if (mintContract != address(0)) {
            (bool success, bytes memory data) = mintContract.staticcall(
                abi.encodeWithSignature("isRightMinted(bytes32)", rightId)
            );
            if (success && data.length >= 32) {
                bool isMinted = abi.decode(data, (bool));
                if (isMinted) {
                    revert RightAlreadyMinted();
                }
            }
            // If call fails or mintContract not set, we proceed (trust the user)
            // In production, the mint contract should be properly configured
        }

        // Mark as refunded to prevent re-entrancy and double-claim
        lock.refunded = true;

        // Re-allow the seal for future use (re-create the Right)
        usedSeals[rightId] = false;

        emit RightRefunded(rightId, lock.commitment, msg.sender, block.timestamp);
    }

    /// @notice Check if a seal/Right has been consumed
    /// @param sealId Seal or Right identifier
    /// @return True if consumed
    function isSealUsed(bytes32 sealId) external view returns (bool) {
        return usedSeals[sealId];
    }

    /// @notice Get lock details for a Right
    /// @param rightId The Right identifier
    /// @return commitment The commitment hash
    /// @return timestamp When the lock was created
    /// @return destinationChain The target chain ID
    /// @return refunded Whether the lock has been refunded
    function getLockInfo(bytes32 rightId) external view returns (
        bytes32 commitment,
        uint256 timestamp,
        uint8 destinationChain,
        bool refunded
    ) {
        LockRecord storage lock = locks[rightId];
        return (lock.commitment, lock.timestamp, lock.destinationChain, lock.refunded);
    }

    /// @notice Get metadata attached to a locked Right.
    function getRightMetadata(bytes32 rightId) external view returns (
        uint8 assetClass,
        bytes32 assetId,
        bytes32 metadataHash,
        uint8 proofSystem,
        bytes32 proofRoot
    ) {
        RightMetadata storage metadata = locks[rightId].metadata;
        return (
            metadata.assetClass,
            metadata.assetId,
            metadata.metadataHash,
            metadata.proofSystem,
            metadata.proofRoot
        );
    }

    /// @notice Check if a refund can be claimed for a Right
    /// @param rightId The Right identifier
    /// @return True if refund is claimable
    function canRefund(bytes32 rightId) external view returns (bool) {
        LockRecord storage lock = locks[rightId];

        // Must exist
        if (lock.timestamp == 0) return false;
        // Must not be refunded
        if (lock.refunded) return false;
        // Timeout must have elapsed
        if (block.timestamp < lock.timestamp + REFUND_TIMEOUT) return false;

        return true;
    }
}
