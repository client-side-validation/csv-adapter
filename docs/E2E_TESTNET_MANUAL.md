# End-to-End Testnet Manual

Related docs: [Developer Guide](DEVELOPER_GUIDE.md), [Implementation Status](CROSS_CHAIN_IMPLEMENTATION.md), [Testnet E2E Report](TESTNET_E2E_REPORT.md)

> The operational steps below are preserved as a manual runbook. Use this together with the developer guide, not as the primary architecture or roadmap document.

---

## Prerequisites

```bash
cargo build -p csv-cli --release
foundryup
sui --version
aptos --version
```

## Step 1: Build the CLI

```bash
cargo build -p csv-cli --release
./target/release/csv --help
```

## Step 2: Generate Wallets

### Bitcoin (Signet)

```bash
csv wallet generate --chain bitcoin --network signet
```

### Ethereum (Sepolia)

```bash
csv wallet generate --chain ethereum --network sepolia
```

### Sui (Testnet)

```bash
csv wallet generate --chain sui --network testnet
```

### Aptos (Testnet)

```bash
csv wallet generate --chain aptos --network testnet
```

### Verify all wallets

```bash
csv wallet list
```

## Step 3: Fund Wallets

### Bitcoin Signet

Use a Signet faucet and confirm the generated address received funds.

### Ethereum Sepolia

Use a Sepolia faucet for the generated wallet address.

### Sui Testnet

```bash
csv wallet fund --chain sui
```

### Aptos Testnet

```bash
csv wallet fund --chain aptos
```

### Verify balances

```bash
csv wallet balance --chain bitcoin
csv wallet balance --chain ethereum
csv wallet balance --chain sui
csv wallet balance --chain aptos
```

## Step 4: Deploy Contracts

### Ethereum

```bash
csv contract deploy --chain ethereum --network sepolia
```

### Sui

```bash
csv contract deploy --chain sui --network testnet
```

### Aptos

```bash
csv contract deploy --chain aptos --network testnet
```

## Step 5: Run cross-chain flows

Example:

```bash
csv cross-chain transfer --from bitcoin --to sui --right-id 0x...
```

## Step 6: Run the test command set

```bash
csv test run-all
```

## Troubleshooting

- If balances are missing, verify faucet delivery before debugging protocol logic.
- If deployment fails, verify the chain CLI and network selection first.
- If transfer verification fails, compare the failure against the notes in [Testnet E2E Report](TESTNET_E2E_REPORT.md).

## Notes

- This manual is intentionally operational and concise.
- Update [Developer Guide](DEVELOPER_GUIDE.md) for normal contributor workflows and keep this file focused on manual testnet execution.
