/**
 * Enhanced error handling with actionable suggestions for AI agents
 * 
 * Provides machine-readable error information that agents can use to:
 * - Auto-fix common issues
 * - Provide helpful suggestions to users
 * - Retry with corrected parameters
 */

export enum ErrorCode {
  // Wallet and connection errors
  WALLET_NOT_CONNECTED = "WALLET_NOT_CONNECTED",
  INSUFFICIENT_FUNDS = "INSUFFICIENT_FUNDS",
  WALLET_LOCKED = "WALLET_LOCKED",
  
  // Chain-specific errors
  CHAIN_NOT_SUPPORTED = "CHAIN_NOT_SUPPORTED",
  RPC_TIMEOUT = "RPC_TIMEOUT",
  RPC_ERROR = "RPC_ERROR",
  
  // Right management errors
  RIGHT_NOT_FOUND = "RIGHT_NOT_FOUND",
  RIGHT_ALREADY_SPENT = "RIGHT_ALREADY_SPENT",
  INVALID_RIGHT_ID = "INVALID_RIGHT_ID",
  
  // Transfer errors
  TRANSFER_FAILED = "TRANSFER_FAILED",
  PROOF_GENERATION_FAILED = "PROOF_GENERATION_FAILED",
  PROOF_VERIFICATION_FAILED = "PROOF_VERIFICATION_FAILED",
  
  // General errors
  INVALID_PARAMETERS = "INVALID_PARAMETERS",
  NETWORK_ERROR = "NETWORK_ERROR",
  TIMEOUT = "TIMEOUT",
}

export interface FixAction {
  type: "retry" | "parameter_change" | "external_action" | "check_state";
  description: string;
  parameters?: Record<string, any>;
  url?: string;
}

export interface ErrorSuggestion {
  message: string;
  fix?: FixAction;
  docs_url: string;
  error_code: ErrorCode;
  retry_after_seconds?: number;
}

export interface CsvError {
  success: false;
  error_code: ErrorCode;
  error_message: string;
  suggestion: ErrorSuggestion;
  context?: Record<string, any>;
}

export class CsvErrorBuilder {
  static build(
    code: ErrorCode,
    message: string,
    context?: Record<string, any>
  ): CsvError {
    const suggestion = this.getSuggestion(code, context);
    
    return {
      success: false,
      error_code: code,
      error_message: message,
      suggestion,
      context,
    };
  }

