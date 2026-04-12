# CSV Adapter — Developer Experience Blueprint

> **Vision:** Make cross-chain client-side validation as easy as `npm install` and `cargo add`
> **Target:** 5-minute time-to-first-successful-transfer · Zero-config local development · Self-documenting APIs

---

## Executive Summary

This blueprint defines the complete developer experience strategy for CSV Adapter. It prioritizes two primary personas:

1. **TypeScript/Web Developers** — Building dApps, frontends, and browser-based applications
2. **Rust Core Developers** — Building libraries, services, infrastructure, and chain integrations

**Core Philosophy:** *Every developer interaction with CSV should feel magical, not mechanical.*

---

## Table of Contents

1. [Developer Personas](#1-developer-personas)
2. [Getting Started Experience](#2-getting-started-experience)
3. [API Design Principles](#3-api-design-principles)
4. [TypeScript SDK Specification](#4-typescript-sdk-specification)
5. [Rust SDK Enhancements](#5-rust-sdk-enhancements)
6. [Developer Tooling Ecosystem](#6-developer-tooling-ecosystem)
7. [Local Development Environment](#7-local-development-environment)
8. [Testing & Debugging Experience](#8-testing--debugging-experience)
9. [Documentation Strategy](#9-documentation-strategy)
10. [Error Handling & Diagnostics](#10-error-handling--diagnostics)
11. [Onboarding Journey](#11-onboarding-journey)
12. [Implementation Roadmap](#12-implementation-roadmap)
13. [Success Metrics](#13-success-metrics)

---

## 1. Developer Personas

### 1.1 TypeScript/Web Developer — "Maya"

| Attribute | Detail |
|-----------|--------|
| **Role** | Full-stack Web3 developer |
| **Experience** | 2-5 years with TypeScript, React, ethers.js/viem |
| **Goal** | Add cross-chain NFT transfers to existing marketplace |
| **Pain Points** | Complex proof generation, multi-chain RPC, wallet integration |
| **Success Metric** | "I shipped cross-chain transfers in one sprint" |
| **Preferred Tools** | npm, TypeScript, React, Next.js, MetaMask/RainbowKit |

**Maya's Ideal Flow:**

```bash
npm install @csv-adapter/sdk
# 5 minutes later...
await csv.transfer({ rightId, from: 'sui', to: 'ethereum', to: '0x...' })
```

### 1.2 Rust Core Developer — "Alex"

| Attribute | Detail |
|-----------|--------|
| **Role** | Systems/infrastructure engineer |
| **Experience** | 3-8 years with Rust, distributed systems |
| **Goal** | Build CSV-based backend service for cross-chain lending |
| **Pain Points** | Chain-specific proof formats, RPC client complexity, state management |
| **Success Metric** | "My service handles 1000 cross-chain verifications/sec" |
| **Preferred Tools** | cargo, tokio, serde, SQLx, gRPC |

**Alex's Ideal Flow:**

```toml
[dependencies]
csv-adapter = { version = "0.3", features = ["all-chains", "tokio"] }
```

```rust
use csv_adapter::prelude::*;

let client = CsvClient::builder()
    .with_chain(Bitcoin::mainnet())
    .with_chain(Ethereum::mainnet())
    .build()?;

let right = client.rights().create(commitment).await?;
let transfer = client.transfers().cross_chain(&right, Chain::Ethereum).await?;
```

### 1.3 AI Agent — "Ava"

| Attribute | Detail |
|-----------|--------|
| **Type** | Autonomous agent (Cursor, Claude, Copilot, custom n8n/Make workflows) |
| **Experience** | N/A — relies on tool definitions, API specs, and code context |
| **Goal** | Execute cross-chain operations from natural language instructions |
| **Pain Points** | Ambiguous APIs, missing tool definitions, unstructured error messages |
| **Success Metric** | "I completed the user's request without needing clarification" |
| **Preferred Inputs** | MCP tool specs, OpenAPI schemas, JSON Schema, TypeScript `.d.ts` files |

**Ava's Ideal Flow:**

```
User: "Transfer my NFT from Bitcoin to Ethereum"

Agent autonomously:
1. Reads csv-agent-spec.yaml (MCP tools)
2. Checks wallet balance → has funds ✓
3. Finds NFT Right ID → 0xabc123 ✓
4. Executes cross-chain transfer → initiated ✓
5. Polls status every 30s → progress updates ✓
6. Reports completion → "Done! Your NFT is on Ethereum: 0x789def" ✓
```

---

## 2. Getting Started Experience

### 2.1 The 5-Minute Rule

**Goal:** Any developer with Rust or Node.js installed can complete their first successful cross-chain transfer in under 5 minutes.

### 2.2 One-Command Setup

#### TypeScript Developer Path

```bash
# Single command creates everything
npx create-csv-app@latest my-crosschain-app

# What it does:
# ✓ Scaffolds Next.js + TypeScript project
# ✓ Installs @csv-adapter/sdk
# ✓ Generates test wallet with devnet funds
# ✓ Creates working cross-chain transfer example
# ✓ Starts local dev server with hot reload
```

#### Rust Developer Path

```bash
# Single command creates everything
cargo install csv-cli
csv init my-csv-service --template lending

# What it does:
# ✓ Creates Cargo project with csv-adapter dependencies
# ✓ Generates dev wallet with testnet configuration
# ✓ Scaffolds cross-chain lending service
# ✓ Includes docker-compose for local chain simulation
# ✓ Runs `cargo test` to verify setup
```

### 2.3 Interactive Tutorial

```bash
# Guided tutorial mode
csv tutorial cross-chain-basics

# Opens interactive terminal session:
# Step 1/7: Generate your wallet ✓
# Step 2/7: Claim test tokens on Signet ✓
# Step 3/7: Create your first Right ✓
# Step 4/7: Transfer it to Sui devnet ✓
# Step 5/7: Verify the proof locally ✓
# Step 6/7: Transfer back to Bitcoin ✓
# Step 7/7: Celebrate! 🎉 View your completion certificate
```

### 2.4 Browser-Based Playground

**Zero-install experience:**

```
https://play.csv.dev/

Features:
- In-browser Rust playground (WASM-based, like play.rust-lang.org)
- Pre-loaded with example code
- Simulated chains (no real RPC needed)
- Step-by-step guided exercises
- Shareable code snippets
- Export to GitHub repo
```

---

## 3. API Design Principles

### 3.1 Design Philosophy

| Principle | Description | Example |
|-----------|-------------|---------|
| **Progressive Disclosure** | Simple things are simple, complex things are possible | `csv.transfer()` for basics, `csv.transfer().with_custom_proof()` for advanced |
| **Sensible Defaults** | Works out of the box with zero config | Auto-detects network, generates wallet if missing |
| **Chain Agnostic** | Same API works across all chains | `from: 'bitcoin'` → `from: 'sui'` no code changes |
| **Explicit > Implicit** | Clear intent over magic | `await transfer.waitForCompletion()` not event listeners |
| **Typed Errors** | Every error is actionable | `CsvError::InsufficientFunds { available, required }` |
| **Async Native** | First-class async support | All I/O operations are async, no blocking |

### 3.2 API Surface Map

```
@csv-adapter/sdk (TypeScript)
├── CSV                    # Main entry point
│   ├── fromMnemonic()     # Initialize with existing wallet
│   ├── createDevWallet()  # Quick setup for development
│   ├── connectExtension() # Browser wallet integration
│   │
├── rights
│   ├── create()           # Create new Right
│   ├── get()              # Fetch Right by ID
│   ├── list()             # List all Rights in wallet
│   ├── transfer()         # Transfer on same chain
│   └── burn()             # Consume Right permanently
│
├── transfers
│   ├── crossChain()       # Initiate cross-chain transfer
│   ├── monitor()          # Watch transfer progress
│   └── history()          # Query past transfers
│
├── proofs
│   ├── generate()         # Create proof bundle
│   ├── verify()           # Verify proof locally
│   └── simulate()         # Test proof without chain interaction
│
├── chains
│   ├── Bitcoin            # Bitcoin-specific utilities
│   ├── Ethereum           # Ethereum-specific utilities
│   ├── Sui                # Sui-specific utilities
│   └── Aptos              # Aptos-specific utilities
│
└── utils
    ├── formatAddress()    # Human-readable addresses
    ├── parseAmount()      # Amount parsing with units
    └── validateRightId()  # Right ID validation
```

### 3.3 TypeScript API Examples

#### Simple Transfer (Maya's Flow)

```typescript
import { CSV, Chain } from '@csv-adapter/sdk';

// Initialize
const csv = await CSV.createDevWallet();

// Create a Right
const right = await csv.rights.create({
  value: { amount: 1000, currency: 'sats' },
});

// Transfer cross-chain
const transfer = await csv.transfers.crossChain({
  rightId: right.id,
  from: Chain.Bitcoin,
  to: Chain.Ethereum,
  toAddress: '0x742d35Cc6634C0532925a3b844Bc9e7595f2bD38',
});

// Wait for completion
const result = await transfer.waitForCompletion({ timeout: '5m' });
console.log(`✅ Right ${right.id} transferred to Ethereum`);
```

#### Browser Extension Integration

```typescript
import { CSV, Chain } from '@csv-adapter/sdk';

// Connect to CSV browser extension
const csv = await CSV.connectExtension();

// dApp requests cross-chain transfer
async function handlePurchase(listing: NftListing) {
  const transfer = await csv.transfers.crossChain({
    rightId: listing.rightId,
    from: listing.chain,
    to: csv.activeChain,
    toAddress: csv.address,
  });

  transfer.on('progress', (step) => {
    updateUI(step); // "Locking on Bitcoin...", "Generating proof...", "Minting on Ethereum..."
  });

  await transfer.waitForCompletion();
  showSuccess('NFT transferred!');
}
```

#### Advanced Custom Proof (Power User)

```typescript
import { CSV, Chain, ProofStrategy } from '@csv-adapter/sdk';

const csv = await CSV.fromMnemonic(mnemonic);

// Custom proof strategy for high-value transfer
const transfer = await csv.transfers.crossChain({
  rightId: highValueRight.id,
  from: Chain.Bitcoin,
  to: Chain.Ethereum,
  toAddress: vaultAddress,
  proof: {
    strategy: ProofStrategy.MaxSecurity,  // Waits for 6 confirmations
    includeMerkleProof: true,
    includeBlockHeader: true,
    customVerifier: async (proof) => {
      // Custom verification logic
      return verifyWithExternalOracle(proof);
    },
  },
});
```

### 3.4 Rust API Examples

#### Service Builder Pattern (Alex's Flow)

```rust
use csv_adapter::prelude::*;

#[tokio::main]
async fn main() -> Result<(), CsvError> {
    // Build client with multiple chains
    let client = CsvClient::builder()
        .with_chain(Bitcoin::mainnet().with_rpc("https://mempool.space/api")?)
        .with_chain(Ethereum::mainnet().with_rpc("https://eth.llamarpc.com")?)
        .with_chain(Sui::mainnet()?)
        .with_wallet(Wallet::from_mnemonic(std::env::var("MNEMONIC")?)?)
        .with_store(SqliteStore::new("data/csv.db").await?)
        .build()?;

    // Create a Right
    let right = client.rights()
        .create(Commitment::from_hash(b"metadata_uri".as_slice()))
        .with_chain(Chain::Sui)
        .await?;

    // Cross-chain transfer
    let transfer = client.transfers()
        .cross_chain(&right.id, Chain::Ethereum)
        .to_address("0x742d35Cc6634C0532925a3b844Bc9e7595f2bD38".parse()?)
        .execute()
        .await?;

    println!("Transfer initiated: {}", transfer.id);

    // Monitor progress
    let mut watcher = transfer.watch();
    while let Some(event) = watcher.next().await {
        println!("Progress: {:?}", event);
    }

    Ok(())
}
```

#### Trait-Based Abstraction

```rust
use csv_adapter::prelude::*;

// Define your own right type
#[derive(Debug, Serialize, Deserialize)]
struct GameItem {
    item_id: Uuid,
    stats: ItemStats,
    owner: PublicKey,
}

// Implement CsvRight trait
impl CsvRight for GameItem {
    fn right_id(&self) -> Hash {
        Hash::from_slice(&[self.item_id.as_bytes(), self.stats.hash()].concat())
    }

    fn commitment(&self) -> Commitment {
        Commitment::from_serializable(&self)
    }
}

// Use with CSV adapter
let game_item = GameItem { /* ... */ };
let right = client.rights().register(game_item).await?;
```

---

## 4. TypeScript SDK Specification

### 4.1 Package Structure

```
@csv-adapter/sdk/
├── package.json
├── tsconfig.json
├── README.md
├── src/
│   ├── index.ts                 # Main exports
│   ├── csv.ts                   # CSV main class
│   ├── wallet.ts                # Wallet management
│   ├── rights.ts                # Right lifecycle
│   ├── transfers.ts             # Cross-chain transfers
│   ├── proofs.ts                # Proof generation/verification
│   ├── chains/
│   │   ├── index.ts
│   │   ├── bitcoin.ts           # Bitcoin provider
│   │   ├── ethereum.ts          # Ethereum provider
│   │   ├── sui.ts               # Sui provider
│   │   └── aptos.ts             # Aptos provider
│   ├── utils/
│   │   ├── format.ts            # Address/amount formatting
│   │   ├── validation.ts        # Input validation
│   │   └── crypto.ts            # Cryptographic utilities
│   ├── errors.ts                # Typed error classes
│   └── types.ts                 # TypeScript type definitions
├── test/
│   ├── unit/                    # Unit tests
│   ├── integration/             # Integration tests with mock chains
│   └── e2e/                     # End-to-end tests with testnets
└── dist/
    ├── cjs/                     # CommonJS build
    ├── esm/                     # ESM build
    └── types/                   # TypeScript declarations
```

### 4.2 Build Configuration

```json
{
  "name": "@csv-adapter/sdk",
  "version": "0.1.0",
  "main": "./dist/cjs/index.js",
  "module": "./dist/esm/index.js",
  "types": "./dist/types/index.d.ts",
  "exports": {
    ".": {
      "import": "./dist/esm/index.js",
      "require": "./dist/cjs/index.js",
      "types": "./dist/types/index.d.ts"
    },
    "./chains": {
      "import": "./dist/esm/chains/index.js",
      "require": "./dist/cjs/chains/index.js"
    },
    "./utils": {
      "import": "./dist/esm/utils/index.js",
      "require": "./dist/cjs/utils/index.js"
    }
  },
  "sideEffects": false,
  "files": ["dist"]
}
```

### 4.3 TypeScript Types

```typescript
// Core types
export type Chain = 'bitcoin' | 'ethereum' | 'sui' | 'aptos';

export interface Right {
  id: string;          // right_id (hash)
  commitment: string;  // commitment hash
  chain: Chain;        // which chain enforces the seal
  createdAt: Date;
  metadata?: Record<string, unknown>;
}

export interface Transfer {
  id: string;
  rightId: string;
  from: Chain;
  to: Chain;
  toAddress: string;
  status: TransferStatus;
  proof?: ProofBundle;
  createdAt: Date;
  completedAt?: Date;
}

export type TransferStatus =
  | 'initiated'
  | 'locking'
  | 'locked'
  | 'generating-proof'
  | 'proof-generated'
  | 'submitting-proof'
  | 'verifying'
  | 'minting'
  | 'completed'
  | 'failed';

export interface ProofBundle {
  inclusion: InclusionProof;
  finality: FinalityProof;
  sealConsumption: SealConsumptionProof;
}

// Typed errors
export class CsvError extends Error {
  constructor(
    public code: ErrorCode,
    message: string,
    public details?: Record<string, unknown>
  ) {
    super(message);
    this.name = 'CsvError';
  }
}

export enum ErrorCode {
  INSUFFICIENT_FUNDS = 'INSUFFICIENT_FUNDS',
  INVALID_RIGHT_ID = 'INVALID_RIGHT_ID',
  CHAIN_NOT_SUPPORTED = 'CHAIN_NOT_SUPPORTED',
  PROOF_VERIFICATION_FAILED = 'PROOF_VERIFICATION_FAILED',
  RPC_TIMEOUT = 'RPC_TIMEOUT',
  WALLET_NOT_CONNECTED = 'WALLET_NOT_CONNECTED',
  // ... more specific codes
}
```

### 4.4 WASM Bindings

For browser-only use without Node.js:

```typescript
// @csv-adapter/wasm
import init, { CsvWasm } from '@csv-adapter/wasm';

// Initialize WASM module
await init();

const csv = CsvWasm.new();

// All cryptography happens in WASM (faster, safer)
const wallet = csv.generate_wallet();
const proof = csv.generate_proof(right_id, chain);
const valid = csv.verify_proof(proof);
```

---

## 5. Rust SDK Enhancements

### 5.1 Unified Client

Current state requires managing multiple adapters. Goal: single entry point.

```rust
// BEFORE (current)
use csv_adapter_bitcoin::BitcoinAnchorLayer;
use csv_adapter_ethereum::EthereumAnchorLayer;
use csv_adapter_sui::SuiAnchorLayer;

let bitcoin = BitcoinAnchorLayer::mainnet()?;
let ethereum = EthereumAnchorLayer::mainnet()?;
// Manual coordination...

// AFTER (proposed)
use csv_adapter::prelude::*;

let client = CsvClient::builder()
    .with_chain(Bitcoin::mainnet()?)
    .with_chain(Ethereum::mainnet()?)
    .build()?;

// Unified API
let right = client.rights().create(commitment).on(Chain::Bitcoin).await?;
client.transfers().cross_chain(&right, Chain::Ethereum).await?;
```

### 5.2 Feature Flags

```toml
[features]
default = ["core"]

# Chain features
bitcoin = ["dep:csv-adapter-bitcoin"]
ethereum = ["dep:csv-adapter-ethereum"]
sui = ["dep:csv-adapter-sui"]
aptos = ["dep:csv-adapter-aptos"]
all-chains = ["bitcoin", "ethereum", "sui", "aptos"]

# Runtime features
tokio = ["dep:tokio", "dep:reqwest"]
async-std = ["dep:async-std", "dep:surf"]

# Storage features
sqlite = ["dep:csv-adapter-store", "dep:sqlx"]
rocksdb = ["dep:rocksdb"]
in-memory = []

# Advanced features
zk-proofs = ["dep:winterfell"]
fraud-proofs = []
post-quantum = ["dep:dilithium"]
```

### 5.3 Builder Pattern for All Config

```rust
// Wallet builder
let wallet = Wallet::builder()
    .with_mnemonic(mnemonic)
    .or_generate()
    .with_encryption(passphrase)
    .with_derivation_path("m/86'/0'/0'/0/0")
    .build()?;

// Transfer builder
let transfer = client.transfers()
    .builder(right_id)
    .to_chain(Chain::Ethereum)
    .to_address(addr)
    .with_priority(Priority::High)  // Faster confirmations
    .with_custom_metadata(metadata)
    .execute()
    .await?;
```

### 5.4 Stream API for Real-Time Events

```rust
use futures::StreamExt;

// Watch all wallet events
let mut events = client.watch().events().await?;

while let Some(event) = events.next().await {
    match event {
        WalletEvent::RightCreated { right_id, chain } => {
            println!("New Right: {} on {:?}", right_id, chain);
        }
        WalletEvent::TransferProgress { transfer_id, step } => {
            println!("Transfer {} step: {:?}", transfer_id, step);
        }
        WalletEvent::TransferCompleted { transfer_id } => {
            println!("Transfer {} completed!", transfer_id);
        }
        WalletEvent::Error { error } => {
            eprintln!("Error: {}", error);
        }
    }
}
```

---

## 5.1 AI Agent SDK Specifications

### 5.1.1 MCP Server Package

```
@csv-adapter/mcp-server/
├── package.json
├── src/
│   ├── index.ts              # MCP server entry point
│   ├── tools/
│   │   ├── right.ts          # csv_right_create, csv_right_get, csv_right_list
│   │   ├── transfer.ts       # csv_transfer_cross_chain, csv_transfer_status
│   │   ├── proof.ts          # csv_proof_verify, csv_proof_generate
│   │   └── wallet.ts         # csv_wallet_balance, csv_wallet_list_chains
│   ├── utils/
│   │   ├── validation.ts     # Input validation for agent calls
│   │   └── formatting.ts     # Human-readable output formatting
│   └── config.ts             # Configuration management
└── README.md                 # Setup instructions for Claude/Cursor
```

### 5.1.2 Agent-Optimized API Features

| Feature | Human-Facing | Agent-Facing | Why It Matters |
|---------|--------------|--------------|----------------|
| **Error messages** | "Insufficient funds. Fund wallet at: <url>" | Structured `ErrorSuggestion` with `FixAction` | Agent can auto-fix |
| **Progress updates** | "Transfer 45% complete" | Structured `TransferStatus::GeneratingProof { progress_percent: 45 }` | Agent can parse and act |
| **API documentation** | docs.csv.dev with examples | OpenAPI spec + JSON Schema | Agent can read and generate clients |
| **Status reporting** | Emojis + natural language | Enum variants + structured data | Agent can pattern-match |
| **Code examples** | Interactive tutorials | Curated prompt+code pairs | Agent context injection |

### 5.1.3 Structured Tool Definitions (JSON Schema)

```json
{
  "tools": [
    {
      "name": "csv_transfer_cross_chain",
      "description": "Transfer a Right from one blockchain to another",
      "inputSchema": {
        "type": "object",
        "properties": {
          "right_id": {
            "type": "string",
            "pattern": "^0x[a-fA-F0-9]{64}$",
            "description": "The 32-byte Right ID to transfer"
          },
          "from_chain": {
            "type": "string",
            "enum": ["bitcoin", "ethereum", "sui", "aptos"],
            "description": "Source chain where Right currently exists"
          },
          "to_chain": {
            "type": "string",
            "enum": ["bitcoin", "ethereum", "sui", "aptos"],
            "description": "Destination chain to transfer to"
          },
          "destination_owner": {
            "type": "string",
            "description": "New owner address on destination chain"
          },
          "wait_for_completion": {
            "type": "boolean",
            "default": true,
            "description": "If true, poll until transfer completes or fails"
          },
          "timeout": {
            "type": "string",
            "pattern": "^\\d+[smh]$",
            "default": "10m",
            "description": "Maximum time to wait (e.g., '10m', '1h')"
          }
        },
        "required": ["right_id", "from_chain", "to_chain", "destination_owner"]
      },
      "outputSchema": {
        "type": "object",
        "properties": {
          "success": { "type": "boolean" },
          "transfer_id": { "type": "string" },
          "status": {
            "type": "string",
            "enum": ["completed", "failed", "timeout"]
          },
          "destination_transaction": { "type": "string" },
          "error": {
            "type": "object",
            "properties": {
              "code": { "type": "string" },
              "message": { "type": "string" },
              "suggested_fix": { "type": "string" }
            }
          }
        }
      }
    }
  ]
}
```

### 5.1.4 Agent Prompt Engineering Guidelines

```markdown
# For IDE Agents (Cursor, Copilot)

## System Context
You are a CSV Adapter expert. The CSV system implements client-side validation 
for cross-chain rights. Key concepts:

- **Right**: A transferrable claim anchored to a chain's single-use seal
- **Seal**: Chain-specific mechanism that enforces single-use (UTXO, Object, etc.)
- **Cross-chain transfer**: Right moves by consuming seal on source, proving on dest

## Code Generation Rules
1. ALWAYS use `csv_adapter::prelude::*` for Rust or `@csv-adapter/sdk` for TS
2. ALWAYS include error handling with typed errors
3. ALWAYS add comments explaining cross-chain mechanics
4. ALWAYS suggest testing with `@csv-adapter/testing`
5. NEVER expose private keys or mnemonics in example code

## Common Patterns

### Pattern 1: Simple Transfer
[Include working code example]

### Pattern 2: Transfer with Progress Monitoring
[Include working code example]

### Pattern 3: Batch Operations
[Include working code example]
```

### 5.1.5 Agent Testing Framework

```typescript
import { AgentTestEnvironment } from '@csv-adapter/testing/agents';

describe('CSV Agent Operations', () => {
  let env: AgentTestEnvironment;

  beforeEach(async () => {
    env = await AgentTestEnvironment.create({
      chains: [Chain.Bitcoin, Chain.Ethereum],
      fundWallets: true,
      mockRpc: true, // Fast, deterministic tests
    });
  });

  it('should complete transfer from natural language instruction', async () => {
    const agent = env.getAgent('alice');

    // Simulate user instruction
    const result = await agent.execute(
      'Transfer my NFT from Bitcoin to Ethereum wallet 0x742d...'
    );

    // Agent should:
    // 1. Find the NFT Right on Bitcoin
    // 2. Initiate cross-chain transfer
    // 3. Wait for completion
    // 4. Report success
    expect(result.success).toBe(true);
    expect(result.steps).toHaveLength(4);
    expect(result.steps[0].action).toBe('find_right');
    expect(result.steps[1].action).toBe('transfer_cross_chain');
    expect(result.steps[2].action).toBe('wait_for_completion');
    expect(result.steps[3].action).toBe('report_success');
  });

  it('should handle errors autonomously', async () => {
    const agent = env.getAgent('bob');

    // Simulate instruction that will fail
    const result = await agent.execute(
      'Transfer Right 0xnonexistent to Ethereum'
    );

    // Agent should:
    // 1. Attempt operation
    // 2. Detect error
    // 3. Read ErrorSuggestion
    // 4. Report issue with context
    expect(result.success).toBe(false);
    expect(result.error.suggested_fix).toBeDefined();
    expect(result.explanation).toContain('Right not found');
  });
});
```

### 6.1 CLI Enhancements

```bash
# Current: csv right create --chain bitcoin --value 100000
# Proposed: Much more developer-friendly

# Interactive mode (guides you through options)
csv right create --interactive

# Dry run (shows what would happen without executing)
csv transfer cross-chain --from bitcoin --to sui --right-id 0x... --dry-run

# Verbose mode (shows all RPC calls, proofs, signatures)
csv transfer cross-chain --from bitcoin --to sui --right-id 0x... -vvv

# Generate code from CLI action (learn by doing)
csv right create --chain bitcoin --value 100000 --generate-code rust
# Outputs: Ready-to-use Rust code snippet

# Local test environment
csv local start
# Spins up: Bitcoin signet regtest + Sui devnet + Ethereum anvil
# All chains running locally, no network needed
```

### 6.2 Code Generator

```bash
# Generate project from template
csv generate project --name my-app --template nft-marketplace

# Generate SDK code from existing actions
csv generate sdk --from-history last-10-transfers --language typescript

# Generate documentation
csv generate docs --output ./docs --format markdown

# Generate smart contracts
csv generate contract --chain ethereum --type nft-with-royalties
```

### 6.3 Local Chain Simulator

```bash
# Start local development environment
csv local start

# What it runs:
# - Bitcoin regtest (elements-project/elements)
# - Sui devnet (sui start)
# - Ethereum anvil (foundry)
# - Aptos testing-net
# - Pre-funded dev wallets
# - Mock RPC endpoints

# Status dashboard
csv local status

# Show chain health:
# Bitcoin regtest:  ✓ 104 blocks  0 pending txs
# Sui devnet:       ✓ epoch 1     0 pending txs
# Ethereum anvil:   ✓ block 1     0 pending txs

# Reset environment
csv local reset --clean
```

### 6.4 Testing Framework

```typescript
// @csv-adapter/testing
import { TestEnvironment, Chain } from '@csv-adapter/testing';

describe('Cross-Chain NFT Transfer', () => {
  let env: TestEnvironment;

  beforeEach(async () => {
    env = await TestEnvironment.create({
      chains: [Chain.Bitcoin, Chain.Ethereum],
      fundWallets: true,  // Auto-fund dev wallets
    });
  });

  it('should transfer NFT from Bitcoin to Ethereum', async () => {
    const csv = env.getCsvClient('alice');

    // Create NFT Right on Bitcoin
    const nft = await csv.rights.create({
      metadata: { name: 'Test NFT', image: 'ipfs://...' },
    });

    // Transfer to Ethereum
    const transfer = await csv.transfers.crossChain({
      rightId: nft.id,
      from: Chain.Bitcoin,
      to: Chain.Ethereum,
      toAddress: env.getAddress('bob', Chain.Ethereum),
    });

    await transfer.waitForCompletion();

    // Verify Bob owns it on Ethereum
    const bobCsv = env.getCsvClient('bob');
    const rights = await bobCsv.rights.list({ chain: Chain.Ethereum });
    expect(rights).toContainEqual(expect.objectContaining({ id: nft.id }));
  });
});
```

### 6.5 Debugging Tools

```bash
# Inspect a Right
csv inspect right 0xabc123...

# Output:
# Right ID:    0xabc123...
# Chain:       Bitcoin
# Seal:        UTXO 8f3a...:0 (spent)
# Commitment:  0xdef456...
# History:
#   [2026-04-10 14:32] Created on Bitcoin
#   [2026-04-10 15:01] Transferred to Ethereum (tx: 0x789...)
#   [2026-04-10 15:03] Verified on Ethereum

# Trace a transfer
csv trace transfer 0x789...

# Shows step-by-step:
# [1/5] Locking Right on Bitcoin...
#       Tx: 8f3a2b... (confirmed, 1 block deep)
# [2/5] Generating Merkle proof...
#       Proof: 234 bytes, 8 nodes
# [3/5] Submitting proof to Ethereum...
#       Tx: 0x789def... (pending)
# [4/5] Verifying proof on Ethereum...
#       Verification: ✓ valid (gas: 87,432)
# [5/5] Minting Right on Ethereum...
#       Tx: 0xabc123... (confirmed)
# ✅ Transfer complete!

# Validate proof offline
csv validate proof proof.json --chain ethereum --verbose
```

---

## 7. Local Development Environment

### 7.1 Docker Compose Setup

```yaml
# docker-compose.dev.yml
version: '3.8'
services:
  bitcoin-regtest:
    image: kylemanna/bitcoind:latest
    command: -regtest -rpcuser=dev -rpcpassword=dev -rpcallowip=0.0.0.0/0
    ports:
      - "18443:18443"

  ethereum-anvil:
    image: ghcr.io/foundry-rs/foundry:latest
    command: anvil --host 0.0.0.0
    ports:
      - "8545:8545"

  sui-devnet:
    image: mysten/sui-tools:mainnet
    command: sui start --with-faucet --with-indexer
    ports:
      - "9000:9000"
      - "9123:9123"  # faucet

  csv-local-rpc:
    build: .
    command: csv local rpc --chains bitcoin,ethereum,sui
    ports:
      - "3000:3000"
    depends_on:
      - bitcoin-regtest
      - ethereum-anvil
      - sui-devnet
```

### 7.2 VS Code Extension

```
csv-adapter-vscode/
├── package.json
├── syntaxes/
│   └── csv.tmLanguage.json      # CSV proof syntax highlighting
├── snippets/
│   └── csv.code-snippets        # Code snippets for Rust/TS
└── src/
    ├── commands/
    │   ├── createRight.ts       # "CSV: Create Right" command
    │   ├── transfer.ts          # "CSV: Transfer Cross-Chain"
    │   └── inspectProof.ts      # "CSV: Inspect Proof"
    ├── providers/
    │   └── walletExplorer.ts    # Wallet explorer in sidebar
    └── debug/
        └── csvDebugger.ts       # Step-through proof verification
```

### 7.3 IDE Integration Features

| Feature | Description |
|---------|-------------|
| **Code Snippets** | `csv-transfer`, `csv-right-create`, `csv-proof-verify` |
| **Inline Documentation** | Hover over `CsvClient` → shows full API docs |
| **Proof Visualizer** | Render Merkle trees as interactive diagrams |
| **Transaction Simulator** | Preview transfer before executing |
| **Error Lens** | Inline error suggestions with fix actions |
| **Wallet Explorer** | Browse Rights in VS Code sidebar |

---

## 8. Testing & Debugging Experience

### 8.1 Test Pyramid

```
                    ┌─────────────┐
                    │   E2E       │  ~10 tests (real testnets, slow)
                    │  Tests      │
                   ───────────────
                  │ Integration │   ~100 tests (local chains, medium)
                  │   Tests     │
                 ─────────────────
                │    Unit       │  ~1000 tests (pure functions, fast)
                │    Tests      │
               ───────────────────
```

### 8.2 Mock Chain Providers

```rust
use csv_adapter::testing::MockChain;

// Mock Bitcoin chain for fast testing
let mock_btc = MockChain::builder()
    .chain(Chain::Bitcoin)
    .with_utxos(vec![UTXO::new(100_000)])
    .with_block_height(100)
    .build()?;

let client = CsvClient::builder()
    .with_chain(mock_btc)
    .build()?;

// Test transfer without real RPC
let right = client.rights().create(commitment).await?;
assert_eq!(right.chain, Chain::Bitcoin);
```

### 8.3 Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn right_id_is_deterministic(seed: Vec<u8>) {
        let commitment = Commitment::from_hash(&seed);
        let right_id = commitment.right_id();

        // Same input always produces same right_id
        let commitment2 = Commitment::from_hash(&seed);
        let right_id2 = commitment2.right_id();

        prop_assert_eq!(right_id, right_id2);
    }

    #[test]
    fn proof_verifies_after_generation(right_data: String) {
        let commitment = Commitment::from_hash(right_data.as_bytes());
        let proof = generate_mock_proof(&commitment);

        // Every generated proof should verify
        prop_assert!(verify_proof(&proof).is_ok());
    }
}
```

### 8.4 Debugging Workflow

```rust
// Enable debug logging
env_logger::Builder::from_env(Env::default().default_filter_or("csv_adapter=debug"))
    .init();

// Structured logging with JSON output (for production)
use tracing_subscriber::fmt;

fmt()
    .json()
    .with_target(true)
    .with_thread_ids(true)
    .init();

// All CSV operations emit structured events:
// {"timestamp":"2026-04-11T10:00:00Z","level":"INFO","target":"csv_adapter::transfers","event":"transfer_initiated","transfer_id":"0x...","from":"bitcoin","to":"ethereum"}
```

---

## 9. Documentation Strategy

### 9.1 Documentation Layers

```
Level 1: Tutorials (Learning-oriented)
  - "Cross-Chain Transfers in 5 Minutes"
  - "Building Your First CSV dApp"
  - Step-by-step, copy-paste, guaranteed success

Level 2: How-To Guides (Goal-oriented)
  - "How to Add Cross-Chain NFT Transfers"
  - "How to Integrate CSV with MetaMask"
  - "How to Deploy CSV to Production"
  - Real-world scenarios, prerequisites noted

Level 3: Reference (Information-oriented)
  - Full API reference (auto-generated from code)
  - CLI command reference
  - Chain-specific implementation details
  - Configuration options

Level 4: Explanation (Understanding-oriented)
  - "Why Client-Side Validation?"
  - "Understanding the Universal Seal Primitive"
  - "Cross-Chain Proof Mechanics"
  - Deep dives, architecture decisions, trade-offs
```

### 9.2 Interactive Documentation Site

```
docs.csv.dev/

Features:
- Searchable API reference
- Runnable code examples (like Stripe docs)
- Chain selector (show examples for Bitcoin/Ethereum/etc)
- Language toggle (Rust/TypeScript)
- "Try It" buttons that run in browser WASM
- Community examples linked from relevant pages
```

### 9.3 Living Examples Repository

```
examples.csv.dev/

Curated examples:
├── basics/
│   ├── create-right/
│   ├── transfer-same-chain/
│   └── cross-chain-transfer/
├── applications/
│   ├── nft-marketplace/
│   ├── cross-chain-lending/
│   └── event-ticketing/
├── integrations/
│   ├── with-metamask/
│   ├── with-rainbowkit/
│   └── with-ledger/
└── advanced/
    ├── custom-proofs/
    ├── fraud-detection/
    └── batch-transfers/

Each example includes:
✓ Working code (Rust + TypeScript)
✓ Live demo (deployed on Vercel)
✓ Step-by-step guide
✓ Common pitfalls section
```

### 9.4 Auto-Generated API Docs

```bash
# Rust (rustdoc)
cargo doc --open --no-deps

# TypeScript (TypeDoc)
npx typedoc --out docs/ts src/

# Deployed to:
# - docs.csv.dev/api/rust
# - docs.csv.dev/api/typescript
```

---

## 10. Error Handling & Diagnostics

### 10.1 Typed Error Classes (TypeScript)

```typescript
// Every error is actionable
try {
  await transfer.waitForCompletion();
} catch (error) {
  if (error instanceof CsvError) {
    switch (error.code) {
      case ErrorCode.INSUFFICIENT_FUNDS:
        console.log(`Need ${error.details.required} sats, have ${error.details.available}`);
        console.log(`Fund wallet: ${error.details.faucetUrl}`);
        break;

      case ErrorCode.PROOF_VERIFICATION_FAILED:
        console.log('Proof verification failed. Details:');
        console.log(`  Expected seal: ${error.details.expectedSeal}`);
        console.log(`  Got seal: ${error.details.gotSeal}`);
        console.log(`  Run: csv validate proof ${error.details.proofFile}`);
        break;

      case ErrorCode.RPC_TIMEOUT:
        console.log(`RPC timeout for ${error.details.chain}`);
        console.log(`  Checked endpoint: ${error.details.rpcUrl}`);
        console.log(`  Try alternative: ${error.details.alternativeRpcUrl}`);
        break;
    }
  }
}
```

### 10.2 Error Codes Registry

| Code | Name | Description | Suggested Action |
|------|------|-------------|------------------|
| `CSV_001` | `INSUFFICIENT_FUNDS` | Not enough balance for operation | Fund wallet from faucet or exchange |
| `CSV_002` | `INVALID_RIGHT_ID` | Right ID format is invalid | Check Right ID is 32-byte hex |
| `CSV_003` | `CHAIN_NOT_SUPPORTED` | Chain not configured in client | Add chain to client builder |
| `CSV_004` | `PROOF_VERIFICATION_FAILED` | Proof doesn't match expected | Verify proof source chain data |
| `CSV_005` | `RPC_TIMEOUT` | Chain RPC didn't respond | Check network, try alternative RPC |
| `CSV_006` | `SEAL_ALREADY_CONSUMED` | Seal was already spent | Right may have been transferred, check history |
| `CSV_007` | `WALLET_NOT_CONNECTED` | Wallet not connected | Connect wallet or provide mnemonic |
| `CSV_008` | `TRANSFER_EXPIRED` | Transfer took too long | Retry with higher priority |

### 10.3 Diagnostic Commands

```bash
# Run full diagnostic
csv doctor

# Output:
# ✓ Rust version: 1.75.0
# ✓ CSV Adapter version: 0.3.0
# ✓ Configuration file: ~/.csv/config.toml
# ✗ Bitcoin RPC: Connection failed
#   → Checked: https://mempool.space/api
#   → Error: timeout after 30s
#   → Try: https://blockstream.info/api
# ✓ Ethereum RPC: Connected (chain: goerli, block: 10,234,567)
# ✓ Sui RPC: Connected (epoch: 1, checkpoint: 5,432)
# ✓ Wallet: Found (address: bc1q..., balance: 0.001 BTC)
# ✓ Store: SQLite database healthy (12 rights, 34 transfers)

# Fix common issues automatically
csv doctor --fix

# Check specific chain
csv doctor --chain bitcoin --verbose
```

---

## 11. Onboarding Journey

### 11.1 Developer Journey Map

```
Awareness → First Contact → First Success → First App → Production

Stage 1: Awareness (Day 0)
  - Sees CSV mentioned on Twitter/GitHub
  - Visits docs.csv.dev
  - Reads 30-second explainer
  - Watches 2-minute demo video

Stage 2: First Contact (Day 0-1)
  - Runs `npx create-csv-app` or `cargo install csv-cli`
  - Completes interactive tutorial
  - Makes first cross-chain transfer (simulated)

Stage 3: First Success (Day 1-3)
  - Makes first REAL cross-chain transfer on testnet
  - Earns "CSV Pioneer" badge/NFT
  - Shares on social media

Stage 4: First App (Week 1-2)
  - Builds simple dApp using CSV SDK
  - Deploys to Vercel/Netlify
  - Gets feedback from community

Stage 5: Production (Month 1-3)
  - Integrates CSV into production product
  - Contributes back examples/fixes
  - Becomes community advocate
```

### 11.2 Community Support

| Channel | Purpose | Response Time |
|---------|---------|---------------|
| **Discord** | General help, discussions | < 1 hour (community) |
| **GitHub Discussions** | Q&A, feature requests | < 1 day (maintainers) |
| **Stack Overflow** | Technical questions (tag: csv-adapter) | Community-driven |
| **Office Hours** | Weekly live coding + Q&A | Every Wednesday 2pm UTC |
| **1-on-1 Onboarding** | Book 30min with core team | For enterprise partners |

---

## 12. Implementation Roadmap

### Phase 1: Foundation (Weeks 1-4)

| Week | Deliverable | Owner | Priority |
|------|-------------|-------|----------|
| 1 | TypeScript SDK core structure | SDK Team | 🔴 Critical |
| 1 | Rust unified client builder | Core Team | 🔴 Critical |
| 1 | `create-csv-app` scaffolding | DX Team | 🔴 Critical |
| 1 | **MCP Server (v1)** | **Agent Team** | **🔴 Critical** |
| 1 | **OpenAPI specification** | **Agent Team** | **🟡 High** |
| 2 | Typed error classes (TS + Rust) | SDK Team | 🟡 High |
| 2 | **Self-describing errors with FixAction** | **Agent Team** | **🟡 High** |
| 2 | Interactive tutorial (CLI mode) | DX Team | 🟡 High |
| 3 | Local chain simulator (Docker) | Infra Team | 🟡 High |
| 3 | Testing framework mocks | SDK Team | 🟡 High |
| 4 | API documentation site (v1) | DX Team | 🟢 Medium |
| 4 | Code generator (CLI) | DX Team | 🟢 Medium |

**Success Criteria:** Developer can complete first cross-chain transfer in < 5 minutes

### Phase 2: Polish (Weeks 5-8)

| Week | Deliverable | Owner | Priority |
|------|-------------|-------|----------|
| 5 | TypeScript SDK chain providers | SDK Team | 🟡 High |
| 5 | WASM bindings for browser | Core Team | 🟡 High |
| 5 | **MCP Server (v2) with streaming** | **Agent Team** | **🟡 High** |
| 6 | Browser-based playground | DX Team | 🟢 Medium |
| 6 | Property-based test suite | QA Team | 🟢 Medium |
| 7 | Living examples repository | DX Team | 🟢 Medium |
| 8 | VS Code extension (v1) | DX Team | 🟢 Medium |

**Success Criteria:** 90% of tutorial participants succeed without external help

### Phase 3: Ecosystem (Weeks 9-12)

| Week | Deliverable | Owner | Priority |
|------|-------------|-------|----------|
| 9 | CSV Explorer — multi-chain indexer (Rust) | Core Team | 🔴 Critical |
| 9 | CSV Explorer — GraphQL + REST API | Core Team | 🔴 Critical |
| 10 | CSV Explorer — Dioxus UI (web + desktop) | UI Team | 🔴 Critical |
| 10 | CSV Explorer — Wallet integration | Core Team | 🟡 High |
| 11 | CSV Wallet browser extension | UI Team | 🟡 High |
| 12 | Python SDK | SDK Team | 🟢 Medium |

**Success Criteria:** Explorer live for all chains, Wallet extension released, 100+ developers

---

## 13. Success Metrics

### 13.1 Developer Experience KPIs

| Metric | Current | Target (Q1) | Target (Q2) | Measurement |
|--------|---------|-------------|-------------|-------------|
| **Time to first transfer** | N/A | < 5 min | < 3 min | Tutorial analytics |
| **Tutorial completion rate** | N/A | > 80% | > 90% | In-app tracking |
| **SDK adoption** | 0 | 100 devs | 500 devs | npm/crates.io downloads |
| **GitHub stars** | Existing | +200 | +1000 | GitHub API |
| **Community size** | Small | 500 | 2000 | Discord members |
| **Production apps** | 0 | 5 | 25 | Self-reported |
| **Documentation satisfaction** | N/A | 4.0/5 | 4.5/5 | Developer survey |
| **Error resolution time** | N/A | < 5 min | < 2 min | Support ticket analysis |

### 13.2 Developer Satisfaction Survey (Quarterly)

```
Rate your experience with CSV Adapter (1-5):

1. How easy was it to get started?
2. How clear is the documentation?
3. How helpful are the error messages?
4. How reliable is the SDK?
5. How likely are you to recommend CSV to a colleague?

Open feedback:
- What's the best thing about CSV?
- What's the most frustrating part?
- What would make your life easier?
```

### 13.3 Feedback Loop

```
Collect → Analyze → Act → Communicate

1. Collect:
   - In-app feedback widget
   - GitHub issue labels (dx-improvement)
   - Discord support channel mining
   - Quarterly developer survey

2. Analyze:
   - Categorize feedback (onboarding, API, docs, bugs)
   - Identify top 3 pain points
   - Track trends over time

3. Act:
   - Prioritize DX improvements in sprint planning
   - Assign owner to each pain point
   - Set deadline for resolution

4. Communicate:
   - Monthly DX update blog post
   - "You Asked, We Built" series
   - Changelog highlights DX improvements
   - Thank contributors publicly
```

---

## Appendix A: Comparison with Existing Solutions

| Feature | CSV (Target) | Bridges (Axelar, LayerZero) | Wrapped (tBTC, wBTC) |
|---------|--------------|------------------------------|----------------------|
| **Setup time** | 5 minutes | 30-60 minutes | 60+ minutes |
| **SDK quality** | TypeScript-first, typed | Varies, often JS-only | Limited SDK |
| **Local dev** | Full simulator, zero config | Requires testnet access | Requires testnet access |
| **Error messages** | Typed, actionable, with fix suggestions | Generic, often unclear | Minimal |
| **Documentation** | Interactive, runnable, multi-language | Static, text-only | Basic |
| **Community** | Active Discord, office hours, achievements | Discord, support tickets | Limited |
| **Testing** | Mock chains, property tests, 1000+ tests | Integration tests only | Basic unit tests |

---

## Appendix B: Competitive Analysis — Developer Experience

| Project | DX Strength | DX Weakness | CSV Opportunity |
|---------|-------------|-------------|-----------------|
| **ethers.js/viem** | Excellent TS types, great docs | Ethereum-only | Be cross-chain DX standard |
| **RainbowKit** | Beautiful onboarding, wallet connect | UI library only | Provide CSV-native wallet connector |
| **Foundry** | Fast tests, great CLI | Steep learning curve | Make CSV as easy as Foundry |
| **Stripe** | Best-in-class docs, interactive | Centralized API | Match Stripe's DX for decentralized |
| **Supabase** | Great onboarding, local dev | Web2 paradigm | Bring local-first to Web3 |

---

## Appendix C: Glossary

| Term | Definition |
|------|------------|
| **Right** | A transferrable claim anchored to a chain's single-use seal |
| **Seal** | On-chain mechanism enforcing single-use (UTXO, Object, Resource, Nullifier) |
| **Commitment** | Hash anchoring Right's data to a Seal |
| **Proof Bundle** | Combined inclusion + finality + seal consumption proof |
| **CSV** | Client-Side Validation — verification happens off-chain |
| **USP** | Universal Seal Primitive — cross-chain single-use abstraction |
| **DX** | Developer Experience |
| **TTF** | Time to First (successful transfer) |

---

## Appendix D: Quick Reference — What to Build First

```
Priority Order for Maximum DX Impact:

1. TypeScript SDK Core (Week 1-2)
   → Enables Maya to start building immediately

2. create-csv-app Scaffold (Week 1)
   → Removes friction from project setup

3. Interactive Tutorial (Week 2)
   → Guarantees first success experience

4. Local Chain Simulator (Week 3)
   → Removes network dependency for development

5. Error Handling & Diagnostics (Week 2-3)
   → Reduces support burden, increases self-service

6. Documentation Site (Week 4)
   → Centralizes knowledge, improves discoverability

7. Testing Framework (Week 3-4)
   → Gives developers confidence in their code

8. Code Generator (Week 4)
   → Accelerates learning through example generation

9. WASM Bindings (Week 5-6)
   → Enables browser-only use cases

10. VS Code Extension (Week 7-8)
    → Deepens integration into developer workflow
```

---

*This is a living document. Last updated: April 11, 2026.  
Contribute: <https://github.com/zorvan/csv-adapter/docs/BLUEPRINT_DX.md>  
Discuss: <https://discord.gg/csv-adapter>*
