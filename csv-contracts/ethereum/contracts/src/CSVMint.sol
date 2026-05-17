// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title CSVMint -- Cross-Chain Sanad Mint on Ethereum
/// @notice Verifies cross-chain transfer proofs and mints new Sanads
/// @dev This contract enforces on-chain verification of cross-chain proofs.
/// The source chain lock event must be proven via a Merkle proof that
/// is verified against a trusted bridge/relayer commitment root.
contract CSVMint {
    uint8 public constant ASSET_CLASS_UNSPECIFIED = 0;
    uint8 public constant ASSET_CLASS_FUNGIBLE_TOKEN = 1;
    uint8 public constant ASSET_CLASS_NON_FUNGIBLE_TOKEN = 2;
    uint8 public constant ASSET_CLASS_PROOF_SANAD = 3;
    uint8 public constant PROOF_SYSTEM_UNSPECIFIED = 0;

    /// @notice Address of the CSVLock contract on the source chain's bridge
    /// @dev Can be updated by admin after deployment
    address public lockContract;

    /// @notice Trusted verifier address that validates proofs before minting
    /// @dev Immutable - set at deployment and cannot be changed
    address public immutable verifier;

    /// @notice Tracks minted Sanads (prevents double-mint)
    mapping(bytes32 => bool) public mintedSanads;

    /// @notice Tracks registered nullifiers (prevents double-spend on Ethereum)
    mapping(bytes32 => bool) public nullifiers;

    struct SanadMetadata {
        uint8 assetClass;
        bytes32 assetId;
        bytes32 metadataHash;
        uint8 proofSystem;
        bytes32 proofRoot;
    }

    mapping(bytes32 => SanadMetadata) public sanadMetadata;

    /// @notice Emitted when a Sanad is minted from cross-chain transfer
    event SanadMinted(
        bytes32 indexed sanadId,
        bytes32 indexed commitment,
        address indexed owner,
        uint8 sourceChain,
        bytes sourceSealPoint,
        uint8 assetClass,
        bytes32 assetId,
        bytes32 metadataHash,
        uint8 proofSystem,
        bytes32 proofRoot
    );

    /// @notice Emitted when a nullifier is registered
    event NullifierRegistered(bytes32 indexed nullifier, bytes32 indexed sanadId);

    event SanadMetadataRecorded(
        bytes32 indexed sanadId,
        uint8 assetClass,
        bytes32 indexed assetId,
        bytes32 metadataHash,
        uint8 proofSystem,
        bytes32 indexed proofRoot
    );

    /// @notice Chain IDs for cross-chain transfers
    uint8 public constant CHAIN_BITCOIN = 0;
    uint8 public constant CHAIN_SUI = 1;
    uint8 public constant CHAIN_APTOS = 2;
    uint8 public constant CHAIN_ETHEREUM = 3;

    error SanadAlreadyMinted();
    error InvalidProof();
    error NullifierAlreadyRegistered();
    error ZeroAddress();
    error ArraysMismatch();
    error InvalidSanadMetadata();
    error Unauthorized();

    constructor(address _lockContract, address _verifier) {
        if (_verifier == address(0)) revert ZeroAddress();
        lockContract = _lockContract;
        verifier = _verifier;
    }

    /// @notice Update the lock contract address (admin only)
    /// @param _lockContract New lock contract address
    function setLockContract(address _lockContract) external {
        if (msg.sender != verifier) revert Unauthorized();
        if (_lockContract == address(0)) revert ZeroAddress();
        lockContract = _lockContract;
    }

    /// @notice Register a nullifier for a Sanad (prevents double-spend)
    /// @param nullifier The nullifier hash (keccak256 of sanadId + secret + context)
    /// @param sanadId The Sanad identifier
    function registerNullifier(bytes32 nullifier, bytes32 sanadId) external {
        if (nullifiers[nullifier]) revert NullifierAlreadyRegistered();
        nullifiers[nullifier] = true;
        emit NullifierRegistered(nullifier, sanadId);
    }

    /// @notice Mint a new Sanad from a verified cross-chain transfer
    /// @param sanadId Unique Sanad identifier (from source chain)
    /// @param commitment Sanad's commitment hash (preserved across chains)
    /// @param stateRoot Off-chain state root (preserved across chains)
    /// @param sourceChain Source chain ID
    /// @param sourceSealPoint Encoded source chain seal reference
    /// @param proof Merkle proof bytes verifying the source chain lock event
    /// @param proofRoot The trusted proof root (e.g., bridge commitment root)
    /// @param leafPosition Position of the leaf in the Merkle tree (for deterministic verification)
    function mintSanad(
        bytes32 sanadId,
        bytes32 commitment,
        bytes32 stateRoot,
        uint8 sourceChain,
        bytes calldata sourceSealPoint,
        bytes calldata proof,
        bytes32 proofRoot,
        uint256 leafPosition
    ) external returns (bool) {
        return _mintSanad(
            sanadId,
            commitment,
            stateRoot,
            sourceChain,
            sourceSealPoint,
            proof,
            proofRoot,
            leafPosition,
            SanadMetadata({
                assetClass: ASSET_CLASS_UNSPECIFIED,
                assetId: bytes32(0),
                metadataHash: bytes32(0),
                proofSystem: PROOF_SYSTEM_UNSPECIFIED,
                proofRoot: proofRoot
            })
        );
    }

    /// @notice Mint a Sanad with token/NFT/proof metadata preserved for indexers and future apps.
    function mintSanadWithMetadata(
        bytes32 sanadId,
        bytes32 commitment,
        bytes32 stateRoot,
        uint8 sourceChain,
        bytes calldata sourceSealPoint,
        bytes calldata proof,
        bytes32 proofRoot,
        uint8 assetClass,
        bytes32 assetId,
        bytes32 metadataHash,
        uint8 proofSystem,
        uint256 leafPosition
    ) external returns (bool) {
        SanadMetadata memory metadata = SanadMetadata({
            assetClass: assetClass,
            assetId: assetId,
            metadataHash: metadataHash,
            proofSystem: proofSystem,
            proofRoot: proofRoot
        });
        _validateMetadata(metadata);
        return _mintSanad(sanadId, commitment, stateRoot, sourceChain, sourceSealPoint, proof, proofRoot, leafPosition, metadata);
    }

    function _mintSanad(
        bytes32 sanadId,
        bytes32 commitment,
        bytes32 stateRoot,
        uint8 sourceChain,
        bytes calldata sourceSealPoint,
        bytes calldata proof,
        bytes32 proofRoot,
        uint256 leafPosition,
        SanadMetadata memory metadata
    ) internal returns (bool) {
        if (mintedSanads[sanadId]) revert SanadAlreadyMinted();
        if (stateRoot == bytes32(0)) revert InvalidProof();

        // Verify the cross-chain proof on-chain with leaf position
        _verifyCrossChainProof(sanadId, commitment, sourceChain, proof, proofRoot, leafPosition);

        mintedSanads[sanadId] = true;
        sanadMetadata[sanadId] = metadata;

        emit SanadMinted(
            sanadId,
            commitment,
            msg.sender,
            sourceChain,
            sourceSealPoint,
            metadata.assetClass,
            metadata.assetId,
            metadata.metadataHash,
            metadata.proofSystem,
            metadata.proofRoot
        );
        emit SanadMetadataRecorded(
            sanadId,
            metadata.assetClass,
            metadata.assetId,
            metadata.metadataHash,
            metadata.proofSystem,
            metadata.proofRoot
        );

        return true;
    }

    function _validateMetadata(SanadMetadata memory metadata) internal pure {
        if (metadata.assetClass > ASSET_CLASS_PROOF_SANAD) revert InvalidSanadMetadata();
        if (metadata.assetClass != ASSET_CLASS_UNSPECIFIED && metadata.assetId == bytes32(0)) {
            revert InvalidSanadMetadata();
        }
        if (metadata.proofSystem != PROOF_SYSTEM_UNSPECIFIED && metadata.proofRoot == bytes32(0)) {
            revert InvalidSanadMetadata();
        }
    }

    /// @notice Verify a cross-chain lock proof using Merkle tree verification
    /// @dev Computes the leaf hash as keccak256(sanadId || commitment || sourceChain)
    /// and verifies it against the proofRoot using the provided Merkle branch.
    function _verifyCrossChainProof(
        bytes32 sanadId,
        bytes32 commitment,
        uint8 sourceChain,
        bytes calldata proof,
        bytes32 proofRoot,
        uint256 leafPosition
    ) internal pure {
        // Validate non-empty inputs
        if (proof.length == 0) revert InvalidProof();
        if (proofRoot == bytes32(0)) revert InvalidProof();
        if (sanadId == bytes32(0)) revert InvalidProof();
        if (commitment == bytes32(0)) revert InvalidProof();

        // Build the leaf hash: keccak256(sanadId || commitment || sourceChain)
        bytes32 leaf = keccak256(abi.encodePacked(sanadId, commitment, sourceChain));

        // Verify the Merkle proof against the trusted root with leaf position
        if (!_verifyMerkleProof(proof, proofRoot, leaf, leafPosition)) revert InvalidProof();
    }

    /// @notice Verify a Merkle proof for leaf inclusion
    /// @dev Walks the Merkle tree bottom-up, hashing pairs at each level.
    /// The proof bytes are a concatenation of 32-byte sibling hashes.
    /// At each level, the current hash is paired with the sibling based on
    /// the current bit of the leaf position index.
    ///
    /// This implementation uses the leaf position to deterministically verify
    /// the Merkle proof. At each level, if the corresponding bit in leafPosition
    /// is 0, the current hash is the left child; if 1, it's the right child.
    function _verifyMerkleProof(
        bytes calldata proof,
        bytes32 root,
        bytes32 leaf,
        uint256 leafPosition
    ) internal pure returns (bool) {
        if (proof.length % 32 != 0) return false;

        uint256 numLevels = proof.length / 32;
        bytes32 current = leaf;

        for (uint256 i = 0; i < numLevels; i++) {
            bytes32 sibling;
            assembly {
                sibling := calldataload(add(proof.offset, mul(i, 32)))
            }

            // Use leafPosition bit to determine ordering
            // If bit is 0, current is left child; if 1, current is right child
            if ((leafPosition >> i) & 1 == 0) {
                current = _hashPair(current, sibling);
            } else {
                current = _hashPair(sibling, current);
            }
        }

        return current == root;
    }

    /// @notice Compute the parent hash of two child hashes (Bitcoin-style double SHA-256)
    /// For Ethereum, we use keccak256 which is the standard for Merkle trees on EVM.
    function _hashPair(bytes32 a, bytes32 b) internal pure returns (bytes32) {
        return a < b ? keccak256(abi.encodePacked(a, b)) : keccak256(abi.encodePacked(b, a));
    }

    /// @notice Check if a Sanad has been minted on this chain
    /// @param sanadId Sanad identifier
    /// @return True if minted
    function isSanadMinted(bytes32 sanadId) external view returns (bool) {
        return mintedSanads[sanadId];
    }

    /// @notice Check if a nullifier has been registered
    /// @param nullifier The nullifier hash
    /// @return True if registered
    function isNullifierRegistered(bytes32 nullifier) external view returns (bool) {
        return nullifiers[nullifier];
    }

    /// @notice Batch mint multiple Sanads (for efficiency)
    /// @param sanadIds Array of Sanad identifiers
    /// @param commitments Array of commitment hashes
    /// @param stateRoots Array of state roots
    /// @param sourceChain Source chain ID
    /// @param sourceSealPoint Source seal reference
    /// @param proofs Array of proof bytes for each mint
    /// @param proofRoot The trusted proof root
    /// @param leafPositions Array of leaf positions for each mint
    function batchMintSanads(
        bytes32[] calldata sanadIds,
        bytes32[] calldata commitments,
        bytes32[] calldata stateRoots,
        uint8 sourceChain,
        bytes calldata sourceSealPoint,
        bytes[] calldata proofs,
        bytes32 proofRoot,
        uint256[] calldata leafPositions
    ) external {
        if (msg.sender != verifier) revert Unauthorized();

        if (
            sanadIds.length != commitments.length ||
            sanadIds.length != stateRoots.length ||
            sanadIds.length != proofs.length ||
            sanadIds.length != leafPositions.length
        ) revert ArraysMismatch();

        for (uint256 i = 0; i < sanadIds.length; i++) {
            this.mintSanad(
                sanadIds[i],
                commitments[i],
                stateRoots[i],
                sourceChain,
                sourceSealPoint,
                proofs[i],
                proofRoot,
                leafPositions[i]
            );
        }
    }
}
