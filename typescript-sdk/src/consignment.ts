import { SealPoint, CommitAnchor } from './seal';
import { Sanad, OwnershipProof } from './sanad';

/**
 * Commitment — encodes state transition rules.
 * Mirrors csv_adapter_core::commitment::Commitment
 *
 * Commitments are published on-chain and form a hash-linked chain
 * that provides tamper-evident provenance for Sanads.
 */
export interface Commitment {
  /** Currently always 2 */
  version: number;
  /** 32-byte protocol namespace identifier */
  protocolId: Uint8Array;
  /** Merkle root of MPC tree */
  mpcRoot: string;
  /** Unique contract/sanad identifier — 32 bytes hex */
  contractId: string;
  /** Hash of previous commitment in chain — 32 bytes hex */
  previousCommitment: string;
  /** SHA-256 hash of state transition payload — 32 bytes hex */
  transitionPayloadHash: string;
  /** SHA-256 hash of consumed seal reference — 32 bytes hex */
  sealId: string;
  /** 32-byte chain-specific domain separator */
  domainSeparator: Uint8Array;
}

/**
 * Metadata key-value pair.
 * Mirrors csv_adapter_core::state::Metadata
 */
export interface Metadata {
  key: string;
  value: Uint8Array;
}

/**
 * Global state entry.
 * Mirrors csv_adapter_core::state::GlobalState
 */
export interface GlobalState {
  typeId: number;
  data: Uint8Array;
}

/**
 * Owned state entry.
 * Mirrors csv_adapter_core::state::OwnedState
 */
export interface OwnedState {
  typeId: number;
  seal: SealPoint;
  data: Uint8Array;
}

/**
 * State assignment — links state to a seal.
 * Mirrors csv_adapter_core::state::StateAssignment
 */
export interface StateAssignment {
  typeId: number;
  seal: SealPoint;
  data: Uint8Array;
}

/**
 * State reference.
 * Mirrors csv_adapter_core::state::StateRef
 */
export interface StateRef {
  typeId: number;
  commitment: string;
  outputIndex: number;
}

/**
 * Genesis — the root of a Sanad's provenance chain.
 * Mirrors csv_adapter_core::genesis::Genesis
 */
export interface Genesis {
  /** Contract identifier — 32 bytes hex */
  contractId: string;
  /** Schema identifier — 32 bytes hex */
  schemaId: string;
  /** Global state entries */
  globalState: GlobalState[];
  /** Owned state entries */
  ownedState: OwnedState[];
  /** Metadata */
  metadata: Metadata[];
}

/**
 * State transition — evolves a Sanad's state.
 * Mirrors csv_adapter_core::transition::Transition
 */
export interface Transition {
  /** Transition identifier */
  transitionId: number;
  /** Owned input state references */
  ownedInputs: StateRef[];
  /** Owned output state assignments */
  ownedOutputs: StateAssignment[];
  /** Global state updates */
  globalUpdates: GlobalState[];
  /** Metadata */
  metadata: Metadata[];
  /** Validation script bytecode */
  validationScript: Uint8Array;
  /** Signatures authorizing this transition */
  signatures: Uint8Array[];
}

/**
 * Anchor — on-chain proof of a commitment.
 * Mirrors csv_adapter_core::consignment::Anchor
 */
export interface ConsignmentAnchor {
  /** Anchor reference */
  anchorRef: CommitAnchor;
  /** Commitment hash that was anchored — 32 bytes hex */
  commitment: string;
  /** Inclusion proof bytes */
  inclusionProof: Uint8Array;
  /** Finality proof bytes */
  finalityProof: Uint8Array;
}

/**
 * Seal assignment — links a seal to state.
 * Mirrors csv_adapter_core::consignment::SealAssignment
 */
export interface SealAssignment {
  /** Seal being assigned */
  sealRef: SealPoint;
  /** State being assigned to this seal */
  assignment: StateAssignment;
  /** Metadata for this assignment */
  metadata: Metadata[];
}

/**
 * Consignment — the complete transfer artifact.
 * Mirrors csv_adapter_core::consignment::Consignment
 *
 * A consignment contains everything needed to verify and accept
 * a Sanad from another party. It includes:
 * - The genesis (root of trust)
 * - All state transitions in topological order
 * - Seal assignments
 * - On-chain anchor proofs
 *
 * This is what gets passed between parties during transfers.
 */
