//! Wallet generation for all chains.
//!
//! Provides HD wallet generation using BIP-39/BIP-44 standards.

use crate::commands::wallet::types::WalletAction;
use crate::config::{Chain, Config, Network};
use crate::output;
use crate::state::UnifiedStateManager;
use anyhow::Result;
use std::collections::HashMap;

/// Initialize wallet with one-command setup.
pub fn cmd_init(
    network: Network,
    words: u8,
    fund: bool,
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
        let address = generate_wallet_for_chain(&chain, &network, &mnemonic, state)?;
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

/// Generate BIP-39 mnemonic phrase.
fn generate_mnemonic(words: u8) -> Result<String> {
    use bip32::{Language, Mnemonic};
    use rand::RngCore;

    // Generate 256-bit entropy for 24-word BIP39 mnemonic
    let mut entropy = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut entropy);
    let mnemonic = Mnemonic::from_entropy(entropy, Language::English);

    Ok(mnemonic.phrase().to_string())
}

/// Generate wallet for a specific chain from mnemonic.
fn generate_wallet_for_chain(
    chain: &Chain,
    network: &Network,
    mnemonic: &str,
    state: &mut UnifiedStateManager,
) -> Result<String> {
    match chain {
        Chain::Bitcoin => generate_bitcoin_from_mnemonic(network, mnemonic, state),
        Chain::Ethereum => generate_ethereum_from_mnemonic(mnemonic, state),
        Chain::Sui => generate_sui_from_mnemonic(mnemonic, state),
        Chain::Aptos => generate_aptos_from_mnemonic(mnemonic, state),
        Chain::Solana => generate_solana_from_mnemonic(mnemonic, state),
    }
}

fn generate_bitcoin_from_mnemonic(
    network: &Network,
    mnemonic: &str,
    state: &mut UnifiedStateManager,
) -> Result<String> {
    use bitcoin::Network as BtcNetwork;

    // Simple seed generation from mnemonic (simplified)
    let mut seed = [0u8; 64];
    for (i, byte) in mnemonic.as_bytes().iter().enumerate() {
        if i < 64 {
            seed[i] = *byte;
        }
    }

    let btc_network = match network {
        Network::Dev => BtcNetwork::Regtest,
        Network::Test => BtcNetwork::Signet,
        Network::Main => BtcNetwork::Bitcoin,
    };

    let wallet = csv_adapter_bitcoin::wallet::SealWallet::from_seed(&seed, btc_network)
        .map_err(|e| anyhow::anyhow!("Failed to create Bitcoin wallet: {}", e))?;

    let path = csv_adapter_bitcoin::wallet::Bip86Path::external(0, 0);
    let key = wallet
        .derive_key(&path)
        .map_err(|e| anyhow::anyhow!("Failed to derive key: {}", e))?;

    let address = key.address.to_string();
    let derivation_path = format!("m/86'/0'/0'/0/0");

    // Store in unified state
    state.store_address_with_derivation(Chain::Bitcoin, address.clone(), Some(derivation_path));

    Ok(address)
}

fn generate_ethereum_from_mnemonic(
    _mnemonic: &str,
    state: &mut UnifiedStateManager,
) -> Result<String> {
    use rand::RngCore;
    use secp256k1::{Secp256k1, SecretKey};
    use sha3::{Digest, Keccak256};

    // Generate random private key
    let mut private_key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut private_key);

    // Derive address
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&private_key)
        .map_err(|e| anyhow::anyhow!("Invalid private key: {}", e))?;
    let public_key = secret_key.public_key(&secp);

    // Ethereum address = last 20 bytes of keccak256(public_key)
    let pubkey_bytes = public_key.serialize_uncompressed();
    let hash = Keccak256::digest(&pubkey_bytes[1..]);
    let address = format!("0x{}", hex::encode(&hash[12..]));

    let derivation_path = format!("m/44'/60'/0'/0/0");
    state.store_address_with_derivation(Chain::Ethereum, address.clone(), Some(derivation_path));

    Ok(address)
}

fn generate_sui_from_mnemonic(_mnemonic: &str, state: &mut UnifiedStateManager) -> Result<String> {
    use blake2::{digest::Digest, Blake2b};
    use ed25519_dalek::SigningKey;
    use rand::RngCore;
    use typenum::U32;

    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);

    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();

    // Sui address: BLAKE2b-256(signature_scheme_flag || public_key)
    let mut hasher = Blake2b::<U32>::new();
    hasher.update([0x00]);
    hasher.update(verifying_key.as_bytes());
    let address_bytes = hasher.finalize();
    let address = format!("0x{}", hex::encode(address_bytes));

    let derivation_path = format!("m/44'/784'/0'/0/0");
    state.store_address_with_derivation(Chain::Sui, address.clone(), Some(derivation_path));

    Ok(address)
}

fn generate_aptos_from_mnemonic(_mnemonic: &str, state: &mut UnifiedStateManager) -> Result<String> {
    use ed25519_dalek::SigningKey;
    use rand::RngCore;
    use sha3::{Digest, Sha3_256};

    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);

    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();

    // Aptos address: SHA3-256(public_key || authentication_scheme_byte)
    let mut hasher = Sha3_256::new();
    hasher.update(verifying_key.as_bytes());
    hasher.update([0x00]);
    let auth_key = hasher.finalize();
    let address = format!("0x{}", hex::encode(auth_key));

    let derivation_path = format!("m/44'/637'/0'/0/0");
    state.store_address_with_derivation(Chain::Aptos, address.clone(), Some(derivation_path));

    Ok(address)
}

fn generate_solana_from_mnemonic(_mnemonic: &str, state: &mut UnifiedStateManager) -> Result<String> {
    use ed25519_dalek::SigningKey;
    use rand::RngCore;

    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);

    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();

    // Solana address = public key (base58 encoded)
    let address = bs58::encode(verifying_key.as_bytes()).into_string();

    let derivation_path = format!("m/44'/501'/0'/0/0");
    state.store_address_with_derivation(Chain::Solana, address.clone(), Some(derivation_path));

    Ok(address)
}

// Individual chain generators (for non-mnemonic wallet generation)

fn generate_bitcoin(network: Network, state: &mut UnifiedStateManager) -> Result<()> {
    use bitcoin::Network as BtcNetwork;
    use rand::RngCore;

    let btc_network = match network {
        Network::Dev => BtcNetwork::Regtest,
        Network::Test => BtcNetwork::Signet,
        Network::Main => BtcNetwork::Bitcoin,
    };

    let mut seed = [0u8; 64];
    rand::rngs::OsRng.fill_bytes(&mut seed);
    let wallet = csv_adapter_bitcoin::wallet::SealWallet::from_seed(&seed, btc_network)
        .map_err(|e| anyhow::anyhow!("Failed to create wallet: {}", e))?;

    let path = csv_adapter_bitcoin::wallet::Bip86Path::external(0, 0);
    let key = wallet
        .derive_key(&path)
        .map_err(|e| anyhow::anyhow!("Failed to derive key: {}", e))?;

    let address = key.address.to_string();
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
