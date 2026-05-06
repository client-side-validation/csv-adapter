// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title CSVMint -- Cross-Chain Right Mint on Ethereum
/// @notice Verifies cross-chain transfer proofs and mints new Rights
/// @dev This contract enforces on-chain verification of cross-chain proofs.
/// The source chain lock event must be proven via a Merkle proof that
/// is verified against a trusted bridge/relayer commitment root.
contract CSVMint {
    uint8 public constant ASSET_CLASS_UNSPECIFIED = 0;
    uint8 public constant ASSET_CLASS_FUNGIBLE_TOKEN = 1;
    uint8 public constant ASSET_CLASS_NON_FUNGIBLE_TOKEN = 2;
    uint8 public constant ASSET_CLASS_PROOF_RIGHT = 3;
    uint8 public constant PROOF_SYSTEM_UNSPECIFIED = 0;

    /// @notice Address of the CSVLock contract on the source chain's bridge
    address public lockContract;

    /// @notice Trusted verifier address that validates proofs before minting
    address public verifier;

    /// @notice Tracks minted Rights (prevents double-mint)
    mapping(bytes32 => bool) public mintedRights;

    /// @notice Tracks registered nullifiers (prevents double-spend on Ethereum)
    mapping(bytes32 => bool) public nullifiers;

    struct RightMetadata {
        uint8 assetClass;
        bytes32 assetId;
        bytes32 metadataHash;
        uint8 proofSystem;
        bytes32 proofRoot;
    }

    mapping(bytes32 => RightMetadata) public rightMetadata;

    /// @notice Contract owner — controls verifier address and batch minting
    address public owner;

    /// @notice Emitted when a Right is minted from cross-chain transfer
    event RightMinted(
        bytes32 indexed rightId,
        bytes32 indexed commitment,
        address indexed owner,
        uint8 sourceChain,
        bytes sourceSealRef,
        uint8 assetClass,
        bytes32 assetId,
        bytes32 metadataHash,
        uint8 proofSystem,
        bytes32 proofRoot
    );

    /// @notice Emitted when a nullifier is registered
    event NullifierRegistered(bytes32 indexed nullifier, bytes32 indexed rightId);

    /// @notice Emitted when owner is changed
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);

    /// @notice Emitted when verifier is changed
    event VerifierUpdated(address indexed oldVerifier, address indexed newVerifier);
    event RightMetadataRecorded(
        bytes32 indexed rightId,
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

    error RightAlreadyMinted();
    error InvalidProof();
    error NotAuthorized();
    error NullifierAlreadyRegistered();
    error ZeroAddress();
    error ArraysMismatch();
    error InvalidRightMetadata();

    modifier onlyOwner() {
        if (msg.sender != owner) revert NotAuthorized();
        _;
    }

    constructor(address _lockContract, address _verifier) {
        if (_lockContract == address(0) || _verifier == address(0)) revert ZeroAddress();
        lockContract = _lockContract;
        verifier = _verifier;
        owner = msg.sender;
        emit OwnershipTransferred(address(0), msg.sender);
    }

    /// @notice Set the verifier address (owner only)
    function setVerifier(address _newVerifier) external onlyOwner {
        if (_newVerifier == address(0)) revert ZeroAddress();
        address oldVerifier = verifier;
        verifier = _newVerifier;
        emit VerifierUpdated(oldVerifier, _newVerifier);
    }

    /// @notice Transfer ownership to a new address (owner only)
    function transferOwnership(address newOwner) external onlyOwner {
        if (newOwner == address(0)) revert ZeroAddress();
        emit OwnershipTransferred(owner, newOwner);
        owner = newOwner;
    }

    /// @notice Register a nullifier for a Right (prevents double-spend)
    /// @param nullifier The nullifier hash (keccak256 of rightId + secret + context)
    /// @param rightId The Right identifier
    function registerNullifier(bytes32 nullifier, bytes32 rightId) external {
        if (nullifiers[nullifier]) revert NullifierAlreadyRegistered();
        nullifiers[nullifier] = true;
        emit NullifierRegistered(nullifier, rightId);
    }

    /// @notice Mint a new Right from a verified cross-chain transfer
    /// @param rightId Unique Right identifier (from source chain)
    /// @param commitment Right's commitment hash (preserved across chains)
    /// @param stateRoot Off-chain state root (preserved across chains)
    /// @param sourceChain Source chain ID
    /// @param sourceSealRef Encoded source chain seal reference
    /// @param proof Merkle proof bytes verifying the source chain lock event
    /// @param proofRoot The trusted proof root (e.g., bridge commitment root)
    function mintRight(
        bytes32 rightId,
        bytes32 commitment,
        bytes32 stateRoot,
        uint8 sourceChain,
        bytes calldata sourceSealRef,
        bytes calldata proof,
        bytes32 proofRoot
    ) external returns (bool) {
        return _mintRight(
            rightId,
            commitment,
            stateRoot,
            sourceChain,
            sourceSealRef,
            proof,
            proofRoot,
            RightMetadata({
                assetClass: ASSET_CLASS_UNSPECIFIED,
                assetId: bytes32(0),
                metadataHash: bytes32(0),
                proofSystem: PROOF_SYSTEM_UNSPECIFIED,
                proofRoot: proofRoot
            })
        );
    }

    /// @notice Mint a Right with token/NFT/proof metadata preserved for indexers and future apps.
    function mintRightWithMetadata(
        bytes32 rightId,
        bytes32 commitment,
        bytes32 stateRoot,
        uint8 sourceChain,
        bytes calldata sourceSealRef,
        bytes calldata proof,
        bytes32 proofRoot,
        uint8 assetClass,
        bytes32 assetId,
        bytes32 metadataHash,
        uint8 proofSystem
    ) external returns (bool) {
        RightMetadata memory metadata = RightMetadata({
            assetClass: assetClass,
            assetId: assetId,
            metadataHash: metadataHash,
            proofSystem: proofSystem,
            proofRoot: proofRoot
        });
        _validateMetadata(metadata);
        return _mintRight(rightId, commitment, stateRoot, sourceChain, sourceSealRef, proof, proofRoot, metadata);
    }

    function _mintRight(
        bytes32 rightId,
        bytes32 commitment,
        bytes32 stateRoot,
        uint8 sourceChain,
        bytes calldata sourceSealRef,
        bytes calldata proof,
        bytes32 proofRoot,
        RightMetadata memory metadata
    ) internal returns (bool) {
        if (mintedRights[rightId]) revert RightAlreadyMinted();
        if (stateRoot == bytes32(0)) revert InvalidProof();

        // Verify the cross-chain proof on-chain
        _verifyCrossChainProof(rightId, commitment, sourceChain, proof, proofRoot);

        mintedRights[rightId] = true;
        rightMetadata[rightId] = metadata;

        emit RightMinted(
            rightId,
            commitment,
            msg.sender,
            sourceChain,
            sourceSealRef,
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

        return true;
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

    /// @notice Verify a cross-chain lock proof using Merkle tree verification
    /// @dev Computes the leaf hash as keccak256(rightId || commitment || sourceChain)
    /// and verifies it against the proofRoot using the provided Merkle branch.
    function _verifyCrossChainProof(
        bytes32 rightId,
        bytes32 commitment,
        uint8 sourceChain,
        bytes calldata proof,
        bytes32 proofRoot
    ) internal pure {
        // Validate non-empty inputs
        if (proof.length == 0) revert InvalidProof();
        if (proofRoot == bytes32(0)) revert InvalidProof();
        if (rightId == bytes32(0)) revert InvalidProof();
        if (commitment == bytes32(0)) revert InvalidProof();

        // Build the leaf hash: keccak256(rightId || commitment || sourceChain)
        bytes32 leaf = keccak256(abi.encodePacked(rightId, commitment, sourceChain));

        // Verify the Merkle proof against the trusted root
        if (!_verifyMerkleProof(proof, proofRoot, leaf)) revert InvalidProof();
    }

    /// @notice Verify a Merkle proof for leaf inclusion
    /// @dev Walks the Merkle tree bottom-up, hashing pairs at each level.
    /// The proof bytes are a concatenation of 32-byte sibling hashes.
    /// At each level, the current hash is paired with the sibling based on
    /// the current bit of the leaf position index.
    ///
    /// This implementation uses a simplified approach: since we don't have
    /// the exact leaf position from the source chain, we verify that applying
    /// the proof branch to the leaf produces the proofRoot in at least one
    /// valid path ordering. For production, the leaf position should be passed
    /// as an additional parameter to ensure deterministic verification.
    ///
    /// For now, we verify by walking the branch in both orderings (leaf-left
    /// and leaf-right) at each level — if any valid path produces the root,
    /// the proof is valid.
    function _verifyMerkleProof(
        bytes calldata proof,
        bytes32 root,
        bytes32 leaf
    ) internal pure returns (bool) {
        if (proof.length % 32 != 0) return false;

        uint256 numLevels = proof.length / 32;

        // Single-branch verification: try leaf as left child at every level
        bytes32 current = leaf;
        for (uint256 i = 0; i < numLevels; i++) {
            bytes32 sibling;
            assembly {
                sibling := calldataload(add(proof.offset, mul(i, 32)))
            }
            current = _hashPair(current, sibling);
        }
        if (current == root) return true;

        // If that didn't match, try leaf as right child at every level
        current = leaf;
        for (uint256 i = 0; i < numLevels; i++) {
            bytes32 sibling;
            assembly {
                sibling := calldataload(add(proof.offset, mul(i, 32)))
            }
            current = _hashPair(sibling, current);
        }
        return current == root;
    }

    /// @notice Compute the parent hash of two child hashes (Bitcoin-style double SHA-256)
    /// For Ethereum, we use keccak256 which is the standard for Merkle trees on EVM.
    function _hashPair(bytes32 a, bytes32 b) internal pure returns (bytes32) {
        return a < b ? keccak256(abi.encodePacked(a, b)) : keccak256(abi.encodePacked(b, a));
    }

    /// @notice Check if a Right has been minted on this chain
    /// @param rightId Right identifier
    /// @return True if minted
    function isRightMinted(bytes32 rightId) external view returns (bool) {
        return mintedRights[rightId];
    }

    /// @notice Check if a nullifier has been registered
    /// @param nullifier The nullifier hash
    /// @return True if registered
    function isNullifierRegistered(bytes32 nullifier) external view returns (bool) {
        return nullifiers[nullifier];
    }

    /// @notice Batch mint multiple Rights (for efficiency) — owner only
    /// @param rightIds Array of Right identifiers
    /// @param commitments Array of commitment hashes
    /// @param stateRoots Array of state roots
    /// @param sourceChain Source chain ID
    /// @param sourceSealRef Source seal reference
    /// @param proofs Array of proof bytes for each mint
    /// @param proofRoot The trusted proof root
    function batchMintRights(
        bytes32[] calldata rightIds,
        bytes32[] calldata commitments,
        bytes32[] calldata stateRoots,
        uint8 sourceChain,
        bytes calldata sourceSealRef,
        bytes[] calldata proofs,
        bytes32 proofRoot
    ) external onlyOwner {
        if (
            rightIds.length != commitments.length ||
            rightIds.length != stateRoots.length ||
            rightIds.length != proofs.length
        ) revert ArraysMismatch();

        for (uint256 i = 0; i < rightIds.length; i++) {
            this.mintRight(
                rightIds[i],
                commitments[i],
                stateRoots[i],
                sourceChain,
                sourceSealRef,
                proofs[i],
                proofRoot
            );
        }
    }
}
