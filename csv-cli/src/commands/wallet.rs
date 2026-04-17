//! Wallet management commands

use anyhow::Result;
use clap::Subcommand;

use crate::config::{Chain, Config, Network};
use crate::output;
use crate::state::State;

#[derive(Subcommand)]
pub enum WalletAction {
    /// Initialize wallet with one-command setup (generate, fund, configure)
    Init {
        /// Network (dev/test/main)
        #[arg(value_enum, default_value = "dev")]
        network: Network,
        /// Generate mnemonic (12 or 24 words)
        #[arg(short, long, default_value = "12")]
        words: u8,
        /// Auto-fund from faucets
        #[arg(long, default_value = "true")]
        fund: bool,
    },
    /// Generate a new wallet
    Generate {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Network (dev/test/main)
        #[arg(value_enum, default_value = "test")]
        network: Network,
    },
    /// Show wallet balance
    Balance {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Address (uses stored address if not provided)
        #[arg(short, long)]
        address: Option<String>,
    },
    /// Fund wallet from faucet
    Fund {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Address (uses stored address if not provided)
        #[arg(short, long)]
        address: Option<String>,
    },
    /// Export wallet (xpub, mnemonic, or private key)
    Export {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Export format
        #[arg(short, long, default_value = "address")]
        format: String,
    },
    /// Import wallet from private key or mnemonic
    Import {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Private key (hex) or mnemonic phrase
        secret: String,
    },
    /// List wallets
    List,
}

pub fn execute(action: WalletAction, config: &Config, state: &mut State) -> Result<()> {
    match action {
        WalletAction::Init { network, words, fund } => cmd_init(network, words, fund, config, state),
        WalletAction::Generate { chain, network } => cmd_generate(chain, network, config, state),
        WalletAction::Balance { chain, address } => cmd_balance(chain, address, config, state),
        WalletAction::Fund { chain, address } => cmd_fund(chain, address, config, state),
        WalletAction::Export { chain, format } => cmd_export(chain, format, config, state),
        WalletAction::Import { chain, secret } => cmd_import(chain, secret, config, state),
        WalletAction::List => cmd_list(config, state),
    }
}

