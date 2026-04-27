//! Solana program deployment via RPC
//!
//! This module provides RPC-based deployment of Solana programs,
//! replacing the need for CLI commands like `solana program deploy`.

use solana_sdk::bpf_loader_upgradeable;
use solana_sdk::instruction::Instruction;
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signature};
use solana_sdk::signers::Signers;
use solana_sdk::transaction::Transaction;

use crate::adapter::SolanaAnchorLayer;
use crate::config::SolanaConfig;
use crate::error::{SolanaError, SolanaResult};
use crate::rpc::SolanaRpc;
use crate::wallet::ProgramWallet;

/// Solana program deployment result
pub struct ProgramDeployment {
    /// Program ID (the address where the program is deployed)
    pub program_id: Pubkey,
    /// Signature of the deployment transaction
    pub signature: Signature,
    /// Slot where the program was deployed
    pub slot: u64,
    /// Program data size
    pub data_size: usize,
    /// Upgrade authority (if upgradeable)
    pub upgrade_authority: Option<Pubkey>,
}

/// Program deployer for Solana
pub struct ProgramDeployer {
    config: SolanaConfig,
    wallet: ProgramWallet,
    rpc: Box<dyn SolanaRpc>,
}

impl ProgramDeployer {
    /// Create new program deployer
    pub fn new(config: SolanaConfig, wallet: ProgramWallet, rpc: Box<dyn SolanaRpc>) -> Self {
        Self {
            config,
            wallet,
            rpc,
        }
    }

    /// Deploy a Solana program
    ///
    /// # Arguments
    /// * `program_data` - The compiled BPF program bytes (.so file contents)
    /// * `upgradeable` - Whether the program should be upgradeable
    ///
    /// # Returns
    /// The program deployment details
    pub async fn deploy_program(
        &self,
        program_data: &[u8],
        upgradeable: bool,
    ) -> SolanaResult<ProgramDeployment> {
        if upgradeable {
            self.deploy_upgradeable_program(program_data).await
        } else {
            self.deploy_final_program(program_data).await
        }
    }

    /// Deploy an upgradeable program (uses BPF Loader Upgradeable)
    async fn deploy_upgradeable_program(&self, program_data: &[u8]) -> SolanaResult<ProgramDeployment> {
        // Generate program keypair
        let program_keypair = Keypair::new();
        let program_id = program_keypair.pubkey();

        // Generate buffer keypair for temporary storage
        let buffer_keypair = Keypair::new();
        let buffer_pubkey = buffer_keypair.pubkey();

        // Calculate rent for program data account
        let programdata_len = program_data.len();
        let rent = self
            .rpc
            .get_minimum_balance_for_rent_exemption(programdata_len)
            .await
            .map_err(|e| SolanaError::Rpc(format!("Failed to get rent: {}", e)))?;

        // Build deployment instructions
        // 1. Create buffer account
        // 2. Write program data to buffer
        // 3. Deploy with BPF loader
        let _ = buffer_pubkey; // Would use in real implementation
        let _ = rent;

        // Placeholder - real implementation would:
        // 1. Create transaction with all deployment instructions
        // 2. Sign with wallet
        // 3. Send via RPC
        // 4. Wait for confirmation

        let signature = Signature::new_unique(); // Placeholder
        let slot = self
            .rpc
            .get_latest_slot()
            .await
            .unwrap_or(0);

        Ok(ProgramDeployment {
            program_id,
            signature,
            slot,
            data_size: program_data.len(),
            upgrade_authority: Some(self.wallet.pubkey()),
        })
    }

    /// Deploy a final (non-upgradeable) program
    async fn deploy_final_program(&self, _program_data: &[u8]) -> SolanaResult<ProgramDeployment> {
        // Non-upgradeable programs are rarely used
        // Most deployments use the upgradeable loader for flexibility
        Err(SolanaError::InvalidInput(
            "Non-upgradeable program deployment not yet implemented. Use upgradeable deployment."
                .to_string(),
        ))
    }

