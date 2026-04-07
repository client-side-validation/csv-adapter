//! Deterministic VM trait for CSV contract execution
//!
//! This trait defines the interface that any deterministic VM must implement
//! to serve as the execution engine for CSV contract transitions.
//!
//! The core invariant: the same bytecode + inputs must always produce
//! the same outputs, regardless of the executing environment.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::seal::SealRef;
use crate::state::{GlobalState, Metadata, OwnedState, StateAssignment, StateRef, StateTypeId};

/// Errors that can occur during VM execution
#[derive(Debug)]
pub enum VMError {
    /// Bytecode is malformed or invalid
    InvalidBytecode(String),
    /// Execution ran out of steps (loop detection / gas limit)
    ExecutionLimitExceeded { max_steps: u64, actual_steps: u64 },
    /// A state input referenced in the transition was not found
    StateNotFound { state_ref: StateRef },
    /// The VM produced inconsistent output (e.g., negative supply)
    InconsistentOutput(String),
    /// Signature verification failed
    InvalidSignature(String),
    /// Seal was already consumed (replay detected)
    SealReplay { seal: SealRef },
    /// Schema validation failed
    SchemaViolation(String),
    /// Generic execution error
    ExecutionError(String),
}

impl core::fmt::Display for VMError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VMError::InvalidBytecode(msg) => write!(f, "Invalid bytecode: {}", msg),
            VMError::ExecutionLimitExceeded {
                max_steps,
                actual_steps,
            } => {
                write!(
                    f,
                    "Execution limit exceeded: {} steps (max {})",
                    actual_steps, max_steps
                )
            }
            VMError::StateNotFound { state_ref } => {
                write!(f, "State not found: {:?}", state_ref)
            }
            VMError::InconsistentOutput(msg) => write!(f, "Inconsistent output: {}", msg),
            VMError::InvalidSignature(msg) => write!(f, "Invalid signature: {}", msg),
            VMError::SealReplay { seal } => write!(f, "Seal replay detected: {:?}", seal),
            VMError::SchemaViolation(msg) => write!(f, "Schema violation: {}", msg),
            VMError::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
        }
    }
}

/// Input state for VM execution
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VMInputs {
    /// Owned state being consumed (resolved StateRef → actual state)
    pub owned_inputs: Vec<OwnedState>,
    /// Current global state values
    pub global_state: Vec<GlobalState>,
    /// Transition metadata
    pub metadata: Vec<Metadata>,
    /// Seal data being consumed (the seal that authorized this transition)
    pub seal_data: Vec<u8>,
}

impl VMInputs {
    /// Create new VM inputs
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

    /// Look up a global state by type ID
    pub fn global_state_of(&self, type_id: StateTypeId) -> Vec<&GlobalState> {
        self.global_state
            .iter()
            .filter(|s| s.type_id == type_id)
            .collect()
    }

    /// Look up owned states by type ID
    pub fn owned_state_of(&self, type_id: StateTypeId) -> Vec<&OwnedState> {
        self.owned_inputs
            .iter()
            .filter(|s| s.type_id == type_id)
            .collect()
    }
}

/// Output state from VM execution
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

impl VMOutputs {
    /// Create new VM outputs
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

    /// Get total value by type ID (for fungible asset validation)
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

/// The DeterministicVM trait defines the interface for CSV contract execution.
///
/// Any VM implementing this trait must guarantee:
/// - The same bytecode + inputs always produce the same outputs
/// - No access to external state (time, network, random, etc.)
/// - Bounded execution (no infinite loops)
pub trait DeterministicVM {
    /// Execute a transition's bytecode with the given inputs.
    ///
    /// # Arguments
    /// * `bytecode` - The validation script (e.g., AluVM bytecode)
    /// * `inputs` - The input state being consumed
    /// * `signatures` - Authorizing signatures
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

/// PassthroughVM: a stub implementation that passes inputs through as outputs.
///
/// This is used for testing the proof pipeline without a full VM.
/// It validates that total input value >= total output value for
/// each type ID (conservation of supply).
pub struct PassthroughVM {
    /// Maximum execution steps (for loop detection)
    pub max_steps: u64,
}

impl PassthroughVM {
    /// Create a new PassthroughVM with the given step limit
    pub fn new(max_steps: u64) -> Self {
        Self { max_steps }
    }
}

impl Default for PassthroughVM {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl DeterministicVM for PassthroughVM {
    fn execute(
        &self,
        _bytecode: &[u8],
        inputs: VMInputs,
        _signatures: &[Vec<u8>],
    ) -> Result<VMOutputs, VMError> {
        // Simulate execution: pass inputs through as outputs
        // In a real VM, this would execute the bytecode

        // Check step limit (simulated)
        let steps = inputs.owned_inputs.len() as u64 + 1;
        if steps > self.max_steps {
            return Err(VMError::ExecutionLimitExceeded {
                max_steps: self.max_steps,
                actual_steps: steps,
            });
        }

        // Owned outputs mirror the input owned states
        let owned_outputs: Vec<StateAssignment> = inputs
            .owned_inputs
            .iter()
            .map(|state| {
                StateAssignment::new(
                    state.type_id,
                    state.seal.clone(), // Same seal (pass-through)
                    state.data.clone(),
                )
            })
            .collect();

        Ok(VMOutputs::new(
            owned_outputs,
            Vec::new(), // No global updates
            inputs.metadata.clone(),
            None, // No next seal
        ))
    }

