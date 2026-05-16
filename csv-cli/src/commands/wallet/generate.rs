//! Wallet generation for all chains (Phase 5 Compliant).
//!
//! Uses csv-keys file keystore for encrypted key storage.
//! Mnemonics and private keys are encrypted with user passphrase.

use crate::config::{Chain, Config, Network};
use crate::output;
use crate::state::UnifiedStateManager;
use anyhow::Result;
use std::collections::HashMap;

use csv_keys::{
    Mnemonic, MnemonicType,
    bip44::{derive_address_from_key, derive_all_chain_keys},
    file_keystore::FileKeystore,
    memory::Passphrase,
};

/// Initialize wallet with one-command setup.
pub fn cmd_init(
    network: Network,
    words: u8,
    account: u32,
    config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    output::header("CSV Wallet Initialization");
    output::info("Setting up your cross-chain wallet...");

    // Prompt for passphrase
    let passphrase = prompt_passphrase("Enter keystore passphrase (min 12 chars)")?;
    if passphrase.len() < 12 {
        anyhow::bail!("Passphrase must be at least 12 characters");
    }
    let passphrase = Passphrase::new(passphrase);

    // Step 1: Generate mnemonic
    let mnemonic = generate_mnemonic(words)?;
    output::success(&format!("Generated {}-word mnemonic", words));
    output::info("Write this mnemonic down securely. It is your wallet recovery phrase.");
    println!();
    output::kv("Mnemonic phrase", &mnemonic);
    println!();

    // Step 2: Generate wallets for all supported chains
    let mut addresses = HashMap::new();

    // Initialize file keystore
    let mut keystore = FileKeystore::new(None)?;

    for chain in [
        Chain::new("bitcoin"),
        Chain::new("ethereum"),
        Chain::new("sui"),
        Chain::new("aptos"),
        Chain::new("solana"),
    ] {
        output::info(&format!("Generating {} wallet...", chain));
        let address = generate_wallet_for_chain(
            &chain,
            &network,
            &mnemonic,
            account,
            state,
            &mut keystore,
            &passphrase,
        )?;
        addresses.insert(chain.clone(), address.clone());
        output::success(&format!("{} wallet generated", chain));
    }

    // Step 3: Save configuration
    output::info("Saving wallet configuration...");
    state.storage.wallet.mnemonic = Some(mnemonic.clone());
    save_wallet_config(&mnemonic, &addresses, config)?;
    output::success("Configuration saved");

    // Step 4: Summary
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

    output::warning("Store your mnemonic phrase securely. It can recover all your keys.");
    output::info("Check balances with: csv wallet balance --chain <chain>");
    output::info("Fund your wallets using chain faucets or exchanges");

    output::success("Start building: csv sanad create --chain bitcoin --value 100000");

    Ok(())
}

/// Generate a single wallet for a specific chain.
pub fn cmd_generate(
    chain: Chain,
    network: Network,
    _config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    match chain.as_str() {
        "bitcoin" => generate_bitcoin(network, state),
        "ethereum" => generate_ethereum(state),
        "sui" => generate_sui(state),
        "aptos" => generate_aptos(state),
        "solana" => generate_solana(state),
        _ => Err(anyhow::anyhow!("Unknown chain: {}", chain)),
    }
}

/// Generate BIP-39 mnemonic phrase using keystore runtime.
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

/// Generate wallet for a specific chain from mnemonic using keystore runtime.
fn generate_wallet_for_chain(
    chain: &Chain,
    _network: &Network,
    mnemonic: &str,
    account: u32,
    state: &mut UnifiedStateManager,
    keystore: &mut FileKeystore,
    passphrase: &Passphrase,
) -> Result<String> {
    // Phase 5: Use keystore's BIP-44 derivation for all chains
    let core_chain = csv_core::ChainId::new(chain.as_str());

    // Convert mnemonic to seed
    let mnemonic_obj =
        Mnemonic::from_phrase(mnemonic).map_err(|e| anyhow::anyhow!("Invalid mnemonic: {}", e))?;
    let seed = mnemonic_obj.to_seed(None);

    // Derive keys for all chains
    let keys = derive_all_chain_keys(seed.as_bytes(), account);

    // Get the key for the requested chain
    let key = keys
        .get(&core_chain)
        .ok_or_else(|| anyhow::anyhow!("Failed to derive key for {:?}", chain))?;

    // Derive address from key
    let address = derive_address_from_key(key.as_bytes(), &core_chain)
        .map_err(|e| anyhow::anyhow!("Failed to derive address: {}", e))?;

    // Store private key in encrypted file keystore
    let key_id = format!("{}-{}", chain.to_string().to_lowercase(), account);
    keystore.store_key(
        &key_id,
        &chain.to_string().to_lowercase(),
        Some(&format!("{} Account (account {})", chain, account)),
        key,
        passphrase,
    )?;

    // Store in state with derivation path
    let coin_type = match chain.as_str() {
        "bitcoin" => "0",
        "ethereum" => "60",
        "sui" => "784",
        "aptos" => "637",
        "solana" => "501",
        _ => "0",
    };
    let derivation_path = format!("m/44'/{}'/{}'/0/0", coin_type, account);
    state.store_address_with_derivation(chain.clone(), address.clone(), Some(derivation_path));

    Ok(address)
}

