# Testing Strategy

This document defines the testing strategy for Lona, optimizing for both high code coverage and fast feedback cycles.

## Overview

Testing bare-metal seL4 applications presents unique challenges: standard Rust tests cannot run because the code targets a custom platform (`aarch64-sel4`) that doesn't exist on the development machine. The solution is a **three-tier testing pyramid** that maximizes host-based testing for speed while using QEMU-based tests only where necessary.

### The Testing Pyramid

```
                    ┌─────────────────┐
                    │   QEMU Tests    │  ← Slowest (seconds per test)
                    │  (Integration)  │     System-level validation
                    └────────┬────────┘
                             │
                 ┌───────────┴───────────┐
                 │   On-Target Tests     │  ← Medium (QEMU boot overhead)
                 │  (Kernel components)  │     seL4/hardware interaction
                 └───────────┬───────────┘
                             │
        ┌────────────────────┴────────────────────┐
        │            Host Tests                   │  ← Fastest (milliseconds)
        │   (Pure logic: parser, data structures) │     Standard cargo test
        └─────────────────────────────────────────┘
```

**Key insight**: Code architecture determines testability. The multi-crate workspace structure separates hardware-independent logic (testable on host) from seL4-specific code (requires QEMU).

## Tier 1: Host Tests (Pure Logic)

### What

Standard Rust `#[test]` functions running on the development machine using `cargo test`.

### Scope

- Lonala lexer and parser
- Bytecode compiler and code generation
- Data structures (process queues, capability tables, ring buffers)
- Algorithms (scheduling policies, memory allocation strategies)
- Serialization and deserialization
- State machines and protocol handlers
- Utility functions (bit manipulation, string processing)

### Requirements

Crates eligible for host testing must be:

1. `#![no_std]` - No standard library dependency
2. **NOT** `#![no_main]` - Must have a standard entry point for tests
3. Free of seL4-specific imports - No `sel4` or `sel4-root-task` dependencies

### Example

```rust
// In crates/lonala-parser/src/lib.rs
#![no_std]
extern crate alloc;

use alloc::vec::Vec;

pub struct Token { /* ... */ }

pub fn tokenize(input: &str) -> Result<Vec<Token>, LexError> {
    // Pure logic - no hardware dependencies
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_simple_expression() {
        let tokens = tokenize("(+ 1 2)").unwrap();
        assert_eq!(tokens.len(), 5);
    }

    #[test]
    fn tokenize_nested_list() {
        let tokens = tokenize("(def x (+ 1 2))").unwrap();
        assert_eq!(tokens.len(), 9);
    }
}
```

### Execution

Host tests run as part of `make test`, which executes `cargo test --workspace --exclude lona-runtime` - testing all crates except the seL4-specific runtime.

### Speed Target

**< 5 seconds** for the entire host test suite.

## Tier 2: On-Target Tests (Kernel Components)

### What

Tests that require seL4 primitives or simulated hardware, running inside QEMU with a custom test harness.

### Scope

- seL4 capability operations (creating, copying, revoking)
- IPC mechanisms (endpoint send/receive, notifications)
- Memory mapping and page table operations
- Exception and interrupt handling
- Timer operations and scheduling with real time
- Device driver interactions

### Approach

Use Rust's custom test framework feature to build test binaries that run in QEMU:

```rust
// In crates/lona-runtime/tests/ipc_test.rs
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(lona_test::runner)]
#![reexport_test_harness_main = "test_main"]

use lona_test::{exit_qemu, QemuExitCode};

#[test_case]
fn test_endpoint_send_receive() {
    // Create an endpoint using seL4 syscalls
    // Send a message
    // Verify receipt
    // This runs in actual seL4 on QEMU
}
```

### Test Harness Implementation

The test harness reports results via serial output and exits QEMU with appropriate codes:

