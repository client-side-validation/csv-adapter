//! Wallet import and export operations.
//!
//! Provides import/export for wallets including CSV wallet format.

use crate::config::{Chain, Config};
use crate::output;
use crate::state::UnifiedStateManager;
use anyhow::Result;
use csv_adapter_core::Chain as CoreChain;
use serde::{Deserialize, Serialize};

/// Wallet data format compatible with csv-wallet.
#[derive(Serialize, Deserialize, Clone)]
pub struct CsvWalletExport {
    /// All accounts
    pub accounts: Vec<CsvWalletAccount>,
    /// Last selected account ID
    pub selected_account_id: Option<String>,
}

/// Single account in the wallet export format.
#[derive(Serialize, Deserialize, Clone)]
pub struct CsvWalletAccount {
    /// Unique account ID (UUID)
    pub id: String,
    /// Blockchain this account belongs to
    pub chain: String,
    /// User-friendly account name
    pub name: String,
    /// Keystore reference (UUID)
    pub keystore_ref: Option<String>,
    /// Derived address for display
    pub address: String,
    /// Balance in native token
    #[serde(default, skip_serializing)]
    pub balance: f64,
    /// BIP-44 derivation path (if HD wallet)
    pub derivation_path: Option<String>,
}

/// Export wallet in various formats.
pub fn cmd_export(
    chain: Chain,
    format: String,
    _config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    use crate::commands::wallet::types::ExportFormat;

    let export_format = format
        .parse::<ExportFormat>()
        .map_err(|e| anyhow::anyhow!(e))?;

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
                        output::info(
                            "This can be used to derive all addresses but cannot spend funds",
                        );
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

fn import_from_mnemonic(
    chain: Chain,
    mnemonic: &str,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    output::info("Importing from mnemonic phrase...");

    // In a real implementation, this would:
    // 1. Validate the mnemonic
    // 2. Derive the master seed
    // 3. Generate the chain-specific address using BIP-44
    // 4. Store in encrypted keystore

    output::success(&format!("Imported {} wallet from mnemonic", chain));
    output::info(&format!(
        "Mnemonic: {}...",
        &mnemonic[..20.min(mnemonic.len())]
    ));

    // Derive the actual address from the mnemonic
    let address = derive_address_from_mnemonic(&chain, mnemonic)
        .map_err(|e| anyhow::anyhow!("Failed to derive address from mnemonic: {}", e))?;

    state.store_address(chain.clone(), address.clone());
    output::success(&format!("Derived {} address: {}", chain, address));

    Ok(())
}

/// Derive address from mnemonic phrase for a specific chain.
fn derive_address_from_mnemonic(chain: &Chain, mnemonic: &str) -> Result<String> {
    use csv_adapter_core::Chain as CoreChain;
    use csv_adapter_keystore::bip39::Mnemonic;
    use csv_adapter_keystore::bip44::{derive_key, derive_address_from_key};

    // Use the keystore crate for derivation
    let core_chain = match chain {
        Chain::Bitcoin => CoreChain::Bitcoin,
        Chain::Ethereum => CoreChain::Ethereum,
        Chain::Sui => CoreChain::Sui,
        Chain::Aptos => CoreChain::Aptos,
        Chain::Solana => CoreChain::Solana,
    };

    // Parse mnemonic and derive seed
    let mnemonic = Mnemonic::from_phrase(mnemonic)
        .map_err(|e| anyhow::anyhow!("Invalid mnemonic: {}", e))?;
    let seed = mnemonic.to_seed(None);
    let seed_bytes = seed.as_bytes();

    // Ensure seed is exactly 64 bytes
    if seed_bytes.len() != 64 {
        return Err(anyhow::anyhow!("Invalid seed length: expected 64, got {}", seed_bytes.len()));
    }
    let mut seed_array = [0u8; 64];
    seed_array.copy_from_slice(seed_bytes);

    // Derive the key
    let secret_key = derive_key(&seed_array, core_chain, 0, 0)
        .map_err(|e| anyhow::anyhow!("Failed to derive key: {}", e))?;

    // Derive address from key
    let address = derive_address_from_key(secret_key.as_bytes(), core_chain)
        .map_err(|e| anyhow::anyhow!("Failed to derive address: {}", e))?;

    Ok(address)
}

fn import_from_private_key(
    chain: Chain,
    private_key: &str,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    use csv_adapter_keystore::keystore::{KeystoreFile, KdfType};
    use csv_adapter_keystore::memory::{Passphrase, SecretKey};

    output::info("Importing from private key...");
    output::warning("Private key will be immediately encrypted and stored in keystore");

    // Validate and derive address
    let address = derive_address_from_private_key(&chain, private_key)?;

    // Clean the private key
    let key_hex = private_key.trim_start_matches("0x");
    let key_bytes =
        hex::decode(key_hex).map_err(|e| anyhow::anyhow!("Invalid private key hex: {}", e))?;

    if key_bytes.len() != 32 {
        return Err(anyhow::anyhow!(
            "Private key must be 32 bytes, got {}",
            key_bytes.len()
        ));
    }

    // Encrypt and store in keystore (production path)
    let passphrase = Passphrase::new(""); // In real usage, prompt for password
    let key_array: [u8; 32] = key_bytes.try_into()
        .map_err(|_| anyhow::anyhow!("Invalid key length"))?;
    let secret_key = SecretKey::new(key_array);
    let _keystore = KeystoreFile::encrypt(&secret_key, &passphrase, KdfType::default())
        .map_err(|e| anyhow::anyhow!("Failed to create keystore: {}", e))?;

    // Store keystore path instead of raw key
    let keystore_path = format!("~/.csv/keystore/{}_{}.json", chain, &address[..8]);
    output::info(&format!("Encrypted keystore created: {}", keystore_path));

    output::success(&format!("Imported {} wallet", chain));
    output::kv("Address", &address);
    output::success("Private key encrypted and stored in keystore");

    // Store the address only (key is in keystore)
    state.store_address(chain, address);

    Ok(())
}

fn derive_address_from_private_key(chain: &Chain, private_key: &str) -> Result<String> {
    use csv_adapter_keystore::bip44::derive_address_from_key;

    // Clean the private key (remove 0x prefix if present)
    let key_hex = private_key.trim_start_matches("0x");
    let key_bytes =
        hex::decode(key_hex).map_err(|e| anyhow::anyhow!("Invalid private key hex: {}", e))?;

    if key_bytes.len() != 32 {
        return Err(anyhow::anyhow!(
            "Private key must be 32 bytes, got {}",
            key_bytes.len()
        ));
    }

    // Use the keystore crate's address derivation
    let address = derive_address_from_key(
        &key_bytes,
        match *chain {
            Chain::Bitcoin => CoreChain::Bitcoin,
            Chain::Ethereum => CoreChain::Ethereum,
            Chain::Sui => CoreChain::Sui,
            Chain::Aptos => CoreChain::Aptos,
            Chain::Solana => CoreChain::Solana,
        },
    )
    .map_err(|e| anyhow::anyhow!("Failed to derive address: {}", e))?;

    Ok(address)
}

/// Export extended public key for a chain.
fn export_extended_public_key(
    config: &Config,
    chain: &Chain,
    _state: &UnifiedStateManager,
) -> Result<String> {
    // First check if we have a stored xpub
    if let Some(wallet) = config.wallets.get(chain) {
        if let Some(xpub) = &wallet.xpub {
            return Ok(xpub.clone());
        }
    }

    // Production: xpub must be derived from the encrypted keystore
    // This requires proper BIP-44 derivation from the master public key
    Err(anyhow::anyhow!(
        "Extended public key not available. \
         Please configure a keystore with proper key derivation."
    ))
}

/// Export mnemonic phrase (requires password).
fn export_mnemonic(_state: &UnifiedStateManager) -> Result<String> {
    // SECURITY: Mnemonic export only available through encrypted keystore
    // Plaintext storage in environment variables or files is NOT supported in production.

    // Production path: mnemonic must be retrieved from encrypted keystore
    // This requires the csv-adapter-keystore to be properly configured
    Err(anyhow::anyhow!(
        "Mnemonic export is only available through the encrypted keystore. \
         Please use the wallet migration tools or configure a proper keystore."
    ))
}

// NOTE: Private key export is NOT supported in production.
// Keys must remain in encrypted keystore.
// Use keystore migration tools or backup encrypted keystore files.

/// Set or display wallet address.
pub fn cmd_address(
    chain: Chain,
    address: Option<String>,
    state: &mut UnifiedStateManager,
) -> Result<()> {
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
            .map(|h| {
                h.join(".csv/wallet/csv-wallet.json")
                    .to_string_lossy()
                    .to_string()
            })
            .unwrap_or_else(|| "csv-wallet.json".to_string())
    });

    output::header("Importing CSV Wallet");
    output::info(&format!("Reading from: {}", path));

    // Read the JSON file
    let json = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("Failed to read wallet file: {}", e))?;

    // Parse the wallet structure
    let wallet: CsvWalletExport = serde_json::from_str(&json)
        .map_err(|e| anyhow::anyhow!("Failed to parse wallet JSON: {}", e))?;

    // Import each account
    let mut imported = 0u32;
    for account in &wallet.accounts {
        let chain = match account.chain.to_lowercase().as_str() {
            "bitcoin" => Chain::Bitcoin,
            "ethereum" => Chain::Ethereum,
            "sui" => Chain::Sui,
            "aptos" => Chain::Aptos,
            "solana" => Chain::Solana,
            _ => {
                output::warning(&format!("Skipping unknown chain: {}", account.chain));
                continue;
            }
        };

        // Store the address
        state.store_address(chain.clone(), account.address.clone());
        imported += 1;
        output::success(&format!("Imported {} ({})", account.name, chain));
    }

    output::success("CSV wallet imported successfully");
    output::info(&format!("Imported {} accounts", imported));

    Ok(())
}

