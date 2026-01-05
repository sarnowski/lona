# ============================================================
# Lona Build System
# Make 3.81+ compatible
# ============================================================
#
# Usage:
#   make env              - Build Docker toolchain image
#   make x86_64           - Build bootable image for x86_64
#   make aarch64          - Build bootable image for aarch64
#   make run-x86_64       - Run x86_64 in QEMU
#   make run-aarch64      - Run aarch64 in QEMU
#   make verify           - Run all checks
#   make clean            - Remove build cache (Docker volume)
#   make clean-all        - Remove everything including Docker volume
#
# ============================================================

# Configuration
DOCKER_IMAGE = lona-build
SEL4_VERSION = 14.0.0
RUST_SEL4_VERSION = 3.0.0

# Version derived from git
VERSION := $(shell git describe --tags --always --dirty)

# QEMU x86_64 configuration
QEMU_X86           = qemu-system-x86_64
QEMU_X86_MACHINE   = q35
QEMU_X86_CPU       = Cascadelake-Server
QEMU_X86_MEMORY    = 1G
OVMF_CODE          = /usr/share/OVMF/OVMF_CODE.fd

# QEMU aarch64 configuration
QEMU_AARCH64         = qemu-system-aarch64
QEMU_AARCH64_MACHINE = virt,virtualization=on,highmem=off,secure=off
QEMU_AARCH64_CPU     = cortex-a57
QEMU_AARCH64_MEMORY  = 1G
QEMU_AARCH64_SMP     = 4

# Docker volumes (named volumes avoid macOS VirtioFS timestamp bugs)
VOLUME_BUILD = lona-build-cache
VOLUME_CARGO = lona-cargo-cache
VOLUME_RUSTUP = lona-rustup-cache

# Python virtual environment (for MCP server, runs on host)
VENV = .venv
PYTHON = $(VENV)/bin/python

# Docker run commands
#
# Mount layout:
#   /source       - Project source (bind mount from host, read-write)
#   /build        - All build artifacts (Docker named volume)
#   /root/.cargo  - Cargo cache (Docker named volume)
#   /root/.rustup - Rustup toolchains (Docker named volume)
#
# Using named volumes avoids macOS VirtioFS timestamp precision bugs
# that cause unnecessary rebuilds with bind mounts.
#
# DOCKER: for non-interactive commands (-i for Ctrl+C signal handling)
# DOCKER_IT: for interactive commands needing a TTY (shell, run-*)
DOCKER = docker run --rm -i \
	--platform linux/amd64 \
	-v "$(CURDIR):/source" \
	-v $(VOLUME_BUILD):/build \
	-v $(VOLUME_CARGO):/root/.cargo \
	-v $(VOLUME_RUSTUP):/root/.rustup \
	-w /source \
	$(DOCKER_IMAGE)

DOCKER_IT = docker run --rm -it \
	--platform linux/amd64 \
	-v "$(CURDIR):/source" \
	-v $(VOLUME_BUILD):/build \
	-v $(VOLUME_CARGO):/root/.cargo \
	-v $(VOLUME_RUSTUP):/root/.rustup \
	-w /source \
	$(DOCKER_IMAGE)

# Marker files for make dependency tracking (on host filesystem)
# These track what's been built in the Docker volume
MARKER_DIR = .build-markers
MARKER_ENV = $(MARKER_DIR)/docker-image
MARKER_SEL4_SRC = $(MARKER_DIR)/sel4-src
MARKER_X86_64_KERNEL = $(MARKER_DIR)/sel4-x86_64-kernel
MARKER_AARCH64_KERNEL = $(MARKER_DIR)/sel4-aarch64-kernel
MARKER_AARCH64_LOADER = $(MARKER_DIR)/sel4-aarch64-loader

# ============================================================
# Default target
# ============================================================

