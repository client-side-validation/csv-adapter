//! CSV Seal — Cross-Chain Sanad Transfer on Solana
//!
//! This Anchor program implements:
//! - `create_seal()` — Create a new Sanad anchored to a Solana account
//! - `consume_seal()` — Consume a Sanad (single-use enforcement via account closure)
//! - `lock_sanad()` — Lock a Sanad for cross-chain transfer (consumes seal, emits event)
//! - `mint_sanad()` — Mint a new Sanad from a cross-chain transfer proof
//! - `refund_sanad()` — Recover a Sanad after lock timeout (settlement strategy)
//!
//! Architecture:
//! - SanadAccount: PDA storing sanad data (sanad_id, commitment, owner, etc.)
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
    const ASSET_CLASS_PROOF_SANAD: u8 = 3;
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

    /// Create a new Sanad on Solana
    pub fn create_seal(
        ctx: Context<CreateSeal>,
        sanad_id: [u8; 32],
        commitment: [u8; 32],
        state_root: [u8; 32],
    ) -> Result<()> {
        let sanad = &mut ctx.accounts.sanad_account;
        let owner = ctx.accounts.owner.key();

        sanad.owner = owner;
        sanad.sanad_id = sanad_id;
        sanad.commitment = commitment;
        sanad.state_root = state_root;
        sanad.nullifier = [0u8; 32];
        sanad.asset_class = ASSET_CLASS_UNSPECIFIED;
        sanad.asset_id = [0u8; 32];
        sanad.metadata_hash = [0u8; 32];
        sanad.proof_system = PROOF_SYSTEM_UNSPECIFIED;
        sanad.proof_root = [0u8; 32];
        sanad.consumed = false;
        sanad.locked = false;
        sanad.created_at = Clock::get()?.unix_timestamp;
        sanad.bump = ctx.bumps.sanad_account;

        emit!(SanadCreated {
            sanad_id,
            commitment,
            owner,
            account: sanad.key(),
            asset_class: sanad.asset_class,
            asset_id: sanad.asset_id,
            metadata_hash: sanad.metadata_hash,
            proof_system: sanad.proof_system,
            proof_root: sanad.proof_root,
        });

        Ok(())
    }

    /// Consume a Sanad (single-use enforcement)
    /// Marks the sanad as consumed and emits an event
    pub fn consume_seal(ctx: Context<ConsumeSeal>) -> Result<()> {
        let sanad = &mut ctx.accounts.sanad_account;
        let consumer = ctx.accounts.consumer.key();

        require!(!sanad.consumed, CsvError::AlreadyConsumed);

        sanad.consumed = true;

        emit!(SanadConsumed {
            sanad_id: sanad.sanad_id,
            commitment: sanad.commitment,
            consumer,
            account: sanad.key(),
        });

        Ok(())
    }

    /// Lock a Sanad for cross-chain transfer
    /// Consumes the Sanad and records it in the LockRegistry
    pub fn lock_sanad(
        ctx: Context<LockSanad>,
        destination_chain: u8,
        destination_owner: [u8; 32],
    ) -> Result<()> {
        let sanad = &mut ctx.accounts.sanad_account;
        let registry = &mut ctx.accounts.registry;
        let owner = ctx.accounts.owner.key();

        require!(!sanad.consumed, CsvError::AlreadyConsumed);
        require!(!sanad.locked, CsvError::AlreadyLocked);

        let locked_at = Clock::get()?.unix_timestamp;

        // Record the lock in registry
        let lock_record = LockRecord {
            sanad_id: sanad.sanad_id,
            commitment: sanad.commitment,
            owner,
            destination_chain,
            destination_owner,
            asset_class: sanad.asset_class,
            asset_id: sanad.asset_id,
            metadata_hash: sanad.metadata_hash,
            proof_system: sanad.proof_system,
            proof_root: sanad.proof_root,
            locked_at,
            refunded: false,
        };

        registry.locks.push(lock_record);
        registry.lock_count += 1;

        sanad.locked = true;
        sanad.consumed = true;

        // Get transaction signature for source_tx_hash
        // In practice, this would be the actual tx signature
        let source_tx_hash = ctx.accounts.recent_blockhashes.key().to_bytes();

        emit!(CrossChainLock {
            sanad_id: sanad.sanad_id,
            commitment: sanad.commitment,
            owner,
            destination_chain,
            destination_owner,
            source_tx_hash,
            locked_at,
            asset_class: sanad.asset_class,
            asset_id: sanad.asset_id,
            metadata_hash: sanad.metadata_hash,
            proof_system: sanad.proof_system,
            proof_root: sanad.proof_root,
        });

        emit!(SanadMetadataRecorded {
            sanad_id: sanad.sanad_id,
            asset_class: sanad.asset_class,
            asset_id: sanad.asset_id,
            metadata_hash: sanad.metadata_hash,
            proof_system: sanad.proof_system,
            proof_root: sanad.proof_root,
        });

        emit!(SanadConsumed {
            sanad_id: sanad.sanad_id,
            commitment: sanad.commitment,
            consumer: owner,
            account: sanad.key(),
        });

        Ok(())
    }

    /// Mint a new Sanad from a cross-chain transfer proof
    /// Creates a new SanadAccount with the same commitment as the source chain
    pub fn mint_sanad(
        ctx: Context<MintSanad>,
        sanad_id: [u8; 32],
        commitment: [u8; 32],
        state_root: [u8; 32],
        source_chain: u8,
        source_seal_ref: [u8; 32],
    ) -> Result<()> {
        let sanad = &mut ctx.accounts.sanad_account;
        let owner = ctx.accounts.owner.key();

        sanad.owner = owner;
        sanad.sanad_id = sanad_id;
        sanad.commitment = commitment;
        sanad.state_root = state_root;
        sanad.nullifier = [0u8; 32];
        sanad.asset_class = ASSET_CLASS_UNSPECIFIED;
        sanad.asset_id = [0u8; 32];
        sanad.metadata_hash = [0u8; 32];
        sanad.proof_system = PROOF_SYSTEM_UNSPECIFIED;
        sanad.proof_root = [0u8; 32];
        sanad.consumed = false;
        sanad.locked = false;
        sanad.created_at = Clock::get()?.unix_timestamp;
        sanad.bump = ctx.bumps.sanad_account;

        emit!(CrossChainMint {
            sanad_id,
            commitment,
            owner,
            source_chain,
            source_seal_ref,
            account: sanad.key(),
            asset_class: sanad.asset_class,
            asset_id: sanad.asset_id,
            metadata_hash: sanad.metadata_hash,
            proof_system: sanad.proof_system,
            proof_root: sanad.proof_root,
        });

        Ok(())
    }

    /// Refund a Sanad after the lock timeout has elapsed
    /// Re-creates the SanadAccount if the lock has expired and not refunded
    pub fn refund_sanad(
        ctx: Context<RefundSanad>,
        state_root: [u8; 32],
    ) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        let sanad = &mut ctx.accounts.new_sanad_account;
        let claimant = ctx.accounts.claimant.key();
        let sanad_id = ctx.accounts.original_sanad.sanad_id;

        // Cache refund_timeout before mutable borrow
        let refund_timeout = registry.refund_timeout;
        
        // Find the lock record
        let lock_idx = registry
            .locks
            .iter()
            .position(|lock| lock.sanad_id == sanad_id)
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

        // Create new sanad account
        sanad.owner = claimant;
        sanad.sanad_id = lock.sanad_id;
        sanad.commitment = lock.commitment;
        sanad.state_root = state_root;
        sanad.nullifier = [0u8; 32];
        sanad.asset_class = lock.asset_class;
        sanad.asset_id = lock.asset_id;
        sanad.metadata_hash = lock.metadata_hash;
        sanad.proof_system = lock.proof_system;
        sanad.proof_root = lock.proof_root;
        sanad.consumed = false;
        sanad.locked = false;
        sanad.created_at = now;
        sanad.bump = ctx.bumps.new_sanad_account;

        emit!(CrossChainRefund {
            sanad_id: lock.sanad_id,
            commitment: lock.commitment,
            claimant,
            refunded_at: now,
        });

        Ok(())
    }

    /// Attach token/NFT/proof metadata to an unconsumed Sanad.
    pub fn record_sanad_metadata(
        ctx: Context<RecordSanadMetadata>,
        asset_class: u8,
        asset_id: [u8; 32],
        metadata_hash: [u8; 32],
        proof_system: u8,
        proof_root: [u8; 32],
    ) -> Result<()> {
        require!(asset_class <= ASSET_CLASS_PROOF_SANAD, CsvError::InvalidSanadMetadata);
        require!(
            asset_class == ASSET_CLASS_UNSPECIFIED || asset_id != [0u8; 32],
            CsvError::InvalidSanadMetadata
        );
        require!(
            proof_system == PROOF_SYSTEM_UNSPECIFIED || proof_root != [0u8; 32],
            CsvError::InvalidSanadMetadata
        );

        let sanad = &mut ctx.accounts.sanad_account;
        require!(!sanad.consumed, CsvError::AlreadyConsumed);

        sanad.asset_class = asset_class;
        sanad.asset_id = asset_id;
        sanad.metadata_hash = metadata_hash;
        sanad.proof_system = proof_system;
        sanad.proof_root = proof_root;

        emit!(SanadMetadataRecorded {
            sanad_id: sanad.sanad_id,
            asset_class,
            asset_id,
            metadata_hash,
            proof_system,
            proof_root,
        });

        Ok(())
    }

    /// Transfer ownership of a Sanad
    pub fn transfer_sanad(
        ctx: Context<TransferSanad>,
        new_owner: Pubkey,
    ) -> Result<()> {
        let sanad = &mut ctx.accounts.sanad_account;
        let current_owner = ctx.accounts.current_owner.key();

        require!(
            sanad.owner == current_owner,
            CsvError::NotAuthorized
        );
        require!(!sanad.consumed, CsvError::AlreadyConsumed);

        sanad.owner = new_owner;

        emit!(SanadTransferred {
            sanad_id: sanad.sanad_id,
            from: current_owner,
            to: new_owner,
        });

        Ok(())
    }

    /// Register a nullifier for a Sanad (prevents double-spend)
    pub fn register_nullifier(
        ctx: Context<RegisterNullifier>,
        nullifier: [u8; 32],
    ) -> Result<()> {
        let sanad = &mut ctx.accounts.sanad_account;
        
        require!(
            sanad.nullifier == [0u8; 32],
            CsvError::NullifierAlreadyRegistered
        );

        sanad.nullifier = nullifier;

        emit!(NullifierRegistered {
            nullifier,
            sanad_id: sanad.sanad_id,
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
#[instruction(sanad_id: [u8; 32], commitment: [u8; 32], state_root: [u8; 32])]
pub struct CreateSeal<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + SanadAccount::SIZE,
        seeds = [b"sanad", owner.key().as_ref(), &sanad_id],
        bump
    )]
    pub sanad_account: Account<'info, SanadAccount>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

/// Consume a seal accounts
#[derive(Accounts)]
pub struct ConsumeSeal<'info> {
    #[account(
        mut,
        seeds = [b"sanad", sanad_account.owner.as_ref(), &sanad_account.sanad_id],
        bump = sanad_account.bump,
        constraint = sanad_account.owner == consumer.key() @ CsvError::NotAuthorized
    )]
    pub sanad_account: Account<'info, SanadAccount>,
    
    pub consumer: Signer<'info>,
}

