# Lona Build Infrastructure

This document describes the build infrastructure for Lona, which builds the seL4 microkernel and the Lona runtime (Rust-based root task).

## Overview

The build system uses Docker to provide a reproducible build environment. All compilation, including QEMU simulation, happens inside Docker containers. The host machine only needs Docker installed.

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          Host Machine                                   │
│                                                                         │
│  Makefile (entry point)                                                 │
│     │                                                                   │
│     ├── make build    → Docker container builds seL4 + Rust root task   │
│     ├── make run      → Docker container runs QEMU with 1GB RAM         │
│     ├── make shell    → Interactive Docker shell for development        │
│     └── make clean    → Clean build artifacts                           │
│                                                                         │
│  Project Structure:                                                     │
│     ├── Cargo.toml              ← Lona runtime crate                    │
│     ├── Makefile                ← Host entry point                      │
│     ├── rust-toolchain.toml     ← Nightly Rust + components             │
│     ├── docker-compose.yml      ← Container orchestration               │
│     ├── .gitignore              ← Ignores build/, target/               │
│     │                                                                   │
│     ├── src/                                                            │
│     │   └── main.rs             ← Root task (prints "Hello Lona!")      │
│     │                                                                   │
│     ├── docker/                                                         │
│     │   ├── Dockerfile          ← seL4 + Rust build environment         │
│     │   └── Makefile            ← Docker image management               │
│     │                                                                   │
│     ├── support/                                                        │
│     │   └── targets/            ← Custom rustc target specifications    │
│     │       └── aarch64-sel4.json                                       │
│     │                                                                   │
│     ├── build/                  ← [GITIGNORED] Build outputs            │
│     └── target/                 ← [GITIGNORED] Cargo build artifacts    │
└─────────────────────────────────────────────────────────────────────────┘
```

## Docker Environment

### Container Contents

The Docker image (`lona-builder`) includes:

| Component | Version | Purpose |
|-----------|---------|---------|
| Debian Bookworm | Latest | Base OS |
| ARM64 Cross-compiler | gcc-aarch64-linux-gnu | Cross-compilation |
| CMake + Ninja | Latest | seL4 build system |
| QEMU | qemu-system-aarch64 | ARM64 emulation |
| QEMU Firmware | qemu-efi-aarch64, ipxe-qemu | QEMU ROM files |
| Rust Nightly | 2024-11-01 | Rust toolchain with `rust-src` |
| seL4 Kernel | 14.0.0 | Microkernel source |
| rust-sel4 crates | Latest from GitHub | Rust bindings for seL4 |
| sel4-kernel-loader | Built from rust-sel4 | Boots kernel + payload |

### Dockerfile

Location: `docker/Dockerfile`

The Dockerfile performs these steps:

1. **Install build dependencies** - Cross-compiler, CMake, Ninja, QEMU, Python packages
2. **Install Rust** - Nightly toolchain with rust-src, rustfmt, clippy, llvm-tools
3. **Clone seL4** - Version 14.0.0 from GitHub
4. **Create Python venv** - For seL4 build tools (cmake-format, pyyaml, pyfdt)
5. **Build seL4** - For qemu-arm-virt platform, Cortex-A57, with hypervisor support
6. **Clone rust-sel4** - For kernel loader and Rust bindings
7. **Build sel4-kernel-loader** - Bare-metal ELF for booting
8. **Install add-payload CLI** - Host tool to combine loader + kernel + root task

### Docker Compose

Location: `docker-compose.yml`

Defines two services:

- **builder** - Main build container with mounted workspace
- **runner** - QEMU execution container (read-only mount)

Uses named volumes for Cargo cache to speed up rebuilds:
- `cargo-cache` - Registry cache
- `cargo-git` - Git dependency cache

## Build Flow

### Step 1: Build seL4 Kernel

The Dockerfile builds seL4 with these CMake options:

```bash
cmake -G Ninja \
    -DCMAKE_INSTALL_PREFIX=/opt/seL4 \
    -DCMAKE_TOOLCHAIN_FILE=../gcc.cmake \
    -DCROSS_COMPILER_PREFIX=aarch64-linux-gnu- \
    -DKernelPlatform=qemu-arm-virt \
    -DKernelSel4Arch=aarch64 \
    -DKernelArmHypervisorSupport=ON \
    -DKernelVerificationBuild=OFF \
    -DARM_CPU=cortex-a57 \
    ..
```

Output: `/opt/seL4/` (inside container)

### Step 2: Build Lona Runtime (Root Task)

```bash
SEL4_PREFIX=/opt/seL4 cargo build \
    --release \
    -Z build-std=core,alloc \
    -Z build-std-features=compiler-builtins-mem \
    --target support/targets/aarch64-sel4.json \
    --package lona-runtime
```

Output: `build/lona-runtime.elf`

### Step 3: Create Bootable Image

```bash
sel4-kernel-loader-add-payload \
    --loader /opt/seL4/bin/sel4-kernel-loader \
    --sel4-prefix /opt/seL4 \
    --app build/lona-runtime.elf \
    -o build/image/lona-qemu.elf