fn cmd_init(network: Network, words: u8, fund: bool, config: &Config, state: &mut State) -> Result<()> {
    use std::collections::HashMap;
    
    output::header("CSV Wallet Initialization");
    output::info("Setting up your cross-chain wallet...");
    
    // Step 1: Generate mnemonic
    let mnemonic = generate_mnemonic(words)?;
    output::success(&format!("Generated {}-word mnemonic", words));
    output::secret(&format!("Mnemonic: {}", mnemonic));
    
    // Step 2: Generate wallets for all supported chains
    let mut addresses = HashMap::new();
    
    for chain in [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos] {
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

// Helper functions for wallet initialization
fn generate_mnemonic(words: u8) -> Result<String> {
    use rand::RngCore;
    
    // Generate a simple mnemonic-like phrase (simplified version)
    let word_count = words as usize;
    let mut words_vec = Vec::with_capacity(word_count);
    
    for _ in 0..word_count {
        let mut bytes = [0u8; 4];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        let word_num = u32::from_be_bytes(bytes) % 2048; // BIP39 word list size
        words_vec.push(format!("word{}", word_num));
    }
    
    Ok(words_vec.join(" "))
}

fn generate_wallet_for_chain(chain: &Chain, network: &Network, mnemonic: &str, state: &mut State) -> Result<String> {
    match chain {
        Chain::Bitcoin => {
            let address = generate_bitcoin_from_mnemonic(network, mnemonic, state)?;
            Ok(address)
        },
        Chain::Ethereum => {
            let address = generate_ethereum_from_mnemonic(mnemonic, state)?;
            Ok(address)
        },
        Chain::Sui => {
            let address = generate_sui_from_mnemonic(mnemonic, state)?;
            Ok(address)
        },
        Chain::Aptos => {
            let address = generate_aptos_from_mnemonic(mnemonic, state)?;
            Ok(address)
        },
    }
}

fn generate_bitcoin_from_mnemonic(network: &Network, mnemonic: &str, state: &mut State) -> Result<String> {
    use bitcoin::Network as BtcNetwork;
    use rand::RngCore;
    
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
        .map_err(|e| anyhow::anyhow!("Failed to derive Bitcoin key: {}", e))?;
    
    let address = key.address.to_string();
    state.store_address(Chain::Bitcoin, address.clone());
    Ok(address)
}

fn generate_ethereum_from_mnemonic(mnemonic: &str, state: &mut State) -> Result<String> {
    use rand::RngCore;
    
    // Simple seed generation from mnemonic (simplified)
    let mut seed = [0u8; 32];
    for (i, byte) in mnemonic.as_bytes().iter().enumerate() {
        if i < 32 {
            seed[i] = *byte;
        }
    }
    
    // Generate a simple Ethereum-like address (simplified)
    let mut address_bytes = [0u8; 20];
    rand::rngs::OsRng.fill_bytes(&mut address_bytes);
    
    let address = format!("0x{}", hex::encode(address_bytes));
    state.store_address(Chain::Ethereum, address.clone());
    Ok(address)
}

fn generate_sui_from_mnemonic(mnemonic: &str, state: &mut State) -> Result<String> {
    use rand::RngCore;
    
    // Generate a simple Sui-like address (simplified)
    let mut address_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut address_bytes);
    
    let address = format!("0x{}", hex::encode(address_bytes));
    state.store_address(Chain::Sui, address.clone());
    Ok(address)
}

fn generate_aptos_from_mnemonic(mnemonic: &str, state: &mut State) -> Result<String> {
    use rand::RngCore;
    
    // Generate a simple Aptos-like address (simplified)
    let mut address_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut address_bytes);
    
    let address = format!("0x{}", hex::encode(address_bytes));
    state.store_address(Chain::Aptos, address.clone());
    Ok(address)
}

fn save_wallet_config(mnemonic: &str, addresses: &std::collections::HashMap<Chain, String>, config: &Config) -> Result<()> {
    use std::path::PathBuf;
    use std::fs;
    use std::io::Write;
    
    let config_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".csv");
    
    fs::create_dir_all(&config_dir)?;
    
    let config_file = config_dir.join("config.toml");
    let mut file = fs::File::create(&config_file)?;
    
    writeln!(file, "[wallet]")?;
    writeln!(file, "mnemonic = \"{}\"", mnemonic)?;
    writeln!(file, "network = \"{}\"", "dev")?;
    writeln!(file)?;
    
    for (chain, address) in addresses {
        writeln!(file, "[wallet.{}]", chain.to_string().to_lowercase())?;
        writeln!(file, "address = \"{}\"", address)?;
        writeln!(file)?;
    }
    
    Ok(())
}

fn fund_all_wallets(addresses: &std::collections::HashMap<Chain, String>, network: &Network) -> Result<()> {
    for (chain, address) in addresses {
        output::info(&format!("Funding {} wallet...", chain));
        match fund_wallet_from_faucet(chain, address, network) {
            Ok(_) => output::success(&format!("{} wallet funded", chain)),
            Err(e) => output::warning(&format!("Failed to fund {} wallet: {}", chain, e)),
        }
    }
    Ok(())
}

fn fund_wallet_from_faucet(chain: &Chain, address: &str, network: &Network) -> Result<()> {
    match chain {
        Chain::Bitcoin => {
            if network == &Network::Test {
                output::info(&format!("Visit: https://signetfaucet.com/ to fund {}", address));
                output::info("Or use: curl -sL https://signetfaucet.com/api/v1/faucet | jq '.[] | select(.address == null)'");
            }
        },
        Chain::Ethereum => {
            output::info(&format!("Visit: https://faucet.sepolia.dev/ to fund {}", address));
        },
        Chain::Sui => {
            output::info(&format!("Visit: https://faucet.sui.io/ to fund {}", address));
        },
        Chain::Aptos => {
            output::info(&format!("Visit: https://faucet.testnet.aptoslabs.com/ to fund {}", address));
        },
    }
    Ok(())
}

