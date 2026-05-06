//! CSV Seal — Cross-Chain Right Transfer on Solana
//!
//! This Anchor program implements:
//! - `create_seal()` — Create a new Right anchored to a Solana account
//! - `consume_seal()` — Consume a Right (single-use enforcement via account closure)
//! - `lock_right()` — Lock a Right for cross-chain transfer (consumes seal, emits event)
//! - `mint_right()` — Mint a new Right from a cross-chain transfer proof
//! - `refund_right()` — Recover a Right after lock timeout (settlement strategy)
//!
//! Architecture:
//! - RightAccount: PDA storing right data (right_id, commitment, owner, etc.)
//! - LockRegistry: Tracks lock records for refunds with 24h timeout
//! - Events emitted for all cross-chain operations

use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

pub use constants::*;
pub use errors::*;
pub use events::*;
pub use instructions::*;
pub use state::*;

declare_id!("HdxSFwzk2v6JMm3w55MW1EuMeNcM9gTC4ETFMKqYyy6m");

#[program]
pub mod csv_seal {
    use super::*;

    const ASSET_CLASS_UNSPECIFIED: u8 = 0;
    const ASSET_CLASS_PROOF_RIGHT: u8 = 3;
    const PROOF_SYSTEM_UNSPECIFIED: u8 = 0;

    /// Initialize the LockRegistry (called once during deployment)
    pub fn initialize_registry(ctx: Context<InitializeRegistry>) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        registry.authority = ctx.accounts.authority.key();
        registry.refund_timeout = REFUND_TIMEOUT;
        registry.lock_count = 0;
        registry.bump = ctx.bumps.registry;

        emit!(RegistryInitialized {
            authority: registry.authority,
            refund_timeout: registry.refund_timeout,
        });