```rust
// In crates/lona-test/src/lib.rs
#![no_std]

pub trait Testable {
    fn run(&self);
}

impl<T: Fn()> Testable for T {
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

pub fn runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    exit_qemu(QemuExitCode::Success);
}

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(code: QemuExitCode) -> ! {
    // Implementation depends on QEMU exit device configuration
    // For ARM, typically use semihosting or custom MMIO
}
```

### Execution

On-target tests run as part of `make test`. Each test binary boots QEMU, runs tests, and exits.

### Speed Target

**< 30 seconds** for all on-target tests.

## Tier 3: Integration Tests (Full System)

### What

End-to-end tests validating complete system workflows in QEMU.

### Scope

- Boot sequence completes successfully
- Lonala programs execute and produce correct output
- Process creation, communication, and lifecycle
- Domain isolation and capability enforcement
- Hot-patching scenarios
- Fault tolerance and recovery

### Approach

Each integration test is a separate QEMU instance running a complete scenario. Tests communicate expected outcomes via serial output, which the test harness parses.

```rust
// In tests/integration/boot_test.rs
#![no_std]
#![no_main]

// This becomes the root task for this test scenario
#[root_task]
fn main(bootinfo: &sel4::BootInfoPtr) -> sel4::Result<Never> {
    // Verify boot info is valid
    assert!(bootinfo.untyped_list().len() > 0);

    // Verify memory regions are accessible
    // ...

    serial_println!("TEST_PASSED: boot_test");
    exit_qemu(QemuExitCode::Success);
}
```

### Execution

Integration tests run as part of `make test`.

### Speed Target

**< 5 minutes** for full integration suite.

## Workspace Structure

The multi-crate workspace enables the tiered testing strategy:

```
lona/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── lona-core/                # Pure types and traits (Tier 1)
│   │   ├── Cargo.toml            # no_std, testable on host
│   │   └── src/lib.rs
│   │
│   ├── lonala-parser/            # Lexer and parser (Tier 1)
│   │   ├── Cargo.toml            # no_std, testable on host
│   │   └── src/lib.rs
│   │
│   ├── lonala-compiler/          # Bytecode compiler (Tier 1)
│   │   ├── Cargo.toml            # no_std, testable on host
│   │   └── src/lib.rs
│   │
│   ├── lona-kernel/              # Kernel abstractions (Tier 1 + mocks)
│   │   ├── Cargo.toml            # no_std, partially testable
│   │   └── src/lib.rs            # VM bytecode tests (opcode-level)
│   │
│   ├── lona-spec-tests/          # Language specification tests (Tier 1)
│   │   ├── Cargo.toml            # no_std, testable on host
│   │   └── src/
│   │       ├── lib.rs            # Test infrastructure
│   │       ├── context.rs        # SpecTestContext for evaluating Lonala
│   │       ├── data_types.rs     # Section 3: Data Types
│   │       ├── literals.rs       # Section 4: Literals
│   │       ├── evaluation.rs     # Section 5: Symbols and Evaluation
│   │       ├── special_forms.rs  # Section 6: Special Forms
│   │       ├── operators.rs      # Section 7: Operators
│   │       ├── functions.rs      # Section 8: Functions
│   │       ├── builtins.rs       # Section 9: Built-in Functions
│   │       ├── reader_macros.rs  # Section 10: Reader Macros
│   │       └── macros.rs         # Section 11: Macros
│   │
│   ├── lona-test/                # Test harness for QEMU tests
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   │
│   └── lona-runtime/             # seL4 root task (Tier 2/3 only)
│       ├── Cargo.toml            # no_std + no_main, requires QEMU
│       ├── src/main.rs
│       └── tests/                # On-target tests
│           └── basic.rs
│
└── tests/                        # Integration tests (Tier 3)
    └── integration/
        └── boot_test.rs
```

### Language Specification Tests

The `lona-spec-tests` crate provides end-to-end tests for the Lonala language against its specification (`docs/lonala.md`). These tests:

