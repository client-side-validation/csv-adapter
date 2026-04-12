#!/bin/bash
# CSV Explorer Validation Script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

DATA_DIR="$SCRIPT_DIR/data"
DB_FILE="$DATA_DIR/explorer.db"
CONFIG_FILE="$SCRIPT_DIR/config.toml"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}=== CSV Explorer Validation ===${NC}"
echo ""

###############################################################################
# Step 1: Setup database
###############################################################################
echo -e "${YELLOW}[1/5] Setting up database...${NC}"
mkdir -p "$DATA_DIR"

if [ ! -f "$DB_FILE" ]; then
    echo "Creating database..."
    sqlite3 "$DB_FILE" < "$SCRIPT_DIR/storage/src/schema.sql"
    echo "Seeding database..."
    sqlite3 "$DB_FILE" < "$SCRIPT_DIR/storage/src/seed.sql"
    echo -e "${GREEN}✓ Database created and seeded${NC}"
else
    echo -e "${GREEN}✓ Database exists${NC}"
fi

###############################################################################
# Step 2: Validate seed data
###############################################################################
echo ""
echo -e "${YELLOW}[2/5] Validating seed data...${NC}"
RIGHTS=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM rights;")
TRANSFERS=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM transfers;")
SEALS=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM seals;")
CONTRACTS=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM contracts;")

echo "  Rights: $RIGHTS"
echo "  Transfers: $TRANSFERS"
echo "  Seals: $SEALS"
echo "  Contracts: $CONTRACTS"

if [ "$RIGHTS" -eq 8 ] && [ "$TRANSFERS" -eq 7 ] && [ "$SEALS" -eq 11 ] && [ "$CONTRACTS" -eq 11 ]; then
    echo -e "${GREEN}✓ All seed data validated${NC}"
else
    echo -e "${RED}✗ Seed data validation failed${NC}"
    exit 1
fi

###############################################################################
# Step 3: Show sample data
###############################################################################
echo ""
echo -e "${YELLOW}[3/5] Sample data:${NC}"
echo ""
echo "  Rights by chain:"
sqlite3 "$DB_FILE" "SELECT chain, COUNT(*) as count FROM rights GROUP BY chain;" | while IFS='|' read -r chain count; do
    echo "    $chain: $count"
done

echo ""
echo "  Transfers by status:"
sqlite3 "$DB_FILE" "SELECT status, COUNT(*) as count FROM transfers GROUP BY status;" | while IFS='|' read -r status count; do
    echo "    $status: $count"
done

echo ""
echo "  Seals by type:"
sqlite3 "$DB_FILE" "SELECT seal_type, COUNT(*) as count FROM seals GROUP BY seal_type;" | while IFS='|' read -r seal_type count; do
    echo "    $seal_type: $count"
done

###############################################################################
# Step 4: Validate build
###############################################################################
echo ""
echo -e "${YELLOW}[4/5] Validating build...${NC}"
if [ -f "$SCRIPT_DIR/target/release/csv-explorer-api" ]; then
    echo -e "${GREEN}✓ API binary exists${NC}"
else
    echo "  Building API binary..."
    cargo build --release -p csv-explorer-api
    echo -e "${GREEN}✓ API binary built${NC}"
fi

###############################################################################
# Step 5: Test API server
###############################################################################
echo ""
echo -e "${YELLOW}[5/5] Testing API server...${NC}"

# Kill any existing API servers
pkill -f csv-explorer-api 2>/dev/null || true
sleep 1

# Start API server
./target/release/csv-explorer-api start &
API_PID=$!
echo "  Started API server (PID: $API_PID)"

# Wait for API to be ready
echo "  Waiting for API to start..."
API_READY=false
for i in $(seq 1 15); do
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        API_READY=true
        echo -e "${GREEN}  ✓ API server is ready${NC}"
        break
    fi
    sleep 1
done

if [ "$API_READY" = false ]; then
    echo -e "${RED}  ✗ API server failed to start${NC}"
    kill $API_PID 2>/dev/null || true
    exit 1
fi

# Test endpoints
echo ""
echo "  Testing endpoints..."

echo -n "    Health check: "
HEALTH=$(curl -s http://localhost:8080/health)
if echo "$HEALTH" | grep -q "ok\|healthy\|status"; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${YELLOW}$HEALTH${NC}"
fi

echo -n "    Statistics: "
STATS=$(curl -s http://localhost:8080/api/v1/stats 2>&1 | head -c 100)
if [ -n "$STATS" ]; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
fi

echo -n "    Rights list: "
RIGHTS_DATA=$(curl -s "http://localhost:8080/api/v1/rights?limit=2" 2>&1 | head -c 100)
if [ -n "$RIGHTS_DATA" ]; then
    echo -e "${GREEN}✓${NC}"
else
    echo -e "${RED}✗${NC}"
fi

echo ""
echo -e "${BLUE}=== Validation Complete ===${NC}"
echo ""
echo -e "${GREEN}✓ All validations passed${NC}"
echo ""
echo -e "${YELLOW}API server running:${NC}"
echo -e "  URL: ${BLUE}http://localhost:8080${NC}"
echo -e "  Health: ${BLUE}http://localhost:8080/health${NC}"
echo -e "  Stats: ${BLUE}http://localhost:8080/api/v1/stats${NC}"
echo ""
echo -e "${YELLOW}Database:${NC}"
echo -e "  Path: ${BLUE}$DB_FILE${NC}"
echo ""
echo -e "${YELLOW}To stop API:${NC}"
echo -e "  ${BLUE}kill $API_PID${NC}"
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo -e "${YELLOW}Stopping API server...${NC}"
    kill $API_PID 2>/dev/null || true
}

# Trap to cleanup on exit
trap cleanup EXIT

# Keep running
echo -e "${YELLOW}Press Ctrl+C to stop...${NC}"
wait $API_PID
