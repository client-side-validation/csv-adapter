# CSV — Blueprint & Development Roadmap

> **Status:** Core architecture complete · 4 chains deployed · 9 cross-chain pairs verified
> **Maturity:** ~79% production readiness (audited, tested, documented)
> **DX Vision:** 5-minute time-to-first-transfer · Zero-config local dev · Self-documenting APIs

---

## 0. Developer Experience Commitment

**Philosophy:** *Every developer interaction with CSV should feel magical, not mechanical.*

### Primary Developer Personas

| Persona | Role | Goal | Success Metric |
|---------|------|------|----------------|
| **Maya** (TypeScript/Web) | Full-stack Web3 developer | Add cross-chain NFT transfers to marketplace | "I shipped cross-chain transfers in one sprint" |
| **Alex** (Rust Core) | Systems/infrastructure engineer | Build CSV-based backend service | "My service handles 1000 cross-chain verifications/sec" |
| **Ava** (AI Agent) | Autonomous agent (Cursor, Claude, Copilot, custom) | Execute cross-chain operations from natural language | "I completed the user's request without clarification" |

### DX KPIs

| Metric | Target Q1 | Target Q2 | Target Q4 |
|--------|-----------|-----------|-----------|
| **Time to first transfer** | < 5 min | < 3 min | < 2 min |
| **Tutorial completion rate** | > 80% | > 90% | > 95% |
| **SDK downloads** | 100 devs | 500 devs | 5000 devs |
| **Production apps** | 5 | 25 | 100+ |
| **Agent success rate** | N/A | > 85% | > 95% |
| **Agent clarification requests** | N/A | < 10% | < 2% |

**Full DX Strategy:** See [Developer Experience Blueprint](BLUEPRINT_DX.md)

---

## 0.1 AI Agent Developer Experience

**Vision:** *AI agents should be able to understand, implement, and operate CSV cross-chain systems autonomously from natural language instructions.*

### 0.1.1 Agent Personas

| Agent Type | Example | Use Case | Requirements |
|------------|---------|----------|--------------|
| **IDE Agent** | Cursor, GitHub Copilot | Generate CSV code from user prompt | Code context, type hints, examples |
| **CLI Agent** | Custom agents, n8n, Make | Execute cross-chain transfers autonomously | Tool definitions, status reporting |
| **Support Agent** | Discord bot, GitHub issue responder | Answer developer questions | Knowledge base, troubleshooting flows |
| **Audit Agent** | Security analysis, code review | Verify CSV implementations | Formal specs, invariant definitions |

### 0.1.2 Agent-Optimized Deliverables

#### A. Machine-Readable API Spec

```yaml
# csv-agent-spec.yaml - MCP-compatible API specification
name: csv-adapter
version: 0.3.0
description: Client-side validation system for cross-chain rights

tools:
  - name: csv_right_create
    description: Create a new Right anchored to a specific chain
    input:
      type: object
      properties:
        chain:
          type: string
          enum: [bitcoin, ethereum, sui, aptos]
          description: Which chain enforces the single-use seal
        commitment_data:
          type: object
          description: Data to commit (will be hashed)
        owner_address:
          type: string
          description: Address that owns the Right
      required: [chain, commitment_data]
    output:
      type: object
      properties:
        right_id:
          type: string
          description: Unique identifier for the Right (32-byte hex)
        transaction_hash:
          type: string
          description: On-chain transaction that created the Right
        status:
          type: string
          enum: [success, pending, failed]

  - name: csv_transfer_cross_chain
    description: Transfer a Right from one chain to another
    input:
      type: object
      properties:
        right_id:
          type: string
          description: The Right to transfer
        from_chain:
          type: string
          enum: [bitcoin, ethereum, sui, aptos]
        to_chain:
          type: string
          enum: [bitcoin, ethereum, sui, aptos]
        destination_owner:
          type: string
          description: New owner address on destination chain
      required: [right_id, from_chain, to_chain, destination_owner]
    output:
      type: object
      properties:
        transfer_id:
          type: string
        status:
          type: string
          enum: [initiated, locking, proof_generated, completed, failed]
        estimated_completion:
          type: string
          format: date-time
        progress_url:
          type: string
          description: URL to poll for transfer status

  - name: csv_proof_verify
    description: Verify a cross-chain proof locally
    input:
      type: object
      properties:
        proof_bundle:
          type: object
          description: The proof to verify
        expected_right_id:
          type: string
          description: Expected Right ID after verification
      required: [proof_bundle]
    output:
      type: object
      properties:
        valid:
          type: boolean
        error_message:
          type: string
          description: If invalid, explains why

  - name: csv_wallet_balance
    description: Check wallet balance across all chains
    input:
      type: object
      properties:
        chains:
          type: array
          items:
            type: string
            enum: [bitcoin, ethereum, sui, aptos]
          description: Chains to check (default: all)
    output:
      type: object
      properties:
        balances:
          type: object
          additionalProperties:
            type: object
            properties:
              amount:
                type: string
              currency:
                type: string
              usd_value:
                type: number
```

#### B. Agent Prompt Templates

```markdown
# CSV Agent System Prompts

## IDE Agent Context
You are a CSV Adapter expert. When generating code:
1. Always use `csv_adapter::prelude::*` for Rust
2. Always use `@csv-adapter/sdk` for TypeScript
3. Include error handling with typed errors
4. Add comments explaining cross-chain mechanics
5. Suggest testing strategies

## CLI Agent Tool Definition
When executing CSV operations:
1. Check wallet balance first
2. Validate right_id format before transfer
3. Report progress at each step (lock → proof → mint)
4. Handle retries automatically on transient failures
5. Return structured output (JSON) for downstream agents

## Support Agent Knowledge
Common issues and solutions:
- "Proof verification failed" → Check source chain confirmations
- "Insufficient funds" → Provide faucet URLs for testnets
- "RPC timeout" → Suggest alternative RPC endpoints
- "Right not found" → Explain Right exists in client state, query history
```

#### C. Structured Status Reporting

```rust
/// Agent-friendly status reporting
/// Every operation returns machine-readable progress
#[derive(Debug, Serialize)]
pub enum TransferStatus {
    Initiated {
        transfer_id: String,
        timestamp: DateTime<Utc>,
    },
    Locking {
        chain: Chain,
        estimated_blocks: u32,
        current_confirmations: u32,
        required_confirmations: u32,
    },
    GeneratingProof {
        proof_type: ProofType,
        estimated_size_bytes: usize,
        progress_percent: u8,
    },
    SubmittingProof {
        destination_chain: Chain,
        gas_estimate: U256,
        tx_hash: Option<String>,
    },
    Completed {
        right_id: String,
        destination_chain: Chain,
        transaction_hash: String,
        total_time_ms: u64,
    },
    Failed {
        error_code: String,
        error_message: String,
        retryable: bool,
        suggested_action: String,
    }
}
```

#### D. Self-Describing Errors

```rust
/// Every error includes machine-actionable metadata
#[derive(Debug, thiserror::Error)]
pub enum CsvError {
    #[error("Insufficient funds: have {available}, need {required}")]
    InsufficientFunds {
        available: u64,
        required: u64,
        chain: Chain,
        #[source]
        suggestion: ErrorSuggestion,
    },
}

#[derive(Debug, Serialize)]
pub struct ErrorSuggestion {
    /// Human-readable suggestion
    pub message: String,
    /// Machine-actionable fix (if available)
    pub fix: Option<FixAction>,
    /// Related documentation URL
    pub docs_url: String,
    /// Error code for agent lookup
    pub error_code: String,
}

#[derive(Debug, Serialize)]
pub enum FixAction {
    /// Fund wallet from faucet
    FundFromFaucet { url: String, amount: String },
    /// Retry with different parameters
    Retry { parameter_changes: HashMap<String, String> },
    /// Check external state
    CheckState { url: String, what: String },
}
```

### 0.1.3 Agent Integration Points

| Integration | Format | Example |
|-------------|--------|---------|
| **MCP Server** | Model Context Protocol | `csv-mcp-server` exposes CSV tools to Claude, Cursor |
| **OpenAPI Spec** | REST API definition | Auto-generate clients for any language |
| **JSON Schema** | Tool definitions | Agents understand input/output contracts |
| **TypeScript Types** | `.d.ts` files | Agents read type definitions for context |
| **Rust Doc Comments** | `///` annotations | Agents parse documentation during code generation |

### 0.1.4 Agent Workflow Examples

#### IDE Agent (Cursor/Copilot)

```
User: "Create a cross-chain NFT transfer from Bitcoin to Ethereum"

Agent workflow:
1. Read csv-agent-spec.yaml for available tools
2. Generate code using csv_adapter SDK:

```typescript
import { CSV, Chain } from '@csv-adapter/sdk';

