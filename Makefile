# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
#
# Makefile - Lona build system
#
# Development targets:
#   build-arm64   - Create bootable ARM64 QEMU image
#   build-x86_64  - Create bootable x86_64 QEMU image
#   run-arm64     - Interactive ARM64 QEMU session
#   run-x86_64    - Interactive x86_64 QEMU session
#   test          - Full verification on both ARM64 and x86_64
#   clean         - Remove all build artifacts
#
# Release targets:
#   release-x86_64      - UEFI-bootable x86_64 (PCs/servers)
#   release-rpi4b       - SD card for Raspberry Pi 4B (8GB)
#   release-rpi4b-4gb   - SD card for Raspberry Pi 4B (4GB)
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
COMPOSE       := docker compose
DEV_CRATE     := lona-runtime

# ARM64 development platform (qemu-arm-virt) - Debug Kernel
ARM64_SEL4_PREFIX := /opt/seL4/qemu-arm-virt-debug
ARM64_TARGET      := support/targets/aarch64-sel4.json
ARM64_IMAGE       := $(BUILD_DIR)/lona-arm64.elf

# x86_64 development platform (qemu-pc99) - Debug Kernel
X86_SEL4_PREFIX_DEBUG := /opt/seL4/x86_64-pc99-debug
# x86_64 release platform (qemu-pc99) - Release Kernel
X86_SEL4_PREFIX_RELEASE := /opt/seL4/x86_64-pc99

X86_TARGET        := support/targets/x86_64-sel4.json
X86_IMAGE_DIR     := $(BUILD_DIR)/qemu-x86_64

# RPi4 platform (prefix selected dynamically based on RPI4_MEMORY)
RPI4_SEL4_PREFIX_8GB := /opt/seL4/rpi4-8gb
RPI4_SEL4_PREFIX_4GB := /opt/seL4/rpi4-4gb
RPI4_TARGET          := support/targets/aarch64-sel4.json

# QEMU ARM64 configuration
QEMU_ARM64         := qemu-system-aarch64
QEMU_ARM64_MACHINE := virt,virtualization=on
QEMU_ARM64_CPU     := cortex-a57
QEMU_ARM64_MEMORY  := 1G

# QEMU x86_64 configuration
QEMU_X86           := qemu-system-x86_64
QEMU_X86_MACHINE   := q35
QEMU_X86_CPU       := Cascadelake-Server
QEMU_X86_MEMORY    := 512M
OVMF_CODE          := /usr/share/OVMF/OVMF_CODE.fd

# Test configuration
TEST_IMAGE    := $(BUILD_DIR)/lona-test.elf
TEST_TIMEOUT  := 10

# Legacy aliases
IMAGE_FILE    := $(ARM64_IMAGE)
DEV_SEL4_PREFIX := $(ARM64_SEL4_PREFIX)
DEV_TARGET      := $(ARM64_TARGET)

# ==============================================================================
# Primary Targets
# ==============================================================================

.DEFAULT_GOAL := debug-arm64

.PHONY: build
build: debug-arm64 ## Alias for debug-arm64

.PHONY: debug-arm64
debug-arm64: ## Create bootable ARM64 QEMU image (Debug)
	$(COMPOSE) run --rm builder make _debug-arm64

.PHONY: debug-x86_64
debug-x86_64: ## Create bootable x86_64 QEMU image (Debug)
	$(COMPOSE) run --rm builder-x86_64 make _debug-x86_64

.PHONY: test
test: ## Full verification on both ARM64 and x86_64
	$(MAKE) test-arm64 && $(MAKE) test-x86_64

.PHONY: test-arm64
test-arm64: ## Full ARM64 verification: fmt, clippy, unit tests, build, integration tests
	$(COMPOSE) run --rm builder make _check
	$(COMPOSE) run --rm builder make _build-test-arm64
	$(COMPOSE) run --rm tester-arm64