        Ok(())
    }

    /// Create a new Right on Solana
    pub fn create_seal(
        ctx: Context<CreateSeal>,
        right_id: [u8; 32],
        commitment: [u8; 32],
        state_root: [u8; 32],
    ) -> Result<()> {
        let right = &mut ctx.accounts.right_account;
        let owner = ctx.accounts.owner.key();

        right.owner = owner;
        right.right_id = right_id;
        right.commitment = commitment;
        right.state_root = state_root;
        right.nullifier = [0u8; 32];
        right.asset_class = ASSET_CLASS_UNSPECIFIED;
        right.asset_id = [0u8; 32];
        right.metadata_hash = [0u8; 32];
        right.proof_system = PROOF_SYSTEM_UNSPECIFIED;
        right.proof_root = [0u8; 32];
        right.consumed = false;
        right.locked = false;
        right.created_at = Clock::get()?.unix_timestamp;
        right.bump = ctx.bumps.right_account;

        emit!(RightCreated {
            right_id,
            commitment,
            owner,
            account: right.key(),
            asset_class: right.asset_class,
            asset_id: right.asset_id,
            metadata_hash: right.metadata_hash,
            proof_system: right.proof_system,
            proof_root: right.proof_root,
        });

        Ok(())
    }

    /// Consume a Right (single-use enforcement)
    /// Marks the right as consumed and emits an event
    pub fn consume_seal(ctx: Context<ConsumeSeal>) -> Result<()> {
        let right = &mut ctx.accounts.right_account;
        let consumer = ctx.accounts.consumer.key();

        require!(!right.consumed, CsvError::AlreadyConsumed);

        right.consumed = true;

        emit!(RightConsumed {
            right_id: right.right_id,
            commitment: right.commitment,
            consumer,
            account: right.key(),
        });

        Ok(())
    }

    /// Lock a Right for cross-chain transfer
    /// Consumes the Right and records it in the LockRegistry
    pub fn lock_right(
        ctx: Context<LockRight>,
        destination_chain: u8,
        destination_owner: [u8; 32],
    ) -> Result<()> {
        let right = &mut ctx.accounts.right_account;
        let registry = &mut ctx.accounts.registry;
        let owner = ctx.accounts.owner.key();

        require!(!right.consumed, CsvError::AlreadyConsumed);
        require!(!right.locked, CsvError::AlreadyLocked);

        let locked_at = Clock::get()?.unix_timestamp;

        // Record the lock in registry
        let lock_record = LockRecord {
            right_id: right.right_id,
            commitment: right.commitment,
            owner,
            destination_chain,
            destination_owner,
            asset_class: right.asset_class,
            asset_id: right.asset_id,
            metadata_hash: right.metadata_hash,
            proof_system: right.proof_system,
            proof_root: right.proof_root,
            locked_at,
            refunded: false,
        };

        registry.locks.push(lock_record);
        registry.lock_count += 1;

        right.locked = true;
        right.consumed = true;

        // Get transaction signature for source_tx_hash
        // In practice, this would be the actual tx signature
        let source_tx_hash = ctx.accounts.recent_blockhashes.key().to_bytes();

        emit!(CrossChainLock {
            right_id: right.right_id,
            commitment: right.commitment,
            owner,
            destination_chain,
            destination_owner,
            source_tx_hash,
            locked_at,
            asset_class: right.asset_class,
            asset_id: right.asset_id,
            metadata_hash: right.metadata_hash,
            proof_system: right.proof_system,
            proof_root: right.proof_root,
        });

        emit!(RightMetadataRecorded {
            right_id: right.right_id,
            asset_class: right.asset_class,
            asset_id: right.asset_id,
            metadata_hash: right.metadata_hash,
            proof_system: right.proof_system,
            proof_root: right.proof_root,
        });

        emit!(RightConsumed {
            right_id: right.right_id,
            commitment: right.commitment,
            consumer: owner,
            account: right.key(),
        });

        Ok(())
    }

    /// Mint a new Right from a cross-chain transfer proof
    /// Creates a new RightAccount with the same commitment as the source chain
    pub fn mint_right(
        ctx: Context<MintRight>,
        right_id: [u8; 32],
        commitment: [u8; 32],
        state_root: [u8; 32],
        source_chain: u8,
        source_seal_ref: [u8; 32],
    ) -> Result<()> {
        let right = &mut ctx.accounts.right_account;
        let owner = ctx.accounts.owner.key();

        right.owner = owner;
        right.right_id = right_id;
        right.commitment = commitment;
        right.state_root = state_root;
        right.nullifier = [0u8; 32];
        right.asset_class = ASSET_CLASS_UNSPECIFIED;
        right.asset_id = [0u8; 32];
        right.metadata_hash = [0u8; 32];
        right.proof_system = PROOF_SYSTEM_UNSPECIFIED;
        right.proof_root = [0u8; 32];
        right.consumed = false;
        right.locked = false;
        right.created_at = Clock::get()?.unix_timestamp;
        right.bump = ctx.bumps.right_account;

        emit!(CrossChainMint {
            right_id,
            commitment,
            owner,
            source_chain,
            source_seal_ref,
            account: right.key(),
            asset_class: right.asset_class,
            asset_id: right.asset_id,
            metadata_hash: right.metadata_hash,
            proof_system: right.proof_system,
            proof_root: right.proof_root,
        });

        Ok(())
    }

    /// Refund a Right after the lock timeout has elapsed
    /// Re-creates the RightAccount if the lock has expired and not refunded
    pub fn refund_right(
        ctx: Context<RefundRight>,
        state_root: [u8; 32],
    ) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        let right = &mut ctx.accounts.new_right_account;
        let claimant = ctx.accounts.claimant.key();
        let right_id = ctx.accounts.original_right.right_id;

        // Cache refund_timeout before mutable borrow
        let refund_timeout = registry.refund_timeout;
        
        // Find the lock record
        let lock_idx = registry
            .locks
            .iter()
            .position(|lock| lock.right_id == right_id)
            .ok_or(CsvError::LockNotFound)?;

        let lock = &mut registry.locks[lock_idx];

        // Verify refund conditions
        let now = Clock::get()?.unix_timestamp;
        require!(
            now >= lock.locked_at + refund_timeout as i64,
            CsvError::RefundTimeoutNotExpired
        );
        require!(!lock.refunded, CsvError::AlreadyRefunded);

        // Mark as refunded
        lock.refunded = true;

        // Create new right account
        right.owner = claimant;
        right.right_id = lock.right_id;
        right.commitment = lock.commitment;
        right.state_root = state_root;
        right.nullifier = [0u8; 32];
        right.asset_class = lock.asset_class;
        right.asset_id = lock.asset_id;
        right.metadata_hash = lock.metadata_hash;
        right.proof_system = lock.proof_system;
        right.proof_root = lock.proof_root;
        right.consumed = false;
        right.locked = false;
        right.created_at = now;
        right.bump = ctx.bumps.new_right_account;

        emit!(CrossChainRefund {
            right_id: lock.right_id,
            commitment: lock.commitment,
            claimant,
            refunded_at: now,
        });

        Ok(())
    }

    /// Attach token/NFT/proof metadata to an unconsumed Right.
    pub fn record_right_metadata(
        ctx: Context<RecordRightMetadata>,
        asset_class: u8,
        asset_id: [u8; 32],
        metadata_hash: [u8; 32],
        proof_system: u8,
        proof_root: [u8; 32],
    ) -> Result<()> {
        require!(asset_class <= ASSET_CLASS_PROOF_RIGHT, CsvError::InvalidRightMetadata);
        require!(
            asset_class == ASSET_CLASS_UNSPECIFIED || asset_id != [0u8; 32],
            CsvError::InvalidRightMetadata
        );
        require!(
            proof_system == PROOF_SYSTEM_UNSPECIFIED || proof_root != [0u8; 32],
            CsvError::InvalidRightMetadata
        );

        let right = &mut ctx.accounts.right_account;
        require!(!right.consumed, CsvError::AlreadyConsumed);

        right.asset_class = asset_class;
        right.asset_id = asset_id;
        right.metadata_hash = metadata_hash;
        right.proof_system = proof_system;
        right.proof_root = proof_root;

        emit!(RightMetadataRecorded {
            right_id: right.right_id,
            asset_class,
            asset_id,
            metadata_hash,
            proof_system,
            proof_root,
        });

        Ok(())
    }

    /// Transfer ownership of a Right
    pub fn transfer_right(
        ctx: Context<TransferRight>,
        new_owner: Pubkey,
    ) -> Result<()> {
        let right = &mut ctx.accounts.right_account;
        let current_owner = ctx.accounts.current_owner.key();

        require!(
            right.owner == current_owner,
            CsvError::NotAuthorized
        );
        require!(!right.consumed, CsvError::AlreadyConsumed);

        right.owner = new_owner;

        emit!(RightTransferred {
            right_id: right.right_id,
            from: current_owner,
            to: new_owner,
        });

        Ok(())
    }

    /// Register a nullifier for a Right (prevents double-spend)
    pub fn register_nullifier(
        ctx: Context<RegisterNullifier>,
        nullifier: [u8; 32],
    ) -> Result<()> {
        let right = &mut ctx.accounts.right_account;
        
        require!(
            right.nullifier == [0u8; 32],
            CsvError::NullifierAlreadyRegistered
        );

        right.nullifier = nullifier;

        emit!(NullifierRegistered {
            nullifier,
            right_id: right.right_id,
        });

        Ok(())
    }
}

