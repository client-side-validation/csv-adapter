//! Event definitions for CSV Seal program

use anchor_lang::prelude::*;

/// Emitted when the LockRegistry is initialized
#[event]
pub struct RegistryInitialized {
    /// Authority that initialized the registry
    pub authority: Pubkey,
    /// Refund timeout in seconds
    pub refund_timeout: u32,
}

/// Emitted when a new Right is created
#[event]
pub struct RightCreated {
    /// Unique Right identifier
    pub right_id: [u8; 32],
    /// Commitment hash
    pub commitment: [u8; 32],
    /// Owner of the right
    pub owner: Pubkey,
    /// Account address (PDA)
    pub account: Pubkey,
}

/// Emitted when a Right is consumed
#[event]
pub struct RightConsumed {
    /// Unique Right identifier
    pub right_id: [u8; 32],
    /// Commitment hash
    pub commitment: [u8; 32],
    /// Address that consumed the right
    pub consumer: Pubkey,
    /// Account address
    pub account: Pubkey,
}

/// Emitted when a Right is locked for cross-chain transfer
#[event]
pub struct CrossChainLock {
    /// Unique Right identifier
    pub right_id: [u8; 32],
    /// Commitment hash
    pub commitment: [u8; 32],
    /// Owner of the right
    pub owner: Pubkey,
    /// Destination chain ID
    pub destination_chain: u8,
    /// Destination owner (hashed)
    pub destination_owner: [u8; 32],
    /// Source transaction hash
    pub source_tx_hash: [u8; 32],
    /// Lock timestamp (Unix epoch seconds)
    pub locked_at: i64,
}

/// Emitted when a Right is minted from a cross-chain transfer
#[event]
pub struct CrossChainMint {
    /// Unique Right identifier (from source chain)
    pub right_id: [u8; 32],
    /// Commitment hash
    pub commitment: [u8; 32],
    /// Owner of the new right
    pub owner: Pubkey,
    /// Source chain ID
    pub source_chain: u8,
    /// Source chain seal reference
    pub source_seal_ref: [u8; 32],
    /// Account address of the new right
    pub account: Pubkey,
}

/// Emitted when a locked Right is refunded
#[event]
pub struct CrossChainRefund {
    /// Unique Right identifier
    pub right_id: [u8; 32],
    /// Commitment hash
    pub commitment: [u8; 32],
    /// Address that claimed the refund
    pub claimant: Pubkey,
    /// Refund timestamp (Unix epoch seconds)
    pub refunded_at: i64,
}

/// Emitted when a Right is transferred to a new owner
#[event]
pub struct RightTransferred {
    /// Unique Right identifier
    pub right_id: [u8; 32],
    /// Previous owner
    pub from: Pubkey,
    /// New owner
    pub to: Pubkey,
}

/// Emitted when a nullifier is registered
#[event]
pub struct NullifierRegistered {
    /// The nullifier hash
    pub nullifier: [u8; 32],
    /// The Right identifier
    pub right_id: [u8; 32],
}
