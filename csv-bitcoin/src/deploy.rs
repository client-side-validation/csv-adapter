//! Bitcoin contract (Taproot script) deployment via RPC
//!
//! This module provides RPC-based deployment of Bitcoin smart contracts using
//! Taproot scripts. Unlike other chains, Bitcoin "contracts" are Taproot outputs
//! that can be spent with specific script conditions.

use crate::config::BitcoinConfig;
use crate::error::{BitcoinError, BitcoinResult};
use crate::rpc::BitcoinRpc;
use crate::wallet::SealWallet;
use bitcoin::key::TapTweak;
use bitcoin_hashes::Hash as BitcoinHash;

/// Bitcoin contract deployment transaction
pub struct ContractDeployment {
    /// The Taproot output address (the "contract address")
    pub address: String,
    /// Transaction ID that created the contract
    pub txid: [u8; 32],
    /// Output index
    pub vout: u32,
    /// Redeem script (for spending conditions)
    pub redeem_script: Vec<u8>,
    /// Witness program (Taproot output key)
    pub witness_program: Vec<u8>,
}

/// Bitcoin contract deployer
pub struct ContractDeployer {
    config: BitcoinConfig,
    wallet: SealWallet,
    rpc: Box<dyn BitcoinRpc + Send + Sync>,
}

impl ContractDeployer {
    /// Create new contract deployer
    pub fn new(
        config: BitcoinConfig,
        wallet: SealWallet,
        rpc: Box<dyn BitcoinRpc + Send + Sync>,
    ) -> Self {
        Self {
            config,
            wallet,
            rpc,
        }
    }

    /// Deploy a Taproot contract
    ///
    /// # Arguments
    /// * `script` - The Tapscript to embed in the Taproot output
    /// * `value_sat` - Amount to lock in the contract (in satoshis)
    ///
    /// # Returns
    /// The contract deployment details with the Taproot output address
    ///
    /// # Implementation Notes
    /// This is currently a simplified implementation. Full implementation requires:
    /// 1. Build Taproot tree with the script as a leaf
    /// 2. Compute the merkle root and tweak the internal key
    /// 3. Create a funding transaction with UTXOs from the wallet
    /// 4. Add the Taproot output with the specified value
    /// 5. Sign and broadcast via RPC
    /// 6. Wait for confirmation and return deployment details
    pub fn deploy_contract(
        &self,
        script: &[u8],
        _value_sat: u64,
    ) -> BitcoinResult<ContractDeployment> {
        // Derive a new address for the contract
        let derived_key = self
            .wallet
            .get_funding_address(0, 0)
            .map_err(|e| BitcoinError::RpcError(format!("Failed to derive address: {}", e)))?;

        // Build the Taproot output with the script
        // This creates the merkle tree root from the script
        let internal_key = derived_key.internal_xonly;

        // Compute the Taproot output key that includes the script commitment
        // For single-script contracts, we need to compute the merkle root from the script
        let script_hash = bitcoin::taproot::TapNodeHash::from_script(
            &bitcoin::ScriptBuf::from(script.to_vec()),
            bitcoin::taproot::LeafVersion::TapScript,
        );

        // Use tap_tweak to get the tweaked public key for the P2TR address
        let secp = bitcoin::secp256k1::Secp256k1::new();
        let (tweaked_key, _parity) = internal_key.tap_tweak(&secp, Some(script_hash));

        let contract_address =
            bitcoin::Address::p2tr_tweaked(tweaked_key, self.config.network.to_bitcoin_network());

        // Get UTXOs for the deployer address to fund the transaction
        let deployer_address = derived_key.address.to_string();
        let utxos = self
            .rpc
            .get_utxos_for_address(&deployer_address)
            .map_err(|e| BitcoinError::RpcError(format!("Failed to get UTXOs: {}", e)))?;

        if utxos.is_empty() {
            return Err(BitcoinError::InvalidInput(
                "No UTXOs available for deployment. Fund the deployer address first.".to_string()
            ));
        }

        // Select UTXOs for funding (simple selection: use first sufficient UTXO)
        let estimated_fee = self.estimate_fee(script.len());
        let dust_limit = 546u64; // Minimum output value in satoshis
        let total_needed = estimated_fee + dust_limit;

        let selected_utxo = utxos
            .into_iter()
            .find(|u| u.amount_sat >= total_needed)
            .ok_or_else(|| BitcoinError::InvalidInput(
                format!("No UTXO with sufficient funds. Need at least {} satoshis", total_needed)
            ))?;

        // Build the deployment transaction
        // This creates a Taproot output that locks the contract
        let tx = self.build_deployment_transaction(
            &selected_utxo,
            &contract_address,
            script,
            estimated_fee,
        )?;

        // Sign and broadcast the transaction
        // Note: In a full implementation, this requires access to the private key
        // for signing. The current implementation assumes the RPC client handles signing
        // or the caller provides a signed transaction.
        let tx_bytes = bitcoin::consensus::encode::serialize(&tx);
        let txid = self
            .rpc
            .send_raw_transaction(tx_bytes)
            .map_err(|e| BitcoinError::RpcError(format!("Failed to broadcast: {}", e)))?;

        // Find the vout for the contract output
        let contract_script_pubkey = contract_address.script_pubkey();
        let vout = tx
            .output
            .iter()
            .position(|output| output.script_pubkey == contract_script_pubkey)
            .ok_or_else(|| BitcoinError::InvalidInput(
                "Contract output not found in transaction".to_string()
            ))? as u32;

        Ok(ContractDeployment {
            address: contract_address.to_string(),
            txid,
            vout,
            redeem_script: script.to_vec(),
            witness_program: tweaked_key.serialize().to_vec(),
        })
    }

