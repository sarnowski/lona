# Lona Makefile
# =================
#
# IMPORTANT: Requires GNU Make 4.0+ (macOS ships with 3.81)
# Install with: brew install make
# Run with: gmake
#
# Build infrastructure for Lona: seL4 microkernel + Lona runtime

# Require GNU Make 4.0+
ifeq ($(filter 4.% 5.%,$(MAKE_VERSION)),)
$(error GNU Make 4.0+ required. Install with 'brew install make' and run 'gmake')
endif

# ==============================================================================
# Configuration
# ==============================================================================

# Build directories (gitignored)
BUILD_DIR     := build
TARGET_DIR    := target
IMAGE_DIR     := $(BUILD_DIR)/image

# Docker configuration
DOCKER_IMAGE  := lona-builder:latest
COMPOSE       := docker compose

# seL4 configuration (set in Docker container)
SEL4_PREFIX   := /opt/seL4

# Target architecture
TARGET        := support/targets/aarch64-sel4.json
CRATE         := lona-runtime

# QEMU configuration
QEMU_MEMORY   := 1G
QEMU_CPU      := cortex-a57
QEMU_MACHINE  := virt,virtualization=on

# Documentation
VENV          := .venv
MKDOCS        := $(VENV)/bin/mkdocs

# ==============================================================================
# Default Target
# ==============================================================================

.DEFAULT_GOAL := image

# ==============================================================================
# Help
# ==============================================================================

.PHONY: help
help: ## Show this help message
	@echo "Lona Build System"
	@echo "===================="
	@echo ""
	@echo "Build targets (run in Docker):"
	@grep -E '^(build|run|shell|image|clean)[a-zA-Z_-]*:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-18s %s\n", $$1, $$2}'
	@echo ""
	@echo "Docker targets:"
	@grep -E '^docker[a-zA-Z_-]*:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-18s %s\n", $$1, $$2}'
	@echo ""
	@echo "Documentation targets:"
	@grep -E '^docs[a-zA-Z_-]*:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-18s %s\n", $$1, $$2}'
	@echo ""
	@echo "Development targets:"
	@grep -E '^(check|fmt|clippy|test)[a-zA-Z_-]*:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  %-18s %s\n", $$1, $$2}'

# ==============================================================================
# Docker Management
# ==============================================================================

.PHONY: docker-build
docker-build: ## Build the Docker development image
	$(COMPOSE) build builder

.PHONY: docker-clean
docker-clean: ## Remove the Docker development image
	$(COMPOSE) down --rmi local --volumes

# ==============================================================================
# Build Targets (executed in Docker)
# ==============================================================================

.PHONY: build
build: check ## Build Lona runtime (seL4 root task) in Docker
	$(COMPOSE) run --rm builder make -C /work _build

.PHONY: image
image: check ## Build complete bootable image in Docker
	$(COMPOSE) run --rm builder make -C /work _image

.PHONY: run
run: image ## Build and run Lona in QEMU (1GB RAM)
	$(COMPOSE) run --rm runner

.PHONY: shell
shell: ## Start interactive development shell in Docker
	$(COMPOSE) run --rm builder bash

# ==============================================================================
# Internal Build Targets (run inside Docker container)
# ==============================================================================

# Build the Rust root task
.PHONY: _build
_build:
	@echo "Building Lona runtime for seL4..."
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
	@echo "Build complete: $(BUILD_DIR)/$(CRATE).elf"

# Create bootable image combining kernel loader + seL4 + root task
.PHONY: _image
_image: _build
	@echo "Creating bootable image..."
	@mkdir -p $(IMAGE_DIR)
	sel4-kernel-loader-add-payload \
		--loader $(SEL4_PREFIX)/bin/sel4-kernel-loader \
		--sel4-prefix $(SEL4_PREFIX) \
		--app $(BUILD_DIR)/$(CRATE).elf \
		-o $(IMAGE_DIR)/lona-qemu.elf
	@echo "Image created: $(IMAGE_DIR)/lona-qemu.elf"

# Run in QEMU (called from inside Docker or directly if QEMU available)
.PHONY: _run
_run:
	qemu-system-aarch64 \
		-machine $(QEMU_MACHINE) \
		-cpu $(QEMU_CPU) \
		-m $(QEMU_MEMORY) \
		-nographic \
		-serial mon:stdio \
		-kernel $(IMAGE_DIR)/lona-qemu.elf

# ==============================================================================
# Development (Quality Checks)
# ==============================================================================

.PHONY: check
check: ## Run all quality checks (fmt, clippy) in Docker
	$(COMPOSE) run --rm builder make -C /work _check

.PHONY: fmt
fmt: ## Format Rust code
	$(COMPOSE) run --rm builder cargo fmt

.PHONY: fmt-check
fmt-check: ## Check Rust code formatting
	$(COMPOSE) run --rm builder cargo fmt --check

.PHONY: clippy
clippy: ## Run clippy lints (cross-compilation target)
	$(COMPOSE) run --rm builder make -C /work _clippy

# Internal check targets (run inside Docker container)
.PHONY: _check
_check: _fmt-check _clippy
	@echo "All checks passed"

.PHONY: _fmt-check
_fmt-check:
	@echo "Checking formatting..."
	cargo fmt --check

.PHONY: _clippy
_clippy:
	@echo "Running clippy for aarch64-sel4 target..."
	SEL4_PREFIX=$(SEL4_PREFIX) cargo clippy \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		--target $(TARGET) \
		--package $(CRATE) \
		-- -D warnings

# ==============================================================================
# Clean
# ==============================================================================

.PHONY: clean
clean: ## Remove build artifacts
	rm -rf $(BUILD_DIR) $(TARGET_DIR)
	@echo "Build artifacts cleaned"

.PHONY: clean-all
clean-all: clean docker-clean ## Remove all artifacts including Docker images
	rm -rf $(VENV)
	@echo "All artifacts cleaned"

# ==============================================================================
# Documentation
# ==============================================================================

$(VENV)/bin/activate:
	python3 -m venv $(VENV)

$(MKDOCS): $(VENV)/bin/activate requirements.txt
	$(VENV)/bin/pip install -r requirements.txt

.PHONY: docs
docs: $(MKDOCS) ## Build the documentation site
	$(MKDOCS) build

.PHONY: docs-live
docs-live: $(MKDOCS) ## Start a live-reloading documentation server
	$(MKDOCS) serve
