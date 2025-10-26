# EasyMCP Build Script
# This script provides convenient commands for building EasyMCP for different platforms

.PHONY: help build clean test lint fmt check install-local install-linux install-windows install-macos

help: ## Show this help message
	@echo "EasyMCP Build Commands:"
	@echo ""
	@echo "Development commands:"
	@echo "  make build       - Build for current platform"
	@echo "  make test        - Run tests"
	@echo "  make lint        - Run clippy lints"
	@echo "  make fmt         - Format code"
	@echo "  make check       - Check compilation without building"
	@echo "  make clean       - Clean build artifacts"
	@echo ""
	@echo "Cross-compilation commands:"
	@echo "  make install-linux    - Install cross-compilation tools for Linux"
	@echo "  make install-windows - Install cross-compilation tools for Windows"
	@echo "  make install-macos   - Install cross-compilation tools for macOS"
	@echo ""
	@echo "Build for specific platforms:"
	@echo "  cargo build --release --target x86_64-unknown-linux-gnu"
	@echo "  cargo build --release --target aarch64-unknown-linux-gnu"
	@echo "  cargo build --release --target x86_64-pc-windows-msvc"
	@echo "  cargo build --release --target x86_64-apple-darwin"
	@echo "  cargo build --release --target aarch64-apple-darwin"

build: ## Build for current platform
	cargo build --release

test: ## Run tests
	cargo test

lint: ## Run clippy lints
	cargo clippy -- -D warnings

fmt: ## Format code
	cargo fmt

check: ## Check compilation without building
	cargo check

clean: ## Clean build artifacts
	cargo clean

# Cross-compilation setup
install-linux: ## Install cross-compilation tools for Linux ARM64
	@echo "Installing cross-compilation tools for Linux ARM64..."
	sudo apt-get update
	sudo apt-get install -y gcc-aarch64-linux-gnu

install-windows: ## Install cross-compilation tools for Windows (requires LLVM)
	@echo "Installing cross-compilation tools for Windows..."
	sudo apt-get update
	sudo apt-get install -y gcc-mingw-w64

install-macos: ## Install cross-compilation tools for macOS (requires osxcross)
	@echo "For macOS cross-compilation, consider using osxcross or GitHub Actions"
	@echo "https://github.com/tpoechtrager/osxcross"