async function transferNFT(rightId: string, toAddress: string) {
  const csv = await CSV.connectExtension();
  
  const transfer = await csv.transfers.crossChain({
    rightId,
    from: Chain.Bitcoin,
    to: Chain.Ethereum,
    toAddress,
  });

  return transfer.waitForCompletion({ timeout: '10m' });
}
```

1. Add error handling (reads ErrorSuggestion spec)
2. Suggest test using @csv-adapter/testing
3. Link to docs.csv.dev/transfers

```

#### CLI Agent (Autonomous Operation)

```

User: "Transfer Right 0xabc123 to my Ethereum wallet"

Agent workflow:

1. Call csv_wallet_balance to verify funds
2. Call csv_right_get(right_id: "0xabc123") to verify ownership
3. Call csv_transfer_cross_chain with parameters
4. Poll transfer status every 30s until completion
5. Report success with transaction hash

Status updates:
[1/4] Locking Right on Bitcoin... (2/6 confirmations)
[2/4] Generating proof... (45% complete)
[3/4] Submitting to Ethereum... (tx: 0x789def)
[4/4] Verifying proof... ✓
✅ Transfer complete! Right 0xabc123 now owned on Ethereum

```

#### Audit Agent (Security Verification)

```

Agent task: "Verify this CSV implementation cannot double-spend"

Agent workflow:

1. Read formal invariants from spec:
   - "A Right can only exist on one chain at a time"
   - "Seal consumption is atomic"
   - "Proof verification is deterministic"
2. Analyze code against invariants
3. Generate test cases that could violate invariants
4. Run property-based tests
5. Report: "✓ No double-spend vectors found in 10,000 random tests"

```

### 0.1.5 Implementation Plan

| Deliverable | Format | Effort | Agent Impact |
|-------------|--------|--------|--------------|
| **MCP Server** | `csv-mcp-server` package | 1 week | Claude/Cursor can operate CSV directly |
| **OpenAPI Spec** | `csv-api.yaml` | 2 days | Auto-generate clients, agent tools |
| **JSON Schema** | Tool definitions | 1 day | Universal agent compatibility |
| **Agent Examples** | Curated prompt+code pairs | 3 days | Better code generation |
| **Self-Describing Errors** | ErrorSuggestion type | 1 day | Agents can auto-fix issues |
| **Status Streaming** | SSE/WebSocket | 2 days | Real-time progress for agents |

```bash
# Install CSV MCP server for Claude/Cursor
npm install -g @csv-adapter/mcp-server

# Add to Claude Desktop config
{
  "mcpServers": {
    "csv": {
      "command": "csv-mcp-server",
      "args": ["--chains", "bitcoin,ethereum,sui,aptos"]
    }
  }
}

# Now Claude can:
# - Check your wallet balance
# - Create Rights
# - Transfer cross-chain
# - Verify proofs
# - Monitor transfers
# All autonomously with structured tool use
```

### 0.1.6 Agent Success Metrics

| Metric | Target Q1 | Target Q2 | Target Q4 |
|--------|-----------|-----------|-----------|
| **Agent task completion rate** | N/A | > 85% | > 95% |
| **Clarification requests** | N/A | < 10% | < 2% |
| **MCP server downloads** | N/A | 500 | 5000 |
| **Agent-generated PRs** | N/A | 50 | 500 |
| **Support agent accuracy** | N/A | > 80% | > 95% |
| **Audit agent coverage** | N/A | 50% of invariants | 100% of invariants |

---

## 1. Application Specifications

Detailed blueprints for the 10 CSV-native applications, ordered by implementation complexity and time-to-market.

### 1.1 Cross-Chain NFTs (Difficulty: ★★★★)

| Aspect | Detail |
|--------|--------|
| **Problem** | "Wrapped" NFTs are unbacked IOUs. A bridge can mint infinite copies. |
| **CSV Solution** | The original NFT Right is consumed (UTXO spent / object deleted) before the destination chain mints a copy. Cryptographic proof ensures exactly one copy exists at any time. |
| **Implementation** | |
| Source Chain | Create Right with `right_id = keccak256(token_id || metadata_hash)`,`commitment = hash(metadata_uri, royalties[], provenance[])` |
| Lock | Burn/lock the original → emits `CrossChainLock` event |
| Proof | Merkle proof of lock event + checkpoint/certification |
| Destination | Verify proof → mint new NFT with same `right_id`, copy commitment |
| Return Path | Burn destination NFT → prove burn → re-mint on source chain |
| **Key Invariant** | `∑ minted(right_id) - ∑ burned(right_id) = 1` at all times |
| **Phase** | Can be built now with existing core + adapters |

### 1.2 Confidential Asset Transfers (Difficulty: ★★★★★)

| Aspect | Detail |
|--------|--------|
| **Problem** | All token transfers are public. Amounts, senders, receivers fully visible. |
| **CSV Solution** | The commitment hash hides the transfer details. Only the seal consumption and proof verification are on-chain. Actual data stays between parties. |
| **Implementation** | |
| Commitment | `commitment = keccak256(sender_pubkey || receiver_pubkey || amount || salt)` |
| On-chain | Publish only `commitment` + seal consumption proof |
| Off-chain | Communicate `sender, receiver, amount, salt` via encrypted channel |
| Verification | Recipient recomputes `commitment` and matches on-chain value |
| **Phase** | Requires encrypted channel protocol (Signal/WireGuard integration) |

### 1.3 Cross-Chain Event Ticketing (Difficulty: ★★★)

| Aspect | Detail |
|--------|--------|
| **Problem** | Tickets sold across chains can be duplicated. No single-use guarantee across ecosystems. |
| **CSV Solution** | A ticket is a Right. Entry = seal consumption. Once consumed, no chain can accept it. |
| **Implementation** | |
| Mint | `right_id = keccak256(event_id || seat_number || date)`,`commitment = hash(ticket_type, price)` |
| Transfer | Cross-chain Right transfer to buyer's preferred chain |
| Redemption | At venue, scan QR → verify proof → consume seal → mark as used |
| Anti-fraud | Double-spend detection: if seal already consumed, reject |
| **Phase** | MVP-ready. Needs QR code standard + mobile verification app |

### 1.4 Supply Chain Provenance (Difficulty: ★★★★)

| Aspect | Detail |
|--------|--------|
| **Problem** | Product certificates on one chain can't prove uniqueness on another. Counterfeiting across chains is trivial. |
| **CSV Solution** | The product's Right moves through a commitment chain: manufacturer → distributor → retailer → consumer. Each hop is a verifiable, single-use transfer. |
| **Implementation** | |
| Origin | Manufacturer mints Right with `commitment = hash(serial_number, materials_cert, factory_id)` |
| Transfer | Each supply chain step = cross-chain Right transfer to the next party's chain |
| Audit | Anyone can verify the full commitment chain by tracing seal consumptions |
| Consumer | Final buyer receives the Right and can verify the entire provenance |
| **Phase** | Requires manufacturer onboarding + serial number standardization |

### 1.5 Cross-Chain Gaming Assets (Difficulty: ★★★)

| Aspect | Detail |
|--------|--------|
| **Problem** | Game items on one chain have no cryptographic guarantee of uniqueness when ported to another game/chain. |
| **CSV Solution** | The sword/NFT is a Right. When transferred to another game's chain, the original is cryptographically consumed. |
| **Implementation** | |
| Game A (Sui) | Right with `right_id = hash(game_a_id, item_id, stats)`, `commitment = hash(item_metadata)` |
| Transfer | Cross-chain to Game B (Ethereum) |
| Game B | Verifies proof → mints equivalent item with same `right_id` |
| Trade | Player can move the item back to Game A at any time |
| **Phase** | Can be built now. Needs game engine integration (Unity/Unreal SDK) |

### 1.6 Privacy-Preserving Credentials (Difficulty: ★★★★★)

| Aspect | Detail |
|--------|--------|
| **Problem** | Credentials on-chain reveal identity. ZK-based solutions are complex and expensive. |
| **CSV Solution** | Present the credential by consuming the seal. Proves you have it without revealing which credential or to whom. |
| **Implementation** | |
| Issuer | Mints Right with `commitment = hash(credential_data, holder_pubkey)`, reveals data to holder only |
| Holder | Presents proof of seal consumption to verifier without revealing commitment contents |
| Verifier | Accepts seal consumption proof + verifies issuer signature on commitment |
| **Phase** | Requires selective disclosure protocol (future) |

### 1.7 Cross-Chain Royalty Enforcement (Difficulty: ★★★)

| Aspect | Detail |
|--------|--------|
| **Problem** | Royalty standards (EIP-2981) only work within one chain. Cross-chain sales bypass royalties. |
| **CSV Solution** | The royalty Right is encoded in the commitment chain. Every cross-chain transfer must verify the previous state. |
| **Implementation** | |
| Mint | `commitment` includes `royalty_bps, royalty_recipient` |
| Transfer | Destination contract reads commitment → enforces royalty on sale |
| **Phase** | Near-term. Integrates with existing NFT marketplaces |

### 1.8 Decentralized Identity (Difficulty: ★★★★)