.PHONY: help
help:
	@echo "Lona Build System"
	@echo ""
	@echo "Setup:"
	@echo "  make env              Build Docker toolchain image"
	@echo "  make shell            Interactive shell in build environment"
	@echo ""
	@echo "Build:"
	@echo "  make x86_64           Build bootable image for x86_64"
	@echo "  make aarch64          Build bootable image for aarch64"
	@echo ""
	@echo "Run:"
	@echo "  make run-x86_64       Run x86_64 in QEMU"
	@echo "  make run-aarch64      Run aarch64 in QEMU"
	@echo ""
	@echo "Test:"
	@echo "  make x86_64-test      Run E2E tests for x86_64"
	@echo "  make aarch64-test     Run E2E tests for aarch64"
	@echo "  make integration-test Run all E2E tests"
	@echo ""
	@echo "Development:"
	@echo "  make format           Check code formatting"
	@echo "  make clippy           Run clippy lints"
	@echo "  make test             Run unit tests"
	@echo "  make verify           Run all checks"
	@echo "  make venv             Create Python virtual environment"
	@echo "  make mcp              Start MCP server for AI agents"
	@echo ""
	@echo "Documentation:"
	@echo "  make docs             Build documentation site"
	@echo "  make docs-browse      Serve documentation locally"
	@echo ""
	@echo "Cleanup:"
	@echo "  make clean            Remove Rust target cache"
	@echo "  make clean-all        Remove entire build cache volume"

# ============================================================
# Docker Environment
# ============================================================

.PHONY: env
env: $(MARKER_ENV)

$(MARKER_ENV): docker/Dockerfile
	@mkdir -p $(MARKER_DIR)
	docker build --platform linux/amd64 \
		--build-arg RUST_SEL4_VERSION=$(RUST_SEL4_VERSION) \
		-t $(DOCKER_IMAGE) docker/
	@touch $(MARKER_ENV)

.PHONY: shell
shell: $(MARKER_ENV)
	$(DOCKER_IT) /bin/bash

# ============================================================
# seL4 Sources
# ============================================================

.PHONY: sel4-sources
sel4-sources: $(MARKER_SEL4_SRC)

$(MARKER_SEL4_SRC): $(MARKER_ENV)
	@mkdir -p $(MARKER_DIR)
	$(DOCKER) sh -c '\
		if [ ! -d /build/sel4-src ]; then \
			echo "=== Cloning seL4 $(SEL4_VERSION) ===" && \
			git clone --depth 1 --branch $(SEL4_VERSION) \
				https://github.com/seL4/seL4.git /build/sel4-src ; \
		fi'
	@touch $(MARKER_SEL4_SRC)

# ============================================================
# seL4 Kernel Builds
# ============================================================

.PHONY: x86_64-kernel
x86_64-kernel: $(MARKER_X86_64_KERNEL)

$(MARKER_X86_64_KERNEL): $(MARKER_SEL4_SRC)
	@mkdir -p $(MARKER_DIR)
	@echo "=== Building seL4 kernel for x86_64 ==="
	$(DOCKER) sh -c '\
		mkdir -p /build/sel4-x86_64-build /build/sel4-x86_64 && \
		cmake -GNinja \
			-S /build/sel4-src \
			-B /build/sel4-x86_64-build \
			-DCROSS_COMPILER_PREFIX="" \
			-DCMAKE_INSTALL_PREFIX=/build/sel4-x86_64 \
			-DKernelPlatform=pc99 \
			-DKernelSel4Arch=x86_64 \
			-DKernelMaxNumNodes=64 \
			-DKernelIsMCS=ON \
			-DKernelVerificationBuild=OFF \
			-DKernelDebugBuild=ON \
			-DKernelPrinting=ON \
			-DKernelSupportPCID=OFF \
			-DLibSel4FunctionAttributes=public && \
		ninja -C /build/sel4-x86_64-build all && \
		ninja -C /build/sel4-x86_64-build install && \
		mkdir -p /build/sel4-x86_64/bin && \
		cp $$(find /build/sel4-x86_64-build -name kernel.elf -type f | head -1) \
			/build/sel4-x86_64/bin/kernel.elf'
	@touch $(MARKER_X86_64_KERNEL)

.PHONY: aarch64-kernel
aarch64-kernel: $(MARKER_AARCH64_KERNEL)

