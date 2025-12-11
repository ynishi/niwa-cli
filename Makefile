# Makefile for NIWA CLI

.DEFAULT_GOAL := help

# Variables
CARGO := cargo
BINARY_NAME := niwa
INSTALL_PATH := $(HOME)/.cargo/bin

# Colors for output
COLOR_RESET := \033[0m
COLOR_BOLD := \033[1m
COLOR_GREEN := \033[32m
COLOR_YELLOW := \033[33m
COLOR_BLUE := \033[34m

.PHONY: help
help: ## Display this help message
	@echo "$(COLOR_BOLD)NIWA CLI - Makefile Commands$(COLOR_RESET)"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(COLOR_BLUE)%-15s$(COLOR_RESET) %s\n", $$1, $$2}'
	@echo ""

.PHONY: check
check: ## Run cargo check on all crates
	@echo "$(COLOR_YELLOW)Checking code...$(COLOR_RESET)"
	$(CARGO) check --workspace --all-targets
	@echo "$(COLOR_GREEN)✓ Check complete$(COLOR_RESET)"

.PHONY: build
build: ## Build the project in release mode
	@echo "$(COLOR_YELLOW)Building release binary...$(COLOR_RESET)"
	$(CARGO) build --release
	@echo "$(COLOR_GREEN)✓ Build complete: target/release/$(BINARY_NAME)$(COLOR_RESET)"

.PHONY: build-dev
build-dev: ## Build the project in debug mode
	@echo "$(COLOR_YELLOW)Building debug binary...$(COLOR_RESET)"
	$(CARGO) build
	@echo "$(COLOR_GREEN)✓ Build complete: target/debug/$(BINARY_NAME)$(COLOR_RESET)"

.PHONY: test
test: ## Run all tests
	@echo "$(COLOR_YELLOW)Running tests...$(COLOR_RESET)"
	$(CARGO) test --workspace
	@echo "$(COLOR_GREEN)✓ All tests passed$(COLOR_RESET)"

.PHONY: test-verbose
test-verbose: ## Run tests with verbose output
	@echo "$(COLOR_YELLOW)Running tests (verbose)...$(COLOR_RESET)"
	$(CARGO) test --workspace -- --nocapture

.PHONY: install
install: build ## Build and install the binary to ~/.cargo/bin
	@echo "$(COLOR_YELLOW)Installing $(BINARY_NAME)...$(COLOR_RESET)"
	$(CARGO) install --path crates/niwa --force
	@echo "$(COLOR_GREEN)✓ Installed to $(INSTALL_PATH)/$(BINARY_NAME)$(COLOR_RESET)"
	@echo ""
	@echo "$(COLOR_BOLD)Run '$(BINARY_NAME) --help' to get started!$(COLOR_RESET)"

.PHONY: uninstall
uninstall: ## Uninstall the binary from ~/.cargo/bin
	@echo "$(COLOR_YELLOW)Uninstalling $(BINARY_NAME)...$(COLOR_RESET)"
	$(CARGO) uninstall $(BINARY_NAME)
	@echo "$(COLOR_GREEN)✓ Uninstalled$(COLOR_RESET)"

.PHONY: clean
clean: ## Clean build artifacts
	@echo "$(COLOR_YELLOW)Cleaning build artifacts...$(COLOR_RESET)"
	$(CARGO) clean
	@echo "$(COLOR_GREEN)✓ Clean complete$(COLOR_RESET)"

.PHONY: fmt
fmt: ## Format code with rustfmt
	@echo "$(COLOR_YELLOW)Formatting code...$(COLOR_RESET)"
	$(CARGO) fmt --all
	@echo "$(COLOR_GREEN)✓ Formatting complete$(COLOR_RESET)"

.PHONY: fmt-check
fmt-check: ## Check code formatting
	@echo "$(COLOR_YELLOW)Checking code formatting...$(COLOR_RESET)"
	$(CARGO) fmt --all -- --check
	@echo "$(COLOR_GREEN)✓ Format check complete$(COLOR_RESET)"

.PHONY: clippy
clippy: ## Run clippy lints
	@echo "$(COLOR_YELLOW)Running clippy...$(COLOR_RESET)"
	$(CARGO) clippy --workspace --all-targets -- -D warnings
	@echo "$(COLOR_GREEN)✓ Clippy check complete$(COLOR_RESET)"

.PHONY: doc
doc: ## Generate documentation
	@echo "$(COLOR_YELLOW)Generating documentation...$(COLOR_RESET)"
	$(CARGO) doc --workspace --no-deps --open
	@echo "$(COLOR_GREEN)✓ Documentation generated$(COLOR_RESET)"

.PHONY: run
run: build-dev ## Run the CLI in debug mode
	@echo "$(COLOR_YELLOW)Running $(BINARY_NAME)...$(COLOR_RESET)"
	$(CARGO) run --bin $(BINARY_NAME) -- $(ARGS)

.PHONY: run-release
run-release: build ## Run the CLI in release mode
	@echo "$(COLOR_YELLOW)Running $(BINARY_NAME) (release)...$(COLOR_RESET)"
	./target/release/$(BINARY_NAME) $(ARGS)

.PHONY: all
all: check test build ## Run check, test, and build

.PHONY: ci
ci: fmt-check clippy test ## Run all CI checks (format, clippy, test)
	@echo "$(COLOR_GREEN)✓ All CI checks passed$(COLOR_RESET)"

.PHONY: preflight
preflight: fmt clippy build test ## Run all preflight checks (fmt, clippy, build, test)
	@echo "$(COLOR_GREEN)✓ All preflight checks passed$(COLOR_RESET)"

.PHONY: dev-setup
dev-setup: ## Setup development environment
	@echo "$(COLOR_YELLOW)Setting up development environment...$(COLOR_RESET)"
	@echo "Installing Rust toolchain components..."
	rustup component add rustfmt clippy
	@echo "$(COLOR_GREEN)✓ Development environment ready$(COLOR_RESET)"

.PHONY: version
version: ## Display version information
	@echo "$(COLOR_BOLD)NIWA CLI Version Information$(COLOR_RESET)"
	@$(CARGO) pkgid -p niwa | sed 's/.*#//'
	@echo ""
	@echo "Rust version:"
	@rustc --version
	@echo ""
	@echo "Cargo version:"
	@cargo --version