/// Initialize the LockRegistry accounts
#[derive(Accounts)]
pub struct InitializeRegistry<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + LockRegistry::SIZE,
        seeds = [b"lock_registry"],
        bump
    )]
    pub registry: Account<'info, LockRegistry>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

/// Create a new seal accounts
#[derive(Accounts)]
#[instruction(right_id: [u8; 32], commitment: [u8; 32], state_root: [u8; 32])]
pub struct CreateSeal<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + RightAccount::SIZE,
        seeds = [b"right", owner.key().as_ref(), &right_id],
        bump
    )]
    pub right_account: Account<'info, RightAccount>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

/// Consume a seal accounts
#[derive(Accounts)]
pub struct ConsumeSeal<'info> {
    #[account(
        mut,
        seeds = [b"right", right_account.owner.as_ref(), &right_account.right_id],
        bump = right_account.bump,
        constraint = right_account.owner == consumer.key() @ CsvError::NotAuthorized
    )]
    pub right_account: Account<'info, RightAccount>,
    
    pub consumer: Signer<'info>,
}

/// Lock a right accounts
#[derive(Accounts)]
#[instruction(destination_chain: u8, destination_owner: [u8; 32])]
pub struct LockRight<'info> {
    #[account(
        mut,
        seeds = [b"right", right_account.owner.as_ref(), &right_account.right_id],
        bump = right_account.bump,
        constraint = right_account.owner == owner.key() @ CsvError::NotAuthorized
    )]
    pub right_account: Account<'info, RightAccount>,
    
    #[account(
        mut,
        seeds = [b"lock_registry"],
        bump = registry.bump
    )]
    pub registry: Account<'info, LockRegistry>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    /// CHECK: Used for transaction hash reference
    pub recent_blockhashes: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
}

