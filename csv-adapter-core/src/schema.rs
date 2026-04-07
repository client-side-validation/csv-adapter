//! Schema: contract logic definition and transition rules
//!
//! A schema defines the valid state types, transition definitions,
//! and validation rules for a class of contracts. It is the "class"
//! in the OOP analogy — contracts are "instances" of schemas.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::hash::Hash;
use crate::state::StateTypeId;
use crate::transition::Transition;

/// Schema validation errors
#[derive(Debug)]
pub enum SchemaError {
    /// Type ID not defined in schema
    TypeNotFound { type_id: StateTypeId },
    /// Transition ID not defined in schema
    TransitionNotFound { transition_id: u16 },
    /// Transition input type mismatch
    InputTypeMismatch {
        transition_id: u16,
        expected: Vec<StateTypeId>,
        actual: Vec<StateTypeId>,
    },
    /// Transition output type mismatch
    OutputTypeMismatch {
        transition_id: u16,
        expected: Vec<StateTypeId>,
        actual: Vec<StateTypeId>,
    },
}

impl core::fmt::Display for SchemaError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SchemaError::TypeNotFound { type_id } => {
                write!(f, "Type ID {} not found in schema", type_id)
            }
            SchemaError::TransitionNotFound { transition_id } => {
                write!(f, "Transition ID {} not found in schema", transition_id)
            }
            SchemaError::InputTypeMismatch {
                transition_id,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Transition {} input type mismatch: expected {:?}, got {:?}",
                    transition_id, expected, actual
                )
            }
            SchemaError::OutputTypeMismatch {
                transition_id,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Transition {} output type mismatch: expected {:?}, got {:?}",
                    transition_id, expected, actual
                )
            }
        }
    }
}

/// Transition validation errors
#[derive(Debug)]
pub enum TransitionValidationError {
    /// Input state not found
    InputNotFound { type_id: StateTypeId },
    /// Output state type not defined in schema
    OutputTypeNotDefined { type_id: StateTypeId },
    /// Validation script execution failed
    ScriptExecutionFailed(String),
    /// Signature verification failed
    SignatureVerificationFailed,
}

impl core::fmt::Display for TransitionValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TransitionValidationError::InputNotFound { type_id } => {
                write!(f, "Input state type {} not found", type_id)
            }
            TransitionValidationError::OutputTypeNotDefined { type_id } => {
                write!(f, "Output state type {} not defined in schema", type_id)
            }
            TransitionValidationError::ScriptExecutionFailed(msg) => {
                write!(f, "Validation script failed: {}", msg)
            }
            TransitionValidationError::SignatureVerificationFailed => {
                write!(f, "Signature verification failed")
            }
        }
    }
}

/// Schema version
pub const SCHEMA_VERSION: u8 = 1;

/// Data type for state values in the schema
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StateDataType {
    /// Fixed-size binary blob (size in bytes)
    FixedSize(u32),
    /// 64-bit unsigned integer
    Integer64,
    /// 32-bit unsigned integer
    Integer32,
    /// 8-bit unsigned integer
    Integer8,
    /// Arbitrary-size binary blob (validated by script)
    Blob,
    /// 256-bit hash
    Hash256,
}

impl StateDataType {
    /// Get the fixed size if applicable
    pub fn fixed_size(&self) -> Option<u32> {
        match self {
            StateDataType::FixedSize(s) => Some(*s),
            StateDataType::Integer64 => Some(8),
            StateDataType::Integer32 => Some(4),
            StateDataType::Integer8 => Some(1),
            StateDataType::Hash256 => Some(32),
            StateDataType::Blob => None,
        }
    }
}

/// Definition of a global state type in the schema
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalStateType {
    /// Type ID (unique within schema)
    pub type_id: StateTypeId,
    /// Human-readable name
    pub name: String,
    /// Data type
    pub data_type: StateDataType,
    /// Whether this state is homomorphic (supports additive operations)
    pub is_homomorphic: bool,
}

impl GlobalStateType {
    pub fn new(
        type_id: StateTypeId,
        name: impl Into<String>,
        data_type: StateDataType,
        is_homomorphic: bool,
    ) -> Self {
        Self {
            type_id,
            name: name.into(),
            data_type,
            is_homomorphic,
        }
    }
}

/// Definition of an owned state type in the schema
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedStateType {
    /// Type ID (unique within schema)
    pub type_id: StateTypeId,
    /// Human-readable name
    pub name: String,
    /// Data type
    pub data_type: StateDataType,
    /// Whether this state represents a fungible asset (adds to total supply)
    pub is_fungible: bool,
}

