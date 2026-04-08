
**Universal Seal Primitive (USP)**

### *Cross-Chain, Gradually Degrading Specification*

**Estimated Reading Time (ERT): ~14 minutes**

* * * * *

## 0. Framing (Why This Exists)

We are not implementing "single-use seals."

We are implementing:

> **A portable right system whose *uniqueness and consumption guarantees degrade gracefully across heterogeneous chains***

This system must:

- Preserve semantics across:

  - UTXO (Bitcoin-like)

  - Object (Sui-like)

  - Account (Ethereum/Solana)

- Avoid leaking abstraction details to developers

- Maintain **consistent mental model** despite different enforcement layers

* * * * *

## 1. Core Invariant (Non-Negotiable)

Every implementation MUST enforce:

> **A Right can be exercised at most once under the strongest available guarantee of the host chain.**

* * * * *

## 2. Canonical Primitive (Chain-Agnostic Spec)

```
Right {
  id: Hash
  commitment: Hash
  owner: OwnershipProof
  nullifier: Optional<Hash>
  state_root: Optional<Hash>
  execution_proof: Optional<Proof>
}

```

* * * * *

2.1 Field Semantics
-------------------

| Field | Purpose |
| --- | --- |
| `id` | Unique identifier of the right |
| `commitment` | Encodes state + rules |
| `owner` | Signature / capability / object ownership |
| `nullifier` | One-time consumption marker (if needed) |
| `state_root` | Off-chain state commitment (RGB-style) |
| `execution_proof` | Optional ZK / fraud proof |

* * * * *

## 3. Degradation Model (The Heart of the System)

This is the most important section.

3.1 Enforcement Layers (Strong → Weak)
--------------------------------------

| Level | Name | Guarantee Type | Chains |
| --- | --- | --- | --- |
| L1 | Structural | Native single-use | Bitcoin, Sui, CKB |
| L2 | Type-Enforced | Language-level scarcity | Aptos, Radix |
| L3 | Cryptographic | Nullifier-based | Ethereum, Solana |
| L4 | Optimistic | Fraud/challenge | Rollups |
| L5 | Social/Economic | Reputation/slashing | Off-chain coordination |

* * * * *

3.2 Degradation Rule
--------------------

```
IF native single-use exists:
    DO NOT introduce nullifier

ELSE IF non-duplicable resource exists:
    USE resource lifecycle

ELSE:
    REQUIRE nullifier tracking

```

* * * * *

## 4. Lifecycle Specification

4.1 Creation
------------

```
INPUT:
  - initial state
  - ownership key
  - rules

OUTPUT:
  - Right.commitment = H(state || rules)
  - Right.id = H(commitment || salt)

```

### Modes

| Mode | Description |
| --- | --- |
| Off-chain | Preferred (RGB-style) |
| On-chain | Required for weak environments |

* * * * *

4.2 Transfer
------------

### Structural Chains (Sui / UTXO)

- Transfer ownership of object / output

### Cryptographic Chains

```
new_owner_signature = Sign(prev_owner, new_owner)
update commitment if needed

```

* * * * *

4.3 Consumption (Critical Path)
-------------------------------

### Unified Logic

```
VERIFY ownership
VERIFY validity of right
CHECK uniqueness constraint
EXECUTE effect
MARK as consumed

```

* * * * *

4.4 Uniqueness Enforcement by Layer
-----------------------------------

| Layer | Mechanism |
| --- | --- |
| Structural | Consume object / spend UTXO |
| Type | Move resource into sink |
| Cryptographic | Register nullifier |
| Optimistic | Allow → challenge later |

* * * * *

## 5. Nullifier Specification (L3 and Below)

5.1 Definition
--------------

```
nullifier = H(right_id || secret || context)

```

### Properties

- Deterministic

- Unique per consumption

- Non-forgeable

* * * * *

5.2 Storage Strategies
----------------------

### Ethereum

```
mapping(bytes32 => bool) public nullifiers;

```

### Solana

- Nullifier = PDA (Program Derived Address)

- Creation = consumption proof

* * * * *

5.3 Optimization
----------------

| Technique | Benefit |
| --- | --- |
| Merkle accumulator | Batch verification |
| Sparse trees | Storage efficiency |
| Rollups | Offload verification |

