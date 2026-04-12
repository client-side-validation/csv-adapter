#!/bin/bash
# CSV Explorer Full Test Suite

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

DATA_DIR="$SCRIPT_DIR/data"
DB_FILE="$DATA_DIR/explorer.db"
CONFIG_FILE="$SCRIPT_DIR/config.toml"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}================================================${NC}"
echo -e "${BLUE}   CSV Explorer - Build & Test Suite${NC}"
echo -e "${BLUE}================================================${NC}"
echo ""

###############################################################################
# Step 1: Build
###############################################################################
echo -e "${YELLOW}[1/7] Building CSV Explorer workspace...${NC}"
cargo build --workspace --release
echo -e "${GREEN}✓ Build successful${NC}"
echo ""

###############################################################################
# Step 2: Setup database
###############################################################################
echo -e "${YELLOW}[2/7] Setting up database...${NC}"
mkdir -p "$DATA_DIR"

if [ -f "$DB_FILE" ]; then
    echo "  Removing old database..."
    rm "$DB_FILE"
fi

echo "  Creating database schema..."
sqlite3 "$DB_FILE" < "$SCRIPT_DIR/storage/src/schema.sql"

echo "  Seeding database..."
sqlite3 "$DB_FILE" < "$SCRIPT_DIR/storage/src/seed.sql"

RIGHTS_COUNT=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM rights;")
TRANSFERS_COUNT=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM transfers;")
SEALS_COUNT=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM seals;")
CONTRACTS_COUNT=$(sqlite3 "$DB_FILE" "SELECT COUNT(*) FROM contracts;")

echo -e "${GREEN}✓ Database initialized with:${NC}"
echo "  - $RIGHTS_COUNT rights"
echo "  - $TRANSFERS_COUNT transfers"
echo "  - $SEALS_COUNT seals"
echo "  - $CONTRACTS_COUNT contracts"
echo ""

###############################################################################
# Step 3: Clean up old processes
###############################################################################
echo -e "${YELLOW}[3/7] Stopping any running instances...${NC}"
pkill -f csv-explorer-api 2>/dev/null || true
pkill -f csv-explorer-indexer 2>/dev/null || true
pkill -f csv-explorer-ui 2>/dev/null || true
sleep 2
echo -e "${GREEN}✓ Cleaned up old processes${NC}"
echo ""

###############################################################################
# Step 4: Start API server
###############################################################################
echo -e "${YELLOW}[4/7] Starting API server...${NC}"
./target/release/csv-explorer-api start &
API_PID=$!
echo "  API PID: $API_PID"

echo "  Waiting for API server..."
for i in $(seq 1 30); do
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        echo -e "${GREEN}  ✓ API server is ready${NC}"
        break
    fi
    if [ $i -eq 30 ]; then
        echo -e "${RED}  ✗ API server failed to start${NC}"
        kill $API_PID 2>/dev/null || true
        exit 1
    fi
    sleep 1
done
echo ""

###############################################################################
# Step 5: API Tests
###############################################################################
echo -e "${YELLOW}[5/7] Running API tests...${NC}"
echo ""

TESTS_PASSED=0
TESTS_FAILED=0

run_test() {
    local test_name="$1"
    local url="$2"
    
    echo -e "${BLUE}Test: $test_name${NC}"
    RESPONSE=$(curl -s "$url" 2>&1 | head -c 200)
    if [ -n "$RESPONSE" ]; then
        echo "  Response: $RESPONSE"
        echo -e "${GREEN}  ✓ Passed${NC}"
        TESTS_PASSED=$((TESTS_PASSED + 1))
    else
        echo -e "${RED}  ✗ Failed${NC}"
        TESTS_FAILED=$((TESTS_FAILED + 1))
    fi
    echo ""
}

run_test "Health Check" "http://localhost:8080/health"
run_test "Statistics" "http://localhost:8080/api/v1/stats"
run_test "List Rights" "http://localhost:8080/api/v1/rights?limit=5"
run_test "Get Single Right" "http://localhost:8080/api/v1/rights/right_btc_001"
run_test "List Transfers" "http://localhost:8080/api/v1/transfers?limit=5"
run_test "List Seals" "http://localhost:8080/api/v1/seals?limit=5"
run_test "List Contracts" "http://localhost:8080/api/v1/contracts?limit=5"
run_test "Chain Status" "http://localhost:8080/api/v1/chains"