```

Output: `build/image/lona-qemu.elf`

### Step 4: Run in QEMU

```bash
qemu-system-aarch64 \
    -machine virt,virtualization=on \
    -cpu cortex-a57 \
    -m 1G \
    -nographic \
    -serial mon:stdio \
    -kernel build/image/lona-qemu.elf
```

## Makefile Targets

### Build Targets (run in Docker)

| Target | Description |
|--------|-------------|
| `make build` | Build Lona runtime (seL4 root task) |
| `make image` | Build complete bootable image |
| `make run` | Build and run Lona in QEMU (1GB RAM) |
| `make shell` | Start interactive development shell |

### Docker Targets

| Target | Description |
|--------|-------------|
| `make docker-build` | Build the Docker development image |
| `make docker-clean` | Remove Docker image and volumes |

### Development Targets

| Target | Description |
|--------|-------------|
| `make check` | Run all quality checks (fmt, clippy) |
| `make fmt` | Format Rust code |
| `make fmt-check` | Check Rust code formatting |
| `make clippy` | Run clippy lints |

> **Note**: `make build` and `make image` both run `make check` first, ensuring code quality before compilation.

### Clean Targets

| Target | Description |
|--------|-------------|
| `make clean` | Remove build artifacts |
| `make clean-all` | Remove all artifacts including Docker images |

## Rust Configuration

### Package (Cargo.toml)

The root `Cargo.toml` defines the `lona-runtime` crate:
- Dependencies: `sel4`, `sel4-root-task` from rust-sel4 GitHub
- Strict clippy lints for kernel code
- Release profile optimized for size (`opt-level = "z"`, LTO, single codegen unit)

### Toolchain (rust-toolchain.toml)

- Channel: `nightly-2024-11-01`
- Components: rustfmt, clippy, rust-src, llvm-tools-preview
- Target: `aarch64-unknown-none`

### Custom Target (support/targets/aarch64-sel4.json)

Custom rustc target specification for seL4 userspace:
- Architecture: aarch64
- Linker: rust-lld (GNU flavor)
- Panic strategy: abort
- Relocation model: static
- Features: ARMv8-A with NEON, strict alignment

## Lona Runtime

### Entry Point (src/main.rs)

```rust
#![no_std]
#![no_main]

use sel4_root_task::{root_task, Never};

#[root_task]
fn main(bootinfo: &sel4::BootInfoPtr) -> sel4::Result<Never> {
    sel4::debug_println!("Hello Lona!");
    // ... print boot info ...
    loop {
        unsafe { core::arch::asm!("wfi", options(nomem, nostack, preserves_flags)); }
    }
}
```

The root task:
1. Prints "Hello Lona!" to the debug console
2. Prints boot information (untyped memory regions)
3. Enters an infinite WFI (Wait For Interrupt) loop

## Sources and References

- [seL4 Rust Support (rust-sel4)](https://github.com/seL4/rust-sel4)
- [Rust Root Task Demo](https://github.com/seL4/rust-root-task-demo)
- [seL4 Docker Environments](https://docs.sel4.systems/projects/dockerfiles/)
- [QEMU ARM Virt Platform](https://docs.sel4.systems/Hardware/qemu-arm-virt.html)
- [seL4 Rust Documentation](https://docs.sel4.systems/projects/rust/)

---

## Implementation Status

### Completed

- [x] **Project structure** - All directories and files created
- [x] **Makefile** - All targets defined with proper help documentation
- [x] **docker-compose.yml** - Builder and runner services configured
- [x] **Cargo.toml** - Workspace with strict lints and dependencies
- [x] **rust-toolchain.toml** - Nightly Rust with required components
- [x] **aarch64-sel4.json** - Custom target specification
- [x] **lona-runtime crate** - Basic root task that prints "Hello Lona!"
- [x] **.gitignore** - Ignores build/, target/, .venv/, site/
- [x] **.cargo/config.toml** - Cargo configuration
- [x] **Docker image build** - Complete seL4 + Rust environment
- [x] **Rust root task compilation** - Builds successfully with rust-lld
- [x] **sel4-kernel-loader build** - Boots kernel + payload
- [x] **Image creation with add-payload** - Combines loader, kernel, root task
- [x] **QEMU execution** - Boots and prints "Hello Lona!"

### Quick Start

```bash
# Build and run (requires Docker)
make run

# Or step by step:
make docker-build    # Build Docker image (first time only)
make build           # Verify code quality (fmt + clippy) and compile
make image           # Create bootable image
make run             # Run in QEMU

# Clean build artifacts for a fresh build:
make clean
```

> **Note**: On macOS, use `gmake` instead of `make` (install with `brew install make`).

### Expected Output

```
seL4 kernel loader | INFO   Starting loader
seL4 kernel loader | INFO   Entering kernel
Bootstrapping kernel
Booting all finished, dropped to user space
Hello Lona!
Lona runtime initialized
Boot info at: 0x217000
Untyped memory regions: 69
Lona starting...
```

### Future Enhancements

- [ ] CI/CD pipeline integration
- [ ] Multi-platform support (RISC-V, x86_64)
- [ ] Debug build configuration
- [ ] Test infrastructure for kernel tests
