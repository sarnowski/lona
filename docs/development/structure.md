# Project Structure

This document describes the directory layout and crate organization of the Lona project.

---

## Overview

Lona is built as a Cargo workspace with three crates that produce two binaries:

| Crate | Type | Purpose |
|-------|------|---------|
| `lona-abi` | Library | Shared ABI definitions (IPC, memory layout, types) |
| `lona-memory-manager` | Binary | seL4 root task, manages resources |
| `lona-vm` | Lib + Binary | Lona VM, mapped into every realm |

```
                  ┌─────────────┐
                  │  lona-abi   │
                  │  (no deps)  │
                  └──────┬──────┘
                         │
           ┌─────────────┴─────────────┐
           │                           │
           ▼                           ▼
┌─────────────────────┐    ┌─────────────────────┐
│ lona-memory-manager │    │      lona-vm        │
│                     │    │                     │
│  deps: lona-abi     │    │  deps: lona-abi     │
│        sel4*        │    │        sel4* (opt)  │
│                     │    │        tar-no-std   │
└─────────────────────┘    └─────────────────────┘
```

---

## Directory Layout

```
lona/
├── Cargo.toml                    # Workspace root
├── Makefile                      # Build orchestration
├── mkdocs.yaml                   # Documentation site config
│
├── crates/
│   ├── lona-abi/                 # Shared ABI definitions
│   ├── lona-memory-manager/      # Root task binary
│   └── lona-vm/                  # VM library + binary
│
├── lib/
│   └── lona/                     # Lonala standard library source
│       ├── core.lona             # Bootstrap (loaded first)
│       └── init.lona             # Init process
│
├── docs/                         # Documentation (MkDocs source)
│   ├── index.md
│   ├── architecture/
│   ├── lonala/
│   └── development/
│
├── docker/                       # Build environment
│   └── Dockerfile
│
└── .cargo/
    └── config.toml               # Compiler flags, cross-compilation
```

---

## Crate Details

### lona-abi

**Purpose:** The contract between Lona Memory Manager and Lona VM. Both binaries depend on this crate for shared definitions. This crate has no dependencies and is 100% host-testable.

```
crates/lona-abi/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── types/
    │   ├── mod.rs
    │   ├── address.rs            # Paddr, Vaddr newtypes
    │   ├── ids.rs                # RealmId, ProcessId, WorkerId
    │   └── caps.rs               # Capability slot indices
    ├── layout/
    │   ├── mod.rs
    │   ├── vspace.rs             # VSpace region addresses
    │   ├── regions.rs            # RegionType enum, Permissions
    │   └── constants.rs          # Page sizes, alignment
    ├── ipc/
    │   ├── mod.rs
    │   ├── messages.rs           # IPC message types
    │   ├── fault.rs              # Fault message structures
    │   ├── realm.rs              # Realm create/terminate
    │   └── memory.rs             # Page allocation requests
    ├── policy.rs                 # Resource policy types
    └── boot.rs                   # Boot protocol (entry args)
```

**Key contents:**

| Module | Contents |
|--------|----------|
| `types::address` | `Paddr(u64)`, `Vaddr(u64)` with alignment helpers |
| `types::ids` | `RealmId(u64)`, `ProcessId(u64)`, `WorkerId(u16)` |
| `layout::vspace` | VSpace region base addresses and sizes |
| `ipc::messages` | `enum LmmRequest`, `enum LmmResponse` |
| `ipc::fault` | `FaultInfo`, `FaultType` |
| `policy` | `ResourcePolicy`, `CpuPolicy`, `MemoryPolicy` |
| `boot` | Entry point argument layout, register ABI |

**Testing:** All types are pure data structures. Test layout assertions, serialization, constant validity.

---

### lona-memory-manager

**Purpose:** The seL4 root task. Minimal, auditable, handles all privileged operations.

```
crates/lona-memory-manager/
├── Cargo.toml
└── src/
    ├── main.rs                   # Entry point (#[root_task])
    ├── boot.rs                   # Parse bootinfo, locate modules
    ├── realm/
    │   ├── mod.rs
    │   ├── create.rs             # VSpace/CSpace/TCB creation
    │   ├── lifecycle.rs          # Start/suspend/terminate
    │   └── table.rs              # RealmId → RealmState mapping
    ├── fault/
    │   ├── mod.rs
    │   ├── handler.rs            # Fault dispatch loop
    │   ├── page.rs               # Page fault resolution
    │   ├── rate_limit.rs         # Per-realm rate limiting
    │   └── region.rs             # Region table, permissions
    ├── memory/
    │   ├── mod.rs
    │   ├── untyped.rs            # Untyped pool management
    │   ├── frame.rs              # Frame allocation
    │   └── quota.rs              # Per-realm quotas
    └── ipc/
        ├── mod.rs
        └── dispatch.rs           # Request handling
```

**Dependencies:**
- `lona-abi` (shared types)
- `sel4`, `sel4-root-task` (seL4 bindings)

**Testing:**
- **Host-testable:** Data structures, quota calculations, rate limit logic
- **seL4-only:** Actual fault handling, realm creation (E2E in QEMU)

---

### lona-vm

**Purpose:** The Lona VM that runs in every realm. Library for testing, binary for realm entry.

