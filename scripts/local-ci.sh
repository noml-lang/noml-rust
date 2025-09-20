#!/bin/bash
# Local CI validation script - run this before pushing!

set -e

echo "🚀 Running local CI validation for NOML..."
echo

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

run_check() {
    echo -e "${YELLOW}▶ $1${NC}"
    if eval "$2"; then
        echo -e "${GREEN}✓ $1 passed${NC}"
    else
        echo -e "${RED}✗ $1 failed${NC}"
        exit 1
    fi
    echo
}

# Basic compilation check
run_check "Checking compilation" "cargo check --all-targets --all-features"

# Code formatting
run_check "Checking code formatting" "cargo fmt --all -- --check"

# Clippy linting
run_check "Running Clippy" "cargo clippy --all-targets --all-features -- -D warnings"

# Tests
run_check "Running all tests" "cargo test --all-features"

# Test with no features
run_check "Testing with no features" "cargo test --no-default-features"

# Test individual features
run_check "Testing async feature" "cargo test --no-default-features --features async"
run_check "Testing chrono feature" "cargo test --no-default-features --features chrono"

# Documentation tests
run_check "Testing documentation" "cargo test --doc --all-features"

# Documentation build
run_check "Building documentation" "cargo doc --all-features --no-deps"

# Examples
run_check "Testing examples" "cargo run --example demo"

# Security audit (if tools are installed)
if command -v cargo-audit >/dev/null 2>&1; then
    run_check "Security audit" "cargo audit"
else
    echo -e "${YELLOW}⚠ cargo-audit not installed, skipping security audit${NC}"
fi

if command -v cargo-deny >/dev/null 2>&1; then
    echo -e "${YELLOW}⚠ cargo-deny found but config format has changed, skipping dependency check${NC}"
    # run_check "Dependency check" "cargo deny check"
else
    echo -e "${YELLOW}⚠ cargo-deny not installed, skipping dependency check${NC}"
fi

# Benchmarks (optional)
if [[ "${1:-}" == "--bench" ]]; then
    run_check "Running benchmarks" "cargo bench --all-features"
fi

echo -e "${GREEN}🎉 All local CI checks passed! Ready to push.${NC}"
echo
echo "To install missing tools:"
echo "  cargo install cargo-audit cargo-deny"
echo
echo "To run with benchmarks:"
echo "  ./scripts/local-ci.sh --bench"