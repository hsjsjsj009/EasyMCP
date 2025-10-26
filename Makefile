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
