#![no_main]
//! Fuzz target for the verification pipeline entrypoint.
//!
//! This fuzz target exercises `csv_core::verifier::verify_proof` with
//! fuzzed `ProofBundle` inputs to catch edge cases and security issues
//! in the verification pipeline.
//!
//! The primary attack surface is the verification pipeline, so this
//! target provides end-to-end coverage that the decode-only targets
//! cannot reach.

use libfuzzer_sys::fuzz_target;
use csv_core::proof::ProofBundle;
use csv_core::signature::SignatureScheme;
use csv_core::verifier::verify_proof;

fuzz_target!(|data: &[u8]| {
    // Attempt to deserialize proof bundle from fuzzed data
    if let Ok(bundle) = serde_cbor::from_slice::<ProofBundle>(data) {
        // Run the full verification pipeline with a seal registry
        // that treats all seals as unused (allows fuzzing through
        // the entire pipeline).
        let _ = verify_proof(
            &bundle,
            |_seal_id| false, // treat all seals as unused
            SignatureScheme::Secp256k1,
        );
    }
});