.PHONY: test-x86_64
test-x86_64: ## Full x86_64 verification: fmt, clippy, unit tests, build, integration tests
	$(COMPOSE) run --rm builder-x86_64 make _check-x86_64
	$(COMPOSE) run --rm builder-x86_64 make _build-test-x86_64
	$(COMPOSE) run --rm tester-x86_64

.PHONY: run-arm64
run-arm64: debug-arm64 ## Interactive ARM64 QEMU session
	$(COMPOSE) run --rm runner-arm64

.PHONY: run-x86_64
run-x86_64: debug-x86_64 ## Interactive x86_64 QEMU session
	$(COMPOSE) run --rm runner-x86_64

.PHONY: clean
clean: ## Remove all build artifacts
	rm -rf $(BUILD_DIR) target
	@echo "Build artifacts removed"

# ==============================================================================
# Release Targets
# ==============================================================================

.PHONY: release-x86_64
release-x86_64: ## Build UEFI-bootable release for x86_64
	$(COMPOSE) run --rm builder-x86_64 make _release-x86_64

.PHONY: release-rpi4b
release-rpi4b: release-rpi4b-8gb ## Build SD card release for RPi4B (default: 8GB)

.PHONY: release-rpi4b-8gb
release-rpi4b-8gb: ## Build SD card release for RPi4B 8GB
	$(COMPOSE) run --rm builder make _release-rpi4b RPI4_MEMORY=8192

.PHONY: release-rpi4b-4gb
release-rpi4b-4gb: ## Build SD card release for RPi4B 4GB
	$(COMPOSE) run --rm builder make _release-rpi4b RPI4_MEMORY=4096

# ==============================================================================
# Secondary Targets
# ==============================================================================

.PHONY: docker
docker: ## Build all Docker images
	$(COMPOSE) build base
	$(COMPOSE) build builder
	$(COMPOSE) build builder-x86_64

.PHONY: shell
shell: ## Interactive Docker shell (aarch64)
	$(COMPOSE) run --rm builder bash

.PHONY: shell-x86_64
shell-x86_64: ## Interactive Docker shell (x86_64)
	$(COMPOSE) run --rm builder-x86_64 bash

.PHONY: help
help: ## Show this help
	@echo "Lona Build System"
	@echo ""
	@echo "Development targets:"
	@echo "  debug-arm64   Create bootable ARM64 QEMU image (Debug)"
	@echo "  debug-x86_64  Create bootable x86_64 QEMU image (Debug)"
	@echo "  run-arm64     Interactive ARM64 QEMU session"
	@echo "  run-x86_64    Interactive x86_64 QEMU session"
	@echo "  test-arm64    Full ARM64 verification suite"
	@echo "  test-x86_64   Full x86_64 verification suite"
	@echo "  clean         Remove build artifacts"
	@echo ""
	@echo "Aliases:"
	@echo "  build         Alias for debug-arm64"
	@echo "  test          Run test-arm64 and test-x86_64"
	@echo ""
	@echo "Release targets:"
	@echo "  release-x86_64      UEFI-bootable x86_64 (PCs/servers)"
	@echo "  release-rpi4b       SD card for Raspberry Pi 4B (8GB)"
	@echo "  release-rpi4b-4gb   SD card for Raspberry Pi 4B (4GB)"
	@echo ""
	@echo "Utility targets:"
	@echo "  docker        Build all Docker images"
	@echo "  shell         Interactive shell (aarch64)"
	@echo "  shell-x86_64  Interactive shell (x86_64)"
	@echo "  mcp           Start Lona dev REPL MCP server"

# ==============================================================================
# Python Tooling
# ==============================================================================

VENV := .venv
PYTHON := $(VENV)/bin/python
PIP := $(VENV)/bin/pip

$(VENV): requirements.txt
	python3 -m venv $(VENV)
	$(PIP) install -r requirements.txt
	@touch $(VENV)

