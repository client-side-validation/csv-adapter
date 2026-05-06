/**
 * SealPoint — single-use seal reference.
 * Mirrors csv_adapter_core::seal::SealPoint
 *
 * A seal is the on-chain mechanism that enforces a Sanad's single-use property.
 * Each chain has its own seal format:
 * - Bitcoin: OutPoint (txid + vout)
 * - Ethereum: (contract_address, storage_slot) or nullifier hash
 * - Sui: ObjectId
 * - Aptos: (resource_address, key)
 * - Solana: Pubkey of the seal account
 */
export interface SealPoint {
  /** Chain-specific seal identifier (max 1024 bytes) */
  sealId: Uint8Array;
  /** Optional nonce for replay resistance */
  nonce: number | null;
}

/**
 * CommitAnchor — on-chain anchor reference.
 * Mirrors csv_adapter_core::seal::CommitAnchor
 *
 * Points to where a commitment was published on-chain.
 */
export interface CommitAnchor {
  /** Chain-specific anchor identifier (max 1024 bytes) */
  anchorId: Uint8Array;
  /** Block height or equivalent ordering */
  blockHeight: number;
  /** Optional chain-specific metadata (max 4096 bytes) */
  metadata: Uint8Array;
}

/**
 * Create a SealPoint from hex strings.
 */
export function sealRefFromHex(sealId: string, nonce?: number): SealPoint {
  return {
    sealId: hexToBytes(sealId),
    nonce: nonce ?? null,
  };
}

/**
 * Serialize a SealPoint to JSON-compatible format.
 */
export function sealRefToJson(seal: SealPoint): { sealId: string; nonce: number | null } {
  return {
    sealId: bytesToHex(seal.sealId),
    nonce: seal.nonce,
  };
}

/**
 * Deserialize a SealPoint from JSON.
 */
export function sealRefFromJson(json: { sealId: string; nonce: number | null }): SealPoint {
  return sealRefFromHex(json.sealId, json.nonce ?? undefined);
}

/**
 * Create an CommitAnchor from hex strings.
 */
export function anchorRefFromHex(
  anchorId: string,
  blockHeight: number,
  metadata?: string,
): CommitAnchor {
  return {
    anchorId: hexToBytes(anchorId),
    blockHeight,
    metadata: metadata ? hexToBytes(metadata) : new Uint8Array(),
  };
}

/**
 * Serialize an CommitAnchor to JSON-compatible format.
 */
export function anchorRefToJson(anchor: CommitAnchor): {
  anchorId: string;
  blockHeight: number;
  metadata: string;
} {
  return {
    anchorId: bytesToHex(anchor.anchorId),
    blockHeight: anchor.blockHeight,
    metadata: bytesToHex(anchor.metadata),
  };
}

/**
 * Deserialize an CommitAnchor from JSON.
 */
export function anchorRefFromJson(json: {
  anchorId: string;
  blockHeight: number;
  metadata: string;
}): CommitAnchor {
  return anchorRefFromHex(json.anchorId, json.blockHeight, json.metadata);
}
