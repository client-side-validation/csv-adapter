import { Chain, ProtocolVersion, Capabilities, SyncStatus, TransferStatus } from './types';
import { Right } from './right';
import { SealRef, AnchorRef } from './seal';
import { ProofBundle } from './proof';
import { Consignment } from './consignment';
import { VerificationResult, verifyProofBundle, verifyConsignment } from './verify';

/**
 * Configuration for the CSV client.
 */
export interface CsvClientConfig {
  /** RPC endpoint URLs per chain */
  rpcEndpoints?: Partial<Record<Chain, string>>;
  /** Default chain */
  defaultChain?: Chain;
  /** Network (mainnet, testnet, signet, etc.) */
  network?: string;
  /** API key for authenticated endpoints */
  apiKey?: string;
}

/**
 * CSV Client — unified entry point for all CSV operations.
 *
 * Mirrors csv_adapter::client::CsvClient but in TypeScript.
 *
 * Key concepts:
 * - **Right**: A verifiable, single-use digital right. Exists in client state.
 * - **Seal**: The on-chain mechanism enforcing a Right's single-use.
 * - **Client-Side Validation**: The client verifies, not the blockchain.
 *
 * Usage:
 * ```typescript
 * const client = new CsvClient({
 *   defaultChain: 'bitcoin',
 *   network: 'signet',
 * });
 *
 * // List rights
 * const rights = await client.getRights('bc1q...');
 *
 * // Verify a proof bundle offline
 * const result = client.verifyProofBundle(bundleJson);
 * ```
 */
export class CsvClient {
  private config: CsvClientConfig;
  private rightsCache: Map<string, Right[]> = new Map();

  constructor(config: CsvClientConfig = {}) {
    this.config = {
      defaultChain: config.defaultChain ?? 'bitcoin',
      network: config.network ?? 'mainnet',
      rpcEndpoints: config.rpcEndpoints ?? {},
      apiKey: config.apiKey,
    };
  }

  /**
   * Get the default chain.
   */
  getDefaultChain(): Chain {
    return this.config.defaultChain!;
  }

  /**
   * Get the configured network.
   */
  getNetwork(): string {
    return this.config.network!;
  }

  /**
   * Get the protocol version.
   */
  getProtocolVersion(): ProtocolVersion {
    return { major: 0, minor: 4, patch: 0 };
  }

  /**
   * Get the protocol capabilities.
   */
  getCapabilities(): Capabilities {
    return {
      advancedCommitments: true,
      mpcProofs: true,
      vmTransitions: true,
      rgbCompat: false,
      tapretVerify: true,
      crossChainTransfers: true,
    };
  }

  /**
   * Get the sync status for a chain.
   */
  getSyncStatus(chain: Chain): SyncStatus {
    return { kind: 'synced', latest: 0 };
  }

  // =========================================================================
  // Rights operations
  // =========================================================================

  /**
   * List rights for an address.
   *
   * @param address - The address to query
   * @param chain - Optional chain filter
   * @returns Array of Rights
   */
  async getRights(address: string, chain?: Chain): Promise<Right[]> {
    const cacheKey = `${address}:${chain ?? this.config.defaultChain}`;
    const cached = this.rightsCache.get(cacheKey);
    if (cached) return cached;

    // In production, this would query the chain adapter or explorer API
    // For now, return empty array
    return [];
  }

  /**
   * Get a specific right by ID.
   *
   * @param rightId - The right ID (32-byte hex string)
   * @returns The Right, or null if not found
   */
  async getRight(rightId: string): Promise<Right | null> {
    // In production, this would query the store or chain
    return null;
  }

  /**
   * Create a new right.
   *
   * @param commitment - The commitment hash
   * @param owner - The ownership proof
   * @param salt - The salt for right ID generation
   * @returns The created Right
   */
  async createRight(
    commitment: string,
    owner: { proof: string; owner: string; scheme: string | null },
    salt: string,
  ): Promise<Right> {
    // In production, this would create a seal first, then the right
    throw new Error('Right creation requires chain adapter integration');
  }

  // =========================================================================
  // Seal operations
  // =========================================================================

  /**
   * Create a seal on a chain.
   *
   * A seal is a chain-native single-use lock that enforces
   * the single-use property of a Right.
   *
   * @param chain - The blockchain to create the seal on
   * @param value - Optional value to lock (chain-specific units)
   * @returns The SealRef
   */
  async createSeal(chain: Chain, value?: number): Promise<SealRef> {
    // In production, this would call the chain adapter's create_seal()
    throw new Error('Seal creation requires chain adapter integration');
  }

