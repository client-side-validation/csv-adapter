/// CSV Seal — Cross-Chain Right Transfer on Sui
///
/// This module implements:
/// - `create_seal()` — Create a new Right anchored to a Sui object
/// - `consume_seal()` — Consume a Right (single-use enforcement via object deletion)
/// - `lock_right()` — Lock a Right for cross-chain transfer (consumes seal, emits event)
/// - `mint_right()` — Mint a new Right from a cross-chain transfer proof
/// - `refund_right()` — Recover a Right after lock timeout (settlement strategy)

module csv_seal::csv_seal {
    use sui::object::{Self, UID, ID};
    use sui::transfer;
    use sui::tx_context::{Self, TxContext};
    use sui::table;
    use sui::event;
    use std::vector;

    const ASSET_CLASS_UNSPECIFIED: u8 = 0;
    const ASSET_CLASS_PROOF_RIGHT: u8 = 3;
    const PROOF_SYSTEM_UNSPECIFIED: u8 = 0;
    const E_INVALID_METADATA: u64 = 1006;

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
        /// Asset class: 0 unspecified, 1 fungible token, 2 NFT, 3 proof right
        asset_class: u8,
        /// Chain-native token/NFT/proof family id
        asset_id: vector<u8>,
        /// Hash of canonical metadata
        metadata_hash: vector<u8>,
        /// Proof system identifier
        proof_system: u8,
        /// Proof root or verification-key commitment
        proof_root: vector<u8>,
    }

    /// Lock record for refund tracking
    struct LockRecord has store {
        /// Right identifier
        right_id: vector<u8>,
        /// Commitment hash
        commitment: vector<u8>,
        /// Original owner
        owner: address,
        /// Destination chain ID
        destination_chain: u8,
        /// Asset/proof metadata copied from the locked Right
        asset_class: u8,
        asset_id: vector<u8>,
        metadata_hash: vector<u8>,
        proof_system: u8,
        proof_root: vector<u8>,
        /// Lock timestamp (Unix epoch seconds)
        locked_at: u64,
        /// Whether this lock has been refunded
        refunded: bool,
    }

    /// Shared object tracking lock records for settlement
    struct LockRegistry has key {
        id: UID,
        /// Map from right_id to LockRecord
        locks: table::Table<vector<u8>, LockRecord>,
        /// Refund timeout in seconds (24 hours)
        refund_timeout: u64,
    }

    // Event structs (copy + drop)
    struct RightCreated has copy, drop {
        right_id: vector<u8>,
        commitment: vector<u8>,
        owner: address,
        object_id: ID,
        asset_class: u8,
        asset_id: vector<u8>,
        metadata_hash: vector<u8>,
        proof_system: u8,
        proof_root: vector<u8>,
    }

    struct RightConsumed has copy, drop {
        right_id: vector<u8>,
        consumer: address,
    }

    struct CrossChainLock has copy, drop {
        right_id: vector<u8>,
        commitment: vector<u8>,
        owner: address,
        destination_chain: u8,
        destination_owner: vector<u8>,
        source_tx_hash: vector<u8>,
        locked_at: u64,
        asset_class: u8,
        asset_id: vector<u8>,
        metadata_hash: vector<u8>,
        proof_system: u8,
        proof_root: vector<u8>,
    }

    struct CrossChainMint has copy, drop {
        right_id: vector<u8>,
        commitment: vector<u8>,
        owner: address,
        source_chain: u8,
        source_seal_ref: vector<u8>,
        asset_class: u8,
        asset_id: vector<u8>,
        metadata_hash: vector<u8>,
        proof_system: u8,
        proof_root: vector<u8>,
    }

    struct CrossChainRefund has copy, drop {
        right_id: vector<u8>,
        commitment: vector<u8>,
        claimant: address,
        refunded_at: u64,
    }

    struct RightMetadataRecorded has copy, drop {
        right_id: vector<u8>,
        asset_class: u8,
        asset_id: vector<u8>,
        metadata_hash: vector<u8>,
        proof_system: u8,
        proof_root: vector<u8>,
    }

    /// Create a new LockRegistry (called once during deployment)
    public fun create_registry(ctx: &mut TxContext) {
        let registry = LockRegistry {
            id: object::new(ctx),
            locks: table::new(ctx),
            refund_timeout: 86400, // 24 hours
        };
        transfer::share_object(registry);
    }

    /// Create a new Right on Sui
    public fun create_seal(
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
            asset_class: ASSET_CLASS_UNSPECIFIED,
            asset_id: vector::empty<u8>(),
            metadata_hash: vector::empty<u8>(),
            proof_system: PROOF_SYSTEM_UNSPECIFIED,
            proof_root: vector::empty<u8>(),
        };

        event::emit(RightCreated {
            right_id: right.right_id,
            commitment: right.commitment,
            owner: right.owner,
            object_id: object::uid_to_inner(&right.id),
            asset_class: right.asset_class,
            asset_id: right.asset_id,
            metadata_hash: right.metadata_hash,
            proof_system: right.proof_system,
            proof_root: right.proof_root,
        });

        transfer::public_transfer(right, tx_context::sender(ctx));
    }

    /// Record token/NFT/proof metadata for an unconsumed Right.
    public fun record_right_metadata(
        right: &mut RightObject,
        asset_class: u8,
        asset_id: vector<u8>,
        metadata_hash: vector<u8>,
        proof_system: u8,
        proof_root: vector<u8>,
    ) {
        assert!(asset_class <= ASSET_CLASS_PROOF_RIGHT, E_INVALID_METADATA);
        assert!(asset_class == ASSET_CLASS_UNSPECIFIED || vector::length(&asset_id) > 0, E_INVALID_METADATA);
        assert!(proof_system == PROOF_SYSTEM_UNSPECIFIED || vector::length(&proof_root) > 0, E_INVALID_METADATA);

        right.asset_class = asset_class;
        right.asset_id = asset_id;
        right.metadata_hash = metadata_hash;
        right.proof_system = proof_system;
        right.proof_root = proof_root;

        event::emit(RightMetadataRecorded {
            right_id: right.right_id,
            asset_class,
            asset_id: right.asset_id,
            metadata_hash: right.metadata_hash,
            proof_system,
            proof_root: right.proof_root,
        });
    }

    /// Consume a Right (single-use enforcement via object deletion)
    public fun consume_seal(
        right: RightObject,
        ctx: &mut TxContext
    ) {
        event::emit(RightConsumed {
            right_id: right.right_id,
            consumer: tx_context::sender(ctx),
        });

        let RightObject {
            id,
            right_id: _,
            commitment: _,
            owner: _,
            nullifier: _,
            state_root: _,
            asset_class: _,
            asset_id: _,
            metadata_hash: _,
            proof_system: _,
            proof_root: _,
        } = right;
        object::delete(id);
    }

    /// Lock a Right for cross-chain transfer.
    /// This consumes the Right (deletes the object) and emits a CrossChainLock event.
    /// The lock is recorded in the registry for refund support.
    public fun lock_right(
        right: RightObject,
        destination_chain: u8,
        destination_owner: vector<u8>,
        registry: &mut LockRegistry,
        ctx: &mut TxContext
    ) {
        let locked_at = tx_context::epoch_timestamp_ms(ctx) / 1000;

        // Record the lock in the registry
        let lock = LockRecord {
            right_id: right.right_id,
            commitment: right.commitment,
            owner: right.owner,
            destination_chain,
            asset_class: right.asset_class,
            asset_id: right.asset_id,
            metadata_hash: right.metadata_hash,
            proof_system: right.proof_system,
            proof_root: right.proof_root,
            locked_at,
            refunded: false,
        };

        table::add(&mut registry.locks, right.right_id, lock);

        event::emit(CrossChainLock {
            right_id: right.right_id,
            commitment: right.commitment,
            owner: right.owner,
            destination_chain,
            destination_owner,
            source_tx_hash: *tx_context::digest(ctx),
            locked_at,
            asset_class: right.asset_class,
            asset_id: right.asset_id,
            metadata_hash: right.metadata_hash,
            proof_system: right.proof_system,
            proof_root: right.proof_root,
        });

        // Consume the Right (object deletion = single-use enforcement)
        let RightObject {
            id,
            right_id: _,
            commitment: _,
            owner: _,
            nullifier: _,
            state_root: _,
            asset_class: _,
            asset_id: _,
            metadata_hash: _,
            proof_system: _,
            proof_root: _,
        } = right;
        object::delete(id);
    }

    /// Mint a new Right from a cross-chain transfer proof.
    /// This creates a new RightObject with the same commitment as the source chain's Right.
    public fun mint_right(
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
            asset_class: ASSET_CLASS_UNSPECIFIED,
            asset_id: vector::empty<u8>(),
            metadata_hash: vector::empty<u8>(),
            proof_system: PROOF_SYSTEM_UNSPECIFIED,
            proof_root: vector::empty<u8>(),
        };

        event::emit(CrossChainMint {
            right_id: right.right_id,
            commitment: right.commitment,
            owner: right.owner,
            source_chain,
            source_seal_ref,
            asset_class: right.asset_class,
            asset_id: right.asset_id,
            metadata_hash: right.metadata_hash,
            proof_system: right.proof_system,
            proof_root: right.proof_root,
        });

        transfer::public_transfer(right, tx_context::sender(ctx));
    }

    /// Refund a Right after the lock timeout has elapsed.
    /// This re-creates the RightObject if:
    /// 1. The lock was recorded in the registry
    /// 2. The REFUND_TIMEOUT has elapsed
    /// 3. The Right has not already been refunded
    public fun refund_right(
        right_id: vector<u8>,
        state_root: vector<u8>,
        registry: &mut LockRegistry,
        ctx: &mut TxContext
    ) {
        assert!(
            table::contains(&registry.locks, right_id),
            1003 // Lock not found in registry
        );

        let lock = table::borrow_mut(&mut registry.locks, right_id);
        let now = tx_context::epoch_timestamp_ms(ctx) / 1000;

        // Verify timeout has elapsed
        assert!(
            now >= lock.locked_at + registry.refund_timeout,
            1004 // Refund timeout not yet expired
        );

        // Verify not already refunded
        assert!(!lock.refunded, 1005); // Already refunded

        // Mark as refunded
        lock.refunded = true;

        // Re-create the RightObject
        let right = RightObject {
            id: object::new(ctx),
            right_id: lock.right_id,
            commitment: lock.commitment,
            owner: tx_context::sender(ctx),
            nullifier: vector::empty<u8>(),
            state_root,
            asset_class: lock.asset_class,
            asset_id: lock.asset_id,
            metadata_hash: lock.metadata_hash,
            proof_system: lock.proof_system,
            proof_root: lock.proof_root,
        };

        event::emit(CrossChainRefund {
            right_id: right.right_id,
            commitment: right.commitment,
            claimant: tx_context::sender(ctx),
            refunded_at: now,
        });

        transfer::public_transfer(right, tx_context::sender(ctx));
    }

    /// Transfer ownership of a Right
    public fun transfer_right(
        right: RightObject,
        new_owner: address,
        _ctx: &mut TxContext
    ) {
        transfer::public_transfer(right, new_owner);
    }

    /// Get lock info (for off-chain refund verification)
    public fun get_lock_info(
        registry: &LockRegistry,
        right_id: vector<u8>,
    ): (vector<u8>, u64, bool) {
        let lock = table::borrow(&registry.locks, right_id);
        (lock.commitment, lock.locked_at, lock.refunded)
    }

    /// Check if refund is available for a Right
    public fun can_refund(
        registry: &LockRegistry,
        right_id: vector<u8>,
        now: u64,
    ): bool {
        if (!table::contains(&registry.locks, right_id)) {
            return false
        };
        let lock = table::borrow(&registry.locks, right_id);
        !lock.refunded && now >= lock.locked_at + registry.refund_timeout
    }
}
