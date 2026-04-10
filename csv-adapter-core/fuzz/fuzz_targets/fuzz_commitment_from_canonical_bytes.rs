//! Fuzz target for `Commitment::from_canonical_bytes()`.

#![no_main]

use csv_adapter_core::commitment::Commitment;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The contract: from_canonical_bytes must never panic on any input.
    let _ = Commitment::from_canonical_bytes(data);
});
