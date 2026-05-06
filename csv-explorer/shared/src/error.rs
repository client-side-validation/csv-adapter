use csv_core::agent_types::{error_codes, FixAction, HasErrorSuggestion};
/// Error types for the CSV Explorer.
use thiserror::Error;

/// Top-level error type for the explorer.
#[derive(Error, Debug)]
pub enum ExplorerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML error: {0}")]
    Toml(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Entity not found: {entity_type} with id {id}")]
    NotFound { entity_type: String, id: String },

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("RPC error on chain {chain}: {message}")]
    RpcError { chain: String, message: String },

    #[error("RPC parse error on chain {chain}: {message}")]
    RpcParseError { chain: String, message: String },

    #[error("Indexer stopped")]
    IndexerStopped,

    #[error("Block processing error on chain {chain} at block {block}: {message}")]
    BlockError {
        chain: String,
        block: u64,
        message: String,
    },

    #[error("Chain reorg detected on {chain} at block {block}, depth: {depth}")]
    ChainReorg {
        chain: String,
        block: u64,
        depth: u64,
    },

    #[error("GraphQL error: {0}")]
    GraphQL(String),

    #[error("HTTP server error: {0}")]
    HttpServer(String),

    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl HasErrorSuggestion for ExplorerError {
    fn error_code(&self) -> &'static str {
        match self {
            ExplorerError::Io(_) => error_codes::EXP_IO_ERROR,
            ExplorerError::Toml(_) => error_codes::EXP_TOML_ERROR,
            ExplorerError::Json(_) => error_codes::EXP_JSON_ERROR,
            ExplorerError::Database(_) => error_codes::EXP_DATABASE_ERROR,
            ExplorerError::Migration(_) => error_codes::EXP_MIGRATION_ERROR,
            ExplorerError::NotFound { .. } => error_codes::EXP_ENTITY_NOT_FOUND,
            ExplorerError::Http(_) => error_codes::EXP_HTTP_ERROR,
            ExplorerError::RpcError { .. } => error_codes::EXP_RPC_ERROR,
            ExplorerError::RpcParseError { .. } => error_codes::EXP_RPC_PARSE_ERROR,
            ExplorerError::IndexerStopped => error_codes::EXP_INDEXER_STOPPED,
            ExplorerError::BlockError { .. } => error_codes::EXP_BLOCK_ERROR,
            ExplorerError::ChainReorg { .. } => error_codes::EXP_CHAIN_REORG,
            ExplorerError::GraphQL(_) => error_codes::EXP_GRAPHQL_ERROR,
            ExplorerError::HttpServer(_) => error_codes::EXP_HTTP_SERVER_ERROR,
            ExplorerError::Hex(_) => error_codes::EXP_HEX_DECODE_ERROR,
            ExplorerError::Parse(_) => error_codes::EXP_PARSE_ERROR,
            ExplorerError::Internal(_) => error_codes::EXP_INTERNAL_ERROR,
        }
    }

    fn description(&self) -> String {
        self.to_string()
    }

    fn suggested_fix(&self) -> String {
        match self {
            ExplorerError::Io(_) => {
                "I/O operation failed. Check file permissions and disk space.".to_string()
            }
            ExplorerError::Toml(_) => {
                "TOML configuration parsing failed. Check syntax in config files.".to_string()
            }
            ExplorerError::Json(_) => {
                "JSON parsing failed. Check API responses are valid JSON.".to_string()
            }
            ExplorerError::Database(_) => {
                "Database operation failed. Check connection and schema.".to_string()
            }
            ExplorerError::Migration(_) => {
                "Database migration failed. Check migration files and database state.".to_string()
            }
            ExplorerError::NotFound { entity_type, id } => {
                format!("{} '{}' not found in database.", entity_type, id)
            }
            ExplorerError::Http(_) => {
                "HTTP request failed. Check network and API endpoints.".to_string()
            }
            ExplorerError::RpcError { chain, .. } => {
                format!("RPC error on chain {}. Check node status and retry.", chain)
            }
            ExplorerError::RpcParseError { chain, .. } => {
                format!(
                    "RPC parse error on {}. Response format may have changed.",
                    chain
                )
            }
            ExplorerError::IndexerStopped => {
                "Indexer has stopped. Check logs and restart.".to_string()
            }
            ExplorerError::BlockError { chain, block, .. } => {
                format!(
                    "Error processing block {} on {}. Check block validity.",
                    block, chain
                )
            }
            ExplorerError::ChainReorg {
                chain,
                block,
                depth,
            } => {
                format!(
                    "Reorg detected on {} at block {} (depth {}). \
                     May need to re-index affected blocks.",
                    chain, block, depth
                )
            }
            ExplorerError::GraphQL(_) => {
                "GraphQL operation failed. Check query syntax and schema.".to_string()
            }
            ExplorerError::HttpServer(_) => {
                "HTTP server error. Check server configuration and ports.".to_string()
            }
            ExplorerError::Hex(_) => {
                "Hex decoding failed. Check input is valid hexadecimal.".to_string()
            }
            ExplorerError::Parse(_) => {
                "Parse error. Check input format matches expected type.".to_string()
            }
            ExplorerError::Internal(_) => {
                "Internal error. Check logs for details and report if persistent.".to_string()
            }
        }
    }

    fn docs_url(&self) -> String {
        error_codes::docs_url(self.error_code())
    }

    fn fix_action(&self) -> Option<FixAction> {
        match self {
            ExplorerError::Io(_) | ExplorerError::Database(_) | ExplorerError::Http(_) => {
                Some(FixAction::Retry {
                    parameter_changes: std::collections::HashMap::new(),
                })
            }
            ExplorerError::RpcError { chain, .. } => Some(FixAction::Retry {
                parameter_changes: std::collections::HashMap::from([
                    ("chain".to_string(), chain.clone()),
                    ("fallback_rpc".to_string(), "true".to_string()),
                ]),
            }),
            ExplorerError::ChainReorg { chain, block, .. } => Some(FixAction::CheckState {
                url: format!("https://{}.csv.dev/reorg/{}?block={}", chain, chain, block),
                what: "Check reorg status and affected blocks".to_string(),
            }),
            ExplorerError::IndexerStopped => Some(FixAction::Retry {
                parameter_changes: std::collections::HashMap::from([(
                    "restart_indexer".to_string(),
                    "true".to_string(),
                )]),
            }),
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, ExplorerError>;
