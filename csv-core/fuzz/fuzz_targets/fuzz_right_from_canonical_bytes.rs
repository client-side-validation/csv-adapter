//! Fuzz target for `Sanad::from_canonical_bytes()`.
//!
//! This fuzz target feeds arbitrary byte sequences into the Sanad
//! deserializer to find panics, infinite loops, or memory safety issues.

#![no_main]

use csv_core::sanad::Sanad;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The contract: from_canonical_bytes must never panic on any input.
    // It should either return Ok(Sanad) or Err(SanadError).
    let _ = Sanad::from_canonical_bytes(data);
});
