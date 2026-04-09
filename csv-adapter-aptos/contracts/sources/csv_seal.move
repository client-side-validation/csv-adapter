/// CSV Seal — Cross-Chain Right Transfer on Aptos
///
/// This module implements:
/// - `create_seal()` — Create a new Right anchored to a Move resource
/// - `delete_seal()` — Consume a Right (single-use enforcement via resource destruction)
/// - `lock_right()` — Lock a Right for cross-chain transfer (destroys resource, emits event)
/// - `mint_right()` — Mint a new Right from a cross-chain transfer proof

module csv_seal::csv_seal {
    use std::signer;
    use aptos_framework::account;
    use aptos_framework::event;

    /// A Right anchored to Aptos as a Move resource.
    /// The resource's existence = the Right's validity.
    /// Destroying the resource = consuming the Right (single-use enforced by Move VM).
    struct RightResource has key {
        /// Unique Right identifier (preserved across chains)
        right_id: vector<u8>,
        /// Commitment hash (preserved across chains)
        commitment: vector<u8>,
        /// Nullifier (for L3 chains that use nullifiers)
        nullifier: vector<u8>,
        /// State root (off-chain state commitment)
        state_root: vector<u8>,
    }

    /// Emitted when a Right is created
    struct RightCreated has drop, store {
        right_id: vector<u8>,
        commitment: vector<u8>,
        owner: address,
    }

    /// Emitted when a Right is consumed
    struct RightConsumed has drop, store {
        right_id: vector<u8>,
        consumer: address,
    }

    /// Emitted when a Right is locked for cross-chain transfer
    struct CrossChainLock has drop, store {
        right_id: vector<u8>,
        commitment: vector<u8>,
        owner: address,
        destination_chain: u8,
        destination_owner: vector<u8>,
    }

    /// Emitted when a Right is minted from cross-chain transfer
    struct CrossChainMint has drop, store {
        right_id: vector<u8>,
        commitment: vector<u8>,
        owner: address,
        source_chain: u8,
        source_seal_ref: vector<u8>,
    }

    /// Create a new Right on Aptos
    public entry fun create_seal(
        account: &signer,
        right_id: vector<u8>,
        commitment: vector<u8>,
        state_root: vector<u8>,
    ) acquires RightResource {
        let owner = signer::address_of(account);

        assert!(
            !exists<RightResource>(owner),
            1001 // Right already exists
        );

        move_to(account, RightResource {
            right_id,
            commitment,
            nullifier: vector::empty<u8>(),
            state_root,
        });

        event::emit(RightCreated {
            right_id,
            commitment,
            owner,
        });
    }

    /// Consume a Right (single-use enforcement via resource destruction)
    public entry fun delete_seal(account: &signer) acquires RightResource {
        let owner = signer::address_of(account);
        let RightResource { right_id, commitment: _, nullifier: _, state_root: _ } =
            move_from<RightResource>(owner);

        event::emit(RightConsumed {
            right_id,
            consumer: owner,
        });
    }

    /// Lock a Right for cross-chain transfer.
    /// This destroys the Right resource (single-use enforced by Move VM) and emits event.
    public entry fun lock_right(
        account: &signer,
        destination_chain: u8,
        destination_owner: vector<u8>,
    ) acquires RightResource {
        let owner = signer::address_of(account);
        let RightResource { right_id, commitment, nullifier: _, state_root: _ } =
            move_from<RightResource>(owner);

        event::emit(CrossChainLock {
            right_id,
            commitment,
            owner,
            destination_chain,
            destination_owner,
        });
    }

    /// Mint a new Right from a cross-chain transfer proof.
    /// This creates a new RightResource with the same commitment as the source chain's Right.
    public entry fun mint_right(
        account: &signer,
        right_id: vector<u8>,
        commitment: vector<u8>,
        state_root: vector<u8>,
        source_chain: u8,
        source_seal_ref: vector<u8>,
    ) acquires RightResource {
        let owner = signer::address_of(account);

        assert!(
            !exists<RightResource>(owner),
            1002 // Right already exists at this address
        );

        move_to(account, RightResource {
            right_id,
            commitment,
            nullifier: vector::empty<u8>(),
            state_root,
        });

        event::emit(CrossChainMint {
            right_id,
            commitment,
            owner,
            source_chain,
            source_seal_ref,
        });
    }
}
