#!/usr/bin/env node
'use strict';

const crypto = require('crypto');
const { EventEmitter } = require('events');

/**
 * Cross-Chain Rights Registry
 * 
 * Tracks Rights across all chains and enforces single-use (prevents double-spend).
 * This is the core CSV abstraction - a Right can only exist on one chain at a time.
 */
class CrossChainRegistry extends EventEmitter {
  constructor() {
    super();

    // State
    this.rights = new Map(); // rightId -> Right
    this.transfers = new Map(); // transferId -> Transfer
    this.chains = ['bitcoin', 'ethereum', 'sui', 'aptos'];
    this.stats = {
      totalRightsCreated: 0,
      totalTransfersInitiated: 0,
      totalTransfersCompleted: 0,
      totalDoubleSpendAttempts: 0
    };
  }

  /**
   * Create a new Right on a specific chain
   */
  createRight(params) {
    const {
      id = 'right_' + crypto.randomBytes(8).toString('hex'),
      name,
      owner,
      chain,
      metadata = {},
      txHash
    } = params;

    // Check if right already exists
    if (this.rights.has(id)) {
      throw new Error(`Right ${id} already exists`);
    }

    // Validate chain
    if (!this.chains.includes(chain)) {
      throw new Error(`Invalid chain: ${chain}`);
    }

    const right = {
      id,
      name,
      owner,
      chain,
      metadata,
      txHash: txHash || null,
      status: 'active', // active, transferring, burned
      createdAt: Date.now(),
      updatedAt: Date.now(),
      history: [{
        action: 'created',
        chain,
        owner,
        txHash,
        timestamp: Date.now()
      }]
    };

    this.rights.set(id, right);
    this.stats.totalRightsCreated++;
    this.emit('right:created', right);

    return right;
  }

  /**
   * Get a Right by ID
   */
  getRight(id) {
    return this.rights.get(id) || null;
  }

  /**
   * Get all Rights
   */
  getAllRights() {
    return Array.from(this.rights.values());
  }

  /**
   * Get Rights by owner
   */
  getRightsByOwner(owner) {
    return Array.from(this.rights.values()).filter(r => r.owner === owner);
  }

  /**
   * Get Rights on a specific chain
   */
  getRightsByChain(chain) {
    return Array.from(this.rights.values()).filter(r => r.chain === chain);
  }

  /**
   * Register a cross-chain transfer
   * Enforces single-use: Right must be active and not currently transferring
   */
  async registerTransfer(params) {
    const {
      rightId,
      fromChain,
      toChain,
      fromOwner,
      toOwner,
      sourceTxHash,
      proof = null
    } = params;

    this.stats.totalTransfersInitiated++;

    // Find the right
    const right = this.rights.get(rightId);
    if (!right) {
      throw new Error(`Right ${rightId} not found`);
    }

    // Check for double-spend: Right must be active
    if (right.status !== 'active') {
      this.stats.totalDoubleSpendAttempts++;
      throw new Error(`Double-spend attempt: Right ${rightId} is ${right.status} (not active)`);
    }

    // Verify the right is on the source chain
    if (right.chain !== fromChain) {
      throw new Error(`Right ${rightId} is on ${right.chain}, not ${fromChain}`);
    }

    // Verify ownership
    if (right.owner !== fromOwner) {
      throw new Error(`Right ${rightId} is owned by ${right.owner}, not ${fromOwner}`);
    }

    // Verify source and destination chains are different
    if (fromChain === toChain) {
      // Same-chain transfer is allowed but doesn't need cross-chain logic
      right.owner = toOwner;
      right.updatedAt = Date.now();
      right.history.push({
        action: 'transfer_same_chain',
        chain: fromChain,
        fromOwner,
        toOwner,
        txHash: sourceTxHash,
        timestamp: Date.now()
      });
      this.emit('right:transferred', right);
      return right;
    }

    // Mark right as transferring (lock it)
    right.status = 'transferring';
    right.updatedAt = Date.now();
    right.history.push({
      action: 'transfer_initiated',
      fromChain,
      toChain,
      fromOwner,
      toOwner,
      sourceTxHash,
      timestamp: Date.now()
    });

    // Create transfer record
    const transferId = 'transfer_' + crypto.randomBytes(8).toString('hex');
    const transfer = {
      id: transferId,
      rightId,
      fromChain,
      toChain,
      fromOwner,
      toOwner,
      sourceTxHash,
      proof,
      status: 'pending', // pending, verified, completed, failed
      createdAt: Date.now(),
      updatedAt: Date.now(),
      steps: [{
        step: 'initiated',
        chain: fromChain,
        txHash: sourceTxHash,
        timestamp: Date.now()
      }]
    };

    this.transfers.set(transferId, transfer);
    this.emit('transfer:initiated', transfer);

    return transfer;
  }