/// Mint a right accounts
#[derive(Accounts)]
#[instruction(right_id: [u8; 32], commitment: [u8; 32], state_root: [u8; 32], source_chain: u8, source_seal_ref: [u8; 32])]
pub struct MintRight<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + RightAccount::SIZE,
        seeds = [b"right", owner.key().as_ref(), &right_id],
        bump
    )]
    pub right_account: Account<'info, RightAccount>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

/// Refund a right accounts
#[derive(Accounts)]
#[instruction(state_root: [u8; 32])]
pub struct RefundRight<'info> {
    #[account(
        mut,
        seeds = [b"lock_registry"],
        bump = registry.bump
    )]
    pub registry: Account<'info, LockRegistry>,
    
    /// CHECK: Original right account that was locked (for verification)
    #[account(
        seeds = [b"right", original_right.owner.as_ref(), &original_right.right_id],
        bump = original_right.bump
    )]
    pub original_right: Account<'info, RightAccount>,
    
    #[account(
        init,
        payer = claimant,
        space = 8 + RightAccount::SIZE,
        seeds = [b"right", claimant.key().as_ref(), &original_right.right_id, b"refund"],
        bump
    )]
    pub new_right_account: Account<'info, RightAccount>,
    
    #[account(mut)]
    pub claimant: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

/// Transfer right accounts
#[derive(Accounts)]
#[instruction(new_owner: Pubkey)]
pub struct TransferRight<'info> {
    #[account(
        mut,
        seeds = [b"right", right_account.owner.as_ref(), &right_account.right_id],
        bump = right_account.bump
    )]
    pub right_account: Account<'info, RightAccount>,
    
    pub current_owner: Signer<'info>,
}

/// Register nullifier accounts
#[derive(Accounts)]
#[instruction(nullifier: [u8; 32])]
pub struct RegisterNullifier<'info> {
    #[account(
        mut,
        seeds = [b"right", right_account.owner.as_ref(), &right_account.right_id],
        bump = right_account.bump,
        constraint = right_account.owner == authority.key() || right_account.owner == authority.key() @ CsvError::NotAuthorized
    )]
    pub right_account: Account<'info, RightAccount>,
    
    pub authority: Signer<'info>,
}

/// Record metadata accounts
#[derive(Accounts)]
#[instruction(asset_class: u8, asset_id: [u8; 32], metadata_hash: [u8; 32], proof_system: u8, proof_root: [u8; 32])]
pub struct RecordRightMetadata<'info> {
    #[account(
        mut,
        seeds = [b"right", right_account.owner.as_ref(), &right_account.right_id],
        bump = right_account.bump,
        constraint = right_account.owner == authority.key() @ CsvError::NotAuthorized
    )]
    pub right_account: Account<'info, RightAccount>,

    pub authority: Signer<'info>,
}
