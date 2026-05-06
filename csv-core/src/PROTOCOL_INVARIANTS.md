# Protocol Invariants — DO NOT VIOLATE

This document defines the fundamental invariants of the CSV (Client-Side Validation) protocol. These invariants are non-negotiable and exist for security reasons. Any code change that violates these invariants must be rejected.

## Invariant 1: Seal IDs Must Come From Real Blockchain Transactions

**Rule:** A `SealPoint.seal_id` must come from a real blockchain transaction.

**Prohibited:**

- Never construct seal IDs from timestamps
- Never construct seal IDs from UUIDs
- Never construct seal IDs from random bytes
- Never use "fake" or "mock" seal IDs in production

**Correct Pattern:**

```rust
// Use the chain adapter's create_seal() method
let seal_ref = chain_adapter.create_seal(value)?;
// seal_ref.seal_id now contains the actual UTXO txid, PDA address, etc.
```

**Security Impact:** Fake seal IDs enable double-spend attacks because the seal is not actually consumed on-chain.

**Error Code:** `CORE_SEAL_NOT_ANCHORED` is raised when a fake seal is detected.

---

## Invariant 2: Commitments Must Be Published On-Chain Before Proof Building

**Rule:** A `Commitment` must be published on-chain before a `ProofBundle` is built.

**Prohibited:**

- Never build a `ProofBundle` without an `CommitAnchor`
- Never use simulated/mock anchors in production
- Never skip the publication step

**Correct Pattern:**

```rust
// 1. Create and publish the commitment
let anchor = anchor_layer.publish(commitment, seal)?;

// 2. Wait for finality
let finality = anchor_layer.verify_finality(anchor.clone())?;

// 3. Build the proof bundle with real anchor data
let proof_bundle = ProofBundle::new(
    dag_segment,
    signatures,
    seal_ref,
    anchor,      // Real anchor from chain
    inclusion,   // Verified inclusion proof
    finality,    // Verified finality proof
)?;
```

**Security Impact:** Proof bundles without on-chain anchors provide no security guarantee. They can be forged by anyone.

---

## Invariant 3: Sanads Must Pass ConsignmentValidator Before Entering AppState

**Rule:** A `Sanad` must pass all 5 validation steps of `ConsignmentValidator` before being accepted into `AppState`.

**Required Steps:**

1. Structural Validation — version, schema, required fields
2. Commitment Chain Validation — genesis to latest integrity
3. Seal Consumption Validation — double-spend detection
4. State Transition Validation — valid evolution rules
5. Final Acceptance Decision — all checks must pass

**Prohibited:**

- Never accept a Sanad without running all 5 validation steps
- Never skip validation for "trusted" sources
- Never cache validation results across consignment updates

**Correct Pattern:**

```rust
let validator = ConsignmentValidator::new();
let report = validator.validate_consignment(&consignment, ChainId::Bitcoin);

if !report.passed {
    // Reject the consignment — do not add to AppState
    return Err(Error::ValidationFailed(report.summary));
}

// Only now add to AppState
app_state.add_sanad(sanad)?;
```

**Security Impact:** Skipping validation allows fraudulent state transitions to enter the wallet state, enabling theft.

---

## Invariant 4: Balances Are Stored as u64 Native Units

**Rule:** Balances must be stored as `u64` native units (satoshis, lamports, MIST, octas, wei).

**Prohibited:**

- Never store balances as `f64` (floating point)
- Never store balances as human-readable strings ("1.5 BTC")
- Never use JSON numbers for financial amounts (precision loss)

**Correct Pattern:**

```rust
pub struct ChainAccount {
    /// Balance in native chain units (satoshis, wei, lamports, etc.)
    pub balance_raw: u64,
}

// Display conversion happens only at UI layer
let display = format!("{:.8} BTC", balance_raw as f64 / 100_000_000.0);
```

