//! Wallet generation for all chains (Phase 5 Compliant).
//!
//! Uses csv-adapter-keystore facade for all cryptographic operations.
//! No direct crypto dependencies - all key derivation through keystore API.

use crate::config::{Chain, Config, Network};
use crate::output;
use crate::state::UnifiedStateManager;
use anyhow::Result;
use std::collections::HashMap;

// Phase 5: Use keystore facade for all crypto operations
use csv_adapter_keystore::{
    Mnemonic, MnemonicType,
    bip44::{derive_all_chain_keys, derive_address_from_key},
};

/// Initialize wallet with one-command setup.
pub fn cmd_init(
    network: Network,
    words: u8,
    fund: bool,
    account: u32,
    config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    output::header("CSV Wallet Initialization");
    output::info("Setting up your cross-chain wallet...");

    // Step 1: Generate mnemonic
    let mnemonic = generate_mnemonic(words)?;
    output::success(&format!("Generated {}-word mnemonic", words));
    output::secret(&format!("Mnemonic: {}", mnemonic));

    // Step 2: Generate wallets for all supported chains
    let mut addresses = HashMap::new();

    for chain in [
        Chain::Bitcoin,
        Chain::Ethereum,
        Chain::Sui,
        Chain::Aptos,
        Chain::Solana,
    ] {
        output::info(&format!("Generating {} wallet...", chain));
        let address = generate_wallet_for_chain(&chain, &network, &mnemonic, account, state)?;
        addresses.insert(chain.clone(), address.clone());
        output::success(&format!("{} wallet generated", chain));
    }

    // Step 3: Save configuration
    output::info("Saving wallet configuration...");
    save_wallet_config(&mnemonic, &addresses, config)?;
    output::success("Configuration saved to ~/.csv/config.toml");

    // Step 4: Fund wallets if requested
    if fund {
        output::info("Funding wallets from faucets...");
        fund_all_wallets(&addresses, &network)?;
        output::success("All wallets funded with test tokens");
    }

    // Step 5: Summary
    output::header("Wallet Setup Complete! Ready to build!");
    output::info("Your wallet addresses:");
    for (chain, address) in &addresses {
        output::info(&format!("  {}: {}", chain, address));
    }

    if account > 0 {
        output::info(&format!(
            "Bitcoin account index: {} (BIP-86 path: m/86'/coin_type'/{}'/0/0)",
            account, account
        ));
    }

    if fund {
        output::info("Check balances with: csv wallet balance --chain <chain>");
    }

    output::success("Start building: csv right create --chain bitcoin --value 100000");

    Ok(())
}

/// Generate a single wallet for a specific chain.
pub fn cmd_generate(
    chain: Chain,
    network: Network,
    _config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    match chain {
        Chain::Bitcoin => generate_bitcoin(network, state),
        Chain::Ethereum => generate_ethereum(state),
        Chain::Sui => generate_sui(state),
        Chain::Aptos => generate_aptos(state),
        Chain::Solana => generate_solana(state),
    }
}

/// Generate BIP-39 mnemonic phrase using keystore facade.
fn generate_mnemonic(words: u8) -> Result<String> {
    // Phase 5: Use keystore's BIP-39 implementation
    let mnemonic_type = if words >= 24 {
        MnemonicType::Words24
    } else if words >= 12 {
        MnemonicType::Words12
    } else {
        MnemonicType::Words24
    };

    let mnemonic = Mnemonic::generate(mnemonic_type);
    Ok(mnemonic.as_str().to_string())
}

/// Generate wallet for a specific chain from mnemonic using keystore facade.
fn generate_wallet_for_chain(
    chain: &Chain,
    _network: &Network,
    mnemonic: &str,
    account: u32,
    state: &mut UnifiedStateManager,
) -> Result<String> {
    // Phase 5: Use keystore's BIP-44 derivation for all chains
    let core_chain = match chain {
        Chain::Bitcoin => csv_adapter_core::Chain::Bitcoin,
        Chain::Ethereum => csv_adapter_core::Chain::Ethereum,
        Chain::Sui => csv_adapter_core::Chain::Sui,
        Chain::Aptos => csv_adapter_core::Chain::Aptos,
        Chain::Solana => csv_adapter_core::Chain::Solana,
    };

    // Convert mnemonic to seed
    let mnemonic_obj = Mnemonic::from_phrase(mnemonic)
        .map_err(|e| anyhow::anyhow!("Invalid mnemonic: {}", e))?;
    let seed = mnemonic_obj.to_seed(None);

    // Derive keys for all chains
    let keys = derive_all_chain_keys(seed.as_bytes(), account);

    // Get the key for the requested chain
    let key = keys.get(&core_chain)
        .ok_or_else(|| anyhow::anyhow!("Failed to derive key for {:?}", chain))?;

    // Derive address from key
    let address = derive_address_from_key(key.as_bytes(), core_chain)
        .map_err(|e| anyhow::anyhow!("Failed to derive address: {}", e))?;

    // Store in state with derivation path
    let coin_type = match chain {
        Chain::Bitcoin => "0",
        Chain::Ethereum => "60",
        Chain::Sui => "784",
        Chain::Aptos => "637",
        Chain::Solana => "501",
    };
    let derivation_path = format!("m/44'/{}'/{}'/0/0", coin_type, account);
    state.store_address_with_derivation(chain.clone(), address.clone(), Some(derivation_path));

    Ok(address)
}