| Aspect | Detail |
|--------|--------|
| **Problem** | DIDs on one chain can be duplicated on another. Identity fragmentation. |
| **CSV Solution** | Identity is a single Right. Exists on exactly one chain. Moving requires seal consumption + proof. |
| **Implementation** | |
| `right_id = hash(DID_method, DID_identifier)` |
| `commitment = hash(public_keys[], services[], proofs[])` |
| **Phase** | Requires DID standardization + registry protocol |

### 1.9 Confidential Voting (Difficulty: ★★★★★)

| Aspect | Detail |
|--------|--------|
| **Problem** | On-chain voting reveals votes. Off-chain voting requires trust. |
| **CSV Solution** | Vote tokens are Rights. Cast vote = consume seal. Tally is verifiable (all seals consumed = all votes cast), individual votes private. |
| **Implementation** | |
| **Phase** | Requires homomorphic tallying integration |

### 1.10 Cross-Chain Subscriptions (Difficulty: ★★)

| Aspect | Detail |
|--------|--------|
| **Problem** | Subscription on chain A can't be used on chain B without centralized auth. |
| **CSV Solution** | Subscription Right moves to whichever chain the service is on. |
| **Phase** | **Easiest to implement.** Can be done this quarter. |

---

## 2. Wallet System

### Current State

- CLI generates/import wallets with seed-based key derivation
- Private keys stored unencrypted in `~/.csv/data/state.json`
- No HD wallet support beyond single-account BIP-86
- No multi-signature or threshold signing

### DX Goal: One-Command Wallet Setup

```bash
# Developer experience target
csv wallet init --devnet

# Output:
# ✓ Generated wallet with BIP-39 mnemonic (12 words)
# ✓ Funded with test tokens:
#   - Bitcoin Signet: 0.01 BTC
#   - Sui Devnet: 100 SUI
#   - Ethereum Goerli: 1 ETH
#   - Aptos Testnet: 10000 APT
# ✓ Configuration saved to ~/.csv/config.toml
# Ready to build! Run: csv tutorial cross-chain-basics
```

### Roadmap

#### Phase 1: Encrypted Key Storage

```
[ ] BIP-39 mnemonic generation with 12/24 word options
[ ] AES-256-GCM encryption with passphrase for keystore
[ ] Secure enclave / HSM support for cloud deployments
[ ] Key rotation protocol for compromised wallets
```

#### Phase 2: Multi-Chain HD Wallet

```
[ ] BIP-44 derivation: m/44'/{coin_type}'/{account}'/{change}/{index}
  - Bitcoin: coin_type = 0 (mainnet), 1 (testnet/signet)
  - Ethereum: coin_type = 60
  - Sui: coin_type = 784 (registered)
  - Aptos: coin_type = 637 (registered)
[ ] Single mnemonic controls all chains
[ ] Account management (multiple accounts per chain)
[ ] Address book with chain-agnostic contact resolution
```

#### Phase 3: Multi-Signature & Threshold

```
[ ] 2-of-3, 3-of-5 multi-sig for Bitcoin (native)
[ ] Multi-sig via smart contracts for Ethereum/Sui/Aptos
[ ] Threshold signatures (FROST/EdDSA) for cross-chain coordination
[ ] Governance wallets for DAO treasury operations
```

#### Phase 4: Hardware & Social Recovery

```
[ ] Ledger/Trezor integration via WebHID
[ ] Social recovery: designate guardians who can recover access
[ ] Dead man's switch: time-locked recovery if wallet goes dormant
```

---

## 3. Developer Tooling & Benchmarks

### 3.1 Getting Started Experience

#### The 5-Minute Rule

**Goal:** Any developer with Rust or Node.js installed can complete their first successful cross-chain transfer in under 5 minutes.

```bash
# TypeScript Developer Path
npx create-csv-app@latest my-crosschain-app
cd my-crosschain-app
npm run dev
# → Browser opens with working cross-chain transfer demo

# Rust Developer Path
cargo install csv-cli
csv init my-csv-service --template lending
cd my-csv-service
cargo test
# → All tests pass, ready to build
```

#### Interactive Tutorial System

```bash
csv tutorial cross-chain-basics

# Interactive terminal session:
# Step 1/7: Generate your wallet ✓
# Step 2/7: Claim test tokens on Signet ✓
# Step 3/7: Create your first Right ✓
# Step 4/7: Transfer it to Sui devnet ✓
# Step 5/7: Verify the proof locally ✓
# Step 6/7: Transfer back to Bitcoin ✓
# Step 7/7: Celebrate! 🎉 Tutorial complete
```

#### Local Development Environment

```bash
# Start full local dev environment
csv local start

# Spins up:
# ✓ Bitcoin regtest (pre-funded wallet)
# ✓ Sui devnet (pre-funded wallet)
# ✓ Ethereum anvil (pre-funded wallet)
# ✓ Aptos testing-net (pre-funded wallet)
# ✓ Local RPC proxy (unified access)

# Status dashboard
csv local status
# Bitcoin regtest:  ✓ 104 blocks  0 pending txs
# Sui devnet:       ✓ epoch 1     0 pending txs
# Ethereum anvil:   ✓ block 1     0 pending txs
```

### 3.2 Performance Benchmarks

All benchmarks should be run in a controlled environment:

- **Hardware**: 8-core CPU, 32GB RAM, 1Gbps network
- **Network**: Same geographic region for all chain nodes
- **Repetitions**: 100 runs per measurement, report median + p99

### 3.1 Latency

| Metric | CSV (current) | Bridge (Axelar) | Wrapped (tBTC) | Notes |
|--------|---------------|-----------------|----------------|-------|
| **Lock → Proof** | ~10s (Sui) | ~2-5min | N/A (always wrapped) | Time from lock to proof generation |
| **Proof → Mint** | ~5s (Sui), ~15s (ETH) | ~2-10min | N/A | Time from proof verification to mint |
| **End-to-end** | ~15-25s (Sui↔Sui) | ~5-15min | ~30min+ (Bitcoin confirmations) | Total cross-chain transfer time |
| **Bitcoin source** | ~60min (6 conf) | ~60min+ | Same | Bitcoin confirmation dominates |

### 3.2 Throughput

| Metric | CSV (current) | Bridge | Wrapped | Notes |
|--------|---------------|--------|---------|-------|
| **TPS per chain pair** | Limited by destination chain | ~10-100 TPS | Limited by mint contract | |
| **Parallel transfers** | Unlimited (independent Rights) | Queue-based | Queue-based | CSV scales linearly with chain capacity |
| **Batch processing** | Supported (batchMintRights) | Not supported | Not supported | |

### 3.3 Proof Size

| Chain Pair | Proof Size | Gas Cost (ETH equiv) | Notes |
|------------|------------|---------------------|-------|
| Bitcoin → Sui | ~200 bytes (Merkle branch) | ~50k gas (Sui compute) | Bitcoin Merkle proof |
| Bitcoin → Ethereum | ~200 bytes | ~100k gas | MPT verification pending |
| Sui → Ethereum | ~300 bytes (checkpoint) | ~120k gas | Checkpoint certification |
| Ethereum → Sui | ~500 bytes (MPT) | ~60k gas (Sui compute) | MPT proof nodes |
| Sui → Aptos | ~250 bytes | ~30k gas (Aptos) | Checkpoint → ledger |
| Aptos → Sui | ~400 bytes | ~40k gas (Sui) | Ledger info → checkpoint |

### 3.4 Cost Comparison

| Transfer Type | CSV Cost | Bridge Cost | Wrapped Cost |
|---------------|----------|-------------|--------------|
| **Gas (source)** | Standard tx | Standard tx + bridge fee | Standard tx + mint fee |
| **Gas (dest)** | Standard tx + proof verify | Standard tx | Standard tx |
| **Bridge fee** | $0 (no intermediary) | 0.05-0.3% of value | 0.1-0.5% of value |
| **Total (typical)** | $0.01-0.50 | $1.00-10.00+ | $2.00-15.00+ |

### 3.5 Security

| Attack Vector | CSV | Bridge | Wrapped |
|---------------|-----|--------|---------|
| **Double-spend** | ❌ Impossible (cryptographic) | ✅ Possible (bridge exploit) | ✅ Possible (mint exploit) |
| **Bridge hack** | ❌ No bridge | ✅ $2B+ lost in bridge hacks | ✅ Multiple exploits |
| **Validator collusion** | ❌ No validators | ✅ Requires 2/3+ validators | ✅ Requires mint signers |
| **Replay attack** | ❌ Seal registry prevents | ✅ Possible | ✅ Possible |
| **Censorship** | ✅ Permissionless | ❌ Bridge operators can censor | ❌ Mint operators can censor |

### 3.6 Developer Security Practices

| Practice | Implementation | DX Impact |
|----------|----------------|-----------|
| **Typed errors** | Every error includes actionable message + fix suggestion | 80% fewer support tickets |
| **Input validation** | All public APIs validate with clear error messages | Prevents common mistakes |
| **Dry-run mode** | `csv transfer --dry-run` shows what would happen | Zero-cost learning |
| **Proof simulation** | Test proofs without chain interaction | Fast iteration |
| **Audit trail** | All operations logged with structured JSON | Easy debugging |