impl OwnedStateType {
    pub fn new(
        type_id: StateTypeId,
        name: impl Into<String>,
        data_type: StateDataType,
        is_fungible: bool,
    ) -> Self {
        Self {
            type_id,
            name: name.into(),
            data_type,
            is_fungible,
        }
    }
}

/// Transition definition in the schema
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionDef {
    /// Transition ID (unique within schema)
    pub transition_id: u16,
    /// Human-readable name
    pub name: String,
    /// Expected owned input type IDs
    pub owned_inputs: Vec<StateTypeId>,
    /// Produced owned output type IDs
    pub owned_outputs: Vec<StateTypeId>,
    /// Updated global state type IDs
    pub global_updates: Vec<StateTypeId>,
    /// Validation script (bytecode for the VM)
    pub validation_script: Vec<u8>,
}

impl TransitionDef {
    pub fn new(
        transition_id: u16,
        name: impl Into<String>,
        owned_inputs: Vec<StateTypeId>,
        owned_outputs: Vec<StateTypeId>,
        global_updates: Vec<StateTypeId>,
        validation_script: Vec<u8>,
    ) -> Self {
        Self {
            transition_id,
            name: name.into(),
            owned_inputs,
            owned_outputs,
            global_updates,
            validation_script,
        }
    }
}

/// Contract schema
///
/// A schema defines the rules that govern a class of contracts:
/// what state types exist, what transitions are valid, and how
/// to validate them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schema {
    /// Schema version
    pub version: u8,
    /// Unique schema identifier
    pub schema_id: Hash,
    /// Human-readable name
    pub name: String,
    /// Defined global state types
    pub global_types: Vec<GlobalStateType>,
    /// Defined owned state types
    pub owned_types: Vec<OwnedStateType>,
    /// Defined transition types
    pub transitions: Vec<TransitionDef>,
    /// Root validation script (runs on every transition)
    pub root_script: Vec<u8>,
}

impl Schema {
    /// Create a new schema
    pub fn new(
        schema_id: Hash,
        name: impl Into<String>,
        global_types: Vec<GlobalStateType>,
        owned_types: Vec<OwnedStateType>,
        transitions: Vec<TransitionDef>,
        root_script: Vec<u8>,
    ) -> Self {
        Self {
            version: SCHEMA_VERSION,
            schema_id,
            name: name.into(),
            global_types,
            owned_types,
            transitions,
            root_script,
        }
    }

    /// Compute the schema hash
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();

        hasher.update(b"CSV-SCHEMA-v1");
        hasher.update(&self.version.to_le_bytes());
        hasher.update(self.name.as_bytes());

        // Global types
        hasher.update(&(self.global_types.len() as u64).to_le_bytes());
        for gt in &self.global_types {
            hasher.update(&gt.type_id.to_le_bytes());
            hasher.update(gt.name.as_bytes());
            hasher.update(&(gt.data_type.fixed_size().unwrap_or(0)).to_le_bytes());
            hasher.update(&[gt.is_homomorphic as u8]);
        }

        // Owned types
        hasher.update(&(self.owned_types.len() as u64).to_le_bytes());
        for ot in &self.owned_types {
            hasher.update(&ot.type_id.to_le_bytes());
            hasher.update(ot.name.as_bytes());
            hasher.update(&(ot.data_type.fixed_size().unwrap_or(0)).to_le_bytes());
            hasher.update(&[ot.is_fungible as u8]);
        }

        // Transitions
        hasher.update(&(self.transitions.len() as u64).to_le_bytes());
        for t in &self.transitions {
            hasher.update(&t.transition_id.to_le_bytes());
            hasher.update(t.name.as_bytes());
            hasher.update(&(t.owned_inputs.len() as u64).to_le_bytes());
            for id in &t.owned_inputs {
                hasher.update(&id.to_le_bytes());
            }
            hasher.update(&(t.owned_outputs.len() as u64).to_le_bytes());
            for id in &t.owned_outputs {
                hasher.update(&id.to_le_bytes());
            }
            hasher.update(&(t.validation_script.len() as u64).to_le_bytes());
            hasher.update(&t.validation_script);
        }

        // Root script
        hasher.update(&(self.root_script.len() as u64).to_le_bytes());
        hasher.update(&self.root_script);

