# Ai generated makefile 
#
# .PHONY explicitly tells Make that these targets are abstract commands,
# not files on disk. This prevents conflicts if you accidentally create
# a file named "run-py" or "clean".
.PHONY: all run-py run-rs clean help build-rs
URL ?= https://browser.engineering/examples/example1-simple.html

# Default shell logic (good practice for cross-platform consistency)
SHELL := /bin/bash

# Default target when you just run `make`
all: help

# Python Targets (High-Level / Interpreted) 
# Runs the interpreter directly on the script.
run-py:
	@echo "Running Python implementation..."
	python3 python/main.py "$(URL)"

# Rust Targets (Low-Level / Compiled) 
# We use --manifest-path to execute cargo from the root without 'cd'.
# This keeps the shell environment stable.
run-rs:
	@echo "Building and Running Rust implementation..."
	cargo run --manifest-path rust/Cargo.toml -- "$(URL)"

# Optimized build for benchmarking (removes debug symbols, enables vectorization)
run-rs-release:
	@echo "Running Rust in Release mode (Optimized)..."
	cargo run --release --manifest-path rust/Cargo.toml -- "$(URL)"

# Utilities 
# clean: Removes build artifacts for both languages to free space.
clean:
	@echo "🧹 Cleaning up..."
	rm -rf python/__pycache__
	cargo clean --manifest-path rust/Cargo.toml

# help: Self-documenting command for new developers.
help:
	@echo "Available commands:"
	@echo "  make run-py         - Run the Python script"
	@echo "  make run-rs         - Run the Rust application (Debug)"
	@echo "  make run-rs-release - Run the Rust application (Release/Optimized)"
	@echo "  make clean          - Remove all build artifacts"
