/**
 * Blockchain chain identifier.
 * Mirrors csv_adapter_core::protocol_version::Chain
 */
export type Chain = 'bitcoin' | 'ethereum' | 'sui' | 'aptos' | 'solana';

/**
 * Signature scheme used for ownership proofs.
 * Mirrors csv_adapter_core::signature::SignatureScheme
 */
export enum SignatureScheme {
  /** ECDSA over secp256k1 (Bitcoin, Ethereum) */
  Secp256k1 = 'Secp256k1',
  /** Ed25519 (Sui, Aptos, Solana) */
  Ed25519 = 'Ed25519',
}

/**
 * Protocol version.
 * Mirrors csv_adapter_core::protocol_version::ProtocolVersion
 */
export interface ProtocolVersion {
  major: number;
  minor: number;
  patch: number;
}

/**
 * Error codes for CSV operations.
 * Mirrors csv_adapter_core::protocol_version::ErrorCode
 */
export enum ErrorCode {
  ProtocolVersionMismatch = 'protocol_version_mismatch',
  InvalidRightId = 'invalid_right_id',
  RightAlreadySpent = 'right_already_spent',
  InvalidSealRef = 'invalid_seal_ref',
  InvalidCommitment = 'invalid_commitment',
  ChainNotSupported = 'chain_not_supported',
  AdapterNotInitialized = 'adapter_not_initialized',
  UnsupportedOperation = 'unsupported_operation',
  RpcRequestFailed = 'rpc_request_failed',
  NetworkTimeout = 'network_timeout',
  RateLimitExceeded = 'rate_limit_exceeded',
  InvalidSignature = 'invalid_signature',
  InvalidProof = 'invalid_proof',
  InsufficientConfirmations = 'insufficient_confirmations',
  StorageError = 'storage_error',
  StateCorruption = 'state_corruption',
  ConcurrentModification = 'concurrent_modification',
}

/**
 * Transfer status for cross-chain transfers.
 * Mirrors csv_adapter_core::protocol_version::TransferStatus
 */
export type TransferStatus =
  | { kind: 'initiated' }
  | {
      kind: 'locking';
      currentConfirmations: number;
      requiredConfirmations: number;
    }
  | {
      kind: 'generatingProof';
      progressPercent: number;
    }
  | { kind: 'submittingProof' }
  | { kind: 'verifying' }
  | { kind: 'minting' }
  | { kind: 'completed' }
  | {
      kind: 'failed';
      errorCode: string;
      retryable: boolean;
    };

/**
 * Sync status for chain synchronization.
 * Mirrors csv_adapter_core::protocol_version::SyncStatus
 */
export type SyncStatus =
  | { kind: 'notStarted' }
  | { kind: 'syncing'; current: number; target: number }
  | { kind: 'synced'; latest: number }
  | { kind: 'error'; errorCode: string };

/**
 * Protocol capabilities.
 * Mirrors csv_adapter_core::protocol_version::Capabilities
 */
export interface Capabilities {
  advancedCommitments: boolean;
  mpcProofs: boolean;
  vmTransitions: boolean;
  rgbCompat: boolean;
  tapretVerify: boolean;
  crossChainTransfers: boolean;
}

/**
 * Convert a hex string to a Uint8Array.
 */
export function hexToBytes(hex: string): Uint8Array {
  const clean = hex.startsWith('0x') ? hex.slice(2) : hex;
  if (clean.length % 2 !== 0) {
    throw new Error(`Invalid hex string: ${hex}`);
  }
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

/**
 * Convert a Uint8Array to a hex string.
 */
export function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, '0'))
    .join('');
}

/**
 * Convert a hex string to a number (u64 represented as number).
 * Note: JavaScript numbers are safe up to 2^53 - 1.
 * For values beyond that, use BigInt.
 */
export function hexToNumber(hex: string): number {
  return parseInt(hex.startsWith('0x') ? hex.slice(2) : hex, 16);
}

/**
 * Convert a number to a hex string.
 */
export function numberToHex(n: number, padTo?: number): string {
  let hex = n.toString(16);
  if (padTo && hex.length < padTo) {
    hex = hex.padStart(padTo, '0');
  }
  return hex;
}

/**
 * Parse a Chain from a string.
 */
export function parseChain(value: string): Chain {
  const chains: Chain[] = ['bitcoin', 'ethereum', 'sui', 'aptos', 'solana'];
  const lower = value.toLowerCase();
  if (!chains.includes(lower as Chain)) {
    throw new Error(`Invalid chain: ${value}. Must be one of: ${chains.join(', ')}`);
  }
  return lower as Chain;
}

/**
 * Convert a Chain to a display string.
 */
export function chainToString(chain: Chain): string {
  return chain.charAt(0).toUpperCase() + chain.slice(1);
}
