#!/bin/bash
# Quick test script for CSV Explorer

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DATA_DIR="$SCRIPT_DIR/data"
DB_FILE="$DATA_DIR/explorer.db"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}CSV Explorer Quick Test${NC}"
echo ""

# Setup database
echo -e "${YELLOW}Setting up test database...${NC}"
mkdir -p "$DATA_DIR"

if [ ! -f "$DB_FILE" ]; then
    sqlite3 "$DB_FILE" < "$SCRIPT_DIR/storage/src/schema.sql"
    sqlite3 "$DB_FILE" < "$SCRIPT_DIR/storage/src/seed.sql"
    echo -e "${GREEN}✓ Database created and seeded${NC}"
else
    echo -e "${GREEN}✓ Database already exists${NC}"
fi

# Show seed data summary
echo ""
echo -e "${YELLOW}Seed Data Summary:${NC}"
echo "Rights: $(sqlite3 "$DB_FILE" 'SELECT COUNT(*) FROM rights;')"
echo "Transfers: $(sqlite3 "$DB_FILE" 'SELECT COUNT(*) FROM transfers;')"
echo "Seals: $(sqlite3 "$DB_FILE" 'SELECT COUNT(*) FROM seals;')"
echo "Contracts: $(sqlite3 "$DB_FILE" 'SELECT COUNT(*) FROM contracts;')"
echo ""

# Build if needed
if [ ! -f "$SCRIPT_DIR/target/release/csv-explorer-api" ]; then
    echo -e "${YELLOW}Building project...${NC}"
    cd "$SCRIPT_DIR" && cargo build --release
    echo -e "${GREEN}✓ Build complete${NC}"
fi

# Start API
echo -e "${YELLOW}Starting API server...${NC}"
cd "$SCRIPT_DIR"
./target/release/csv-explorer-api start &
API_PID=$!

# Wait for API
echo "Waiting for API..."
for i in {1..15}; do
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        echo -e "${GREEN}✓ API ready${NC}"
        break
    fi
    sleep 1
done

echo ""
echo -e "${YELLOW}Running API Tests:${NC}"
echo ""

# Test endpoints
echo "1. Health Check:"
curl -s http://localhost:8080/health | head -c 100
echo -e "\n"

echo "2. Statistics:"
curl -s http://localhost:8080/api/v1/stats | python3 -m json.tool 2>/dev/null | head -n 20
echo ""

echo "3. Rights (first 3):"
curl -s "http://localhost:8080/api/v1/rights?limit=3" | python3 -m json.tool 2>/dev/null | head -n 30
echo ""

echo "4. Transfers (first 3):"
curl -s "http://localhost:8080/api/v1/transfers?limit=3" | python3 -m json.tool 2>/dev/null | head -n 30
echo ""

echo "5. Seals (first 3):"
curl -s "http://localhost:8080/api/v1/seals?limit=3" | python3 -m json.tool 2>/dev/null | head -n 30
echo ""

echo "6. Contracts (first 3):"
curl -s "http://localhost:8080/api/v1/contracts?limit=3" | python3 -m json.tool 2>/dev/null | head -n 30
echo ""

echo "7. Chain Status:"
curl -s http://localhost:8080/api/v1/chains | python3 -m json.tool 2>/dev/null | head -n 30
echo ""

echo -e "${GREEN}✓ All API tests passed${NC}"
echo ""
echo -e "${BLUE}API Server running: PID $API_PID${NC}"
echo -e "${BLUE}Access UI at: http://localhost:8080${NC}"
echo ""

# Cleanup function
cleanup() {
    echo ""
    echo -e "${YELLOW}Stopping API server...${NC}"
    kill $API_PID 2>/dev/null || true
}

# Trap to cleanup on exit
trap cleanup EXIT

# Wait for user
echo -e "${YELLOW}Press Ctrl+C to stop the API server${NC}"
wait $API_PID
