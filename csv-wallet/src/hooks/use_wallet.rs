//! Wallet state hook.

use crate::wallet_core::{WalletData as Wallet, ChainAccount};
use csv_adapter_core::Chain;
use csv_adapter_keystore::{
    bip39::{Mnemonic, MnemonicType},
    bip44::derive_all_chain_keys,
    memory::Seed,
    browser_keystore::BrowserKeystore,
};
use dioxus::prelude::*;

/// Wallet state.
#[derive(Clone, PartialEq)]
pub struct WalletState {
    /// Whether wallet is initialized
    pub initialized: bool,
    /// Whether wallet is unlocked
    pub unlocked: bool,
    /// Current wallet
    pub wallet: Option<Wallet>,
    /// Wallet addresses
    pub addresses: std::collections::HashMap<csv_adapter_core::Chain, String>,
}

/// Wallet context.
#[derive(Clone)]
pub struct WalletContext {
    pub state: Signal<WalletState>,
}

impl WalletContext {
    /// Create a new wallet by generating a fresh mnemonic and deriving keys for all chains.
    pub fn create_wallet(&mut self, password: &str) -> Result<(Wallet, String), String> {
        // Generate a new 24-word mnemonic
        let mnemonic = Mnemonic::generate(MnemonicType::Words24);
        let phrase = mnemonic.as_str().to_string();
        
        // Convert to seed
        let seed = mnemonic.to_seed(None);
        
        // Create wallet with derived accounts
        let wallet = self.create_wallet_from_seed(&seed, password)?;
        
        // Store the mnemonic phrase (encrypted) in state for backup display
        self.state.write().wallet = Some(wallet.clone());
        self.state.write().unlocked = true;
        self.state.write().initialized = true;
        
        Ok((wallet, phrase))
    }
    
    /// Create wallet accounts from a seed.
    fn create_wallet_from_seed(&self, seed: &Seed, password: &str) -> Result<Wallet, String> {
        let mut wallet = Wallet::default();
        
        // Initialize browser keystore
        let mut keystore = BrowserKeystore::new()
            .map_err(|e| format!("Failed to initialize keystore: {}", e))?;
        
        // Derive keys for all supported chains
        let chain_keys = derive_all_chain_keys(seed.as_bytes(), 0);
        
        for (chain, secret_key) in chain_keys {
            // Store the encrypted key in browser keystore
            let keystore_ref = format!("{}_account_0", chain.to_string().to_lowercase());
            
            use csv_adapter_keystore::memory::Passphrase;
            let passphrase = Passphrase::new(password);
            
            keystore.store_key(
                &keystore_ref,
                &chain.to_string(),
                &secret_key,
                &passphrase,
            ).map_err(|e| format!("Failed to store key for {:?}: {}", chain, e))?;
            
            // Derive address from the key
            let private_key_hex = hex::encode(secret_key.as_bytes());
            let address = ChainAccount::derive_address(chain, &private_key_hex)
                .map_err(|e| format!("Failed to derive address for {:?}: {}", chain, e))?;
            
            // Create account with keystore reference
            let account = ChainAccount::from_keystore(
                chain,
                &format!("{:?} Account 1", chain),
                &address,
                &keystore_ref,
                Some("m/44'/0'/0'/0/0"), // Generic derivation path
            );
            
            wallet.add_account(account);
            
            // Store address in context
            self.state.write().addresses.insert(chain, address);
        }
        
        Ok(wallet)
    }

    /// Import a wallet from an existing mnemonic phrase.
    pub fn import_wallet(&mut self, mnemonic: &str, password: &str) -> Result<Wallet, String> {
        // Parse and validate the mnemonic
        let mnemonic = Mnemonic::from_phrase(mnemonic)
            .map_err(|e| format!("Invalid mnemonic: {}", e))?;
        
        // Convert to seed (no BIP-39 passphrase for now)
        let seed = mnemonic.to_seed(None);
        
        // Create wallet from seed
        let wallet = self.create_wallet_from_seed(&seed, password)?;
        
        self.state.write().wallet = Some(wallet.clone());
        self.state.write().unlocked = true;
        self.state.write().initialized = true;

        Ok(wallet)
    }
    
