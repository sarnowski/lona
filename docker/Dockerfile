# syntax=docker/dockerfile:1
# Lona Build Environment
#
# Toolchain-only image for building Lona.
# seL4 kernel is built at make time, not docker build time.
#
# Usage (via Makefile):
#   make env      - Build this image
#   make shell    - Interactive shell with project mounted at /lona
#   make x86_64   - Build for x86_64
#   make aarch64  - Build for aarch64
#
# The Makefile mounts the project at /lona. All build artifacts
# (seL4, cargo, images) are written to the host filesystem.

FROM debian:bookworm

ENV DEBIAN_FRONTEND=noninteractive

# ============================================================
# System packages
# ============================================================

RUN apt-get update && apt-get install -y \
    # Essential build tools
    build-essential \
    cmake \
    ninja-build \
    # Version control and download
    git \
    curl \
    wget \
    # Python (for seL4 build system)
    python3 \
    python3-dev \
    python3-pip \
    python3-venv \
    python3-setuptools \
    # seL4 build dependencies
    device-tree-compiler \
    libxml2-utils \
    ncurses-dev \
    # Cross-compilers
    gcc-aarch64-linux-gnu \
    g++-aarch64-linux-gnu \
    # QEMU emulators
    qemu-system-arm \
    qemu-system-x86 \
    qemu-system-misc \
    # UEFI firmware for QEMU
    qemu-efi-aarch64 \
    ovmf \
    # For Rust bindgen
    libclang-dev \
    # For kernel loader
    u-boot-tools \
    # GRUB2 for x86_64 UEFI boot
    grub-efi-amd64-bin \
    # Misc utilities
    xxd \
    xz-utils \
    cpio \
    && rm -rf /var/lib/apt/lists/*

# ============================================================
# Python packages (seL4 build system)
# ============================================================

RUN pip3 install --break-system-packages sel4-deps pexpect

# ============================================================
# Rust toolchain
# ============================================================

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    --default-toolchain nightly

ENV PATH="/root/.cargo/bin:${PATH}"

RUN rustup component add rust-src llvm-tools-preview rustfmt clippy

# ============================================================
# Cargo tools
# ============================================================

RUN cargo install cargo-llvm-cov

# Install sel4-kernel-loader-add-payload from rust-sel4
# Used to create combined kernel+loader+app images for aarch64
ARG RUST_SEL4_VERSION=3.0.0
RUN cargo install \
    --git https://github.com/seL4/rust-sel4.git \
    --tag v${RUST_SEL4_VERSION} \
    sel4-kernel-loader-add-payload

# ============================================================
# Working directory
# ============================================================

WORKDIR /lona

CMD ["/bin/bash"]
