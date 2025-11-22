#!/bin/bash

# OpenChime Test Runner Script
# Provides convenient commands for running different test categories

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to run unit tests
run_unit_tests() {
    print_status "Running unit tests..."
    cargo test --lib -- --nocapture
}

# Function to run integration tests
run_integration_tests() {
    print_status "Running integration tests..."
    cargo test --test '*' -- --nocapture --ignored
}

# Function to run E2E tests
run_e2e_tests() {
    print_status "Running end-to-end tests..."
    print_warning "E2E tests require GUI environment and may take longer"
    cargo test --test e2e_app_workflows -- --nocapture --ignored
}

# Function to run all tests
run_all_tests() {
    print_status "Running all tests..."
    cargo test --all -- --nocapture
}

# Function to run tests with coverage (requires cargo-tarpaulin)
run_coverage() {
    if command -v cargo-tarpaulin &> /dev/null; then
        print_status "Running tests with coverage..."
        cargo tarpaulin --out Html --output-dir coverage/
    else
        print_error "cargo-tarpaulin not found. Install with: cargo install cargo-tarpaulin"
        exit 1
    fi
}

# Function to run tests in watch mode
run_watch_tests() {
    if command -v cargo-watch &> /dev/null; then
        print_status "Running tests in watch mode..."
        cargo watch -x "test --all -- --nocapture"
    else
        print_error "cargo-watch not found. Install with: cargo install cargo-watch"
        exit 1
    fi
}

# Function to check code formatting
check_formatting() {
    print_status "Checking code formatting..."
    cargo fmt -- --check
}

# Function to run clippy
run_clippy() {
    print_status "Running clippy..."
    cargo clippy --all-targets --all-features -- -D warnings
}

# Function to run audit
run_audit() {
    print_status "Running cargo audit..."
    if command -v cargo-audit &> /dev/null; then
        cargo audit
    else
        print_warning "cargo-audit not found. Install with: cargo install cargo-audit"
    fi
}

# Function to setup test environment
setup_test_env() {
    print_status "Setting up test environment..."
    
    # Create test directories
    mkdir -p tests/data
    mkdir -p coverage
    
    # Set environment variables for testing
    export OPENCHIME_TEST_MODE="true"
    export RUST_LOG="debug"
    export RUST_BACKTRACE="1"
    
    print_status "Test environment configured"
}

# Function to clean test artifacts
clean_tests() {
    print_status "Cleaning test artifacts..."
    cargo clean
    rm -rf tests/data/*
    rm -rf coverage/
    print_status "Test artifacts cleaned"
}

# Function to show help
show_help() {
    echo "OpenChime Test Runner"
    echo ""
    echo "Usage: $0 [COMMAND]"
    echo ""
    echo "Commands:"
    echo "  unit          Run unit tests only"
    echo "  integration   Run integration tests only"
    echo "  e2e           Run end-to-end tests only"
    echo "  all           Run all tests (default)"
    echo "  coverage      Run tests with coverage report"
    echo "  watch         Run tests in watch mode"
    echo "  format        Check code formatting"
    echo "  clippy        Run clippy lints"
    echo "  audit         Run security audit"
    echo "  setup         Setup test environment"
    echo "  clean         Clean test artifacts"
    echo "  help          Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 unit                    # Run unit tests only"
    echo "  $0 integration             # Run integration tests only"
    echo "  $0 e2e                     # Run E2E tests only"
    echo "  $0 all                     # Run all tests"
    echo "  $0 coverage                # Generate coverage report"
    echo "  RUST_LOG=debug $0 unit     # Run unit tests with debug logging"
}

# Main script logic
case "${1:-all}" in
    "unit")
        setup_test_env
        run_unit_tests
        ;;
    "integration")
        setup_test_env
        run_integration_tests
        ;;
    "e2e")
        setup_test_env
        run_e2e_tests
        ;;
    "all")
        setup_test_env
        run_all_tests
        ;;
    "coverage")
        setup_test_env
        run_coverage
        ;;
    "watch")
        setup_test_env
        run_watch_tests
        ;;
    "format")
        check_formatting
        ;;
    "clippy")
        run_clippy
        ;;
    "audit")
        run_audit
        ;;
    "setup")
        setup_test_env
        ;;
    "clean")
        clean_tests
        ;;
    "help"|"-h"|"--help")
        show_help
        ;;
    *)
        print_error "Unknown command: $1"
        show_help
        exit 1
        ;;
esac

print_status "Test run completed successfully!"