#![allow(dead_code)]
//! Fuzz target for RPC response parsing
//!
//! This fuzz target tests the robustness of RPC response parsing
//! against malformed or unexpected JSON input.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Attempt to parse RPC response from fuzzed data
    // This tests that the parser handles malformed JSON gracefully
    if let Ok(s) = std::str::from_utf8(data) {
        let _: Result<serde_json::Value, _> = serde_json::from_str(s);

        // Also test parsing as a generic RPC response structure
        #[derive(serde::Deserialize)]
        struct RpcResponse {
            jsonrpc: Option<String>,
            id: Option<serde_json::Value>,
            result: Option<serde_json::Value>,
            error: Option<serde_json::Value>,
        }
        let _: Result<RpcResponse, _> = serde_json::from_str(s);
    }
});