---

## 4. Privacy & Advanced Cryptography

### 4.1 Current Privacy Level

| Component | Visibility |
|-----------|------------|
| Lock event | Public (tx hash, block height) |
| Commitment hash | Public (but data is hidden) |
| Proof | Public (but only verifies inclusion) |
| Off-chain data | Private (between parties) |

### 4.2 Zero-Knowledge Integration

#### ZK-SNARK Proof Verification

```
Goal: Verify cross-chain proofs without revealing the full proof on-chain

Current: Full Merkle proof published on-chain
Proposed: ZK proof that "I know a valid Merkle proof for this commitment"
Benefit: Proof size from ~500 bytes → ~200 bytes, full privacy

Implementation:
- Halo2 for Ethereum (native ZK-friendly)
- Groth16 for Sui/Aptos (via Move libraries)
- Off-chain prover, on-chain verifier
```

#### Selective Disclosure

```
Goal: Prove you own a credential without revealing which one

Current: Full commitment hash published
Proposed: ZK proof of membership in a set, revealing only necessary attributes
Benefit: Privacy for credentials, identity, voting

Implementation:
- Merkle tree of commitments
- ZK proof of leaf membership
- Reveal only required predicates (age > 18, country = X, etc.)
```

### 4.3 Post-Quantum Security

#### Current State

| Component | Algorithm | PQ Status |
|-----------|-----------|-----------|
| Bitcoin seals | secp256k1 (ECDSA/Schnorr) | ❌ Vulnerable |
| Ethereum | secp256k1 | ❌ Vulnerable |
| Sui | ed25519 | ❌ Vulnerable |
| Aptos | ed25519 | ❌ Vulnerable |
| Commitment hash | SHA-256 / Keccak256 | ✅ Grover-resistant (use 256-bit) |
| Merkle proofs | SHA-256 | ✅ Grover-resistant |

#### Migration Path

```
Phase 1 (Now):
- Add Dilithium/SPHINCS+ support as parallel signing scheme
- Dual-sign all seals (classical + PQ)
- Store both signatures in seal reference

Phase 2 (2026):
- Support hybrid keys (secp256k1 + Dilithium)
- Gradual migration: new Rights use PQ-only
- Legacy Rights can be "upgraded" via re-mint

Phase 3 (Post-Quantum):
- Deprecate classical signatures
- All seals use PQ signatures only
- Update all chain adapters
```

#### PQ Algorithms Under Consideration

| Algorithm | Type | Key Size | Signature Size | NIST Status |
|-----------|------|----------|----------------|-------------|
| **Dilithium2** | Lattice | 1.3KB | 2.4KB | Standardized (FIPS 204) |
| **Falcon-512** | Lattice | 0.9KB | 0.7KB | Standardized (FIPS 206) |
| **SPHINCS+-128f** | Hash-based | 32B | 16KB | Standardized (FIPS 205) |

**Recommendation**: Start with Dilithium2 (best size/speed tradeoff), add SPHINCS+ as fallback (no structured secrets).

---

## 5. Optimization & Macros

### 5.1 Performance Bottlenecks (Current)

| Bottleneck | Location | Impact | Fix Priority |
|------------|----------|--------|-------------|
| HTTP request per UTXO | `mempool_rpc.rs` | High latency | Medium (already has retry) |
| Merkle proof recomputation | `proofs.rs` | CPU waste | High (cache proofs) |
| Full block fetch for Merkle | `real_rpc.rs` | Bandwidth waste | Medium (partial fetch) |
| Seal registry linear scan | `seal_registry.rs` | O(n) lookups | High (use hash map) |

### 5.2 Proposed Macros

#### `#[csv_seal]` Macro

```rust
// Generate a complete Right lifecycle module from a schema
#[csv_seal(
    name = "NFTTransfer",
    fields = [
        token_id: U256,
        metadata_uri: String,
        royalty_bps: u16,
    ],
    chains = [Bitcoin, Ethereum, Sui, Aptos],
)]
pub struct NFTTransfer;

// Expands to:
// - Right definition with typed fields
// - Lock/unlock functions for each chain
// - Proof generation + verification
// - Cross-chain transfer trait implementation
// - CLI commands (generate, transfer, verify)
```

#### `#[cross_chain_proof]` Macro

```rust
// Generate chain-specific proof implementations
#[cross_chain_proof(source = Bitcoin, dest = Ethereum)]
pub fn bitcoin_to_eth_proof(lock_event: LockEvent) -> ProofBundle {
    // Expands to:
    // - Merkle proof extraction from Bitcoin block
    // - MPT proof construction for Ethereum
    // - Gas optimization for Ethereum verification
}
```

#### `#[seal_registry]` Macro

```rust
// Generate optimized seal registry with persistence
#[seal_registry(
    storage = "rocksdb",
    cache_size = 1_000_000,
    bloom_filter = true,
)]
pub struct GlobalSealRegistry;

// Expands to:
// - LRU cache with configurable size
// - Bloom filter for O(1) negative lookups
// - RocksDB persistence with WAL
// - Replication protocol for distributed registries
```

### 5.3 Compiler Optimizations

| Optimization | Impact | Effort |
|--------------|--------|--------|
| **Proof caching** | Skip recomputation for known blocks | 1 day |
| **Bloom filter for seal registry** | O(1) negative lookups | 2 days |
| **Parallel proof verification** | Multi-core utilization | 3 days |
| **Batch UTXO scanning** | Single HTTP request for multiple addresses | 2 days |
| **Merkle tree precomputation** | Cache tree levels for frequent blocks | 1 day |

---

## 6. Supporting Other Chains

### 6.1 Priority Queue

| Chain | Priority | Why | Effort |
|-------|----------|-----|--------|
| **Solana** | ★★★★★ | High throughput, growing DeFi, program model compatible | 2-3 weeks |
| **Cosmos** | ★★★★ | IBC ecosystem, Tendermint finality | 2-3 weeks |
| **Polkadot** | ★★★★ | Cross-chain native, parachain ecosystem | 3-4 weeks |
| **Ton** | ★★★ | Growing rapidly, unique cell model | 3-4 weeks |
| **Starknet** | ★★★ | ZK-native, Cairo VM interesting for proofs | 4-5 weeks |
| **Cardano** | ★★ | EUTXO model is natural fit, but slow adoption | 3-4 weeks |
| **Tron** | ★★ | High stablecoin volume, but centralized | 2 weeks |

### 6.2 Adapter Template

Each new chain requires:

```
csv-adapter-{chain}/
├── Cargo.toml
├── src/
│   ├── lib.rs           # Module exports
│   ├── adapter.rs       # AnchorLayer implementation
│   ├── config.rs        # Network + contract configuration
│   ├── error.rs         # Chain-specific errors
│   ├── proofs.rs        # Inclusion + finality proofs
│   ├── real_rpc.rs      # Real RPC client (feature-gated)
│   ├── seal.rs          # Seal registry
│   ├── signatures.rs    # Signing scheme
│   └── types.rs         # Chain-specific types
└── tests/
    └── testnet_e2e.rs   # Network-dependent integration tests
```

#### Required Implementations per Chain

| Trait/Component | Purpose | Complexity |
|-----------------|---------|------------|
| `AnchorLayer` | Core CSV trait: publish, verify, enforce | Medium |
| `SealRef` | Single-use seal reference type | Low |
| `AnchorRef` | Transaction/block reference | Low |
| `InclusionProof` | Merkle/state proof type | Medium-High |
| `FinalityProof` | Finality verification | Medium |
| `ProofBundle` | Combined inclusion + finality | Medium |

### 6.3 Chain-Specific Notes

#### Solana

- Seal = consumed SPL token account
- Proof = transaction receipt + slot verification
- Finality = confirmed slot (32 confirmations)
- Challenge: Account model vs UTXO model

#### Cosmos

- Seal = spent IBC packet
- Proof = IBC proof + Tendermint validator signatures
- Finality = Tendermint consensus (instant)
- Advantage: IBC already does cross-chain, CSV adds single-use guarantee

#### Polkadot

- Seal = consumed XCMP message
- Proof = SPREE proof + relay chain inclusion
- Finality = GRANDPA finality
- Advantage: Parachains designed for cross-chain

---

## 7. SDK Development — DX-First Design

### 7.1 Design Philosophy

| Principle | Description | Example |
|-----------|-------------|---------|
| **Progressive Disclosure** | Simple things are simple, complex things are possible | `csv.transfer()` for basics, `.with_custom_proof()` for advanced |
| **Sensible Defaults** | Works out of the box with zero config | Auto-detects network, generates wallet if missing |
| **Chain Agnostic** | Same API works across all chains | `from: 'bitcoin'` → `from: 'sui'` no code changes |
| **Explicit > Implicit** | Clear intent over magic | `await transfer.waitForCompletion()` not event listeners |
| **Typed Errors** | Every error is actionable | `CsvError::InsufficientFunds { available, required }` |
| **Async Native** | First-class async support | All I/O operations are async, no blocking |