.PHONY: mcp
mcp: $(VENV) ## Start Lona dev REPL MCP server
	$(PYTHON) -m tools.lona_dev_repl

# ==============================================================================
# Internal Targets (run inside Docker container)
# ==============================================================================

.PHONY: _check
_check:
	@echo "==> Formatting code..."
	cargo fmt
	@echo "==> Compiling runtime (ARM64)..."
	SEL4_PREFIX=$(ARM64_SEL4_PREFIX) cargo build \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		--target $(ARM64_TARGET) \
		--package $(DEV_CRATE)
	@echo "==> Running clippy on host-testable crates..."
	cargo clippy --workspace --exclude $(DEV_CRATE) -- -D warnings
	@echo "==> Running clippy on runtime (ARM64)..."
	SEL4_PREFIX=$(ARM64_SEL4_PREFIX) cargo clippy \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		--target $(ARM64_TARGET) \
		--package $(DEV_CRATE) \
		-- -D warnings
	@echo "==> Running unit tests..."
	@CRATE_COUNT=$$(find crates -mindepth 1 -maxdepth 1 -type d ! -name $(DEV_CRATE) 2>/dev/null | wc -l); \
	if [ $$CRATE_COUNT -gt 0 ]; then \
		cargo test --workspace --exclude $(DEV_CRATE); \
	else \
		echo "    (no host-testable crates yet)"; \
	fi
	@echo "==> All checks passed"

.PHONY: _check-x86_64
_check-x86_64:
	@echo "==> Formatting code..."
	cargo fmt
	@echo "==> Compiling runtime (x86_64)..."
	SEL4_PREFIX=$(X86_SEL4_PREFIX_DEBUG) cargo build \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		--target $(X86_TARGET) \
		--package $(DEV_CRATE)
	@echo "==> Running clippy on host-testable crates..."
	cargo clippy --workspace --exclude $(DEV_CRATE) -- -D warnings
	@echo "==> Running clippy on runtime (x86_64)..."
	SEL4_PREFIX=$(X86_SEL4_PREFIX_DEBUG) cargo clippy \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		--target $(X86_TARGET) \
		--package $(DEV_CRATE) \
		-- -D warnings
	@echo "==> Running unit tests..."
	@CRATE_COUNT=$$(find crates -mindepth 1 -maxdepth 1 -type d ! -name $(DEV_CRATE) 2>/dev/null | wc -l); \
	if [ $$CRATE_COUNT -gt 0 ]; then \
		cargo test --workspace --exclude $(DEV_CRATE); \
	else \
		echo "    (no host-testable crates yet)"; \
	fi
	@echo "==> All checks passed"

.PHONY: _debug-arm64
_debug-arm64:
	@echo "==> Building debug binary (qemu-arm-virt)..."
	@mkdir -p $(BUILD_DIR)
	SEL4_PREFIX=$(ARM64_SEL4_PREFIX) cargo build \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		-Z unstable-options \
		--target $(ARM64_TARGET) \
		--package $(DEV_CRATE) \
		--target-dir $(BUILD_DIR)/target \
		--artifact-dir $(BUILD_DIR)
	@echo "==> Creating bootable image..."
	sel4-kernel-loader-add-payload \
		--loader $(ARM64_SEL4_PREFIX)/bin/sel4-kernel-loader \
		--sel4-prefix $(ARM64_SEL4_PREFIX) \
		--app $(BUILD_DIR)/$(DEV_CRATE).elf \
		-o $(ARM64_IMAGE)
	@echo "==> Image created: $(ARM64_IMAGE)"

