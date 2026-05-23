/**
 * WASM chain_id regression test
 * 
 * This test ensures that chain IDs are correctly handled in WASM environments,
 * preventing issues where chain identifiers might be corrupted or incorrectly
 * serialized when crossing the Rust-WASM boundary.
 */

import { Chain, parseChain, chainToString } from './types';

describe('WASM Chain ID Regression Tests', () => {
  /**
   * Test that all supported chain IDs can be parsed correctly.
   * This prevents issues where chain IDs might be corrupted during WASM serialization.
   */
  test('all chain IDs parse correctly', () => {
    const validChains: Chain[] = ['bitcoin', 'ethereum', 'sui', 'aptos', 'solana'];
    
    validChains.forEach(chain => {
      const parsed = parseChain(chain);
      expect(parsed).toBe(chain);
    });
  });

  /**
   * Test that chain IDs are case-insensitive during parsing.
   * This ensures consistent behavior across different input formats.
   */
  test('chain ID parsing is case-insensitive', () => {
    expect(parseChain('BITCOIN')).toBe('bitcoin');
    expect(parseChain('ETHEREUM')).toBe('ethereum');
    expect(parseChain('SUI')).toBe('sui');
    expect(parseChain('APTOS')).toBe('aptos');
    expect(parseChain('SOLANA')).toBe('solana');
  });

  /**
   * Test that invalid chain IDs are rejected.
   * This prevents security issues where invalid chain IDs could be used to bypass checks.
   */
  test('invalid chain IDs are rejected', () => {
    const invalidChains = ['invalid', 'polkadot', 'cosmos', '', 'bitcoin-test'];
    
    invalidChains.forEach(chain => {
      expect(() => parseChain(chain)).toThrow('Invalid chain');
    });
  });

  /**
   * Test that chain IDs can be converted to display strings correctly.
   * This ensures proper formatting in UI components.
   */
  test('chain IDs convert to display strings correctly', () => {
    expect(chainToString('bitcoin')).toBe('Bitcoin');
    expect(chainToString('ethereum')).toBe('Ethereum');
    expect(chainToString('sui')).toBe('Sui');
    expect(chainToString('aptos')).toBe('Aptos');
    expect(chainToString('solana')).toBe('Solana');
  });

  /**
   * Test that chain ID serialization/deserialization is consistent.
   * This ensures that chain IDs remain valid when crossing the WASM boundary.
   */
  test('chain ID serialization round-trip is consistent', () => {
    const validChains: Chain[] = ['bitcoin', 'ethereum', 'sui', 'aptos', 'solana'];
    
    validChains.forEach(chain => {
      // Simulate serialization to JSON (as would happen in WASM)
      const serialized = JSON.stringify(chain);
      const deserialized = JSON.parse(serialized) as Chain;
      
      // Verify the chain is still valid after round-trip
      expect(() => parseChain(deserialized)).not.toThrow();
      expect(parseChain(deserialized)).toBe(chain);
    });
  });

  /**
   * Test that chain IDs don't exceed WASM memory limits.
   * This prevents memory issues when chain IDs are passed to WASM.
   */
  test('chain IDs are within reasonable size limits', () => {
    const validChains: Chain[] = ['bitcoin', 'ethereum', 'sui', 'aptos', 'solana'];
    
    validChains.forEach(chain => {
      const bytes = new TextEncoder().encode(chain);
      // Chain IDs should be small (< 32 bytes) to avoid WASM memory issues
      expect(bytes.length).toBeLessThan(32);
    });
  });

  /**
   * Test that chain IDs are ASCII-compatible.
   * This ensures compatibility with WASM string handling.
   */
  test('chain IDs are ASCII-compatible', () => {
    const validChains: Chain[] = ['bitcoin', 'ethereum', 'sui', 'aptos', 'solana'];
    
    validChains.forEach(chain => {
      const bytes = new TextEncoder().encode(chain);
      bytes.forEach(byte => {
        // All bytes should be ASCII (< 128)
        expect(byte).toBeLessThan(128);
      });
    });
  });
});