### 7.2 Target Languages

| Language | Priority | Target Users | Effort | DX Goal |
|----------|----------|--------------|--------|---------|
| **Rust** | Core | Already exists as adapters | Ongoing | `cargo add csv-adapter` → production ready |
| **TypeScript** | ★★★★★ | Web3 dApps, frontends | 4-6 weeks | `npm install @csv-adapter/sdk` → 5-min first transfer |
| **Go** | ★★★★ | Backend services, infrastructure | 3-4 weeks | `go get csv.dev/sdk` → production services |
| **Python** | ★★★ | Data science, scripting | 2-3 weeks | `pip install csv-adapter` → scripting joy |
| **Swift** | ★★ | iOS apps | 3-4 weeks | Swift Package Manager integration |
| **Kotlin** | ★★ | Android apps | 3-4 weeks | Maven Central + Android Studio plugin |

### 7.3 SDK Architecture

```
csv-sdk/
├── rust/              # Existing adapters (already the core)
│   ├── csv-adapter/           # Unified meta-crate
│   │   ├── Cargo.toml         # Feature flags for each chain
│   │   └── src/
│   │       ├── lib.rs         # Re-exports + CsvClient builder
│   │       ├── prelude.rs     # `use csv_adapter::prelude::*`
│   │       └── builder.rs     # Fluent builder patterns
│   └── csv-adapter-testing/   # Testing framework
│       └── src/
│           ├── mock_chain.rs  # Mock chain providers
│           ├── test_env.rs    # Test environment builder
│           └── fixtures.rs    # Common test fixtures
│
├── typescript/
│   ├── package.json
│   ├── src/
│   │   ├── index.ts           # Main exports
│   │   ├── csv.ts             # CSV main class (progressive API)
│   │   ├── wallet.ts          # Wallet management
│   │   ├── rights.ts          # Right lifecycle
│   │   ├── transfers.ts       # Cross-chain transfers
│   │   ├── proofs.ts          # Proof generation/verification
│   │   ├── chains/            # Chain-specific implementations
│   │   │   ├── bitcoin.ts
│   │   │   ├── ethereum.ts
│   │   │   ├── sui.ts
│   │   │   └── aptos.ts
│   │   ├── errors.ts          # Typed error classes with suggestions
│   │   └── types.ts           # Full TypeScript definitions
│   ├── test/
│   │   ├── unit/              # Unit tests with mocks
│   │   ├── integration/       # Integration tests with local chains
│   │   └── e2e/               # E2E tests with testnets
│   └── dist/
│       ├── cjs/               # CommonJS build
│       ├── esm/               # ESM build
│       └── types/             # TypeScript declarations
│
├── go/
│   ├── csv/
│   │   ├── client.go          # Main client
│   │   ├── wallet.go
│   │   ├── right.go
│   │   ├── transfer.go
│   │   ├── proof.go
│   │   └── chains/
│   └── go.mod
│
└── python/
    ├── csv_adapter/
    │   ├── __init__.py
    │   ├── client.py
    │   ├── wallet.py
    │   ├── right.py
    │   ├── transfer.py
    │   └── proof.py
    └── setup.py
```

### 7.4 TypeScript SDK API (Maya's Flow)

```typescript
import { CSV, Chain } from '@csv-adapter/sdk';

// Initialize - works immediately
const csv = await CSV.createDevWallet();

// Create a Right - one liner
const right = await csv.rights.create({
  value: { amount: 1000, currency: 'sats' },
});

// Transfer cross-chain - simple
const transfer = await csv.transfers.crossChain({
  rightId: right.id,
  from: Chain.Bitcoin,
  to: Chain.Ethereum,
  toAddress: '0x742d35Cc6634C0532925a3b844Bc9e7595f2bD38',
});

// Wait for completion - explicit
const result = await transfer.waitForCompletion({ timeout: '5m' });
console.log(`✅ Right ${right.id} transferred to Ethereum`);
```

### 7.5 Rust SDK API (Alex's Flow)

```rust
use csv_adapter::prelude::*;

#[tokio::main]
async fn main() -> Result<(), CsvError> {
    // Build client - fluent builder pattern
    let client = CsvClient::builder()
        .with_chain(Bitcoin::mainnet()?)
        .with_chain(Ethereum::mainnet()?)
        .with_chain(Sui::mainnet()?)
        .with_wallet(Wallet::from_mnemonic(std::env::var("MNEMONIC")?)?)
        .with_store(SqliteStore::new("data/csv.db").await?)
        .build()?;

    // Create a Right - one liner
    let right = client.rights()
        .create(Commitment::from_hash(b"metadata".as_slice()))
        .on(Chain::Sui)
        .await?;

    // Cross-chain transfer - explicit
    let transfer = client.transfers()
        .cross_chain(&right.id, Chain::Ethereum)
        .to_address("0x742d35Cc6634C0532925a3b844Bc9e7595f2bD38".parse()?)
        .execute()
        .await?;

    // Monitor progress - stream API
    let mut watcher = transfer.watch();
    while let Some(event) = watcher.next().await {
        println!("Progress: {:?}", event);
    }

    Ok(())
}
```

### 7.6 WASM Bindings (Browser-Only)

```typescript
// @csv-adapter/wasm - all crypto in WASM
import init, { CsvWasm } from '@csv-adapter/wasm';

// Initialize WASM module
await init();

const csv = CsvWasm.new();

// All cryptography happens in WASM (faster, safer)
const wallet = csv.generate_wallet();
const proof = csv.generate_proof(right_id, chain);
const valid = csv.verify_proof(proof);
```

### 7.7 Testing Framework

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

### 7.8 Error Handling — Typed & Actionable

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

### 7.9 Diagnostic Tools

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
```

---

## 8. RGB Compatibility & AluVM

### 8.1 RGB Protocol Overview

RGB is a client-side validation protocol for Bitcoin that uses:

- **UTXOs as single-use seals** (same as CSV)
- **Taproot commitments** (same as CSV)
- **Client-side validation** (same philosophy as CSV)
- **AluVM** for state machine execution

### 8.2 Compatibility Analysis

| Feature | CSV | RGB | Compatible? |
|---------|-----|-----|-------------|
| Single-use seals | UTXO-based | UTXO-based | ✅ Yes, identical |
| Commitments | Tapret/OP_RETURN | Tapret/OP_RETURN | ✅ Yes, identical |
| State machines | Off-chain | AluVM | ⚠️ Bridge needed |
| Cross-chain | Native (core design) | Limited (Bitcoin-only) | ✅ CSV adds value |
| Schema system | Custom | RGB Schema 21 | ⚠️ Mapping needed |

### 8.3 Integration Plan

#### Phase 1: RGB Asset Import

```
Goal: Import RGB assets as CSV Rights

Process:
1. Read RGB contract state from local stash
2. Compute `right_id = keccak256(rgb_contract_id, state_hash)`
3. Create CSV Right with `commitment = hash(rgb_schema, rgb_state)`
4. Transfer to other chains via CSV protocol
```

#### Phase 2: AluVM Execution

```
Goal: Execute AluVM state machines within CSV Rights

Process:
1. Embed AluVM bytecode in Right's commitment
2. Execute state transitions off-chain
3. Commit new state via Tapret on Bitcoin
4. Cross-chain proof includes AluVM execution trace
```

#### Phase 3: Bidirectional Bridge

```
Goal: Rights can flow RGB ↔ CSV seamlessly

Process:
1. CSV → RGB: Right → RGB contract (schema 21 compatible)
2. RGB → CSV: RGB contract → Right (seal consumption proof)
3. Unified wallet shows both CSV Rights and RGB assets
```

### 8.4 AluVM Details

AluVM is a register-based virtual machine designed for:

- **Deterministic execution** (required for consensus)
- **Turing-complete** (unlike Bitcoin script)
- **Formal verification** (mathematical proofs of correctness)

#### Why AluVM Matters for CSV

| Use Case | Benefit |
|----------|---------|
| **Complex Rights** | Rights with executable logic (escrow, vesting, voting) |
| **Formal Verification** | Prove Rights cannot be double-spent mathematically |
| **Interoperability** | Shared execution model across RGB and CSV |

#### Implementation Effort: 6-8 weeks

```
Week 1-2: AluVM integration as optional dependency
Week 3-4: State machine execution within Right lifecycle
Week 5-6: Cross-chain proof includes AluVM execution trace
Week 7-8: Formal verification of core invariants
```

### 8.5 Other VM Considerations

| VM | Status | Integration Complexity | Value Add |
|----|--------|----------------------|-----------|
| **AluVM** | Primary target | Medium-High | Formal verification, RGB compat |
| **EVM** | Already supported | Low | Ethereum compatibility |
| **MoveVM** | Already supported | Low | Sui/Aptos compatibility |
| **WASM** | Future consideration | Medium | Language-agnostic smart contracts |
| **RISC-V** | Future consideration | High | CKB/ Nervos compatibility |

---

## 9. Browser Extension Wallet

### Vision

A MetaMask-style browser extension that manages CSV Rights across all chains from a single interface. Users can send, receive, and transfer Rights cross-chain without leaving their browser.

### Architecture

```
csv-extension/
├── manifest.json              # Manifest V3
├── background/
│   ├── service-worker.ts      # Key management, RPC routing
│   ├── vault.ts               # Encrypted keystore (AES-GCM + PBKDF2)
│   └── rpc-router.ts          # Route requests to correct chain RPC
├── content-script/
│   └── provider.ts            // Inject window.csv provider (EIP-1193 compatible)
├── popup/
│   ├── React UI              // Dashboard, send, receive, transfer
│   └── chain-selector.tsx    // Multi-chain network switcher
├── lib/
│   ├── csv-sdk.ts            // Core CSV SDK
│   └── chains/               // Chain-specific providers
└── test/
    └── e2e/                  // Playwright extension tests