    fn validate_outputs(&self, inputs: &VMInputs, outputs: &VMOutputs) -> Result<(), VMError> {
        // Conservation of supply: output total <= input total for each type
        let mut input_totals: BTreeMap<StateTypeId, u64> = BTreeMap::new();
        for state in &inputs.owned_inputs {
            let value = decode_integer(&state.data).unwrap_or(0);
            *input_totals.entry(state.type_id).or_insert(0) += value;
        }
        // Add input global state
        for state in &inputs.global_state {
            let value = decode_integer(&state.data).unwrap_or(0);
            *input_totals.entry(state.type_id).or_insert(0) += value;
        }

        let output_totals = outputs.total_by_type();

        for (type_id, output_total) in &output_totals {
            let input_total = input_totals.get(type_id).copied().unwrap_or(0);
            if *output_total > input_total {
                return Err(VMError::InconsistentOutput(format!(
                    "Output total {} exceeds input total {} for type {}",
                    output_total, input_total, type_id
                )));
            }
        }

        Ok(())
    }
}

/// Decode a state data value as a u64 (little-endian)
fn decode_integer(data: &[u8]) -> Option<u64> {
    if data.len() < 8 {
        return None;
    }
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[..8]);
    Some(u64::from_le_bytes(bytes))
}

/// Execute a transition through the VM and validate
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

    // ─────────────────────────────────────────────
    // VMInputs tests
    // ─────────────────────────────────────────────

    #[test]
    fn test_vm_inputs_creation() {
        let inputs = test_inputs();
        assert_eq!(inputs.owned_inputs.len(), 1);
        assert_eq!(inputs.global_state.len(), 1);
    }

    #[test]
    fn test_vm_inputs_global_state_lookup() {
        let inputs = test_inputs();
        let states = inputs.global_state_of(1);
        assert_eq!(states.len(), 1);
        assert!(inputs.global_state_of(99).is_empty());
    }

    #[test]
    fn test_vm_inputs_owned_state_lookup() {
        let inputs = test_inputs();
        let states = inputs.owned_state_of(10);
        assert_eq!(states.len(), 1);
        assert!(inputs.owned_state_of(99).is_empty());
    }

    // ─────────────────────────────────────────────
    // VMOutputs tests
    // ─────────────────────────────────────────────

    #[test]
    fn test_vm_outputs_creation() {
        let outputs = VMOutputs::new(
            vec![StateAssignment::new(
                10,
                SealRef::new(vec![0xAA; 16], Some(1)).unwrap(),
                1000u64.to_le_bytes().to_vec(),
            )],
            vec![GlobalState::from_hash(1, Hash::new([100u8; 32]))],
            vec![],
            None,
        );
        assert_eq!(outputs.owned_outputs.len(), 1);
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

    // ─────────────────────────────────────────────
    // PassthroughVM tests
    // ─────────────────────────────────────────────

    #[test]
    fn test_passthrough_vm_basic() {
        let vm = PassthroughVM::default();
        let inputs = test_inputs();
        let outputs = vm.execute(&[0x01], inputs.clone(), &[]).unwrap();
        assert_eq!(outputs.owned_outputs.len(), inputs.owned_inputs.len());
    }

    #[test]
    fn test_passthrough_vm_validation_conservation() {
        let vm = PassthroughVM::default();
        let inputs = test_inputs();
        let outputs = vm.execute(&[0x01], inputs.clone(), &[]).unwrap();
        vm.validate_outputs(&inputs, &outputs).unwrap();
    }

    #[test]
    fn test_passthrough_vm_execution_limit() {
        let vm = PassthroughVM::new(0); // Zero steps allowed
        let inputs = test_inputs();
        let result = vm.execute(&[0x01], inputs, &[]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VMError::ExecutionLimitExceeded { .. }
        ));
    }

    #[test]
    fn test_execute_transition_valid() {
        let vm = PassthroughVM::default();
        let inputs = test_inputs();
        let outputs = execute_transition(&vm, &[0x01], inputs.clone(), &[]).unwrap();
        assert_eq!(outputs.owned_outputs.len(), 1);
    }

    // ─────────────────────────────────────────────
    // decode_integer tests
    // ─────────────────────────────────────────────

    #[test]
    fn test_decode_integer_valid() {
        let bytes = 1000u64.to_le_bytes();
        assert_eq!(decode_integer(&bytes), Some(1000));
    }

    #[test]
    fn test_decode_integer_too_short() {
        assert_eq!(decode_integer(&[1, 2, 3]), None);
    }

    #[test]
    fn test_decode_integer_extra_bytes() {
        let mut bytes = 42u64.to_le_bytes().to_vec();
        bytes.push(0xFF);
        assert_eq!(decode_integer(&bytes), Some(42));
    }

    // ─────────────────────────────────────────────
    // VMError display tests
    // ─────────────────────────────────────────────

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

    // ─────────────────────────────────────────────
    // Integration: full transition through VM
    // ─────────────────────────────────────────────

    #[test]
    fn test_full_transition_with_multiple_inputs() {
        let vm = PassthroughVM::new(100);

        let inputs = VMInputs::new(
            vec![
                OwnedState::from_hash(
                    10,
                    SealRef::new(vec![0xAA; 16], Some(1)).unwrap(),
                    Hash::new([1u8; 32]),
                ),
                OwnedState::from_hash(
                    10,
                    SealRef::new(vec![0xBB; 16], Some(2)).unwrap(),
                    Hash::new([2u8; 32]),
                ),
            ],
            vec![GlobalState::from_hash(1, Hash::new([100u8; 32]))],
            vec![Metadata::from_string("memo", "multi-input transition")],
            vec![0xDE, 0xAD],
        );

        let outputs = execute_transition(&vm, &[0x01, 0x02], inputs, &[]).unwrap();
        assert_eq!(outputs.owned_outputs.len(), 2);
    }
}