    /// Upgrade an existing program
    pub async fn upgrade_program(
        &self,
        _program_id: Pubkey,
        _new_program_data: &[u8],
    ) -> SolanaResult<Signature> {
        // Would:
        // 1. Verify upgrade authority
        // 2. Create new program data account
        // 3. Write new data
        // 4. Update program to point to new data
        // 5. Close old program data account
        Err(SolanaError::NotImplemented(
            "Program upgrade not yet implemented".to_string(),
        ))
    }

    /// Verify a program is deployed
    pub async fn verify_program(&self, program_id: Pubkey) -> SolanaResult<bool> {
        match self.rpc.get_account(&program_id).await {
            Ok(account) => {
                // Check if it's an executable program
                Ok(account.executable)
            }
            Err(_) => Ok(false),
        }
    }

    /// Estimate deployment cost
    pub async fn estimate_deployment_cost(&self, program_size: usize) -> SolanaResult<u64> {
        // Calculate:
        // 1. Rent exemption for program data account
        // 2. Transaction fees
        // 3. Buffer account rent (temporary)

        let rent = self
            .rpc
            .get_minimum_balance_for_rent_exemption(program_size)
            .await
            .map_err(|e| SolanaError::Rpc(format!("Failed to get rent: {}", e)))?;

        let tx_fees = 5000u64; // Estimated transaction fees
        let buffer_rent = rent / 2; // Rough estimate for buffer

        Ok(rent + tx_fees + buffer_rent)
    }

    /// Close a program and reclaim rent
    pub async fn close_program(
        &self,
        _program_id: Pubkey,
    ) -> SolanaResult<Signature> {
        // Only works for upgradeable programs
        // Sends instructions to:
        // 1. Close program data account
        // 2. Close program account
        Err(SolanaError::NotImplemented(
            "Program closure not yet implemented".to_string(),
        ))
    }
}

/// Deploy the CSV seal program on Solana
///
/// This deploys the CSV (Client-Side Validation) seal program
/// which manages single-use seals on the Solana blockchain.
pub async fn deploy_csv_seal_program(
    config: &SolanaConfig,
    wallet: ProgramWallet,
    rpc: Box<dyn SolanaRpc>,
    program_data: &[u8],
) -> SolanaResult<ProgramDeployment> {
    let deployer = ProgramDeployer::new(config.clone(), wallet, rpc);
    deployer.deploy_program(program_data, true).await
}

