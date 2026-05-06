import { SealPoint, CommitAnchor } from './seal';

/**
 * Inclusion proof — proves a commitment was included in a block.
 * Mirrors csv_adapter_core::proof::InclusionProof
 */
export interface InclusionProof {
  /** Merkle proof or equivalent (max 64KB) */
  proofBytes: Uint8Array;
  /** Block hash containing the commitment — 32 bytes hex */
  blockHash: string;
  /** Position in block (for verification) */
  position: number;
}

/**
 * Finality proof — proves a commitment cannot be reverted.
 * Mirrors csv_adapter_core::proof::FinalityProof
 */
export interface FinalityProof {
  /** Finality checkpoint or depth (max 4KB) */
  finalityData: Uint8Array;
  /** Number of confirmations */
  confirmations: number;
  /** Whether finality is deterministic */
  isDeterministic: boolean;
}

/**
 * DAG node in a state transition graph.
 * Mirrors csv_adapter_core::dag::DAGNode
 */
export interface DAGNode {
  /** Node identifier — 32 bytes hex */
  nodeId: string;
  /** Bytecode */
  bytecode: Uint8Array;
  /** Authorizing signatures */
  signatures: Uint8Array[];
  /** Witness data */
  witnesses: Uint8Array[];
  /** Parent node IDs */
  parents: string[];
}

/**
 * DAG segment — a connected subgraph of state transitions.
 * Mirrors csv_adapter_core::dag::DAGSegment
 */
export interface DAGSegment {
  /** Nodes in the segment */
  nodes: DAGNode[];
  /** Root commitment — 32 bytes hex */
  rootCommitment: string;
}

/**
 * ProofBundle — the complete verification artifact.
 * Mirrors csv_adapter_core::proof::ProofBundle
 *
 * A proof bundle is a first-class object that can be:
 * - Exported as a file
 * - Shared peer-to-peer
 * - Verified offline by any counterparty
 * - Used to mint a Sanad on a destination chain
 *
 * This is the core CSV competitive advantage over bridges:
 * Traditional bridges give you a receipt; CSV gives you a cryptographic proof.
 */
export interface ProofBundle {
  /** State transition DAG segment */
  transitionDag: DAGSegment;
  /** Authorizing signatures (total max 1MB) */
  signatures: Uint8Array[];
  /** Seal reference that was consumed */
  sealRef: SealPoint;
  /** Anchor reference (on-chain location) */
  anchorRef: CommitAnchor;
  /** Inclusion proof */
  inclusionProof: InclusionProof;
  /** Finality proof */
  finalityProof: FinalityProof;
}

/**
 * Create an InclusionProof from hex strings.
 */
export function inclusionProofFromHex(
  proofBytes: string,
  blockHash: string,
  position: number,
): InclusionProof {
  return {
    proofBytes: hexToBytes(proofBytes),
    blockHash,
    position,
  };
}

/**
 * Create a FinalityProof from hex strings.
 */
export function finalityProofFromHex(
  finalityData: string,
  confirmations: number,
  isDeterministic: boolean,
): FinalityProof {
  return {
    finalityData: hexToBytes(finalityData),
    confirmations,
    isDeterministic,
  };
}

/**
 * Serialize a ProofBundle to JSON-compatible format.
 */
export function proofBundleToJson(bundle: ProofBundle): {
  transitionDag: {
    nodes: {
      nodeId: string;
      bytecode: string;
      signatures: string[];
      witnesses: string[];
      parents: string[];
    }[];
    rootCommitment: string;
  };
  signatures: string[];
  sealRef: { sealId: string; nonce: number | null };
  anchorRef: { anchorId: string; blockHeight: number; metadata: string };
  inclusionProof: {
    proofBytes: string;
    blockHash: string;
    position: number;
  };
  finalityProof: {
    finalityData: string;
    confirmations: number;
    isDeterministic: boolean;
  };
} {
  return {
    transitionDag: {
      nodes: bundle.transitionDag.nodes.map((n) => ({
        nodeId: n.nodeId,
        bytecode: bytesToHex(n.bytecode),
        signatures: n.signatures.map(bytesToHex),
        witnesses: n.witnesses.map(bytesToHex),
        parents: n.parents,
      })),
      rootCommitment: bundle.transitionDag.rootCommitment,
    },
    signatures: bundle.signatures.map(bytesToHex),
    sealRef: {
      sealId: bytesToHex(bundle.sealRef.sealId),
      nonce: bundle.sealRef.nonce,
    },
    anchorRef: {
      anchorId: bytesToHex(bundle.anchorRef.anchorId),
      blockHeight: bundle.anchorRef.blockHeight,
      metadata: bytesToHex(bundle.anchorRef.metadata),
    },
    inclusionProof: {
      proofBytes: bytesToHex(bundle.inclusionProof.proofBytes),
      blockHash: bundle.inclusionProof.blockHash,
      position: bundle.inclusionProof.position,
    },
    finalityProof: {
      finalityData: bytesToHex(bundle.finalityProof.finalityData),
      confirmations: bundle.finalityProof.confirmations,
      isDeterministic: bundle.finalityProof.isDeterministic,
    },
  };
}

/**
 * Deserialize a ProofBundle from JSON.
 */
export function proofBundleFromJson(json: {
  transitionDag: {
    nodes: {
      nodeId: string;
      bytecode: string;
      signatures: string[];
      witnesses: string[];
      parents: string[];
    }[];
    rootCommitment: string;
  };
  signatures: string[];
  sealRef: { sealId: string; nonce: number | null };
  anchorRef: { anchorId: string; blockHeight: number; metadata: string };
  inclusionProof: {
    proofBytes: string;
    blockHash: string;
    position: number;
  };
  finalityProof: {
    finalityData: string;
    confirmations: number;
    isDeterministic: boolean;
  };
}): ProofBundle {
  return {
    transitionDag: {
      nodes: json.transitionDag.nodes.map((n) => ({
        nodeId: n.nodeId,
        bytecode: hexToBytes(n.bytecode),
        signatures: n.signatures.map(hexToBytes),
        witnesses: n.witnesses.map(hexToBytes),
        parents: n.parents,
      })),
      rootCommitment: json.transitionDag.rootCommitment,
    },
    signatures: json.signatures.map(hexToBytes),
    sealRef: {
      sealId: hexToBytes(json.sealRef.sealId),
      nonce: json.sealRef.nonce,
    },
    anchorRef: {
      anchorId: hexToBytes(json.anchorRef.anchorId),
      blockHeight: json.anchorRef.blockHeight,
      metadata: hexToBytes(json.anchorRef.metadata),
    },
    inclusionProof: {
      proofBytes: hexToBytes(json.inclusionProof.proofBytes),
      blockHash: json.inclusionProof.blockHash,
      position: json.inclusionProof.position,
    },
    finalityProof: {
      finalityData: hexToBytes(json.finalityProof.finalityData),
      confirmations: json.finalityProof.confirmations,
      isDeterministic: json.finalityProof.isDeterministic,
    },
  };
}
