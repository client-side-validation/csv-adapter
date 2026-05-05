#!/usr/bin/env node
/**
 * CSV MCP Server — AI Agent Integration
 *
 * Enables AI agents (Claude, GPT, etc.) to operate CSV rights workflows
 * through the Model Context Protocol (MCP).
 *
 * High-value actions for MCP:
 * - create_seal(chain, value) — agent creates a seal
 * - transfer_right(right_id, destination) — agent transfers a right
 * - verify_proof(bundle_json) — agent verifies a proof bundle
 * - get_rights(address) — agent lists rights for an address
 * - monitor_transfer(transfer_id) — agent watches transfer status
 *
 * Usage:
 *   csv-mcp                    # Start MCP server (stdio transport)
 *   csv-mcp --stdio            # Explicit stdio transport
 *   csv-mcp --sse --port 3000  # SSE transport on port 3000
 */

import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { SSEServerTransport } from '@modelcontextprotocol/sdk/server/sse.js';
import { z } from 'zod';

// =========================================================================
// CSV CLI wrapper
// =========================================================================

/**
 * Execute a csv-cli command and return the result.
 * In production, this would call the actual csv-cli binary.
 * For now, it returns mock results for demonstration.
 */
async function executeCsvCommand(args: string[]): Promise<{ stdout: string; stderr: string; exitCode: number }> {
  // In production, this would use child_process.spawn to call csv-cli
  // For now, return mock results
  console.error(`[csv-mcp] Would execute: csv ${args.join(' ')}`);
  return {
    stdout: JSON.stringify({ status: 'mock', args }),
    stderr: '',
    exitCode: 0,
  };
}

// =========================================================================
// Tool definitions
// =========================================================================

/**
 * Get a list of all MCP tools provided by this server.
 */
function getTools() {
  return [
    {
      name: 'create_seal',
      description:
        'Create a single-use seal on a blockchain. ' +
        'A seal is a chain-native lock that enforces the single-use property of a digital right. ' +
        'Each chain has its own seal format (Bitcoin: UTXO, Ethereum: storage slot, Sui: ObjectId, etc.).',
      inputSchema: {
        type: 'object',
        properties: {
          chain: {
            type: 'string',
            enum: ['bitcoin', 'ethereum', 'sui', 'aptos', 'solana'],
            description: 'The blockchain to create the seal on',
          },
          value: {
            type: 'number',
            description: 'Optional value to lock (chain-specific units: satoshis, wei, etc.)',
          },
        },
        required: ['chain'],
      },
    },
    {
      name: 'transfer_right',
      description:
        'Transfer a digital right to a new owner. ' +
        'This consumes the current seal and creates a new one for the destination. ' +
        'The transfer is recorded in the commitment chain for provenance.',
      inputSchema: {
        type: 'object',
        properties: {
          right_id: {
            type: 'string',
            description: 'The right ID to transfer (32-byte hex string)',
          },
          destination: {
            type: 'string',
            description: 'The destination address or owner identifier',
          },
          chain: {
            type: 'string',
            enum: ['bitcoin', 'ethereum', 'sui', 'aptos', 'solana'],
            description: 'The chain where the right exists',
          },
        },
        required: ['right_id', 'destination'],
      },
    },
    {
      name: 'verify_proof',
      description:
        'Verify a proof bundle offline. ' +
        'A proof bundle contains all cryptographic evidence needed to verify a right. ' +
        'This verification requires NO blockchain RPC calls — pure cryptography. ' +
        'This is the CSV competitive advantage over traditional bridges.',
      inputSchema: {
        type: 'object',
        properties: {
          bundle_json: {
            type: 'string',
            description: 'JSON string of a ProofBundle to verify',
          },
        },
        required: ['bundle_json'],
      },
    },
    {
      name: 'get_rights',
      description:
        'List all rights owned by an address on a specific chain. ' +
        'Returns right IDs, values, and current status.',
      inputSchema: {
        type: 'object',
        properties: {
          address: {
            type: 'string',
            description: 'The blockchain address to query',
          },
          chain: {
            type: 'string',
            enum: ['bitcoin', 'ethereum', 'sui', 'aptos', 'solana'],
            description: 'Optional chain filter',
          },
        },
        required: ['address'],
      },
    },
    {
      name: 'monitor_transfer',
      description:
        'Monitor the status of a cross-chain transfer. ' +
        'Returns the current state in the transfer lifecycle: ' +
        'Locked → AwaitingFinality → BuildingProof → ProofReady → Minting → Complete',
      inputSchema: {
        type: 'object',
        properties: {
          transfer_id: {
            type: 'string',
            description: 'The transfer ID to monitor',
          },
        },
        required: ['transfer_id'],
      },
    },
    {
      name: 'get_protocol_info',
      description:
        'Get CSV protocol information including version, capabilities, and supported chains.',
      inputSchema: {
        type: 'object',
        properties: {},
        required: [],
      },
    },
    {
      name: 'export_proof_bundle',
      description:
        'Export a proof bundle as a portable JSON file. ' +
        'The exported bundle can be shared with any counterparty for offline verification.',
      inputSchema: {
        type: 'object',
        properties: {
          right_id: {
            type: 'string',
            description: 'The right ID to generate a proof bundle for',
          },
        },
        required: ['right_id'],
      },
    },
    {
      name: 'accept_consignment',
      description:
        'Accept a consignment (complete transfer artifact) into local state. ' +
        'The consignment is verified before acceptance: ' +
        '1. Structural validation 2. Commitment chain 3. Double-spend check 4. State transitions',
      inputSchema: {
        type: 'object',
        properties: {
          consignment_json: {
            type: 'string',
            description: 'JSON string of a Consignment to accept',
          },
        },
        required: ['consignment_json'],
      },
    },
  ];
}

