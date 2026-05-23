import { SealPoint } from '../seal';
import { hexToBytes } from '../types';

/**
 * Solana chain utilities.
 *
 * Solana seals use PDAs (Program Derived Addresses) of the CSV program.
 */
export namespace SolanaChain {
  /**
   * Create a Solana seal from a Pubkey.
   *
   * @param pubkey - Solana public key (base58 or hex string)
   * @returns SealPoint
   */
  export function createSeal(pubkey: string): SealPoint {
    // Try to detect if it's base58 or hex
    const isBase58 = /^[1-9A-HJ-NP-Za-km-z]+$/.test(pubkey);
    return {
      sealId: isBase58
        ? base58ToBytes(pubkey)
        : hexToBytes(pubkey.startsWith('0x') ? pubkey.slice(2) : pubkey),
      nonce: null,
    };
  }

  /**
   * Derive a Solana address from a private key (Ed25519).
   */
  export function deriveAddress(privateKey: string): string {
    throw new Error('Address derivation requires Ed25519 integration');
  }

  /**
   * Base58 decode (simplified).
   */
  function base58ToBytes(input: string): Uint8Array {
    const alphabet = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
    const base = BigInt(58);
    let num = BigInt(0);
    for (const char of input) {
      const idx = alphabet.indexOf(char);
      if (idx === -1) throw new Error(`Invalid base58 character: ${char}`);
      num = num * base + BigInt(idx);
    }
    const bytes = [];
    while (num > 0n) {
      bytes.unshift(Number(num % 256n));
      num = num / 256n;
    }
    // Leading zeros
    for (const char of input) {
      if (char === '1') bytes.unshift(0);
      else break;
    }
    return new Uint8Array(bytes);
  }
}