$(MARKER_AARCH64_KERNEL): $(MARKER_SEL4_SRC)
	@mkdir -p $(MARKER_DIR)
	@echo "=== Building seL4 kernel for aarch64 ==="
	$(DOCKER) sh -c '\
		mkdir -p /build/sel4-aarch64-build /build/sel4-aarch64 && \
		cmake -GNinja \
			-S /build/sel4-src \
			-B /build/sel4-aarch64-build \
			-DCROSS_COMPILER_PREFIX=aarch64-linux-gnu- \
			-DCMAKE_INSTALL_PREFIX=/build/sel4-aarch64 \
			-DKernelPlatform=qemu-arm-virt \
			-DKernelSel4Arch=aarch64 \
			-DKernelMaxNumNodes=4 \
			-DKernelIsMCS=ON \
			-DKernelArmHypervisorSupport=ON \
			-DKernelVerificationBuild=OFF \
			-DKernelDebugBuild=ON \
			-DKernelPrinting=ON \
			-DARM_CPU=cortex-a57 \
			-DLibSel4FunctionAttributes=public && \
		ninja -C /build/sel4-aarch64-build all && \
		ninja -C /build/sel4-aarch64-build install'
	@touch $(MARKER_AARCH64_KERNEL)

.PHONY: aarch64-loader
aarch64-loader: $(MARKER_AARCH64_LOADER)

$(MARKER_AARCH64_LOADER): $(MARKER_AARCH64_KERNEL)
	@mkdir -p $(MARKER_DIR)
	@echo "=== Building kernel loader for aarch64 ==="
	$(DOCKER) sh -c '\
		if [ ! -d /build/rust-sel4 ]; then \
			git clone --depth 1 --branch v$(RUST_SEL4_VERSION) \
				https://github.com/seL4/rust-sel4.git /build/rust-sel4 ; \
		fi && \
		export SEL4_PREFIX=/build/sel4-aarch64 && \
		export SEL4_INCLUDE_DIRS=/build/sel4-aarch64/libsel4/include && \
		export RUST_TARGET_PATH=/build/rust-sel4/support/targets && \
		export CARGO_TARGET_DIR=/build/rust-sel4-target && \
		export CC=aarch64-linux-gnu-gcc && \
		export AR=aarch64-linux-gnu-ar && \
		cd /build/rust-sel4/crates/sel4-kernel-loader && \
		cargo build --release --target aarch64-sel4 \
			-Z build-std=core,alloc,compiler_builtins \
			-Z build-std-features=compiler-builtins-mem && \
		mkdir -p /build/sel4-aarch64/bin && \
		cp /build/rust-sel4-target/aarch64-sel4/release/sel4-kernel-loader.elf \
			/build/sel4-aarch64/bin/sel4-kernel-loader'
	@touch $(MARKER_AARCH64_LOADER)

# ============================================================
# Rust Builds
# ============================================================

# Common cargo flags for seL4 builds
CARGO_SEL4_FLAGS = -Z build-std=core,alloc,compiler_builtins -Z build-std-features=compiler-builtins-mem

.PHONY: x86_64-build
x86_64-build: $(MARKER_X86_64_KERNEL)
	@echo "=== Building Lona VM for x86_64 (release) ==="
	$(DOCKER) env \
		LONA_VERSION=$(VERSION) \
		SEL4_PREFIX=/build/sel4-x86_64 \
		SEL4_INCLUDE_DIRS=/build/sel4-x86_64/libsel4/include \
		RUST_TARGET_PATH=/source/targets \
		CARGO_TARGET_DIR=/build/target/x86_64 \
		cargo build --release --target x86_64-sel4 \
			-p lona-vm --no-default-features --features sel4 \
			$(CARGO_SEL4_FLAGS)
	@echo "=== Building Lona Memory Manager for x86_64 (release, with embedded VM) ==="
	$(DOCKER) env \
		LONA_VERSION=$(VERSION) \
		SEL4_PREFIX=/build/sel4-x86_64 \
		SEL4_INCLUDE_DIRS=/build/sel4-x86_64/libsel4/include \
		RUST_TARGET_PATH=/source/targets \
		CARGO_TARGET_DIR=/build/target/x86_64 \
		LONA_VM_ELF=/build/target/x86_64/x86_64-sel4/release/lona-vm.elf \
		cargo build --release --target x86_64-sel4 \
			-p lona-memory-manager --no-default-features --features sel4 \
			$(CARGO_SEL4_FLAGS)

