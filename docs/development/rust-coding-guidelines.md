# Rust Implementation Guide

This document describes Rust conventions for the Lona VM, focusing on testing patterns and project-specific abstractions.

**Related:** [architecture/](../architecture/index.md) (architecture) | [lonala/](../lonala/index.md) (language spec) | [lona.kernel](../lonala/lona.kernel.md) (seL4 primitives)

---

## Table of Contents

1. [Project Structure](#project-structure)
2. [Coding Guidelines](#coding-guidelines)
3. [Memory Layout Conventions](#memory-layout-conventions)
4. [Testing Strategy](#testing-strategy)
5. [Platform Abstraction](#platform-abstraction)

---

## Project Structure

### Build Infrastructure

| File | Purpose |
|------|---------|
| `Makefile` | Build orchestration. Run `make help` for available targets. |
| `Cargo.toml` | Rust dependencies and crate configuration |
| `docker/Dockerfile` | Containerized build environment |
| `.cargo/config.toml` | Compiler flags, lints, cross-compilation settings |

### Verification

All checks (format, lint, test, build) run via a single command:

```bash
make verify
```

This is the canonical command. CI runs it, and local development should too. Do not run individual check commands unless debugging a specific failure.

---

## Coding Guidelines

### License Header

Every source file must begin with a two-line SPDX license header:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski
```

### No Magic Numbers

Never use literal numbers with implicit meaning. Define named constants:

```rust
// Bad
if attempts > 3 { return Err(TooManyRetries); }

// Good
const MAX_RETRY_ATTEMPTS: u32 = 3;
if attempts > MAX_RETRY_ATTEMPTS { return Err(TooManyRetries); }
```

### Lint Suppression

Use `#[expect]` instead of `#[allow]`. This ensures the suppression is removed once the underlying issue is fixed:

```rust
// Bad - silently persists even when no longer needed
#[allow(dead_code)]

// Good - compiler warns when suppression becomes unnecessary
#[expect(dead_code, reason = "used in upcoming scheduler module")]
```

**Any lint suppression requires explicit approval.** Do not add `#[expect(...)]` without prior sign-off.

### File Length

Keep source files under **600 lines**. Longer files indicate too many responsibilities. Split into focused modules:

```
// Too long: src/vm.rs (800+ lines with parsing, evaluation, GC)

// Better: Split by responsibility
src/vm/mod.rs        // Module exports, VM struct
src/vm/parser.rs     // Parsing logic
src/vm/eval.rs       // Evaluation
src/vm/gc.rs         // Garbage collection
```

### Code Documentation

Document the **why**, not the **how**. Code is self-explanatory for *what* it does; comments explain *why* it exists.

**Doc comments (`///`):**
- First line: one-sentence summary (appears in search results)
- Explain purpose, not implementation
- Include `# Panics`, `# Errors`, `# Safety` sections where applicable
- Add examples for non-trivial APIs

**Inline comments (`//`):**
- Explain non-obvious decisions, workarounds, or business logic
- Link to issues/references for copied code or external constraints
- Delete comments that restate the code

```rust
/// Allocates a process heap within the given memory region.
///
/// Returns `None` if the region is too small for the minimum heap size.
///
/// # Panics
///
/// Panics if `base` is not page-aligned.
pub fn alloc_heap(mem: &mut impl MemorySpace, base: Vaddr, size: usize) -> Option<ProcessHeap> {
    // Align down to page boundary - required by seL4 VSpace mapping constraints
    let aligned_size = size & !0xFFF;
    // ...
}
```

---

## Memory Layout Conventions

### Address Type Safety

Physical and virtual addresses use distinct newtypes to prevent mixing at compile time:

```rust
/// Physical address (hardware/DMA visible).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Paddr(pub u64);

/// Virtual address (CPU visible).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct Vaddr(pub u64);

// Mixing these is a compile error:
// fn map_page(vaddr: Vaddr, paddr: Paddr) { ... }
// map_page(physical, virtual)  // ERROR: expected Vaddr, found Paddr
```

### Struct Layout

Use `#[repr(C)]` for structures that cross FFI boundaries or require stable layout:

```rust
/// A lightweight process within a realm.
#[repr(C)]
pub struct Process {
    /// Unique process identifier.
    pub pid: u64,
    /// Current heap allocation pointer (grows down).
    pub heap_ptr: Vaddr,
    /// Current stack pointer (grows up).
    pub stack_ptr: Vaddr,
}

// Compile-time layout verification
const _: () = assert!(core::mem::offset_of!(Process, heap_ptr) == 0x08);
```

### VSpace Layout Constants

Virtual address space regions (see [realm-memory-layout.md](../architecture/realm-memory-layout.md)):

| Region | Base Address | Purpose |
|--------|--------------|---------|
| `NULL_GUARD` | `0x0000_0000_0000` | Trap null pointer dereferences |
| `GLOBAL_CONTROL` | `0x0000_0010_0000` | Realm control structures |
| `SCHEDULER_STATE` | `0x0000_0020_0000` | Per-core scheduler data |
| `NAMESPACE_RO` | `0x0000_0100_0000` | Read-only namespace mappings |
| `NAMESPACE_RW` | `0x0000_0200_0000` | Writable namespace mappings |
| `PROCESS_HEAPS` | `0x0000_4000_0000` | Process heap/stack regions |
| `SHARED_BINARY` | `0x0000_8000_0000` | Reference-counted binaries |

---

## Testing Strategy

The VM runs on seL4 (`no_std`), but most code can be tested on the host using mocks.

### Conditional std for Testing

```rust
// src/lib.rs
#![cfg_attr(not(test), no_std)]

#[cfg(test)]
extern crate std;

#[cfg(not(test))]
extern crate alloc;
```

This allows `cargo test` to run with standard library access while release builds remain `no_std`.

### Test Module Lints

Test code prioritizes clarity over defensive programming. Add these allows at the top of every `_test.rs` file, immediately after the module doc comment:

```rust
// src/heap/heap_test.rs
//! Tests for the heap allocator.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
```

**Why allow `unwrap` and `expect` in tests:**
- Tests should panic on unexpected `None`/`Err` - that's a test failure
- Explicit error handling obscures test intent
- `.unwrap()` on a known-good value is clearer than matching

**Placement:** Always immediately after the `//!` doc comment, before any `use` statements.

### Testability Matrix

| Component | Host Testable | Notes |
|-----------|---------------|-------|
| GC algorithms | Yes | Mock heap, no real pages |
| Bytecode interpreter | Yes | Pure computation |
| Pattern matching | Yes | Pure computation |
| Chase-Lev deque | Yes | Atomics work on host |
| MPSC mailbox | Yes | Atomics work on host |
| Value encoding/decoding | Yes | Bit manipulation |
| seL4 syscalls | No | Requires QEMU + seL4 |
| VSpace mapping | No | Requires MMU |
| Real IPC | No | Requires endpoints |
| MMIO/DMA | No | Requires hardware model |

### Test Categories

**Unit tests** — in dedicated `_test.rs` files alongside the module:

```
src/heap/
├── mod.rs           # Implementation
└── heap_test.rs     # Tests for this module
```

```rust
// src/heap/heap_test.rs
//! Tests for the heap allocator.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;
use crate::platform::MockVSpace;

#[test]
fn heap_grows_downward() {
    let mut mem = MockVSpace::new(4096, Vaddr(0x1000));
    let mut heap = ProcessHeap { base: Vaddr(0x1000), ptr: Vaddr(0x2000) };

    let first = heap.alloc(&mut mem, 64);
    let second = heap.alloc(&mut mem, 64);

    assert!(first.is_some());
    assert!(second.is_some());
    assert!(second < first, "heap should grow downward");
}
```

The test file must be included in the parent module:

```rust
// src/heap/mod.rs
#[cfg(test)]
mod heap_test;
```

**Integration tests** — in `tests/`, verify component interactions:

```rust
// tests/gc_integration.rs
#[test]
fn gc_preserves_reachable_objects() {
    let mut mem = MockVSpace::new(64 * 1024, Vaddr(0x1000_0000));
    let mut heap = ProcessHeap::new(&mut mem, 64 * 1024);
    let mut gc = GarbageCollector::new();

    let Some(cell) = heap.alloc_cons(&mut mem, Value::int(1), Value::nil()) else { return };
    let _ = heap.alloc_cons(&mut mem, Value::int(99), Value::nil()); // garbage

    gc.collect(&mut heap, &mut mem, &[cell]);
    assert!(heap.is_valid(&mem, cell));
}
```

**Fuzz tests** — in `fuzz/fuzz_targets/`, find edge cases:

```rust
// fuzz/fuzz_targets/gc.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|ops: Vec<u8>| {
    let mut mem = MockVSpace::new(64 * 1024, Vaddr(0x1000_0000));
    let mut heap = ProcessHeap::new(&mut mem, 64 * 1024);
    let mut gc = GarbageCollector::new();
    let mut roots = Vec::new();

    for op in ops {
        match op % 3 {
            0 => { if let Some(v) = heap.try_alloc_int(&mut mem, i64::from(op)) { roots.push(v); } }
            1 => { gc.collect(&mut heap, &mut mem, &roots); }
            _ => { roots.pop(); }
        }
    }
});
```

---

## Platform Abstraction

Platform-specific operations are abstracted behind traits, enabling mock implementations for host testing. This module requires `#![allow(unsafe_code)]` since memory access is inherently unsafe.

### MemorySpace Trait

```rust
/// Abstraction over a virtual address space.
pub trait MemorySpace {
    /// Reads a value from a virtual address.
    fn read<T: Copy>(&self, vaddr: Vaddr) -> T;
    /// Writes a value to a virtual address.
    fn write<T>(&mut self, vaddr: Vaddr, value: T);
    /// Returns a byte slice at a virtual address.
    fn slice(&self, vaddr: Vaddr, len: usize) -> &[u8];
    /// Returns a mutable byte slice at a virtual address.
    fn slice_mut(&mut self, vaddr: Vaddr, len: usize) -> &mut [u8];
}
```

### Mock Implementation (for testing)

```rust
/// Mock VSpace backed by heap-allocated memory.
#[cfg(test)]
pub struct MockVSpace {
    memory: Box<[u8]>,
    base: Vaddr,
}

#[cfg(test)]
impl MockVSpace {
    /// Creates a new mock address space.
    pub fn new(size: usize, base: Vaddr) -> Self {
        Self { memory: vec![0u8; size].into_boxed_slice(), base }
    }

    fn offset(&self, vaddr: Vaddr) -> Option<usize> {
        let off = vaddr.0.checked_sub(self.base.0)?;
        if off < self.memory.len() as u64 { Some(off as usize) } else { None }
    }
}

#[cfg(test)]
impl MemorySpace for MockVSpace {
    fn read<T: Copy>(&self, vaddr: Vaddr) -> T {
        let Some(off) = self.offset(vaddr) else { return unsafe { core::mem::zeroed() } };
        // SAFETY: offset is bounds-checked, T is Copy
        unsafe { self.memory[off..].as_ptr().cast::<T>().read_unaligned() }
    }

    fn write<T>(&mut self, vaddr: Vaddr, value: T) {
        let Some(off) = self.offset(vaddr) else { return };
        // SAFETY: offset is bounds-checked
        unsafe { self.memory[off..].as_mut_ptr().cast::<T>().write_unaligned(value); }
    }

    fn slice(&self, vaddr: Vaddr, len: usize) -> &[u8] {
        let Some(off) = self.offset(vaddr) else { return &[] };
        self.memory.get(off..off + len).unwrap_or(&[])
    }

    fn slice_mut(&mut self, vaddr: Vaddr, len: usize) -> &mut [u8] {
        let Some(off) = self.offset(vaddr) else { return &mut [] };
        self.memory.get_mut(off..off + len).unwrap_or(&mut [])
    }
}
```

### Real seL4 Implementation

```rust
/// Real VSpace that interprets addresses directly.
#[cfg(not(test))]
pub struct Sel4VSpace;

#[cfg(not(test))]
impl MemorySpace for Sel4VSpace {
    fn read<T: Copy>(&self, vaddr: Vaddr) -> T {
        // SAFETY: caller ensures vaddr is valid and mapped
        unsafe { (vaddr.0 as *const T).read() }
    }

    fn write<T>(&mut self, vaddr: Vaddr, value: T) {
        // SAFETY: caller ensures vaddr is valid and mapped
        unsafe { (vaddr.0 as *mut T).write(value); }
    }

    fn slice(&self, vaddr: Vaddr, len: usize) -> &[u8] {
        // SAFETY: caller ensures range is valid and mapped
        unsafe { core::slice::from_raw_parts(vaddr.0 as *const u8, len) }
    }

    fn slice_mut(&mut self, vaddr: Vaddr, len: usize) -> &mut [u8] {
        // SAFETY: caller ensures range is valid and mapped
        unsafe { core::slice::from_raw_parts_mut(vaddr.0 as *mut u8, len) }
    }
}
```

This pattern allows the same VM code to run against `MockVSpace` in tests and `Sel4VSpace` on real hardware.