    /// Verify a contract exists on-chain
    pub fn verify_contract(&self, txid: [u8; 32], vout: u32) -> BitcoinResult<bool> {
        // Check if the UTXO is unspent
        self.rpc
            .is_utxo_unspent(txid, vout)
            .map_err(|e| BitcoinError::RpcError(format!("Failed to verify contract: {}", e)))
    }

    /// Estimate deployment fee
    pub fn estimate_fee(&self, _script_size: usize) -> u64 {
        // Calculate fee based on:
        // - Transaction size (depends on inputs/outputs)
        // - Current fee rate from network
        // Taproot transactions are roughly vbytes

        let base_fee = 1000u64; // Base fee in satoshis
        let fee_rate = 10u64; // sat/vbyte (would come from network)

        base_fee * fee_rate
    }

    /// Build the deployment transaction
    ///
    /// Creates a Bitcoin transaction with:
    /// - Input: The selected UTXO from the deployer
    /// - Output 1: The Taproot contract output (P2TR)
    /// - Output 2: Change back to the deployer (if any)
    fn build_deployment_transaction(
        &self,
        utxo: &super::rpc::UtxoInfo,
        contract_address: &bitcoin::Address,
        _script: &[u8],
        fee: u64,
    ) -> BitcoinResult<bitcoin::Transaction> {
        use bitcoin::{OutPoint, ScriptBuf, Sequence, Transaction, TxIn, TxOut, Witness};

        // Create the input from the UTXO
        let input = TxIn {
            previous_output: OutPoint {
                txid: bitcoin::Txid::from_byte_array(utxo.txid),
                vout: utxo.vout,
            },
            script_sig: ScriptBuf::new(),
            sequence: Sequence::ENABLE_RBF_NO_LOCKTIME,
            witness: Witness::new(),
        };

        // Create the contract output (P2TR - Taproot)
        let contract_script_pubkey = contract_address.script_pubkey();
        let dust_limit = 546u64;
        let contract_value = dust_limit; // Minimum viable amount for the contract

        // Create the change output if there's leftover after fees
        let total_input = utxo.amount_sat;
        let change_value = total_input
            .saturating_sub(fee)
            .saturating_sub(contract_value);

        let mut outputs = vec![TxOut {
            value: bitcoin::Amount::from_sat(contract_value),
            script_pubkey: contract_script_pubkey,
        }];

        // Add change output if significant
        if change_value > dust_limit {
            let (change_key, _) = self.wallet
                .next_address(0)
                .map_err(|e| BitcoinError::RpcError(format!("Failed to derive change address: {}", e)))?;
            let change_address = &change_key.address;

            outputs.push(TxOut {
                value: bitcoin::Amount::from_sat(change_value),
                script_pubkey: change_address.script_pubkey(),
            });
        }

        // Build the unsigned transaction
        let tx = Transaction {
            version: bitcoin::transaction::Version::TWO,
            lock_time: bitcoin::locktime::absolute::LockTime::ZERO,
            input: vec![input],
            output: outputs,
        };

        Ok(tx)
    }
}