.PHONY: _debug-x86_64
_debug-x86_64:
	@echo "==> Building debug binary (qemu-x86_64)..."
	@mkdir -p $(X86_IMAGE_DIR)/EFI/BOOT
	@mkdir -p $(X86_IMAGE_DIR)/lona

	SEL4_PREFIX=$(X86_SEL4_PREFIX_DEBUG) cargo build \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		-Z unstable-options \
		--target $(X86_TARGET) \
		--package $(DEV_CRATE) \
		--target-dir $(BUILD_DIR)/target-x86_64-dev \
		--artifact-dir $(BUILD_DIR)/x86_64-dev-artifacts

	@echo "==> Creating GRUB EFI binary..."
	@# Bootstrap config: search for the drive containing grub.cfg, then load it
	printf 'search --file --set=root /lona/grub.cfg\nconfigfile ($$root)/lona/grub.cfg\n' > $(BUILD_DIR)/grub-bootstrap.cfg
	grub-mkstandalone \
		-O x86_64-efi \
		--modules="part_gpt part_msdos fat normal configfile multiboot multiboot2 boot all_video efi_gop efi_uga search" \
		--locales="" \
		--themes="" \
		-o "$(X86_IMAGE_DIR)/EFI/BOOT/BOOTX64.EFI" \
		"boot/grub/grub.cfg=$(BUILD_DIR)/grub-bootstrap.cfg"

	@echo "==> Assembling QEMU image..."
	cp $(X86_SEL4_PREFIX_DEBUG)/bin/kernel.elf $(X86_IMAGE_DIR)/lona/kernel-x86_64.elf
	cp $(BUILD_DIR)/x86_64-dev-artifacts/$(DEV_CRATE).elf $(X86_IMAGE_DIR)/lona/lona-x86_64.elf
	cp support/boot/grub-x86_64.cfg $(X86_IMAGE_DIR)/lona/grub.cfg
	@echo "==> Image created: $(X86_IMAGE_DIR)/"

.PHONY: _build-test-arm64
_build-test-arm64:
	@echo "==> Building ARM64 test binary (Debug)..."
	@mkdir -p $(BUILD_DIR)
	SEL4_PREFIX=$(ARM64_SEL4_PREFIX) cargo build \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		-Z unstable-options \
		--target $(ARM64_TARGET) \
		--package $(DEV_CRATE) \
		--features integration-test \
		--target-dir $(BUILD_DIR)/target-test-arm64 \
		--artifact-dir $(BUILD_DIR)/test-artifacts-arm64
	@echo "==> Creating ARM64 test image..."
	sel4-kernel-loader-add-payload \
		--loader $(ARM64_SEL4_PREFIX)/bin/sel4-kernel-loader \
		--sel4-prefix $(ARM64_SEL4_PREFIX) \
		--app $(BUILD_DIR)/test-artifacts-arm64/$(DEV_CRATE).elf \
		-o $(BUILD_DIR)/lona-test-arm64.elf
	@echo "==> Test image created: $(BUILD_DIR)/lona-test-arm64.elf"

.PHONY: _build-test-x86_64
_build-test-x86_64:
	@echo "==> Building x86_64 test binary (Debug)..."
	@mkdir -p $(BUILD_DIR)/qemu-x86_64-test/EFI/BOOT
	@mkdir -p $(BUILD_DIR)/qemu-x86_64-test/lona

	SEL4_PREFIX=$(X86_SEL4_PREFIX_DEBUG) cargo build \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		-Z unstable-options \
		--target $(X86_TARGET) \
		--package $(DEV_CRATE) \
		--features integration-test \
		--target-dir $(BUILD_DIR)/target-test-x86_64 \
		--artifact-dir $(BUILD_DIR)/test-artifacts-x86_64

	@echo "==> Creating GRUB EFI binary for test..."
	@# Bootstrap config: search for the drive containing grub.cfg, then load it
	printf 'search --file --set=root /lona/grub.cfg\nconfigfile ($$root)/lona/grub.cfg\n' > $(BUILD_DIR)/grub-bootstrap-test.cfg
	grub-mkstandalone \
		-O x86_64-efi \
		--modules="part_gpt part_msdos fat normal configfile multiboot multiboot2 boot all_video efi_gop efi_uga search" \
		--locales="" \
		--themes="" \
		-o "$(BUILD_DIR)/qemu-x86_64-test/EFI/BOOT/BOOTX64.EFI" \
		"boot/grub/grub.cfg=$(BUILD_DIR)/grub-bootstrap-test.cfg"

	@echo "==> Assembling x86_64 test image..."
	cp $(X86_SEL4_PREFIX_DEBUG)/bin/kernel.elf $(BUILD_DIR)/qemu-x86_64-test/lona/kernel-x86_64.elf
	cp $(BUILD_DIR)/test-artifacts-x86_64/$(DEV_CRATE).elf $(BUILD_DIR)/qemu-x86_64-test/lona/lona-x86_64.elf
	cp support/boot/grub-x86_64.cfg $(BUILD_DIR)/qemu-x86_64-test/lona/grub.cfg
	@echo "==> Test image created: $(BUILD_DIR)/qemu-x86_64-test/"

