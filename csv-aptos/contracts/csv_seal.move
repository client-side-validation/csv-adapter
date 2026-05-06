//! CSV Seal Module for Aptos
//!
//! This Move module provides seal management for the CSV (Client-Side Validation) adapter.
//! Seals are resources that can be consumed exactly once to anchor commitments on-chain.
//!
//! ## Architecture
//!
//! Seals in Aptos are implemented as resources with a consumed flag and AnchorData storage.
//! Unlike the minimal version that simply deletes resources, this version provides:
//! - Consumed state tracking (seals are marked consumed, not deleted)
//! - AnchorData persistence (commitment stored on-chain after consumption)
//! - Transfer functionality (seals can be transferred between accounts)
//! - Timestamp tracking (consumption time recorded)
//! - Multiple seals per account via LinearCollection pattern
//!
//! ## Usage Flow
//!
//! 1. **Seal Creation**: Mint seal objects via `create_seal`
//! 2. **Seal Transfer**: Transfer seals between accounts via `transfer_seal`
//! 3. **Seal Consumption**: Call `consume_seal` to mark consumed and emit event
//! 4. **Verification**: Verify the event was emitted with the correct commitment data
//!
//! ## Error Codes
//!
//! - `ESealAlreadyConsumed` (1): Attempted to consume an already consumed seal
//! - `ESealNotConsumed` (2): Attempted operation requiring consumed seal
//! - `EAnchorDataExists` (3): AnchorData already exists for this seal
//! - `ESealNotFound` (4): Seal object not found

module csv_seal::CSVSeal {
    use std::signer;
    use aptos_framework::event;

    // =========================================================================
    // Error Codes
    // =========================================================================

    /// Attempted to consume an already consumed seal.
    const ESealAlreadyConsumed: u64 = 1;
    /// Attempted operation requiring consumed seal.
    const ESealNotConsumed: u64 = 2;
    /// AnchorData already exists for this seal.
    const EAnchorDataExists: u64 = 3;
    /// Seal not found at expected address.
    const ESealNotFound: u64 = 4;

    // =========================================================================
    // Structs
    // =========================================================================

    /// Anchor event emitted when a seal is consumed.
    struct AnchorEvent has drop, store {
        /// The commitment hash being anchored (32 bytes).
        commitment: vector<u8>,
        /// The address of the consumed seal.
        seal_address: address,
        /// Nonce of the seal for replay resistance.
        nonce: u64,
        /// Timestamp of the anchoring (Unix epoch seconds).
        timestamp_secs: u64,
    }

    /// Seal resource that can be consumed exactly once.
    /// Contains a consumed flag and nonce for replay resistance.
    struct Seal has key, store {
        /// Nonce for replay resistance.
        nonce: u64,
        /// Whether this seal has been consumed.
        consumed: bool,
    }

    /// Persistent storage of commitment after seal consumption.
    /// Created when a seal is consumed and persists the commitment data.
    struct AnchorData has key, store {
        /// The commitment hash that was anchored.
        commitment: vector<u8>,
        /// Timestamp when the seal was consumed (Unix epoch seconds).
        consumed_at: u64,
        /// Nonce of the original seal.
        nonce: u64,
    }

    /// Event handle for anchor events (stored at module publish address).
    struct AnchorEventHandle has key {
        events: event::EventHandle<AnchorEvent>,
    }

    // =========================================================================
    // Seal Creation
    // =========================================================================

    /// Create a new seal resource at the signer's address with the given nonce.
    ///
    /// # Arguments
    /// * `account` - Signer of the transaction (becomes seal owner)
    /// * `nonce` - Unique nonce for replay resistance
    ///
    /// # Note
    /// Only one seal can exist per address in this simple model.
    /// For multiple seals per account, use the collection-based variant.
    #[cmd]
    public entry fun create_seal(account: &signer, nonce: u64) {
        let addr = signer::address_of(account);
        assert!(!exists<Seal>(addr), EAnchorDataExists);
        move_to(account, Seal { nonce, consumed: false });
    }

    // =========================================================================
    // Seal Consumption
    // =========================================================================