* * * * *

## 6. Client-Side Validation (RGB Mode)

6.1 Principles
--------------

- Chain stores:

  - commitments

  - nullifiers (if needed)

- Client stores:

  - full state history

* * * * *

6.2 Validation Flow
-------------------

```
1\. Fetch state proof chain
2. Verify commitment chain
3. Check no conflicting consumption
4. Accept as valid

```

* * * * *

6.3 Failure Modes
-----------------

| Failure | Handling |
| --- | --- |
| Missing history | Reject |
| Conflicting state | Require resolution |
| Double-use | escalate to chain |

* * * * *

## 7. Chain-Specific Adapters

=

* * * * *

7.1 Ethereum Adapter
--------------------

### Components

- Smart contract:

  - nullifier registry

  - execution verifier

### Flow

```
submit:
  - ownership proof
  - nullifier
  - execution data

contract:
  require(!nullifier_used)
  mark nullifier
  execute

```

* * * * *

7.2 Solana Adapter
------------------

### Components

- Program

- PDA-based nullifiers

### Flow

```
derive PDA(nullifier)
if PDA exists → reject
else:
  create PDA
  execute logic

```

* * * * *

7.3 Sui Adapter (Reference Implementation)
------------------------------------------

- Right = Object

- Consumption = object deletion / mutation

No nullifier required.

* * * * *

## 8. Strong Guarantee Extensions

=====

* * * * *

8.1 ZK Nullifiers
-----------------

- Hide:

  - ownership

  - state

- Only reveal:

  - nullifier

* * * * *

8.2 Fraud Proof Layer
---------------------

```
allow execution
window for challenge
if fraud proven → revert/slash

```

* * * * *

8.3 Slashing Mechanism
----------------------

- Stake required for issuers

- Double-use → slash stake

* * * * *

## 9. Security Considerations

=

* * * * *

9.1 Double-Spend Race
---------------------

- Mitigation:

  - first-seen rule (chain)

  - ordering guarantees

* * * * *

9.2 Replay Attacks
------------------

- Include:

  - chain_id

  - context

  - domain separator

* * * * *

9.3 State Divergence
--------------------

- Resolve via:

  - canonical commitment

  - dispute resolution

* * * * *

## 10. Developer Interface (Abstraction Layer)

```
interface Right {
  create(...)
  transfer(...)
  consume(...)
  verify(...)
}

```

* * * * *

10.1 Execution Modes
--------------------

| Mode | Description |
| --- | --- |
| Native | Sui / UTXO |
| Verified | Ethereum |
| Optimistic | Off-chain first |

* * * * *

## 11. Implementation Phases

* * * * *

Phase 1 --- Canonical Model
-------------------------

- Define Right struct

- Implement client validation engine

* * * * *

Phase 2 --- Native Chain
----------------------

- Deploy on Sui

- Validate lifecycle

* * * * *

Phase 3 --- Cryptographic Chain
-----------------------------

- Ethereum adapter

- Nullifier system

* * * * *

Phase 4 --- Parallel Model
------------------------

- Solana adapter

- PDA optimization

* * * * *

Phase 5 --- Advanced Guarantees
-----------------------------

- ZK integration

- Fraud proofs

* * * * *

## 12. Open Design Edges (We Should Decide Together)

======

These will change architecture significantly:

### 1\. Nullifier Scope

- Global?

- Per contract?

- Per application?

* * * * *

### 2\. Privacy Level

- Transparent (simpler)

- ZK (complex, powerful)

* * * * *

### 3\. Settlement Strategy

- Immediate finality

- Optimistic delay

* * * * *

### 4\. Cross-Chain Portability

- Lock/mint?

- Proof-based teleportation?

* * * * *

## Key Takeaways

- We are not porting seals---we are **reconstructing them under different guarantees**

- The system hinges on:

  - **nullifiers (for weak chains)**

  - **object consumption (for strong chains)**

- The abstraction must:

  - preserve semantics

  - adapt enforcement

- Ethereum/Solana are:

  - **verification layers, not state layers**

* * * * *

## Test

### 1\. One concrete use-case

→ run through ALL layers

### 2\. One adversarial scenario

→ double-spend under latency

### 3\. One cross-chain flow

→ move Right between chains