  /**
   * Check if a seal is consumed.
   *
   * @param sealRef - The seal reference to check
   * @returns true if the seal has been consumed
   */
  async isSealConsumed(sealRef: SealRef): Promise<boolean> {
    // In production, this would check the seal registry
    return false;
  }

  // =========================================================================
  // Proof operations
  // =========================================================================

  /**
   * Generate a proof bundle for a right.
   *
   * A proof bundle contains:
   * - The state transition DAG
   * - Signatures authorizing the transition
   * - Seal reference (consumed)
   * - Anchor reference (on-chain location)
   * - Inclusion proof (Merkle branch)
   * - Finality proof (confirmations)
   *
   * @param rightId - The right to prove
   * @param chain - The source chain
   * @returns The ProofBundle
   */
  async generateProofBundle(rightId: string, chain: Chain): Promise<ProofBundle> {
    // In production, this would build the bundle from chain data
    throw new Error('Proof bundle generation requires chain adapter integration');
  }

  /**
   * Verify a proof bundle offline.
   *
   * This is the CSV competitive advantage:
   * "No RPC calls needed. Pure cryptographic verification."
   *
   * @param bundle - The ProofBundle to verify
   * @returns VerificationResult
   */
  verifyProofBundle(bundle: ProofBundle): VerificationResult {
    return verifyProofBundle(bundle);
  }

  /**
   * Verify a proof bundle from JSON string.
   *
   * @param json - JSON string of a ProofBundle
   * @returns VerificationResult
   */
  verifyProofBundleFromJson(json: string): VerificationResult {
    return verifyProofBundleFromJson(json);
  }

  // =========================================================================
  // Consignment operations
  // =========================================================================

  /**
   * Verify a consignment offline.
   *
   * A consignment is the complete transfer artifact containing
   * genesis, transitions, seal assignments, and anchor proofs.
   *
   * @param consignment - The Consignment to verify
   * @returns VerificationResult
   */
  verifyConsignment(consignment: Consignment): VerificationResult {
    return verifyConsignment(consignment);
  }

  /**
   * Accept a consignment into local state.
   *
   * Before accepting, the consignment is verified:
   * 1. Structural validation
   * 2. Commitment chain validation
   * 3. Seal consumption validation (double-spend check)
   * 4. State transition validation
   * 5. Final acceptance
   *
   * @param consignment - The verified Consignment to accept
   * @returns The accepted Right
   */
  async acceptConsignment(consignment: Consignment): Promise<Right> {
    // First verify
    const result = this.verifyConsignment(consignment);
    if (!result.valid) {
      throw new Error(`Consignment verification failed: ${result.steps.find(s => !s.passed)?.details}`);
    }

    // In production, this would add the right to local state
    throw new Error('Consignment acceptance requires store integration');
  }

  // =========================================================================
  // Cross-chain operations
  // =========================================================================

  /**
   * Start a cross-chain transfer.
   *
   * The transfer follows this state machine:
   * Locked → AwaitingFinality → BuildingProof → ProofReady → Minting → Complete
   *
   * @param rightId - The right to transfer
   * @param sourceChain - The source chain
   * @param destinationChain - The destination chain
   * @param destinationOwner - The destination owner address
   * @returns Transfer ID
   */
  async startCrossChainTransfer(
    rightId: string,
    sourceChain: Chain,
    destinationChain: Chain,
    destinationOwner: string,
  ): Promise<string> {
    // In production, this would:
    // 1. Create a seal on the source chain
    // 2. Lock the right
    // 3. Return a transfer ID
    throw new Error('Cross-chain transfer requires chain adapter integration');
  }

  /**
   * Get the status of a cross-chain transfer.
   *
   * @param transferId - The transfer ID
   * @returns The current TransferStatus
   */
  async getTransferStatus(transferId: string): Promise<TransferStatus> {
    // In production, this would query the transfer state machine
    return { kind: 'initiated' };
  }

  // =========================================================================
  // Utility methods
  // =========================================================================

  /**
   * Export all rights as JSON.
   */
  async exportRights(): Promise<string> {
    const allRights: Right[] = [];
    for (const rights of this.rightsCache.values()) {
      allRights.push(...rights);
    }
    return JSON.stringify(allRights, null, 2);
  }

  /**
   * Import rights from JSON.
   */
  async importRights(json: string): Promise<void> {
    const rights: Right[] = JSON.parse(json);
    for (const right of rights) {
      // Store by owner address
      const ownerHex = bytesToHex(right.owner.owner);
      const existing = this.rightsCache.get(ownerHex) ?? [];
      existing.push(right);
      this.rightsCache.set(ownerHex, existing);
    }
  }
}