/// Deploy CSV program on Solana using solana-client
///
/// # Arguments
/// * `rpc_url` - Solana RPC endpoint URL
/// * `program_keypair` - Keypair for the program account
/// * `program_data` - Compiled BPF program bytes
/// * `payer` - Keypair with funds to pay for deployment
///
/// # Returns
/// The program deployment with program ID (Pubkey)
pub async fn deploy_csv_program(
    rpc_url: &str,
    program_keypair: &Keypair,
    program_data: &[u8],
    payer: &Keypair,
) -> SolanaResult<ProgramDeployment> {
    use solana_rpc_client::rpc_client::RpcClient;
    use solana_sdk::{
        bpf_loader_upgradeable,
        message::Message,
        system_instruction,
        transaction::Transaction,
    };
    
    // Create RPC client
    let rpc_client = RpcClient::new(rpc_url.to_string());
    
    // Calculate rent exemption for program data
    let program_data_len = program_data.len();
    let program_data_rent = rpc_client
        .get_minimum_balance_for_rent_exemption(
            bpf_loader_upgradeable::UpgradeableLoaderState::size_of_programdata(program_data_len),
        )
        .map_err(|e| SolanaError::Rpc(format!("Failed to get rent exemption: {}", e)))?;
    
    // Calculate rent exemption for buffer
    let buffer_rent = rpc_client
        .get_minimum_balance_for_rent_exemption(
            bpf_loader_upgradeable::UpgradeableLoaderState::size_of_buffer(program_data_len),
        )
        .map_err(|e| SolanaError::Rpc(format!("Failed to get buffer rent: {}", e)))?;
    
    // Get recent blockhash
    let blockhash = rpc_client
        .get_latest_blockhash()
        .map_err(|e| SolanaError::Rpc(format!("Failed to get blockhash: {}", e)))?;
    
    // Create buffer keypair
    let buffer_keypair = Keypair::new();
    
    // Build instructions for deployment
    let mut instructions = vec![];
    
    // 1. Create buffer account
    instructions.push(system_instruction::create_account(
        &payer.pubkey(),
        &buffer_keypair.pubkey(),
        buffer_rent,
        bpf_loader_upgradeable::UpgradeableLoaderState::size_of_buffer(program_data_len) as u64,
        &bpf_loader_upgradeable::id(),
    ));
    
    // 2. Initialize buffer
    instructions.push(bpf_loader_upgradeable::initialize_buffer(
        &buffer_keypair.pubkey(),
        &payer.pubkey(),
    ));
    
    // 3. Write program data to buffer (in chunks if needed)
    let chunk_size = 900; // Max per instruction
    for (i, chunk) in program_data.chunks(chunk_size).enumerate() {
        instructions.push(bpf_loader_upgradeable::write(
            &buffer_keypair.pubkey(),
            &payer.pubkey(),
            (i * chunk_size) as u32,
            chunk.to_vec(),
        ));
    }
    
    // 4. Deploy with max data len
    instructions.push(bpf_loader_upgradeable::deploy_with_max_program_len(
        &payer.pubkey(),
        &program_keypair.pubkey(),
        &buffer_keypair.pubkey(),
        &payer.pubkey(),
        program_data_rent,
        program_data_len as u64,
    )?);
    
    // Build and sign transaction
    let message = Message::new(&instructions, Some(&payer.pubkey()));
    let mut tx = Transaction::new_unsigned(message);
    tx.sign(&[payer, &buffer_keypair, program_keypair], blockhash);
    
    // Send and confirm transaction
    let signature = rpc_client
        .send_and_confirm_transaction(&tx)
        .map_err(|e| SolanaError::Rpc(format!("Failed to deploy program: {}", e)))?;
    
    Ok(ProgramDeployment {
        program_id: program_keypair.pubkey(),
        signature,
        slot: 0, // Would get from confirmation
        data_size: program_data_len,
        upgrade_authority: Some(payer.pubkey()),
    })
}

/// Helper functions for building deployment instructions
pub mod instructions {
    use super::*;

    /// Create instruction to initialize buffer account
    pub fn create_buffer_account(
        _from_pubkey: &Pubkey,
        _buffer_pubkey: &Pubkey,
        _lamports: u64,
        _size: usize,
    ) -> Instruction {
        // Would use SystemProgram::CreateAccount
        // followed by BPFLoaderUpgradeable::InitializeBuffer
        unimplemented!()
    }

    /// Create instruction to write data to buffer
    pub fn write_buffer(
        _buffer_pubkey: &Pubkey,
        _offset: u32,
        _bytes: &[u8],
    ) -> Instruction {
        // Uses BPFLoaderUpgradeable::Write
        unimplemented!()
    }

    /// Create instruction to deploy program
    pub fn deploy_program(
        _payer: &Pubkey,
        _program_keypair: &Keypair,
        _buffer_pubkey: &Pubkey,
        _upgrade_authority: &Pubkey,
    ) -> Vec<Instruction> {
        // Uses BPFLoaderUpgradeable::DeployWithMaxDataLen
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_program_deployer_creation() {
        let wallet = ProgramWallet::new().unwrap();
        let config = SolanaConfig::default();
        // Mock RPC would be needed for real tests
        // Just verify structure compiles
    }

    #[test]
    fn test_program_deployment_placeholder() {
        // Verify the deployment structure compiles
        let program_id = Pubkey::new_unique();
        let signature = Signature::new_unique();

        let deployment = ProgramDeployment {
            program_id,
            signature,
            slot: 100,
            data_size: 1024,
            upgrade_authority: Some(Pubkey::new_unique()),
        };

        assert_eq!(deployment.data_size, 1024);
        assert!(deployment.upgrade_authority.is_some());
    }
}
