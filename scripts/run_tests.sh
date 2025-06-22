#!/bin/bash

# Test runner script for submod integration tests
# This script runs the comprehensive test suite with proper reporting

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
if ! command -v git &> /dev/null; then
    print_error "Git is required for running integration tests"
    exit 1
fi

# Parse command line arguments
VERBOSE=false
PERFORMANCE=false
FILTER=""

while [[ $# -gt 0 ]]; do
    case $1 in
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -p|--performance)
            PERFORMANCE=true
            shift
            ;;
        -f|--filter)
            FILTER="$2"
            shift 2
            ;;
        -h|--help)
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

print_status "Starting submod integration test suite..."

# Build the project first
print_status "Building submod binary..."
if $VERBOSE; then
    cargo build --bin submod
else
    cargo build --bin submod > /dev/null 2>&1
fi

if [[ $? -eq 0 ]]; then
    print_success "Build completed successfully"
else
    print_error "Build failed"
    exit 1
fi

# Function to run a specific test module
run_test_module() {
    local module=$1
    local description=$2

    print_status "Running $description..."

    local cmd="cargo test --test $module"
    if [[ -n "$FILTER" ]]; then
        cmd="$cmd $FILTER"
    fi

    if $VERBOSE; then
        cmd="$cmd -- --nocapture"
    fi

    if eval "$cmd"; then
        print_success "$description completed successfully"
        return 0
    else
        print_error "$description failed"
        return 1
    fi
}

# Track test results
declare -a failed_tests=()
total_tests=0

# Core integration tests
if [[ -z "$FILTER" ]] || [[ "integration_tests" =~ $FILTER ]]; then
    total_tests=$((total_tests + 1))
    if ! run_test_module "integration_tests" "Core integration tests"; then
        failed_tests+=("integration_tests")
    fi
fi

# Configuration tests
if [[ -z "$FILTER" ]] || [[ "config_tests" =~ $FILTER ]]; then
    total_tests=$((total_tests + 1))
    if ! run_test_module "config_tests" "Configuration management tests"; then
        failed_tests+=("config_tests")
    fi
fi

# Sparse checkout tests
if [[ -z "$FILTER" ]] || [[ "sparse_checkout_tests" =~ $FILTER ]]; then
    total_tests=$((total_tests + 1))
    if ! run_test_module "sparse_checkout_tests" "Sparse checkout functionality tests"; then
        failed_tests+=("sparse_checkout_tests")
    fi
fi

# Error handling tests
if [[ -z "$FILTER" ]] || [[ "error_handling_tests" =~ $FILTER ]]; then
    total_tests=$((total_tests + 1))
    if ! run_test_module "error_handling_tests" "Error handling and edge case tests"; then
        failed_tests+=("error_handling_tests")
    fi
fi

# Performance tests (optional)
if $PERFORMANCE; then
    if [[ -z "$FILTER" ]] || [[ "performance_tests" =~ $FILTER ]]; then
        total_tests=$((total_tests + 1))
        print_warning "Running performance tests (this may take a while)..."
        if ! run_test_module "performance_tests" "Performance and stress tests"; then
            failed_tests+=("performance_tests")
        fi
    fi
fi

# Run unit tests as well
if [[ -z "$FILTER" ]] || [[ "unit" =~ $FILTER ]]; then
    print_status "Running unit tests..."
    total_tests=$((total_tests + 1))

    cmd="cargo test --lib"
    if $VERBOSE; then
        cmd="$cmd -- --nocapture"
    fi

    if eval "$cmd"; then
        print_success "Unit tests completed successfully"
    else
        print_error "Unit tests failed"
        failed_tests+=("unit_tests")
    fi
fi

# Report results
echo ""
print_status "Test Suite Summary"
echo "=================="

if [[ ${#failed_tests[@]} -eq 0 ]]; then
    print_success "All $total_tests test modules passed! 🎉"
    echo ""
    echo "The submod tool is working correctly and ready for use."
    exit 0
else
    print_error "${#failed_tests[@]} out of $total_tests test modules failed:"
    for test in "${failed_tests[@]}"; do
        echo "  - $test"
    done
    echo ""
    echo "Please check the test output above for details on the failures."
    exit 1
fi