```

### Key Features

| Feature | Detail |
|---------|--------|
| **Multi-chain accounts** | Single seed controls Bitcoin, Ethereum, Sui, Aptos, Solana, etc. |
| **EIP-1193 compatible** | `window.csv.request()` follows Ethereum provider pattern for dApp compatibility |
| **Right management** | View, send, and transfer Rights with visual previews |
| **Cross-chain transfer wizard** | Step-by-step UI: select Right → pick destination → confirm → monitor |
| **Hardware wallet support** | Ledger/Trezor via WebHID for high-value Rights |
| **Phishing protection** | Domain allowlisting + transaction simulation before signing |

### Integration with dApps

```typescript
// dApp detects CSV extension
if (window.csv) {
  const accounts = await window.csv.request({ method: 'csv_accounts' });
  
  // Transfer a Right cross-chain
  await window.csv.request({
    method: 'csv_transfer',
    params: {
      rightId: '0x...',
      from: 'sui',
      to: 'ethereum',
      destinationOwner: '0x...',
    },
  });
}
```

### Implementation Effort: 4-5 weeks

```
Week 1: Extension scaffold + vault encryption
Week 2: Service worker + RPC routing + chain providers
Week 3: Popup React UI (dashboard, send, receive)
Week 4: Cross-chain transfer wizard + EIP-1193 provider injection
Week 5: Hardware wallet + phishing protection + testing
```

---

## 10. DeFi Applications

### 10.1 Cross-Chain Lending

```
Problem: Collateral on Ethereum can't be used as collateral on Sui without wrapping.
CSV Solution: The collateral Right moves to the lending chain. If liquidated, the Right is consumed and the lender receives the collateral on the original chain.
```

| Component | Implementation |
|-----------|----------------|
| **Collateral lock** | Lender receives Right with `commitment = hash(collateral_details, liquidation_price)` |
| **Cross-chain proof** | Borrower proves collateral lock on the lending chain |
| **Liquidation** | If price drops below threshold, lender consumes seal → receives collateral on source chain |
| **Repayment** | Borrower repays → Right returns to borrower's chain |

### 10.2 Cross-Chain DEX

```
Problem: AMMs are chain-specific. Cross-chain swaps require trusted bridges.
CSV Solution: Atomic cross-chain swap using dual-seal consumption. Both parties lock Rights simultaneously. Neither can claim without the other.
```

| Component | Implementation |
|-----------|----------------|
| **Hash Time-Locked Swap (HTLS)** | Party A locks Right on chain X with hash H. Party B locks Right on chain Y with same H. |
| **Claim** | Party A reveals preimage on chain Y → claims B's Right. Party B sees preimage → claims A's Right on chain X. |
| **Refund** | If timeout expires, both Rights return to original owners. |
| **No trusted party** | Cryptographic guarantee — no bridge, no oracle, no multisig. |

### 10.3 Cross-Chain Yield Aggregation

```
Problem: Yield farming opportunities exist across chains. Moving assets requires bridging (trust) or manual transfers (slow).
CSV Solution: Yield positions are Rights. The aggregator moves them to whichever chain offers the best yield at any moment.
```

### 10.4 Cross-Chain Insurance

```
Problem: Insurance pools are chain-specific. Claims on one chain can't access liquidity on another.
CSV Solution: Policy is a Right. Claim = consume seal. Payout verified via cross-chain proof from the insured chain's oracle.
```

### Implementation Effort: 6-8 weeks per application

```
Cross-Chain Lending:    6 weeks (collateral management + liquidation engine)
Cross-Chain DEX:        8 weeks (HTLS protocol + order book + UI)
Yield Aggregator:       6 weeks (yield scanning + automated rebalancing)
Insurance Protocol:     7 weeks (policy creation + oracle integration + claims)
```

---

## 11. Fraud Proofs

### 11.1 The Problem

Current cross-chain bridges and light clients assume honest relayers. If a relayer submits an invalid proof, there's no on-chain mechanism to challenge it. CSV's current model verifies proofs at mint time — if verification fails, the mint reverts. But there's no way to *retroactively* prove that a previously-minted Right was based on a fraudulent lock event.

### 11.2 CSV Fraud Proof System

```
Goal: Allow anyone to challenge a minted Right by proving the source chain lock event never happened or was double-spent.

Mechanism:
1. Challenger submits fraud proof with evidence:
   - Source chain block header showing the seal was NOT spent
   - OR evidence the seal was spent in a different transaction
2. On-chain verification of fraud proof
3. If valid: burn the minted Right, slash the relayer (if bonded), refund the minter
4. If invalid: challenger loses bond
```

### 11.3 Fraud Proof Types

| Type | What It Proves | Verification Cost |
|------|----------------|-------------------|
| **Non-existence proof** | Seal was never consumed on source chain | Merkle proof of block state + UTXO set |
| **Double-spend proof** | Same seal consumed in two different transactions | Two conflicting transaction proofs |
| **Invalid lock proof** | Lock event data doesn't match the commitment | Recompute commitment hash from event data |
| **Reorg proof** | Source chain block containing lock was reorged out | Chain reorganization evidence + new tip header |

### 11.4 Challenge Period

```
Timeline:
T+0:   Right minted on destination chain
T+0 to T+24h: Challenge period open
T+24h: Challenge period closes, Right becomes final

During challenge period:
- Anyone can submit a fraud proof
- If fraud proof is valid → Right is burned, challenger rewarded
- If no valid fraud proof → Right is final and transferable

After challenge period:
- Right is considered final
- Only reorg-based fraud proofs accepted (longer window: 7 days)
```

### 11.5 Integration with Existing Architecture

```rust
// New trait for fraud proof submission
pub trait FraudProofSubmitter {
    type FraudProof;
    type Challenge;
    
    fn submit_fraud_proof(&self, proof: Self::FraudProof) -> Result<Self::Challenge, Error>;
    fn resolve_challenge(&self, challenge_id: ChallengeId) -> Result<Resolution, Error>;
    fn claim_reward(&self, challenge_id: ChallengeId) -> Result<(), Error>;
}

// Fraud proof structure
pub struct FraudProof {
    pub right_id: Hash,
    pub fraud_type: FraudType,
    pub evidence: Vec<u8>,
    pub source_chain_proof: Vec<u8>,
    pub challenger_address: Address,
}

pub enum FraudType {
    NonExistence,    // Seal was never consumed
    DoubleSpend,     // Same seal consumed twice
    InvalidLock,     // Lock event data doesn't match
    Reorg,           // Source block was reorged
}
```

### Implementation Effort: 4-5 weeks

```
Week 1: Fraud proof types + evidence format specification
Week 2: On-chain fraud proof verifier (Ethereum, Sui, Aptos)
Week 3: Challenge period state machine + reward distribution
Week 4: Integration with existing mint flow + testing
Week 5: CLI commands + documentation
```

---

## 12. MPC Wallet

### 12.1 Vision

A wallet where the private key is never reconstructed. Instead, it's split into shares distributed among the user's devices, a cloud backup, and optional guardians. Signing requires a threshold of shares (e.g., 2-of-3).

### 12.2 Architecture

```
                    ┌─────────────────┐
                    │   User Device 1 │ (Phone)
                    │   Share A       │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │   User Device 2 │ (Laptop)
                    │   Share B       │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │   Cloud Backup  │ (Encrypted Share C)
                    │   Share C       │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │   Guardians     │ (Optional, 2-of-3)
                    │   Shares D, E, F│
                    └─────────────────┘