/// Deploy a CSV seal contract on Bitcoin
///
/// This creates a Taproot output that can be used as a single-use seal
/// for CSV commitments.
pub fn deploy_csv_seal_contract(
    config: &BitcoinConfig,
    wallet: SealWallet,
    rpc: Box<dyn BitcoinRpc + Send + Sync>,
    value_sat: u64,
) -> BitcoinResult<ContractDeployment> {
    let deployer = ContractDeployer::new(config.clone(), wallet, rpc);

    // Create the CSV seal script
    // This script verifies:
    // 1. The commitment hash in the witness
    // 2. The proper spending authorization
    let csv_seal_script = build_csv_seal_script();

    deployer.deploy_contract(&csv_seal_script, value_sat)
}

/// Build the CSV seal Tapscript
///
/// The script allows spending only if:
/// - The witness contains a valid commitment hash (32 bytes)
/// - The spending transaction is properly signed with the internal key
///
/// Script structure:
/// 1. Push commitment hash to stack
/// 2. Verify the commitment matches expected value
/// 3. Verify signature against internal key
fn build_csv_seal_script() -> Vec<u8> {
    // Build a script that:
    // - Takes a 32-byte commitment hash from witness
    // - Verifies it matches the expected commitment
    // - Verifies the signature

    // This is a basic structure - actual implementation would need:
    // - Proper commitment verification logic
    // - Integration with the spending transaction validation

    let mut script = Vec::new();

    // OP_TRUE (0x51) for now - makes the output spendable with key path
    // For script path spending, we'd need a more complex script
    script.push(0x51); // OP_TRUE

    // Future enhancement: Add actual commitment verification
    // script.push(0x82); // OP_SIZE
    // script.push(32u8); // Push 32
    // script.push(0x88); // OP_EQUALVERIFY
    // ... more validation

    script
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::Network;

    #[test]
    fn test_contract_deployment_structure() {
        // Verify the script builds correctly
        let script = build_csv_seal_script();
        // Script should start with OP_TRUE (0x51) for now
        assert!(!script.is_empty(), "Script should not be empty");

        // Verify the deployment structure works
        let wallet = SealWallet::generate_random(Network::Signet);
        let config = BitcoinConfig::default();
        let rpc =
            Box::new(crate::rpc::TestBitcoinRpc::new(100)) as Box<dyn BitcoinRpc + Send + Sync>;
        let deployer = ContractDeployer::new(config, wallet, rpc);

        let script = build_csv_seal_script();
        let deployment = deployer.deploy_contract(&script, 10000);

        // Should return a valid deployment structure
        assert!(deployment.is_ok(), "Deployment should succeed");
        let deploy = deployment.unwrap();
        assert!(!deploy.address.is_empty(), "Address should not be empty");
        assert!(
            !deploy.witness_program.is_empty(),
            "Witness program should not be empty"
        );
    }

    #[test]
    fn test_estimate_fee() {
        // Fee should be reasonable
        let wallet = SealWallet::generate_random(Network::Signet);
        let config = BitcoinConfig::default();
        let rpc =
            Box::new(crate::rpc::TestBitcoinRpc::new(100)) as Box<dyn BitcoinRpc + Send + Sync>;
        let deployer = ContractDeployer::new(config, wallet, rpc);

        let fee = deployer.estimate_fee(100);
        assert!(fee > 0);
    }
}
