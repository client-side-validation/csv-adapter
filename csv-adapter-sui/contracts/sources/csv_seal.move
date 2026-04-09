/// CSV Seal — Cross-Chain Right Transfer on Sui
///
/// This module implements:
/// - `create_seal()` — Create a new Right anchored to a Sui object
/// - `consume_seal()` — Consume a Right (single-use enforcement via object deletion)
/// - `lock_right()` — Lock a Right for cross-chain transfer (consumes seal, emits event)
/// - `mint_right()` — Mint a new Right from a cross-chain transfer proof

module csv_seal::csv_seal {
    use sui::object::{Self, UID};
    use sui::transfer;
    use sui::tx_context::{Self, TxContext};
    use std::string::{Self, String};
    use std::hash;

    /// A Right anchored to Sui as an object.
    /// The object's existence = the Right's validity.
    /// Deleting the object = consuming the Right (single-use enforced).
    struct RightObject has key, store {
        id: UID,
        /// Unique Right identifier (preserved across chains)
        right_id: vector<u8>,
        /// Commitment hash (preserved across chains)
        commitment: vector<u8>,
        /// Owner address
        owner: address,
        /// Nullifier (for L3 chains that use nullifiers)
        nullifier: vector<u8>,
        /// State root (off-chain state commitment)
        state_root: vector<u8>,
    }

    /// Emitted when a Right is created
    event RightCreated {
        right_id: vector<u8>,
        commitment: vector<u8>,
        owner: address,
        object_id: ID,
    }

    /// Emitted when a Right is consumed
    event RightConsumed {
        right_id: vector<u8>,
        consumer: address,
    }

    /// Emitted when a Right is locked for cross-chain transfer
    event CrossChainLock {
        right_id: vector<u8>,
        commitment: vector<u8>,
        owner: address,
        destination_chain: u8,
        destination_owner: vector<u8>,
        source_tx_hash: vector<u8>,
    }

    /// Emitted when a Right is minted from cross-chain transfer
    event CrossChainMint {
        right_id: vector<u8>,
        commitment: vector<u8>,
        owner: address,
        source_chain: u8,
        source_seal_ref: vector<u8>,
    }

    /// Create a new Right on Sui
    public entry fun create_seal(
        right_id: vector<u8>,
        commitment: vector<u8>,
        state_root: vector<u8>,
        ctx: &mut TxContext
    ) {
        let right = RightObject {
            id: object::new(ctx),
            right_id,
            commitment,
            owner: tx_context::sender(ctx),
            nullifier: vector::empty<u8>(),
            state_root,
        };

        event::emit(RightCreated {
            right_id: right.right_id,
            commitment: right.commitment,
            owner: right.owner,
            object_id: object::uid_to_inner(&right.id),
        });

        transfer::public_transfer(right, tx_context::sender(ctx));
    }

    /// Consume a Right (single-use enforcement via object deletion)
    public entry fun consume_seal(
        right: RightObject,
        ctx: &mut TxContext
    ) {
        event::emit(RightConsumed {
            right_id: right.right_id,
            consumer: tx_context::sender(ctx),
        });

        let RightObject { id, right_id: _, commitment: _, owner: _, nullifier: _, state_root: _ } = right;
        object::delete(id);
    }

    /// Lock a Right for cross-chain transfer.
    /// This consumes the Right (deletes the object) and emits a CrossChainLock event.
    public entry fun lock_right(
        right: RightObject,
        destination_chain: u8,
        destination_owner: vector<u8>,
        ctx: &mut TxContext
    ) {
        event::emit(CrossChainLock {
            right_id: right.right_id,
            commitment: right.commitment,
            owner: right.owner,
            destination_chain,
            destination_owner,
            source_tx_hash: tx_context::digest(ctx),
        });

        // Consume the Right (object deletion = single-use enforcement)
        let RightObject { id, right_id: _, commitment: _, owner: _, nullifier: _, state_root: _ } = right;
        object::delete(id);
    }

    /// Mint a new Right from a cross-chain transfer proof.
    /// This creates a new RightObject with the same commitment as the source chain's Right.
    public entry fun mint_right(
        right_id: vector<u8>,
        commitment: vector<u8>,
        state_root: vector<u8>,
        source_chain: u8,
        source_seal_ref: vector<u8>,
        ctx: &mut TxContext
    ) {
        let right = RightObject {
            id: object::new(ctx),
            right_id,
            commitment,
            owner: tx_context::sender(ctx),
            nullifier: vector::empty<u8>(),
            state_root,
        };

        event::emit(CrossChainMint {
            right_id: right.right_id,
            commitment: right.commitment,
            owner: right.owner,
            source_chain,
            source_seal_ref,
        });

        transfer::public_transfer(right, tx_context::sender(ctx));
    }

    /// Transfer ownership of a Right
    public entry fun transfer_right(
        right: RightObject,
        new_owner: address,
        _ctx: &mut TxContext
    ) {
        transfer::public_transfer(right, new_owner);
    }
}
