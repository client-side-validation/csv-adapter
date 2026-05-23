# @csv-protocol/sdk

TypeScript SDK for CSV (Client-Side Validation) Protocol — cross-chain digital sanads with single-use seals.

[![npm version](https://badge.fury.io/js/@csv-protocol%2Fsdk.svg)](https://www.npmjs.com/package/@csv-protocol/sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

## What is CSV?

**CSV (Client-Side Validation) Protocol** is a cross-chain verification primitive that enables:

- **Offline proof verification** — Verify ownership without internet connection
- **No trusted bridges** — Self-verifying proof bundles, no validator sets
- **Single-use seals** — Cryptographic guarantee of no double-spend
- **Cross-chain transfers** — Move assets between any chains that support single-use commitments

## Installation

```bash
npm install @csv-protocol/sdk
```

## Quick Start

### 1. Create a Client

```typescript
import { CsvClient } from '@csv-protocol/sdk';

const client = new CsvClient({
  defaultChain: 'bitcoin',
  network: 'signet',
});
```

### 2. Verify a Proof Bundle Offline

```typescript
// Load a proof bundle (from file, QR code, or API)
const bundleJson = await fetch('/api/proof/123').then(r => r.json());

// Verify completely offline — no RPC calls
const result = client.verifyProofBundleFromJson(bundleJson);

if (result.valid) {
  console.log('✅ Proof is valid!');
  console.log('Chain of origin:', result.chain);
  console.log('Block height:', result.blockHeight);
} else {
  console.log('❌ Proof verification failed:', result.error);
}
```

### 3. Create a Seal

```typescript
import { createSeal } from '@csv-protocol/sdk';

// Create a seal on Bitcoin
const seal = await createSeal({
  chain: 'bitcoin',
  value: 10000, // satoshis
});

console.log('Seal created:', seal.id);
```

### 4. Transfer Cross-Chain

```typescript
import { initiateTransfer } from '@csv-protocol/sdk';

// Transfer from Bitcoin to Solana
const transfer = await initiateTransfer({
  sanadId: 'abc123...',
  sourceChain: 'bitcoin',
  destinationChain: 'solana',
  destinationAddress: 'solana_address_here',
});

// Monitor the transfer
const status = await transfer.getStatus();
console.log('Transfer status:', status.state); // 'locked' | 'proof_built' | 'minted'
```

### 5. Build and Export Proof Bundle

```typescript
import { buildProofBundle } from '@csv-protocol/sdk';

const bundle = await buildProofBundle({
  sanadId: 'abc123...',
  chain: 'ethereum',
});

// Export as JSON for sharing
const bundleJson = JSON.stringify(bundle, null, 2);

// Or generate QR code for mobile verification
const qrCodeData = bundle.toQrCode();
```

## Features

### Offline Verification

The killer feature of CSV — verify proofs without any internet connection:

```typescript
import { ProofBundle, verifyOffline } from '@csv-protocol/sdk';

// On a phone with WiFi turned off:
const bundle = ProofBundle.fromQrCode(scanResult);
const result = verifyOffline(bundle);

// Result in under 200ms, zero API calls
console.log(result.valid ? '✅ Valid' : '❌ Invalid');
```

### Cross-Chain Support

Supported chains:

- **Bitcoin** (UTXO-based seals)
- **Ethereum** (storage slot seals via CSVLock.sol)
- **Sui** (object-based seals)
- **Aptos** (Move resource seals)
- **Solana** (PDA-based seals)

### TypeScript Types

Full type safety for all operations:

```typescript
import type {
  Chain,
  SealPoint,
  Sanad,
  ProofBundle,
  TransferStatus,
} from '@csv-protocol/sdk';
```

## WASM Crypto

For high-performance cryptographic operations, use the WASM module:

```typescript
import * as wasm from '@csv-protocol/sdk/wasm';

// Generate seal commitment in WASM
const commitment = wasm.generateCommitment(seed, nonce);

// Verify proof bundle in WASM
const valid = wasm.verifyProofBundle(bundleBytes);
```

## AI Agent Integration

Use with LangChain, Claude, or any MCP-compatible agent:

```typescript
import { CsvMcpClient } from '@csv-protocol/sdk/mcp';

const client = new CsvMcpClient();

// Agent can create seals
const seal = await client.createSeal('ethereum');

// Agent can verify proofs
const verified = await client.verifyProof(bundleJson);

// Agent can transfer cross-chain
const transfer = await client.transferSanad(sanadId, 'solana');
```

## API Reference

### `CsvClient`

Main entry point for SDK operations.

```typescript
const client = new CsvClient({
  defaultChain: 'bitcoin',    // Default chain for operations
  network: 'signet',          // Network (mainnet, testnet, signet, etc.)
  rpcEndpoint?: string,       // Optional custom RPC
});
```

Methods:

- `verifyProofBundleFromJson(bundle)` — Offline verification
- `createSeal(chain, value)` — Create new seal
- `transferSanad(sanadId, destination)` — Initiate cross-chain transfer
- `getSanadStatus(sanadId)` — Check sanad state
- `buildProofBundle(sanadId)` — Generate proof bundle

### `ProofBundle`

Self-contained cryptographic proof of sanad ownership.

```typescript
interface ProofBundle {
  sealRef: SealPoint;           // Seal identifier
  anchorRef: CommitAnchor;      // On-chain anchor
  inclusionProof: InclusionProof; // Merkle/MPT proof
  finalityProof: FinalityProof;   // Confirmation proof
  signatures: Signature[];        // Ownership signatures
  transitionDag: DAGSegment;      // State transitions
}
```

## Examples

See the `examples/` directory for complete working examples:

- `basic-verification.ts` — Simple offline proof verification
- `cross-chain-transfer.ts` — Bitcoin → Ethereum transfer
- `agent-integration.ts` — LangChain agent integration
- `gaming-item-transfer.ts` — Game asset passport example

## Contributing

Contributions welcome! Please read our [Contributing Guide](../../CONTRIBUTING.md).

## Security

This SDK handles cryptographic operations. For production use:

- Always verify proof bundles offline before accepting transfers
- Store private keys in secure key management systems
- Use the WASM crypto module for performance-critical operations

## License

MIT OR Apache-2.0

## Resources

- [GitHub Repository](https://github.com/client-side-validation/csv-adapter)
- [Documentation](https://docs.csv.dev)
- [Issue Tracker](https://github.com/client-side-validation/csv-adapter/issues)