- **Compile and execute** actual Lonala source code through the full pipeline
- **Test spec compliance** by verifying behavior matches documented semantics
- **Include spec references** in assertion messages: `[Spec X.Y Topic] description`
- **Organize by spec section** with one test file per major section

Test naming convention: `test_<section>_<subsection>_<description>`

Examples:
- `test_3_2_nil_is_falsy`
- `test_6_3_if_no_else_returns_nil`
- `test_7_1_1_addition_mixed_types`

### Crate Dependencies

```
lona-runtime
    ├── lona-kernel
    │   └── lona-core
    ├── lonala-compiler
    │   ├── lonala-parser
    │   │   └── lona-core
    │   └── lona-core
    └── sel4, sel4-root-task (external)
```

Only `lona-runtime` depends on seL4 crates. All other crates are host-testable.

## Mocking Strategy

For code that needs seL4 primitives but should be testable on host, use trait-based abstraction:

```rust
// In crates/lona-kernel/src/memory.rs

/// Trait for page allocation - can be implemented by real or mock allocator
pub trait PageAllocator {
    type Error;
    fn allocate(&mut self) -> Result<PhysAddr, Self::Error>;
    fn deallocate(&mut self, addr: PhysAddr) -> Result<(), Self::Error>;
}

/// Real implementation using seL4 untypeds (in lona-runtime)
#[cfg(not(test))]
pub struct Sel4PageAllocator {
    untypeds: /* seL4 untyped capabilities */,
}

/// Mock for host testing
#[cfg(test)]
pub struct MockPageAllocator {
    allocated: alloc::vec::Vec<u64>,
    next_addr: u64,
}

#[cfg(test)]
impl PageAllocator for MockPageAllocator {
    type Error = AllocError;

    fn allocate(&mut self) -> Result<PhysAddr, Self::Error> {
        let addr = self.next_addr;
        self.next_addr = self.next_addr.checked_add(4096)
            .ok_or(AllocError::OutOfMemory)?;
        self.allocated.push(addr);
        Ok(PhysAddr(addr))
    }

    fn deallocate(&mut self, addr: PhysAddr) -> Result<(), Self::Error> {
        self.allocated.retain(|&a| a != addr.0);
        Ok(())
    }
}
```

This pattern enables testing scheduler logic, process management, and memory algorithms without running on seL4.

## Test Execution Commands

### Primary Targets

```bash
make build    # Create bootable QEMU image
make test     # Full verification: fmt, clippy, unit tests, build, integration tests
make run      # Interactive QEMU session
```

### Development Workflow

```bash
# Full validation (run before committing)
make test
```

### Individual Crate Tests

```bash
# Run specific crate tests (inside Docker shell)
make shell
cargo test -p lonala-parser
cargo test -p lona-kernel

# Run with output visible
cargo test -p lonala-parser -- --nocapture

# Run specific test
cargo test -p lonala-parser test_tokenize_string
```

## Coverage Goals

| Component | Target | Test Tier | Notes |
|-----------|--------|-----------|-------|
| Lonala lexer | 95%+ | Host | Critical for language correctness |
| Lonala parser | 90%+ | Host | High coverage, many edge cases |
| Lonala compiler | 85%+ | Host | Complex but deterministic |
| Data structures | 90%+ | Host | Foundation for everything |
| Scheduler logic | 85%+ | Host + Mock | Use mock allocators |
| Memory management | 70%+ | QEMU | Requires real page tables |
| IPC primitives | 70%+ | QEMU | Requires seL4 syscalls |
| Device drivers | 60%+ | QEMU | Hardware-dependent |
| Boot sequence | Smoke | Integration | Verify it works |
| E2E scenarios | Key paths | Integration | Critical user journeys |

## Writing Testable Code

### Separation of Concerns

Extract pure logic from hardware interaction:

