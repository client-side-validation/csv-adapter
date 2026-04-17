/**
 * Right management tools for CSV MCP Server
 * 
 * Tools:
 * - csv_right_create: Create a new Right anchored to a specific chain
 * - csv_right_get: Get details of a specific Right
 * - csv_right_list: List all Rights in wallet
 */

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { z } from "zod";
import { CsvErrorBuilder, ErrorCode } from "../types/errors.js";

const ChainEnum = z.enum(["bitcoin", "ethereum", "sui", "aptos"]);

export function registerRightTools(server: McpServer) {
  // Create Right
  server.tool(
    "csv_right_create",
    "Create a new Right anchored to a specific blockchain chain",
    {
      chain: ChainEnum.describe("Which chain enforces the single-use seal"),
      commitment_data: z.object({}).passthrough().describe("Data to commit (will be hashed)"),
      owner_address: z.string().optional().describe("Address that owns the Right (default: wallet address)"),
      metadata: z.record(z.unknown()).optional().describe("Optional metadata for the Right"),
    },
    async ({ chain, commitment_data, owner_address, metadata }) => {
      try {
        // TODO: Implement with @csv-adapter/sdk
        const right_id = "0x" + "a".repeat(64); // Placeholder
        const transaction_hash = "0x" + "b".repeat(64); // Placeholder
        
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify({
                success: true,
                right_id,
                chain,
                transaction_hash,
                commitment_hash: "0x" + "c".repeat(64),
                owner: owner_address || "default_wallet_address",
                metadata: metadata || {},
                created_at: new Date().toISOString(),
              }, null, 2),
            },
          ],
        };
      } catch (error: any) {
        const csvError = CsvErrorBuilder.build(
          ErrorCode.TRANSFER_FAILED,
          `Failed to create Right: ${error.message}`,
          { chain, error: error.name }
        );
        
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(csvError, null, 2),
            },
          ],
          isError: true,
        };
      }
    }
  );

  // Get Right
  server.tool(
    "csv_right_get",
    "Get details of a specific Right by its ID",
    {
      right_id: z.string().regex(/^0x[a-fA-F0-9]{64}$/).describe("The 32-byte Right ID (hex format)"),
    },
    async ({ right_id }) => {
      try {
        // TODO: Implement with @csv-adapter/sdk
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify({
                success: true,
                right_id,
                chain: "bitcoin",
                commitment: "0x" + "d".repeat(64),
                owner: "bc1q...",
                created_at: "2026-04-10T14:32:00Z",
                status: "active",
                history: [
                  {
                    action: "created",
                    timestamp: "2026-04-10T14:32:00Z",
                    chain: "bitcoin",
                    transaction: "0x" + "e".repeat(64),
                  },
                ],
              }, null, 2),
            },
          ],
        };
      } catch (error: any) {
        const csvError = CsvErrorBuilder.build(
          ErrorCode.RIGHT_NOT_FOUND,
          `Right ${right_id} not found`,
          { right_id, error: error.name }
        );
        
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(csvError, null, 2),
            },
          ],
          isError: true,
        };
      }
    }
  );

  // List Rights
  server.tool(
    "csv_right_list",
    "List all Rights in the wallet, optionally filtered by chain",
    {
      chain: ChainEnum.optional().describe("Filter by chain (default: all chains)"),
      status: z.enum(["active", "spent", "all"]).optional().describe("Filter by status (default: active)"),
    },
    async ({ chain, status }) => {
      try {
        // TODO: Implement with @csv-adapter/sdk
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify({
                success: true,
                count: 3,
                rights: [
                  {
                    right_id: "0x" + "a".repeat(64),
                    chain: "bitcoin",
                    commitment: "0x" + "b".repeat(64),
                    status: "active",
                    created_at: "2026-04-10T14:32:00Z",
                  },
                  {
                    right_id: "0x" + "c".repeat(64),
                    chain: "ethereum",
                    commitment: "0x" + "d".repeat(64),
                    status: "active",
                    created_at: "2026-04-10T15:01:00Z",
                  },
                ],
              }, null, 2),
            },
          ],
        };
      } catch (error: any) {
        const csvError = CsvErrorBuilder.build(
          ErrorCode.WALLET_NOT_CONNECTED,
          `Failed to list Rights: ${error.message}`,
          { chain, status, error: error.name }
        );
        
        return {
          content: [
            {
              type: "text",
              text: JSON.stringify(csvError, null, 2),
            },
          ],
          isError: true,
        };
      }
    }
  );
}