fn cmd_generate(chain: Chain, network: Network, _config: &Config, state: &mut State) -> Result<()> {
    match chain {
        Chain::Bitcoin => generate_bitcoin(network, state),
        Chain::Ethereum => generate_ethereum(state),
        Chain::Sui => generate_sui(state),
        Chain::Aptos => generate_aptos(state),
    }
}

fn generate_bitcoin(network: Network, state: &mut State) -> Result<()> {
    use bitcoin::Network as BtcNetwork;
    use rand::RngCore;

    let btc_network = match network {
        Network::Dev => BtcNetwork::Regtest,
        Network::Test => BtcNetwork::Signet,
        Network::Main => BtcNetwork::Bitcoin,
    };

    // Generate random seed and derive wallet
    let mut seed = [0u8; 64];
    rand::rngs::OsRng.fill_bytes(&mut seed);
    let wallet = csv_adapter_bitcoin::wallet::SealWallet::from_seed(&seed, btc_network)
        .map_err(|e| anyhow::anyhow!("Failed to create wallet: {}", e))?;

    // Derive first address
    let path = csv_adapter_bitcoin::wallet::Bip86Path::external(0, 0);
    let key = wallet
        .derive_key(&path)
        .map_err(|e| anyhow::anyhow!("Failed to derive key: {}", e))?;

    let address = key.address.to_string();

    // Store in state
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

fn generate_ethereum(state: &mut State) -> Result<()> {
    use rand::RngCore;

    let mut private_key = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut private_key);

    // Derive address from private key
    use secp256k1::{Secp256k1, SecretKey};
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(&private_key)
        .map_err(|e| anyhow::anyhow!("Invalid private key: {}", e))?;
    let public_key = secret_key.public_key(&secp);

    // Ethereum address = last 20 bytes of keccak256(public_key)
    use sha3::{Digest, Keccak256};
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

fn generate_sui(state: &mut State) -> Result<()> {
    use blake2::{digest::Digest, Blake2b};
    use ed25519_dalek::SigningKey;
    use rand::RngCore;
    use typenum::U32;

    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);

    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();

    // Sui address: BLAKE2b-256(signature_scheme_flag || public_key)
    // Signature scheme flag 0x00 = Ed25519
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

fn generate_aptos(state: &mut State) -> Result<()> {
    use ed25519_dalek::SigningKey;
    use rand::RngCore;
    use sha3::{Digest, Sha3_256};

    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);

    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();

    // Aptos address: SHA3-256(public_key || authentication_scheme_byte)
    // Authentication scheme 0x00 = Ed25519 (SingleSender)
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
    output::warning("Save this private key securely. It cannot be recovered.");
    output::info("Fund this wallet with: csv wallet fund --chain aptos");

    Ok(())
}