.PHONY: _test-arm64
_test-arm64:
	TIMEOUT=$(TEST_TIMEOUT) ./scripts/run-integration-tests.sh aarch64 $(BUILD_DIR)/lona-test-arm64.elf

.PHONY: _test-x86_64
_test-x86_64:
	TIMEOUT=$(TEST_TIMEOUT) ./scripts/run-integration-tests.sh x86_64 $(BUILD_DIR)/qemu-x86_64-test

.PHONY: _run-arm64
_run-arm64:
	$(QEMU_ARM64) \
		-machine $(QEMU_ARM64_MACHINE) \
		-cpu $(QEMU_ARM64_CPU) \
		-m $(QEMU_ARM64_MEMORY) \
		-nographic \
		-serial mon:stdio \
		-kernel $(ARM64_IMAGE)

.PHONY: _run-x86_64
_run-x86_64:
	$(QEMU_X86) \
		-machine $(QEMU_X86_MACHINE) \
		-cpu $(QEMU_X86_CPU) \
		-m $(QEMU_X86_MEMORY) \
		-bios $(OVMF_CODE) \
		-drive format=raw,file=fat:rw:$(X86_IMAGE_DIR) \
		-nographic \
		-serial mon:stdio

# ==============================================================================
# Internal: x86_64 Release Build
# ==============================================================================

.PHONY: _release-x86_64
_release-x86_64:
	@echo "==> Building Lona for x86_64..."
	@mkdir -p $(BUILD_DIR)/release-x86_64/EFI/BOOT
	@mkdir -p $(BUILD_DIR)/release-x86_64/lona

	@echo "==> Compiling root task..."
	SEL4_PREFIX=$(X86_SEL4_PREFIX_RELEASE) cargo build \
		--release \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		-Z unstable-options \
		--target $(X86_TARGET) \
		--package $(DEV_CRATE) \
		--target-dir $(BUILD_DIR)/target-x86_64 \
		--artifact-dir $(BUILD_DIR)/x86_64-artifacts

	@echo "==> Creating GRUB EFI binary..."
	@# Bootstrap config: search for the drive containing grub.cfg, then load it
	printf 'search --file --set=root /lona/grub.cfg\nconfigfile ($$root)/lona/grub.cfg\n' > $(BUILD_DIR)/grub-bootstrap.cfg
	grub-mkstandalone \
		-O x86_64-efi \
		--modules="part_gpt part_msdos fat normal configfile multiboot multiboot2 boot all_video efi_gop efi_uga search" \
		--locales="" \
		--themes="" \
		-o "$(BUILD_DIR)/release-x86_64/EFI/BOOT/BOOTX64.EFI" \
		"boot/grub/grub.cfg=$(BUILD_DIR)/grub-bootstrap.cfg"

	@echo "==> Assembling release bundle..."
	cp $(X86_SEL4_PREFIX_RELEASE)/bin/kernel.elf $(BUILD_DIR)/release-x86_64/lona/kernel-x86_64.elf
	cp $(BUILD_DIR)/x86_64-artifacts/$(DEV_CRATE).elf $(BUILD_DIR)/release-x86_64/lona/lona-x86_64.elf
	cp support/boot/grub-x86_64.cfg $(BUILD_DIR)/release-x86_64/lona/grub.cfg

	@echo ""
	@echo "==> x86_64 release ready: $(BUILD_DIR)/release-x86_64/"
	@echo "    Copy contents to FAT32 EFI partition to boot"
	@echo ""
	@ls -la $(BUILD_DIR)/release-x86_64/