/// Lock a sanad accounts
#[derive(Accounts)]
#[instruction(destination_chain: u8, destination_owner: [u8; 32])]
pub struct LockSanad<'info> {
    #[account(
        mut,
        seeds = [b"sanad", sanad_account.owner.as_ref(), &sanad_account.sanad_id],
        bump = sanad_account.bump,
        constraint = sanad_account.owner == owner.key() @ CsvError::NotAuthorized
    )]
    pub sanad_account: Account<'info, SanadAccount>,
    
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

/// Mint a sanad accounts
#[derive(Accounts)]
#[instruction(sanad_id: [u8; 32], commitment: [u8; 32], state_root: [u8; 32], source_chain: u8, source_seal_ref: [u8; 32])]
pub struct MintSanad<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + SanadAccount::SIZE,
        seeds = [b"sanad", owner.key().as_ref(), &sanad_id],
        bump
    )]
    pub sanad_account: Account<'info, SanadAccount>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

/// Refund a sanad accounts
#[derive(Accounts)]
#[instruction(state_root: [u8; 32])]
pub struct RefundSanad<'info> {
    #[account(
        mut,
        seeds = [b"lock_registry"],
        bump = registry.bump
    )]
    pub registry: Account<'info, LockRegistry>,
    
    /// CHECK: Original sanad account that was locked (for verification)
    #[account(
        seeds = [b"sanad", original_sanad.owner.as_ref(), &original_sanad.sanad_id],
        bump = original_sanad.bump
    )]
    pub original_sanad: Account<'info, SanadAccount>,
    
    #[account(
        init,
        payer = claimant,
        space = 8 + SanadAccount::SIZE,
        seeds = [b"sanad", claimant.key().as_ref(), &original_sanad.sanad_id, b"refund"],
        bump
    )]
    pub new_sanad_account: Account<'info, SanadAccount>,
    
    #[account(mut)]
    pub claimant: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

