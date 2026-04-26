#!/usr/bin/env bash
# Deploy CSV Seal contracts on Sui Testnet
# Usage: ./deploy.sh [network] [sui-client-path]
#   network: testnet (default), devnet, mainnet
#   sui-client-path: path to sui binary (default: sui)

set -euo pipefail

NETWORK="${1:-testnet}"
SUI="${2:-sui}"

echo "=== Sui ${NETWORK} Deployment ==="
echo ""

# Check dependencies
if ! command -v "$SUI" &>/dev/null; then
    echo "ERROR: sui client not found. Install with: cargo install --git https://github.com/MystenLabs/sui.git --bin sui"
    exit 1
fi

cd "$(dirname "$0")/.."

# Check if sui config exists
SUI_CONFIG_DIR="${SUI_CONFIG_DIR:-$HOME/.sui}"
if [ ! -f "$SUI_CONFIG_DIR/client.yaml" ]; then
    echo "ERROR: Sui client not configured. Please run:"
    echo "  $SUI client new-address ed25519"
    echo "  $SUI client switch --address <your-address>"
    echo "Or set SUI_CONFIG_DIR to your sui config directory."
    exit 1
fi

# Handle csv-cli wallet if specified
if [ -n "${CSV_SUI_PRIVATE_KEY:-}" ]; then
    echo "Using csv-cli wallet for deployment..."
    # Create temp keypair file
    KEYPAIR_FILE=$(mktemp)
    echo "{\"privateKey\": \"$CSV_SUI_PRIVATE_KEY\", \"scheme\": \"ed25519\"}" > "$KEYPAIR_FILE"
    echo "Created keypair file: $KEYPAIR_FILE"
    cat "$KEYPAIR_FILE"
    # Import the keypair
    IMPORT_OUTPUT=$("$SUI" client import --keypair-file "$KEYPAIR_FILE" --alias csv-deploy --json 2>&1)
    IMPORT_EXIT=$?
    echo "Import exit code: $IMPORT_EXIT"
    echo "Import output: $IMPORT_OUTPUT"
    if [ $IMPORT_EXIT -ne 0 ]; then
        echo "Failed to import private key: $IMPORT_OUTPUT"
        rm "$KEYPAIR_FILE"
        exit 1
    fi
    # Switch to the imported address
    SWITCH_OUTPUT=$("$SUI" client switch --address "$CSV_SUI_ADDRESS" 2>&1)
    SWITCH_EXIT=$?
    echo "Switch exit code: $SWITCH_EXIT"
    echo "Switch output: $SWITCH_OUTPUT"
    rm "$KEYPAIR_FILE"
    echo "Switched to csv-cli wallet: $CSV_SUI_ADDRESS"
else
    echo "Using Sui CLI active wallet"
fi

# Get active address
echo "Active wallet:"
"$SUI" client active-address 2>/dev/null || {
    echo "No active wallet. Run: $SUI client new-address ed25519"
    exit 1
}

echo ""

# Check balance
echo "Wallet balance:"
"$SUI" client gas 2>/dev/null | head -5 || echo "Unable to fetch gas (may need faucet)"
echo ""

# Build the package
echo "Building Move package..."
"$SUI" move build --path contracts 2>&1 | tail -5
echo ""

# Publish to testnet
echo "Publishing to ${NETWORK}..."
PUBLISH_OUTPUT=$("$SUI" client publish contracts \
    --gas-budget 500000000 \
    --json 2>&1)

echo "$PUBLISH_OUTPUT" > "scripts/deploy-output-${NETWORK}.json"

# Extract package ID from output
PACKAGE_ID=$(echo "$PUBLISH_OUTPUT" | python3 -c "
import sys, json
data = json.load(sys.stdin)
for event in data.get('events', []):
    if event.get('type') == 'published':
        print(event['packageId'])
        sys.exit(0)
# Fallback: parse from objectChanges
for change in data.get('objectChanges', []):
    if change.get('type') == 'published':
        print(change.get('packageId', ''))
        sys.exit(0)
print('')
" 2>/dev/null || echo "")

if [ -z "$PACKAGE_ID" ]; then
    echo "WARNING: Could not extract package ID from output."
    echo "Check scripts/deploy-output-${NETWORK}.json for the full response."
    echo "Look for 'packageId' in the output."
    exit 1
fi

echo ""
echo "=== DEPLOYMENT SUMMARY ==="
echo "Package ID: ${PACKAGE_ID}"
echo "Network: ${NETWORK}"
echo "Module: csv_seal::csv_seal"
echo "=========================="
echo ""
echo "Next steps:"
echo "1. Update Move.toml: csv_seal = \"${PACKAGE_ID}\""
echo "2. Initialize LockRegistry:"
echo "   sui client call --package ${PACKAGE_ID} --module csv_seal --function create_registry --gas-budget 10000000"
echo ""

# Initialize the LockRegistry
echo "Initializing LockRegistry..."
"$SUI" client call \
    --package "$PACKAGE_ID" \
    --module csv_seal \
    --function create_registry \
    --gas-budget 10000000 \
    --json 2>&1 | tee "scripts/registry-init-${NETWORK}.json" | tail -3

echo ""
echo "Deployment complete!"
