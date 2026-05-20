#!/bin/bash
# CI Enforcement Script for Forbidden Patterns
# 
# This script checks for forbidden patterns in the codebase as specified in AUDIT_2.md
# CI MUST fail if any forbidden patterns are found in protocol modules.
#
# Forbidden Patterns:
# - TODO in protocol modules
# - FIXME in protocol modules
# - unwrap() in runtime code
# - expect() in runtime code
# - unsafe outside approved modules (csv-crypto/, csv-zk/)
# - raw hashing outside crypto module (Sha256::digest, Keccak256::digest, blake3::hash)
# - mock proofs in production code
# - manual ABI encoding in EVM adapters

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track overall status
FAILED=0

echo "=== Checking for forbidden patterns ==="
echo ""

# Function to check pattern in specific directories
check_pattern() {
    local pattern="$1"
    local description="$2"
    local dirs="$3"
    local exclude_dirs="$4"
    
    echo -n "Checking for $description... "
    
    # Build exclude arguments
    exclude_args=""
    if [ -n "$exclude_dirs" ]; then
        for dir in $exclude_dirs; do
            exclude_args="$exclude_args --exclude-dir=$dir"
        done
    fi
    
    # Search for pattern
    if grep -r "$pattern" $dirs $exclude_args --include="*.rs" 2>/dev/null; then
        echo -e "${RED}FAILED${NC}"
        echo "  Found $description in the following files:"
        grep -rn "$pattern" $dirs $exclude_args --include="*.rs" 2>/dev/null | head -20
        FAILED=1
    else
        echo -e "${GREEN}PASSED${NC}"
    fi
}

# 1. Check for TODO in protocol modules
echo "--- Protocol Module Checks ---"
check_pattern "TODO" "TODO comments" "csv-core/src csv-bitcoin/src csv-ethereum/src csv-aptos/src csv-solana/src" "tests fuzz benches"

# 2. Check for FIXME in protocol modules
check_pattern "FIXME" "FIXME comments" "csv-core/src csv-bitcoin/src csv-ethereum/src csv-aptos/src csv-solana/src" "tests fuzz benches"

# 3. Check for unwrap() in runtime code
echo ""
echo "--- Runtime Code Safety Checks ---"
check_pattern "\.unwrap()" "unwrap() calls" "csv-core/src csv-bitcoin/src csv-ethereum/src csv-aptos/src csv-solana/src" "tests fuzz benches"

# 4. Check for expect() in runtime code
check_pattern "\.expect(" "expect() calls" "csv-core/src csv-bitcoin/src csv-ethereum/src csv-aptos/src csv-solana/src" "tests fuzz benches"

# 5. Check for unsafe outside approved modules
echo ""
echo "--- Unsafe Code Checks ---"
# Check for unsafe in non-approved modules
if grep -r "unsafe" csv-core/src csv-bitcoin/src csv-ethereum/src csv-aptos/src csv-solana/src --include="*.rs" --exclude-dir=tests --exclude-dir=fuzz --exclude-dir=benches 2>/dev/null | grep -v "// SAFETY:" | grep -v "/// SAFETY:"; then
    echo -e "${RED}FAILED${NC}"
    echo "  Found unsafe blocks without SAFETY comments:"
    grep -rn "unsafe" csv-core/src csv-bitcoin/src csv-ethereum/src csv-aptos/src csv-solana/src --include="*.rs" --exclude-dir=tests --exclude-dir=fuzz --exclude-dir=benches 2>/dev/null | grep -v "// SAFETY:" | grep -v "/// SAFETY:" | head -20
    FAILED=1
else
    echo -e "${GREEN}PASSED${NC} (all unsafe blocks have SAFETY comments)"
fi

