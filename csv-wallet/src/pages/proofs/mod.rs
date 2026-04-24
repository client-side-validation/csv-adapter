//! Proof management pages.

pub mod list;
pub mod generate;
pub mod verify;
pub mod verify_cross_chain;

pub use list::Proofs;
pub use generate::GenerateProof;
pub use verify::VerifyProof;
pub use verify_cross_chain::VerifyCrossChainProof;
