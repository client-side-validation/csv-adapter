//! Celestia Adapter for CSV (Client-Side Validation)
//!
//! This adapter implements the AnchorLayer trait for Celestia,
//! using namespaced blob IDs as seals and PayForBlob for commitment publication.

#![warn(missing_docs)]
#![allow(missing_docs)]
#![allow(dead_code)]

pub mod adapter;
pub mod blob;
pub mod config;
pub mod error;
pub mod rpc;
pub mod signatures;
pub mod types;

pub use adapter::CelestiaAnchorLayer;
pub use config::CelestiaConfig;
pub use types::{CelestiaSealRef, CelestiaAnchorRef, CelestiaFinalityProof, CelestiaInclusionProof};
pub use rpc::{CelestiaRpc, MockCelestiaRpc};
pub use blob::{BlobSubmitter, DasVerifier};