.PHONY: aarch64-build
aarch64-build: $(MARKER_AARCH64_KERNEL)
	@echo "=== Building Lona VM for aarch64 (release) ==="
	$(DOCKER) env \
		LONA_VERSION=$(VERSION) \
		SEL4_PREFIX=/build/sel4-aarch64 \
		SEL4_INCLUDE_DIRS=/build/sel4-aarch64/libsel4/include \
		RUST_TARGET_PATH=/source/targets \
		CARGO_TARGET_DIR=/build/target/aarch64 \
		cargo build --release --target aarch64-sel4 \
			-p lona-vm --no-default-features --features sel4 \
			$(CARGO_SEL4_FLAGS)
	@echo "=== Building Lona Memory Manager for aarch64 (release, with embedded VM) ==="
	$(DOCKER) env \
		LONA_VERSION=$(VERSION) \
		SEL4_PREFIX=/build/sel4-aarch64 \
		SEL4_INCLUDE_DIRS=/build/sel4-aarch64/libsel4/include \
		RUST_TARGET_PATH=/source/targets \
		CARGO_TARGET_DIR=/build/target/aarch64 \
		LONA_VM_ELF=/build/target/aarch64/aarch64-sel4/release/lona-vm.elf \
		cargo build --release --target aarch64-sel4 \
			-p lona-memory-manager --no-default-features --features sel4 \
			$(CARGO_SEL4_FLAGS)

.PHONY: x86_64-build-test
x86_64-build-test: $(MARKER_X86_64_KERNEL)
	@echo "=== Building Lona VM for x86_64 (test) ==="
	$(DOCKER) env \
		LONA_VERSION=$(VERSION) \
		SEL4_PREFIX=/build/sel4-x86_64 \
		SEL4_INCLUDE_DIRS=/build/sel4-x86_64/libsel4/include \
		RUST_TARGET_PATH=/source/targets \
		CARGO_TARGET_DIR=/build/target/x86_64 \
		cargo build --target x86_64-sel4 \
			-p lona-vm --no-default-features --features e2e-test \
			$(CARGO_SEL4_FLAGS)
	@echo "=== Building Lona Memory Manager for x86_64 (test, with embedded VM) ==="
	$(DOCKER) env \
		LONA_VERSION=$(VERSION) \
		SEL4_PREFIX=/build/sel4-x86_64 \
		SEL4_INCLUDE_DIRS=/build/sel4-x86_64/libsel4/include \
		RUST_TARGET_PATH=/source/targets \
		CARGO_TARGET_DIR=/build/target/x86_64 \
		LONA_VM_ELF=/build/target/x86_64/x86_64-sel4/debug/lona-vm.elf \
		cargo build --target x86_64-sel4 \
			-p lona-memory-manager --no-default-features --features sel4 \
			$(CARGO_SEL4_FLAGS)

.PHONY: aarch64-build-test
aarch64-build-test: $(MARKER_AARCH64_KERNEL)
	@echo "=== Building Lona VM for aarch64 (test) ==="
	$(DOCKER) env \
		LONA_VERSION=$(VERSION) \
		SEL4_PREFIX=/build/sel4-aarch64 \
		SEL4_INCLUDE_DIRS=/build/sel4-aarch64/libsel4/include \
		RUST_TARGET_PATH=/source/targets \
		CARGO_TARGET_DIR=/build/target/aarch64 \
		cargo build --target aarch64-sel4 \
			-p lona-vm --no-default-features --features e2e-test \
			$(CARGO_SEL4_FLAGS)
	@echo "=== Building Lona Memory Manager for aarch64 (test, with embedded VM) ==="
	$(DOCKER) env \
		LONA_VERSION=$(VERSION) \
		SEL4_PREFIX=/build/sel4-aarch64 \
		SEL4_INCLUDE_DIRS=/build/sel4-aarch64/libsel4/include \
		RUST_TARGET_PATH=/source/targets \
		CARGO_TARGET_DIR=/build/target/aarch64 \
		LONA_VM_ELF=/build/target/aarch64/aarch64-sel4/debug/lona-vm.elf \
		cargo build --target aarch64-sel4 \
			-p lona-memory-manager --no-default-features --features sel4 \
			$(CARGO_SEL4_FLAGS)

