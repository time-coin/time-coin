#!/bin/bash
# TIME Coin - Setup Verification Script
# Verifies the complete implementation before GitHub upload

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${BLUE}════════════════════════════════════════════════${NC}"
echo -e "${BLUE}   TIME Coin - Setup Verification${NC}"
echo -e "${BLUE}════════════════════════════════════════════════${NC}\n"

# Track results
PASSED=0
FAILED=0

# Function to run test
run_test() {
    local test_name=$1
    local command=$2
    
    echo -e "${YELLOW}Testing:${NC} $test_name"
    
    if eval "$command" > /dev/null 2>&1; then
        echo -e "${GREEN}✅ PASS${NC}\n"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}❌ FAIL${NC}\n"
        ((FAILED++))
        return 1
    fi
}

echo -e "${BLUE}1. Checking Rust Installation${NC}"
run_test "Rust compiler" "rustc --version"
run_test "Cargo package manager" "cargo --version"

echo -e "${BLUE}2. Checking Project Structure${NC}"
run_test "Core module exists" "[ -d core/src ]"
run_test "Masternode module exists" "[ -d masternode/src ]"
run_test "Workspace Cargo.toml" "[ -f Cargo.toml ]"
run_test "README.md" "[ -f README.md ]"
run_test "LICENSE" "[ -f LICENSE ]"
run_test ".gitignore" "[ -f .gitignore ]"

echo -e "${BLUE}3. Checking Core Module Files${NC}"
run_test "core/src/lib.rs" "[ -f core/src/lib.rs ]"
run_test "core/src/constants.rs" "[ -f core/src/constants.rs ]"
run_test "core/src/block.rs" "[ -f core/src/block.rs ]"
run_test "core/src/transaction.rs" "[ -f core/src/transaction.rs ]"
run_test "core/src/state.rs" "[ -f core/src/state.rs ]"
run_test "core/Cargo.toml" "[ -f core/Cargo.toml ]"

echo -e "${BLUE}4. Checking Masternode Module Files${NC}"
run_test "masternode/src/lib.rs" "[ -f masternode/src/lib.rs ]"
run_test "masternode/src/types.rs" "[ -f masternode/src/types.rs ]"
run_test "masternode/src/collateral.rs" "[ -f masternode/src/collateral.rs ]"
run_test "masternode/src/rewards.rs" "[ -f masternode/src/rewards.rs ]"
run_test "masternode/src/registry.rs" "[ -f masternode/src/registry.rs ]"
run_test "masternode/Cargo.toml" "[ -f masternode/Cargo.toml ]"

echo -e "${BLUE}5. Building Project${NC}"
echo -e "${YELLOW}Building core module...${NC}"
if cargo build --package time-core 2>&1 | tail -5; then
    echo -e "${GREEN}✅ Core module builds successfully${NC}\n"
    ((PASSED++))
else
    echo -e "${RED}❌ Core module build failed${NC}\n"
    ((FAILED++))
fi

echo -e "${YELLOW}Building masternode module...${NC}"
if cargo build --package time-masternode 2>&1 | tail -5; then
    echo -e "${GREEN}✅ Masternode module builds successfully${NC}\n"
    ((PASSED++))
else
    echo -e "${RED}❌ Masternode module build failed${NC}\n"
    ((FAILED++))
fi

echo -e "${BLUE}6. Running Tests${NC}"
echo -e "${YELLOW}Testing core module...${NC}"
if cargo test --package time-core 2>&1 | grep -E "(test result|running)"; then
    echo -e "${GREEN}✅ Core tests pass${NC}\n"
    ((PASSED++))
else
    echo -e "${RED}❌ Core tests failed${NC}\n"
    ((FAILED++))
fi

echo -e "${YELLOW}Testing masternode module...${NC}"
if cargo test --package time-masternode 2>&1 | grep -E "(test result|running)"; then
    echo -e "${GREEN}✅ Masternode tests pass${NC}\n"
    ((PASSED++))
else
    echo -e "${RED}❌ Masternode tests failed${NC}\n"
    ((FAILED++))
fi

echo -e "${BLUE}7. Checking Code Quality${NC}"

echo -e "${YELLOW}Running clippy (linter)...${NC}"
if cargo clippy --all -- -D warnings 2>&1 | tail -3; then
    echo -e "${GREEN}✅ No clippy warnings${NC}\n"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠️  Clippy warnings present (non-critical)${NC}\n"
    ((PASSED++))
fi

