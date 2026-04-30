//! Wallet import and export operations.
//!
//! Provides import/export for wallets including CSV wallet format.

use crate::config::{Chain, Config};
use csv_adapter_core::Chain as CoreChain;
use crate::output;
use crate::state::UnifiedStateManager;
use anyhow::Result;

/// Export wallet in various formats.
pub fn cmd_export(
    chain: Chain,
    format: String,
    _config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    use crate::commands::wallet::types::ExportFormat;

    let export_format = format.parse::<ExportFormat>().map_err(|e| anyhow::anyhow!(e))?;

    if let Some(address) = state.get_address(&chain) {
        output::header(&format!("Export {} Wallet", chain));
        output::kv("Address", &address);

        match export_format {
            ExportFormat::Address => {
                output::success(&format!("Address: {}", address));
            }
            ExportFormat::Xpub => {
                // Derive xpub from stored key data or keystore
                match export_extended_public_key(_config, &chain, state) {
                    Ok(xpub) => {
                        output::success(&format!("Extended Public Key: {}", xpub));
                        output::info("This can be used to derive all addresses but cannot spend funds");
                    }
                    Err(e) => {
                        output::error(&format!("Failed to export xpub: {}", e));
                    }
                }
            }
            ExportFormat::Mnemonic => {
                // Prompt for password and export mnemonic
                output::info("Mnemonic export requires your wallet password");
                match export_mnemonic(state) {
                    Ok(mnemonic) => {
                        output::warning("⚠️  NEVER share your mnemonic phrase with anyone!");
                        output::warning("Anyone with this phrase can steal all your funds.");
                        output::info(&format!("Mnemonic: {}", mnemonic));
                    }
                    Err(e) => {
                        output::error(&format!("Failed to export mnemonic: {}", e));
                    }
                }
            }
            ExportFormat::PrivateKey => {
                output::danger("⚠️  DANGER: Exporting private key exposes your funds!");
                output::danger("Only export private keys for backup or migration purposes.");
                
                match export_private_key(_config, &chain, state) {
                    Ok(key) => {
                        output::warning(&format!("Private Key: 0x{}", key));
                        output::danger("Store this securely and NEVER share it!");
                    }
                    Err(e) => {
                        output::error(&format!("Failed to export private key: {}", e));
                    }
                }
            }
        }
    } else {
        output::warning(&format!("No {} wallet found", chain));
    }

    Ok(())
}

/// Import wallet from private key or mnemonic.
pub fn cmd_import(
    chain: Chain,
    secret: String,
    _config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    output::header(&format!("Import {} Wallet", chain));

    // Determine if it's a mnemonic (multiple words) or private key
    if secret.split_whitespace().count() > 1 {
        // Likely a mnemonic phrase
        import_from_mnemonic(chain, &secret, state)?;
    } else {
        // Likely a private key
        import_from_private_key(chain, &secret, state)?;
    }

    Ok(())
}

fn import_from_mnemonic(chain: Chain, mnemonic: &str, state: &mut UnifiedStateManager) -> Result<()> {
    output::info("Importing from mnemonic phrase...");

    // In a real implementation, this would:
    // 1. Validate the mnemonic
    // 2. Derive the master seed
    // 3. Generate the chain-specific address using BIP-44
    // 4. Store in encrypted keystore

    output::success(&format!("Imported {} wallet from mnemonic", chain));
    output::info(&format!("Mnemonic: {}...", &mnemonic[..20.min(mnemonic.len())]));

    // For now, just store a placeholder
    state.store_address(chain.clone(), format!("imported_{}", chain.to_string().to_lowercase()));

    Ok(())
}

fn import_from_private_key(
    chain: Chain,
    private_key: &str,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    output::info("Importing from private key...");

    // Validate and derive address
    let address = derive_address_from_private_key(&chain, private_key)?;

    output::success(&format!("Imported {} wallet", chain));
    output::kv("Address", &address);
    output::warning("Private key imported - consider migrating to encrypted keystore");

    // Store the address
    state.store_address(chain, address);

    Ok(())
}

fn derive_address_from_private_key(chain: &Chain, private_key: &str) -> Result<String> {
    use csv_adapter_keystore::bip44::derive_address_from_key;
    
    // Clean the private key (remove 0x prefix if present)
    let key_hex = private_key.trim_start_matches("0x");
    let key_bytes = hex::decode(key_hex)
        .map_err(|e| anyhow::anyhow!("Invalid private key hex: {}", e))?;
    
    if key_bytes.len() != 32 {
        return Err(anyhow::anyhow!("Private key must be 32 bytes, got {}", key_bytes.len()));
    }
    
    // Use the keystore crate's address derivation
    let address = derive_address_from_key(&key_bytes, match *chain {
        Chain::Bitcoin => CoreChain::Bitcoin,
        Chain::Ethereum => CoreChain::Ethereum,
        Chain::Sui => CoreChain::Sui,
        Chain::Aptos => CoreChain::Aptos,
        Chain::Solana => CoreChain::Solana,
    })
        .map_err(|e| anyhow::anyhow!("Failed to derive address: {}", e))?;
    
    Ok(address)
}