# ==============================================================================
# Internal: Raspberry Pi 4B Release Build
# ==============================================================================

RPI4_MEMORY ?= 8192
RPI4_VARIANT := rpi4b-$(shell echo $$(( $(RPI4_MEMORY) / 1024 )))gb

.PHONY: _release-rpi4b
_release-rpi4b:
	$(eval RPI4_SEL4_PREFIX := $(if $(filter 4096,$(RPI4_MEMORY)),$(RPI4_SEL4_PREFIX_4GB),$(RPI4_SEL4_PREFIX_8GB)))
	@echo "==> Building Lona for Raspberry Pi 4B ($(RPI4_MEMORY)MB)..."
	@mkdir -p $(BUILD_DIR)/release-$(RPI4_VARIANT)/lona
	@mkdir -p $(BUILD_DIR)/release-$(RPI4_VARIANT)/overlays

	@echo "==> Compiling root task..."
	SEL4_PREFIX=$(RPI4_SEL4_PREFIX) cargo build \
		--release \
		-Z build-std=core,alloc \
		-Z build-std-features=compiler-builtins-mem \
		-Z unstable-options \
		--target $(RPI4_TARGET) \
		--package $(DEV_CRATE) \
		--target-dir $(BUILD_DIR)/target-rpi4 \
		--artifact-dir $(BUILD_DIR)/rpi4-artifacts

	@echo "==> Creating kernel-loader bundle..."
	sel4-kernel-loader-add-payload \
		--loader $(RPI4_SEL4_PREFIX)/bin/sel4-kernel-loader \
		--sel4-prefix $(RPI4_SEL4_PREFIX) \
		--app $(BUILD_DIR)/rpi4-artifacts/$(DEV_CRATE).elf \
		-o $(BUILD_DIR)/release-$(RPI4_VARIANT)/lona/lona-rpi4b.elf

	@echo "==> Copying Raspberry Pi firmware..."
	cp /opt/rpi-firmware/boot/start4.elf $(BUILD_DIR)/release-$(RPI4_VARIANT)/
	cp /opt/rpi-firmware/boot/fixup4.dat $(BUILD_DIR)/release-$(RPI4_VARIANT)/
	cp /opt/rpi-firmware/boot/bcm2711-rpi-4-b.dtb $(BUILD_DIR)/release-$(RPI4_VARIANT)/
	cp -r /opt/rpi-firmware/boot/overlays/* $(BUILD_DIR)/release-$(RPI4_VARIANT)/overlays/

	@echo "==> Copying U-Boot..."
	cp $(RPI4_SEL4_PREFIX)/u-boot.bin $(BUILD_DIR)/release-$(RPI4_VARIANT)/

	@echo "==> Generating boot configuration..."
	cp support/boot/rpi4b-config.txt $(BUILD_DIR)/release-$(RPI4_VARIANT)/config.txt
	mkimage -A arm64 -O linux -T script -C none -n "Lona Boot" \
		-d support/boot/rpi4b-boot.txt $(BUILD_DIR)/release-$(RPI4_VARIANT)/boot.scr

	@echo ""
	@echo "==> RPi4B release ready: $(BUILD_DIR)/release-$(RPI4_VARIANT)/"
	@echo "    Copy contents to FAT32 SD card to boot"
	@echo ""
	@ls -la $(BUILD_DIR)/release-$(RPI4_VARIANT)/
