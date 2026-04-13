//! Agent-friendly types for CSV Adapter
//!
//! This module provides structured error types and status reporting
//! optimized for AI agent consumption.
//!
//! ## Key Features
//!
//! - **Self-describing errors**: Every error includes machine-actionable metadata
//! - **Structured status**: All operations return machine-readable progress
//! - **Fix suggestions**: Errors include actionable `FixAction` for autonomous resolution

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Agent-friendly transfer status with structured progress
///
/// Every operation returns machine-readable progress that agents can parse and act upon.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TransferStatus {
    /// Transfer initiated
    Initiated {
        /// Unique transfer identifier
        transfer_id: String,
        /// ISO 8601 timestamp
        timestamp: String,
    },
    /// Locking Right on source chain
    Locking {
        /// Source chain
        chain: Chain,
        /// Estimated blocks to wait
        estimated_blocks: u32,
        /// Current confirmation count
        current_confirmations: u32,
        /// Required confirmation count
        required_confirmations: u32,
    },
    /// Generating cryptographic proof
    GeneratingProof {
        /// Type of proof being generated
        proof_type: String,
        /// Estimated proof size in bytes
        estimated_size_bytes: usize,
        /// Progress percentage (0-100)
        progress_percent: u8,
    },
    /// Submitting proof to destination chain
    SubmittingProof {
        /// Destination chain
        destination_chain: Chain,
        /// Estimated gas cost
        gas_estimate: String,
        /// Transaction hash (if submitted)
        tx_hash: Option<String>,
    },
    /// Verifying proof on destination chain
    Verifying {
        /// Destination chain
        chain: Chain,
        /// Verification time in milliseconds
        verification_time_ms: Option<u64>,
    },
    /// Minting Right on destination chain
    Minting {
        /// Destination chain
        chain: Chain,
        /// Transaction hash
        tx_hash: String,
    },
    /// Transfer completed successfully
    Completed {
        /// Right ID on destination chain
        right_id: String,
        /// Destination chain
        destination_chain: Chain,
        /// Destination transaction hash
        transaction_hash: String,
        /// Total transfer time in milliseconds
        total_time_ms: u64,
    },
    /// Transfer failed
    Failed {
        /// Machine-readable error code
        error_code: String,
        /// Human-readable error message
        error_message: String,
        /// Whether retrying might succeed
        retryable: bool,
        /// Suggested action for agent
        suggested_action: String,
    },
}

impl TransferStatus {
    /// Get progress percentage (0-100)
    pub fn progress_percent(&self) -> u8 {
        match self {
            Self::Initiated { .. } => 0,
            Self::Locking { current_confirmations, required_confirmations, .. } => {
                ((current_confirmations * 25) / required_confirmations).min(25) as u8
            }
            Self::GeneratingProof { progress_percent, .. } => *progress_percent,
            Self::SubmittingProof { .. } => 50,
            Self::Verifying { .. } => 75,
            Self::Minting { .. } => 90,
            Self::Completed { .. } => 100,
            Self::Failed { .. } => 0,
        }
    }

    /// Check if transfer is complete
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Completed { .. })
    }

    /// Check if transfer has failed
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed { .. })
    }

    /// Get current step name
    pub fn current_step(&self) -> &'static str {
        match self {
            Self::Initiated { .. } => "initiated",
            Self::Locking { .. } => "locking",
            Self::GeneratingProof { .. } => "generating_proof",
            Self::SubmittingProof { .. } => "submitting_proof",
            Self::Verifying { .. } => "verifying",
            Self::Minting { .. } => "minting",
            Self::Completed { .. } => "completed",
            Self::Failed { .. } => "failed",
        }
    }
}

