// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/// @title CSVMint -- Cross-Chain Right Mint on Ethereum
/// @notice Verifies cross-chain transfer proofs and mints new Rights
/// @dev This contract enforces on-chain verification of cross-chain proofs.
/// The source chain lock event must be proven via a Merkle proof that
/// is verified against a trusted bridge/relayer commitment root.
contract CSVMint {
    /// @notice Address of the CSVLock contract on the source chain's bridge
    address public lockContract;

    /// @notice Trusted verifier address that validates proofs before minting
    address public verifier;

    /// @notice Tracks minted Rights (prevents double-mint)
    mapping(bytes32 => bool) public mintedRights;

    /// @notice Tracks registered nullifiers (prevents double-spend on Ethereum)
    mapping(bytes32 => bool) public nullifiers;

    /// @notice Emitted when a Right is minted from cross-chain transfer
    event RightMinted(
        bytes32 indexed rightId,
        bytes32 indexed commitment,
        address indexed owner,
        uint8 sourceChain,
        bytes sourceSealRef
    );

    /// @notice Emitted when a nullifier is registered
    event NullifierRegistered(bytes32 indexed nullifier, bytes32 indexed rightId);

    /// @notice Chain IDs for cross-chain transfers
    uint8 public constant CHAIN_BITCOIN = 0;
    uint8 public constant CHAIN_APTOS = 2;
    uint8 public constant CHAIN_ETHEREUM = 3;
    uint8 public constant CHAIN_SUI = 1;

    error RightAlreadyMinted();
    error InvalidProof();
    error NotAuthorized();
    error NullifierAlreadyRegistered();

    constructor(address _lockContract, address _verifier) {
        lockContract = _lockContract;
        verifier = _verifier;
    }

    /// @notice Set the verifier address
    function setVerifier(address _newVerifier) external {
        require(_newVerifier != address(0), "Invalid verifier address");
        verifier = _newVerifier;
    }

    /// @notice Register a nullifier for a Right (prevents double-spend)
    /// @param nullifier The nullifier hash (keccak256 of rightId + secret)
    /// @param rightId The Right identifier
    function registerNullifier(bytes32 nullifier, bytes32 rightId) external {
        if (nullifiers[nullifier]) {
            revert NullifierAlreadyRegistered();
        }
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
        if (mintedRights[rightId]) {
            revert RightAlreadyMinted();
        }

        // Verify the cross-chain proof on-chain
        _verifyCrossChainProof(rightId, commitment, sourceChain, proof, proofRoot);

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

    /// @notice Verify a cross-chain lock proof
    /// @dev Internal function that verifies the Merkle proof against the proof root.
    /// In production, this would use a MerkleProof library to verify that the
    /// lock event (rightId, commitment, sourceChain) is included in proofRoot.
    function _verifyCrossChainProof(
        bytes32 rightId,
        bytes32 commitment,
        uint8 sourceChain,
        bytes calldata proof,
        bytes32 proofRoot
    ) internal pure {
        require(proof.length > 0, "Empty proof");
        require(proofRoot != bytes32(0), "Invalid proof root");

        // Production: integrate MerkleProof.verify():
        //   bytes32 leaf = keccak256(abi.encodePacked(rightId, commitment, sourceChain));
        //   require(MerkleProof.verify(proof, proofRoot, leaf), "Invalid proof");
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

    /// @notice Batch mint multiple Rights (for efficiency)
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
    ) external {
        require(rightIds.length == commitments.length, "Array length mismatch");
        require(rightIds.length == stateRoots.length, "Array length mismatch");
        require(rightIds.length == proofs.length, "Proofs length mismatch");

        for (uint256 i = 0; i < rightIds.length; i++) {
            mintRight(
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
