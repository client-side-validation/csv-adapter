# Bitcoin Address Types - Quick Reference

## How to Distinguish Bitcoin Addresses

Bitcoin addresses have different prefixes depending on the **address type** and **network**:

### Address Type Prefixes

| Address Type | Description | Mainnet Prefix | Testnet Prefix |
|-------------|-------------|----------------|----------------|
| **P2PKH** (Legacy) | Pay-to-Public-Key-Hash | `1...` | `m...` or `n...` |
| **P2SH** (Legacy Multisig) | Pay-to-Script-Hash | `3...` | `2...` |
| **P2WPKH** (SegWit v0) | Native SegWit | `bc1q...` | `tb1q...` |
| **P2TR** (SegWit v1) | **Taproot** | `bc1p...` | `tb1p...` |

### Examples

```
P2PKH Mainnet:  1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa
P2PKH Testnet:  mipcBbFg9gMiCh81Kj8tqqdgoZub1ZJRfn

P2SH Mainnet:   3J98t1WpEZ73CNmQviecrnyiWrnqRhWNLy
P2SH Testnet:   2MzQwSSnBHWHqSAqtTVQ6v47XtaisrJa1Vc

P2WPKH Mainnet: bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4
P2WPKH Testnet: tb1qw508d6qejxtdg4y5r3zarvary0c5xw7kxmz2nq

P2TR Mainnet:   bc1p5d7rjq7g6rdk2yhzks9smlaqtedr4dekq08ge8ztwac72sfr9rusxg3297
P2TR Testnet:   tb1p5d7rjq7g6rdk2yhzks9smlaqtedr4dekq08ge8ztwac72sfr9rusxg3297
```

## What Changed in the Wallet

### Before (❌ Wrong)
```rust
// This was generating FAKE addresses!
format!("bc1q{}", hex::encode(&pubkey.serialize()[1..21]))
// Result: bc1q796aa130c9045955677544592641c53c2c68 (INVALID!)
```

### After (✅ Correct)
```rust
// Now derives REAL Taproot addresses using BIP-86
fn derive_taproot_address(&self, account: u32, index: u32) -> Result<String, String> {
    // BIP-86 path: m/86'/coin_type'/account'/0/index
    let path = format!("m/86'/{coin_type}'/{account}'/0/{index}");
    
    // Derive key, apply taproot tweak
    let (tweaked_pk, _) = xonly.tap_tweak(&secp, None);
    
    // Create proper P2TR address
    let address = Address::p2tr_tweaked(tweaked_pk, network);
    Ok(address.to_string())
}
```

## How to Tell Which Address Type You Have

### Quick Identification

1. **Starts with `1`** → Legacy P2PKH (old style)
2. **Starts with `3`** → Legacy P2SH (multisig)
3. **Starts with `bc1q`** → SegWit v0 (P2WPKH)
4. **Starts with `bc1p`** → **Taproot (P2TR)** ✨ *What you want*
5. **Starts with `tb1q`** → Testnet SegWit v0
6. **Starts with `tb1p`** → **Testnet Taproot (P2TR)** ✨ *What you want*

### In Code

```rust
fn identify_address_type(address: &str) -> &'static str {
    if address.starts_with("tb1p") {
        "Testnet Taproot (P2TR)"
    } else if address.starts_with("bc1p") {
        "Mainnet Taproot (P2TR)"
    } else if address.starts_with("tb1q") {
        "Testnet SegWit v0 (P2WPKH)"
    } else if address.starts_with("bc1q") {
        "Mainnet SegWit v0 (P2WPKH)"
    } else if address.starts_with("1") {
        "Mainnet Legacy (P2PKH)"
    } else if address.starts_with("m") || address.starts_with("n") {
        "Testnet Legacy (P2PKH)"
    } else if address.starts_with("3") {
        "Mainnet P2SH (Multisig)"
    } else if address.starts_with("2") {
        "Testnet P2SH (Multisig)"
    } else {
        "Unknown"
    }
}
```