export interface Consignment {
  /** Consignment version (always 1) */
  version: number;
  /** Contract genesis */
  genesis: Genesis;
  /** State transitions in topological order */
  transitions: Transition[];
  /** Seal assignments indexed by transition output */
  sealAssignments: SealAssignment[];
  /** On-chain anchor proofs */
  anchors: ConsignmentAnchor[];
  /** Schema ID for validation */
  schemaId: string;
}

/**
 * Create a Genesis from hex strings.
 */
export function genesisFromHex(
  contractId: string,
  schemaId: string,
  globalState?: GlobalState[],
  ownedState?: OwnedState[],
  metadata?: Metadata[],
): Genesis {
  return {
    contractId,
    schemaId,
    globalState: globalState ?? [],
    ownedState: ownedState ?? [],
    metadata: metadata ?? [],
  };
}

/**
 * Create a Consignment from its components.
 */
export function consignmentFromHex(
  genesis: Genesis,
  transitions: Transition[],
  sealAssignments: SealAssignment[],
  anchors: ConsignmentAnchor[],
  schemaId: string,
): Consignment {
  return {
    version: 1,
    genesis,
    transitions,
    sealAssignments,
    anchors,
    schemaId,
  };
}

/**
 * Serialize a Consignment to JSON-compatible format.
 */
export function consignmentToJson(c: Consignment): any {
  return {
    version: c.version,
    genesis: {
      contractId: c.genesis.contractId,
      schemaId: c.genesis.schemaId,
      globalState: c.genesis.globalState.map((gs) => ({
        typeId: gs.typeId,
        data: bytesToHex(gs.data),
      })),
      ownedState: c.genesis.ownedState.map((os) => ({
        typeId: os.typeId,
        seal: {
          sealId: bytesToHex(os.seal.sealId),
          nonce: os.seal.nonce,
        },
        data: bytesToHex(os.data),
      })),
      metadata: c.genesis.metadata.map((m) => ({
        key: m.key,
        value: bytesToHex(m.value),
      })),
    },
    transitions: c.transitions.map((t) => ({
      transitionId: t.transitionId,
      ownedInputs: t.ownedInputs.map((si) => ({
        typeId: si.typeId,
        commitment: si.commitment,
        outputIndex: si.outputIndex,
      })),
      ownedOutputs: t.ownedOutputs.map((sa) => ({
        typeId: sa.typeId,
        seal: {
          sealId: bytesToHex(sa.seal.sealId),
          nonce: sa.seal.nonce,
        },
        data: bytesToHex(sa.data),
      })),
      globalUpdates: t.globalUpdates.map((gs) => ({
        typeId: gs.typeId,
        data: bytesToHex(gs.data),
      })),
      metadata: t.metadata.map((m) => ({
        key: m.key,
        value: bytesToHex(m.value),
      })),
      validationScript: bytesToHex(t.validationScript),
      signatures: t.signatures.map(bytesToHex),
    })),
    sealAssignments: c.sealAssignments.map((sa) => ({
      sealRef: {
        sealId: bytesToHex(sa.sealRef.sealId),
        nonce: sa.sealRef.nonce,
      },
      assignment: {
        typeId: sa.assignment.typeId,
        seal: {
          sealId: bytesToHex(sa.assignment.seal.sealId),
          nonce: sa.assignment.seal.nonce,
        },
        data: bytesToHex(sa.assignment.data),
      },
      metadata: sa.metadata.map((m) => ({
        key: m.key,
        value: bytesToHex(m.value),
      })),
    })),
    anchors: c.anchors.map((a) => ({
      anchorRef: {
        anchorId: bytesToHex(a.anchorRef.anchorId),
        blockHeight: a.anchorRef.blockHeight,
        metadata: bytesToHex(a.anchorRef.metadata),
      },
      commitment: a.commitment,
      inclusionProof: bytesToHex(a.inclusionProof),
      finalityProof: bytesToHex(a.finalityProof),
    })),
    schemaId: c.schemaId,
  };
}