```
crates/lona-vm/
├── Cargo.toml
├── build.rs                      # Builds lonalib.tar from lib/
└── src/
    ├── lib.rs                    # Library exports
    ├── bin/
    │   └── realm-entry.rs        # Binary entry point
    ├── types/
    │   ├── mod.rs
    │   └── address.rs            # Re-exports from lona-abi
    ├── heap/
    │   ├── mod.rs
    │   └── heap_test.rs
    ├── reader/
    │   ├── mod.rs
    │   ├── lexer.rs
    │   ├── lexer_test.rs
    │   ├── parser.rs
    │   └── parser_test.rs
    ├── value/
    │   ├── mod.rs
    │   ├── mod_test.rs
    │   ├── printer.rs
    │   └── printer_test.rs
    ├── platform/
    │   ├── mod.rs
    │   ├── traits.rs             # MemorySpace trait
    │   ├── mock.rs               # MockVSpace for testing
    │   └── mmio.rs               # MMIO mapping
    ├── uart/
    │   ├── mod.rs
    │   ├── aarch64.rs
    │   ├── x86_64.rs
    │   └── mock.rs
    ├── scheduler/
    │   ├── mod.rs
    │   ├── run_queue.rs
    │   ├── reductions.rs
    │   └── work_steal.rs         # Chase-Lev deque
    ├── gc/
    │   ├── mod.rs
    │   ├── mark.rs
    │   └── sweep.rs
    ├── process/
    │   ├── mod.rs
    │   ├── spawn.rs
    │   ├── mailbox.rs            # MPSC queue
    │   └── links.rs              # Links/monitors
    ├── ipc/
    │   ├── mod.rs
    │   ├── client.rs             # Requests to LMM
    │   └── serialize.rs          # Message serialization
    ├── loader/
    │   ├── mod.rs
    │   └── loader_test.rs
    ├── repl/
    │   ├── mod.rs
    │   └── mod_test.rs
    └── e2e/                       # E2E test framework
        ├── mod.rs
        ├── runner.rs
        └── tests.rs
```

**Dependencies:**
- `lona-abi` (shared types)
- `sel4` (optional, for seL4 target)
- `tar-no-std` (tar archive parsing)

**Testing:**
- **Host-testable (90%+):** reader, value, heap, gc, scheduler, mailbox
- **seL4-only:** Entry point, IPC with LMM
- Uses `MockVSpace` for memory operations

---

## Standard Library

The Lonala standard library lives in `lib/lona/` as source files:

```
lib/
└── lona/
    ├── core.lona                 # Bootstrap (macros, arithmetic)
    └── init.lona                 # Init process entry point
```

**Build process:**

1. `build.rs` creates `lonalib.tar` from `lib/` using USTAR format
2. Cargo embeds via `include_bytes!` into Lona VM ELF
3. At runtime, source is parsed and compiled on demand

See [Library Loading](library-loading.md) for details.

---

## Build Artifacts

The build produces a boot image containing:

```
Boot Image:
├── seL4 Kernel
├── lona-memory-manager (ELF)     # Root task, started by kernel
└── lona-vm (ELF)                 # Mapped into realms
    └── lonalib.tar (embedded)    # Standard library source
```

**Build commands:**

| Command | Description |
|---------|-------------|
| `make verify` | Run all checks (format, lint, test, build) |
| `make build` | Build for seL4 target |
| `make test` | Run host tests only |
| `make qemu` | Run in QEMU emulator |
| `make help` | Show all targets |

---

## Testing Strategy

| Crate | Host Tests | seL4 Tests | Notes |
|-------|------------|------------|-------|
| `lona-abi` | 100% | N/A | Pure data types |
| `lona-memory-manager` | Logic only | E2E | Mock traits for seL4 ops |
| `lona-vm` | 90%+ | E2E | MockVSpace pattern |

### Test File Convention

Unit tests live in dedicated `*_test.rs` files alongside their module:

```
src/heap/
├── mod.rs                        # Implementation
└── heap_test.rs                  # Tests
```

The test file is included conditionally:

```rust
// src/heap/mod.rs
#[cfg(test)]
mod heap_test;
```

### Host vs seL4 Testing

```rust
// Conditional std for testing
#![cfg_attr(not(any(test, feature = "std")), no_std)]

#[cfg(any(test, feature = "std"))]
extern crate std;
```

This allows `cargo test` with standard library while release builds remain `no_std`.

### MockVSpace Pattern

Memory operations are abstracted behind the `MemorySpace` trait:

```rust
pub trait MemorySpace {
    fn read<T: Copy>(&self, vaddr: Vaddr) -> T;
    fn write<T>(&mut self, vaddr: Vaddr, value: T);
    fn slice(&self, vaddr: Vaddr, len: usize) -> &[u8];
    fn slice_mut(&mut self, vaddr: Vaddr, len: usize) -> &mut [u8];
}
```

- `MockVSpace`: Heap-backed implementation for host tests
- `Sel4VSpace`: Direct pointer access for seL4

---

## Configuration Files

| File | Purpose |
|------|---------|
| `Cargo.toml` | Workspace members, shared settings |
| `Makefile` | Build orchestration |
| `.cargo/config.toml` | Compiler flags, lints, cross-compilation |
| `docker/Dockerfile` | Reproducible build environment |
| `mkdocs.yaml` | Documentation site structure |
| `CLAUDE.md` | AI assistant instructions |

---

## Key Design Decisions

### Why Three Crates

1. **`lona-abi` separate:** Changes to IPC protocol require updating both binaries. Shared crate makes this explicit and compiler-enforced.

2. **Library + Binary for `lona-vm`:** Library exports enable `cargo test` on host. Binary entry point is only for seL4.

3. **No seL4 in `lona-abi`:** Keeps it dependency-free and host-testable. seL4 types wrapped at crate boundaries.

### Why Embed Source, Not Bytecode

- **Simplicity:** No separate bytecode compiler toolchain
- **Debuggability:** Source available for error messages
- **Flexibility:** Compile-time optimization based on context
- **Single artifact:** One ELF file to deploy

### Why USTAR Tar Format

- **Compatibility:** Works with `tar-no-std` crate
- **Simplicity:** No compression overhead, direct memory access
- **Portability:** Standard format, easy to inspect