    /// Restore wallet from seed (used when keystore exists but wallet state needs rebuild).
    pub fn restore_from_keystore(&mut self, password: &str) -> Result<Wallet, String> {
        let mut keystore = BrowserKeystore::new()
            .map_err(|e| format!("Failed to initialize keystore: {}", e))?;
        
        let mut wallet = Wallet::default();
        
        // List all stored keys
        let key_ids = keystore.list_keys()
            .map_err(|e| format!("Failed to list keys: {}", e))?;
        
        if key_ids.is_empty() {
            return Err("No keys found in keystore".to_string());
        }
        
        use csv_adapter_keystore::memory::Passphrase;
        let passphrase = Passphrase::new(password);
        
        // Restore accounts from keystore
        for keystore_ref in key_ids {
            // Try to retrieve the key to verify password and get chain info
            let secret_key = keystore.retrieve_key(&keystore_ref, &passphrase)
                .map_err(|e| format!("Failed to retrieve key {}: {}", keystore_ref, e))?;
            
            // Parse chain from keystore_ref (format: "{chain}_account_{index}")
            let chain_str = keystore_ref.split('_').next().unwrap_or("unknown");
            let chain = match chain_str {
                "bitcoin" => Chain::Bitcoin,
                "ethereum" => Chain::Ethereum,
                "sui" => Chain::Sui,
                "aptos" => Chain::Aptos,
                "solana" => Chain::Solana,
                _ => continue, // Skip unknown chains
            };
            
            // Derive address from key
            let private_key_hex = hex::encode(secret_key.as_bytes());
            let address = ChainAccount::derive_address(chain, &private_key_hex)
                .map_err(|e| format!("Failed to derive address: {}", e))?;
            
            let account = ChainAccount::from_keystore(
                chain,
                &format!("{:?} Account", chain),
                &address,
                &keystore_ref,
                None,
            );
            
            wallet.add_account(account);
            self.state.write().addresses.insert(chain, address);
        }
        
        self.state.write().wallet = Some(wallet.clone());
        self.state.write().unlocked = true;
        self.state.write().initialized = true;
        
        Ok(wallet)
    }

    pub fn lock(&mut self) {
        self.state.write().unlocked = false;
    }

    pub fn unlock(&mut self, password: &str) -> Result<(), String> {
        // Verify password by attempting to retrieve a key from keystore
        let mut keystore = BrowserKeystore::new()
            .map_err(|e| format!("Failed to initialize keystore: {}", e))?;
        
        let key_ids = keystore.list_keys()
            .map_err(|e| format!("Failed to list keys: {}", e))?;
        
        if key_ids.is_empty() {
            // No keys in keystore, just mark as unlocked (new wallet)
            self.state.write().unlocked = true;
            return Ok(());
        }
        
        // Try to decrypt the first key to verify password
        use csv_adapter_keystore::memory::Passphrase;
        let passphrase = Passphrase::new(password);
        
        let first_key = &key_ids[0];
        keystore.retrieve_key(first_key, &passphrase)
            .map_err(|_| "Invalid password".to_string())?;
        
        // Password verified, mark as unlocked
        self.state.write().unlocked = true;
        
        // Start a session for key caching
        keystore.start_session();
        
        Ok(())
    }
    
    /// Lock the wallet and clear any cached keys.
    pub fn lock_wallet(&mut self) {
        // End any active keystore session
        if let Ok(mut keystore) = BrowserKeystore::new() {
            keystore.end_session();
        }
        
        self.state.write().unlocked = false;
    }
}

/// Wallet provider component.
#[component]
pub fn WalletProvider(children: Element) -> Element {
    let state = use_signal(|| WalletState {
        initialized: false,
        unlocked: false,
        wallet: None,
        addresses: std::collections::HashMap::new(),
    });

    use_context_provider(|| WalletContext { state });

    rsx! { { children } }
}

/// Hook to access wallet state.
pub fn use_wallet() -> WalletContext {
    use_context::<WalletContext>()
}