# ============================================================
# Bootable Images
# ============================================================

.PHONY: x86_64-image
x86_64-image: x86_64-build
	@echo "=== Creating x86_64 bootable image ==="
	$(DOCKER) sh -c '\
		rm -rf /build/images/x86_64 && \
		mkdir -p /build/images/x86_64/EFI/BOOT /build/images/x86_64/boot && \
		cp /build/sel4-x86_64/bin/kernel.elf /build/images/x86_64/boot/kernel.elf && \
		cp /build/target/x86_64/x86_64-sel4/release/lona-root-task.elf \
			/build/images/x86_64/boot/rootserver.elf && \
		cp /build/target/x86_64/x86_64-sel4/release/lona-vm.elf \
			/build/images/x86_64/boot/lona-vm.elf && \
		truncate -s ">1M" /build/images/x86_64/boot/rootserver.elf && \
		echo "set timeout=0" > /build/images/x86_64/boot/grub.cfg && \
		echo "set default=0" >> /build/images/x86_64/boot/grub.cfg && \
		echo "serial --unit=0 --speed=115200" >> /build/images/x86_64/boot/grub.cfg && \
		echo "terminal_output serial" >> /build/images/x86_64/boot/grub.cfg && \
		echo "terminal_input serial" >> /build/images/x86_64/boot/grub.cfg && \
		echo "search --set=root --file /boot/kernel.elf" >> /build/images/x86_64/boot/grub.cfg && \
		echo "menuentry \"Lona\" {" >> /build/images/x86_64/boot/grub.cfg && \
		echo "    multiboot2 /boot/kernel.elf" >> /build/images/x86_64/boot/grub.cfg && \
		echo "    module2 /boot/rootserver.elf" >> /build/images/x86_64/boot/grub.cfg && \
		echo "    module2 /boot/lona-vm.elf" >> /build/images/x86_64/boot/grub.cfg && \
		echo "    boot" >> /build/images/x86_64/boot/grub.cfg && \
		echo "}" >> /build/images/x86_64/boot/grub.cfg && \
		grub-mkstandalone \
			-O x86_64-efi \
			-o /build/images/x86_64/EFI/BOOT/BOOTX64.EFI \
			--modules="part_gpt part_msdos fat normal multiboot2 serial" \
			"/boot/grub/grub.cfg=/build/images/x86_64/boot/grub.cfg"'
	@echo "Image created in Docker volume at /build/images/x86_64/"

# NOTE: aarch64 currently only loads the root task. The sel4-kernel-loader-add-payload
# tool does not support multiple applications. Future options:
# 1. Modify the loader to support multiple payloads
# 2. Embed VM binary within Memory Manager
# 3. Use different boot method (device tree, separate loading)
.PHONY: aarch64-image
aarch64-image: aarch64-build $(MARKER_AARCH64_LOADER)
	@echo "=== Creating aarch64 bootable image ==="
	$(DOCKER) sh -c '\
		rm -rf /build/images/aarch64 && \
		mkdir -p /build/images/aarch64 && \
		sel4-kernel-loader-add-payload \
			--loader /build/sel4-aarch64/bin/sel4-kernel-loader \
			--sel4-prefix /build/sel4-aarch64 \
			--app /build/target/aarch64/aarch64-sel4/release/lona-root-task.elf \
			-o /build/images/aarch64/lona-image.elf'
	@echo "Image created in Docker volume at /build/images/aarch64/"