**Security Impact:** Floating point rounding errors and precision loss can be exploited for value manipulation (e.g., 0.1 + 0.2 != 0.3 bugs).

---

## Invariant 5: Cross-Chain Transfers Must Follow the TransferState Machine

**Rule:** All cross-chain transfers must progress through the `TransferState` machine states in order:

```
Locked → AwaitingFinality → BuildingProof → ProofReady → Minting → Complete
```

**Prohibited:**

- Never skip from `Locked` directly to `Minting`
- Never build proofs before finality is reached
- Never retry a failed transfer without checking `recoverable` flag

**Correct Pattern:**

```rust
// Drive the state machine forward
match transfer.state {
    TransferState::Locked { source_tx, lock_height } => {
        // Check confirmations
        let confirmations = chain.get_confirmations(source_tx).await?;
        if confirmations >= REQUIRED_CONFIRMATIONS {
            transfer.state = TransferState::AwaitFinality {
                confirmations_needed: REQUIRED_CONFIRMATIONS,
                confirmations_have: confirmations,
            };
        }
    }
    TransferState::AwaitingFinality { .. } => {
        // Wait for finality before building proof
        if finality_reached {
            transfer.state = TransferState::BuildingProof;
        }
    }
    // ... etc
}
```

**Security Impact:** Skipping steps enables attacks like minting before the source seal is actually consumed, allowing double-spends.

---

## Invariant 6: SealRegistry Must Be Checked Before Accepting Any Transfer

**Rule:** `SealRegistry::check_consumed` must run before accepting any incoming transfer.

**Prohibited:**

- Never accept a transfer without double-spend check
- Never rely on client-side caching alone
- Never skip the check for "fast path" optimizations

**Correct Pattern:**

```rust
// Check the cross-chain seal registry for double-spends
match registry.check_seal_status(&seal_ref) {
    SealStatus::Unconsumed => {
        // Safe to proceed
        accept_transfer(transfer)?;
    }
    SealStatus::ConsumedOnChain { chain, .. } => {
        // Reject — this seal was already used
        return Err(Error::DoubleSpendDetected(chain));
    }
    SealStatus::DoubleSpent => {
        // Critical security alert
        return Err(Error::DoubleSpendAttackDetected);
    }
}
```

**Security Impact:** Without this check, an attacker can reuse the same seal across multiple transfers, stealing funds.

---

## Invariant 7: Domain Separation Must Be Used for All Hashes

**Rule:** All cryptographic hashes must use domain separation to prevent cross-chain replay attacks.

**Prohibited:**

- Never hash raw data without domain prefix
- Never use the same hash function across different chains without separation
- Never omit chain identifier from commitment hashes

**Correct Pattern:**

```rust
// Domain-separated commitment hash
let commitment = hash(
    domain_separator ||      // Chain-specific domain
    chain_id ||              // Chain identifier
    contract_id ||           // Contract identifier
    previous_commitment ||   // Previous in chain
    transition_payload_hash || // Transition data
    seal_hash                // Seal reference
);
```

**Security Impact:** Without domain separation, a commitment from one chain can be replayed on another chain, enabling cross-chain double-spends.

---

## Audit Checklist for Code Reviews

When reviewing code changes, verify:

- [ ] No fake seal IDs are constructed
- [ ] Proof bundles always include real CommitAnchors
- [ ] ConsignmentValidator runs before accepting Sanads
- [ ] Balances are u64 native units only
- [ ] TransferState machine is not skipped
- [ ] SealRegistry check runs before transfer acceptance
- [ ] Domain separation is used for all hashes

**Violations of any of these invariants must block the PR.**

---

## Questions?

If you're unsure whether your change violates an invariant:

1. Read the relevant section of `docs/ARCHITECTURE.md`
2. Check `docs/BLUEPRINT.md` for detailed protocols
3. Ask in #protocol-security channel before merging

**When in doubt, ask. Security is everyone's responsibility.**