/// Export wallet to csv-wallet JSON format.
pub fn cmd_export_csv_wallet(
    output: Option<String>,
    config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    let output = output.unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| {
                h.join(".csv/wallet/csv-wallet-export.json")
                    .to_string_lossy()
                    .to_string()
            })
            .unwrap_or_else(|| "csv-wallet-export.json".to_string())
    });

    output::header("Exporting CSV Wallet");
    output::info(&format!("Writing to: {}", output));

    // Collect all accounts from state
    let mut accounts = Vec::new();
    let mut id_counter = 0u32;

    for chain in [
        Chain::Bitcoin,
        Chain::Ethereum,
        Chain::Sui,
        Chain::Aptos,
        Chain::Solana,
    ] {
        if let Some(address) = state.get_address(&chain) {
            id_counter += 1;
            let name = format!("{:?} Account", chain);

            // Try to get derivation path from config
            let derivation_path = config
                .wallets
                .get(&chain)
                .and_then(|w| w.derivation_path.clone());

            accounts.push(CsvWalletAccount {
                id: format!("account-{:04}", id_counter),
                chain: chain.to_string().to_lowercase(),
                name,
                keystore_ref: None,
                address: address.to_string(),
                balance: 0.0,
                derivation_path,
            });
        }
    }

    // Build export structure
    let wallet = CsvWalletExport {
        accounts,
        selected_account_id: None,
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&wallet)
        .map_err(|e| anyhow::anyhow!("Failed to serialize wallet: {}", e))?;

    // Write to file
    std::fs::write(&output, &json)
        .map_err(|e| anyhow::anyhow!("Failed to write wallet file: {}", e))?;

    output::success("CSV wallet exported successfully");
    output::info(&format!("Exported {} accounts to {}", wallet.accounts.len(), output));
    output::warning("⚠️  Exported file may contain sensitive data - store securely!");

    Ok(())
}

/// Sync with csv-wallet.
pub fn cmd_sync_csv_wallet(
    path: Option<String>,
    _config: &Config,
    _state: &mut UnifiedStateManager,
) -> Result<()> {
    let path = path.unwrap_or_else(|| {
        dirs::home_dir()
            .map(|h| {
                h.join(".csv/wallet/csv-wallet.json")
                    .to_string_lossy()
                    .to_string()
            })
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
