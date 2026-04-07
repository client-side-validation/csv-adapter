//! CSV Seal Module for Aptos
//!
//! This Move module provides seal management for the CSV (Client-Side Validation) adapter.
//! Seals are resources that can be deleted once to anchor commitments on-chain.
//!
//! ## Architecture
//!
//! Seals in Aptos are implemented as resources with `key` ability. They are created
//! at a known account address and can be deleted using `delete_seal`, which is a
//! one-time operation (resources can only be deleted once).
//!
//! ## Usage Flow
//!
//! 1. **Seal Creation**: Deploy CSVSeal module, create seal resources
//! 2. **Seal Consumption**: Call `delete_seal` to consume the resource and emit an event
//! 3. **Verification**: Verify the event was emitted with the correct commitment data

module CSVSeal {
    use std::signer;
    use aptos_framework::event;

    /// Anchor event emitted when a seal is consumed.
    ///
    /// This event contains the commitment hash and is used for verification.
    struct AnchorEvent has drop, store {
        commitment: vector<u8>,
        seal_address: address,
        nonce: u64,
    }

    /// Seal resource that can be deleted once.
    ///
    /// The `key` ability means this can be stored at an account address.
    /// The `store` ability means it can be moved between accounts (though we don't do that).
    struct Seal has key, store {
        /// Unique nonce for this seal
        nonce: u64,
    }

    /// Event handle for anchor events.
    ///
    /// Stored at the module account to track all anchor events.
    struct AnchorEventHandle has key {
        events: event::EventHandle<AnchorEvent>,
    }

    /// Create a new seal at the signer's address.
    ///
    /// # Arguments
    /// * `account`: The account creating the seal (must have signer capability)
    /// * `nonce`: Unique nonce for replay resistance
    ///
    /// # Aborts
    /// * If a Seal resource already exists at this address
    #[cmd]
    public fun create_seal(account: &signer, nonce: u64) {
        assert!(!exists<Seal>(signer::address_of(account)), 0);

        move_to(account, Seal { nonce });
    }

    /// Delete a seal and emit an anchor event with the commitment.
    ///
    /// This is the core anchoring operation. The seal resource is deleted
    /// (one-time operation) and an event is emitted with the commitment hash.
    ///
    /// # Arguments
    /// * `account`: The account that owns the seal (must have signer capability)
    /// * `commitment`: The commitment hash to anchor (32 bytes)
    ///
    /// # Aborts
    /// * If no Seal resource exists at this address
    #[cmd]
    public fun delete_seal(account: &signer, commitment: vector<u8>) {
        let seal_addr = signer::address_of(account);
        assert!(exists<Seal>(seal_addr), 1);

        // Extract and destroy the seal resource (one-time operation)
        let seal: Seal = move_from<Seal>(seal_addr);
        let nonce = seal.nonce;

        // Emit the anchor event
        // Note: This requires the AnchorEventHandle to exist at the module account
        // In production, this would be initialized during module deployment
        event::emit_event<AnchorEvent>(
            &mut borrow_global_mut<AnchorEventHandle>(@CSV).events,
            AnchorEvent {
                commitment,
                seal_address: seal_addr,
                nonce,
            },
        );
    }

    /// Check if a seal exists at the given address.
    ///
    /// # Arguments
    /// * `addr`: The address to check
    ///
    /// # Returns
    /// `true` if a Seal resource exists at the address
    public fun seal_exists(addr: address): bool {
        exists<Seal>(addr)
    }

    /// Get the nonce of a seal at the given address.
    ///
    /// # Arguments
    /// * `addr`: The address of the seal
    ///
    /// # Returns
    /// The nonce value of the seal
    ///
    /// # Aborts
    /// * If no Seal resource exists at the address
    public fun get_seal_nonce(addr: address): u64 {
        assert!(exists<Seal>(addr), 1);
        borrow_global<Seal>(addr).nonce
    }

    /// Initialize the module by creating the event handle.
    ///
    /// This should be called once during module deployment.
    ///
    /// # Arguments
    /// * `account`: The module account (must have signer capability)
    #[cmd]
    public fun initialize_module(account: &signer) {
        assert!(!exists<AnchorEventHandle>(signer::address_of(account)), 2);

        move_to(account, AnchorEventHandle {
            events: event::new_event_handle<AnchorEvent>(account),
        });
    }

    #[test_only]
    public fun test_only_create_seal(account: &signer, nonce: u64) {
        create_seal(account, nonce);
    }

    #[test_only]
    public fun test_only_initialize_module(account: &signer) {
        initialize_module(account);
    }
}