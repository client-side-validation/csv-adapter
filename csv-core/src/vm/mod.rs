//! Deterministic VM for CSV Contract Execution
//!
//! This module provides the execution engine for CSV contract transitions.
//! Any VM implementing [`DeterministicVM`] must guarantee:
//!
//! - **Determinism**: The same bytecode + inputs always produce the same outputs
//! - **Isolation**: No access to external state (time, network, random, etc.)
//! - **Bounded execution**: No infinite loops via cycle/gas counting
//!
//! ## Implementations
//!
//! | Adapter | Purpose |
//! |---------|---------|
//! | [`AluVmAdapter`] | Real AluVM execution engine for production CSV contracts |
//! | [`MeteredVMAdapter`] | Wraps any `DeterministicVM` and tracks step counts for gas accounting |
//! | [`PassthroughVM`] | Testing shim that passes inputs through as outputs |
//!
//! ## Usage
//!
//! ```
//! use csv_adapter_core::vm::{DeterministicVM, VMInputs, AluVmAdapter, execute_transition};
//!
//! let vm = AluVmAdapter::new(1_000_000);
//! let inputs = VMInputs::default();
//! let outputs = vm.execute(&bytecode, inputs, &sigs).unwrap();
//! ```

mod aluvm;
mod metered;
mod passthrough;

pub use aluvm::AluVmAdapter;
pub use metered::MeteredVMAdapter;
pub use passthrough::PassthroughVM;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::seal::SealRef;
use crate::state::{GlobalState, Metadata, OwnedState, StateAssignment, StateRef, StateTypeId};

// ─────────────────────────────────────────────────────────────────────────────
// Error Types
// ─────────────────────────────────────────────────────────────────────────────

/// Errors that can occur during VM execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VMError {
    /// Bytecode is malformed or invalid
    InvalidBytecode(String),
    /// Execution ran out of steps (loop detection / gas limit)
    ExecutionLimitExceeded {
        /// Maximum allowed execution steps
        max_steps: u64,
        /// Actual number of steps executed before limit reached
        actual_steps: u64,
    },
    /// A state input referenced in the transition was not found
    StateNotFound {
        /// The state reference that was not found
        state_ref: StateRef,
    },
    /// The VM produced inconsistent output (e.g., negative supply)
    InconsistentOutput(String),
    /// Signature verification failed
    InvalidSignature(String),
    /// Seal was already consumed (replay detected)
    SealReplay {
        /// The seal that was replayed
        seal: SealRef,
    },
    /// Schema validation failed
    SchemaViolation(String),
    /// Generic execution error
    ExecutionError(String),
}

impl core::fmt::Display for VMError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VMError::InvalidBytecode(msg) => write!(f, "Invalid bytecode: {msg}"),
            VMError::ExecutionLimitExceeded {
                max_steps,
                actual_steps,
            } => write!(
                f,
                "Execution limit exceeded: {actual_steps} steps (max {max_steps})"
            ),
            VMError::StateNotFound { state_ref } => {
                write!(f, "State not found: {:?}", state_ref)
            }
            VMError::InconsistentOutput(msg) => write!(f, "Inconsistent output: {msg}"),
            VMError::InvalidSignature(msg) => write!(f, "Invalid signature: {msg}"),
            VMError::SealReplay { seal } => write!(f, "Seal replay detected: {:?}", seal),
            VMError::SchemaViolation(msg) => write!(f, "Schema violation: {msg}"),
            VMError::ExecutionError(msg) => write!(f, "Execution error: {msg}"),
        }
    }
}



// ─────────────────────────────────────────────────────────────────────────────
// Input / Output Types
// ─────────────────────────────────────────────────────────────────────────────

/// Input state for VM execution.
///
/// Contains all state that must be consumed as input to a transition,
/// including owned states, global state, metadata, and seal data.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VMInputs {
    /// Owned states being consumed (resolved from StateRef)
    pub owned_inputs: Vec<OwnedState>,
    /// Current global state values
    pub global_state: Vec<GlobalState>,
    /// Transition metadata
    pub metadata: Vec<Metadata>,
    /// Seal data being consumed (authorizes this transition)
    pub seal_data: Vec<u8>,
}

impl Default for VMInputs {
    fn default() -> Self {
        Self {
            owned_inputs: Vec::new(),
            global_state: Vec::new(),
            metadata: Vec::new(),
            seal_data: Vec::new(),
        }
    }
}

impl VMInputs {
    /// Create new VM inputs.
    pub fn new(
        owned_inputs: Vec<OwnedState>,
        global_state: Vec<GlobalState>,
        metadata: Vec<Metadata>,
        seal_data: Vec<u8>,
    ) -> Self {
        Self {
            owned_inputs,
            global_state,
            metadata,
            seal_data,
        }
    }

    /// Look up a global state by type ID.
    pub fn global_state_of(&self, type_id: StateTypeId) -> Vec<&GlobalState> {
        self.global_state
            .iter()
            .filter(|s| s.type_id == type_id)
            .collect()
    }

    /// Look up owned states by type ID.
    pub fn owned_state_of(&self, type_id: StateTypeId) -> Vec<&OwnedState> {
        self.owned_inputs
            .iter()
            .filter(|s| s.type_id == type_id)
            .collect()
    }
}

/// Output state from VM execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VMOutputs {
    /// New owned state assignments (who gets what)
    pub owned_outputs: Vec<StateAssignment>,
    /// Updated global state values
    pub global_updates: Vec<GlobalState>,
    /// Updated metadata
    pub metadata_updates: Vec<Metadata>,
    /// The next seal to be consumed (derived from the transition)
    pub next_seal: Option<SealRef>,
}

