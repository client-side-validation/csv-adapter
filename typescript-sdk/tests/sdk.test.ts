import {
  hexToBytes,
  bytesToHex,
  parseChain,
  chainToString,
  numberToHex,
} from '../src/types';
import { sealRefFromHex, sealRefToJson, sealRefFromJson } from '../src/seal';
import { rightFromHex, rightToJson, rightFromJson } from '../src/right';
import { proofBundleFromJson, proofBundleToJson } from '../src/proof';
import { verifyProofBundle, verifyProofBundleFromJson } from '../src/verify';
import { BitcoinChain } from '../src/chains/bitcoin';

describe('Type utilities', () => {
  test('hexToBytes converts hex to bytes', () => {
    const bytes = hexToBytes('48656c6c6f');
    expect(bytes).toEqual(new Uint8Array([72, 101, 108, 108, 111]));
  });

  test('bytesToHex converts bytes to hex', () => {
    const hex = bytesToHex(new Uint8Array([72, 101, 108, 108, 111]));
    expect(hex).toBe('48656c6c6f');
  });

  test('parseChain parses valid chain names', () => {
    expect(parseChain('bitcoin')).toBe('bitcoin');
    expect(parseChain('ethereum')).toBe('ethereum');
    expect(parseChain('SUI')).toBe('sui');
    expect(parseChain('Aptos')).toBe('aptos');
    expect(parseChain('solana')).toBe('solana');
  });

  test('parseChain throws on invalid chain', () => {
    expect(() => parseChain('cosmos')).toThrow('Invalid chain');
  });

  test('chainToString capitalizes chain name', () => {
    expect(chainToString('bitcoin')).toBe('Bitcoin');
    expect(chainToString('ethereum')).toBe('Ethereum');
  });

  test('numberToHex converts number to hex', () => {
    expect(numberToHex(255)).toBe('ff');
    expect(numberToHex(255, 4)).toBe('00ff');
  });
});

describe('SealRef', () => {
  test('sealRefFromHex creates seal from hex', () => {
    const seal = sealRefFromHex('00112233445566778899aabbccddeeff', 42);
    expect(seal.sealId).toEqual(hexToBytes('00112233445566778899aabbccddeeff'));
    expect(seal.nonce).toBe(42);
  });

  test('sealRefToJson serializes seal', () => {
    const seal = sealRefFromHex('aabbccdd', 100);
    const json = sealRefToJson(seal);
    expect(json.sealId).toBe('aabbccdd');
    expect(json.nonce).toBe(100);
  });

  test('sealRefFromJson deserializes seal', () => {
    const json = { sealId: 'aabbccdd', nonce: 100 };
    const seal = sealRefFromJson(json);
    expect(seal.sealId).toEqual(hexToBytes('aabbccdd'));
    expect(seal.nonce).toBe(100);
  });
});

describe('Right', () => {
  test('rightFromHex creates right from hex', () => {
    const right = rightFromHex(
      'aa'.repeat(32),
      'bb'.repeat(32),
      {
        proof: hexToBytes('cc'.repeat(64)),
        owner: hexToBytes('dd'.repeat(42)),
        scheme: 'Secp256k1',
      },
      'ee'.repeat(32),
    );
    expect(right.id).toBe('aa'.repeat(32));
    expect(right.commitment).toBe('bb'.repeat(32));
    expect(right.nullifier).toBeNull();
  });

  test('rightToJson serializes right', () => {
    const right = rightFromHex(
      'aa'.repeat(32),
      'bb'.repeat(32),
      {
        proof: hexToBytes('cc'.repeat(64)),
        owner: hexToBytes('dd'.repeat(42)),
        scheme: 'Ed25519',
      },
      'ee'.repeat(32),
    );
    const json = rightToJson(right);
    expect(json.id).toBe('aa'.repeat(32));
    expect(json.owner.scheme).toBe('Ed25519');
  });
});