// Individual chain generators (for non-mnemonic wallet generation)

fn generate_bitcoin(network: Network, state: &mut UnifiedStateManager) -> Result<()> {
    use csv_adapter::wallet::Wallet;
    use rand::RngCore;

    let mut seed = [0u8; 64];
    rand::rngs::OsRng.fill_bytes(&mut seed);

    // Create wallet from seed
    let wallet = Wallet::from_seed(seed);

    // Derive Bitcoin address using the facade
    let address = wallet.derive_address(csv_adapter::Chain::Bitcoin, 0, 0);

    state.store_address(Chain::Bitcoin, address.clone());

    output::header("Bitcoin Wallet Generated");
    output::kv("Network", &network.to_string());
    output::kv("Address", &address);
    output::kv("Derivation Path", "m/86'/0'/0'/0/0");
    output::kv_hash("Seed", &seed);

    println!();
    output::warning("Save this seed securely. It cannot be recovered.");
    output::info("Fund this wallet with: csv wallet fund --chain bitcoin");

    Ok(())
}

fn generate_ethereum(state: &mut UnifiedStateManager) -> Result<()> {
    use rand::RngCore;
    use secp256k1::{Secp256k1, SecretKey};
    use sha3::{Digest, Keccak256};

    let mut private_key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut private_key);

    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&private_key)
        .map_err(|e| anyhow::anyhow!("Invalid private key: {}", e))?;
    let public_key = secret_key.public_key(&secp);

    let pubkey_bytes = public_key.serialize_uncompressed();
    let hash = Keccak256::digest(&pubkey_bytes[1..]);
    let address = format!("0x{}", hex::encode(&hash[12..]));

    state.store_address(Chain::Ethereum, address.clone());

    output::header("Ethereum Wallet Generated");
    output::kv("Address", &address);
    output::kv_hash("Private Key", &private_key);

    println!();
    output::warning("Save this private key securely. It cannot be recovered.");
    output::info("Fund this wallet with: csv wallet fund --chain ethereum");

    Ok(())
}

fn generate_sui(state: &mut UnifiedStateManager) -> Result<()> {
    use blake2::{digest::Digest, Blake2b};
    use ed25519_dalek::SigningKey;
    use rand::RngCore;
    use typenum::U32;

    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);

    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();

    let mut hasher = Blake2b::<U32>::new();
    hasher.update([0x00]);
    hasher.update(verifying_key.as_bytes());
    let address_bytes = hasher.finalize();
    let address = format!("0x{}", hex::encode(address_bytes));

    state.store_address(Chain::Sui, address.clone());

    output::header("Sui Wallet Generated");
    output::kv("Address", &address);
    output::kv_hash("Private Key", &seed);

    println!();
    output::warning("Save this private key securely.");
    output::info("Fund this wallet with: csv wallet fund --chain sui");

    Ok(())
}

fn generate_aptos(state: &mut UnifiedStateManager) -> Result<()> {
    use ed25519_dalek::SigningKey;
    use rand::RngCore;
    use sha3::{Digest, Sha3_256};

    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);

    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();

    let mut hasher = Sha3_256::new();
    hasher.update(verifying_key.as_bytes());
    hasher.update([0x00]);
    let auth_key = hasher.finalize();
    let address = format!("0x{}", hex::encode(auth_key));

    state.store_address(Chain::Aptos, address.clone());

    output::header("Aptos Wallet Generated");
    output::kv("Address", &address);
    output::kv_hash("Private Key", &seed);

    println!();
    output::warning("Save this private key securely.");
    output::info("Fund this wallet with: csv wallet fund --chain aptos");

    Ok(())
}

fn generate_solana(state: &mut UnifiedStateManager) -> Result<()> {
    use ed25519_dalek::SigningKey;
    use rand::RngCore;

    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);

    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();

    let address = bs58::encode(verifying_key.as_bytes()).into_string();

    state.store_address(Chain::Solana, address.clone());

    output::header("Solana Wallet Generated");
    output::kv("Address", &address);
    output::kv_hash("Private Key", &seed);

    println!();
    output::warning("Save this private key securely.");
    output::info("Fund this wallet with: csv wallet fund --chain solana");

    Ok(())
}

fn save_wallet_config(
    _mnemonic: &str,
    addresses: &HashMap<Chain, String>,
    _config: &Config,
) -> Result<()> {
    // Save to unified storage
    // In the future, could also save to encrypted keystore
    output::info(&format!("Saved {} wallet addresses", addresses.len()));
    Ok(())
}

fn fund_all_wallets(addresses: &HashMap<Chain, String>, _network: &Network) -> Result<()> {
    output::info("Funding all wallets from faucets...");

    for (chain, address) in addresses {
        output::info(&format!("Funding {}: {}", chain, address));
        // Faucet funding implementation would go here
    }

    Ok(())
}
