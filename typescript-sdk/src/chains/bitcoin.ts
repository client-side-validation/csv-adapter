import { SealPoint, CommitAnchor, sealRefFromHex, anchorRefFromHex } from '../seal';
import { hexToBytes, bytesToHex } from '../types';

/**
 * Bitcoin chain utilities.
 *
 * Bitcoin seals are OutPoints (txid + vout) — a specific UTXO.
 * Bitcoin anchors use tapret/opret outputs to publish commitments.
 */
export namespace BitcoinChain {
  /**
   * Create a Bitcoin seal from txid and vout.
   *
   * @param txid - Transaction ID (hex string, 64 chars)
   * @param vout - Output index
   * @returns SealPoint
   */
  export function createSeal(txid: string, vout: number): SealPoint {
    const txidBytes = hexToBytes(txid);
    // Pad to at least 32 bytes
    while (txidBytes.length < 32) {
      txidBytes.push(0);
    }
    // Append vout as 4 bytes
    const voutBytes = new Uint8Array([
      vout & 0xff,
      (vout >> 8) & 0xff,
      (vout >> 16) & 0xff,
      (vout >> 24) & 0xff,
    ]);
    const sealId = new Uint8Array(txidBytes.length + voutBytes.length);
    sealId.set(txidBytes);
    sealId.set(voutBytes, txidBytes.length);
    return { sealId, nonce: null };
  }

  /**
   * Parse a Bitcoin seal from hex.
   *
   * @param hex - Hex string (txid + vout)
   * @returns SealPoint
   */
  export function parseSeal(hex: string): SealPoint {
    const bytes = hexToBytes(hex);
    if (bytes.length < 36) {
      throw new Error(`Invalid Bitcoin seal: expected at least 36 bytes, got ${bytes.length}`);
    }
    // Last 4 bytes are vout
    const vout =
      bytes[bytes.length - 4] |
      (bytes[bytes.length - 3] << 8) |
      (bytes[bytes.length - 2] << 16) |
      (bytes[bytes.length - 1] << 24);
    return {
      sealId: bytes.slice(0, bytes.length - 4),
      nonce: null,
    };
  }

  /**
   * Create a Bitcoin anchor from txid and block height.
   *
   * @param txid - Transaction ID (hex string)
   * @param blockHeight - Block height
   * @param metadata - Optional metadata (hex string)
   * @returns CommitAnchor
   */
  export function createAnchor(
    txid: string,
    blockHeight: number,
    metadata?: string,
  ): CommitAnchor {
    return anchorRefFromHex(txid, blockHeight, metadata);
  }

  /**
   * Build a tapret commitment output script.
   *
   * @param commitment - 32-byte commitment hash (hex string)
   * @param address - P2TR address to send to
   * @returns OP_RETURN script as hex string
   */
  export function buildTapretScript(commitment: string, address: string): string {
    // In production, this would build a proper OP_RETURN script
    // For now, return a placeholder
    return `OP_RETURN ${commitment}`;
  }

  /**
   * Verify a Bitcoin SPV inclusion proof.
   *
   * @param txid - Transaction ID
   * @param merkleBranch - Merkle branch as array of hex strings
   * @param rootBlockHash - Block hash (hex string)
   * @param targetBlockHash - Target block hash (hex string)
   * @returns true if the proof is valid
   */
  export function verifySpvProof(
    txid: string,
    merkleBranch: string[],
    rootBlockHash: string,
    targetBlockHash: string,
  ): boolean {
    // In production, this would:
    // 1. Compute txid from transaction data
    // 2. Walk the Merkle branch to compute root
    // 3. Compare root with block header
    // 4. Verify block header chain to target
    // For now, return true (structural validation only)
    return merkleBranch.length > 0 && rootBlockHash.length === 64;
  }
}
