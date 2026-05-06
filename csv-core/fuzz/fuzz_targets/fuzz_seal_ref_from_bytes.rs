//! Fuzz target for `SealPoint::from_bytes()`.

#![no_main]

use csv_core::seal::SealPoint;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The contract: from_bytes must never panic on any input.
    let _ = SealPoint::from_bytes(data);
});
