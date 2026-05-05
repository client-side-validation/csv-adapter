/**
 * SealRef — single-use seal reference.
 * Mirrors csv_adapter_core::seal::SealRef
 *
 * A seal is the on-chain mechanism that enforces a Right's single-use property.
 * Each chain has its own seal format:
 * - Bitcoin: OutPoint (txid + vout)
 * - Ethereum: (contract_address, storage_slot) or nullifier hash
 * - Sui: ObjectId
 * - Aptos: (resource_address, key)
 * - Solana: Pubkey of the seal account
 */
export interface SealRef {
  /** Chain-specific seal identifier (max 1024 bytes) */
  sealId: Uint8Array;
  /** Optional nonce for replay resistance */
  nonce: number | null;
}

/**
 * AnchorRef — on-chain anchor reference.
 * Mirrors csv_adapter_core::seal::AnchorRef
 *
 * Points to where a commitment was published on-chain.
 */
export interface AnchorRef {
  /** Chain-specific anchor identifier (max 1024 bytes) */
  anchorId: Uint8Array;
  /** Block height or equivalent ordering */
  blockHeight: number;
  /** Optional chain-specific metadata (max 4096 bytes) */
  metadata: Uint8Array;
}

/**
 * Create a SealRef from hex strings.
 */
export function sealRefFromHex(sealId: string, nonce?: number): SealRef {
  return {
    sealId: hexToBytes(sealId),
    nonce: nonce ?? null,
  };
}

/**
 * Serialize a SealRef to JSON-compatible format.
 */
export function sealRefToJson(seal: SealRef): { sealId: string; nonce: number | null } {
  return {
    sealId: bytesToHex(seal.sealId),
    nonce: seal.nonce,
  };
}

/**
 * Deserialize a SealRef from JSON.
 */
export function sealRefFromJson(json: { sealId: string; nonce: number | null }): SealRef {
  return sealRefFromHex(json.sealId, json.nonce ?? undefined);
}

/**
 * Create an AnchorRef from hex strings.
 */
export function anchorRefFromHex(
  anchorId: string,
  blockHeight: number,
  metadata?: string,
): AnchorRef {
  return {
    anchorId: hexToBytes(anchorId),
    blockHeight,
    metadata: metadata ? hexToBytes(metadata) : new Uint8Array(),
  };
}

/**
 * Serialize an AnchorRef to JSON-compatible format.
 */
export function anchorRefToJson(anchor: AnchorRef): {
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
 * Deserialize an AnchorRef from JSON.
 */
export function anchorRefFromJson(json: {
  anchorId: string;
  blockHeight: number;
  metadata: string;
}): AnchorRef {
  return anchorRefFromHex(json.anchorId, json.blockHeight, json.metadata);
}
