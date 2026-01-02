# Rust Implementation Guide

This document describes the implementation strategy for the Lonala VM in Rust, including project structure, testing methodology, and tooling.

**Related:** [concept.md](concept.md) (architecture) | [lonala.md](lonala.md) (language spec) | [lonala-kernel.md](lonala-kernel.md) (seL4 primitives)

---

## Table of Contents

1. [Why Rust](#why-rust)
2. [Project Structure](#project-structure)
3. [Memory Layout Control](#memory-layout-control)
4. [Testing Strategy](#testing-strategy)
5. [Platform Abstraction](#platform-abstraction)
6. [Linting and Static Analysis](#linting-and-static-analysis)
7. [Code Coverage](#code-coverage)
8. [Fuzzing](#fuzzing)
9. [Cross-Compilation](#cross-compilation)
10. [Development Workflow](#development-workflow)

---

## Why Rust

The Lonala VM combines several challenging requirements:

| Requirement | Challenge |
|-------------|-----------|
| Per-process moving GC | Must not corrupt memory during collection |
| Lock-free data structures | Data races cause silent corruption |
| Capability-based security | Capability leaks compromise isolation |
| seL4 kernel integration | Direct syscalls, no safety net |
| Multicore scheduling | Concurrent access to shared state |

Rust addresses these through:

**Compile-Time Memory Safety** — The borrow checker prevents use-after-free, double-free, and buffer overflows. In a VM managing millions of lightweight processes with per-process heaps, this eliminates entire classes of bugs.

**Concurrency Safety** — The `Send`/`Sync` traits ensure data races are caught at compile time. Lock-free MPSC mailboxes and Chase-Lev deques are notoriously difficult to implement correctly; Rust's type system helps enforce invariants.

**Zero-Cost Abstractions** — Newtypes for `Paddr`, `Vaddr`, `Pid`, and capability types provide compile-time safety with zero runtime overhead. Mixing physical and virtual addresses becomes a compile error, not a runtime crash.

**Explicit Unsafe Boundaries** — Low-level memory operations require `unsafe` blocks, making dangerous code auditable. The VM's unsafe surface area is confined to memory management, seL4 FFI, and atomic operations.

**Excellent Tooling** — Built-in testing, Clippy lints, code coverage, and fuzzing support enable high-quality development from day one.

### Alternative Considered: C

C remains viable due to seL4's native C ecosystem and BEAM VM precedent. However, the combination of moving GC, lock-free structures, and capability plumbing creates high risk for memory safety bugs — exactly where Rust provides the most value.

---

## Project Structure

We use a **single crate with modules** for simplicity during initial development:

```
lona-vm/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Crate root, feature flags
│   │
│   ├── types/              # Core type definitions
│   │   ├── mod.rs
│   │   ├── value.rs        # Tagged value representation
│   │   ├── address.rs      # Paddr, Vaddr newtypes
│   │   └── pid.rs          # Process identifiers
│   │
│   ├── mem/                # Memory management
│   │   ├── mod.rs
│   │   ├── heap.rs         # Per-process heap (down-growing)
│   │   ├── stack.rs        # Per-process stack (up-growing)
│   │   ├── gc.rs           # Generational copying GC
│   │   └── binary.rs       # Reference-counted large binaries
│   │
│   ├── process/            # Process management
│   │   ├── mod.rs
│   │   ├── process.rs      # Process structure and lifecycle
│   │   ├── mailbox.rs      # Lock-free MPSC queue
│   │   └── registry.rs     # Process name registry
│   │
│   ├── sched/              # Scheduling
│   │   ├── mod.rs
│   │   ├── scheduler.rs    # Per-core scheduler
│   │   ├── deque.rs        # Chase-Lev work-stealing deque
│   │   └── reductions.rs   # Reduction counting
│   │
│   ├── vm/                 # Bytecode VM
│   │   ├── mod.rs
│   │   ├── bytecode.rs     # Instruction definitions
│   │   ├── interp.rs       # Interpreter loop
│   │   └── pattern.rs      # Pattern matching
│   │
│   ├── realm/              # Realm management
│   │   ├── mod.rs
│   │   ├── realm.rs        # Realm structure
│   │   ├── namespace.rs    # Var and namespace handling
│   │   └── vspace.rs       # VSpace layout constants
│   │
│   ├── kernel/             # seL4 integration
│   │   ├── mod.rs
│   │   ├── sel4.rs         # seL4 syscall wrappers
│   │   ├── ipc.rs          # IPC primitives
│   │   └── cap.rs          # Capability management
│   │
│   ├── io/                 # Device I/O (for driver realms)
│   │   ├── mod.rs
│   │   ├── mmio.rs         # Memory-mapped I/O
│   │   ├── dma.rs          # DMA buffer management
│   │   └── irq.rs          # Interrupt handling
│   │
│   └── platform/           # Platform abstraction layer
│       ├── mod.rs
│       ├── traits.rs       # Platform trait definitions
│       ├── sel4.rs         # Real seL4 implementation
│       └── mock.rs         # Mock implementation for testing
│
├── tests/                  # Integration tests
│   ├── gc_integration.rs
│   ├── scheduler_integration.rs
│   └── vm_integration.rs
│
└── fuzz/                   # Fuzz targets
    └── fuzz_targets/
        ├── gc.rs
        ├── bytecode.rs
        └── pattern.rs
```

### Why a Single Crate?

- **Simplicity** — One `Cargo.toml`, straightforward dependencies
- **Fast Iteration** — No cross-crate coordination during early development
- **Better Inlining** — Compiler sees all code, optimizes across module boundaries
- **Easy Refactoring** — Move code between modules without crate boundary friction

### When to Split

Consider splitting into workspace crates when:

- Incremental compile times exceed 30-60 seconds
- A component becomes reusable outside the VM
- Team ownership boundaries require separation

---

## Memory Layout Control

Rust provides precise control over memory layout using `#[repr(C)]` and compile-time assertions.

### Struct Layout

```rust
/// Process structure - matches concept.md Section 5
#[repr(C)]
pub struct Process {
    pub pid: u64,
    pub status: ProcessStatus,

    // Memory layout (heap grows down, stack grows up)
    pub heap_start: Vaddr,      // Top of heap region
    pub heap_ptr: Vaddr,        // Current allocation point (moves down)
    pub stack_start: Vaddr,     // Bottom of stack region
    pub stack_ptr: Vaddr,       // Current stack top (moves up)

    // Execution state
    pub ip: Vaddr,              // Instruction pointer
    pub env: Vaddr,             // Environment chain
    pub reductions: u32,        // Reduction counter

    // Mailbox
    pub mailbox: Mailbox,

    // ... additional fields
}

// Compile-time layout verification
const _: () = assert!(core::mem::size_of::<Process>() == EXPECTED_SIZE);
const _: () = assert!(core::mem::offset_of!(Process, heap_ptr) == 0x18);
```

### Address Type Safety

```rust
/// Physical address (hardware/DMA visible)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Paddr(pub u64);

/// Virtual address (CPU visible)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Vaddr(pub u64);

// Mixing these is a compile error:
// fn map_page(vaddr: Vaddr, paddr: Paddr) { ... }
// map_page(physical, virtual)  // ERROR: expected Vaddr, found Paddr
```

### VSpace Layout Constants

```rust
/// Virtual address space layout - matches concept.md Section 14
pub mod vspace {
    use super::Vaddr;

    pub const NULL_GUARD: Vaddr        = Vaddr(0x0000_0000_0000);
    pub const GLOBAL_CONTROL: Vaddr    = Vaddr(0x0000_0010_0000);
    pub const SCHEDULER_STATE: Vaddr   = Vaddr(0x0000_0020_0000);
    pub const NAMESPACE_RO: Vaddr      = Vaddr(0x0000_0100_0000);
    pub const NAMESPACE_RW: Vaddr      = Vaddr(0x0000_0200_0000);
    pub const NAMESPACE_OBJECTS: Vaddr = Vaddr(0x0000_1000_0000);
    pub const ANCESTOR_CODE: Vaddr     = Vaddr(0x0000_2000_0000);
    pub const LOCAL_CODE: Vaddr        = Vaddr(0x0000_3000_0000);
    pub const PROCESS_HEAPS: Vaddr     = Vaddr(0x0000_4000_0000);
    pub const SHARED_BINARY: Vaddr     = Vaddr(0x0000_8000_0000);
    pub const CROSS_REALM_SHARED: Vaddr = Vaddr(0x0001_0000_0000);
    pub const DEVICE_MAPPINGS: Vaddr   = Vaddr(0x00F0_0000_0000);
}
```

---

## Testing Strategy

The VM runs on seL4 without a standard library (`no_std`), but **most code can be tested on the host** using mocks for platform-specific operations.

### Conditional `std` for Testing

```rust
// src/lib.rs
#![cfg_attr(not(test), no_std)]

#[cfg(test)]
extern crate std;

#[cfg(not(test))]
extern crate alloc;
```

This allows `cargo test` to run with full standard library access while the release build remains `no_std`.

### Test Pyramid

```
┌─────────────────────────────────────────────────────────────────┐
│                        REAL HARDWARE                             │
│  Full system tests, performance benchmarks, stress testing       │
│  Run: Rarely (CI nightly, release validation)                    │
├─────────────────────────────────────────────────────────────────┤
│                      QEMU + seL4 (no_std)                        │
│  Integration tests requiring real seL4: IPC, VSpace, scheduling  │
│  Run: CI on every PR, locally for platform code changes          │
├─────────────────────────────────────────────────────────────────┤
│                      HOST TESTS (std, mocked)                    │
│  Unit tests, property tests, fuzzing                             │
│  GC, interpreter, data structures, pattern matching              │
│  Run: Continuously during development (`cargo test`)             │
│  Target: 80%+ code coverage                                      │
└─────────────────────────────────────────────────────────────────┘
```

### What Can Be Tested on Host

| Component | Host Testable | Notes |
|-----------|---------------|-------|
| GC algorithms | Yes | Mock heap, no real pages |
| Bytecode interpreter | Yes | Pure computation |
| Pattern matching | Yes | Pure computation |
| Chase-Lev deque | Yes | Atomics work on host |
| MPSC mailbox | Yes | Atomics work on host |
| Value encoding/decoding | Yes | Bit manipulation |
| Reduction counting | Yes | Counter logic |
| Address arithmetic | Yes | Integer math |
| Namespace management | Yes | Data structure logic |

### What Requires QEMU/Hardware

| Component | Why | Test Approach |
|-----------|-----|---------------|
| seL4 syscalls | Kernel not present | QEMU + seL4 |
| VSpace mapping | Needs MMU | QEMU + seL4 |
| Real IPC | Needs endpoints | QEMU + seL4 |
| MMIO/DMA | Needs hardware model | QEMU with device |
| IRQ handling | Needs interrupt controller | QEMU |

---

## Platform Abstraction

Platform-specific operations are abstracted behind traits, enabling mock implementations for testing.

### Memory Space Trait

```rust
/// Abstraction over a virtual address space
pub trait MemorySpace {
    /// Read a value from a virtual address
    fn read<T: Copy>(&self, vaddr: Vaddr) -> T;

    /// Write a value to a virtual address
    fn write<T>(&mut self, vaddr: Vaddr, value: T);

    /// Get a byte slice at a virtual address
    fn slice(&self, vaddr: Vaddr, len: usize) -> &[u8];

    /// Get a mutable byte slice at a virtual address
    fn slice_mut(&mut self, vaddr: Vaddr, len: usize) -> &mut [u8];
}
```

### Mock VSpace (for testing)

```rust
/// Mock VSpace backed by a heap-allocated buffer
#[cfg(test)]
pub struct MockVSpace {
    memory: Box<[u8]>,
    base: Vaddr,
}

#[cfg(test)]
impl MockVSpace {
    pub fn new(size: usize, base: Vaddr) -> Self {
        Self {
            memory: vec![0u8; size].into_boxed_slice(),
            base,
        }
    }

    fn offset(&self, vaddr: Vaddr) -> usize {
        let off = vaddr.0.checked_sub(self.base.0)
            .expect("vaddr below base");
        assert!(off < self.memory.len() as u64, "vaddr out of bounds");
        off as usize
    }
}

#[cfg(test)]
impl MemorySpace for MockVSpace {
    fn read<T: Copy>(&self, vaddr: Vaddr) -> T {
        let off = self.offset(vaddr);
        let ptr = self.memory[off..].as_ptr() as *const T;
        unsafe { ptr.read_unaligned() }
    }

    fn write<T>(&mut self, vaddr: Vaddr, value: T) {
        let off = self.offset(vaddr);
        let ptr = self.memory[off..].as_mut_ptr() as *mut T;
        unsafe { ptr.write_unaligned(value) }
    }

    fn slice(&self, vaddr: Vaddr, len: usize) -> &[u8] {
        let off = self.offset(vaddr);
        &self.memory[off..off + len]
    }

    fn slice_mut(&mut self, vaddr: Vaddr, len: usize) -> &mut [u8] {
        let off = self.offset(vaddr);
        &mut self.memory[off..off + len]
    }
}
```

### Real seL4 VSpace

```rust
/// Real VSpace - just interprets addresses directly
#[cfg(not(test))]
pub struct Sel4VSpace;

#[cfg(not(test))]
impl MemorySpace for Sel4VSpace {
    fn read<T: Copy>(&self, vaddr: Vaddr) -> T {
        unsafe { (vaddr.0 as *const T).read() }
    }

    fn write<T>(&mut self, vaddr: Vaddr, value: T) {
        unsafe { (vaddr.0 as *mut T).write(value) }
    }

    fn slice(&self, vaddr: Vaddr, len: usize) -> &[u8] {
        unsafe { core::slice::from_raw_parts(vaddr.0 as *const u8, len) }
    }

    fn slice_mut(&mut self, vaddr: Vaddr, len: usize) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(vaddr.0 as *mut u8, len) }
    }
}
```

### Platform Trait

```rust
/// Platform-specific operations
pub trait Platform {
    type VSpace: MemorySpace;

    /// Get the VSpace for the current realm
    fn vspace(&self) -> &Self::VSpace;
    fn vspace_mut(&mut self) -> &mut Self::VSpace;

    /// Current time in nanoseconds
    fn time_ns(&self) -> u64;

    /// Yield to kernel scheduler
    fn yield_cpu(&self);

    /// Map a page into the VSpace
    fn map_page(&mut self, vaddr: Vaddr, paddr: Paddr, perms: PagePerms)
        -> Result<(), MapError>;

    /// Send IPC message
    fn ipc_send(&self, endpoint: Cap<Endpoint>, msg: &IpcMessage)
        -> Result<(), IpcError>;
}
```

### Example Test Using Mocks

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_heap_allocation() {
        // Create mock VSpace for process heap region
        let mut vspace = MockVSpace::new(
            64 * 1024,  // 64KB
            vspace::PROCESS_HEAPS,
        );

        // Initialize a process
        let mut proc = Process::new(
            Pid::new(0, 1),
            &mut vspace,
            64 * 1024,
        );

        // Allocate on heap (grows down)
        let val1 = proc.heap_alloc::<Value>(&mut vspace);
        let val2 = proc.heap_alloc::<Value>(&mut vspace);

        // Verify heap grows downward
        assert!(val2 < val1);

        // Verify addresses are in expected range
        assert!(val1.0 < vspace::PROCESS_HEAPS.0 + 64 * 1024);
        assert!(val1.0 >= vspace::PROCESS_HEAPS.0);
    }

    #[test]
    fn test_gc_preserves_reachable() {
        let mut vspace = MockVSpace::new(64 * 1024, vspace::PROCESS_HEAPS);
        let mut heap = ProcessHeap::new(&mut vspace, 64 * 1024);
        let mut gc = GarbageCollector::new();

        // Allocate cons cells
        let nil = Value::nil();
        let cell1 = heap.alloc_cons(&mut vspace, Value::int(1), nil);
        let cell2 = heap.alloc_cons(&mut vspace, Value::int(2), cell1);
        let unreachable = heap.alloc_cons(&mut vspace, Value::int(99), nil);

        // GC with cell2 as root
        let roots = [cell2];
        gc.collect(&mut heap, &mut vspace, &roots);

        // cell1 and cell2 should survive (reachable from root)
        // unreachable should be collected
        assert!(heap.is_valid(&vspace, cell2));
        assert!(heap.is_valid(&vspace, cell1));
        // Note: can't directly test unreachable is gone, but heap size should decrease
    }
}
```

---

## Linting and Static Analysis

### Clippy Configuration

Create `clippy.toml` in the project root:

```toml
# Deny common mistakes
msrv = "1.75"
```

Create `.cargo/config.toml`:

```toml
[target.'cfg(all())']
rustflags = [
    "-Dwarnings",              # Treat warnings as errors
    "-Dclippy::all",           # All standard lints
    "-Dclippy::pedantic",      # Pedantic lints
    "-Dclippy::nursery",       # Experimental lints

    # Project-specific denies
    "-Dclippy::unwrap_used",   # Prefer expect() or ?
    "-Dclippy::panic",         # No panics in library code

    # Allows for systems programming
    "-Aclippy::cast_possible_truncation",
    "-Aclippy::cast_sign_loss",
    "-Aclippy::cast_ptr_alignment",
]
```

### Running Lints

```bash
# Standard lint run
cargo clippy --all-targets --all-features

# With extra checks
cargo clippy --all-targets --all-features -- -W clippy::cargo
```

### Formatting

```bash
# Check formatting
cargo fmt --check

# Apply formatting
cargo fmt
```

### Additional Static Analysis

```bash
# Check for undefined behavior in tests (requires nightly)
cargo +nightly miri test

# Security audit of dependencies
cargo audit

# Detect unsafe code statistics
cargo geiger
```

---

## Code Coverage

### Using cargo-llvm-cov

```bash
# Install
cargo install cargo-llvm-cov

# Run tests with coverage
cargo llvm-cov --html

# Open report
open target/llvm-cov/html/index.html
```

### Using cargo-tarpaulin (Linux)

```bash
# Install
cargo install cargo-tarpaulin

# Run with HTML report
cargo tarpaulin --out Html --output-dir target/tarpaulin

# With branch coverage
cargo tarpaulin --out Html --branch
```

### CI Integration

```yaml
# .github/workflows/coverage.yml
name: Coverage

on: [push, pull_request]

jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable
        with:
          components: llvm-tools-preview

      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov

      - name: Generate coverage
        run: cargo llvm-cov --lcov --output-path lcov.info

      - name: Upload to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: lcov.info
```

### Coverage Targets

| Component | Target | Rationale |
|-----------|--------|-----------|
| GC | 90%+ | Critical for correctness |
| Interpreter | 85%+ | Core functionality |
| Data structures | 90%+ | Foundational code |
| Pattern matching | 85%+ | Complex logic |
| Platform abstraction | 70%+ | Some paths are error handling |
| seL4 bindings | 50%+ | Tested via QEMU integration |

---

## Fuzzing

### Setup

```bash
# Install cargo-fuzz (requires nightly)
cargo install cargo-fuzz

# Initialize fuzz targets
cargo fuzz init
```

### Fuzz Targets

```rust
// fuzz/fuzz_targets/gc.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use lona_vm::{MockVSpace, ProcessHeap, GarbageCollector, Value};

fuzz_target!(|operations: Vec<u8>| {
    let mut vspace = MockVSpace::new(256 * 1024, 0x1000_0000.into());
    let mut heap = ProcessHeap::new(&mut vspace, 256 * 1024);
    let mut gc = GarbageCollector::new();
    let mut roots = Vec::new();

    for op in operations {
        match op % 4 {
            0 => {
                // Allocate
                if let Some(val) = heap.try_alloc_int(&mut vspace, op as i64) {
                    if roots.len() < 100 {
                        roots.push(val);
                    }
                }
            }
            1 => {
                // Collect
                gc.collect(&mut heap, &mut vspace, &roots);
            }
            2 => {
                // Drop random root
                if !roots.is_empty() {
                    roots.swap_remove(op as usize % roots.len());
                }
            }
            _ => {
                // Allocate cons
                if roots.len() >= 2 {
                    let a = roots[op as usize % roots.len()];
                    let b = roots[(op as usize + 1) % roots.len()];
                    if let Some(val) = heap.try_alloc_cons(&mut vspace, a, b) {
                        roots.push(val);
                    }
                }
            }
        }
    }
});
```

### Running Fuzz Tests

```bash
# Run GC fuzzer
cargo +nightly fuzz run gc

# Run for specific duration
cargo +nightly fuzz run gc -- -max_total_time=300

# Run with specific corpus
cargo +nightly fuzz run gc fuzz/corpus/gc
```

---

## Cross-Compilation

### Target Setup

```bash
# Install targets
rustup target add x86_64-unknown-none
rustup target add aarch64-unknown-none
```

### Build Configuration

```toml
# .cargo/config.toml

[build]
# Default target for development
# target = "x86_64-unknown-none"

[target.x86_64-unknown-none]
rustflags = ["-C", "link-arg=-nostartfiles"]

[target.aarch64-unknown-none]
rustflags = ["-C", "link-arg=-nostartfiles"]
```

### Building for Targets

```bash
# Build for x86_64
cargo build --target x86_64-unknown-none --release

# Build for aarch64
cargo build --target aarch64-unknown-none --release
```

---

## Development Workflow

### Daily Development

```bash
# Format, lint, test cycle
cargo fmt
cargo clippy --all-targets
cargo test

# With coverage
cargo llvm-cov --html && open target/llvm-cov/html/index.html
```

### Pre-Commit Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/sh
set -e

cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --quiet
```

### CI Pipeline

```yaml
# .github/workflows/ci.yml
name: CI

on: [push, pull_request]

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-action@stable
      - run: cargo fmt --check
      - run: cargo clippy --all-targets -- -D warnings
      - run: cargo test

  cross:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        target:
          - x86_64-unknown-none
          - aarch64-unknown-none
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-action@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --target ${{ matrix.target }}
```

### Release Checklist

1. All tests pass: `cargo test`
2. No lint warnings: `cargo clippy -- -D warnings`
3. Coverage meets targets: `cargo llvm-cov`
4. Builds for all targets: `cargo build --target <target>`
5. Fuzz tests run clean: `cargo +nightly fuzz run <target> -- -max_total_time=60`
6. QEMU integration tests pass (if applicable)

---

## Docker Build Environment

The seL4 kernel and Lona VM are built using Docker to ensure reproducible builds across different development machines. This setup supports both local and remote Docker daemons.

### Prerequisites

- Docker installed locally or access to a remote Docker daemon
- For remote Docker: `export DOCKER_HOST=tcp://<host>:<port>`
- For local QEMU testing: QEMU installed (`qemu-system-x86_64`, etc.)

### Building the Kernel

```bash
# Build for x86_64 (default)
./scripts/build-kernel.sh

# Build for ARM64
./scripts/build-kernel.sh aarch64
```

The build script:
1. Sends the project as Docker build context (respects `.dockerignore`)
2. Builds the Docker image with seL4 + Rust toolchain
3. Runs the build inside the container
4. Extracts artifacts via `docker cp` (works with remote daemons)

### Remote Docker Daemon

When using a remote Docker daemon (e.g., `DOCKER_HOST=tcp://192.168.1.10:2375`):

**How it works:**
- Build context is sent over the network to the remote daemon
- Build executes on the remote machine
- Artifacts are retrieved via `docker cp` (streams over the Docker API)

**Considerations:**
- Large build contexts take time to transfer; `.dockerignore` is critical
- TCP without TLS is unencrypted; use only on trusted networks
- For production, configure TLS: `DOCKER_HOST=tcp://host:2376` with `DOCKER_TLS_VERIFY=1`

**Network bandwidth:** Initial image build sends ~10-50MB of context. Subsequent builds with Docker layer caching are faster.

### Running with QEMU

After building, test the kernel locally:

```bash
# Run x86_64 kernel in QEMU
./scripts/run-qemu.sh x86_64

# Run ARM64 kernel
./scripts/run-qemu.sh aarch64
```

Exit QEMU: `Ctrl-A X`

### File Structure

```
docker/
└── Dockerfile.sel4     # seL4 + Rust build environment

scripts/
├── build-kernel.sh         # Host-side build orchestrator
├── docker-build-kernel.sh  # Runs inside container
└── run-qemu.sh             # Local QEMU runner

build/                  # Output directory (git-ignored)
├── kernel.elf          # Compiled kernel
├── liblona_vm.a        # VM library (if built)
└── build-info.json     # Build metadata
```

### Customizing the Build

Environment variables for `build-kernel.sh`:

| Variable | Default | Description |
|----------|---------|-------------|
| `DOCKER_HOST` | (local) | Docker daemon address |
| `SEL4_CONFIG` | `release` | Build config: `release` or `debug` |

Build arguments for the Dockerfile:

| Argument | Default | Description |
|----------|---------|-------------|
| `SEL4_PLATFORM` | `x86_64-pc99` | Target platform |
| `SEL4_CONFIG` | `release` | Build configuration |
