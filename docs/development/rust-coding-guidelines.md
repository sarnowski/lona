# Rust Coding Guidelines for Lona

This document defines the coding standards for Rust code in the Lona project. These guidelines focus on bare-metal and kernel-specific concerns—standard Rust style (enforced by `rustfmt` and `clippy`) applies for everything else.

## Tooling

All Rust code must pass quality checks before merging. Use the standard build commands:

```bash
make build    # Verify code quality (runs fmt + clippy) and compile
make image    # Build the complete bootable OS image
make run      # Build and run in QEMU
make clean    # Remove build artifacts for a fresh build
```

Use default `rustfmt` and `clippy` configurations. Do not add project-specific overrides without documenting them in an ADR.

A dedicated testing guide will be added as the project develops.

> **Note**: On macOS, use `gmake` instead of `make` (install with `brew install make`).

## License Header

Every source file begins with this license header:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) <year> Tobias Sarnowski <tobias@sarnowski.cloud>
//
// <filename> - <brief description of what this file contains>
```

## Module Organization

### Directory Structure

```
src/
├── main.rs              # Kernel entry point, minimal code
├── arch/
│   └── aarch64/
│       ├── mod.rs       # Architecture module root
│       ├── boot.rs      # Boot sequence
│       ├── exceptions.rs # Exception handling
│       └── mmu.rs       # Memory management unit
├── kernel/
│   ├── mod.rs
│   ├── process.rs       # Process management
│   ├── scheduler.rs     # Scheduling
│   ├── memory.rs        # Memory allocation
│   └── ipc.rs           # Inter-process communication
├── runtime/
│   ├── mod.rs
│   ├── parser.rs        # Lonala parser
│   ├── compiler.rs      # Lonala compiler
│   └── gc.rs            # Garbage collector
└── drivers/
    └── uart.rs          # UART driver (kernel-mode only)
```

### Module Guidelines

- Keep `mod.rs` files minimal—primarily re-exports and module declarations
- One primary type or concept per file
- Use `pub(crate)` for internal APIs, reserve `pub` for true public interfaces

## No-Std Environment

All kernel and runtime code operates without the standard library:

```rust
#![no_std]
#![no_main]

// Available: core, alloc (with custom allocator)
// Not available: std, threads, filesystem, networking
```

### Essential Attributes

```rust
#![no_std]                    // No standard library
#![no_main]                   // Custom entry point
#![feature(naked_functions)]  // For exception handlers
#![feature(asm_const)]        // Constants in asm! blocks
```

## Unsafe Code

Unsafe code is necessary for OS development but must be carefully controlled.

### Rules for Unsafe

1. **Minimize scope**: Keep `unsafe` blocks as small as possible
2. **Document invariants**: Every `unsafe` block must have a `// SAFETY:` comment
3. **Encapsulate**: Wrap unsafe operations in safe abstractions where possible
4. **Review required**: All new `unsafe` code requires explicit review

### Safety Comments

```rust
// GOOD: Specific safety justification
let value = unsafe {
    // SAFETY: UART_BASE is a valid MMIO address mapped in the page tables
    // during early boot. This register is write-only and has no side effects
    // beyond transmitting the character.
    core::ptr::write_volatile(UART_BASE as *mut u8, byte);
};

// BAD: Vague or missing justification
let value = unsafe {
    // This is safe because we know what we're doing
    core::ptr::write_volatile(UART_BASE as *mut u8, byte);
};
```

### Unsafe Abstractions

Prefer creating safe wrappers around unsafe operations:

```rust
/// MMIO register for 32-bit read/write access.
///
/// # Safety
///
/// The caller must ensure the base address points to valid MMIO space
/// that remains mapped for the lifetime of this struct.
pub struct MmioReg32 {
    addr: *mut u32,
}

impl MmioReg32 {
    /// Creates a new MMIO register wrapper.
    ///
    /// # Safety
    ///
    /// `addr` must be a valid, aligned MMIO address.
    pub const unsafe fn new(addr: usize) -> Self {
        Self { addr: addr as *mut u32 }
    }

    pub fn read(&self) -> u32 {
        // SAFETY: Constructor invariant guarantees valid MMIO address
        unsafe { core::ptr::read_volatile(self.addr) }
    }

    pub fn write(&self, value: u32) {
        // SAFETY: Constructor invariant guarantees valid MMIO address
        unsafe { core::ptr::write_volatile(self.addr, value) }
    }
}
```

## Inline Assembly

Use `asm!()` for hardware operations that cannot be expressed in Rust.

### Guidelines

```rust
// Document what the assembly does and why it's necessary
/// Read the current exception level.
///
/// Returns the current EL (0-3) in the lowest 2 bits.
#[inline]
pub fn current_el() -> u64 {
    let el: u64;
    // SAFETY: Reading CurrentEL is always safe and has no side effects
    unsafe {
        core::arch::asm!(
            "mrs {el}, CurrentEL",
            "lsr {el}, {el}, #2",
            el = out(reg) el,
            options(nomem, nostack, preserves_flags),
        );
    }
    el
}
```

