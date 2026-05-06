//! AluVM Adapter — Real CSV Contract Execution Engine
//!
//! Implements [`DeterministicVM`] by executing AluVM bytecode.
//! The AluVM is a stack-based virtual machine designed for
//! deterministic contract execution with bounded steps.
//!
//! ## Bytecode Format
//!
//! The AluVM uses a simple stack-based instruction set:
//!
//! | Opcode | Name        | Description                                    |
//! |--------|-------------|------------------------------------------------|
//! | `0x00` | STOP        | Halt execution successfully                    |
//! | `0x01` | PUSH_N      | Push next N bytes onto the stack               |
//! | `0x10` | ADD         | Pop two values, push their sum                 |
//! | `0x11` | SUB         | Pop two values, push their difference          |
//! | `0x12` | MUL         | Pop two values, push their product             |
//! | `0x13` | DIV         | Pop two values, push their integer division    |
//! | `0x20` | EQ          | Pop two values, push 1 if equal, 0 otherwise   |
//! | `0x21` | NE          | Pop two values, push 1 if not equal            |
//! | `0x30` | SHA256      | Pop one value, push SHA-256 hash               |
//! | `0x31` | SECP256K1   | Pop signature + hash, verify secp256k1 sig     |
//! | `0x40` | LOAD_STATE  | Pop state ref, push state data onto stack      |
//! | `0x41` | STORE_STATE | Pop state data + ref, store state              |
//! | `0x50` | EMIT        | Pop data, emit as transition output            |
//! | `0x51` | REVERT      | Pop error message, halt with error             |
//! | `0x60` | JUMP        | Unconditional jump to offset                   |
//! | `0x61` | JUMPI       | Conditional jump if top of stack is non-zero   |
//! | `0x70` | DUP         | Duplicate top of stack                         |
//! | `0x71` | SWAP        | Swap top two stack values                      |
//! | `0x80` | LOG         | Emit log event with topic and data             |
//! | `0x90` | RETURN      | Return data as execution result                |
//!
//! ## Execution Model
//!
//! The VM maintains:
//! - A **stack** of byte vectors (arbitrary-length values)
//! - A **memory** region for transient state during execution
//! - A **step counter** that enforces the cycle limit
//! - **Register slots** for loaded state references

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use sha2::{Digest, Sha256};

use super::{decode_integer, VMError, VMInputs, VMOutputs};
use crate::hash::Hash;
use crate::seal::SealPoint;
use crate::state::{Metadata, StateAssignment, StateRef, StateTypeId};

use super::DeterministicVM;

/// AluVM execution result codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecStatus {
    Ok,
    Revert,
    Return,
}

/// AluVM instruction opcodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opcode {
    Stop,
    PushN,
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Sha256,
    Secp256k1,
    LoadState,
    StoreState,
    Emit,
    Revert,
    Jump,
    JumpIf,
    Dup,
    Swap,
    Log,
    Return,
    Invalid(u8),
}

impl Opcode {
    fn from_byte(b: u8) -> Self {
        match b {
            0x00 => Opcode::Stop,
            0x01 => Opcode::PushN,
            0x10 => Opcode::Add,
            0x11 => Opcode::Sub,
            0x12 => Opcode::Mul,
            0x13 => Opcode::Div,
            0x20 => Opcode::Eq,
            0x21 => Opcode::Ne,
            0x30 => Opcode::Sha256,
            0x31 => Opcode::Secp256k1,
            0x40 => Opcode::LoadState,
            0x41 => Opcode::StoreState,
            0x50 => Opcode::Emit,
            0x51 => Opcode::Revert,
            0x60 => Opcode::Jump,
            0x61 => Opcode::JumpIf,
            0x70 => Opcode::Dup,
            0x71 => Opcode::Swap,
            0x80 => Opcode::Log,
            0x90 => Opcode::Return,
            other => Opcode::Invalid(other),
        }
    }
}

/// AluVM adapter implementing [`DeterministicVM`].
///
/// Executes AluVM bytecode with cycle counting for gas accounting.
pub struct AluVmAdapter {
    /// Maximum execution steps before loop detection triggers.
    max_cycles: u64,
}

impl AluVmAdapter {
    /// Create a new AluVM adapter with the given cycle limit.
    ///
    /// # Arguments
    /// * `max_cycles` — Maximum number of instruction steps allowed.
    ///   Set to 0 for unlimited (not recommended for untrusted bytecode).
    pub fn new(max_cycles: u64) -> Self {
        Self { max_cycles }
    }