impl Default for VMOutputs {
    fn default() -> Self {
        Self {
            owned_outputs: Vec::new(),
            global_updates: Vec::new(),
            metadata_updates: Vec::new(),
            next_seal: None,
        }
    }
}

impl VMOutputs {
    /// Create new VM outputs.
    pub fn new(
        owned_outputs: Vec<StateAssignment>,
        global_updates: Vec<GlobalState>,
        metadata_updates: Vec<Metadata>,
        next_seal: Option<SealRef>,
    ) -> Self {
        Self {
            owned_outputs,
            global_updates,
            metadata_updates,
            next_seal,
        }
    }

    /// Get total value by type ID (for fungible asset validation).
    pub fn total_by_type(&self) -> BTreeMap<StateTypeId, u64> {
        let mut totals = BTreeMap::new();
        for assignment in &self.owned_outputs {
            let value = decode_integer(&assignment.data).unwrap_or(0);
            *totals.entry(assignment.type_id).or_insert(0) += value;
        }
        for update in &self.global_updates {
            let value = decode_integer(&update.data).unwrap_or(0);
            *totals.entry(update.type_id).or_insert(0) += value;
        }
        totals
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DeterministicVM Trait
// ─────────────────────────────────────────────────────────────────────────────

/// The `DeterministicVM` trait defines the interface for CSV contract execution.
///
/// Any VM implementing this trait must guarantee:
/// - The same bytecode + inputs always produce the same outputs
/// - No access to external state (time, network, random, etc.)
/// - Bounded execution (no infinite loops)
pub trait DeterministicVM {
    /// Execute a transition's bytecode with the given inputs.
    ///
    /// # Arguments
    /// * `bytecode` — The validation script (e.g., AluVM bytecode)
    /// * `inputs` — The input state being consumed
    /// * `signatures` — Authorizing signatures
    ///
    /// # Returns
    /// The output state produced by execution.
    fn execute(
        &self,
        bytecode: &[u8],
        inputs: VMInputs,
        signatures: &[Vec<u8>],
    ) -> Result<VMOutputs, VMError>;

    /// Validate that outputs are consistent with the schema.
    ///
    /// This is called after execution to ensure the VM
    /// hasn't produced invalid state (e.g., negative supply,
    /// undefined type IDs).
    fn validate_outputs(&self, inputs: &VMInputs, outputs: &VMOutputs) -> Result<(), VMError>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Execute a transition through the VM and validate.
///
/// This is the primary entry point for consignment validation.
pub fn execute_transition(
    vm: &impl DeterministicVM,
    bytecode: &[u8],
    inputs: VMInputs,
    signatures: &[Vec<u8>],
) -> Result<VMOutputs, VMError> {
    let outputs = vm.execute(bytecode, inputs.clone(), signatures)?;
    vm.validate_outputs(&inputs, &outputs)?;
    Ok(outputs)
}

/// Decode a state data value as a u64 (little-endian).
pub fn decode_integer(data: &[u8]) -> Option<u64> {
    if data.len() < 8 {
        return None;
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[..8]);
    Some(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Hash;

    fn test_inputs() -> VMInputs {
        VMInputs::new(
            vec![OwnedState::from_hash(
                10,
                SealRef::new(vec![0xAA; 16], Some(1)).unwrap(),
                Hash::new([1u8; 32]),
            )],
            vec![GlobalState::from_hash(1, Hash::new([100u8; 32]))],
            vec![Metadata::from_string("memo", "test")],
            vec![0x01, 0x02, 0x03],
        )
    }

    #[test]
    fn test_vm_inputs_creation() {
        let inputs = test_inputs();
        assert_eq!(inputs.owned_inputs.len(), 1);
        assert_eq!(inputs.global_state.len(), 1);
    }

    #[test]
    fn test_vm_inputs_lookup() {
        let inputs = test_inputs();
        assert_eq!(inputs.global_state_of(1).len(), 1);
        assert!(inputs.global_state_of(99).is_empty());
        assert_eq!(inputs.owned_state_of(10).len(), 1);
        assert!(inputs.owned_state_of(99).is_empty());
    }

    #[test]
    fn test_vm_outputs_total_by_type() {
        let outputs = VMOutputs::new(
            vec![
                StateAssignment::new(
                    10,
                    SealRef::new(vec![0xAA; 16], Some(1)).unwrap(),
                    600u64.to_le_bytes().to_vec(),
                ),
                StateAssignment::new(
                    10,
                    SealRef::new(vec![0xBB; 16], Some(2)).unwrap(),
                    400u64.to_le_bytes().to_vec(),
                ),
            ],
            vec![],
            vec![],
            None,
        );
        let totals = outputs.total_by_type();
        assert_eq!(totals.get(&10), Some(&1000));
    }

    #[test]
    fn test_decode_integer() {
        assert_eq!(decode_integer(&1000u64.to_le_bytes()), Some(1000));
        assert_eq!(decode_integer(&[1, 2, 3]), None);
        let mut bytes = 42u64.to_le_bytes().to_vec();
        bytes.push(0xFF);
        assert_eq!(decode_integer(&bytes), Some(42));
    }

    #[test]
    fn test_vm_error_display() {
        let err = VMError::InvalidBytecode("bad opcode".to_string());
        assert!(err.to_string().contains("Invalid bytecode"));
        let err = VMError::ExecutionLimitExceeded {
            max_steps: 100,
            actual_steps: 200,
        };
        assert!(err.to_string().contains("200"));
        let err = VMError::SealReplay {
            seal: SealRef::new(vec![1], Some(1)).unwrap(),
        };
        assert!(err.to_string().contains("replay"));
    }
}
