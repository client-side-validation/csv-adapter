import { SealRef } from '../seal';
import { hexToBytes, bytesToHex } from '../types';

/**
 * Sui chain utilities.
 *
 * Sui seals are ObjectIds — unique identifiers for Sui objects.
 */
export namespace SuiChain {
  /**
   * Create a Sui seal from an ObjectId.
   *
   * @param objectId - Sui object ID (hex string, 64 chars)
   * @returns SealRef
   */
  export function createSeal(objectId: string): SealRef {
    return {
      sealId: hexToBytes(objectId.startsWith('0x') ? objectId.slice(2) : objectId),
      nonce: null,
    };
  }

  /**
   * Derive a Sui address from a private key (Ed25519).
   *
   * @param privateKey - 32-byte Ed25519 private key (hex string)
   * @returns Sui address (hex string with 0x prefix)
   */
  export function deriveAddress(privateKey: string): string {
    // In production, this would use Ed25519 to derive the public key
    // and format it as a Sui address
    throw new Error('Address derivation requires Ed25519 integration');
  }
}