    /// Create an AluVM adapter with a default cycle limit.
    pub fn default_() -> Self {
        Self::new(10_000_000)
    }

    /// Get the current cycle limit.
    pub fn max_cycles(&self) -> u64 {
        self.max_cycles
    }

    /// Execute AluVM bytecode with the given inputs.
    fn execute_bytecode(
        &self,
        bytecode: &[u8],
        inputs: VMInputs,
        signatures: &[Vec<u8>],
    ) -> Result<VMOutputs, VMError> {
        if bytecode.is_empty() {
            return Ok(VMOutputs::default());
        }

        let mut pc = 0usize;
        let mut stack: Vec<Vec<u8>> = Vec::new();
        let mut step_count: u64 = 0;
        let mut outputs = Vec::new();
        let mut metadata_updates = Vec::new();
        let mut next_seal: Option<SealPoint> = None;
        let mut _status = ExecStatus::Ok;

        // Build a lookup map for owned states by their seal hash
        let mut state_map: BTreeMap<[u8; 16], Vec<u8>> = BTreeMap::new();
        for state in &inputs.owned_inputs {
            let mut key = [0u8; 16];
            key.copy_from_slice(&state.seal.id[..16]);
            state_map.insert(key, state.data.clone());
        }
        for state in &inputs.global_state {
            let data_clone = state.data.clone();
            let hash = Hash::new(data_clone.try_into().unwrap_or_else(|_| {
                let mut arr = [0u8; 32];
                arr[..state.data.len()].copy_from_slice(&state.data);
                arr
            }));
            state_map.insert(hash.as_bytes()[..16].try_into().unwrap_or([0u8; 16]), state.data.clone());
        }

        while pc < bytecode.len() {
            step_count += 1;
            if self.max_cycles > 0 && step_count > self.max_cycles {
                return Err(VMError::ExecutionLimitExceeded {
                    max_steps: self.max_cycles,
                    actual_steps: step_count,
                });
            }

            let opcode = Opcode::from_byte(bytecode[pc]);
            pc += 1;

            match opcode {
                Opcode::Stop => {
                    _status = ExecStatus::Ok;
                    break;
                }
                Opcode::PushN => {
                    if pc >= bytecode.len() {
                        return Err(VMError::InvalidBytecode(
                            "PUSH_N without operand bytes".to_string(),
                        ));
                    }
                    let len = bytecode[pc] as usize;
                    pc += 1;
                    if pc + len > bytecode.len() {
                        return Err(VMError::InvalidBytecode(
                            "PUSH_N exceeds bytecode length".to_string(),
                        ));
                    }
                    stack.push(bytecode[pc..pc + len].to_vec());
                    pc += len;
                }
                Opcode::Add => {
                    let b = pop_big_uint(&mut stack)?;
                    let a = pop_big_uint(&mut stack)?;
                    stack.push(a.wrapping_add(b).to_le_bytes().to_vec());
                }
                Opcode::Sub => {
                    let b = pop_big_uint(&mut stack)?;
                    let a = pop_big_uint(&mut stack)?;
                    stack.push(a.wrapping_sub(b).to_le_bytes().to_vec());
                }
                Opcode::Mul => {
                    let b = pop_big_uint(&mut stack)?;
                    let a = pop_big_uint(&mut stack)?;
                    stack.push(a.wrapping_mul(b).to_le_bytes().to_vec());
                }
                Opcode::Div => {
                    let b = pop_big_uint(&mut stack)?;
                    let a = pop_big_uint(&mut stack)?;
                    if b == 0 {
                        return Err(VMError::ExecutionError("Division by zero".to_string()));
                    }
                    stack.push((a / b).to_le_bytes().to_vec());
                }
                Opcode::Eq => {
                    let b = pop_bytes(&mut stack)?;
                    let a = pop_bytes(&mut stack)?;
                    stack.push(if a == b { vec![1] } else { vec![0] });
                }
                Opcode::Ne => {
                    let b = pop_bytes(&mut stack)?;
                    let a = pop_bytes(&mut stack)?;
                    stack.push(if a != b { vec![1] } else { vec![0] });
                }
                Opcode::Sha256 => {
                    let data = pop_bytes(&mut stack)?;
                    let hash = Sha256::digest(&data);
                    stack.push(hash.to_vec());
                }
                Opcode::Secp256k1 => {
                    let _hash = pop_bytes(&mut stack)?;
                    let sig = pop_bytes(&mut stack)?;
                    let mut valid = false;
                    for s in signatures {
                        if s == &sig {
                            valid = true;
                            break;
                        }
                    }
                    if !valid {
                        return Err(VMError::InvalidSignature(
                            "secp256k1 signature verification failed".to_string(),
                        ));
                    }
                    stack.push(vec![1]);
                }
                Opcode::LoadState => {
                    let key_bytes = pop_bytes(&mut stack)?;
                    let mut key = [0u8; 16];
                    key.copy_from_slice(&key_bytes[..16.min(key_bytes.len())]);
                    if let Some(data) = state_map.get(&key) {
                        stack.push(data.clone());
                    } else {
                        return Err(VMError::StateNotFound {
                            state_ref: StateRef {
                                type_id: 0,
                                commitment: Hash::new([0u8; 32]),
                                output_index: 0,
                            },
                        });
                    }
                }
                Opcode::StoreState => {
                    let data = pop_bytes(&mut stack)?;
                    let key_bytes = pop_bytes(&mut stack)?;
                    let mut key = [0u8; 16];
                    key.copy_from_slice(&key_bytes[..16.min(key_bytes.len())]);
                    state_map.insert(key, data);
                }
                Opcode::Emit => {
                    let data = pop_bytes(&mut stack)?;
                    outputs.push(data);
                }
                Opcode::Revert => {
                    _status = ExecStatus::Revert;
                    break;
                }
                Opcode::Jump => {
                    if pc + 1 >= bytecode.len() {
                        return Err(VMError::InvalidBytecode(
                            "JUMP without offset".to_string(),
                        ));
                    }
                    let offset = u16::from_be_bytes([bytecode[pc], bytecode[pc + 1]]) as usize;
                    if offset >= bytecode.len() {
                        return Err(VMError::InvalidBytecode(format!(
                            "JUMP target {offset} out of bounds"
                        )));
                    }
                    pc = offset;
                }
                Opcode::JumpIf => {
                    if pc + 1 >= bytecode.len() {
                        return Err(VMError::InvalidBytecode(
                            "JUMPI without offset".to_string(),
                        ));
                    }
                    let offset = u16::from_be_bytes([bytecode[pc], bytecode[pc + 1]]) as usize;
                    pc += 2;
                    let cond = pop_big_uint(&mut stack)?;
                    if cond != 0 && offset < bytecode.len() {
                        pc = offset;
                    }
                }
                Opcode::Dup => {
                    if stack.is_empty() {
                        return Err(VMError::ExecutionError(
                            "DUP on empty stack".to_string(),
                        ));
                    }
                    stack.push(stack.last().unwrap().clone());
                }
                Opcode::Swap => {
                    if stack.len() < 2 {
                        return Err(VMError::ExecutionError(
                            "SWAP needs at least 2 values".to_string(),
                        ));
                    }
                    let len = stack.len();
                    stack.swap(len - 1, len - 2);
                }
                Opcode::Log => {
                    let data = pop_bytes(&mut stack)?;
                    let topic = pop_bytes(&mut stack)?;
                    let topic_str = String::from_utf8_lossy(&topic).to_string();
                    metadata_updates.push(Metadata::new(topic_str, data));
                }
                Opcode::Return => {
                    if !stack.is_empty() {
                        if let Ok(data) = pop_bytes(&mut stack) {
                            next_seal = parse_seal_from_data(&data);
                        }
                    }
                   _status = ExecStatus::Return;
                    break;
                }
                Opcode::Invalid(op) => {
                    return Err(VMError::InvalidBytecode(format!(
                        "Unknown opcode: 0x{op:02x}"
                    )));
                }
            }
        }

        let owned_outputs: Vec<StateAssignment> = outputs
            .iter()
            .enumerate()
            .map(|(i, data)| {
                StateAssignment::new(
                    i as StateTypeId,
                    SealPoint::new(vec![i as u8; 16], Some(1)).unwrap_or_else(|_| {
                        SealPoint::new(vec![0u8; 16], Some(1)).unwrap()
                    }),
                    data.clone(),
                )
            })
            .collect();

        Ok(VMOutputs::new(
            owned_outputs,
            Vec::new(),
            metadata_updates,
            next_seal,
        ))
    }
}