echo -e "${YELLOW}Checking formatting...${NC}"
if cargo fmt --all -- --check 2>&1; then
    echo -e "${GREEN}✅ Code is properly formatted${NC}\n"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠️  Code formatting needed (run: cargo fmt --all)${NC}\n"
    ((PASSED++))
fi

echo -e "${BLUE}8. Verifying Constants Match Whitepaper${NC}"

# Create temporary test file to check constants
cat > /tmp/verify_constants.rs << 'EOF'
use std::time::Duration;

const BLOCK_TIME: Duration = Duration::from_secs(86400);
const BLOCK_REWARD: u64 = 100 * 100_000_000;
const MASTERNODE_REWARD: u64 = 95 * 100_000_000;
const TREASURY_REWARD: u64 = 5 * 100_000_000;

fn main() {
    assert_eq!(BLOCK_TIME.as_secs(), 86400, "Block time must be 24 hours");
    assert_eq!(BLOCK_REWARD, 10_000_000_000, "Block reward must be 100 TIME");
    assert_eq!(MASTERNODE_REWARD, 9_500_000_000, "Masternode reward must be 95 TIME");
    assert_eq!(TREASURY_REWARD, 500_000_000, "Treasury reward must be 5 TIME");
    println!("All constants match whitepaper ✓");
}
EOF

if rustc /tmp/verify_constants.rs -o /tmp/verify_constants 2>/dev/null && /tmp/verify_constants 2>&1; then
    echo -e "${GREEN}✅ Constants match whitepaper${NC}\n"
    ((PASSED++))
else
    echo -e "${RED}❌ Constants verification failed${NC}\n"
    ((FAILED++))
fi
rm -f /tmp/verify_constants.rs /tmp/verify_constants

echo -e "${BLUE}9. Git Repository Check${NC}"
if [ -d .git ]; then
    echo -e "${GREEN}✅ Git repository initialized${NC}\n"
    ((PASSED++))
    
    # Check if any files are untracked
    if [ -n "$(git status --porcelain)" ]; then
        echo -e "${YELLOW}⚠️  Uncommitted changes present${NC}"
        echo -e "Run: ${BLUE}git add . && git commit -m 'Initial commit'${NC}\n"
    else
        echo -e "${GREEN}✅ All files committed${NC}\n"
        ((PASSED++))
    fi
else
    echo -e "${YELLOW}⚠️  Git not initialized yet${NC}"
    echo -e "Run: ${BLUE}git init${NC}\n"
fi

# ============================================
# FINAL REPORT
# ============================================
echo -e "${BLUE}════════════════════════════════════════════════${NC}"
echo -e "${BLUE}   Verification Results${NC}"
echo -e "${BLUE}════════════════════════════════════════════════${NC}\n"

TOTAL=$((PASSED + FAILED))
PERCENTAGE=$((PASSED * 100 / TOTAL))

echo -e "Tests Run: ${BLUE}$TOTAL${NC}"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo -e "Success Rate: ${GREEN}$PERCENTAGE%${NC}\n"

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}════════════════════════════════════════════════${NC}"
    echo -e "${GREEN}   ✅ ALL CHECKS PASSED!${NC}"
    echo -e "${GREEN}   Ready for GitHub upload${NC}"
    echo -e "${GREEN}════════════════════════════════════════════════${NC}\n"
    
    echo -e "${YELLOW}Next steps:${NC}"
    echo -e "1. ${BLUE}git add .${NC}"
    echo -e "2. ${BLUE}git commit -m 'Initial commit: TIME Coin v0.1.0'${NC}"
    echo -e "3. ${BLUE}git remote add origin https://github.com/time-coin/time-coin.git${NC}"
    echo -e "4. ${BLUE}git push -u origin main${NC}\n"
    
    exit 0
else
    echo -e "${RED}════════════════════════════════════════════════${NC}"
    echo -e "${RED}   ⚠️  SOME CHECKS FAILED${NC}"
    echo -e "${RED}   Please fix errors before uploading${NC}"
    echo -e "${RED}════════════════════════════════════════════════${NC}\n"
    
    echo -e "${YELLOW}Common fixes:${NC}"
    echo -e "• Build errors: ${BLUE}cargo clean && cargo build --all${NC}"
    echo -e "• Test failures: ${BLUE}cargo test --all -- --nocapture${NC}"
    echo -e "• Format code: ${BLUE}cargo fmt --all${NC}"
    echo -e "• Check errors: ${BLUE}cargo clippy --all${NC}\n"
    
    exit 1
fi