// =========================================================================
// Server setup
// =========================================================================

async function startServer(transportType: 'stdio' | 'sse' = 'stdio', port?: number) {
  const server = new McpServer({
    name: 'csv-mcp-server',
    version: '0.4.0',
  });

  // Register all tools
  const tools = getTools();

  // create_seal tool
  server.registerTool('create_seal', {
    description: tools.find((t) => t.name === 'create_seal')!.description,
    inputSchema: tools.find((t) => t.name === 'create_seal')!.inputSchema as any,
  }, async (args: any) => {
    const chain = args.chain;
    const value = args.value;
    const result = await executeCsvCommand(['seal', 'create', '--chain', chain, ...(value ? ['--value', String(value)] : [])]);
    return {
      content: [{ type: 'text', text: result.stdout }],
      isError: result.exitCode !== 0,
    };
  });

  // transfer_right tool
  server.registerTool('transfer_right', {
    description: tools.find((t) => t.name === 'transfer_right')!.description,
    inputSchema: tools.find((t) => t.name === 'transfer_right')!.inputSchema as any,
  }, async (args: any) => {
    const result = await executeCsvCommand([
      'right', 'transfer',
      '--right-id', args.right_id,
      '--destination', args.destination,
      ...(args.chain ? ['--chain', args.chain] : []),
    ]);
    return {
      content: [{ type: 'text', text: result.stdout }],
      isError: result.exitCode !== 0,
    };
  });

  // verify_proof tool
  server.registerTool('verify_proof', {
    description: tools.find((t) => t.name === 'verify_proof')!.description,
    inputSchema: tools.find((t) => t.name === 'verify_proof')!.inputSchema as any,
  }, async (args: any) => {
    const result = await executeCsvCommand(['proof', 'verify', '--bundle', args.bundle_json]);
    return {
      content: [{ type: 'text', text: result.stdout }],
      isError: result.exitCode !== 0,
    };
  });

  // get_rights tool
  server.registerTool('get_rights', {
    description: tools.find((t) => t.name === 'get_rights')!.description,
    inputSchema: tools.find((t) => t.name === 'get_rights')!.inputSchema as any,
  }, async (args: any) => {
    const result = await executeCsvCommand([
      'right', 'list',
      '--address', args.address,
      ...(args.chain ? ['--chain', args.chain] : []),
    ]);
    return {
      content: [{ type: 'text', text: result.stdout }],
      isError: result.exitCode !== 0,
    };
  });

  // monitor_transfer tool
  server.registerTool('monitor_transfer', {
    description: tools.find((t) => t.name === 'monitor_transfer')!.description,
    inputSchema: tools.find((t) => t.name === 'monitor_transfer')!.inputSchema as any,
  }, async (args: any) => {
    const result = await executeCsvCommand(['transfer', 'status', '--id', args.transfer_id]);
    return {
      content: [{ type: 'text', text: result.stdout }],
      isError: result.exitCode !== 0,
    };
  });

  // get_protocol_info tool
  server.registerTool('get_protocol_info', {
    description: tools.find((t) => t.name === 'get_protocol_info')!.description,
    inputSchema: tools.find((t) => t.name === 'get_protocol_info')!.inputSchema as any,
  }, async () => {
    const info = {
      protocol: 'CSV (Client-Side Validation)',
      version: '0.4.0',
      supportedChains: ['bitcoin', 'ethereum', 'sui', 'aptos', 'solana'],
      features: {
        singleUseSeals: true,
        offlineVerification: true,
        crossChainTransfers: true,
        commitmentChain: true,
        mpcBatching: true,
        zkProofs: true,
      },
      competitiveAdvantages: [
        'No custody — rights are off-chain state, seals are chain-enforced',
        'No trusted bridge — proof bundles are self-verifying',
        'Offline verification — anyone with the bundle can verify',
        'Cryptographic double-spend prevention',
        'Cross-chain provenance — tamper-evident audit log',
      ],
    };
    return {
      content: [{ type: 'text', text: JSON.stringify(info, null, 2) }],
      isError: false,
    };
  });

  // export_proof_bundle tool
  server.registerTool('export_proof_bundle', {
    description: tools.find((t) => t.name === 'export_proof_bundle')!.description,
    inputSchema: tools.find((t) => t.name === 'export_proof_bundle')!.inputSchema as any,
  }, async (args: any) => {
    const result = await executeCsvCommand(['proof', 'export', '--right-id', args.right_id]);
    return {
      content: [{ type: 'text', text: result.stdout }],
      isError: result.exitCode !== 0,
    };
  });

  // accept_consignment tool
  server.registerTool('accept_consignment', {
    description: tools.find((t) => t.name === 'accept_consignment')!.description,
    inputSchema: tools.find((t) => t.name === 'accept_consignment')!.inputSchema as any,
  }, async (args: any) => {
    const result = await executeCsvCommand(['consignment', 'accept', '--json', args.consignment_json]);
    return {
      content: [{ type: 'text', text: result.stdout }],
      isError: result.exitCode !== 0,
    };
  });

  // Start the server
  if (transportType === 'stdio') {
    const transport = new StdioServerTransport();
    await server.connect(transport);
    console.error('CSV MCP Server running on stdio');
  } else if (transportType === 'sse' && port) {
    const transport = await SSEServerTransport.create({ port });
    await server.connect(transport);
    console.error(`CSV MCP Server running on SSE at http://localhost:${port}`);
  }
}

// =========================================================================
// CLI entry point
// =========================================================================

const args = process.argv.slice(2);
const transportType: 'stdio' | 'sse' = args.includes('--sse') ? 'sse' : 'stdio';
const portMatch = args.find((a) => a.startsWith('--port='));
const port = portMatch ? parseInt(portMatch.split('=')[1], 10) : undefined;

startServer(transportType, port).catch((err) => {
  console.error('Failed to start MCP server:', err);
  process.exit(1);
});
