/**
 * @csv-adapter/sdk — TypeScript SDK for Client-Side Validation (CSV) Protocol
 *
 * Scalable, maintainable, real-world DeFi application maximizing
 * CSV + Single-Use Seal advantages.
 *
 * ## Key Features
 *
 * - **Cross-chain digital rights** — Rights that move between blockchains
 * - **Single-use seals** — Chain-enforced one-time consumption
 * - **Offline verification** — Verify proofs without RPC calls
 * - **No trusted bridge** — Self-verifying proof bundles
 * - **Cryptographic double-spend prevention** — Math, not trust
 *
 * ## Quick Start
 *
 * ```typescript
 * import { CsvClient } from '@csv-adapter/sdk';
 *
 * const client = new CsvClient({
 *   defaultChain: 'bitcoin',
 *   network: 'signet',
 * });
 *
 * // Verify a proof bundle offline
 * const result = client.verifyProofBundleFromJson(bundleJson);
 * if (result.valid) {
 *   console.log('Proof is valid!');
 * }
 * ```
 */

// Core types
export {
  Chain,
  SignatureScheme,
  ProtocolVersion,
  ErrorCode,
  TransferStatus,
  SyncStatus,
  Capabilities,
  parseChain,
  chainToString,
  hexToBytes,
  bytesToHex,
  hexToNumber,
  numberToHex,
} from './types';

// Seal types
export {
  SealRef,
  AnchorRef,
  sealRefFromHex,
  sealRefFromJson,
  sealRefToJson,
  anchorRefFromHex,
  anchorRefFromJson,
  anchorRefToJson,
} from './seal';

// Right types
export {
  Right,
  OwnershipProof,
  rightFromHex,
  rightFromJson,
  rightToJson,
  ownershipProofFromHex,
  ownershipProofToJson,
} from './right';

// Proof types
export {
  ProofBundle,
  InclusionProof,
  FinalityProof,
  DAGNode,
  DAGSegment,
  inclusionProofFromHex,
  finalityProofFromHex,
  proofBundleFromJson,
  proofBundleToJson,
} from './proof';

// Consignment types
export {
  Consignment,
  ConsignmentAnchor,
  SealAssignment,
  Commitment,
  Genesis,
  Transition,
  Metadata,
  GlobalState,
  OwnedState,
  StateAssignment,
  StateRef,
  genesisFromHex,
  consignmentFromHex,
  consignmentToJson,
} from './consignment';

// Verification
export {
  VerificationResult,
  VerificationStep,
  verifyProofBundle,
  verifyConsignment,
  verifyProofBundleFromJson,
} from './verify';

// Client
export { CsvClient, CsvClientConfig } from './client';

// Chain-specific utilities
export { BitcoinChain } from './chains/bitcoin';
export { EthereumChain } from './chains/ethereum';
export { SuiChain } from './chains/sui';
export { AptosChain } from './chains/aptos';
export { SolanaChain } from './chains/solana';

// Protocol constants
export const COMMITMENT_VERSION = 2;
export const CONSIGNMENT_VERSION = 1;
export const SCHEMA_VERSION = 1;

/**
 * CSV Protocol version string.
 */
export const PROTOCOL_VERSION = '0.4.0';