.PHONY: x86_64-image-test
x86_64-image-test: x86_64-build-test
	@echo "=== Creating x86_64 test image ==="
	$(DOCKER) sh -c '\
		rm -rf /build/images/x86_64 && \
		mkdir -p /build/images/x86_64/EFI/BOOT /build/images/x86_64/boot && \
		cp /build/sel4-x86_64/bin/kernel.elf /build/images/x86_64/boot/kernel.elf && \
		cp /build/target/x86_64/x86_64-sel4/debug/lona-root-task.elf \
			/build/images/x86_64/boot/rootserver.elf && \
		cp /build/target/x86_64/x86_64-sel4/debug/lona-vm.elf \
			/build/images/x86_64/boot/lona-vm.elf && \
		truncate -s ">1M" /build/images/x86_64/boot/rootserver.elf && \
		echo "set timeout=0" > /build/images/x86_64/boot/grub.cfg && \
		echo "set default=0" >> /build/images/x86_64/boot/grub.cfg && \
		echo "serial --unit=0 --speed=115200" >> /build/images/x86_64/boot/grub.cfg && \
		echo "terminal_output serial" >> /build/images/x86_64/boot/grub.cfg && \
		echo "terminal_input serial" >> /build/images/x86_64/boot/grub.cfg && \
		echo "search --set=root --file /boot/kernel.elf" >> /build/images/x86_64/boot/grub.cfg && \
		echo "menuentry \"Lona\" {" >> /build/images/x86_64/boot/grub.cfg && \
		echo "    multiboot2 /boot/kernel.elf" >> /build/images/x86_64/boot/grub.cfg && \
		echo "    module2 /boot/rootserver.elf" >> /build/images/x86_64/boot/grub.cfg && \
		echo "    module2 /boot/lona-vm.elf" >> /build/images/x86_64/boot/grub.cfg && \
		echo "    boot" >> /build/images/x86_64/boot/grub.cfg && \
		echo "}" >> /build/images/x86_64/boot/grub.cfg && \
		grub-mkstandalone \
			-O x86_64-efi \
			-o /build/images/x86_64/EFI/BOOT/BOOTX64.EFI \
			--modules="part_gpt part_msdos fat normal multiboot2 serial" \
			"/boot/grub/grub.cfg=/build/images/x86_64/boot/grub.cfg"'

.PHONY: aarch64-image-test
aarch64-image-test: aarch64-build-test $(MARKER_AARCH64_LOADER)
	@echo "=== Creating aarch64 test image ==="
	$(DOCKER) sh -c '\
		rm -rf /build/images/aarch64 && \
		mkdir -p /build/images/aarch64 && \
		sel4-kernel-loader-add-payload \
			--loader /build/sel4-aarch64/bin/sel4-kernel-loader \
			--sel4-prefix /build/sel4-aarch64 \
			--app /build/target/aarch64/aarch64-sel4/debug/lona-root-task.elf \
			-o /build/images/aarch64/lona-image.elf'

# Convenience aliases (build + export to dist/)
.PHONY: x86_64 aarch64
x86_64: export-x86_64
aarch64: export-aarch64

# ============================================================
# Export Images to Host
# ============================================================

.PHONY: export-x86_64
export-x86_64: x86_64-image
	@echo "=== Exporting x86_64 image to dist/ ==="
	@rm -rf dist/x86_64
	@mkdir -p dist
	$(DOCKER) sh -c 'cp -r /build/images/x86_64 /source/dist/'
	@echo "Image exported to dist/x86_64/"

.PHONY: export-aarch64
export-aarch64: aarch64-image
	@echo "=== Exporting aarch64 image to dist/ ==="
	@rm -rf dist/aarch64
	@mkdir -p dist
	$(DOCKER) sh -c 'cp -r /build/images/aarch64 /source/dist/'
	@echo "Image exported to dist/aarch64/"

.PHONY: export
export: export-x86_64 export-aarch64
	@echo "=== All images exported to dist/ ==="

# ============================================================
# Run in QEMU
# ============================================================

.PHONY: run-x86_64
run-x86_64: x86_64
	@echo "=== Running Lona x86_64 in QEMU ==="
	$(DOCKER_IT) $(QEMU_X86) \
		-machine $(QEMU_X86_MACHINE) \
		-cpu $(QEMU_X86_CPU) \
		-m $(QEMU_X86_MEMORY) \
		-display none \
		-serial stdio \
		-monitor none \
		-bios $(OVMF_CODE) \
		-drive format=raw,file=fat:rw:/build/images/x86_64 \
		-no-reboot