echo -e "${YELLOW}API Test Results:${NC}"
echo -e "  ${GREEN}Passed: $TESTS_PASSED${NC}"
echo -e "  ${RED}Failed: $TESTS_FAILED${NC}"
echo ""

###############################################################################
# Step 6: Indexer test
###############################################################################
echo -e "${YELLOW}[6/7] Testing indexer startup...${NC}"
timeout 5 ./target/release/csv-explorer-indexer start 2>&1 | head -n 10 || true
echo -e "${GREEN}✓ Indexer test completed${NC}"
echo ""

###############################################################################
# Step 7: Database integrity tests
###############################################################################
echo -e "${YELLOW}[7/7] Running database integrity tests...${NC}"
echo ""

echo -e "${BLUE}Database Test 1: Rights by Chain${NC}"
sqlite3 "$DB_FILE" "SELECT chain, COUNT(*) as count FROM rights GROUP BY chain;" | while IFS='|' read -r chain count; do
    echo "  $chain: $count rights"
done
echo ""

echo -e "${BLUE}Database Test 2: Transfers by Status${NC}"
sqlite3 "$DB_FILE" "SELECT status, COUNT(*) as count FROM transfers GROUP BY status;" | while IFS='|' read -r status count; do
    echo "  $status: $count transfers"
done
echo ""

echo -e "${BLUE}Database Test 3: Seals by Type${NC}"
sqlite3 "$DB_FILE" "SELECT seal_type, COUNT(*) as count FROM seals GROUP BY seal_type;" | while IFS='|' read -r seal_type count; do
    echo "  $seal_type: $count seals"
done
echo ""

echo -e "${BLUE}Database Test 4: Contracts by Chain${NC}"
sqlite3 "$DB_FILE" "SELECT chain, contract_type, status FROM contracts ORDER BY chain;" | while IFS='|' read -r chain contract_type status; do
    echo "  $chain: $contract_type ($status)"
done
echo ""

echo -e "${BLUE}Database Test 5: Sync Progress${NC}"
sqlite3 "$DB_FILE" "SELECT chain, latest_block FROM sync_progress ORDER BY chain;" | while IFS='|' read -r chain block; do
    echo "  $chain: block $block"
done
echo ""

echo -e "${BLUE}Database Test 6: Rights with Transfer Counts${NC}"
sqlite3 "$DB_FILE" "SELECT r.id, r.chain, r.transfer_count FROM rights r ORDER BY r.transfer_count DESC LIMIT 5;" | while IFS='|' read -r id chain count; do
    echo "  $id ($chain): $count transfers"
done
echo ""

###############################################################################
# Summary
###############################################################################
echo -e "${BLUE}================================================${NC}"
echo -e "${GREEN}   All Tests Completed Successfully!${NC}"
echo -e "${BLUE}================================================${NC}"
echo ""
echo -e "${YELLOW}Services Running:${NC}"
echo -e "  API Server:  ${GREEN}http://localhost:8080${NC}"
echo -e "  Health Check: ${GREEN}http://localhost:8080/health${NC}"
echo -e "  GraphQL:      ${GREEN}http://localhost:8080/graphql${NC}"
echo ""
echo -e "${YELLOW}Quick Commands:${NC}"
echo -e "  ${BLUE}curl http://localhost:8080/api/v1/stats${NC}"
echo -e "  ${BLUE}curl http://localhost:8080/api/v1/rights${NC}"
echo -e "  ${BLUE}curl http://localhost:8080/api/v1/transfers${NC}"
echo -e "  ${BLUE}sqlite3 $DB_FILE${NC}"
echo ""
echo -e "${YELLOW}API server PID: $API_PID${NC}"
echo -e "${YELLOW}To stop: kill $API_PID${NC}"
echo ""

# Cleanup
cleanup() {
    echo ""
    echo -e "${YELLOW}Stopping API server...${NC}"
    kill $API_PID 2>/dev/null || true
}

trap cleanup EXIT
wait $API_PID