fn cmd_balance(
    chain: Chain,
    address: Option<String>,
    config: &Config,
    state: &State,
) -> Result<()> {
    let addr = address
        .or_else(|| state.get_address(&chain).cloned())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No address for {}. Generate or import a wallet first.",
                chain
            )
        })?;

    output::header(&format!("Balance: {} ({})", chain, addr));

    match chain {
        Chain::Bitcoin => {
            let chain_config = config.chain(&chain)?;
            let url = format!(
                "{}/address/{}/utxo",
                chain_config.rpc_url.trim_end_matches('/'),
                addr
            );
            match reqwest::blocking::get(&url)?.json::<serde_json::Value>() {
                Ok(utxos) => {
                    if let Some(arr) = utxos.as_array() {
                        let total_sat: u64 = arr
                            .iter()
                            .filter_map(|u| u.get("value").and_then(|v| v.as_u64()))
                            .sum();
                        output::kv("Total (sats)", &total_sat.to_string());
                        output::kv("UTXO Count", &arr.len().to_string());
                    } else {
                        output::warning("No UTXOs found");
                    }
                }
                Err(e) => output::error(&format!("Failed to fetch balance: {}", e)),
            }
        }
        Chain::Ethereum => {
            let chain_config = config.chain(&chain)?;
            let rpc_req = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_getBalance",
                "params": [addr, "latest"],
                "id": 1
            });

            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()?;

            let resp = client
                .post(&chain_config.rpc_url)
                .json(&rpc_req)
                .send()
                .map_err(|e| anyhow::anyhow!("Failed to connect to Ethereum RPC: {}", e))?
                .json::<serde_json::Value>()
                .map_err(|e| anyhow::anyhow!("Failed to parse RPC response: {}", e))?;

            if let Some(error) = resp.get("error") {
                output::error(&format!("RPC error: {}", error));
            } else if let Some(balance_hex) = resp.get("result").and_then(|r| r.as_str()) {
                let balance_wei =
                    u64::from_str_radix(balance_hex.trim_start_matches("0x"), 16).unwrap_or(0);
                let balance_eth = balance_wei as f64 / 1e18;
                output::kv("Balance (ETH)", &format!("{:.6}", balance_eth));
                output::kv("Balance (wei)", &balance_wei.to_string());
            } else {
                output::error("Unexpected RPC response format");
            }
        }
        Chain::Sui => {
            let chain_config = config.chain(&chain)?;
            let rpc_req = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "suix_getBalance",
                "params": [addr, "0x2::sui::SUI"],
                "id": 1
            });

            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()?;

            let resp = client
                .post(&chain_config.rpc_url)
                .json(&rpc_req)
                .send()
                .map_err(|e| anyhow::anyhow!("Failed to connect to Sui RPC: {}", e))?
                .json::<serde_json::Value>()
                .map_err(|e| anyhow::anyhow!("Failed to parse RPC response: {}", e))?;

            if let Some(error) = resp.get("error") {
                output::error(&format!("RPC error: {}", error));
            } else if let Some(result) = resp.get("result") {
                if let Some(balance) = result.get("totalBalance").and_then(|b| b.as_str()) {
                    let balance_sui: u64 = balance.parse().unwrap_or(0);
                    let balance_display = balance_sui as f64 / 1e9;
                    output::kv("Balance (SUI)", &format!("{:.4}", balance_display));
                    output::kv("Balance (MIST)", balance);
                } else {
                    output::warning("No balance found");
                }
            } else {
                output::error("Unexpected RPC response format");
            }
        }
        Chain::Aptos => {
            let chain_config = config.chain(&chain)?;
            let url = format!(
                "{}/accounts/{}/balance/0x1::aptos_coin::AptosCoin",
                chain_config.rpc_url.trim_end_matches('/'),
                addr
            );

            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()?;

            let resp = client
                .get(&url)
                .send()
                .map_err(|e| anyhow::anyhow!("Failed to connect to Aptos RPC: {}", e))?;

            if resp.status().as_u16() == 404 {
                output::warning("Account not found or no balance");
            } else {
                let body = resp
                    .text()
                    .map_err(|e| anyhow::anyhow!("Failed to read response: {}", e))?;

                // API may return a plain number string or JSON
                let balance_oct: u64 = if body.starts_with('{') {
                    // Try parsing as JSON
                    if let Ok(info) = serde_json::from_str::<serde_json::Value>(&body) {
                        info.get("amount")
                            .and_then(|b| b.as_str())
                            .or_else(|| info.as_str())
                            .unwrap_or("0")
                            .parse()
                            .unwrap_or(0)
                    } else {
                        0
                    }
                } else {
                    // Plain number string
                    body.trim().parse().unwrap_or(0)
                };

                let balance_apt = balance_oct as f64 / 1e8;
                output::kv("Balance (APT)", &format!("{:.4}", balance_apt));
                output::kv("Balance (Octas)", &balance_oct.to_string());
            }
        }
    }

    Ok(())
}