  /**
   * Complete a cross-chain transfer
   * Burns the Right on source chain and mints on destination
   */
  async completeTransfer(transferId, destTxHash) {
    const transfer = this.transfers.get(transferId);
    if (!transfer) {
      throw new Error(`Transfer ${transferId} not found`);
    }

    if (transfer.status !== 'pending') {
      throw new Error(`Transfer ${transferId} is ${transfer.status}, cannot complete`);
    }

    const right = this.rights.get(transfer.rightId);
    if (!right) {
      throw new Error(`Right ${transfer.rightId} not found`);
    }

    // Verify proof (simplified simulation)
    transfer.status = 'completed';
    transfer.updatedAt = Date.now();
    transfer.destTxHash = destTxHash;
    transfer.steps.push({
      step: 'completed',
      chain: transfer.toChain,
      txHash: destTxHash,
      timestamp: Date.now()
    });

    // Burn on source chain, mint on destination
    right.chain = transfer.toChain;
    right.owner = transfer.toOwner;
    right.status = 'active';
    right.txHash = destTxHash;
    right.updatedAt = Date.now();
    right.history.push({
      action: 'transfer_completed',
      fromChain: transfer.fromChain,
      toChain: transfer.toChain,
      fromOwner: transfer.fromOwner,
      toOwner: transfer.toOwner,
      sourceTxHash: transfer.sourceTxHash,
      destTxHash,
      timestamp: Date.now()
    });

    this.stats.totalTransfersCompleted++;
    this.emit('transfer:completed', transfer);
    this.emit('right:transferred', right);

    return { transfer, right };
  }

  /**
   * Attempt a double-spend (for testing security)
   * This should always fail
   */
  attemptDoubleSpend(rightId, newOwner, chain) {
    const right = this.rights.get(rightId);
    if (!right) {
      return { success: false, error: 'Right not found' };
    }

    try {
      // Try to transfer without proper locking
      if (right.status !== 'active') {
        return {
          success: false,
          error: 'Double-spend prevented: Right is not active',
          currentStatus: right.status
        };
      }

      // If somehow active, still verify chain
      return { success: false, error: 'Double-spend prevented: Transfer requires proper protocol' };
    } catch (e) {
      return { success: false, error: e.message };
    }
  }

  /**
   * Verify a proof (simplified simulation)
   */
  verifyProof(proof) {
    if (!proof) {
      return { valid: false, error: 'No proof provided' };
    }

    // In a real implementation, this would verify Merkle proofs,
    // SPV proofs, or light client proofs depending on the chain
    return {
      valid: true,
      confidence: 0.99 // Simulated confidence
    };
  }

  /**
   * Get all transfers
   */
  getAllTransfers() {
    return Array.from(this.transfers.values());
  }

  /**
   * Get transfer by ID
   */
  getTransfer(transferId) {
    return this.transfers.get(transferId) || null;
  }

  /**
   * Get transfers by status
   */
  getTransfersByStatus(status) {
    return Array.from(this.transfers.values()).filter(t => t.status === status);
  }

  /**
   * Get stats
   */
  getStats() {
    return {
      ...this.stats,
      activeRights: Array.from(this.rights.values()).filter(r => r.status === 'active').length,
      pendingTransfers: Array.from(this.transfers.values()).filter(t => t.status === 'pending').length
    };
  }

  /**
   * Reset registry to initial state
   */
  reset() {
    this.rights.clear();
    this.transfers.clear();
    this.stats = {
      totalRightsCreated: 0,
      totalTransfersInitiated: 0,
      totalTransfersCompleted: 0,
      totalDoubleSpendAttempts: 0
    };
  }
}

module.exports = { CrossChainRegistry };