/// Transfer sanad accounts
#[derive(Accounts)]
#[instruction(new_owner: Pubkey)]
pub struct TransferSanad<'info> {
    #[account(
        mut,
        seeds = [b"sanad", sanad_account.owner.as_ref(), &sanad_account.sanad_id],
        bump = sanad_account.bump
    )]
    pub sanad_account: Account<'info, SanadAccount>,
    
    pub current_owner: Signer<'info>,
}

/// Register nullifier accounts
#[derive(Accounts)]
#[instruction(nullifier: [u8; 32])]
pub struct RegisterNullifier<'info> {
    #[account(
        mut,
        seeds = [b"sanad", sanad_account.owner.as_ref(), &sanad_account.sanad_id],
        bump = sanad_account.bump,
        constraint = sanad_account.owner == authority.key() || sanad_account.owner == authority.key() @ CsvError::NotAuthorized
    )]
    pub sanad_account: Account<'info, SanadAccount>,
    
    pub authority: Signer<'info>,
}

/// Record metadata accounts
#[derive(Accounts)]
#[instruction(asset_class: u8, asset_id: [u8; 32], metadata_hash: [u8; 32], proof_system: u8, proof_root: [u8; 32])]
pub struct RecordSanadMetadata<'info> {
    #[account(
        mut,
        seeds = [b"sanad", sanad_account.owner.as_ref(), &sanad_account.sanad_id],
        bump = sanad_account.bump,
        constraint = sanad_account.owner == authority.key() @ CsvError::NotAuthorized
    )]
    pub sanad_account: Account<'info, SanadAccount>,

    pub authority: Signer<'info>,
}
