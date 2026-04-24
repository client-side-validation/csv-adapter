//! Validation pages.

pub mod list;
pub mod consignment;
pub mod proof;
pub mod seal;
pub mod commitment_chain;

pub use list::Validate;
pub use consignment::ValidateConsignment;
pub use proof::ValidateProof;
pub use seal::ValidateSeal;
pub use commitment_chain::ValidateCommitmentChain;