### Assembly Options

Always specify the most restrictive options that apply:

| Option | Use when |
|--------|----------|
| `nomem` | Assembly does not access memory |
| `nostack` | Assembly does not use the stack |
| `preserves_flags` | Assembly does not modify condition flags |
| `pure` | Assembly has no side effects (implies `nomem`) |

### Separate Assembly Files

For complex assembly (exception vectors, context switch), use separate `.S` files:

```rust
// In arch/aarch64/mod.rs
core::arch::global_asm!(include_str!("boot.S"));
core::arch::global_asm!(include_str!("exceptions.S"));
```

These files should follow ARM64 assembly conventions (assembly coding guidelines to be documented as needed).

## Error Handling

Without `std`, error handling requires explicit design.

### Error Types

```rust
/// Kernel error type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Error {
    /// Invalid argument provided
    InvalidArgument = -1,
    /// Out of memory
    OutOfMemory = -2,
    /// Resource is busy
    Busy = -3,
    /// Operation timed out
    Timeout = -4,
    /// Permission denied
    PermissionDenied = -5,
}

pub type Result<T> = core::result::Result<T, Error>;
```

### Panic Handling

The kernel must never panic in normal operation. Use explicit error handling:

```rust
// GOOD: Explicit error handling
pub fn allocate_page() -> Result<*mut u8> {
    let page = try_allocate()?;
    Ok(page)
}

// BAD: Can panic
pub fn allocate_page() -> *mut u8 {
    try_allocate().unwrap()  // Never use unwrap in kernel code
}
```

Define a panic handler that halts or provides diagnostics:

```rust
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Print panic info to debug UART if available
    if let Some(location) = info.location() {
        // ... print location
    }

    // Halt the CPU
    loop {
        unsafe { core::arch::asm!("wfi") };
    }
}
```

## Documentation

### Public API Documentation

All public items must have documentation:

```rust
/// Initializes the memory management subsystem.
///
/// Sets up the page allocator and initial kernel page tables.
/// Must be called exactly once during early boot, after the
/// MMU is disabled.
///
/// # Arguments
///
/// * `memory_map` - Physical memory regions from the bootloader
///
/// # Errors
///
/// Returns `Error::InvalidArgument` if the memory map is empty or
/// contains overlapping regions.
///
/// # Safety
///
/// This function must be called with the MMU disabled and before
/// any other memory allocation functions.
pub unsafe fn init(memory_map: &[MemoryRegion]) -> Result<()> {
    // ...
}
```

### Internal Documentation

Use regular comments for implementation details:

```rust
fn schedule_next() -> Option<ProcessId> {
    // Check the real-time queue first (highest priority)
    if let Some(pid) = self.rt_queue.pop() {
        return Some(pid);
    }

    // Fall back to the normal queue with round-robin
    self.normal_queue.rotate_left(1);
    self.normal_queue.front().copied()
}
```

## Constants and Configuration

### Hardware Constants

```rust
/// Hardware and architecture constants.
pub mod consts {
    /// PL011 UART base address (QEMU virt machine)
    pub const UART0_BASE: usize = 0x0900_0000;

    /// Page size (4 KiB)
    pub const PAGE_SIZE: usize = 4096;
    pub const PAGE_SHIFT: usize = 12;

    /// Kernel virtual address base
    pub const KERNEL_VADDR_BASE: usize = 0xFFFF_0000_0000_0000;
}
```

### Configuration

Use Cargo features for compile-time configuration:

```toml
[features]
default = ["uart-debug"]
uart-debug = []           # Enable UART debug output
smp = []                  # Enable multi-core support
```

```rust
#[cfg(feature = "uart-debug")]
pub fn debug_print(s: &str) {
    // ...
}

#[cfg(not(feature = "uart-debug"))]
pub fn debug_print(_: &str) {}
```

## Memory Safety Patterns

### Volatile Access

Always use volatile operations for MMIO and shared memory:

```rust
use core::ptr::{read_volatile, write_volatile};

// MMIO read
let status = unsafe { read_volatile(STATUS_REG as *const u32) };

// MMIO write
unsafe { write_volatile(CONTROL_REG as *mut u32, value) };
```

### Memory Barriers

Document and use appropriate barriers:

```rust
use core::sync::atomic::{compiler_fence, Ordering};

// Compiler barrier only
compiler_fence(Ordering::SeqCst);

// Full memory barrier (via inline assembly)
unsafe {
    core::arch::asm!("dmb sy", options(nostack, preserves_flags));
}
```

## Testing

Testing is integral to kernel development.

