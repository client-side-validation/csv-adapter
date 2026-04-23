#!/usr/bin/env bash
# Deploy CSV Seal program on Solana Devnet/Testnet/Mainnet
# Usage: ./deploy.sh [network] [anchor-path]
#   network: devnet (default), testnet, mainnet, localnet
#   anchor-path: path to anchor binary (default: anchor)

set -euo pipefail

NETWORK="${1:-devnet}"
ANCHOR="${2:-anchor}"

echo "=== Solana ${NETWORK} Deployment ==="
echo ""

# Check dependencies
if ! command -v "$ANCHOR" &>/dev/null; then
    echo "ERROR: Anchor not found. Install with:"
    echo "  npm install -g @coral-xyz/anchor-cli"
    exit 1
fi

if ! command -v solana &>/dev/null; then
    echo "ERROR: Solana CLI not found. Install from:"
    echo "  https://docs.solana.com/cli/install"
    exit 1
fi

cd "$(dirname "$0")/.."

# Get active wallet
echo "Active wallet:"
solana address 2>/dev/null || {
    echo "No active wallet. Run: solana-keygen new"
    exit 1
}
echo ""

# Check balance
echo "Wallet balance:"
solana balance 2>/dev/null || echo "Unable to fetch balance (may need airdrop)"
echo ""

# Set cluster
solana config set --url "$NETWORK"

# Build the program
echo "Building Anchor program..."
$ANCHOR build 2>&1 | tail -10
echo ""

# Deploy
echo "Deploying to ${NETWORK}..."
deploy_output=$($ANCHOR deploy --provider.cluster "$NETWORK" 2>&1)
echo "$deploy_output"
echo ""

# Extract program ID from the output
program_id=$(echo "$deploy_output" | grep -oP 'Program Id: \K[0-9A-Za-z]{32,44}' || echo "")

if [ -z "$program_id" ]; then
    # Try to get from Anchor.toml or keypair
    program_id=$(solana-keygen pubkey target/deploy/csv_seal-keypair.json 2>/dev/null || echo "")
fi

if [ -z "$program_id" ]; then
    echo "WARNING: Could not extract program ID from deploy output."
    echo "Check the output above for the program address."
else
    echo "=== DEPLOYMENT SUMMARY ==="
    echo "Program ID: ${program_id}"
    echo "Network: ${NETWORK}"
    echo "=========================="
    echo ""
    
    # Save to state file
    mkdir -p "scripts"
    cat > "scripts/deploy-${NETWORK}.json" <<EOF
{
  "program_id": "${program_id}",
  "network": "${NETWORK}",
  "deployed_at": $(date +%s),
  "module": "csv_seal"
}
EOF
    
    echo "Deployment info saved to scripts/deploy-${NETWORK}.json"
    echo ""
fi

# Initialize the LockRegistry
echo "Initializing LockRegistry..."
$ANCHOR run initialize --provider.cluster "$NETWORK" 2>&1 || {
    echo "Note: Registry initialization may require manual execution:"
    echo "  anchor run initialize --provider.cluster ${NETWORK}"
}

echo ""
echo "Deployment complete!"
echo ""
echo "Next steps:"
echo "1. Update Anchor.toml with the program ID: ${program_id}"
echo "2. Update your csv-cli configuration to use this program ID"
echo "3. Run tests: anchor test --provider.cluster ${NETWORK}"
