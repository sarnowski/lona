# Custom Rust Target Specifications

This directory contains custom Rust target specifications for compiling
Lona on seL4 platforms. These targets define the ABI, linker settings, and
platform features needed for bare-metal seL4 userspace.

## Files

- `aarch64-sel4.json` - Target for ARM64 platforms (QEMU virt, Raspberry Pi 4B)
- `x86_64-sel4.json` - Target for x86_64 platforms (QEMU, bare metal PCs)

## Usage

These targets are used automatically by the Makefile build system:

```bash
make build         # Uses aarch64-sel4.json
make build-x86_64  # Uses x86_64-sel4.json
```

## Creating New Targets

For guidance on adding support for additional platforms, see the
"Adding Support for Other ARM Boards" appendix in `docs/installation.md`.