describe('ProofBundle', () => {
  test('proofBundleToJson serializes bundle', () => {
    const bundle: any = {
      transitionDag: {
        nodes: [
          {
            nodeId: 'aa'.repeat(32),
            bytecode: 'bb'.repeat(10),
            signatures: ['cc'.repeat(64)],
            witnesses: [],
            parents: [],
          },
        ],
        rootCommitment: 'dd'.repeat(32),
      },
      signatures: ['ee'.repeat(64)],
      sealRef: { sealId: 'ff'.repeat(32), nonce: null },
      anchorRef: {
        anchorId: '00'.repeat(32),
        blockHeight: 800000,
        metadata: '11'.repeat(10),
      },
      inclusionProof: {
        proofBytes: '22'.repeat(100),
        blockHash: '33'.repeat(32),
        position: 42,
      },
      finalityProof: {
        finalityData: '44'.repeat(10),
        confirmations: 6,
        isDeterministic: true,
      },
    };
    const json = proofBundleToJson(bundle);
    expect(json.sealRef.sealId).toBe('ff'.repeat(32));
    expect(json.anchorRef.blockHeight).toBe(800000);
  });
});

describe('Verification', () => {
  test('verifyProofBundle validates structure', () => {
    const bundle: any = {
      transitionDag: {
        nodes: [
          {
            nodeId: 'aa'.repeat(32),
            bytecode: 'bb'.repeat(10),
            signatures: ['cc'.repeat(64)],
            witnesses: [],
            parents: [],
          },
        ],
        rootCommitment: 'dd'.repeat(32),
      },
      signatures: ['ee'.repeat(64)],
      sealRef: { sealId: 'ff'.repeat(32), nonce: null },
      anchorRef: {
        anchorId: '00'.repeat(32),
        blockHeight: 800000,
        metadata: '11'.repeat(10),
      },
      inclusionProof: {
        proofBytes: '22'.repeat(100),
        blockHash: '33'.repeat(32),
        position: 42,
      },
      finalityProof: {
        finalityData: '44'.repeat(10),
        confirmations: 6,
        isDeterministic: true,
      },
    };
    const result = verifyProofBundle(bundle);
    expect(result.valid).toBe(true);
    expect(result.error).toBeNull();
    expect(result.steps.length).toBe(7);
  });

  test('verifyProofBundle rejects invalid bundle', () => {
    const bundle: any = {
      transitionDag: { nodes: [], rootCommitment: '' },
      signatures: [],
      sealRef: { sealId: new Uint8Array(), nonce: null },
      anchorRef: { anchorId: new Uint8Array(), blockHeight: 0, metadata: new Uint8Array() },
      inclusionProof: { proofBytes: new Uint8Array(), blockHash: '', position: 0 },
      finalityProof: { finalityData: new Uint8Array(), confirmations: 0, isDeterministic: false },
    };
    const result = verifyProofBundle(bundle);
    expect(result.valid).toBe(false);
  });

  test('verifyProofBundleFromJson parses and verifies', () => {
    const json = JSON.stringify({
      transitionDag: {
        nodes: [
          {
            nodeId: 'aa'.repeat(32),
            bytecode: 'bb'.repeat(10),
            signatures: ['cc'.repeat(64)],
            witnesses: [],
            parents: [],
          },
        ],
        rootCommitment: 'dd'.repeat(32),
      },
      signatures: ['ee'.repeat(64)],
      sealRef: { sealId: 'ff'.repeat(32), nonce: null },
      anchorRef: {
        anchorId: '00'.repeat(32),
        blockHeight: 800000,
        metadata: '11'.repeat(10),
      },
      inclusionProof: {
        proofBytes: '22'.repeat(100),
        blockHash: '33'.repeat(32),
        position: 42,
      },
      finalityProof: {
        finalityData: '44'.repeat(10),
        confirmations: 6,
        isDeterministic: true,
      },
    });
    const result = verifyProofBundleFromJson(json);
    expect(result.valid).toBe(true);
  });

  test('verifyProofBundleFromJson handles invalid JSON', () => {
    const result = verifyProofBundleFromJson('not valid json');
    expect(result.valid).toBe(false);
    expect(result.error).toContain('Invalid JSON');
  });
});

describe('BitcoinChain', () => {
  test('createSeal creates seal from txid and vout', () => {
    const seal = BitcoinChain.createSeal('aaaa'.repeat(8), 0);
    expect(seal.sealId.length).toBeGreaterThan(0);
    expect(seal.nonce).toBeNull();
  });

  test('verifySpvProof validates structure', () => {
    const valid = BitcoinChain.verifySpvProof(
      'aaaa'.repeat(8),
      ['bbbb'.repeat(64)],
      'cccc'.repeat(64),
      'dddd'.repeat(64),
    );
    expect(valid).toBe(true);
  });
});