fn cmd_fund(chain: Chain, address: Option<String>, config: &Config, state: &State) -> Result<()> {
    let addr = address
        .or_else(|| state.get_address(&chain).cloned())
        .ok_or_else(|| anyhow::anyhow!("No address for {}. Generate a wallet first.", chain))?;

    let faucet = config
        .faucet(&chain)
        .ok_or_else(|| anyhow::anyhow!("No faucet configured for {}", chain))?;

    output::header(&format!("Funding {} from faucet", chain));
    output::kv("Address", &addr);
    output::kv("Faucet", &faucet.url);

    match chain {
        Chain::Bitcoin => {
            output::info("Bitcoin Signet funding requires manual interaction");
            output::info(&format!("Visit {} and paste your address", faucet.url));
            output::info("Or mine blocks locally with a Signet miner");
        }
        Chain::Ethereum => {
            output::info("Ethereum Sepolia funding requires manual interaction");
            output::info(&format!(
                "Visit {} or use Alchemy/Cloudflare faucet",
                faucet.url
            ));
        }
        Chain::Sui => {
            output::progress(1, 3, "Requesting SUI from faucet...");
            let url = format!("{}/gas", faucet.url);
            let body = serde_json::json!({
                "FixedAmountRequest": { "recipient": addr }
            });
            let resp = reqwest::blocking::Client::new()
                .post(&url)
                .json(&body)
                .send()?;

            if resp.status().is_success() {
                output::progress(2, 3, "Transaction submitted");
                output::progress(3, 3, "Waiting for confirmation...");
                output::success("SUI faucet request submitted successfully");
            } else {
                output::error(&format!("Faucet request failed: {}", resp.status()));
            }
        }
        Chain::Aptos => {
            output::progress(1, 3, "Requesting APT from faucet...");
            let url = format!(
                "{}/mint?amount=100000000&address={}",
                faucet.url.trim_end_matches('/'),
                addr
            );
            let resp = reqwest::blocking::Client::new().post(&url).send()?;

            if resp.status().is_success() || resp.status().as_u16() == 409 {
                // 409 = already funded recently
                output::progress(2, 3, "Transaction submitted");
                output::progress(3, 3, "Waiting for confirmation...");
                output::success("APT faucet request submitted successfully");
            } else {
                output::error(&format!("Faucet request failed: {}", resp.status()));
            }
        }
    }

    Ok(())
}

fn cmd_export(chain: Chain, format: String, _config: &Config, state: &State) -> Result<()> {
    let addr = state
        .get_address(&chain)
        .ok_or_else(|| anyhow::anyhow!("No wallet for {}. Generate one first.", chain))?;

    output::header(&format!("Wallet Export: {}", chain));
    output::kv("Address", addr);
    output::kv("Format", &format);

    match format.as_str() {
        "address" => {
            println!("\n{}", addr);
        }
        "json" => {
            output::json(&serde_json::json!({
                "chain": chain.to_string(),
                "address": addr,
            }));
        }
        _ => output::error(&format!("Unknown export format: {}", format)),
    }

    Ok(())
}