# 6. Check for raw hashing outside crypto module
echo ""
echo "--- Raw Hashing Checks ---"
# Check for Sha256::digest outside csv-crypto/
if grep -r "Sha256::digest" csv-core/src csv-bitcoin/src csv-ethereum/src csv-aptos/src csv-solana/src --include="*.rs" --exclude-dir=tests --exclude-dir=fuzz --exclude-dir=benches 2>/dev/null; then
    echo -e "${RED}FAILED${NC}"
    echo "  Found direct Sha256::digest calls (should use DomainSeparatedHash):"
    grep -rn "Sha256::digest" csv-core/src csv-bitcoin/src csv-ethereum/src csv-aptos/src csv-solana/src --include="*.rs" --exclude-dir=tests --exclude-dir=fuzz --exclude-dir=benches 2>/dev/null | head -20
    FAILED=1
else
    echo -e "${GREEN}PASSED${NC} (no direct Sha256::digest calls)"
fi

# Check for Keccak256::digest outside csv-crypto/
if grep -r "Keccak256::digest" csv-core/src csv-bitcoin/src csv-ethereum/src csv-aptos/src csv-solana/src --include="*.rs" --exclude-dir=tests --exclude-dir=fuzz --exclude-dir=benches 2>/dev/null; then
    echo -e "${RED}FAILED${NC}"
    echo "  Found direct Keccak256::digest calls (should use DomainSeparatedHash):"
    grep -rn "Keccak256::digest" csv-core/src csv-bitcoin/src csv-ethereum/src csv-aptos/src csv-solana/src --include="*.rs" --exclude-dir=tests --exclude-dir=fuzz --exclude-dir=benches 2>/dev/null | head -20
    FAILED=1
else
    echo -e "${GREEN}PASSED${NC} (no direct Keccak256::digest calls)"
fi

# Check for blake3::hash outside csv-crypto/
if grep -r "blake3::hash" csv-core/src csv-bitcoin/src csv-ethereum/src csv-aptos/src csv-solana/src --include="*.rs" --exclude-dir=tests --exclude-dir=fuzz --exclude-dir=benches 2>/dev/null; then
    echo -e "${RED}FAILED${NC}"
    echo "  Found direct blake3::hash calls (should use DomainSeparatedHash):"
    grep -rn "blake3::hash" csv-core/src csv-bitcoin/src csv-ethereum/src csv-aptos/src csv-solana/src --include="*.rs" --exclude-dir=tests --exclude-dir=fuzz --exclude-dir=benches 2>/dev/null | head -20
    FAILED=1
else
    echo -e "${GREEN}PASSED${NC} (no direct blake3::hash calls)"
fi

# 7. Check for mock proofs in production code
echo ""
echo "--- Mock Proof Checks ---"
check_pattern "mock_proof\|MockProof\|MOCK_PROOF" "mock proofs" "csv-core/src csv-bitcoin/src csv-ethereum/src csv-aptos/src csv-solana/src" "tests fuzz benches"

# 8. Check for verify_structure_only in production modules
echo ""
echo "--- LedgerProof Structural Mock Checks ---"
check_pattern "verify_structure_only\b" "LedgerProof structural mock in production" "csv-aptos/src csv-core/src" "tests fuzz"

# 9. Check for manual ABI encoding in EVM adapters
echo ""
echo "--- Manual ABI Encoding Checks ---"
if grep -r "build_abi_call\|manual_selector\|abi\.encode" csv-ethereum/src --include="*.rs" --exclude-dir=tests --exclude-dir=fuzz 2>/dev/null; then
    echo -e "${RED}FAILED${NC}"
    echo "  Found manual ABI encoding in EVM adapters (should use generated bindings):"
    grep -rn "build_abi_call\|manual_selector\|abi\.encode" csv-ethereum/src --include="*.rs" --exclude-dir=tests --exclude-dir=fuzz 2>/dev/null | head -20
    FAILED=1
else
    echo -e "${GREEN}PASSED${NC} (no manual ABI encoding)"
fi

# Final result
echo ""
echo "=== Final Result ==="
if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}All checks passed!${NC}"
    exit 0
else
    echo -e "${RED}Some checks failed. CI will block this commit.${NC}"
    exit 1
fi
