//! PassthroughVM — Testing Shim for VM Pipeline
//!
//! Passes inputs through as outputs with conservation-of-supply validation.
//! Used for testing the proof pipeline without a full VM implementation.
//!
//! This module is intended for testing only.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::{decode_integer, VMError, VMInputs, VMOutputs};
use crate::state::{StateAssignment, StateTypeId};

use super::DeterministicVM;

/// PassthroughVM: a basic implementation that passes inputs through as outputs.
///
/// Validates that total input value >= total output value for each type ID
/// (conservation of supply).
pub struct PassthroughVM {
    /// Maximum execution steps before loop detection triggers.
    max_steps: u64,
}

impl PassthroughVM {
    /// Create a new PassthroughVM with the given step limit.
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
        // Check step limit
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
                    state.seal.clone(),
                    state.data.clone(),
                )
            })
            .collect();

        Ok(VMOutputs::new(
            owned_outputs,
            Vec::new(),
            inputs.metadata.clone(),
            None,
        ))
    }

    fn validate_outputs(&self, inputs: &VMInputs, outputs: &VMOutputs) -> Result<(), VMError> {
        let mut input_totals: BTreeMap<StateTypeId, u64> = BTreeMap::new();
        for state in &inputs.owned_inputs {
            let value = decode_integer(&state.data).unwrap_or(0);
            *input_totals.entry(state.type_id).or_insert(0) += value;
        }
        for state in &inputs.global_state {
            let value = decode_integer(&state.data).unwrap_or(0);
            *input_totals.entry(state.type_id).or_insert(0) += value;
        }

        let output_totals = outputs.total_by_type();

        for (type_id, output_total) in &output_totals {
            let input_total = input_totals.get(type_id).copied().unwrap_or(0);
            if *output_total > input_total {
                return Err(VMError::InconsistentOutput(format!(
                    "Output total {output_total} exceeds input total {input_total} for type {type_id}"
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Hash;

    fn test_inputs() -> VMInputs {
        VMInputs::new(
            vec![OwnedState::from_hash(
                10,
                crate::seal::SealRef::new(vec![0xAA; 16], Some(1)).unwrap(),
                Hash::new([1u8; 32]),
            )],
            vec![GlobalState::from_hash(1, Hash::new([100u8; 32]))],
            vec![Metadata::from_string("memo", "test")],
            vec![0x01, 0x02, 0x03],
        )
    }

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
        let vm = PassthroughVM::new(0);
        let inputs = test_inputs();
        let result = vm.execute(&[0x01], inputs, &[]);
        assert!(result.is_err());
    }
}