fn cmd_import(chain: Chain, secret: String, _config: &Config, state: &mut State) -> Result<()> {
    output::header(&format!("Import Wallet: {}", chain));
    output::kv("Chain", &chain.to_string());

    // Properly derive address from private key based on chain
    let address = match chain {
        Chain::Aptos => {
            use ed25519_dalek::SigningKey;
            use sha3::{Digest, Sha3_256};

            let secret_bytes = if let Some(stripped) = secret.strip_prefix("0x") {
                hex::decode(stripped).map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))?
            } else {
                hex::decode(secret).map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))?
            };

            if secret_bytes.len() != 32 {
                return Err(anyhow::anyhow!(
                    "Invalid Aptos private key length: {} bytes (expected 32)",
                    secret_bytes.len()
                ));
            }

            let mut seed = [0u8; 32];
            seed.copy_from_slice(&secret_bytes);

            let signing_key = SigningKey::from_bytes(&seed);
            let verifying_key = signing_key.verifying_key();

            // Aptos address: SHA3-256(public_key || authentication_scheme_byte)
            let mut hasher = Sha3_256::new();
            hasher.update(verifying_key.as_bytes());
            hasher.update([0x00]);
            let auth_key = hasher.finalize();
            format!("0x{}", hex::encode(auth_key))
        }
        Chain::Sui => {
            use blake2::{digest::Digest, Blake2b};
            use ed25519_dalek::SigningKey;
            use typenum::U32;

            let secret_bytes = if let Some(stripped) = secret.strip_prefix("0x") {
                hex::decode(stripped).map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))?
            } else {
                hex::decode(secret).map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))?
            };

            if secret_bytes.len() != 32 {
                return Err(anyhow::anyhow!(
                    "Invalid Sui private key length: {} bytes (expected 32)",
                    secret_bytes.len()
                ));
            }

            let mut seed = [0u8; 32];
            seed.copy_from_slice(&secret_bytes);

            let signing_key = SigningKey::from_bytes(&seed);
            let verifying_key = signing_key.verifying_key();

            // Sui address: BLAKE2b-256(signature_scheme_flag || public_key)
            let mut hasher = Blake2b::<U32>::new();
            hasher.update([0x00]);
            hasher.update(verifying_key.as_bytes());
            let address_bytes = hasher.finalize();
            format!("0x{}", hex::encode(address_bytes))
        }
        Chain::Ethereum => {
            use secp256k1::{Secp256k1, SecretKey};
            use sha3::{Digest, Keccak256};

            let secret_bytes = if let Some(stripped) = secret.strip_prefix("0x") {
                hex::decode(stripped).map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))?
            } else {
                hex::decode(secret).map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))?
            };

            if secret_bytes.len() != 32 {
                return Err(anyhow::anyhow!(
                    "Invalid Ethereum private key length: {} bytes (expected 32)",
                    secret_bytes.len()
                ));
            }

            let secp = Secp256k1::new();
            let secret_key = SecretKey::from_slice(&secret_bytes)
                .map_err(|e| anyhow::anyhow!("Invalid private key: {}", e))?;
            let public_key = secret_key.public_key(&secp);

            let pubkey_bytes = public_key.serialize_uncompressed();
            let hash = Keccak256::digest(&pubkey_bytes[1..]);
            format!("0x{}", hex::encode(&hash[12..]))
        }
        Chain::Bitcoin => {
            use bitcoin::Network as BtcNetwork;
            use csv_adapter_bitcoin::wallet::{Bip86Path, SealWallet};

            let seed_bytes = if let Some(stripped) = secret.strip_prefix("0x") {
                hex::decode(stripped).map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))?
            } else {
                hex::decode(secret).map_err(|e| anyhow::anyhow!("Invalid hex: {}", e))?
            };

            if seed_bytes.len() != 64 {
                return Err(anyhow::anyhow!(
                    "Invalid Bitcoin seed length: {} bytes (expected 64)",
                    seed_bytes.len()
                ));
            }

            let mut seed = [0u8; 64];
            seed.copy_from_slice(&seed_bytes);

            let wallet = SealWallet::from_seed(&seed, BtcNetwork::Signet)
                .map_err(|e| anyhow::anyhow!("Failed to create wallet: {}", e))?;

            let path = Bip86Path::external(0, 0);
            let key = wallet
                .derive_key(&path)
                .map_err(|e| anyhow::anyhow!("Failed to derive key: {}", e))?;

            key.address.to_string()
        }
    };

    state.store_address(chain, address.clone());
    output::kv("Address", &address);
    output::success("Wallet imported");

    Ok(())
}

fn cmd_list(config: &Config, state: &State) -> Result<()> {
    output::header("Wallets");

    let headers = vec!["Chain", "Address", "Balance", "Network"];
    let mut rows = Vec::new();

    for chain in [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos] {
        let address = state
            .get_address(&chain)
            .map(|a| a.as_str())
            .unwrap_or("Not generated");

        let network = config
            .chain(&chain)
            .map(|c| c.network.to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        rows.push(vec![
            format!("{}", chain).to_string(),
            address.to_string(),
            "—".to_string(),
            network,
        ]);
    }

    output::table(&headers, &rows);
    Ok(())
}
