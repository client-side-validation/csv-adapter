import { SignatureScheme } from './types';

/**
 * Ownership proof for a Right.
 * Mirrors csv_adapter_core::right::OwnershipProof
 */
export interface OwnershipProof {
  /** Proof bytes (chain-specific format) */
  proof: Uint8Array;
  /** Owner identifier (address, pubkey, etc.) */
  owner: Uint8Array;
  /** Signature scheme used */
  scheme: SignatureScheme | null;
}

/**
 * A verifiable, single-use digital right.
 * Mirrors csv_adapter_core::right::Right
 *
 * Rights exist in client state, not on any chain.
 * The chain only records commitments and enforces single-use via seals.
 */
export interface Right {
  /** Unique ID: H(commitment || salt) — 32 bytes hex */
  id: string;
  /** Encodes state + rules — 32 bytes hex */
  commitment: string;
  /** Proof of ownership */
  owner: OwnershipProof;
  /** Salt used to compute the Right ID */
  salt: Uint8Array;
  /** One-time consumption marker (L3+) — 32 bytes hex or null */
  nullifier: string | null;
  /** Off-chain state commitment root — 32 bytes hex or null */
  stateRoot: string | null;
  /** Optional execution proof (ZK, fraud proof, etc.) */
  executionProof: Uint8Array | null;
}

/**
 * Create an OwnershipProof from hex strings.
 */
export function ownershipProofFromHex(
  proof: string,
  owner: string,
  scheme?: SignatureScheme,
): OwnershipProof {
  return {
    proof: hexToBytes(proof),
    owner: hexToBytes(owner),
    scheme: scheme ?? null,
  };
}

/**
 * Serialize an OwnershipProof to JSON-compatible format.
 */
export function ownershipProofToJson(op: OwnershipProof): {
  proof: string;
  owner: string;
  scheme: SignatureScheme | null;
} {
  return {
    proof: bytesToHex(op.proof),
    owner: bytesToHex(op.owner),
    scheme: op.scheme,
  };
}

/**
 * Create a Right from hex strings.
 */
export function rightFromHex(
  id: string,
  commitment: string,
  owner: OwnershipProof,
  salt: string,
  nullifier?: string,
  stateRoot?: string,
  executionProof?: string,
): Right {
  return {
    id,
    commitment,
    owner,
    salt: hexToBytes(salt),
    nullifier: nullifier ?? null,
    stateRoot: stateRoot ?? null,
    executionProof: executionProof ? hexToBytes(executionProof) : null,
  };
}

/**
 * Serialize a Right to JSON-compatible format.
 */
export function rightToJson(right: Right): {
  id: string;
  commitment: string;
  owner: { proof: string; owner: string; scheme: SignatureScheme | null };
  salt: string;
  nullifier: string | null;
  stateRoot: string | null;
  executionProof: string | null;
} {
  return {
    id: right.id,
    commitment: right.commitment,
    owner: ownershipProofToJson(right.owner),
    salt: bytesToHex(right.salt),
    nullifier: right.nullifier,
    stateRoot: right.stateRoot,
    executionProof: right.executionProof ? bytesToHex(right.executionProof) : null,
  };
}

/**
 * Deserialize a Right from JSON.
 */
export function rightFromJson(json: {
  id: string;
  commitment: string;
  owner: { proof: string; owner: string; scheme: SignatureScheme | null };
  salt: string;
  nullifier: string | null;
  stateRoot: string | null;
  executionProof: string | null;
}): Right {
  return {
    id: json.id,
    commitment: json.commitment,
    owner: {
      proof: hexToBytes(json.owner.proof),
      owner: hexToBytes(json.owner.owner),
      scheme: json.owner.scheme,
    },
    salt: hexToBytes(json.salt),
    nullifier: json.nullifier,
    stateRoot: json.stateRoot,
    executionProof: json.executionProof ? hexToBytes(json.executionProof) : null,
  };
}
