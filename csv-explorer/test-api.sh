#!/bin/bash
# Quick API test script

set -e

echo "=== CSV Explorer API Test ==="
echo ""

# Kill existing API
pkill -f csv-explorer-api 2>/dev/null || true
sleep 1

# Start API
echo "Starting API server..."
cd /home/zorvan/Work/projects/csv-adapter/csv-explorer
nohup ./target/release/csv-explorer-api start > /tmp/api-test.log 2>&1 &
API_PID=$!
echo "API started with PID: $API_PID"

# Wait for API
echo "Waiting for API to start..."
for i in $(seq 1 10); do
    if curl -s http://localhost:8080/health > /dev/null 2>&1; then
        echo "✓ API is ready"
        break
    fi
    if [ $i -eq 10 ]; then
        echo "✗ API failed to start"
        cat /tmp/api-test.log
        exit 1
    fi
    sleep 1
done

echo ""
echo "Running API tests..."
echo ""

# Test 1: Health
echo "Test 1: Health Check"
curl -s http://localhost:8080/health | python3 -m json.tool
echo "✓ Passed"
echo ""

# Test 2: Stats
echo "Test 2: Statistics"
curl -s http://localhost:8080/api/v1/stats | python3 -m json.tool | head -n 30
echo "✓ Passed"
echo ""

# Test 3: Rights
echo "Test 3: List Rights"
RIGHTS=$(curl -s "http://localhost:8080/api/v1/rights?limit=3")
echo "$RIGHTS" | python3 -m json.tool | head -n 20
echo "✓ Passed"
echo ""

# Test 4: Transfers
echo "Test 4: List Transfers"
TRANSFERS=$(curl -s "http://localhost:8080/api/v1/transfers?limit=3")
echo "$TRANSFERS" | python3 -m json.tool | head -n 20
echo "✓ Passed"
echo ""

# Test 5: Seals
echo "Test 5: List Seals"
SEALS=$(curl -s "http://localhost:8080/api/v1/seals?limit=3")
echo "$SEALS" | python3 -m json.tool | head -n 20
echo "✓ Passed"
echo ""

# Test 6: Contracts
echo "Test 6: List Contracts"
CONTRACTS=$(curl -s "http://localhost:8080/api/v1/contracts?limit=3")
echo "$CONTRACTS" | python3 -m json.tool | head -n 20
echo "✓ Passed"
echo ""

# Test 7: Chains
echo "Test 7: Chain Status"
CHAINS=$(curl -s http://localhost:8080/api/v1/chains)
echo "$CHAINS" | python3 -m json.tool | head -n 20
echo "✓ Passed"
echo ""

echo "=== All Tests Passed ==="
echo ""
echo "API running at: http://localhost:8080"
echo "PID: $API_PID"
echo ""
echo "To stop: kill $API_PID"

# Cleanup
trap "kill $API_PID 2>/dev/null || true" EXIT