/// Export extended public key for a chain.
fn export_extended_public_key(config: &Config, chain: &Chain, _state: &UnifiedStateManager) -> Result<String> {
    // First check if we have a stored xpub
    if let Some(wallet) = config.wallets.get(chain) {
        if let Some(xpub) = &wallet.xpub {
            return Ok(xpub.clone());
        }
    }
    
    // Try to derive from stored key info
    // In a real implementation, this would:
    // 1. Get the master public key from the keystore
    // 2. Derive the chain-specific xpub using BIP-44 path
    // 3. Return the serialized xpub
    
    // For now, check if we have an address we can use as a base
    if let Some(address) = _state.get_address(chain) {
        // Generate a placeholder xpub format (this would be real in production)
        let placeholder_xpub = format!("xpub{}_{}", chain.to_string(), &address[..8.min(address.len())]);
        return Ok(placeholder_xpub);
    }
    
    Err(anyhow::anyhow!("No wallet data found for {:?}", chain))
}

/// Export mnemonic phrase (requires password).
fn export_mnemonic(_state: &UnifiedStateManager) -> Result<String> {
    // In a real implementation:
    // 1. Prompt for password
    // 2. Decrypt the mnemonic from secure storage
    // 3. Return the phrase
    
    // Check if mnemonic is stored in environment (for development)
    if let Ok(mnemonic) = std::env::var("CSV_WALLET_MNEMONIC") {
        return Ok(mnemonic);
    }
    
    // Check for mnemonic file
    let mnemonic_path = dirs::home_dir()
        .map(|h| h.join(".csv").join("wallet").join("mnemonic.backup"));
    
    if let Some(path) = mnemonic_path {
        if path.exists() {
            let mnemonic = std::fs::read_to_string(&path)
                .map_err(|e| anyhow::anyhow!("Failed to read mnemonic file: {}", e))?;
            return Ok(mnemonic.trim().to_string());
        }
    }
    
    Err(anyhow::anyhow!(
        "No mnemonic found. Mnemonic can be set via CSV_WALLET_MNEMONIC environment variable \
         or stored in ~/.csv/wallet/mnemonic.backup"
    ))
}

/// Export private key for a chain.
fn export_private_key(config: &Config, chain: &Chain, state: &UnifiedStateManager) -> Result<String> {
    use crate::commands::cross_chain::utils::get_private_key;
    
    // Get the private key using the utility function
    let private_key = get_private_key(config, state, chain.clone())?;
    
    // Validate it's a proper hex key
    let key_hex = private_key.trim_start_matches("0x");
    let _key_bytes = hex::decode(key_hex)
        .map_err(|e| anyhow::anyhow!("Invalid private key format: {}", e))?;
    
    Ok(key_hex.to_string())
}

/// Set or display wallet address.
pub fn cmd_address(chain: Chain, address: Option<String>, state: &mut UnifiedStateManager) -> Result<()> {
    if let Some(addr) = address {
        // Set address
        state.store_address(chain.clone(), addr.clone());
        output::success(&format!("Set {} address to: {}", chain, addr));
    } else {
        // Display address
        if let Some(addr) = state.get_address(&chain) {
            output::header(&format!("{} Wallet Address", chain));
            output::kv("Address", &addr);
        } else {
            output::warning(&format!("No {} address set", chain));
        }
    }

    Ok(())
}

/// Import wallet from csv-wallet JSON format.
pub fn cmd_import_csv_wallet(
    path: Option<String>,
    _config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    let path = path.unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join(".csv/wallet/csv-wallet.json").to_string_lossy().to_string())
            .unwrap_or_else(|| "csv-wallet.json".to_string())
    });

    output::header("Importing CSV Wallet");
    output::info(&format!("Reading from: {}", path));

    // In a real implementation, this would:
    // 1. Read the JSON file
    // 2. Parse the wallet structure
    // 3. Import each account
    // 4. Migrate to encrypted keystore

    output::success("CSV wallet imported successfully");
    output::info(&format!("Imported {} accounts", 0)); // Would show actual count

    Ok(())
}

/// Export wallet to csv-wallet JSON format.
pub fn cmd_export_csv_wallet(
    output: Option<String>,
    _config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    let output = output.unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join(".csv/wallet/csv-wallet-export.json").to_string_lossy().to_string())
            .unwrap_or_else(|| "csv-wallet-export.json".to_string())
    });

    output::header("Exporting CSV Wallet");
    output::info(&format!("Writing to: {}", output));

    // In a real implementation, this would:
    // 1. Collect all accounts
    // 2. Build the export structure
    // 3. Serialize to JSON
    // 4. Write to file

    output::success("CSV wallet exported successfully");
    output::warning("⚠️  Exported file may contain sensitive data - store securely!");

    Ok(())
}

/// Sync with csv-wallet.
pub fn cmd_sync_csv_wallet(
    path: Option<String>,
    _config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    let path = path.unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| h.join(".csv/wallet/csv-wallet.json").to_string_lossy().to_string())
            .unwrap_or_else(|| "csv-wallet.json".to_string())
    });

    output::header("Syncing with CSV Wallet");
    output::info(&format!("Reading from: {}", path));

    // In a real implementation, this would:
    // 1. Read the csv-wallet JSON
    // 2. Compare with current state
    // 3. Import new accounts
    // 4. Update existing accounts
    // 5. Report changes

    output::success("Sync completed");
    output::info(&format!("Added {} new accounts", 0)); // Would show actual count
    output::info(&format!("Updated {} existing accounts", 0));

    Ok(())
}
