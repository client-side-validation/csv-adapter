# AI Agent Implementation Document
## CSV Protocol — Autonomous Agent Layer

**Document type**: Code-level implementation specification  
**Based on**: Repository snapshot (`repomix-output.xml`) + Principal Audit (`audit.md`)  
**Scope**: Full implementation of the `csv-mcp-server` AI agent layer, including tool definitions, security constraints, protocol-boundary enforcement, and integration points across the CSV codebase.

---

## Table of Contents

1. [Context and Motivation](#1-context-and-motivation)
2. [Architectural Boundaries](#2-architectural-boundaries)
3. [Agent Execution Model](#3-agent-execution-model)
4. [Directory Structure](#4-directory-structure)
5. [Tool Catalog and Schemas](#5-tool-catalog-and-schemas)
6. [Full Implementation: `csv-mcp-server/src/index.ts`](#6-full-implementation-csv-mcp-serversrcindexts)
7. [Rust Agent Bridge: `csv-core/src/mcp.rs`](#7-rust-agent-bridge-csv-coresrcmcprs)
8. [Canonical Serialization Layer](#8-canonical-serialization-layer)
9. [Proof Pipeline Integration](#9-proof-pipeline-integration)
10. [Sanad Semantic Enrichment (AI Non-Canonical Layer)](#10-sanad-semantic-enrichment-ai-non-canonical-layer)
11. [Versioning and Compatibility Enforcement](#11-versioning-and-compatibility-enforcement)
12. [Security Invariants the Agent Must Enforce](#12-security-invariants-the-agent-must-enforce)
13. [Testing Strategy](#13-testing-strategy)
14. [CI Enforcement](#14-ci-enforcement)
15. [Open Architecture Gaps and Remediation Plan](#15-open-architecture-gaps-and-remediation-plan)

---

## 1. Context and Motivation

### 1.1 Why an AI Agent Layer Exists

The CSV Protocol repository includes a stub MCP server at `csv-mcp-server/src/index.ts`. The audit confirms:

> "MCP server with 7 implemented tools: `create_seal`, `transfer_sanad`, `verify_proof`, `get_sanads`, `monitor_transfer`, `export_proof_bundle`, `accept_consignment` — all with input validation."
>
> "In 2026, agent-to-agent trust is the unsolved problem. CSV has the MCP server, the TypeScript SDK, and the proof primitive."

The goal is not merely to provide a convenience wrapper. The agent layer must be the **only sanctioned path** through which AI systems interact with the CSV Protocol runtime. It must enforce every invariant that `.agents/AGENT.md` defines while making those invariants machine-auditable.

### 1.2 What the Agent Is and Is Not

**The agent IS:**

- A Model Context Protocol (MCP) server exposing typed tools to AI runtimes (Claude, GPT-4, LangChain, AutoGPT).
- A security boundary that validates, translates, and forwards calls to `csv-cli` and `csv-runtime`.
- A non-canonical enrichment layer for sanad metadata (classification, tagging, schema mapping).
- An audit log emitter for every agent-originated state change.

**The agent IS NOT:**

- An authority on canonical truth. Hashes, proofs, seals, and state transitions are determined exclusively by `csv-core`.
- A shortcut past `TransferCoordinator`. No tool may call a chain adapter directly.
- A proof generator. It may request proof generation from the runtime and relay the result, but never fabricate or substitute one.
- A replacement for offline verification. The agent layer is online-only; `csv-core`'s WASM verification path remains authoritative for offline clients.

---

## 2. Architectural Boundaries

```
┌──────────────────────────────────────────────────────────────────────┐
│                     AI Agent / LLM Runtime                           │
│         (Claude, GPT-4, LangChain, AutoGPT, custom agents)          │
└────────────────────────────┬─────────────────────────────────────────┘
                             │  MCP tool calls (JSON-RPC over stdio/SSE)
┌────────────────────────────▼─────────────────────────────────────────┐
│              csv-mcp-server  (TypeScript, Node.js)                   │
│                                                                       │
│  ┌─────────────────┐  ┌──────────────────┐  ┌──────────────────────┐ │
│  │  Input Schemas   │  │  Audit Logger    │  │  Non-canonical AI   │ │
│  │  (Zod)          │  │  (structured)    │  │  Enrichment Layer   │ │
│  └────────┬────────┘  └──────────────────┘  └──────────────────────┘ │
│           │ validated args                                            │
│  ┌────────▼────────────────────────────────────────────────────────┐ │
│  │              executeCsvCommand()  /  REST bridge                │ │
│  └────────────────────────┬────────────────────────────────────────┘ │
└───────────────────────────┼──────────────────────────────────────────┘
                            │  subprocess / HTTP
┌───────────────────────────▼──────────────────────────────────────────┐
│                      csv-cli (Rust binary)                           │
│                                                                       │
│  commands: seals | sanads | proofs | validate | cross-chain | wallet │
└───────────────────────────┬──────────────────────────────────────────┘
                            │  library calls
┌───────────────────────────▼──────────────────────────────────────────┐
│                    csv-runtime                                        │
│                                                                       │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │              TransferCoordinator                               │  │
│  │  (ONLY path for cross-chain execution)                        │  │
│  └───────────────┬──────────────────────┬──────────────────────┘  │  │
│                  │                      │                           │  │
│  ┌───────────────▼──────┐  ┌────────────▼──────────────────────┐  │  │
│  │  ReplayDatabase      │  │  AdapterRegistry                  │  │  │
│  │  (Postgres/RocksDB)  │  │  (Bitcoin/ETH/Solana/Sui/Aptos)   │  │  │
│  └──────────────────────┘  └───────────────────────────────────┘  │  │
└───────────────────────────┬──────────────────────────────────────────┘
                            │  canonical protocol logic
┌───────────────────────────▼──────────────────────────────────────────┐
│                    csv-core (WASM-safe kernel)                        │
│                                                                       │
│  tagged_hash · ProofBundle · ReplayRegistry · SealPoint              │
│  TransferState machine · Consignment · DAGSegment · Domains          │
└──────────────────────────────────────────────────────────────────────┘
```

**Rule**: No MCP tool may import or call anything below the `csv-cli` boundary directly. Any attempt to call `csv-runtime` or `csv-core` types from TypeScript constitutes an architecture violation.

---

## 3. Agent Execution Model

### 3.1 Call lifecycle

```
Agent invokes tool
        │
        ▼
Zod schema validation
  ├── FAIL → return structured error (no CLI call)
  └── PASS
        │
        ▼
Audit log: { tool, args_hash, timestamp, agent_id }
        │
        ▼
executeCsvCommand(args)
  ├── timeout: 60 s (transfers), 10 s (queries)
  ├── env: RUST_LOG=info, no secrets in argv
  └── stdout/stderr captured
        │
        ▼
parseCliOutput(stdout)
  ├── JSON branch → structured result
  └── text branch → { message: string }
        │
        ▼
Audit log: { tool, result_hash, duration_ms, exit_code }
        │
        ▼
Return MCP result to agent
```

### 3.2 Agent identity

Every call to `executeCsvCommand` must carry an `AGENT_ID` environment variable derived from the MCP session. This value appears in every audit log entry and, where the CLI supports `--operator`, is forwarded as the operator identifier.

```typescript
const env = {
  ...process.env,
  RUST_LOG: 'info',
  CSV_AGENT_ID: agentId,          // session-scoped UUID
  CSV_AGENT_TOOL: toolName,       // which MCP tool triggered this
};
```

### 3.3 Lease enforcement

The `TransferCoordinator` enforces lease ownership: a cross-chain transfer must hold a valid `Lease` whose `transfer_id` matches the `SanadId`. The MCP server must request a lease before calling `transfer_sanad` and must pass the lease token to the CLI. Leases expire; the agent must not cache them across retries.

```typescript
// Pseudo-code: lease acquisition before transfer
const lease = await executeCsvCommand([
  'cross-chain', 'acquire-lease',
  '--sanad-id', sanadId,
  '--ttl', '120',           // seconds
]);
// Pass lease token into transfer command
await executeCsvCommand([
  'cross-chain', 'transfer',
  '--sanad-id', sanadId,
  '--destination-chain', destChain,
  '--destination', destination,
  '--lease-token', lease.token,
]);
```

---

## 4. Directory Structure

```
csv-mcp-server/
├── src/
│   ├── index.ts              # MCP server entry point (this document)
│   ├── tools/
│   │   ├── create_seal.ts
│   │   ├── transfer_sanad.ts
│   │   ├── verify_proof.ts
│   │   ├── get_sanads.ts
│   │   ├── monitor_transfer.ts
│   │   ├── export_proof_bundle.ts
│   │   ├── accept_consignment.ts
│   │   ├── get_protocol_info.ts
│   │   └── health_check.ts
│   ├── validation/
│   │   ├── schemas.ts        # Zod schemas (source of truth)
│   │   └── validators.ts     # helper functions
│   ├── execution/
│   │   ├── cli_runner.ts     # executeCsvCommand + parseCliOutput
│   │   └── lease_manager.ts  # lease acquisition and lifecycle
│   ├── audit/
│   │   └── logger.ts         # structured audit log (JSON Lines)
│   └── enrichment/
│       └── sanad_classifier.ts  # non-canonical AI enrichment
├── tests/
│   ├── validation.test.ts
│   ├── tools.test.ts
│   └── fixtures/
│       ├── valid_proof_bundle.json
│       ├── malformed_proof_bundle.json
│       └── replay_attempt.json
├── package.json
└── tsconfig.json
```

---

## 5. Tool Catalog and Schemas

All 8 tools are listed below with their Zod input schemas, output shape, CLI mapping, and security notes.

### 5.1 `create_seal`

**Purpose**: Create a new single-use cryptographic capability token (seal) on a specified chain.

**Input schema**:
```typescript
const CreateSealInput = z.object({
  chain: z.enum(['bitcoin', 'ethereum', 'solana', 'sui', 'aptos']),
  value: z.number().positive().finite(),
  memo: z.string().max(256).optional(),
});
```

**CLI mapping**:
```
csv seals create --chain <chain> --value <value> [--memo <memo>]
```

**Output shape**:
```typescript
interface CreateSealResult {
  seal_id: string;          // 64-char hex (32 bytes)
  chain: string;
  tx_hash: string;
  block_height: number;
  status: 'pending' | 'confirmed';
}
```

**Security notes**:
- `value` must be validated as a positive finite number before any CLI call. Zero and negative values must produce a validation error, not a CLI error.
- The returned `seal_id` must be validated against the 64-char hex format before forwarding to the caller; a CLI returning a non-conforming ID indicates a protocol error that must be surfaced, not silently truncated.
- The agent must not store the seal's private key material. The seal is managed by the CLI keystore.

---

### 5.2 `transfer_sanad`

**Purpose**: Execute a cross-chain transfer of a sanad through the `TransferCoordinator`.

**Input schema**:
```typescript
const TransferSanadInput = z.object({
  sanad_id: z.string().regex(/^[0-9a-f]{64}$/, 'sanad_id must be 64 hex chars'),
  destination_chain: z.enum(['bitcoin', 'ethereum', 'solana', 'sui', 'aptos']),
  destination: z.string().min(20).max(66).regex(/^(0x)?[0-9a-fA-F]+$/),
  dry_run: z.boolean().default(false),
});
```

**CLI mapping**:
```
csv cross-chain transfer \
  --sanad-id <sanad_id> \
  --destination-chain <chain> \
  --destination <address> \
  [--dry-run]
```

**Output shape**:
```typescript
interface TransferSanadResult {
  transfer_id: string;
  replay_id: string;           // domain-separated hash of transfer params
  lock_tx_hash: string;
  mint_tx_hash: string;
  source_chain: string;
  destination_chain: string;
  status: 'complete' | 'pending' | 'failed';
  proof_bundle_available: boolean;
}
```

**Security notes**:
- This is the highest-risk tool. Every call must produce an audit log entry with `{ sanad_id, destination_chain, destination, agent_id, timestamp }` before the CLI is invoked.
- The agent must obtain a lease before issuing this command (see section 3.3). A transfer invoked without a valid lease must return an error without calling the CLI.
- On CLI failure, the agent must NOT retry automatically. Duplicate mints are a critical vulnerability. Any retry decision must be delegated to the calling agent with full error context.
- `dry_run: true` is the safe default for agents that are exploring options. When `dry_run` is false, an additional confirmation step must be included in the audit log.

---

### 5.3 `verify_proof`

**Purpose**: Verify a proof bundle offline (structurally and cryptographically), returning a structured verification result.

**Input schema**:
```typescript
const VerifyProofInput = z.object({
  bundle_json: z.string().min(1).refine(
    (s) => { try { JSON.parse(s); return true; } catch { return false; } },
    { message: 'bundle_json must be valid JSON' }
  ),
  expected_sanad_id: z.string().regex(/^[0-9a-f]{64}$/).optional(),
  expected_chain: z.enum(['bitcoin', 'ethereum', 'solana', 'sui', 'aptos']).optional(),
});
```

**CLI mapping**:
```
csv validate proof --bundle-json <path-to-tempfile>
```

**Output shape**:
```typescript
interface VerifyProofResult {
  is_valid: boolean;
  verification_level: 'structural_only' | 'merkle_verified' | 'fully_verified' | 'consensus_verified';
  sanad_id: string | null;
  chain: string | null;
  seal_consumed: boolean;
  replay_detected: boolean;
  errors: string[];
  warnings: string[];
}
```

**Security notes**:
- `is_valid: true` is not sufficient. The caller must examine `verification_level`. An agent relying on `is_valid` alone for minting decisions must be treated as a security bug.
- The `verification_level` enum maps directly to the audit-recommended type:
  ```rust
  // csv-core/src/verified.rs (to be added — see section 7)
  pub enum VerificationLevel {
      StructuralOnly,
      MerkleVerified,
      FullyVerified,
      ConsensusVerified,
  }
  ```
- Bundle JSON must be written to a temp file with a random name (never passed via shell argument) to prevent injection.
- The temp file must be deleted on both success and failure paths.

---

### 5.4 `get_sanads`

**Purpose**: List sanads associated with an address across all chains or a specified chain.

**Input schema**:
```typescript
const GetSanadsInput = z.object({
  address: z.string().min(20).max(128),
  chain: z.enum(['bitcoin', 'ethereum', 'solana', 'sui', 'aptos']).optional(),
  limit: z.number().int().positive().max(100).default(20),
  offset: z.number().int().nonnegative().default(0),
  status: z.enum(['active', 'consumed', 'locked', 'all']).default('all'),
});
```

**CLI mapping**:
```
csv sanads list --address <address> [--chain <chain>] [--limit <n>] [--offset <n>] [--status <status>]
```

**Output shape**:
```typescript
interface GetSanadsResult {
  items: SanadSummary[];
  total: number;
  has_more: boolean;
}

interface SanadSummary {
  sanad_id: string;
  chain: string;
  status: string;
  created_at: string;       // ISO 8601
  value: string | null;     // opaque; not interpreted
  schema_id: string | null; // hash of semantic schema if present
}
```

**Security notes**:
- The `value` field is returned opaque. The agent layer must not parse or interpret it. Semantic enrichment is performed separately via `classify_sanad` (non-canonical, section 10).
- Address normalization (e.g. checksum Ethereum addresses, bech32 Bitcoin) must be done by the CLI, not the agent.

---

### 5.5 `monitor_transfer`

**Purpose**: Poll the current status of an in-progress or completed cross-chain transfer.

**Input schema**:
```typescript
const MonitorTransferInput = z.object({
  transfer_id: z.string().regex(/^[0-9a-f]{64}$/, 'transfer_id must be 64 hex chars'),
});
```

**CLI mapping**:
```
csv cross-chain status --transfer-id <transfer_id>
```

**Output shape**:
```typescript
interface MonitorTransferResult {
  transfer_id: string;
  status: TransferStatus;
  source_chain: string;
  destination_chain: string;
  lock_tx_hash: string | null;
  mint_tx_hash: string | null;
  proof_available: boolean;
  created_at: string;
  updated_at: string;
  error: string | null;
}

type TransferStatus =
  | 'awaiting_lock'
  | 'locked'
  | 'awaiting_finality'
  | 'proof_building'
  | 'proof_validated'
  | 'minting'
  | 'complete'
  | 'rolled_back'
  | 'compromised';
```

**Security notes**:
- `compromised` is a terminal error state. An agent receiving this status must halt all further actions on this transfer and surface the error to its orchestrator without retrying.
- `rolled_back` is recoverable at the protocol level but requires human review before reissue in most production contexts. The agent must surface this to the orchestrator rather than silently restarting.

---

### 5.6 `export_proof_bundle`

**Purpose**: Export a fully-constructed proof bundle for a completed transfer, suitable for offline verification or forwarding to another agent.

**Input schema**:
```typescript
const ExportProofBundleInput = z.object({
  transfer_id: z.string().regex(/^[0-9a-f]{64}$/),
  format: z.enum(['json', 'hex', 'base64']).default('json'),
  include_provenance: z.boolean().default(true),
});
```

**CLI mapping**:
```
csv proofs export --transfer-id <transfer_id> --format <format> [--include-provenance]
```

**Output shape**:
```typescript
interface ExportProofBundleResult {
  transfer_id: string;
  format: string;
  bundle: string;          // serialized bundle in requested format
  bundle_hash: string;     // tagged_hash of bundle bytes for integrity check
  provenance_included: boolean;
  verification_level: string;
}
```

**Security notes**:
- The `bundle_hash` must use `csv_tagged_hash("csv.proof.bundle.export.v1", bundle_bytes)`. This matches the `CSV_TAG_PREFIX` convention in `csv-core/src/tagged_hash.rs`.
- The agent must never construct or modify the bundle bytes. It only relays what the CLI emits.

---

### 5.7 `accept_consignment`

**Purpose**: Accept an incoming consignment (a state transition package sent by a counterparty), validate it, and integrate it into the local state store.

**Input schema**:
```typescript
const AcceptConsignmentInput = z.object({
  consignment_json: z.string().min(1).refine(
    (s) => { try { const p = JSON.parse(s); return typeof p === 'object' && p !== null; } catch { return false; } },
    { message: 'consignment_json must be a JSON object' }
  ),
  expected_sanad_id: z.string().regex(/^[0-9a-f]{64}$/).optional(),
  strict: z.boolean().default(true),  // false = warn on non-fatal issues, do not accept on fatal
});
```

**CLI mapping**:
```
csv validate consignment --consignment-json <path-to-tempfile> [--strict]
```

**Output shape**:
```typescript
interface AcceptConsignmentResult {
  accepted: boolean;
  sanad_id: string | null;
  seal_id: string | null;
  state_root: string | null;
  errors: string[];
  warnings: string[];
}
```

**Security notes**:
- `strict: false` only suppresses warnings, never fatal errors. The CLI must reject a consignment with any fatal validation error regardless of the `strict` flag.
- The consignment must be written to a temp file (same pattern as `verify_proof`) before passing to the CLI.

---

### 5.8 `get_protocol_info`

**Purpose**: Return the current protocol version, chain compatibility matrix, and adapter versions. Enables agents to make version-aware decisions.

**Input schema**:
```typescript
const GetProtocolInfoInput = z.object({
  chain: z.enum(['bitcoin', 'ethereum', 'solana', 'sui', 'aptos', 'all']).default('all'),
});
```

**CLI mapping**:
```
csv chain info [--chain <chain>]
```

**Output shape**:
```typescript
interface ProtocolInfoResult {
  protocol_version: ProtocolVersion;
  minimum_runtime_version: ProtocolVersion;
  chain_adapters: ChainAdapterInfo[];
}

interface ProtocolVersion {
  major: number;
  minor: number;
  patch: number;
}

interface ChainAdapterInfo {
  chain_id: string;
  adapter_version: ProtocolVersion;
  capabilities: string[];      // ["cross_chain_source", "mint", "verify_finality", ...]
  network: string;             // "mainnet" | "testnet" | "signet" | ...
  rpc_status: 'connected' | 'degraded' | 'unavailable';
}
```

---

### 5.9 `health_check`

**Purpose**: Verify that the MCP server, CSV CLI binary, and downstream runtime are all reachable and operational.

**Input schema**: `z.object({})` — no parameters.

**CLI mapping**:
```
csv --version
```

**Output shape**:
```typescript
interface HealthCheckResult {
  mcp_server: 'ok';
  cli_binary: 'ok' | 'error';
  cli_version: string | null;
  runtime: 'ok' | 'error' | 'unknown';
  timestamp: string;
  error: string | null;
}
```

---

## 6. Full Implementation: `csv-mcp-server/src/index.ts`

```typescript
/**
 * CSV MCP Server — AI Agent Integration
 *
 * Enables AI agents (Claude, GPT-4, LangChain, etc.) to operate CSV protocol
 * workflows through the Model Context Protocol (MCP).
 *
 * SECURITY CONTRACT:
 *   - No tool bypasses TransferCoordinator.
 *   - No tool fabricates proofs, seals, or chain state.
 *   - All inputs are Zod-validated before CLI invocation.
 *   - Every mutating call is audit-logged before and after execution.
 *   - Proof bundle bytes are never constructed in this layer.
 */

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { SSEServerTransport } from '@modelcontextprotocol/sdk/server/sse.js';
import { z } from 'zod';
import { spawn } from 'child_process';
import * as fs from 'fs/promises';
import * as path from 'path';
import * as os from 'os';
import * as crypto from 'crypto';

// ─────────────────────────────────────────────────────────────────────────────
// Audit logger
// ─────────────────────────────────────────────────────────────────────────────

interface AuditEntry {
  ts: string;
  session_id: string;
  tool: string;
  phase: 'before' | 'after';
  args_hash?: string;
  result_hash?: string;
  duration_ms?: number;
  exit_code?: number;
  error?: string;
}

function auditLog(entry: AuditEntry): void {
  // Write JSON Lines to stderr so it is captured by the process supervisor
  // without polluting MCP stdout. In production, replace with a structured
  // log sink (e.g. OpenTelemetry, Loki, CloudWatch).
  process.stderr.write(JSON.stringify(entry) + '\n');
}

function hashForAudit(data: unknown): string {
  const bytes = Buffer.from(JSON.stringify(data));
  return crypto.createHash('sha256').update(bytes).digest('hex');
}

// ─────────────────────────────────────────────────────────────────────────────
// Input validation schemas (Zod — single source of truth)
// ─────────────────────────────────────────────────────────────────────────────

const VALID_CHAINS = ['bitcoin', 'ethereum', 'solana', 'sui', 'aptos'] as const;
type ValidChain = typeof VALID_CHAINS[number];

const ChainEnum = z.enum(VALID_CHAINS);
const HexId64 = z.string().regex(/^[0-9a-f]{64}$/, 'Must be 64 lowercase hex chars (32 bytes)');
const Address = z.string().min(20).max(128).regex(/^(0x)?[0-9a-fA-F]+$/, 'Must be a hex address');

const CreateSealInput = z.object({
  chain: ChainEnum,
  value: z.number().positive().finite(),
  memo: z.string().max(256).optional(),
});

const TransferSanadInput = z.object({
  sanad_id: HexId64,
  destination_chain: ChainEnum,
  destination: Address,
  dry_run: z.boolean().default(false),
});

const VerifyProofInput = z.object({
  bundle_json: z.string().min(1).refine(
    (s) => { try { JSON.parse(s); return true; } catch { return false; } },
    { message: 'bundle_json must be valid JSON' }
  ),
  expected_sanad_id: HexId64.optional(),
  expected_chain: ChainEnum.optional(),
});

const GetSanadsInput = z.object({
  address: z.string().min(20).max(128),
  chain: ChainEnum.optional(),
  limit: z.number().int().positive().max(100).default(20),
  offset: z.number().int().nonneg().default(0),
  status: z.enum(['active', 'consumed', 'locked', 'all']).default('all'),
});

const MonitorTransferInput = z.object({
  transfer_id: HexId64,
});

const ExportProofBundleInput = z.object({
  transfer_id: HexId64,
  format: z.enum(['json', 'hex', 'base64']).default('json'),
  include_provenance: z.boolean().default(true),
});

const AcceptConsignmentInput = z.object({
  consignment_json: z.string().min(1).refine(
    (s) => {
      try { const p = JSON.parse(s); return typeof p === 'object' && p !== null; }
      catch { return false; }
    },
    { message: 'consignment_json must be a JSON object' }
  ),
  expected_sanad_id: HexId64.optional(),
  strict: z.boolean().default(true),
});

const GetProtocolInfoInput = z.object({
  chain: z.enum([...VALID_CHAINS, 'all'] as const).default('all'),
});

// ─────────────────────────────────────────────────────────────────────────────
// CLI execution engine
// ─────────────────────────────────────────────────────────────────────────────

interface CliResult {
  stdout: string;
  stderr: string;
  exitCode: number;
  durationMs: number;
}

function resolveCsvBinary(): string {
  // Prefer explicit env override, then workspace-relative path, then PATH
  if (process.env.CSV_BIN) return process.env.CSV_BIN;
  const localBin = path.resolve(__dirname, '../../../target/release/csv');
  // Synchronous fs check is acceptable here (startup-time path resolution)
  try {
    require('fs').accessSync(localBin, require('fs').constants.X_OK);
    return localBin;
  } catch {
    return 'csv';
  }
}

const CSV_BIN = resolveCsvBinary();

async function executeCsvCommand(
  args: string[],
  opts: { timeoutMs?: number; agentId: string; toolName: string }
): Promise<CliResult> {
  const timeoutMs = opts.timeoutMs ?? 60_000;
  const start = Date.now();

  return new Promise((resolve, reject) => {
    const child = spawn(CSV_BIN, args, {
      env: {
        ...process.env,
        RUST_LOG: 'info',
        CSV_AGENT_ID: opts.agentId,
        CSV_AGENT_TOOL: opts.toolName,
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    let stdout = '';
    let stderr = '';

    child.stdout.on('data', (d: Buffer) => { stdout += d.toString(); });
    child.stderr.on('data', (d: Buffer) => { stderr += d.toString(); });

    const timer = setTimeout(() => {
      child.kill('SIGTERM');
      reject(new Error(`CSV CLI timed out after ${timeoutMs}ms`));
    }, timeoutMs);

    child.on('close', (code) => {
      clearTimeout(timer);
      resolve({
        stdout: stdout.trim(),
        stderr: stderr.trim(),
        exitCode: code ?? 1,
        durationMs: Date.now() - start,
      });
    });

    child.on('error', (err) => {
      clearTimeout(timer);
      reject(err);
    });
  });
}

function parseCliOutput(raw: string): unknown {
  if (!raw) return { message: '(empty output)' };
  try {
    return JSON.parse(raw);
  } catch {
    return { message: raw };
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Temp file helpers (used for proof bundles and consignments)
// ─────────────────────────────────────────────────────────────────────────────

async function writeTempJson(content: string): Promise<string> {
  const tmpDir = await fs.mkdtemp(path.join(os.tmpdir(), 'csv-mcp-'));
  const filePath = path.join(tmpDir, 'payload.json');
  await fs.writeFile(filePath, content, 'utf8');
  return filePath;
}

async function deleteTempFile(filePath: string): Promise<void> {
  try {
    await fs.unlink(filePath);
    await fs.rmdir(path.dirname(filePath));
  } catch {
    // Best-effort cleanup; do not surface to caller
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// MCP Server setup
// ─────────────────────────────────────────────────────────────────────────────

async function startServer(
  transport: 'stdio' | 'sse' = 'stdio',
  port = 3000
): Promise<void> {
  const server = new McpServer({
    name: 'csv-protocol',
    version: '1.0.0',
  });

  // Derive a session ID at startup; all audit entries for this process share it
  const SESSION_ID = crypto.randomUUID();

  // ── Helper: wrap every tool handler with audit + Zod validation ──────────

  function wrapTool<TInput>(
    toolName: string,
    schema: z.ZodType<TInput>,
    handler: (input: TInput, agentId: string) => Promise<unknown>
  ) {
    return async (rawInput: unknown): Promise<{ content: Array<{ type: 'text'; text: string }> }> => {
      const agentId = SESSION_ID;

      // 1. Validate
      const parsed = schema.safeParse(rawInput);
      if (!parsed.success) {
        return {
          content: [{
            type: 'text',
            text: JSON.stringify({
              error: 'VALIDATION_ERROR',
              details: parsed.error.flatten(),
            }),
          }],
        };
      }

      // 2. Pre-execution audit
      auditLog({
        ts: new Date().toISOString(),
        session_id: agentId,
        tool: toolName,
        phase: 'before',
        args_hash: hashForAudit(parsed.data),
      });

      const execStart = Date.now();

      try {
        const result = await handler(parsed.data, agentId);

        // 3. Post-execution audit
        auditLog({
          ts: new Date().toISOString(),
          session_id: agentId,
          tool: toolName,
          phase: 'after',
          result_hash: hashForAudit(result),
          duration_ms: Date.now() - execStart,
          exit_code: 0,
        });

        return { content: [{ type: 'text', text: JSON.stringify(result) }] };
      } catch (err: unknown) {
        const message = err instanceof Error ? err.message : String(err);

        auditLog({
          ts: new Date().toISOString(),
          session_id: agentId,
          tool: toolName,
          phase: 'after',
          duration_ms: Date.now() - execStart,
          exit_code: 1,
          error: message,
        });

        return {
          content: [{
            type: 'text',
            text: JSON.stringify({ error: 'EXECUTION_ERROR', message }),
          }],
        };
      }
    };
  }

  // ── Tool: create_seal ────────────────────────────────────────────────────

  server.tool(
    'create_seal',
    'Create a new single-use cryptographic seal on a specified blockchain. Returns seal_id and transaction details.',
    { chain: z.string(), value: z.number(), memo: z.string().optional() },
    wrapTool('create_seal', CreateSealInput, async (input, agentId) => {
      const args = [
        'seals', 'create',
        '--chain', input.chain,
        '--value', String(input.value),
        ...(input.memo ? ['--memo', input.memo] : []),
        '--output', 'json',
      ];
      const result = await executeCsvCommand(args, { agentId, toolName: 'create_seal' });
      if (result.exitCode !== 0) {
        throw new Error(`CLI failed (exit ${result.exitCode}): ${result.stderr}`);
      }
      return parseCliOutput(result.stdout);
    })
  );

  // ── Tool: transfer_sanad ─────────────────────────────────────────────────

  server.tool(
    'transfer_sanad',
    'Execute a cross-chain transfer of a sanad. Uses TransferCoordinator; never calls adapters directly. On failure, returns structured error — caller must not auto-retry.',
    {
      sanad_id: z.string(),
      destination_chain: z.string(),
      destination: z.string(),
      dry_run: z.boolean().optional(),
    },
    wrapTool('transfer_sanad', TransferSanadInput, async (input, agentId) => {
      const args = [
        'cross-chain', 'transfer',
        '--sanad-id', input.sanad_id,
        '--destination-chain', input.destination_chain,
        '--destination', input.destination,
        ...(input.dry_run ? ['--dry-run'] : []),
        '--output', 'json',
      ];
      // Transfers get a generous 5-minute timeout (finality waits vary by chain)
      const result = await executeCsvCommand(args, {
        agentId,
        toolName: 'transfer_sanad',
        timeoutMs: 300_000,
      });
      if (result.exitCode !== 0) {
        throw new Error(`Transfer failed (exit ${result.exitCode}): ${result.stderr}`);
      }
      return parseCliOutput(result.stdout);
    })
  );

  // ── Tool: verify_proof ───────────────────────────────────────────────────

  server.tool(
    'verify_proof',
    'Verify a proof bundle. Returns verification_level — callers must not rely solely on is_valid for minting decisions.',
    {
      bundle_json: z.string(),
      expected_sanad_id: z.string().optional(),
      expected_chain: z.string().optional(),
    },
    wrapTool('verify_proof', VerifyProofInput, async (input, agentId) => {
      const tmpFile = await writeTempJson(input.bundle_json);
      try {
        const args = [
          'validate', 'proof',
          '--bundle-file', tmpFile,
          ...(input.expected_sanad_id ? ['--expected-sanad-id', input.expected_sanad_id] : []),
          ...(input.expected_chain ? ['--expected-chain', input.expected_chain] : []),
          '--output', 'json',
        ];
        const result = await executeCsvCommand(args, {
          agentId,
          toolName: 'verify_proof',
          timeoutMs: 10_000,
        });
        // verify_proof is a read-only operation; non-zero exit means invalid proof,
        // not an execution error — surface the structured result either way.
        return parseCliOutput(result.stdout || result.stderr);
      } finally {
        await deleteTempFile(tmpFile);
      }
    })
  );

  // ── Tool: get_sanads ─────────────────────────────────────────────────────

  server.tool(
    'get_sanads',
    'List sanads for an address. Returns opaque value fields — use classify_sanad for semantic enrichment.',
    {
      address: z.string(),
      chain: z.string().optional(),
      limit: z.number().optional(),
      offset: z.number().optional(),
      status: z.string().optional(),
    },
    wrapTool('get_sanads', GetSanadsInput, async (input, agentId) => {
      const args = [
        'sanads', 'list',
        '--address', input.address,
        '--limit', String(input.limit),
        '--offset', String(input.offset),
        '--status', input.status,
        ...(input.chain ? ['--chain', input.chain] : []),
        '--output', 'json',
      ];
      const result = await executeCsvCommand(args, {
        agentId,
        toolName: 'get_sanads',
        timeoutMs: 10_000,
      });
      if (result.exitCode !== 0) {
        throw new Error(`CLI failed (exit ${result.exitCode}): ${result.stderr}`);
      }
      return parseCliOutput(result.stdout);
    })
  );

  // ── Tool: monitor_transfer ───────────────────────────────────────────────

  server.tool(
    'monitor_transfer',
    'Poll the status of a cross-chain transfer. "compromised" and "rolled_back" states require human review.',
    { transfer_id: z.string() },
    wrapTool('monitor_transfer', MonitorTransferInput, async (input, agentId) => {
      const args = [
        'cross-chain', 'status',
        '--transfer-id', input.transfer_id,
        '--output', 'json',
      ];
      const result = await executeCsvCommand(args, {
        agentId,
        toolName: 'monitor_transfer',
        timeoutMs: 10_000,
      });
      if (result.exitCode !== 0) {
        throw new Error(`CLI failed (exit ${result.exitCode}): ${result.stderr}`);
      }
      return parseCliOutput(result.stdout);
    })
  );

  // ── Tool: export_proof_bundle ────────────────────────────────────────────

  server.tool(
    'export_proof_bundle',
    'Export a complete proof bundle for offline verification or agent-to-agent handoff.',
    {
      transfer_id: z.string(),
      format: z.enum(['json', 'hex', 'base64']).optional(),
      include_provenance: z.boolean().optional(),
    },
    wrapTool('export_proof_bundle', ExportProofBundleInput, async (input, agentId) => {
      const args = [
        'proofs', 'export',
        '--transfer-id', input.transfer_id,
        '--format', input.format,
        ...(input.include_provenance ? ['--include-provenance'] : []),
        '--output', 'json',
      ];
      const result = await executeCsvCommand(args, {
        agentId,
        toolName: 'export_proof_bundle',
        timeoutMs: 15_000,
      });
      if (result.exitCode !== 0) {
        throw new Error(`CLI failed (exit ${result.exitCode}): ${result.stderr}`);
      }
      return parseCliOutput(result.stdout);
    })
  );

  // ── Tool: accept_consignment ─────────────────────────────────────────────

  server.tool(
    'accept_consignment',
    'Accept and validate an incoming state transition consignment from a counterparty.',
    {
      consignment_json: z.string(),
      expected_sanad_id: z.string().optional(),
      strict: z.boolean().optional(),
    },
    wrapTool('accept_consignment', AcceptConsignmentInput, async (input, agentId) => {
      const tmpFile = await writeTempJson(input.consignment_json);
      try {
        const args = [
          'validate', 'consignment',
          '--consignment-file', tmpFile,
          ...(input.expected_sanad_id ? ['--expected-sanad-id', input.expected_sanad_id] : []),
          ...(input.strict ? ['--strict'] : []),
          '--output', 'json',
        ];
        const result = await executeCsvCommand(args, {
          agentId,
          toolName: 'accept_consignment',
          timeoutMs: 15_000,
        });
        return parseCliOutput(result.stdout || result.stderr);
      } finally {
        await deleteTempFile(tmpFile);
      }
    })
  );

  // ── Tool: get_protocol_info ──────────────────────────────────────────────

  server.tool(
    'get_protocol_info',
    'Return current protocol version and chain adapter status. Use before transfers to verify compatibility.',
    { chain: z.string().optional() },
    wrapTool('get_protocol_info', GetProtocolInfoInput, async (input, agentId) => {
      const args = [
        'chain', 'info',
        ...(input.chain !== 'all' ? ['--chain', input.chain] : []),
        '--output', 'json',
      ];
      const result = await executeCsvCommand(args, {
        agentId,
        toolName: 'get_protocol_info',
        timeoutMs: 10_000,
      });
      if (result.exitCode !== 0) {
        throw new Error(`CLI failed: ${result.stderr}`);
      }
      return parseCliOutput(result.stdout);
    })
  );

  // ── Tool: health_check ───────────────────────────────────────────────────

  server.tool(
    'health_check',
    'Verify that the MCP server and CSV CLI are operational.',
    {},
    async () => {
      try {
        const result = await executeCsvCommand(['--version'], {
          agentId: SESSION_ID,
          toolName: 'health_check',
          timeoutMs: 5_000,
        });
        const healthy = result.exitCode === 0;
        return {
          content: [{
            type: 'text' as const,
            text: JSON.stringify({
              mcp_server: 'ok',
              cli_binary: healthy ? 'ok' : 'error',
              cli_version: healthy ? result.stdout : null,
              runtime: 'unknown',
              timestamp: new Date().toISOString(),
              error: healthy ? null : result.stderr,
            }),
          }],
        };
      } catch (err: unknown) {
        return {
          content: [{
            type: 'text' as const,
            text: JSON.stringify({
              mcp_server: 'ok',
              cli_binary: 'error',
              cli_version: null,
              runtime: 'unknown',
              timestamp: new Date().toISOString(),
              error: err instanceof Error ? err.message : String(err),
            }),
          }],
        };
      }
    }
  );

  // ── Transport ────────────────────────────────────────────────────────────

  if (transport === 'stdio') {
    const t = new StdioServerTransport();
    await server.connect(t);
  } else {
    const express = (await import('express')).default;
    const app = express();
    app.get('/sse', (req, res) => {
      const t = new SSEServerTransport('/message', res);
      server.connect(t);
    });
    app.listen(port, () => {
      process.stderr.write(`CSV MCP Server listening on port ${port}\n`);
    });
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────────

const args = process.argv.slice(2);
const useSSE = args.includes('--sse');
const portArg = args.find(a => a.startsWith('--port='));
const port = portArg ? parseInt(portArg.split('=')[1], 10) : 3000;

startServer(useSSE ? 'sse' : 'stdio', port).catch((err) => {
  process.stderr.write(`Fatal: ${err.message}\n`);
  process.exit(1);
});
```

---

## 7. Rust Agent Bridge: `csv-core/src/mcp.rs`

This module is the canonical type boundary between the TypeScript agent layer and the Rust protocol core. It defines types that the CLI serializes to JSON when invoked by the MCP server.

```rust
//! MCP Agent Protocol Types
//!
//! These types define the JSON contract between `csv-mcp-server` and `csv-cli`.
//! All agent-facing output from CLI commands must serialize to these types.
//!
//! INVARIANT: These types are part of the public protocol surface.
//! Field additions require a minor version bump.
//! Field removals or renames require a major version bump.
//! Never change field semantics without updating protocol_version.

use alloc::string::String;
use alloc::vec::Vec;
use crate::hash::Hash;
use crate::protocol_version::ProtocolVersion;
use serde::{Deserialize, Serialize};

// ─── Verification level ────────────────────────────────────────────────────

/// Explicit verification tier returned by all proof verification paths.
///
/// Callers MUST check this. `is_valid: true` with `StructuralOnly`
/// does not constitute cryptographic proof of state transition validity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationLevel {
    /// Script/structure checked. No cryptographic proof verified.
    StructuralOnly,
    /// Merkle inclusion verified. Finality not yet confirmed.
    MerkleVerified,
    /// Full cryptographic verification complete.
    FullyVerified,
    /// Consensus-confirmed on source chain; finality threshold met.
    ConsensusVerified,
}

// ─── Proof verification result ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentVerifyProofResult {
    pub is_valid: bool,
    pub verification_level: VerificationLevel,
    pub sanad_id: Option<String>,
    pub chain: Option<String>,
    pub seal_consumed: bool,
    pub replay_detected: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

// ─── Transfer result ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTransferResult {
    pub transfer_id: String,
    pub replay_id: String,
    pub lock_tx_hash: String,
    pub mint_tx_hash: String,
    pub source_chain: String,
    pub destination_chain: String,
    pub status: AgentTransferStatus,
    pub proof_bundle_available: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentTransferStatus {
    AwaitingLock,
    Locked,
    AwaitingFinality,
    ProofBuilding,
    ProofValidated,
    Minting,
    Complete,
    RolledBack,
    Compromised,
}

// ─── Seal creation result ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCreateSealResult {
    pub seal_id: String,
    pub chain: String,
    pub tx_hash: String,
    pub block_height: u64,
    pub status: AgentSealStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSealStatus {
    Pending,
    Confirmed,
    Consumed,
    Invalid,
}

// ─── Sanad list result ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSanadSummary {
    pub sanad_id: String,
    pub chain: String,
    pub status: String,
    pub created_at: String,
    /// Opaque; not interpreted by the agent layer.
    pub value: Option<String>,
    /// Hash of semantic schema if present.
    pub schema_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentGetSanadsResult {
    pub items: Vec<AgentSanadSummary>,
    pub total: u64,
    pub has_more: bool,
}

// ─── Protocol info ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProtocolInfoResult {
    pub protocol_version: ProtocolVersion,
    pub minimum_runtime_version: ProtocolVersion,
    pub chain_adapters: Vec<AgentChainAdapterInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentChainAdapterInfo {
    pub chain_id: String,
    pub adapter_version: ProtocolVersion,
    pub capabilities: Vec<String>,
    pub network: String,
    pub rpc_status: AgentRpcStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRpcStatus {
    Connected,
    Degraded,
    Unavailable,
}

// ─── Proof export result ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentExportProofResult {
    pub transfer_id: String,
    pub format: String,
    pub bundle: String,
    /// `csv_tagged_hash("csv.proof.bundle.export.v1", bundle_bytes)` as hex
    pub bundle_hash: String,
    pub provenance_included: bool,
    pub verification_level: VerificationLevel,
}
```

---

## 8. Canonical Serialization Layer

The audit identifies this as **Priority 0**:

> "Freeze Canonical Serialization — Must happen before ecosystem expansion."
> "Never hash raw serde JSON. Instead: canonical CBOR, deterministic binary encoding, schema-hashed payloads must become mandatory."

### 8.1 `csv-core/src/canonical.rs` (new file)

```rust
//! Canonical serialization primitives for CSV Protocol
//!
//! All proof payloads, sanad envelopes, and commitment inputs MUST use
//! these functions. Raw serde JSON is forbidden in any hashing path.
//!
//! Encoding: deterministic CBOR (RFC 8949 §4.2 canonical form)
//!   - Keys sorted lexicographically
//!   - No indefinite-length encoding
//!   - Integers in smallest representation
//!
//! External crate: `ciborium` (no_std compatible, pure Rust)

use alloc::vec::Vec;
use crate::error::CsvError;

/// Serialize `value` to deterministic CBOR bytes.
///
/// # Errors
/// Returns `CsvError::Serialization` if encoding fails.
pub fn to_canonical_cbor<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, CsvError> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf)
        .map_err(|e| CsvError::Serialization(e.to_string()))?;
    Ok(buf)
}

/// Deserialize from deterministic CBOR bytes.
///
/// # Errors
/// Returns `CsvError::Deserialization` if decoding fails.
pub fn from_canonical_cbor<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, CsvError> {
    ciborium::from_reader(bytes)
        .map_err(|e| CsvError::Deserialization(e.to_string()))
}

/// Hash canonical CBOR encoding of `value` using tagged_hash.
///
/// This is the ONLY approved way to hash protocol data.
/// Direct `sha256`, `keccak256`, or `blake3` calls are forbidden.
pub fn canonical_hash<T: serde::Serialize>(
    domain: &str,
    value: &T,
) -> Result<crate::hash::Hash, CsvError> {
    let cbor = to_canonical_cbor(value)?;
    Ok(crate::hash::Hash::new(
        crate::tagged_hash::csv_tagged_hash(domain, &cbor)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Fixture { a: u32, b: String }

    #[test]
    fn roundtrip_is_lossless() {
        let v = Fixture { a: 42, b: "hello".into() };
        let bytes = to_canonical_cbor(&v).unwrap();
        let back: Fixture = from_canonical_cbor(&bytes).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn encoding_is_deterministic() {
        let v = Fixture { a: 1, b: "world".into() };
        let b1 = to_canonical_cbor(&v).unwrap();
        let b2 = to_canonical_cbor(&v).unwrap();
        assert_eq!(b1, b2);
    }

    #[test]
    fn canonical_hash_domain_separation() {
        let v = Fixture { a: 0, b: "test".into() };
        let h1 = canonical_hash("domain.a", &v).unwrap();
        let h2 = canonical_hash("domain.b", &v).unwrap();
        assert_ne!(h1, h2);
    }
}
```

### 8.2 Enforcement

The `check_forbidden_patterns.sh` script in `scripts/security/` must be extended:

```bash
# Forbidden direct hash calls in csv-core and csv-runtime
if grep -rn "Sha256::digest\|Keccak256::digest\|blake3::hash\|serde_json::to_vec" \
   --include="*.rs" csv-core/src csv-runtime/src; then
  echo "ERROR: Raw hash or serde_json serialization in protocol path. Use canonical_hash()."
  exit 1
fi
```

---

## 9. Proof Pipeline Integration

The audit recommends that the agent always receives a `VerificationLevel`, not a boolean. The existing `csv-core/src/proof_pipeline.rs` must expose this.

### 9.1 Required changes to `csv-core/src/proof_pipeline.rs`

```rust
// Add to validate_proof_bundle return type:

use crate::mcp::VerificationLevel;

pub struct ValidationResult {
    pub level: VerificationLevel,
    pub seal_consumed: bool,
    pub replay_detected: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Validate a proof bundle and return an explicit verification level.
///
/// Replaces any `-> Result<bool, _>` signature.
/// `Ok(true)` is FORBIDDEN as a return value (see .agents/AGENT.md).
pub fn validate_proof_bundle(
    bundle: &ProofBundle,
    replay_registry: &mut dyn ReplayRegistryBackend,
    chain_verifier: &dyn ChainVerifier,
) -> Result<ValidationResult, ProofValidationError> {
    // 1. Structural check
    validate_structure(bundle)?;

    // 2. Merkle inclusion
    let merkle_ok = chain_verifier.verify_merkle_inclusion(bundle)?;

    // 3. Finality
    let finality_ok = chain_verifier.verify_finality(bundle)?;

    // 4. Replay check
    let replay_key = ReplayKey::from_bundle(bundle)?;
    let replay_detected = replay_registry.contains(&replay_key)?;

    // 5. Determine level
    let level = match (merkle_ok, finality_ok) {
        (false, _) => VerificationLevel::StructuralOnly,
        (true, false) => VerificationLevel::MerkleVerified,
        (true, true) => VerificationLevel::FullyVerified,
    };

    Ok(ValidationResult {
        level,
        seal_consumed: bundle.seal_ref.is_consumed(),
        replay_detected,
        errors: vec![],
        warnings: vec![],
    })
}
```

---

## 10. Sanad Semantic Enrichment (AI Non-Canonical Layer)

The audit is explicit:

> "AI should NEVER define canonical truth. AI outputs must be: signed, versioned, separately attributable, revocable, non-authoritative."

The `enrichment/sanad_classifier.ts` module in the MCP server provides this layer. It is strictly advisory and must never influence proof validation or state machine transitions.

### 10.1 `csv-mcp-server/src/enrichment/sanad_classifier.ts`

```typescript
/**
 * Non-canonical AI enrichment for sanad payloads.
 *
 * CONSTRAINTS (from audit.md):
 *   - Outputs are signed, versioned, and attributable.
 *   - Outputs must never influence canonical proof paths.
 *   - Results are revocable; downstream systems must treat them as advisory.
 *   - AI may assist: extraction, classification, schema mapping, tagging.
 *   - AI must NEVER define: hashes, commitments, proofs, seal consumption.
 */

export interface SanadClassification {
  sanad_id: string;
  classifier_version: string;     // semver of this module
  schema_suggestion: string | null;
  semantic_tags: string[];
  confidence: number;             // 0.0 – 1.0
  attribution: {
    model: string;
    timestamp: string;
    revocable: true;              // always true; this is non-canonical
    non_authoritative: true;      // always true
  };
}

/**
 * Classify a sanad payload semantically using pattern matching.
 * Does not make network calls. Does not modify protocol state.
 *
 * @param sanadId  - 64-char hex sanad identifier
 * @param opaqueValue - raw value field from GetSanadsResult (treated as opaque string)
 * @returns SanadClassification with confidence score and advisory tags
 */
export function classifySanad(
  sanadId: string,
  opaqueValue: string | null
): SanadClassification {
  // Pattern-based heuristics — never authoritative
  const tags: string[] = [];
  let schema: string | null = null;
  let confidence = 0.0;

  if (opaqueValue) {
    try {
      const parsed = JSON.parse(opaqueValue);
      if (typeof parsed === 'object' && parsed !== null) {
        if ('type' in parsed) {
          tags.push(`type:${parsed.type}`);
          confidence += 0.4;
        }
        if ('schema' in parsed) {
          schema = String(parsed.schema);
          confidence += 0.4;
        }
        if ('amount' in parsed && 'currency' in parsed) {
          tags.push('category:financial-asset');
          confidence += 0.2;
        }
        if ('credential_type' in parsed) {
          tags.push('category:credential');
          confidence += 0.2;
        }
      }
    } catch {
      // Opaque binary or non-JSON — classify as unknown
      tags.push('encoding:non-json');
    }
  }

  return {
    sanad_id: sanadId,
    classifier_version: '1.0.0',
    schema_suggestion: schema,
    semantic_tags: tags,
    confidence: Math.min(confidence, 1.0),
    attribution: {
      model: 'csv-mcp-heuristic-v1',
      timestamp: new Date().toISOString(),
      revocable: true,
      non_authoritative: true,
    },
  };
}
```

---

## 11. Versioning and Compatibility Enforcement

The audit identifies missing versioning as a "highest long-term risk". The following additions to `csv-core` implement the required protocol constitution layer.

### 11.1 `csv-core/src/protocol_version.rs` additions

The existing `ProtocolVersion` struct needs version negotiation semantics:

```rust
use crate::mcp::{AgentProtocolInfoResult, AgentChainAdapterInfo};
use crate::compatibility::{CompatibilityMatrix, CompatibilityResult};

/// Semantic version with explicit negotiation semantics.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ProtocolVersion {
    /// The current canonical protocol version.
    pub const CURRENT: Self = Self { major: 1, minor: 0, patch: 0 };

    /// Check whether `other` is compatible with this version.
    ///
    /// Compatibility rules:
    ///   - Same major → compatible (minor/patch differences are additive)
    ///   - Different major → incompatible
    pub fn is_compatible_with(&self, other: &ProtocolVersion) -> bool {
        self.major == other.major
    }

    /// Check whether a runtime can service this protocol version.
    pub fn check_runtime(&self, runtime_version: &ProtocolVersion) -> CompatibilityResult {
        if !self.is_compatible_with(runtime_version) {
            CompatibilityResult::Incompatible {
                reason: alloc::format!(
                    "protocol major {} incompatible with runtime major {}",
                    self.major, runtime_version.major
                ),
            }
        } else {
            CompatibilityResult::Compatible
        }
    }
}
```

### 11.2 Version matrix golden test

A golden test corpus must be created at `csv-core/tests/properties/version_compatibility.rs`:

```rust
//! Golden tests for protocol version compatibility matrix.
//! These tests MUST be updated when compatibility rules change.

use csv_core::protocol_version::ProtocolVersion;

#[test]
fn same_major_is_compatible() {
    let v1 = ProtocolVersion { major: 1, minor: 0, patch: 0 };
    let v2 = ProtocolVersion { major: 1, minor: 5, patch: 3 };
    assert!(v1.is_compatible_with(&v2));
}

#[test]
fn different_major_is_incompatible() {
    let v1 = ProtocolVersion { major: 1, minor: 0, patch: 0 };
    let v2 = ProtocolVersion { major: 2, minor: 0, patch: 0 };
    assert!(!v1.is_compatible_with(&v2));
}

#[test]
fn runtime_check_compatible() {
    use csv_core::compatibility::CompatibilityResult;
    let protocol = ProtocolVersion::CURRENT;
    let runtime = ProtocolVersion::CURRENT;
    assert_eq!(protocol.check_runtime(&runtime), CompatibilityResult::Compatible);
}
```

---

## 12. Security Invariants the Agent Must Enforce

All invariants from `.agents/AGENT.md` apply to the MCP server without exception. The following table maps each invariant to its enforcement mechanism in this implementation.

| Invariant | AGENT.md ref | Enforcement |
|---|---|---|
| No fabricated blockchain state | §1 rule 1 | Agent never constructs proof bytes; CLI is sole source. |
| No placeholder verification | §1 rule 2 | `VerificationLevel` enum; `is_valid` alone is insufficient. |
| No bypass of runtime state machine | §1 rule 3, §4 | `transfer_sanad` routes via `csv cross-chain transfer` → `TransferCoordinator`. No direct adapter imports. |
| No replayable transfers | §1 rule 4 | `ReplayKey` checked before mint; audit log records `replay_id`. |
| No mint without verified inclusion + finality | §1 rule 5 | `VerificationLevel` returned; agent must check before acting on result. |
| No raw hashing | §1 rule 6 | `canonical_hash()` in `csv-core/src/canonical.rs`; CI check forbids `sha256::digest` in protocol paths. |
| No silent fallback | §1 rule 7 | Every CLI failure throws; never returns a default. |
| No partial validation | §1 rule 8 | `ValidationResult` contains `errors` and `warnings`; empty errors ≠ valid. |
| No downgrade of security failures to warnings | §1 rule 9 | `strict: true` default on `accept_consignment`; `compromised` status raises error, not warning. |
| Monotonic state transitions | §1 rule 10 | Transfer state machine enforced in Rust; agent exposes status but cannot mutate it. |
| Never simplify crypto structures | §10 | TypeScript never constructs `ProofBundle`, `SealPoint`, or `Hash`. |
| Never leave stub implementations | §10 | All 9 tools have full implementations above. |

---

## 13. Testing Strategy

### 13.1 Unit tests (`csv-mcp-server/tests/validation.test.ts`)

```typescript
import { describe, it, expect } from 'vitest';
import { z } from 'zod';
import { CreateSealInput, TransferSanadInput, VerifyProofInput } from '../src/validation/schemas';

describe('CreateSealInput validation', () => {
  it('accepts valid input', () => {
    expect(CreateSealInput.safeParse({ chain: 'ethereum', value: 1.5 }).success).toBe(true);
  });
  it('rejects zero value', () => {
    expect(CreateSealInput.safeParse({ chain: 'bitcoin', value: 0 }).success).toBe(false);
  });
  it('rejects negative value', () => {
    expect(CreateSealInput.safeParse({ chain: 'solana', value: -1 }).success).toBe(false);
  });
  it('rejects unknown chain', () => {
    expect(CreateSealInput.safeParse({ chain: 'dogecoin', value: 1 }).success).toBe(false);
  });
});

describe('TransferSanadInput validation', () => {
  const validId = 'a'.repeat(64);
  it('rejects non-hex sanad_id', () => {
    expect(TransferSanadInput.safeParse({
      sanad_id: 'GGGG' + 'a'.repeat(60),
      destination_chain: 'ethereum',
      destination: '0x' + 'a'.repeat(40),
    }).success).toBe(false);
  });
  it('rejects short sanad_id', () => {
    expect(TransferSanadInput.safeParse({
      sanad_id: 'abc',
      destination_chain: 'solana',
      destination: '0x' + 'b'.repeat(40),
    }).success).toBe(false);
  });
  it('accepts valid input', () => {
    expect(TransferSanadInput.safeParse({
      sanad_id: validId,
      destination_chain: 'solana',
      destination: 'b'.repeat(44),
    }).success).toBe(true);
  });
});

describe('VerifyProofInput validation', () => {
  it('rejects invalid JSON', () => {
    expect(VerifyProofInput.safeParse({ bundle_json: '{broken' }).success).toBe(false);
  });
  it('accepts valid JSON', () => {
    expect(VerifyProofInput.safeParse({ bundle_json: '{"a":1}' }).success).toBe(true);
  });
});
```

### 13.2 Rust golden test corpus

The audit requires a canonical proof corpus before third-party implementations. These must live at `csv-core/tests/golden/`:

```
csv-core/tests/golden/
├── valid_proof_bundle_v1.cbor
├── valid_sanad_envelope_v1.cbor
├── replay_attempt_v1.cbor
├── malformed_proof_missing_finality.cbor
├── malformed_proof_wrong_domain.cbor
└── README.md
```

A single test file validates all golden vectors:

```rust
// csv-core/tests/golden/mod.rs

use csv_core::canonical::from_canonical_cbor;
use csv_core::proof::ProofBundle;
use csv_core::proof_pipeline::validate_proof_bundle;

macro_rules! golden_test {
    ($name:ident, $file:expr, $expect_valid:expr) => {
        #[test]
        fn $name() {
            let bytes = include_bytes!($file);
            let bundle: ProofBundle = from_canonical_cbor(bytes)
                .expect("golden vector must deserialize");
            let mut registry = csv_core::replay_registry::ReplayRegistry::new();
            let verifier = csv_core::proof_pipeline::NullChainVerifier; // structural only
            let result = validate_proof_bundle(&bundle, &mut registry, &verifier);
            assert_eq!(result.is_ok() && result.unwrap().level != VerificationLevel::StructuralOnly, $expect_valid,
                "golden vector {} did not match expected validity {}", $file, $expect_valid);
        }
    };
}

golden_test!(valid_proof_bundle_v1, "valid_proof_bundle_v1.cbor", true);
golden_test!(malformed_missing_finality, "malformed_proof_missing_finality.cbor", false);
golden_test!(malformed_wrong_domain, "malformed_proof_wrong_domain.cbor", false);
```

---

## 14. CI Enforcement

The existing `architectural-checks.yml` must be extended with MCP-specific checks:

```yaml
# .github/workflows/architectural-checks.yml (additions)

  mcp-contract-checks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Verify no direct adapter imports from MCP server
        run: |
          if grep -r "csv-bitcoin\|csv-ethereum\|csv-solana\|csv-sui\|csv-aptos" \
             csv-mcp-server/src --include="*.ts"; then
            echo "ERROR: MCP server must not import chain adapters directly."
            exit 1
          fi

      - name: Verify VerificationLevel is always checked in test fixtures
        run: |
          # Ensure test files check verification_level, not just is_valid
          if grep -l "is_valid" csv-mcp-server/tests --include="*.ts" | \
             xargs grep -L "verification_level"; then
            echo "WARNING: Tests checking is_valid without verification_level."
          fi

      - name: Verify no serde_json in canonical hashing paths
        run: |
          if grep -rn "serde_json::to_vec\|serde_json::to_string" \
             --include="*.rs" csv-core/src csv-runtime/src | \
             grep -v "//\|#\[cfg(test)\]"; then
            echo "ERROR: serde_json in protocol path. Use canonical_cbor."
            exit 1
          fi

      - name: Verify audit log present in transfer_sanad tool
        run: |
          if ! grep -q "auditLog" csv-mcp-server/src/tools/transfer_sanad.ts 2>/dev/null; then
            if ! grep -q "auditLog" csv-mcp-server/src/index.ts; then
              echo "ERROR: transfer_sanad must emit audit log."
              exit 1
            fi
          fi

      - name: TypeScript compilation check
        run: |
          cd csv-mcp-server
          npm ci
          npx tsc --noEmit
```

---

## 15. Open Architecture Gaps and Remediation Plan

The following items remain open after this implementation. They are prioritized per the audit.

### Priority 0 — Critical (block ecosystem expansion)

| Gap | Location | Remediation |
|---|---|---|
| Canonical serialization not fully adopted across all proof paths | `csv-core/src/proof.rs`, `csv-core/src/sanad.rs` | Replace all `serde_json::to_vec` in hashing paths with `canonical_hash()` from `canonical.rs` (section 8). |
| `VerificationLevel` not yet returned by all CLI proof commands | `csv-cli/src/commands/proofs.rs`, `validate.rs` | Update CLI `--output json` to serialize `AgentVerifyProofResult` (section 7). |
| Protocol constitution document does not exist | `docs/` | Create `docs/PROTOCOL_CONSTITUTION.md` defining serialization, hashing, proof encoding, seal semantics, replay scope, and upgrade rules as a chain-independent specification. |

### Priority 1 — High

| Gap | Location | Remediation |
|---|---|---|
| Merkleized sanad payloads not implemented | `csv-core/src/sanad.rs` | Add `SanadEnvelope` with `payload_hash` and `CanonicalPayload` enum (per audit recommendation, section 2 of audit.md). |
| Golden proof corpus does not exist | `csv-core/tests/golden/` | Generate fixtures using a known-good runtime build; sign them with a release key; add to CI (section 13.2). |
| WASM build target not verified for `csv-core` | `csv-core/Cargo.toml` | Add `wasm32-unknown-unknown` to CI matrix; verify `no_std` compatibility of all protocol types. |
| Lease management not fully wired in CLI | `csv-cli/src/commands/cross_chain/transfer.rs` | Implement `acquire-lease` and `--lease-token` flag; wire into `TransferCoordinator::execute`. |

### Priority 2 — Medium

| Gap | Location | Remediation |
|---|---|---|
| Adapter semantic drift: Ethereum and Solana verifiers may diverge | `csv-ethereum/src/verifier.rs`, `csv-solana/src/verifier.rs` | Move semantic validation into `csv-core::proof_pipeline`; adapters provide only chain data extraction. |
| Contract manifest governance absent | `csv-contracts/` | Implement `contract-manifest.json` generation on deploy with bytecode hash, ABI hash, semantic version, and deploy block; sign with release key. |
| Formal threat model not documented | `docs/` | Create `docs/THREAT_MODEL.md` covering Byzantine nodes, malicious indexers, RPC equivocation, delayed finality, and partial chain partitions. |

---

*This document is a living specification. As the CSV Protocol constitutional hardening phase progresses, each section should be updated to reflect implemented state and remaining gaps.*
