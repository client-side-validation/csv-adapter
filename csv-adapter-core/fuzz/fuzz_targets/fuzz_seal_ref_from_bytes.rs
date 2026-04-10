//! Fuzz target for `SealRef::from_bytes()`.

#![no_main]

use csv_adapter_core::seal::SealRef;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The contract: from_bytes must never panic on any input.
    let _ = SealRef::from_bytes(data);
});
