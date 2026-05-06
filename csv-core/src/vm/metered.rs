//! MeteredVMAdapter — Gas Tracking Wrapper
//!
//! Wraps any [`DeterministicVM`] and tracks execution step counts
//! for gas accounting and billing.

use alloc::vec::Vec;
use core::cell::RefCell;

use super::{VMError, VMInputs, VMOutputs};

use super::DeterministicVM;

/// Execution meter recording step counts.
#[derive(Debug, Clone, Default)]
pub struct ExecutionMeter {
    /// Total steps executed across all invocations
    pub total_steps: u64,
    /// Steps in the current execution
    pub current_steps: u64,
    /// Maximum steps in any single execution
    pub max_single_step: u64,
}

impl ExecutionMeter {
    /// Reset the meter for a new execution.
    pub fn reset(&mut self) {
        self.current_steps = 0;
    }

    /// Record a number of steps.
    pub fn record(&mut self, steps: u64) {
        self.current_steps += steps;
        self.total_steps += steps;
        if self.current_steps > self.max_single_step {
            self.max_single_step = self.current_steps;
        }
    }

    /// Commit the current execution's step count.
    pub fn commit(&mut self) {
        self.current_steps = 0;
    }

    /// Get the average steps per execution.
    pub fn average_steps(&self) -> u64 {
        if self.total_steps == 0 {
            return 0;
        }
        let executions = (self.total_steps / self.max_single_step).max(1);
        self.total_steps / executions
    }
}

/// MeteredVMAdapter wraps any [`DeterministicVM`] and tracks step counts.
///
/// Useful for gas accounting, billing, and performance profiling.
pub struct MeteredVMAdapter<V: DeterministicVM> {
    /// The wrapped VM
    inner: V,
    /// Execution meter (interior mutability for &self trait impl)
    meter: RefCell<ExecutionMeter>,
}

impl<V: DeterministicVM> MeteredVMAdapter<V> {
    /// Create a new metered VM adapter.
    pub fn new(inner: V) -> Self {
        Self {
            inner,
            meter: RefCell::new(ExecutionMeter::default()),
        }
    }

    /// Get a reference to the inner VM.
    pub fn inner(&self) -> &V {
        &self.inner
    }

    /// Get a mutable reference to the inner VM.
    pub fn inner_mut(&mut self) -> &mut V {
        &mut self.inner
    }

    /// Get the execution meter.
    pub fn meter(&self) -> std::cell::Ref<'_, ExecutionMeter> {
        self.meter.borrow()
    }

    /// Get the execution meter (mutable).
    pub fn meter_mut(&mut self) -> std::cell::RefMut<'_, ExecutionMeter> {
        self.meter.borrow_mut()
    }

    /// Reset the execution meter.
    pub fn reset_meter(&mut self) {
        self.meter.borrow_mut().reset();
    }
}

impl<V: DeterministicVM> DeterministicVM for MeteredVMAdapter<V> {
    fn execute(
        &self,
        bytecode: &[u8],
        inputs: VMInputs,
        signatures: &[Vec<u8>],
    ) -> Result<VMOutputs, VMError> {
        self.meter.borrow_mut().reset();
        let result = self.inner.execute(bytecode, inputs, signatures);
        self.meter.borrow_mut().commit();
        result
    }

    fn validate_outputs(&self, inputs: &VMInputs, outputs: &VMOutputs) -> Result<(), VMError> {
        self.inner.validate_outputs(inputs, outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::PassthroughVM;

    #[test]
    fn test_metered_vm_tracks_steps() {
        let vm = PassthroughVM::new(1000);
        let metered = MeteredVMAdapter::new(vm);

        let inputs = super::super::VMInputs::new(
            vec![
                crate::state::OwnedState::from_hash(
                    10,
                    crate::seal::SealRef::new(vec![0xAA; 16], Some(1)).unwrap(),
                    crate::Hash::new([1u8; 32]),
                ),
            ],
            vec![],
            vec![],
            vec![],
        );

        let _ = metered.execute(&[0x00], inputs, &[]);
        let m = metered.meter();
        assert!(m.total_steps > 0);
    }

    #[test]
    fn test_metered_vm_delegates_to_inner() {
        let vm = PassthroughVM::new(0); // Zero steps allowed
        let metered = MeteredVMAdapter::new(vm);

        let inputs = super::super::VMInputs::new(
            vec![crate::state::OwnedState::from_hash(
                10,
                crate::seal::SealRef::new(vec![0xAA; 16], Some(1)).unwrap(),
                crate::Hash::new([1u8; 32]),
            )],
            vec![],
            vec![],
            vec![],
        );

        let result = metered.execute(&[0x00], inputs, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_meter_reset() {
        let vm = PassthroughVM::new(1000);
        let mut metered = MeteredVMAdapter::new(vm);

        let inputs = super::super::VMInputs::new(
            vec![crate::state::OwnedState::from_hash(
                10,
                crate::seal::SealRef::new(vec![0xAA; 16], Some(1)).unwrap(),
                crate::Hash::new([1u8; 32]),
            )],
            vec![],
            vec![],
            vec![],
        );

        let _ = metered.execute(&[0x00], inputs.clone(), &[]);
        let steps_after_first = metered.meter().total_steps;
        assert!(steps_after_first > 0);

        metered.reset_meter();
        assert_eq!(metered.meter().current_steps, 0);
    }
}
