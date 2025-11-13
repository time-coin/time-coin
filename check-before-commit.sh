#!/bin/bash
# Pre-commit check script for time-coin project

echo "🔍 Running pre-commit checks..."
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ERRORS=0

# Check 1: Cargo fmt
echo "📝 Checking code formatting..."
if cargo fmt --check; then
    echo -e "${GREEN}✓ Code formatting passed${NC}"
else
    echo -e "${RED}✗ Code formatting failed${NC}"
    echo -e "${YELLOW}  Run: cargo fmt${NC}"
    ERRORS=$((ERRORS + 1))
    cargo fmt
fi
echo ""

# Check 2: Cargo clippy
echo "📎 Running clippy lints..."
if cargo clippy --all-targets --all-features -- -D warnings; then
    echo -e "${GREEN}✓ Clippy passed${NC}"
else
    echo -e "${RED}✗ Clippy found issues${NC}"
    echo -e "${YELLOW}  Fix warnings or run: cargo clippy --fix${NC}"
    ERRORS=$((ERRORS + 1))
    cargo clippy --fix --allow-dirty --allow-staged
fi
echo ""

# Check 3: Cargo check
echo "🔧 Checking compilation..."
if cargo check --all-targets --all-features; then
    echo -e "${GREEN}✓ Compilation check passed${NC}"
else
    echo -e "${RED}✗ Compilation check failed${NC}"
    ERRORS=$((ERRORS + 1))
fi
echo ""

# Check 4: Cargo test
echo "🧪 Running tests..."
if cargo test --all-features; then
    echo -e "${GREEN}✓ Cargo tests passed${NC}"
else
    echo -e "${RED}✗ Cargo tests failed${NC}"
    ERRORS=$((ERRORS + 1))
fi
echo ""

# Check 5: Cargo Deny Check
echo "  Running Cargo Deny Tests..."
if cargo deny check --hide-inclusion-graph; then
    echo -e "${GREEN}✓ Cargo Deny Tests passed${NC}"
else
    echo -e "${RED}✗ Cargo Deny Tests failed${NC}"
    ERRORS=$((ERRORS + 1))
fi
echo ""

# Summary
echo "═══════════════════════════════════════════════════════════"
if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed! Safe to commit.${NC}"
    exit 0
else
    echo -e "${RED}✗ $ERRORS check(s) failed. Please fix before committing.${NC}"
    exit 1
fi
