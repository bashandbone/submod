#!/bin/bash

# SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
#
# SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

# Test runner script for submod integration tests
# This script runs the comprehensive test suite with proper reporting
#
# Test parallelism is managed by nextest test groups in .config/nextest.toml.
# Integration tests that modify git repos run serially within their group;
# other tests (unit, config, contract) run in parallel.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if we're in the right directory
if [[ ! -f "Cargo.toml" ]] || [[ ! -d "src" ]] || [[ ! -d "tests" ]]; then
    print_error "Please run this script from the project root directory"
    exit 1
fi

# Check if git is available
if ! command -v git &>/dev/null; then
    print_error "Git is required for running integration tests"
    exit 1
fi

# Parse command line arguments
VERBOSE=false
PERFORMANCE=false
FILTER=""

while [[ $# -gt 0 ]]; do
    case $1 in
    -v | --verbose)
        VERBOSE=true
        shift
        ;;
    -p | --performance)
        PERFORMANCE=true
        shift
        ;;
    -f | --filter)
        FILTER="$2"
        shift 2
        ;;
    -h | --help)
        echo "Usage: $0 [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  -v, --verbose      Enable verbose output"
        echo "  -p, --performance  Run performance tests"
        echo "  -f, --filter PATTERN  Run only tests matching PATTERN"
        echo "  -h, --help         Show this help message"
        exit 0
        ;;
    *)
        print_error "Unknown option: $1"
        exit 1
        ;;
    esac
done

print_status "Starting submod test suite..."

# Build the project first
print_status "Building submod binary..."
PROFILE="test"
if [[ "$PERFORMANCE" == true ]]; then
    PROFILE="bench"
fi

if $VERBOSE; then
    cargo build --bin submod --profile "$PROFILE"
else
    cargo build --bin submod --profile "$PROFILE" >/dev/null 2>&1
fi
print_success "Build completed successfully"

# Build the nextest command
NEXTEST_ARGS=(
    nextest --manifest-path ./Cargo.toml run
    --all-features
    --no-fail-fast
)

# Build the filterset expression
FILTERSET=""

# Exclude performance tests unless explicitly requested
if [[ "$PERFORMANCE" != true ]]; then
    FILTERSET="not binary_id(submod::performance_tests)"
fi

# Apply filter if provided
if [[ -n "$FILTER" ]]; then
    if [[ -n "$FILTERSET" ]]; then
        FILTERSET="($FILTERSET) & test(/$FILTER/)"
    else
        FILTERSET="test(/$FILTER/)"
    fi
fi

if [[ -n "$FILTERSET" ]]; then
    NEXTEST_ARGS+=(-E "$FILTERSET")
fi

# Run the full suite in a single nextest invocation so that test groups
# can schedule serial and parallel tests optimally.
print_status "Running tests..."
if $VERBOSE; then
    print_status "cargo ${NEXTEST_ARGS[*]}"
fi

if cargo "${NEXTEST_ARGS[@]}"; then
    echo ""
    print_success "All tests passed!"
    exit 0
else
    echo ""
    print_error "Some tests failed. See output above for details."
    exit 1
fi