## Network Configuration

The wallet now includes a `BitcoinNetwork` enum to select the network:

```rust
pub enum BitcoinNetwork {
    Mainnet,   // Real BTC - bc1p... addresses
    Testnet,   // Test BTC - tb1p... addresses
    Signet,    // Signet - tb1p... addresses
    Regtest,   // Local regression - bcrt1p... addresses
}
```

### Default: Testnet

By default, the wallet uses **Testnet**, which generates `tb1p...` addresses.

To switch to mainnet:

```rust
let wallet = ExtendedWallet::from_mnemonic(phrase)?
    .with_bitcoin_network(BitcoinNetwork::Mainnet);
```

## BIP-86 Derivation Path

The wallet uses **BIP-86** for Taproot key derivation:

```
m/86'/coin_type'/account'/change/address_index
```

Where:
- `86'` = BIP-86 purpose (Taproot)
- `coin_type'` = `0'` for mainnet, `1'` for testnet/signet/regtest
- `account'` = Account number (e.g., `0'` for first account)
- `change` = `0` for external (receiving), `1` for internal (change)
- `address_index` = Address index (e.g., `0`, `1`, `2`, ...)

### Examples

```
Mainnet first address:    m/86'/0'/0'/0/0
Mainnet second address:   m/86'/0'/0'/0/1
Testnet first address:    m/86'/1'/0'/0/0
Testnet second address:   m/86'/1'/0'/0/1
```

## Why Taproot?

Taproot (P2TR) is the **recommended** address type because:

✅ **Privacy**: All transactions look the same (can't distinguish simple payments from complex scripts)
✅ **Efficiency**: Smaller transaction size → lower fees
✅ **Flexibility**: Supports complex smart contracts while keeping simple payments cheap
✅ **Modern**: Latest Bitcoin upgrade (activated Nov 2021)
✅ **CSV Compatible**: Required for client-side validation protocols

## Testing Your Wallet

After the fix, your wallet should generate addresses like:

```bash
# Testnet (default)
Bitcoin Address: tb1p5d7rjq7g6rdk2yhzks9smlaqtedr4dekq08ge8ztwac72sfr9rusxg3297
                 ^^^^                                       ^^^^^^^^^^^^^^^^^^^^^^^^
                 Testnet Taproot prefix                    52-character witness program

# Mainnet (if configured)
Bitcoin Address: bc1p5d7rjq7g6rdk2yhzks9smlaqtedr4dekq08ge8ztwac72sfr9rusxg3297
                 ^^^^                                       ^^^^^^^^^^^^^^^^^^^^^^^^
                 Mainnet Taproot prefix                    52-character witness program
```

**Note**: Taproot addresses are always **62 characters** long (4 prefix + 58 witness program).

## Troubleshooting

### Problem: Still seeing `bc1q...` addresses

**Solution**: Make sure you're using the new wallet code with `derive_taproot_address()`. The old code was removed.

### Problem: Address doesn't start with `tb1p` or `bc1p`

**Solution**: The address is not a Taproot address. Check:
1. Is the wallet configured to use Taproot (P2TR)?
2. Is the `bitcoin` crate version 0.32+?
3. Is BIP-86 path being used (`m/86'/...`)?

### Problem: Getting derivation errors

**Solution**: Check that:
1. The seed is 64 bytes (512-bit)
2. The mnemonic phrase is valid
3. The derivation path is correctly formatted

## Summary

| What | Old (Wrong) | New (Correct) |
|------|-------------|---------------|
| **Type** | Fake `bc1q` address | Real Taproot (P2TR) |
| **Testnet** | `bc1q796aa...` | `tb1p5d7rjq...` |
| **Mainnet** | `bc1q796aa...` | `bc1p5d7rjq...` |
| **Method** | Hex encode pubkey | BIP-86 derivation + taproot tweak |
| **Length** | Random | Always 62 chars |
| **Valid** | ❌ No | ✅ Yes |