        let result = hasher.finalize();
        let mut array = [0u8; 32];
        array.copy_from_slice(&result);
        Hash::new(array)
    }

    /// Get a global state type by ID
    pub fn global_type(&self, type_id: StateTypeId) -> Option<&GlobalStateType> {
        self.global_types.iter().find(|t| t.type_id == type_id)
    }

    /// Get an owned state type by ID
    pub fn owned_type(&self, type_id: StateTypeId) -> Option<&OwnedStateType> {
        self.owned_types.iter().find(|t| t.type_id == type_id)
    }

    /// Get a transition definition by ID
    pub fn transition_def(&self, transition_id: u16) -> Option<&TransitionDef> {
        self.transitions
            .iter()
            .find(|t| t.transition_id == transition_id)
    }

    /// Validate that a type ID is defined in the schema
    pub fn has_type(&self, type_id: StateTypeId) -> bool {
        self.global_type(type_id).is_some() || self.owned_type(type_id).is_some()
    }

    /// Validate that a transition ID is defined in the schema
    pub fn has_transition(&self, transition_id: u16) -> bool {
        self.transition_def(transition_id).is_some()
    }

    /// Validate a transition against this schema
    pub fn validate_transition(&self, transition: &Transition) -> Result<(), SchemaError> {
        let def = self.transition_def(transition.transition_id).ok_or(
            SchemaError::TransitionNotFound {
                transition_id: transition.transition_id,
            },
        )?;

        // Check input types match
        let actual_input_types: Vec<StateTypeId> =
            transition.owned_inputs.iter().map(|i| i.type_id).collect();
        if actual_input_types != def.owned_inputs {
            return Err(SchemaError::InputTypeMismatch {
                transition_id: transition.transition_id,
                expected: def.owned_inputs.clone(),
                actual: actual_input_types,
            });
        }

        // Check output types match
        let actual_output_types: Vec<StateTypeId> =
            transition.owned_outputs.iter().map(|o| o.type_id).collect();
        if actual_output_types != def.owned_outputs {
            return Err(SchemaError::OutputTypeMismatch {
                transition_id: transition.transition_id,
                expected: def.owned_outputs.clone(),
                actual: actual_output_types,
            });
        }

        // Check global update types are defined
        for update in &transition.global_updates {
            if !self.has_type(update.type_id) {
                return Err(SchemaError::TypeNotFound {
                    type_id: update.type_id,
                });
            }
        }

        Ok(())
    }

    /// Create a minimal fungible token schema
    pub fn fungible_token(schema_id: Hash, name: impl Into<String>) -> Self {
        let name = name.into();
        Self::new(
            schema_id,
            name.clone(),
            vec![GlobalStateType::new(
                1,
                "supply",
                StateDataType::Integer64,
                true,
            )],
            vec![OwnedStateType::new(
                10,
                "asset",
                StateDataType::Integer64,
                true,
            )],
            vec![
                // Genesis: no inputs, produces asset outputs
                TransitionDef::new(
                    0,
                    "genesis",
                    vec![],
                    vec![10],
                    vec![1],
                    vec![0x01], // placeholder script
                ),
                // Transfer: consumes asset, produces asset outputs
                TransitionDef::new(
                    1,
                    "transfer",
                    vec![10],
                    vec![10],
                    vec![],
                    vec![0x02], // placeholder script
                ),
                // Burn: consumes asset, no outputs
                TransitionDef::new(
                    2,
                    "burn",
                    vec![10],
                    vec![],
                    vec![1],    // updates supply
                    vec![0x03], // placeholder script
                ),
            ],
            vec![0x00], // minimal root script
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seal::SealRef;
    use crate::state::{StateAssignment, StateRef};

    fn test_schema() -> Schema {
        Schema::fungible_token(Hash::new([1u8; 32]), "TestToken")
    }

    #[test]
    fn test_schema_creation() {
        let s = test_schema();
        assert_eq!(s.version, SCHEMA_VERSION);
        assert_eq!(s.name, "TestToken");
        assert_eq!(s.global_types.len(), 1);
        assert_eq!(s.owned_types.len(), 1);
        assert_eq!(s.transitions.len(), 3);
    }

    #[test]
    fn test_schema_hash() {
        let s = test_schema();
        let hash = s.hash();
        assert_eq!(hash.as_bytes().len(), 32);
    }

    #[test]
    fn test_schema_hash_deterministic() {
        let s1 = test_schema();
        let s2 = test_schema();
        assert_eq!(s1.hash(), s2.hash());
    }

    #[test]
    fn test_schema_hash_differs_by_name() {
        let mut s = test_schema();
        let original = s.hash();
        s.name = "DifferentToken".to_string();
        assert_ne!(s.hash(), original);
    }

    #[test]
    fn test_schema_hash_differs_by_types() {
        let mut s = test_schema();
        let original = s.hash();
        s.global_types.push(GlobalStateType::new(
            99,
            "extra",
            StateDataType::Integer8,
            false,
        ));
        assert_ne!(s.hash(), original);
    }

    #[test]
    fn test_global_type_lookup() {
        let s = test_schema();
        let gt = s.global_type(1);
        assert!(gt.is_some());
        assert_eq!(gt.unwrap().name, "supply");
        assert!(s.global_type(99).is_none());
    }

    #[test]
    fn test_owned_type_lookup() {
        let s = test_schema();
        let ot = s.owned_type(10);
        assert!(ot.is_some());
        assert_eq!(ot.unwrap().name, "asset");
        assert!(s.owned_type(99).is_none());
    }

    #[test]
    fn test_transition_def_lookup() {
        let s = test_schema();
        let td = s.transition_def(1);
        assert!(td.is_some());
        assert_eq!(td.unwrap().name, "transfer");
        assert!(s.transition_def(99).is_none());
    }

    #[test]
    fn test_has_type() {
        let s = test_schema();
        assert!(s.has_type(1)); // global
        assert!(s.has_type(10)); // owned
        assert!(!s.has_type(99));
    }

    #[test]
    fn test_has_transition() {
        let s = test_schema();
        assert!(s.has_transition(0)); // genesis
        assert!(s.has_transition(1)); // transfer
        assert!(s.has_transition(2)); // burn
        assert!(!s.has_transition(99));
    }

    #[test]
    fn test_state_data_type_sizes() {
        assert_eq!(StateDataType::Integer64.fixed_size(), Some(8));
        assert_eq!(StateDataType::Integer32.fixed_size(), Some(4));
        assert_eq!(StateDataType::Integer8.fixed_size(), Some(1));
        assert_eq!(StateDataType::Hash256.fixed_size(), Some(32));
        assert_eq!(StateDataType::Blob.fixed_size(), None);
        assert_eq!(StateDataType::FixedSize(128).fixed_size(), Some(128));
    }

    #[test]
    fn test_schema_serialization_roundtrip() {
        let s = test_schema();
        let bytes = bincode::serialize(&s).unwrap();
        let restored: Schema = bincode::deserialize(&bytes).unwrap();
        assert_eq!(s, restored);
        assert_eq!(s.hash(), restored.hash());
    }

    #[test]
    fn test_fungible_token_schema_structure() {
        let s = test_schema();

        // Genesis should have no inputs
        let genesis = s.transition_def(0).unwrap();
        assert!(genesis.owned_inputs.is_empty());
        assert_eq!(genesis.owned_outputs, vec![10]);

        // Transfer should consume and produce assets
        let transfer = s.transition_def(1).unwrap();
        assert_eq!(transfer.owned_inputs, vec![10]);
        assert_eq!(transfer.owned_outputs, vec![10]);

        // Burn should consume asset but not produce any
        let burn = s.transition_def(2).unwrap();
        assert_eq!(burn.owned_inputs, vec![10]);
        assert!(burn.owned_outputs.is_empty());
    }

    #[test]
    fn test_validate_transition_valid() {
        let s = test_schema();

        // Valid transfer: input type 10, output type 10
        let transfer = Transition::new(
            1,
            vec![StateRef::new(10, Hash::new([1u8; 32]), 0)],
            vec![StateAssignment::new(
                10,
                SealRef::new(vec![0xAA; 16], Some(1)).unwrap(),
                500u64.to_le_bytes().to_vec(),
            )],
            vec![],
            vec![],
            vec![0x02],
            vec![],
        );
        assert!(s.validate_transition(&transfer).is_ok());
    }

    #[test]
    fn test_validate_transition_unknown_transition_id() {
        let s = test_schema();
        let bad = Transition::new(99, vec![], vec![], vec![], vec![], vec![], vec![]);
        let err = s.validate_transition(&bad).unwrap_err();
        assert!(matches!(err, SchemaError::TransitionNotFound { .. }));
    }

    #[test]
    fn test_validate_transition_input_type_mismatch() {
        let s = test_schema();

        // Transfer expects input type 10, but we give type 99
        let bad = Transition::new(
            1,
            vec![StateRef::new(99, Hash::new([1u8; 32]), 0)],
            vec![],
            vec![],
            vec![],
            vec![0x02],
            vec![],
        );
        let err = s.validate_transition(&bad).unwrap_err();
        assert!(matches!(err, SchemaError::InputTypeMismatch { .. }));
    }

    #[test]
    fn test_validate_transition_output_type_mismatch() {
        let s = test_schema();

        // Transfer expects output type 10, but we give type 99
        let bad = Transition::new(
            1,
            vec![StateRef::new(10, Hash::new([1u8; 32]), 0)],
            vec![StateAssignment::new(
                99,
                SealRef::new(vec![0xAA; 16], Some(1)).unwrap(),
                500u64.to_le_bytes().to_vec(),
            )],
            vec![],
            vec![],
            vec![0x02],
            vec![],
        );
        let err = s.validate_transition(&bad).unwrap_err();
        assert!(matches!(err, SchemaError::OutputTypeMismatch { .. }));
    }
}
