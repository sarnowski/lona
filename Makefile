# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
#
# Makefile - Lona build system
#
# Primary targets:
#   check  - Fast verification (fmt, compile, clippy, unit tests)
#   build  - Create bootable QEMU image (includes check)
#   test   - Run integration tests in QEMU
#   run    - Interactive QEMU session
#   clean  - Remove all build artifacts
#
# Requires: Docker, GNU Make 4.0+ (macOS: brew install make, use gmake)

# Require GNU Make 4.0+
ifeq ($(filter 4.% 5.%,$(MAKE_VERSION)),)
$(error GNU Make 4.0+ required. On macOS: brew install make && gmake)
endif

# ==============================================================================
# Configuration
# ==============================================================================

BUILD_DIR     := build
IMAGE_FILE    := $(BUILD_DIR)/lona.elf
COMPOSE       := docker compose

# seL4 configuration (paths inside Docker container)
SEL4_PREFIX   := /opt/seL4
TARGET        := support/targets/aarch64-sel4.json
CRATE         := lona-runtime

# QEMU configuration
QEMU          := qemu-system-aarch64
QEMU_MACHINE  := virt,virtualization=on
QEMU_CPU      := cortex-a57
QEMU_MEMORY   := 1G

# ==============================================================================
# Primary Targets
# ==============================================================================

.DEFAULT_GOAL := build

.PHONY: check
check: ## Fast verification: fmt, compile, clippy, unit tests
	$(COMPOSE) run --rm builder make _check

.PHONY: build
build: check ## Create bootable QEMU image
	$(COMPOSE) run --rm builder make _build

.PHONY: test
test: build ## Run integration tests in QEMU
	@echo "Integration tests not yet implemented"

.PHONY: run
run: build ## Interactive QEMU session
	$(COMPOSE) run --rm runner

.PHONY: clean
clean: ## Remove all build artifacts
	rm -rf $(BUILD_DIR) target
	@echo "Build artifacts removed"

# ==============================================================================
# Secondary Targets
# ==============================================================================

.PHONY: docker
docker: ## Build Docker development image
	$(COMPOSE) build builder

.PHONY: shell
shell: ## Interactive Docker shell for debugging
	$(COMPOSE) run --rm builder bash

.PHONY: help
help: ## Show this help
	@echo "Lona Build System"
	@echo ""
	@echo "Primary targets:"
	@echo "  check   Fast verification (fmt, compile, clippy, unit tests)"
	@echo "  build   Create bootable QEMU image (includes check)"
	@echo "  test    Run integration tests in QEMU"
	@echo "  run     Interactive QEMU session"
	@echo "  clean   Remove all build artifacts"
	@echo ""
	@echo "Secondary targets:"
	@echo "  docker  Build Docker development image"
	@echo "  shell   Interactive Docker shell"
	@echo ""
	@echo "Dependency: check -> build -> test"
	@echo "                  -> run"

# ==============================================================================
# Internal Targets (run inside Docker container)
# ==============================================================================

.PHONY: _check
_check:
	@echo "==> Checking formatting..."
	cargo fmt --check
	@echo "==> Compiling..."
	SEL4_PREFIX=$(SEL4_PREFIX) cargo build \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		--target $(TARGET) \
		--package $(CRATE)
	@echo "==> Running clippy..."
	SEL4_PREFIX=$(SEL4_PREFIX) cargo clippy \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		--target $(TARGET) \
		--package $(CRATE) \
		-- -D warnings
	@echo "==> Running unit tests..."
	@CRATE_COUNT=$$(find crates -mindepth 1 -maxdepth 1 -type d ! -name $(CRATE) 2>/dev/null | wc -l); \
	if [ $$CRATE_COUNT -gt 0 ]; then \
		cargo test --workspace --exclude $(CRATE); \
	else \
		echo "    (no host-testable crates yet)"; \
	fi
	@echo "==> All checks passed"

.PHONY: _build
_build:
	@echo "==> Building release binary..."
	@mkdir -p $(BUILD_DIR)
	SEL4_PREFIX=$(SEL4_PREFIX) cargo build \
		--release \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		-Z unstable-options \
		--target $(TARGET) \
		--package $(CRATE) \
		--target-dir $(BUILD_DIR)/target \
		--artifact-dir $(BUILD_DIR)
	@echo "==> Creating bootable image..."
	sel4-kernel-loader-add-payload \
		--loader $(SEL4_PREFIX)/bin/sel4-kernel-loader \
		--sel4-prefix $(SEL4_PREFIX) \
		--app $(BUILD_DIR)/$(CRATE).elf \
		-o $(IMAGE_FILE)
	@echo "==> Image created: $(IMAGE_FILE)"

.PHONY: _run
_run:
	$(QEMU) \
		-machine $(QEMU_MACHINE) \
		-cpu $(QEMU_CPU) \
		-m $(QEMU_MEMORY) \
		-nographic \
		-serial mon:stdio \
		-kernel $(IMAGE_FILE)
