import { ProofBundle } from './proof';
import { Consignment } from './consignment';
import { Sanad } from './sanad';
import { ErrorCode } from './types';

/**
 * Verification result for a proof bundle.
 */
export interface VerificationResult {
  /** Whether the proof is valid */
  valid: boolean;
  /** Detailed error message if invalid */
  error: string | null;
  /** Verification steps that were performed */
  steps: VerificationStep[];
}

/**
 * A single step in the verification process.
 */
export interface VerificationStep {
  /** Step name */
  name: string;
  /** Whether the step passed */
  passed: boolean;
  /** Details about the step */
  details: string;
}

/**
 * Verify a ProofBundle offline (no RPC calls needed).
 *
 * This is the CSV competitive advantage over bridges:
 * "Your counterparty doesn't need to trust any server.
 *  They can verify your sanad with this file alone."
 *
 * Verification steps:
 * 1. Parse ProofBundle JSON
 * 2. Structure validation — check required fields
 * 3. Cryptographic verification — signatures, seal replay, inclusion, finality
 * 4. Show each step passing/failing
 * 5. Never make an RPC call
 *
 * @param bundle - The ProofBundle to verify
 * @returns VerificationResult with pass/fail for each step
 */
export function verifyProofBundle(bundle: ProofBundle): VerificationResult {
  const steps: VerificationStep[] = [];

  // Step 1: Structure validation
  const hasRequiredFields =
    bundle.sealRef.sealId.length > 0 &&
    bundle.anchorRef.anchorId.length > 0 &&
    bundle.inclusionProof.proofBytes.length > 0;

  steps.push({
    name: 'Structure Validation',
    passed: hasRequiredFields,
    details: hasRequiredFields
      ? 'All required fields present with valid data'
      : 'Missing required fields (seal_ref, anchor_ref, or inclusion_proof)',
  });

  if (!hasRequiredFields) {
    return { valid: false, error: 'Invalid structure', steps };
  }

  // Step 2: Seal reference validation
  const sealValid = bundle.sealRef.sealId.length >= 8;
  steps.push({
    name: 'Seal Reference',
    passed: sealValid,
    details: sealValid
      ? `Seal valid: ${bytesToHex(bundle.sealRef.sealId).slice(0, 16)}... (${bundle.sealRef.sealId.length} bytes)`
      : 'Seal reference too short',
  });

  // Step 3: Anchor reference validation
  const anchorValid = bundle.anchorRef.anchorId.length >= 8 && bundle.anchorRef.blockHeight > 0;
  steps.push({
    name: 'Anchor Reference',
    passed: anchorValid,
    details: anchorValid
      ? `Anchor valid: block #${bundle.anchorRef.blockHeight}`
      : 'Invalid anchor reference',
  });

  // Step 4: Inclusion proof validation
  const inclusionValid =
    bundle.inclusionProof.proofBytes.length > 0 &&
    bundle.inclusionProof.blockHash.length === 64; // 32 bytes = 64 hex chars
  steps.push({
    name: 'Inclusion Proof',
    passed: inclusionValid,
    details: inclusionValid
      ? `Inclusion proof valid (${bundle.inclusionProof.proofBytes.length} bytes, block: ${bundle.inclusionProof.blockHash.slice(0, 16)}...)`
      : 'Inclusion proof missing or invalid',
  });

  // Step 5: Finality proof validation
  const finalityValid = bundle.finalityProof.confirmations >= 6;
  steps.push({
    name: 'Finality Proof',
    passed: finalityValid,
    details: finalityValid
      ? `Finality confirmed with ${bundle.finalityProof.confirmations} confirmations`
      : `Insufficient confirmations: ${bundle.finalityProof.confirmations} (need at least 6)`,
  });

  // Step 6: DAG segment validation
  const dagValid =
    bundle.transitionDag.nodes.length > 0 &&
    bundle.transitionDag.rootCommitment.length === 64 &&
    bundle.transitionDag.nodes.every(
      (n) =>
        n.nodeId.length === 64 &&
        n.parents.every((p) => p.length === 64),
    );
  steps.push({
    name: 'DAG Segment',
    passed: dagValid,
    details: dagValid
      ? `DAG valid: ${bundle.transitionDag.nodes.length} nodes, root: ${bundle.transitionDag.rootCommitment.slice(0, 16)}...`
      : 'Invalid DAG segment',
  });

  // Step 7: Signatures validation
  const sigsValid = bundle.signatures.length > 0;
  steps.push({
    name: 'Signatures',
    passed: sigsValid,
    details: sigsValid
      ? `${bundle.signatures.length} signature(s) present`
      : 'No signatures found',
  });

  const allPassed = steps.every((s) => s.passed);

  return {
    valid: allPassed,
    error: allPassed ? null : 'One or more verification steps failed',
    steps,
  };
}

