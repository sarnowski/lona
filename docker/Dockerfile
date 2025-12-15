# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
#
# Dockerfile - Build environment for Lona OS (seL4 + Rust root task)
#
# This Dockerfile creates a complete build environment including:
# - ARM64 cross-compilation toolchain
# - seL4 microkernel (compiled for qemu-arm-virt)
# - Rust nightly with bare-metal target support
# - sel4-kernel-loader for booting
# - QEMU for ARM64 emulation

FROM debian:bookworm-slim

# Build arguments
ARG UID=1000
ARG GID=1000
ARG RUST_CHANNEL=nightly-2025-10-20
ARG SEL4_VERSION=14.0.0

# Prevent interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    # Build essentials
    build-essential \
    cmake \
    ninja-build \
    git \
    curl \
    ca-certificates \
    sudo \
    # ARM64 cross-compiler
    gcc-aarch64-linux-gnu \
    g++-aarch64-linux-gnu \
    # QEMU for ARM64 emulation (including firmware/ROM files)
    qemu-system-arm \
    qemu-efi-aarch64 \
    ipxe-qemu \
    # Python for seL4 build system
    python3 \
    python3-pip \
    python3-venv \
    python3-yaml \
    python3-jinja2 \
    python3-jsonschema \
    python3-ply \
    python3-six \
    python3-future \
    python3-sortedcontainers \
    # Additional tools
    device-tree-compiler \
    libxml2-utils \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
ENV RUSTUP_HOME=/opt/rustup
ENV CARGO_HOME=/opt/cargo
ENV PATH="${CARGO_HOME}/bin:${PATH}"

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain ${RUST_CHANNEL} --profile minimal && \
    rustup component add rust-src rustfmt clippy llvm-tools-preview

# Clone seL4 kernel source
WORKDIR /opt/seL4-src
RUN git clone --depth 1 --branch ${SEL4_VERSION} https://github.com/seL4/seL4.git seL4

# Create Python virtual environment and install seL4 tools
RUN python3 -m venv /opt/seL4-venv && \
    /opt/seL4-venv/bin/pip install --upgrade pip && \
    /opt/seL4-venv/bin/pip install cmake-format pyyaml pyfdt jinja2 ply lxml

ENV PATH="/opt/seL4-venv/bin:${PATH}"

# Build seL4 for QEMU ARM64 (Cortex-A57)
WORKDIR /opt/seL4-src/seL4
RUN mkdir -p build && cd build && \
    cmake -G Ninja \
        -DCMAKE_INSTALL_PREFIX=/opt/seL4 \
        -DCMAKE_TOOLCHAIN_FILE=../gcc.cmake \
        -DCROSS_COMPILER_PREFIX=aarch64-linux-gnu- \
        -DKernelPlatform=qemu-arm-virt \
        -DKernelSel4Arch=aarch64 \
        -DKernelArmHypervisorSupport=ON \
        -DKernelVerificationBuild=OFF \
        -DARM_CPU=cortex-a57 \
        .. && \
    ninja all && \
    ninja install

# Clone rust-sel4 repository for kernel loader and tools
WORKDIR /opt
RUN git clone --depth 1 https://github.com/seL4/rust-sel4.git rust-sel4

# Build sel4-kernel-loader (bare-metal ELF for booting)
# Set cross-compilation environment for ARM64 assembly (linker uses rust-lld per target spec)
WORKDIR /opt/rust-sel4
RUN CC=aarch64-linux-gnu-gcc \
    AS=aarch64-linux-gnu-as \
    AR=aarch64-linux-gnu-ar \
    SEL4_PREFIX=/opt/seL4 cargo build \
    --release \
    -Z build-std=core,alloc,compiler_builtins \
    -Z build-std-features=compiler-builtins-mem \
    --target support/targets/aarch64-sel4.json \
    --package sel4-kernel-loader \
    && mkdir -p /opt/seL4/bin \
    && cp target/aarch64-sel4/release/sel4-kernel-loader.elf /opt/seL4/bin/sel4-kernel-loader

# Install sel4-kernel-loader-add-payload (host CLI tool)
RUN cargo install --locked --path crates/sel4-kernel-loader/add-payload

# Set environment variables for seL4 builds
ENV SEL4_PREFIX=/opt/seL4
ENV SEL4_INSTALL_DIR=/opt/seL4
ENV CROSS_COMPILE=aarch64-linux-gnu-

# Create non-root user for development
RUN groupadd -g ${GID} builder 2>/dev/null || true && \
    useradd -m -u ${UID} -g ${GID} -s /bin/bash builder 2>/dev/null || true && \
    mkdir -p /work && chown -R ${UID}:${GID} /work && \
    echo "builder ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

# Fix permissions for Cargo/Rustup
RUN chmod -R a+rw /opt/cargo /opt/rustup

# Working directory
WORKDIR /work

# Default to non-root user
USER builder

CMD ["bash"]
