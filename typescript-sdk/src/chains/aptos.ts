import { SealRef } from '../seal';
import { hexToBytes } from '../types';

/**
 * Aptos chain utilities.
 *
 * Aptos seals use resource addresses and keys.
 */
export namespace AptosChain {
  /**
   * Create an Aptos seal from a resource address.
   *
   * @param address - Aptos resource address (hex string with 0x prefix)
   * @returns SealRef
   */
  export function createSeal(address: string): SealRef {
    const addr = address.startsWith('0x') ? address.slice(2) : address;
    return {
      sealId: hexToBytes(addr),
      nonce: null,
    };
  }

  /**
   * Derive an Aptos address from a private key (Ed25519).
   */
  export function deriveAddress(privateKey: string): string {
    throw new Error('Address derivation requires Ed25519 integration');
  }
}