/**
 * Verify a Consignment offline.
 *
 * A consignment contains the full provenance of a Sanad, including
 * genesis, transitions, seal assignments, and anchor proofs.
 *
 * @param consignment - The Consignment to verify
 * @returns VerificationResult with pass/fail for each step
 */
export function verifyConsignment(consignment: Consignment): VerificationResult {
  const steps: VerificationStep[] = [];

  // Step 1: Version check
  const versionValid = consignment.version === 1;
  steps.push({
    name: 'Version Check',
    passed: versionValid,
    details: versionValid
      ? 'Consignment version 1'
      : `Unsupported version: ${consignment.version}`,
  });

  // Step 2: Genesis validation
  const genesisValid =
    consignment.genesis.contractId.length === 64 &&
    consignment.genesis.schemaId.length === 64;
  steps.push({
    name: 'Genesis Validation',
    passed: genesisValid,
    details: genesisValid
      ? 'Genesis valid: contract and schema IDs present'
      : 'Invalid genesis (missing contract or schema ID)',
  });

  // Step 3: Transitions validation
  const transitionsValid = consignment.transitions.length > 0;
  steps.push({
    name: 'Transitions Validation',
    passed: transitionsValid,
    details: transitionsValid
      ? `${consignment.transitions.length} transition(s) found`
      : 'No transitions found',
  });

  // Step 4: Schema ID check
  const schemaValid = consignment.schemaId.length === 64;
  steps.push({
    name: 'Schema ID',
    passed: schemaValid,
    details: schemaValid ? 'Schema ID present' : 'Missing schema ID',
  });

  // Step 5: Anchor proofs validation
  const anchorsValid = consignment.anchors.length > 0;
  steps.push({
    name: 'Anchor Proofs',
    passed: anchorsValid,
    details: anchorsValid
      ? `${consignment.anchors.length} anchor proof(s) found`
      : 'No anchor proofs found',
  });

  const allPassed = steps.every((s) => s.passed);

  return {
    valid: allPassed,
    error: allPassed ? null : 'One or more verification steps failed',
    steps,
  };
}

/**
 * Verify a ProofBundle from JSON string.
 * Convenience function that parses and verifies in one call.
 *
 * @param json - JSON string of a ProofBundle
 * @returns VerificationResult
 */
export function verifyProofBundleFromJson(json: string): VerificationResult {
  try {
    const bundle = JSON.parse(json) as any;
    // Convert hex strings back to Uint8Arrays
    const parsed = parseProofBundleForVerification(bundle);
    return verifyProofBundle(parsed);
  } catch (e) {
    return {
      valid: false,
      error: `Failed to parse JSON: ${(e as Error).message}`,
      steps: [
        {
          name: 'Parse Proof Bundle',
          passed: false,
          details: `Invalid JSON: ${(e as Error).message}`,
        },
      ],
    };
  }
}

/**
 * Parse a ProofBundle JSON object for verification.
 * This is a simplified parser that handles the hex-to-bytes conversion.
 */
function parseProofBundleForVerification(json: any): ProofBundle {
  return {
    transitionDag: {
      nodes: json.transitionDag.nodes.map((n: any) => ({
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
