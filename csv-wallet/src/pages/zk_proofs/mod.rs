//! Zero-Knowledge Proof Pages
//!
//! Provides UI for generating and verifying ZK proofs for seal consumption.
//! This is Phase 5 of the CSV protocol - trustless verification without RPC.

pub mod generate;
pub mod verify;

pub use generate::ZkGenerateProof;
pub use verify::ZkVerifyProof;