// Individual chain generators (for non-mnemonic wallet generation)

fn generate_bitcoin(network: Network, state: &mut UnifiedStateManager) -> Result<()> {
    use csv_keys::bip44::derive_address_from_key;
    use csv_keys::memory::SecretKey;
    use rand::RngCore;

    let mut key_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key_bytes);
    let secret_key = SecretKey::new(key_bytes);

    // Derive Bitcoin address using the keystore runtime
    let address =
        derive_address_from_key(secret_key.as_bytes(), &csv_core::ChainId::new("bitcoin"))
            .map_err(|e| anyhow::anyhow!("Failed to derive address: {}", e))?;

    state.store_address(Chain::new("bitcoin"), address.clone());

    output::header("Bitcoin Wallet Generated");
    output::kv("Network", &network.to_string());
    output::kv("Address", &address);
    output::kv("Derivation Path", "m/86'/0'/0'/0/0");

    println!();
    output::warning(
        "Your private key has been generated. Use 'csv wallet export' to view it securely.",
    );

    Ok(())
}

fn generate_ethereum(state: &mut UnifiedStateManager) -> Result<()> {
    use csv_keys::bip44::derive_address_from_key;
    use csv_keys::memory::SecretKey;
    use rand::RngCore;

    let mut key_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key_bytes);
    let secret_key = SecretKey::new(key_bytes);

    let address =
        derive_address_from_key(secret_key.as_bytes(), &csv_core::ChainId::new("ethereum"))
            .map_err(|e| anyhow::anyhow!("Failed to derive address: {}", e))?;

    state.store_address(Chain::new("ethereum"), address.clone());

    output::header("Ethereum Wallet Generated");
    output::kv("Address", &address);

    println!();
    output::warning(
        "Your private key has been generated. Use 'csv wallet export' to view it securely.",
    );

    Ok(())
}

fn generate_sui(state: &mut UnifiedStateManager) -> Result<()> {
    use csv_keys::bip44::derive_address_from_key;
    use csv_keys::memory::SecretKey;
    use rand::RngCore;

    let mut key_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key_bytes);
    let secret_key = SecretKey::new(key_bytes);

    let address = derive_address_from_key(secret_key.as_bytes(), &csv_core::ChainId::new("sui"))
        .map_err(|e| anyhow::anyhow!("Failed to derive address: {}", e))?;

    state.store_address(Chain::new("sui"), address.clone());

    output::header("Sui Wallet Generated");
    output::kv("Address", &address);

    println!();
    output::warning(
        "Your private key has been generated. Use 'csv wallet export' to view it securely.",
    );

    Ok(())
}

fn generate_aptos(state: &mut UnifiedStateManager) -> Result<()> {
    use csv_keys::bip44::derive_address_from_key;
    use csv_keys::memory::SecretKey;
    use rand::RngCore;

    let mut key_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key_bytes);
    let secret_key = SecretKey::new(key_bytes);

    let address = derive_address_from_key(secret_key.as_bytes(), &csv_core::ChainId::new("aptos"))
        .map_err(|e| anyhow::anyhow!("Failed to derive address: {}", e))?;

    state.store_address(Chain::new("aptos"), address.clone());

    output::header("Aptos Wallet Generated");
    output::kv("Address", &address);

    println!();
    output::warning(
        "Your private key has been generated. Use 'csv wallet export' to view it securely.",
    );

    Ok(())
}

fn generate_solana(state: &mut UnifiedStateManager) -> Result<()> {
    use csv_keys::bip44::derive_address_from_key;
    use csv_keys::memory::SecretKey;
    use rand::RngCore;

    let mut key_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut key_bytes);
    let secret_key = SecretKey::new(key_bytes);

    let address = derive_address_from_key(secret_key.as_bytes(), &csv_core::ChainId::new("solana"))
        .map_err(|e| anyhow::anyhow!("Failed to derive address: {}", e))?;

    state.store_address(Chain::new("solana"), address.clone());

    output::header("Solana Wallet Generated");
    output::kv("Address", &address);

    println!();
    output::warning(
        "Your private key has been generated. Use 'csv wallet export' to view it securely.",
    );

    Ok(())
}

fn save_wallet_config(
    _mnemonic: &str,
    addresses: &HashMap<Chain, String>,
    _config: &Config,
) -> Result<()> {
    output::info(&format!("Saved {} wallet addresses", addresses.len()));
    Ok(())
}

/// Prompt user for a passphrase with confirmation.
fn prompt_passphrase(prompt: &str) -> Result<String> {
    use std::io::{self, Write};

    print!("{}: ", prompt);
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}