### Testing Requirements

1. **Pure logic must have unit tests**: Bit manipulation, parsing, calculations, data structures
2. **Hardware interaction needs kernel tests**: MMIO, page tables, exception handling
3. **All tests must pass**: `make build` runs quality checks; failures block merging

### Writing Testable Code

Design for testability by separating pure logic from hardware interaction:

```rust
// BAD: Logic mixed with hardware access
pub fn configure_uart(baud: u32) {
    let divisor = 115200 / baud;  // Pure logic
    unsafe { write_volatile(UART_DLH, (divisor >> 8) as u8) };  // Hardware
}

// GOOD: Pure logic extracted for testing
pub fn calculate_divisor(clock: u32, baud: u32) -> u16 {
    (clock / baud) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divisor_calculation() {
        assert_eq!(calculate_divisor(115200, 9600), 12);
        assert_eq!(calculate_divisor(115200, 115200), 1);
    }
}
```

### Test Placement

- **Unit tests**: In `#[cfg(test)] mod tests` within the same file
- **Kernel tests**: In a dedicated test module (to be defined as the project develops)

### Exception: `unwrap()` in Tests

While kernel code prohibits `unwrap()` and `expect()`, test code may use them for clarity:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_allocation() {
        let page = allocate_page().expect("allocation should succeed");
        assert!(!page.is_null());
    }
}
```

## Clippy Allow Directive Policy

**`#[allow(clippy::...)]` and `#[allow(dead_code)]` directives are FORBIDDEN without explicit approval.**

All clippy warnings must be fixed, not suppressed. This ensures the lint configuration in `Cargo.toml` remains meaningful.

### Approval Process

1. Developer must explain WHY the lint cannot be satisfied
2. Developer must document what invariants make the code safe despite the warning
3. Project owner must explicitly approve each exception
4. Each approved exception MUST have a documented justification comment

### Exception Format

When an exception is approved, use this format:

```rust
// LINT-EXCEPTION: clippy::lint_name
// Reason: <why this specific case cannot satisfy the lint>
// Safety: <what invariants ensure correctness despite suppressing>
#[allow(clippy::lint_name)]
fn my_function() { ... }
```

### Preferred Alternatives

Instead of suppressing lints, fix the underlying issue:

| Lint | Fix |
|------|-----|
| `arithmetic_side_effects` | Use `.checked_add()`, `.saturating_sub()`, `.wrapping_mul()` |
| `indexing_slicing` | Use `.get()` or `.get_mut()` with proper error handling |
| `cast_possible_truncation` | Use `TryFrom::try_from()` with error handling |
| `cast_sign_loss` | Use explicit conversion with validation |
| `dead_code` | Remove unused code, or use `#[cfg(feature = "...")]` for staged features |
| `unused_imports` | Remove the unused import |
| `unused_assignments` | Restructure code to avoid the unnecessary assignment |

### Example: Fixing `arithmetic_side_effects`

```rust
// BAD: Suppresses the warning
#[allow(clippy::arithmetic_side_effects)]
fn calculate_offset(base: u64, index: usize) -> u64 {
    base + (index as u64 * 8)
}

// GOOD: Uses checked arithmetic
fn calculate_offset(base: u64, index: usize) -> Option<u64> {
    let offset = (index as u64).checked_mul(8)?;
    base.checked_add(offset)
}
```

### Example: Fixing `indexing_slicing`

```rust
// BAD: Suppresses the warning
#[allow(clippy::indexing_slicing)]
fn get_item(items: &[u8], index: usize) -> u8 {
    items[index]
}

// GOOD: Uses safe accessor
fn get_item(items: &[u8], index: usize) -> Option<u8> {
    items.get(index).copied()
}
```

## Code Quality Checklist

All Rust code meets these requirements:

- [ ] License header with SPDX identifier present
- [ ] Passes `make build` (runs fmt, clippy)
- [ ] All `unsafe` blocks have `// SAFETY:` comments
- [ ] Public items have documentation
- [ ] No `unwrap()` or `expect()` in kernel code paths (tests excepted)
- [ ] No panicking operations in interrupt handlers
- [ ] Memory barriers documented and justified
- [ ] Hardware constants are named, not magic numbers
- [ ] Pure logic has unit tests
- [ ] New functionality has appropriate test coverage
- [ ] **No `#[allow(...)]` directives without documented approval**

## References

- [Lona Goals](../goals.md) — Project vision and core concepts
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [The Rustonomicon](https://doc.rust-lang.org/nomicon/) — Unsafe Rust guide
- [Rust Embedded Book](https://docs.rust-embedded.org/book/)
- [Writing an OS in Rust: Testing](https://os.phil-opp.com/testing/) — Custom test frameworks for bare-metal
- [ARM Architecture Reference Manual](https://developer.arm.com/documentation/ddi0487/latest)