Signing: Any 2 shares → distributed Schnorr/EdDSA → signature
Recovery: Any 3 shares → key reconstruction (emergency only)
```

### 12.3 Protocol Choice

| Protocol | Type | Threshold | Performance | CSV Fit |
|----------|------|-----------|-------------|---------|
| **FROST** | Schnorr (Bitcoin) | t-of-n | Fast (~50ms) | ✅ Best for Bitcoin seals |
| **GG20** | ECDSA (Ethereum) | t-of-n | Medium (~200ms) | ✅ Best for Ethereum |
| **EDMPC** | EdDSA (Sui/Aptos) | t-of-n | Fast (~40ms) | ✅ Best for Sui/Aptos |
| **Lindell 17** | ECDSA | 2-of-n | Medium | ⚠️ Good but slower |

**Recommendation**: FROST for Bitcoin + Sui/Aptos (EdDSA variant), GG20 for Ethereum.

### 12.4 Key Features

| Feature | Description |
|---------|-------------|
| **Threshold signing** | 2-of-3 by default, configurable to 3-of-5 for high-value wallets |
| **Device enrollment** | New device joins via existing device + cloud backup verification |
| **Share refresh** | Periodic share rotation without reconstructing the key |
| **Guardian recovery** | Designated contacts can help recover access if all devices are lost |
| **Spending limits** | Single-share signing allowed up to a daily limit; higher amounts require threshold |
| **Policy engine** | "Require 2-of-3 for transfers > 0.1 BTC, 1-of-3 for smaller amounts" |

### 12.5 CSV-Specific Benefits

| Use Case | MPC Benefit |
|----------|-------------|
| **Cross-chain transfers** | Signing happens on-device, no server ever sees the full key |
| **High-value Rights** | Institutional-grade security for valuable NFTs/credentials |
| **Inheritance** | Guardians can transfer Rights to heirs without key reconstruction |
| **Corporate wallets** | Board-approved cross-chain transfers with m-of-n governance |

### Implementation Effort: 8-10 weeks

```
Week 1-2: FROST implementation for Bitcoin (using frost-rust or similar)
Week 3-4: GG20 implementation for Ethereum
Week 5-6: EdDSA MPC for Sui/Aptos
Week 7: Share management + device enrollment protocol
Week 8: Policy engine + spending limits
Week 9: Integration with CSV wallet + cross-chain signing
Week 10: Testing + security audit
```

---

## 13. ZK-STARK Support

### 13.1 Why ZK-STARKs (Not Just SNARKs)

| Property | ZK-SNARK | ZK-STARK | CSV Relevance |
|----------|----------|----------|---------------|
| **Trusted setup** | Required ❌ | Not required ✅ | No ceremony needed |
| **Post-quantum** | Vulnerable ❌ | Resistant ✅ | Future-proof |
| **Proof size** | ~200 bytes ✅ | ~10-50KB ❌ | Trade-off for PQ security |
| **Verification cost** | Low ✅ | Medium ⚠️ | Acceptable for high-value transfers |
| **Prover speed** | Fast ✅ | Fast ✅ | No bottleneck |

### 13.2 ZK-STARK Integration Points

#### 1. Cross-Chain Proof Compression

```
Current: Full Merkle proof (~500 bytes) published on destination chain
With STARK: ZK proof that "I know a valid Merkle proof" (~15KB)
Trade-off: 30x larger proof, but:
  - Post-quantum secure
  - No trusted setup
  - Hides the exact Merkle path (privacy)
```

#### 2. Seal Consumption Privacy

```
Current: Seal consumption is public on the source chain
With STARK: ZK proof that "I consumed a valid seal" without revealing which one
Benefit: Unlinkable transfers — observer can't link source and destination transactions
```

#### 3. Batch Verification

```
Current: Each cross-chain transfer verified independently
With STARK: Aggregate 100 transfers into a single STARK proof
Benefit: Gas cost per transfer drops from ~100k to ~10k (on Ethereum)
```

### 13.3 STARK Library Choice

| Library | Language | Proof Size | Prover Time | Verification Gas (ETH) | PQ Secure |
|---------|----------|------------|-------------|------------------------|-----------|
| **Winterfell** | Rust | ~15KB | ~2s | ~500k | ✅ Yes |
| **RISC Zero** | Rust | ~20KB | ~5s | ~300k (with Bonsai) | ✅ Yes |
| **StarkWare (Cairo)** | Cairo | ~10KB | ~3s | ~200k (on Starknet) | ✅ Yes |

**Recommendation**: Winterfell for pure Rust integration, RISC Zero for general-purpose VM flexibility.

### 13.4 Implementation Plan

```
Phase 1 (Weeks 1-3): STARK prover integration
  - Add Winterfell as optional dependency
  - Implement Merkle proof STARK circuit
  - Generate proofs off-chain, verify on-chain

Phase 2 (Weeks 4-5): On-chain STARK verifier
  - Ethereum: Solidity verifier (generated from circuit)
  - Sui/Aptos: Move verifier module
  - Bitcoin: N/A (no smart contracts)

Phase 3 (Weeks 6-7): Privacy features
  - Seal consumption unlinkability
  - Commitment hiding
  - Selective disclosure

Phase 4 (Weeks 8-9): Batch verification
  - Aggregate 10-100 transfers into single proof
  - Reduce gas cost per transfer by 10x

Phase 5 (Week 10): Performance optimization
  - Parallel proof generation
  - GPU acceleration (optional)
  - Proof caching
```

---

## 14. React-Based UI for Applications

### 14.1 Design System

```
csv-ui/
├── src/
│   ├── components/
│   │   ├── Wallet/
│   │   │   ├── WalletConnect.tsx       // Multi-chain wallet connection
│   │   │   ├── BalanceCard.tsx         // Balance display with chain selector
│   │   │   ├── SendForm.tsx            // Send form with validation
│   │   │   └── ReceiveQR.tsx           // QR code for receiving
│   │   ├── Rights/
│   │   │   ├── RightCard.tsx           // Right preview with metadata
│   │   │   ├── RightList.tsx           // Grid/list of Rights
│   │   │   ├── RightDetail.tsx         // Full Right details + history
│   │   │   └── RightTransfer.tsx       // Cross-chain transfer wizard
│   │   ├── Chains/
│   │   │   ├── ChainSelector.tsx       // Multi-chain network switcher
│   │   │   ├── ChainStatus.tsx         // Chain health indicators
│   │   │   └── BridgeStatus.tsx        // Transfer progress across chains
│   │   └── Common/
│   │       ├── TransactionToast.tsx    // Transaction status toasts
│   │       ├── ErrorBoundary.tsx       // Error handling with recovery
│   │       └── LoadingSkeleton.tsx     // Skeleton loading states
│   ├── hooks/
│   │   ├── useCsvWallet.ts             // Wallet state management
│   │   ├── useRightTransfer.ts         // Cross-chain transfer hook
│   │   ├── useChainBalance.ts          // Balance polling hook
│   │   └── useTransactionWatcher.ts    // Transaction confirmation hook
│   ├── providers/
│   │   ├── CsvProvider.tsx             // CSV context provider
│   │   └── ChainProvider.tsx           // Chain-specific providers
│   └── utils/
│       ├── formatters.ts               // Address, amount formatting
│       ├── validators.ts               // Form validation
│       └── constants.ts                // Chain IDs, colors, etc.
├── stories/                            // Storybook documentation
└── test/                               // Component tests
```

### 14.2 Core Components API

```tsx
// Wallet Connection
<CsvProvider chains={[Chain.Bitcoin, Chain.Ethereum, Chain.Sui, Chain.Aptos]}>
  <WalletConnect
    onConnect={(wallet) => console.log('Connected:', wallet)}
    theme="dark"
  />
</CsvProvider>

// Right Display
<RightCard
  rightId={right.id}
  commitment={right.commitment}
  chain={right.chain}
  onClick={() => navigate(`/right/${right.id}`)}
/>

// Cross-Chain Transfer Wizard
<RightTransfer
  rightId={selectedRight.id}
  from={selectedRight.chain}
  availableDestinations={[Chain.Ethereum, Chain.Sui, Chain.Aptos]}
  onComplete={(result) => showSuccessToast(`Transferred to ${result.to}`)}
  onCancel={() => navigate('/')}
/>
```

### 14.3 State Management

```typescript
// Zustand store for wallet state
interface WalletState {
  accounts: Record<Chain, string>;
  balances: Record<Chain, bigint>;
  rights: Right[];
  transfers: Transfer[];
  
  connect: (mnemonic: string) => Promise<void>;
  disconnect: () => void;
  transfer: (params: TransferParams) => Promise<TransferResult>;
  refreshBalances: () => Promise<void>;
}

// React Query for data fetching
function useChainBalance(chain: Chain, address: string) {
  return useQuery({
    queryKey: ['balance', chain, address],
    queryFn: () => fetchBalance(chain, address),
    refetchInterval: 15_000, // Poll every 15s
  });
}