.PHONY: run-aarch64
run-aarch64: aarch64
	@echo "=== Running Lona aarch64 in QEMU ==="
	$(DOCKER_IT) $(QEMU_AARCH64) \
		-machine $(QEMU_AARCH64_MACHINE) \
		-cpu $(QEMU_AARCH64_CPU) \
		-smp $(QEMU_AARCH64_SMP) \
		-m $(QEMU_AARCH64_MEMORY) \
		-nographic \
		-kernel /build/images/aarch64/lona-image.elf \
		-no-reboot

# ============================================================
# E2E Tests
# ============================================================

.PHONY: x86_64-test
x86_64-test: x86_64-image-test
	@echo "=== Running E2E tests for x86_64 ==="
	python3 scripts/parse-e2e-results.py x86_64 \
		$(DOCKER) $(QEMU_X86) \
			-machine $(QEMU_X86_MACHINE) \
			-cpu $(QEMU_X86_CPU) \
			-m $(QEMU_X86_MEMORY) \
			-display none \
			-serial stdio \
			-monitor none \
			-bios $(OVMF_CODE) \
			-drive format=raw,file=fat:rw:/build/images/x86_64 \
			-no-reboot

.PHONY: aarch64-test
aarch64-test: aarch64-image-test
	@echo "=== Running E2E tests for aarch64 ==="
	python3 scripts/parse-e2e-results.py aarch64 \
		$(DOCKER) $(QEMU_AARCH64) \
			-machine $(QEMU_AARCH64_MACHINE) \
			-cpu $(QEMU_AARCH64_CPU) \
			-smp $(QEMU_AARCH64_SMP) \
			-m $(QEMU_AARCH64_MEMORY) \
			-nographic \
			-kernel /build/images/aarch64/lona-image.elf \
			-no-reboot

.PHONY: integration-test
integration-test: aarch64-test x86_64-test
	@echo "=== All E2E tests passed ==="

# ============================================================
# Development
# ============================================================

.PHONY: format
format: $(MARKER_ENV)
	@echo "=== Formatting code ==="
	$(DOCKER) cargo fmt

.PHONY: clippy
clippy: $(MARKER_ENV)
	@echo "=== Running clippy ==="
	$(DOCKER) env LONA_VERSION=$(VERSION) cargo clippy --all-targets -- -D warnings

.PHONY: test
test: $(MARKER_ENV)
	@echo "=== Running unit tests ==="
	$(DOCKER) sh -c 'LONA_VERSION=$(VERSION) cargo test && LONA_VERSION=$(VERSION) cargo llvm-cov --fail-under-lines 60'

.PHONY: verify
verify: format clippy test integration-test
	@echo "=== All checks passed ==="

# ============================================================
# Python Tooling (runs on host, not in Docker)
# ============================================================

.PHONY: venv
venv: $(VENV)

$(VENV): requirements.txt
	python3 -m venv $(VENV)
	$(VENV)/bin/pip install -r requirements.txt
	@touch $(VENV)

.PHONY: mcp
mcp: $(VENV)
	$(PYTHON) -m tools.lona_dev_repl

# ============================================================
# Documentation (runs on host, not in Docker)
# ============================================================

.PHONY: docs
docs: $(VENV)
	$(VENV)/bin/mkdocs build --strict

.PHONY: docs-browse
docs-browse: $(VENV)
	$(VENV)/bin/mkdocs serve

# ============================================================
# Cleanup
# ============================================================

.PHONY: clean
clean:
	@echo "=== Cleaning Rust target cache ==="
	$(DOCKER) rm -rf /build/target
	@echo "=== Clean complete (seL4 cached) ==="

.PHONY: clean-all
clean-all:
	@echo "=== Removing all Docker volumes ==="
	-docker volume rm $(VOLUME_BUILD) $(VOLUME_CARGO) $(VOLUME_RUSTUP)
	rm -rf $(MARKER_DIR)
	rm -rf dist
	rm -rf $(VENV)
	@echo "=== Clean complete ==="

.PHONY: env-clean
env-clean:
	@echo "=== Removing Docker image ==="
	-docker rmi $(DOCKER_IMAGE)
	rm -f $(MARKER_ENV)