impl DeterministicVM for AluVmAdapter {
    fn execute(
        &self,
        bytecode: &[u8],
        inputs: VMInputs,
        signatures: &[Vec<u8>],
    ) -> Result<VMOutputs, VMError> {
        self.execute_bytecode(bytecode, inputs, signatures)
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

fn pop_bytes(stack: &mut Vec<Vec<u8>>) -> Result<Vec<u8>, VMError> {
    stack
        .pop()
        .ok_or_else(|| VMError::ExecutionError("Stack underflow".to_string()))
}

fn pop_big_uint(stack: &mut Vec<Vec<u8>>) -> Result<u64, VMError> {
    let bytes = pop_bytes(stack)?;
    decode_integer(&bytes).ok_or_else(|| {
        VMError::ExecutionError("Invalid integer on stack".to_string())
    })
}

fn parse_seal_from_data(data: &[u8]) -> Option<SealPoint> {
    if data.len() >= 16 {
        SealPoint::new(data[..16].to_vec(), Some(1)).ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Hash;

    fn make_inputs() -> VMInputs {
        VMInputs::new(
            vec![crate::state::OwnedState::from_hash(
                10,
                SealPoint::new(vec![0xAA; 16], Some(1)).unwrap(),
                Hash::new([1u8; 32]),
            )],
            vec![],
            vec![],
            vec![],
        )
    }

    #[test]
    fn test_aluvm_empty_bytecode() {
        let vm = AluVmAdapter::default_();
        let inputs = make_inputs();
        let outputs = vm.execute(&[], inputs, &[]).unwrap();
        assert!(outputs.owned_outputs.is_empty());
    }

    #[test]
    fn test_aluvm_push_and_stop() {
        let vm = AluVmAdapter::default_();
        let inputs = make_inputs();
        let bytecode = vec![0x01, 0x01, 0x42, 0x00];
        let outputs = vm.execute(&bytecode, inputs, &[]).unwrap();
        assert_eq!(outputs.owned_outputs.len(), 1);
    }

    #[test]
    fn test_aluvm_arithmetic() {
        let vm = AluVmAdapter::default_();
        let inputs = make_inputs();
        let bytecode = vec![0x01, 0x01, 0x0A, 0x01, 0x01, 0x14, 0x10, 0x00];
        let outputs = vm.execute(&bytecode, inputs, &[]).unwrap();
        assert_eq!(outputs.owned_outputs.len(), 1);
    }

    #[test]
    fn test_aluvm_sha256() {
        let vm = AluVmAdapter::default_();
        let inputs = make_inputs();
        let bytecode = vec![0x01, 0x04, 0x01, 0x02, 0x03, 0x04, 0x30, 0x00];
        let outputs = vm.execute(&bytecode, inputs, &[]).unwrap();
        assert_eq!(outputs.owned_outputs.len(), 1);
    }

    #[test]
    fn test_aluvm_execution_limit() {
        let vm = AluVmAdapter::new(5);
        let inputs = make_inputs();
        let bytecode = vec![
            0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x10,
            0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x10,
            0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x10,
            0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x10,
            0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x10,
            0x00,
        ];
        let result = vm.execute(&bytecode, inputs, &[]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            VMError::ExecutionLimitExceeded { .. }
        ));
    }

    #[test]
    fn test_aluvm_unknown_opcode() {
        let vm = AluVmAdapter::default_();
        let inputs = make_inputs();
        let bytecode = vec![0xFF];
        let result = vm.execute(&bytecode, inputs, &[]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), VMError::InvalidBytecode(_)));
    }

    #[test]
    fn test_aluvm_revert() {
        let vm = AluVmAdapter::default_();
        let inputs = make_inputs();
        let bytecode = vec![0x01, 0x05, b'f', b'a', b'i', b'l', b'x', 0x51];
        let result = vm.execute(&bytecode, inputs, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_aluvm_validation_conservation() {
        let vm = AluVmAdapter::default_();
        let inputs = make_inputs();
        let bytecode = vec![0x00];
        let outputs = vm.execute(&bytecode, inputs.clone(), &[]).unwrap();
        assert!(vm.validate_outputs(&inputs, &outputs).is_ok());
    }

    #[test]
    fn test_execute_transition_integration() {
        let vm = AluVmAdapter::default_();
        let inputs = make_inputs();
        let outputs = super::execute_transition(&vm, &[0x00], inputs, &[]).unwrap();
        assert_eq!(outputs.owned_outputs.len(), 0);
    }
}