// WebSocket for real-time transfer updates
function useTransferWatcher(transferId: string) {
  const [status, setStatus] = useState<TransferStatus>('initiated');
  
  useEffect(() => {
    const ws = new WebSocket(`${WS_URL}/transfers/${transferId}`);
    ws.onmessage = (event) => setStatus(JSON.parse(event.data).status);
    return () => ws.close();
  }, [transferId]);
  
  return status;
}
```

### 14.4 Theming & Customization

```typescript
// Theme provider with chain-specific colors
<CsvTheme theme={{
  colors: {
    bitcoin: '#F7931A',
    ethereum: '#627EEA',
    sui: '#4DA2FF',
    aptos: '#2DD8A3',
    background: '#0D1117',
    surface: '#161B22',
  },
  borderRadius: '8px',
  fontFamily: 'Inter, sans-serif',
}}>
  <App />
</CsvTheme>
```

### 14.5 Example Application: Cross-Chain NFT Marketplace

```tsx
// Full application scaffold
function NFTMarketplace() {
  const { accounts, rights } = useCsvWallet();
  
  return (
    <div className="min-h-screen bg-background text-white">
      <Navbar>
        <WalletConnect />
        <ChainSelector />
      </Navbar>
      
      <main className="container mx-auto px-4 py-8">
        <Tabs defaultValue="my-rights">
          <TabsList>
            <TabsTrigger value="my-rights">My Rights</TabsTrigger>
            <TabsTrigger value="marketplace">Marketplace</TabsTrigger>
            <TabsTrigger value="transfers">Transfers</TabsTrigger>
          </TabsList>
          
          <TabsContent value="my-rights">
            <RightList
              rights={rights}
              onTransfer={(right) => openTransferModal(right)}
              onDetails={(right) => navigate(`/right/${right.id}`)}
            />
          </TabsContent>
          
          <TabsContent value="marketplace">
            <MarketplaceGrid
              listings={marketListings}
              onBuy={(listing) => executePurchase(listing)}
            />
          </TabsContent>
          
          <TabsContent value="transfers">
            <TransferHistory transfers={recentTransfers} />
          </TabsContent>
        </Tabs>
      </main>
      
      <TransferModal
        isOpen={isTransferOpen}
        right={selectedRight}
        onConfirm={handleTransfer}
        onClose={() => setIsTransferOpen(false)}
      />
    </div>
  );
}
```

### Implementation Effort: 6-8 weeks

```
Week 1-2: Component library + design system + Storybook
Week 3-4: Core hooks (wallet, balances, transfers)
Week 5: Right management UI (list, detail, transfer wizard)
Week 6: Cross-chain transfer wizard with progress tracking
Week 7: Theme system + responsive design + accessibility
Week 8: Documentation + example apps (marketplace, lending)
```

---

---

## Implementation Priority Matrix — DX-First

### Q1 2026 (Immediate) — DX Foundation

**Developer Experience (Priority 1)**

- [x] Developer Experience Blueprint (BLUEPRINT_DX.md)
- [ ] TypeScript SDK (core APIs) — `npm install @csv-adapter/sdk`
- [ ] `create-csv-app` scaffolding — `npx create-csv-app@latest`
- [ ] Interactive tutorial system — `csv tutorial cross-chain-basics`
- [ ] Local chain simulator — `csv local start`
- [ ] Typed error classes with suggestions
- [ ] `csv doctor` diagnostic tool

**Core Features (Priority 2)**

- [ ] Cross-chain subscription app (easiest app)
- [ ] Wallet encryption + BIP-39 support
- [ ] Seal registry performance optimization
- [ ] Solana adapter

**Success Criteria:** Developer can complete first cross-chain transfer in < 5 minutes

### Q2 2026 — DX Polish

**Developer Experience (Priority 1)**

- [ ] TypeScript SDK chain providers (Bitcoin, Ethereum, Sui, Aptos)
- [ ] WASM bindings for browser — `@csv-adapter/wasm`
- [ ] Testing framework with mock chains — `@csv-adapter/testing`
- [ ] Browser-based playground — `play.csv.dev`
- [ ] Living examples repository — `examples.csv.dev`
- [ ] Code generator — `csv generate project --template nft-marketplace`
- [ ] Property-based test suite

**Core Features (Priority 2)**

- [ ] Cross-chain NFT app
- [ ] Multi-sig wallet support
- [ ] Cosmos adapter
- [ ] ZK proof verification (Phase 1)
- [ ] RGB asset import

**Success Criteria:** 90% of tutorial participants succeed without external help

### Q3 2026 — Ecosystem

**Developer Experience (Priority 1)**
- [x] VS Code extension (v1)
- [x] React component library
- [x] Production deployment guide
- [x] CSV Explorer — multi-chain indexer + Dioxus UI
- [ ] CSV Wallet browser extension

**Core Features (Priority 2)**
- [ ] Python SDK
- [ ] Post-quantum signatures (Dilithium2)
- [ ] AluVM integration

**Success Criteria:** Explorer live for all chains, Wallet extension released, 100+ developers

### Q4 2026 — Maturity

**Developer Experience (Priority 1)**

- [ ] Developer satisfaction survey (quarterly)
- [ ] DX metrics dashboard
- [ ] "You Asked, We Built" series
- [ ] Developer advocacy program

**Core Features (Priority 2)**

- [ ] Privacy-preserving credentials
- [ ] ZK selective disclosure
- [ ] Polkadot adapter
- [ ] WASM browser bindings
- [ ] RGB bidirectional bridge
- [ ] Formal verification of core invariants

**Success Criteria:** 500+ developers, 25+ production apps, 4.5/5 DX satisfaction

---

## Success Metrics — DX-Focused

### Technical Metrics

| Metric | Current | Target Q1 | Target Q2 | Target Q4 |
|--------|---------|-----------|-----------|-----------|
| **Supported chains** | 4 | 5 | 7 | 10 |
| **Cross-chain pairs** | 9 | 20 | 42 | 90 |
| **E2E test coverage** | 9/9 | 20/20 | 42/42 | 90/90 |
| **SDK languages** | 1 (Rust) | 2 (Rust, TS) | 3 (Rust, TS, Go) | 5 (+ Python, Swift) |
| **Production readiness** | 79% | 85% | 92% | 98% |
| **Real transactions** | 1 (BTC→Signet) | 5 | 20 | 100+ |
| **Apps built on CSV** | 0 | 1 | 3 | 10 |

### Developer Experience Metrics

| Metric | Current | Target Q1 | Target Q2 | Target Q4 |
|--------|---------|-----------|-----------|-----------|
| **Time to first transfer** | N/A | < 5 min | < 3 min | < 2 min |
| **Tutorial completion rate** | N/A | > 80% | > 90% | > 95% |
| **SDK downloads** | 0 | 100 devs | 500 devs | 5000 devs |
| **GitHub stars** | Existing | +200 | +1000 | +5000 |
| **Community size** | Small | 500 | 2000 | 10000 |
| **Production apps** | 0 | 5 | 25 | 100+ |
| **Documentation satisfaction** | N/A | 4.0/5 | 4.5/5 | 4.8/5 |
| **Error resolution time** | N/A | < 5 min | < 2 min | < 1 min |
| **Support tickets per dev** | N/A | < 0.5 | < 0.2 | < 0.1 |

### Community Health Metrics

| Metric | Target Q2 | Target Q4 |
|--------|-----------|-----------|
| **Active contributors** | 20 | 100 |
| **Community examples** | 10 | 50 |
| **Discord activity** | 500 msgs/month | 2000 msgs/month |
| **Office hours attendance** | 20/session | 100/session |

---

## Documentation Strategy

### Documentation Layers (Diátaxis Framework)

| Layer | Purpose | Example | Audience |
|-------|---------|---------|----------|
| **Tutorials** | Learning-oriented, step-by-step | "Cross-Chain Transfers in 5 Minutes" | Beginners |
| **How-To Guides** | Goal-oriented, real-world scenarios | "How to Add Cross-Chain NFT Transfers" | Intermediate |
| **Reference** | Information-oriented, complete API docs | TypeScript SDK API Reference | All levels |
| **Explanation** | Understanding-oriented, deep dives | "Why Client-Side Validation?" | Advanced |

### Documentation Sites

| Site | Purpose | Features |
|------|---------|----------|
| **docs.csv.dev** | Main documentation hub | Search, chain selector, language toggle |
| **play.csv.dev** | Browser-based playground | In-browser Rust/TS, simulated chains |
| **examples.csv.dev** | Living examples repository | Working code, live demos, guides |
| **api.csv.dev** | Auto-generated API docs | Rust rustdoc, TypeScript TypeDoc |

### Community-Driven Content

| Content Type | Source | Review Process |
|--------------|--------|----------------|
| **Core docs** | Maintainers | PR review by DX team |
| **Examples** | Community | Curated by community team |
| **Tutorials** | Community + Maintainers | Peer-reviewed before publishing |
| **Troubleshooting** | Community | Wiki-style, moderated |
| **Blog posts** | Maintainers | Monthly DX updates |

### Feedback Loop

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

*This is a living document. Last updated: April 11, 2026.  
Related: [Developer Experience Blueprint](BLUEPRINT_DX.md)  
Contribute: <https://github.com/zorvan/csv-adapter>  
Discuss: <https://discord.gg/csv-adapter>*