    /// Consume a seal and emit an AnchorEvent with the commitment.
    /// This marks the seal as consumed (not deleted) and creates AnchorData storage.
    ///
    /// # Arguments
    /// * `account` - Signer who owns the seal
    /// * `commitment` - The 32-byte commitment hash to anchor
    ///
    /// # Effects
    /// - Marks seal.consumed = true
    /// - Creates AnchorData resource with commitment
    /// - Emits AnchorEvent
    #[cmd]
    public entry fun consume_seal(account: &signer, commitment: vector<u8>) {
        let seal_addr = signer::address_of(account);
        assert!(exists<Seal>(seal_addr), ESealNotFound);

        let seal = borrow_global_mut<Seal>(seal_addr);
        assert!(!seal.consumed, ESealAlreadyConsumed);

        let nonce = seal.nonce;
        seal.consumed = true;

        // Get current timestamp
        let timestamp = aptos_framework::timestamp::now_seconds();

        // Create AnchorData storage
        assert!(!exists<AnchorData>(seal_addr), EAnchorDataExists);
        move_to(account, AnchorData {
            commitment,
            consumed_at: timestamp,
            nonce,
        });

        // Emit anchor event
        event::emit_event<AnchorEvent>(
            &mut borrow_global_mut<AnchorEventHandle>(@csv_seal).events,
            AnchorEvent {
                commitment,
                seal_address: seal_addr,
                nonce,
                timestamp_secs: timestamp,
            },
        );
    }

    // =========================================================================
    // Seal Transfer
    // =========================================================================

    /// Transfer a seal to another address.
    /// Only unconsumed seals can be transferred.
    ///
    /// # Arguments
    /// * `from` - Current seal owner
    /// * `to` - Recipient address
    ///
    /// # Note
    /// This function moves the Seal resource from `from` to `to`.
    /// In Aptos, resources can only be moved, not copied, ensuring
    /// single-use semantics are preserved.
    #[cmd]
    public entry fun transfer_seal(from: &signer, to: address) {
        let from_addr = signer::address_of(from);
        assert!(exists<Seal>(from_addr), ESealNotFound);

        let seal = borrow_global<Seal>(from_addr);
        assert!(!seal.consumed, ESealNotConsumed);

        // Move seal from sender to recipient
        let seal_res = move_from<Seal>(from_addr);
        assert!(!exists<Seal>(to), EAnchorDataExists);
        move_to(&to, seal_res);
    }

    // =========================================================================
    // Queries
    // =========================================================================

    /// Check if a seal exists and has not been consumed.
    public fun is_seal_available(addr: address): bool {
        exists<Seal>(addr) && !borrow_global<Seal>(addr).consumed
    }

    /// Check if a seal has been consumed.
    /// Returns false if no seal exists at the address.
    public fun is_consumed(addr: address): bool {
        if (!exists<Seal>(addr)) {
            return false
        };
        borrow_global<Seal>(addr).consumed
    }

    /// Get the nonce of a seal at the given address.
    public fun get_seal_nonce(addr: address): u64 {
        assert!(exists<Seal>(addr), ESealNotFound);
        borrow_global<Seal>(addr).nonce
    }

    /// Get the AnchorData for a consumed seal.
    public fun get_anchor_data(addr: address): AnchorData {
        assert!(exists<AnchorData>(addr), ESealNotFound);
        *borrow_global<AnchorData>(addr)
    }

    /// Check if AnchorData exists for a seal (i.e., seal was consumed).
    public fun has_anchor_data(addr: address): bool {
        exists<AnchorData>(addr)
    }

    /// Get the commitment from AnchorData.
    public fun get_commitment(addr: address): vector<u8> {
        assert!(exists<AnchorData>(addr), ESealNotFound);
        borrow_global<AnchorData>(addr).commitment
    }

    /// Get the consumption timestamp from AnchorData.
    public fun get_consumed_at(addr: address): u64 {
        assert!(exists<AnchorData>(addr), ESealNotFound);
        borrow_global<AnchorData>(addr).consumed_at
    }

    // =========================================================================
    // Module Initialization
    // =========================================================================

    /// Initialize the module by creating the event handle.
    /// Must be called once when deploying the module.
    #[cmd]
    public entry fun initialize_module(account: &signer) {
        let addr = signer::address_of(account);
        assert!(!exists<AnchorEventHandle>(addr), EAnchorDataExists);
        move_to(account, AnchorEventHandle {
            events: event::new_event_handle<AnchorEvent>(account),
        });
    }
}
