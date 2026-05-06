//! State definitions for CSV Seal program

use anchor_lang::prelude::*;

/// RightAccount stores the state of a Right on Solana
/// This is a PDA (Program Derived Address) account
#[account]
pub struct RightAccount {
    /// Owner of the right
    pub owner: Pubkey,
    /// Unique Right identifier (preserved across chains)
    pub right_id: [u8; 32],
    /// Commitment hash (preserved across chains)
    pub commitment: [u8; 32],
    /// State root (off-chain state commitment)
    pub state_root: [u8; 32],
    /// Nullifier for this right (for L3 chains that use nullifiers)
    pub nullifier: [u8; 32],
    /// Asset class: 0 unspecified, 1 fungible token, 2 NFT, 3 proof right
    pub asset_class: u8,
    /// Chain-native token mint, NFT collection/item id, or proof family id
    pub asset_id: [u8; 32],
    /// Hash of canonical metadata for token/NFT/proof payloads
    pub metadata_hash: [u8; 32],
    /// Proof system: 0 unspecified, chain/app-specific values above zero
    pub proof_system: u8,
    /// Root/verification key commitment for advanced proof systems
    pub proof_root: [u8; 32],
    /// Whether this right has been consumed
    pub consumed: bool,
    /// Whether this right is locked for cross-chain transfer
    pub locked: bool,
    /// Creation timestamp (Unix epoch seconds)
    pub created_at: i64,
    /// PDA bump seed
    pub bump: u8,
}

impl RightAccount {
    /// Account size for space calculation
    /// 8 (discriminator) + 32 (owner) + 32 (right_id) + 32 (commitment) + 
    /// 32 (state_root) + 32 (nullifier) + metadata/proof fields + flags + timestamp + bump
    pub const SIZE: usize = 32 + 32 + 32 + 32 + 32 + 1 + 32 + 32 + 1 + 32 + 1 + 1 + 8 + 1;
}

/// LockRecord stores information about a locked right for refund purposes
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct LockRecord {
    /// Right identifier
    pub right_id: [u8; 32],
    /// Commitment hash
    pub commitment: [u8; 32],
    /// Original owner
    pub owner: Pubkey,
    /// Destination chain ID
    pub destination_chain: u8,
    /// Destination owner (hashed)
    pub destination_owner: [u8; 32],
    /// Asset class for the locked right
    pub asset_class: u8,
    /// Chain-native asset id
    pub asset_id: [u8; 32],
    /// Canonical metadata hash
    pub metadata_hash: [u8; 32],
    /// Proof system identifier
    pub proof_system: u8,
    /// Proof root or verification-key commitment
    pub proof_root: [u8; 32],
    /// Lock timestamp (Unix epoch seconds)
    pub locked_at: i64,
    /// Whether this lock has been refunded
    pub refunded: bool,
}

impl LockRecord {
    /// Size of LockRecord for space calculation
    pub const SIZE: usize = 32 + 32 + 32 + 1 + 32 + 1 + 32 + 32 + 1 + 32 + 8 + 1;
}

/// LockRegistry tracks all locks for refund support
/// This is a singleton PDA account
#[account]
pub struct LockRegistry {
    /// Authority that can initialize and manage the registry
    pub authority: Pubkey,
    /// Refund timeout in seconds (default: 24 hours = 86400)
    pub refund_timeout: u32,
    /// Total number of locks
    pub lock_count: u32,
    /// List of all lock records
    pub locks: Vec<LockRecord>,
    /// PDA bump seed
    pub bump: u8,
}

impl LockRegistry {
    /// Base size without the vector
    /// 8 (discriminator) + 32 (authority) + 4 (refund_timeout) + 4 (lock_count) + 4 (vec len) + 1 (bump)
    pub const BASE_SIZE: usize = 32 + 4 + 4 + 4 + 1;
    /// Maximum number of locks to prevent account bloat
    pub const MAX_LOCKS: usize = 1000;
    /// Fixed size for initial account creation (with empty vector)
    pub const SIZE: usize = Self::BASE_SIZE;
    
    /// Calculate total size with current locks
    pub fn size(&self) -> usize {
        Self::BASE_SIZE + (self.locks.len() * LockRecord::SIZE)
    }
}