/// Machine-actionable fix suggestion for errors
///
/// Agents can use this to automatically resolve issues without human intervention.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum FixAction {
    /// Fund wallet from faucet
    FundFromFaucet {
        /// Faucet URL
        url: String,
        /// Amount to request
        amount: String,
    },
    /// Retry with different parameters
    Retry {
        /// Parameter changes to apply
        parameter_changes: HashMap<String, String>,
    },
    /// Check external state
    CheckState {
        /// URL to check
        url: String,
        /// What to check for
        what: String,
    },
    /// Wait for confirmations
    WaitForConfirmations {
        /// Required confirmations
        confirmations: u32,
        /// Estimated time in seconds
        estimated_seconds: u64,
    },
}

/// Self-describing error with machine-actionable metadata
///
/// Every error includes:
/// - Human-readable message
/// - Machine-readable error code
/// - Suggested fix (if available)
/// - Documentation URL
#[derive(Debug, Clone, Serialize)]
pub struct ErrorSuggestion {
    /// Human-readable suggestion
    pub message: String,
    /// Machine-actionable fix (if available)
    pub fix: Option<FixAction>,
    /// Related documentation URL
    pub docs_url: String,
    /// Error code for agent lookup
    pub error_code: String,
}

impl ErrorSuggestion {
    /// Create a new error suggestion
    pub fn new(
        error_code: impl Into<String>,
        message: impl Into<String>,
        docs_url: impl Into<String>,
    ) -> Self {
        Self {
            error_code: error_code.into(),
            message: message.into(),
            docs_url: docs_url.into(),
            fix: None,
        }
    }

    /// Add a fix action
    pub fn with_fix(mut self, fix: FixAction) -> Self {
        self.fix = Some(fix);
        self
    }
}

/// Chain identifier for agent compatibility
///
/// This enum is used across all agent-facing APIs to ensure consistency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    Bitcoin,
    Ethereum,
    Sui,
    Aptos,
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bitcoin => write!(f, "bitcoin"),
            Self::Ethereum => write!(f, "ethereum"),
            Self::Sui => write!(f, "sui"),
            Self::Aptos => write!(f, "aptos"),
        }
    }
}

impl std::str::FromStr for Chain {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bitcoin" | "btc" => Ok(Self::Bitcoin),
            "ethereum" | "eth" => Ok(Self::Ethereum),
            "sui" => Ok(Self::Sui),
            "aptos" | "apt" => Ok(Self::Aptos),
            _ => Err(format!("Unknown chain: {}. Supported: bitcoin, ethereum, sui, aptos", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_display() {
        assert_eq!(Chain::Bitcoin.to_string(), "bitcoin");
        assert_eq!(Chain::Ethereum.to_string(), "ethereum");
        assert_eq!(Chain::Sui.to_string(), "sui");
        assert_eq!(Chain::Aptos.to_string(), "aptos");
    }

    #[test]
    fn test_chain_from_str() {
        assert_eq!("bitcoin".parse::<Chain>().unwrap(), Chain::Bitcoin);
        assert_eq!("btc".parse::<Chain>().unwrap(), Chain::Bitcoin);
        assert_eq!("ETH".parse::<Chain>().unwrap(), Chain::Ethereum);
        assert!("solana".parse::<Chain>().is_err());
    }

    #[test]
    fn test_transfer_status_progress() {
        let status = TransferStatus::Initiated {
            transfer_id: "0x123".to_string(),
            timestamp: "2026-04-11T10:00:00Z".to_string(),
        };
        assert_eq!(status.progress_percent(), 0);
        assert_eq!(status.current_step(), "initiated");
        assert!(!status.is_complete());
        assert!(!status.is_failed());
    }

    #[test]
    fn test_error_suggestion_with_fix() {
        let suggestion = ErrorSuggestion::new(
            "CSV_001",
            "Insufficient funds",
            "https://docs.csv.dev/errors/CSV_001",
        )
        .with_fix(FixAction::FundFromFaucet {
            url: "https://faucet.csv.dev".to_string(),
            amount: "0.01 BTC".to_string(),
        });

        assert!(suggestion.fix.is_some());
        assert_eq!(suggestion.error_code, "CSV_001");
    }
}
