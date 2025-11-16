#!/usr/bin/env bash
# Rustbot Development Utilities
# Convenience script for common development workflows

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Change to project root
cd "$PROJECT_ROOT"

# Helper functions
print_header() {
    echo -e "\n${BLUE}==>${NC} ${1}"
}

print_success() {
    echo -e "${GREEN}✓${NC} ${1}"
}

print_error() {
    echo -e "${RED}✗${NC} ${1}" >&2
}

print_warning() {
    echo -e "${YELLOW}!${NC} ${1}"
}

# Check if cargo-watch is installed
check_cargo_watch() {
    if ! command -v cargo-watch &> /dev/null; then
        print_error "cargo-watch not found"
        echo "Install with: cargo install cargo-watch"
        exit 1
    fi
}

# Check if jq is installed
check_jq() {
    if ! command -v jq &> /dev/null; then
        print_error "jq not found"
        echo "Install with: brew install jq (macOS) or apt install jq (Linux)"
        exit 1
    fi
}

# Validate all JSON agent configurations
validate_configs() {
    print_header "Validating agent configurations"

    check_jq

    local error_count=0
    local success_count=0

    shopt -s nullglob
    for json_file in agents/presets/*.json agents/custom/*.json; do
        if [ -f "$json_file" ]; then
            if jq empty "$json_file" 2>/dev/null; then
                print_success "$json_file"
                ((success_count++))
            else
                print_error "$json_file - Invalid JSON"
                ((error_count++))
            fi
        fi
    done

    echo ""
    if [ $error_count -eq 0 ]; then
        print_success "All configurations valid ($success_count files)"
        return 0
    else
        print_error "$error_count configuration(s) failed validation"
        return 1
    fi
}

# Watch mode - auto-rebuild on code changes
watch_code() {
    print_header "Starting watch mode (code only)"
    check_cargo_watch

    print_warning "Watching Rust source files..."
    echo "Press Ctrl+C to stop"
    echo ""

    cargo watch -x run
}

# Watch mode - auto-rebuild on code AND config changes
watch_all() {
    print_header "Starting watch mode (code + configs)"
    check_cargo_watch

    print_warning "Watching Rust source files, agent configs, and .env..."
    echo "Press Ctrl+C to stop"
    echo ""

    cargo watch -x run -w agents -w .env
}

# Run the application with debug logging
run_debug() {
    print_header "Running with debug logging"

    RUST_LOG=debug cargo run
}

# Run the application with trace logging
run_trace() {
    print_header "Running with trace logging"

    RUST_LOG=trace cargo run
}

# Build and run
build_run() {
    print_header "Building project"

    if cargo build; then
        print_success "Build successful"
        print_header "Running application"
        ./target/debug/rustbot
    else
        print_error "Build failed"
        exit 1
    fi
}

# Full validation pipeline
full_check() {
    print_header "Running full validation pipeline"

    # Validate JSON configs
    if ! validate_configs; then
        exit 1
    fi

    # Run tests
    print_header "Running tests"
    if cargo test --quiet; then
        print_success "All tests passed"
    else
        print_error "Tests failed"
        exit 1
    fi

    # Run clippy
    print_header "Running clippy"
    if cargo clippy --quiet -- -D warnings; then
        print_success "No clippy warnings"
    else
        print_error "Clippy found issues"
        exit 1
    fi

    echo ""
    print_success "All checks passed!"
}

# Quick setup for new developers
setup() {
    print_header "Setting up development environment"

    # Check Rust installation
    if ! command -v cargo &> /dev/null; then
        print_error "Rust not installed"
        echo "Install from: https://rustup.rs"
        exit 1
    fi
    print_success "Rust installed"

    # Install cargo-watch
    print_header "Installing cargo-watch"
    if cargo install cargo-watch; then
        print_success "cargo-watch installed"
    else
        print_warning "cargo-watch installation failed or already installed"
    fi

    # Check for .env file
    if [ ! -f ".env" ]; then
        print_warning ".env file not found"
        echo "Creating .env template..."
        cat > .env << 'EOF'
# OpenRouter API Key (required)
OPENROUTER_API_KEY=your-key-here

# Optional: Anthropic API Key
# ANTHROPIC_API_KEY=your-key-here

# Optional: Debug logging
# RUST_LOG=debug
EOF
        print_success "Created .env template - please add your API keys"
    else
        print_success ".env file exists"
    fi

    # Validate agent configs
    validate_configs

    # Build project
    print_header "Building project"
    if cargo build; then
        print_success "Build successful"
    else
        print_error "Build failed"
        exit 1
    fi

    echo ""
    print_success "Setup complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Add your API keys to .env file"
    echo "  2. Run: ./scripts/dev.sh watch"
    echo "  3. Start coding!"
}

# Show usage information
show_help() {
    cat << EOF
Rustbot Development Utilities

Usage: ./scripts/dev.sh <command>

Commands:
  watch         Watch Rust code, auto-rebuild on changes
  watch-all     Watch code + agent configs + .env, auto-rebuild
  run           Build and run application
  run-debug     Run with debug logging (RUST_LOG=debug)
  run-trace     Run with trace logging (RUST_LOG=trace)
  validate      Validate all agent JSON configurations
  check         Run full validation (JSON + tests + clippy)
  setup         Set up development environment for new developers
  help          Show this help message

Examples:
  ./scripts/dev.sh watch        # Start auto-reload development
  ./scripts/dev.sh validate     # Check all JSON configs
  ./scripts/dev.sh check        # Run all validations
  ./scripts/dev.sh run-debug    # Run with debug output

For more information, see DEVELOPMENT.md
EOF
}

# Main command dispatcher
main() {
    case "${1:-help}" in
        watch)
            watch_code
            ;;
        watch-all)
            watch_all
            ;;
        run)
            build_run
            ;;
        run-debug)
            run_debug
            ;;
        run-trace)
            run_trace
            ;;
        validate)
            validate_configs
            ;;
        check)
            full_check
            ;;
        setup)
            setup
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            print_error "Unknown command: ${1}"
            echo ""
            show_help
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
