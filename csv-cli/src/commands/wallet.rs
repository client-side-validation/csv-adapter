//! Wallet management commands

use sha2::Digest;
use anyhow::Result;
use clap::Subcommand;


use crate::config::{Config, Chain, Network};
use crate::state::State;
use crate::output;

#[derive(Subcommand)]
pub enum WalletAction {
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
        WalletAction::Generate { chain, network } => cmd_generate(chain, network, config, state),
        WalletAction::Balance { chain, address } => cmd_balance(chain, address, config, state),
        WalletAction::Fund { chain, address } => cmd_fund(chain, address, config, state),
        WalletAction::Export { chain, format } => cmd_export(chain, format, config, state),
        WalletAction::Import { chain, secret } => cmd_import(chain, secret, config, state),
        WalletAction::List => cmd_list(config, state),
    }
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
    let key = wallet.derive_key(&path)
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
    use ed25519_dalek::SigningKey;
    use rand::RngCore;

    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);

    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();

    // Sui address: flag (0x00) + public key (32 bytes)
    let mut addr_bytes = vec![0x00];
    addr_bytes.extend_from_slice(verifying_key.as_bytes());

    // Use first 20 bytes as address hex
    let address = format!("0x{}", hex::encode(&addr_bytes));

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

    let mut seed = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut seed);

    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();

    // Aptos address: 0x + 32-byte public key
    let address = format!("0x{}", hex::encode(verifying_key.as_bytes()));

    state.store_address(Chain::Aptos, address.clone());

    output::header("Aptos Wallet Generated");
    output::kv("Address", &address);
    output::kv_hash("Private Key", &seed);

    println!();
    output::warning("Save this private key securely.");
    output::info("Fund this wallet with: csv wallet fund --chain aptos");

    Ok(())
}

fn cmd_balance(chain: Chain, address: Option<String>, config: &Config, state: &State) -> Result<()> {
    let addr = address.or_else(|| state.get_address(&chain).cloned())
        .ok_or_else(|| anyhow::anyhow!("No address for {}. Generate or import a wallet first.", chain))?;

    output::header(&format!("Balance: {} ({})", chain, addr));

    match chain {
        Chain::Bitcoin => {
            let chain_config = config.chain(&chain)?;
            let url = format!("{}/address/{}/utxo", chain_config.rpc_url.trim_end_matches('/'), addr);
            match reqwest::blocking::get(&url)?.json::<serde_json::Value>() {
                Ok(utxos) => {
                    if let Some(arr) = utxos.as_array() {
                        let total_sat: u64 = arr.iter()
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
            let resp = reqwest::blocking::Client::new()
                .post(&chain_config.rpc_url)
                .json(&rpc_req)
                .send()?
                .json::<serde_json::Value>()?;

            if let Some(balance_hex) = resp.get("result").and_then(|r| r.as_str()) {
                let balance_wei = u64::from_str_radix(balance_hex.trim_start_matches("0x"), 16).unwrap_or(0);
                let balance_eth = balance_wei as f64 / 1e18;
                output::kv("Balance (ETH)", &format!("{:.6}", balance_eth));
                output::kv("Balance (wei)", &balance_wei.to_string());
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
            let resp = reqwest::blocking::Client::new()
                .post(&chain_config.rpc_url)
                .json(&rpc_req)
                .send()?
                .json::<serde_json::Value>()?;

            if let Some(result) = resp.get("result") {
                if let Some(balance) = result.get("totalBalance").and_then(|b| b.as_str()) {
                    let balance_sui: u64 = balance.parse().unwrap_or(0);
                    let balance_display = balance_sui as f64 / 1e9;
                    output::kv("Balance (SUI)", &format!("{:.4}", balance_display));
                    output::kv("Balance (MIST)", &balance.to_string());
                }
            }
        }
        Chain::Aptos => {
            let chain_config = config.chain(&chain)?;
            let url = format!("{}/accounts/{}/balances/0x1::aptos_coin::AptosCoin",
                chain_config.rpc_url.trim_end_matches('/'), addr);
            match reqwest::blocking::get(&url)?.json::<serde_json::Value>() {
                Ok(info) => {
                    if let Some(balance) = info.get("amount").and_then(|b| b.as_str()) {
                        let balance_oct: u64 = balance.parse().unwrap_or(0);
                        let balance_apt = balance_oct as f64 / 1e8;
                        output::kv("Balance (APT)", &format!("{:.4}", balance_apt));
                        output::kv("Balance (Octas)", &balance.to_string());
                    } else {
                        output::warning("No balance found");
                    }
                }
                Err(e) => output::error(&format!("Failed to fetch balance: {}", e)),
            }
        }
    }

    Ok(())
}

fn cmd_fund(chain: Chain, address: Option<String>, config: &Config, state: &State) -> Result<()> {
    let addr = address.or_else(|| state.get_address(&chain).cloned())
        .ok_or_else(|| anyhow::anyhow!("No address for {}. Generate a wallet first.", chain))?;

    let faucet = config.faucet(&chain)
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
            output::info(&format!("Visit {} or use Alchemy/Cloudflare faucet", faucet.url));
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
            let url = format!("{}/mint?amount=100000000&address={}", faucet.url.trim_end_matches('/'), addr);
            let resp = reqwest::blocking::Client::new()
                .post(&url)
                .send()?;

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
    let addr = state.get_address(&chain)
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

    // For now, store the secret and derive an address
    // In production, properly derive addresses from the secret
    let address = if secret.starts_with("0x") {
        secret.clone()
    } else {
        format!("0x{}", hex::encode(sha2::Sha256::digest(secret.as_bytes())))
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
        let address = state.get_address(&chain)
            .map(|a| a.as_str())
            .unwrap_or("Not generated");

        let network = config.chain(&chain)
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