```rust
// BAD: Logic mixed with hardware access
pub fn configure_uart(baud: u32) {
    let divisor = CLOCK_FREQ / baud;  // Pure logic
    unsafe { write_volatile(UART_DLH, (divisor >> 8) as u8) };  // Hardware
    unsafe { write_volatile(UART_DLL, divisor as u8) };
}

// GOOD: Pure logic extracted
pub fn calculate_divisor(clock: u32, baud: u32) -> Option<u16> {
    clock.checked_div(baud).map(|d| d as u16)
}

pub fn configure_uart(baud: u32) {
    let divisor = calculate_divisor(CLOCK_FREQ, baud)
        .expect("baud rate cannot be zero");
    unsafe { write_volatile(UART_DLH, (divisor >> 8) as u8) };
    unsafe { write_volatile(UART_DLL, divisor as u8) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divisor_calculation() {
        assert_eq!(calculate_divisor(115200, 9600), Some(12));
        assert_eq!(calculate_divisor(115200, 115200), Some(1));
        assert_eq!(calculate_divisor(115200, 0), None);
    }
}
```

### Dependency Injection

Use generics and traits to enable mocking:

```rust
// BAD: Hard-coded dependency
pub struct Scheduler {
    allocator: Sel4Allocator,  // Can't test without seL4
}

// GOOD: Injectable dependency
pub struct Scheduler<A: Allocator> {
    allocator: A,
}

impl<A: Allocator> Scheduler<A> {
    pub fn new(allocator: A) -> Self {
        Self { allocator }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockAllocator;
    impl Allocator for MockAllocator { /* ... */ }

    #[test]
    fn scheduler_round_robin() {
        let scheduler = Scheduler::new(MockAllocator);
        // Test scheduling logic without seL4
    }
}
```

## Panic Handling in Tests

### Host Tests

Standard Rust test behavior - panics are caught and reported as failures.

### QEMU Tests

Implement a custom panic handler that reports to serial and exits:

```rust
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("PANIC: {}", info);
    exit_qemu(QemuExitCode::Failed);
}
```

### Assertions

Use standard `assert!`, `assert_eq!`, `assert_ne!` macros. In QEMU tests, these will trigger the panic handler on failure.

## Test Organization Guidelines

1. **One test file per module**: `parser.rs` has tests in `parser.rs` or `parser/tests.rs`
2. **Descriptive test names**: `test_tokenize_unclosed_string_returns_error`
3. **Arrange-Act-Assert pattern**: Clear setup, action, and verification
4. **No test interdependence**: Each test runs in isolation
5. **Fast tests first**: Put quick sanity checks before slow property tests

## References

### Rust OS Testing

- [Testing | Writing an OS in Rust](https://os.phil-opp.com/testing/) - Custom test frameworks for bare-metal
- [Integration Tests | Writing an OS in Rust](https://os.phil-opp.com/integration-tests/) - Separate executable tests

### Embedded Rust Testing

- [Testing an Embedded Application - Ferrous Systems](https://ferrous-systems.com/blog/test-embedded-app/) - Three-tier testing approach
- [Testing a Hardware Abstraction Layer - Ferrous Systems](https://ferrous-systems.com/blog/defmt-test-hal/) - Mocking strategies
- [Testing a Driver Crate - Ferrous Systems](https://ferrous-systems.com/blog/test-driver-crate/) - On-target testing

### seL4 Testing

- [seL4Test | seL4 docs](https://docs.sel4.systems/projects/sel4test/) - Official seL4 test framework
- [rust-sel4 Repository](https://github.com/seL4/rust-sel4) - Rust bindings with test examples

### Tools

- [embedded-hal-mock](https://crates.io/crates/embedded-hal-mock) - Mock HAL traits
- [defmt-test](https://crates.io/crates/defmt-test) - On-target test harness
- [embedded-test](https://crates.io/crates/embedded-test) - Test harness for embedded devices
- [QEMU ARM Virtual Platform | seL4 docs](https://docs.sel4.systems/Hardware/qemu-arm-virt.html) - QEMU configuration
