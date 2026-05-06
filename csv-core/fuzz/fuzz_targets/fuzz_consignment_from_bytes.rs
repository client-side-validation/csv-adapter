//! Fuzz target for `Consignment::from_bytes()`.

#![no_main]

use csv_core::consignment::Consignment;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The contract: from_bytes must never panic on any input.
    let _ = Consignment::from_bytes(data);
});