  private static getSuggestion(code: ErrorCode, context?: Record<string, any>): ErrorSuggestion {
    switch (code) {
      case ErrorCode.INSUFFICIENT_FUNDS:
        return {
          message: `Insufficient funds: have ${context?.available || 'unknown'}, need ${context?.required || 'unknown'}`,
          fix: {
            type: "external_action",
            description: "Fund wallet from faucet or exchange",
            url: this.getFaucetUrl(context?.chain),
          },
          docs_url: "https://docs.csv.dev/errors/insufficient-funds",
          error_code: code,
        };

      case ErrorCode.WALLET_NOT_CONNECTED:
        return {
          message: "Wallet not connected or initialized",
          fix: {
            type: "external_action",
            description: "Initialize wallet with: csv wallet init",
          },
          docs_url: "https://docs.csv.dev/errors/wallet-not-connected",
          error_code: code,
        };

      case ErrorCode.CHAIN_NOT_SUPPORTED:
        return {
          message: `Chain '${context?.chain}' not supported`,
          fix: {
            type: "parameter_change",
            description: "Use supported chains: bitcoin, ethereum, sui, aptos, solana",
            parameters: { supported_chains: ["bitcoin", "ethereum", "sui", "aptos", "solana"] },
          },
          docs_url: "https://docs.csv.dev/errors/chain-not-supported",
          error_code: code,
        };

      case ErrorCode.RPC_TIMEOUT:
        return {
          message: "RPC request timed out",
          fix: {
            type: "retry",
            description: "Retry with different RPC endpoint",
            parameters: { retry_after_seconds: 5 },
          },
          docs_url: "https://docs.csv.dev/errors/rpc-timeout",
          error_code: code,
          retry_after_seconds: 5,
        };

      case ErrorCode.RIGHT_NOT_FOUND:
        return {
          message: `Right ${context?.right_id} not found`,
          fix: {
            type: "check_state",
            description: "Check wallet history or query with csv_right_list",
          },
          docs_url: "https://docs.csv.dev/errors/right-not-found",
          error_code: code,
        };

      case ErrorCode.RIGHT_ALREADY_SPENT:
        return {
          message: `Right ${context?.right_id} already spent`,
          fix: {
            type: "check_state",
            description: "Verify Right status with csv_right_get",
          },
          docs_url: "https://docs.csv.dev/errors/right-already-spent",
          error_code: code,
        };

      case ErrorCode.INVALID_RIGHT_ID:
        return {
          message: "Invalid Right ID format",
          fix: {
            type: "parameter_change",
            description: "Right ID must be 32-byte hex string (0x...)",
            parameters: { format: "0x[a-fA-F0-9]{64}" },
          },
          docs_url: "https://docs.csv.dev/errors/invalid-right-id",
          error_code: code,
        };

      case ErrorCode.TRANSFER_FAILED:
        return {
          message: "Cross-chain transfer failed",
          fix: {
            type: "retry",
            description: "Check network conditions and retry",
            parameters: { retry_after_seconds: 10 },
          },
          docs_url: "https://docs.csv.dev/errors/transfer-failed",
          error_code: code,
          retry_after_seconds: 10,
        };

      case ErrorCode.PROOF_GENERATION_FAILED:
        return {
          message: "Failed to generate cross-chain proof",
          fix: {
            type: "retry",
            description: "Retry with different confirmation depth",
            parameters: { retry_after_seconds: 15 },
          },
          docs_url: "https://docs.csv.dev/errors/proof-generation-failed",
          error_code: code,
          retry_after_seconds: 15,
        };

      case ErrorCode.PROOF_VERIFICATION_FAILED:
        return {
          message: "Cross-chain proof verification failed",
          fix: {
            type: "check_state",
            description: "Verify source chain confirmations and proof integrity",
          },
          docs_url: "https://docs.csv.dev/errors/proof-verification-failed",
          error_code: code,
        };

      case ErrorCode.INVALID_PARAMETERS:
        return {
          message: "Invalid parameters provided",
          fix: {
            type: "parameter_change",
            description: "Check parameter formats and constraints",
          },
          docs_url: "https://docs.csv.dev/errors/invalid-parameters",
          error_code: code,
        };

      case ErrorCode.NETWORK_ERROR:
        return {
          message: "Network connectivity issue",
          fix: {
            type: "retry",
            description: "Check network connection and retry",
            parameters: { retry_after_seconds: 3 },
          },
          docs_url: "https://docs.csv.dev/errors/network-error",
          error_code: code,
          retry_after_seconds: 3,
        };

      case ErrorCode.TIMEOUT:
        return {
          message: "Operation timed out",
          fix: {
            type: "retry",
            description: "Increase timeout or retry with smaller batch",
            parameters: { retry_after_seconds: 5 },
          },
          docs_url: "https://docs.csv.dev/errors/timeout",
          error_code: code,
          retry_after_seconds: 5,
        };

      default:
        return {
          message: "Unknown error occurred",
          docs_url: "https://docs.csv.dev/errors/unknown",
          error_code: code,
        };
    }
  }

  private static getFaucetUrl(chain?: string): string {
    const faucets: Record<string, string> = {
      bitcoin: "https://signetfaucet.com/",
      ethereum: "https://faucet.sepolia.dev/",
      sui: "https://faucet.sui.io/",
      aptos: "https://faucet.testnet.aptoslabs.com/",
      solana: "https://faucet.solana.com/",
    };
    return faucets[chain || ""] || "https://docs.csv.dev/faucets";
  }
}
