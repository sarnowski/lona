# Rust Coding Guidelines for Lona

This document defines coding guidelines and best practices for developing Lona's runtime in Rust on seL4. These guidelines complement the testing strategy and focus on patterns that are unusual or especially important for bare-metal seL4 development.

---

## Overview

Developing Rust for seL4 differs from typical Rust applications in several ways:

| Aspect | Normal Rust | seL4 Rust |
|--------|-------------|-----------|
| Standard library | Full `std` | `no_std` + `alloc` |
| Memory allocation | System allocator | Custom `GlobalAlloc` on seL4 untypeds |
| Panic handling | Unwind by default | Abort only, custom handler |
| Error trait | `std::error::Error` | Not available in `core` (as of Rust 1.81, moved to core) |
| Concurrency | OS threads | seL4 TCBs + green threads |
| I/O | File descriptors | Capabilities + MMIO |

---

## Code Organization

### Layered Architecture

Structure code to maximize host-testability by separating hardware-independent logic:

```
┌─────────────────────────────────────────────────────────┐
│  lona-runtime (seL4-specific, QEMU-tested only)         │
│  - Root task entry point                                │
│  - seL4 system calls                                    │
│  - Hardware interaction                                 │
├─────────────────────────────────────────────────────────┤
│  lona-kernel (abstractions, mostly host-testable)       │
│  - Traits for hardware abstraction                      │
│  - Domain/Process logic with mock implementations       │
├─────────────────────────────────────────────────────────┤
│  lonala-compiler, lonala-parser (pure logic)            │
│  - Zero seL4 dependencies                               │
│  - 100% host-testable                                   │
├─────────────────────────────────────────────────────────┤
│  lona-core (foundational types)                         │
│  - Value types, traits, errors                          │
│  - 100% host-testable                                   │
└─────────────────────────────────────────────────────────┘
```

### Crate Design Principles

1. **Minimize seL4 dependencies**: Only `lona-runtime` should depend on `sel4` and `sel4-root-task`
2. **Use traits for hardware abstraction**: Enable mocking in tests
3. **Prefer `core` over `alloc`**: Use `alloc` only when heap is necessary
4. **Feature-gate allocator-dependent code**: Allow crates to be used without allocation

```rust
// In Cargo.toml
[features]
default = ["alloc"]
alloc = []

// In lib.rs
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;
```

---

## `no_std` Patterns

### Available vs Unavailable

| Available in `core` | Available in `alloc` | Not available |
|---------------------|----------------------|---------------|
| `Option`, `Result` | `Vec`, `String` | `std::io` |
| `Iterator` | `Box`, `Rc`, `Arc` | `std::fs` |
| Primitives, slices | `BTreeMap`, `BTreeSet` | `HashMap`, `HashSet`* |
| `core::fmt` | `format!` macro | `std::net` |
| Atomics, SIMD | `Cow`, `ToOwned` | `std::thread` |

*`HashMap`/`HashSet` require random seeds from OS; use `BTreeMap`/`BTreeSet` or provide custom hasher.

### Import Conventions

Use explicit paths from `core` and `alloc`:

```rust
// Good: explicit about no_std
use core::fmt::{self, Display};
use core::result::Result;

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};

// Bad: assumes prelude availability
use std::fmt::Display;
```

### Fallible Allocation

Standard `alloc` assumes infallible allocation. For memory-constrained environments, use fallible APIs:

```rust
// Preferred: fallible allocation
let mut vec = Vec::new();
vec.try_reserve(100)?;

// Or use try_* methods where available
let boxed = Box::try_new(value)?;

// Avoid: panics on OOM
let vec = Vec::with_capacity(100);
let boxed = Box::new(value);
```

---

## Unsafe Code Guidelines

### The SAFETY Comment Convention

Every `unsafe` block must have a preceding `// SAFETY:` comment explaining why the code is sound:

```rust
// SAFETY: `ptr` is valid because:
// 1. It was obtained from `Box::into_raw()` in `new()`
// 2. No other code has access to this pointer
// 3. The pointer is properly aligned for `T`
unsafe {
    Box::from_raw(ptr)
}
```

### Safety Documentation for Functions

Unsafe functions must document their preconditions under a `# Safety` section:

```rust
/// Writes a byte to the UART transmit register.
///
/// # Safety
///
/// - `base_addr` must point to a valid UART MMIO region
/// - The UART must be initialized before calling this function
/// - The caller must have exclusive access to the UART
pub unsafe fn uart_write_byte(base_addr: *mut u8, byte: u8) {
    // SAFETY: Caller guarantees base_addr is valid UART MMIO
    unsafe {
        core::ptr::write_volatile(base_addr.add(TX_OFFSET), byte);
    }
}
```

### Safe Abstraction Pattern

Encapsulate unsafe operations behind safe APIs:

```rust
/// A UART driver with ownership-based safety.
pub struct Uart {
    base: *mut u8,
}

impl Uart {
    /// Creates a new UART driver.
    ///
    /// # Safety
    ///
    /// - `base` must point to valid UART MMIO memory
    /// - Only one `Uart` instance may exist per physical UART
    pub unsafe fn new(base: *mut u8) -> Self {
        Self { base }
    }

    /// Writes a byte (safe because we own the UART).
    pub fn write_byte(&mut self, byte: u8) {
        // SAFETY: Constructor guarantees base is valid, &mut self
        // guarantees exclusive access
        unsafe {
            core::ptr::write_volatile(self.base.add(TX_OFFSET), byte);
        }
    }
}
```

### Minimizing Unsafe Scope

Keep unsafe blocks as small as possible:

```rust
// Bad: large unsafe block
unsafe {
    let ptr = allocate_page();
    let page_num = ptr as usize / PAGE_SIZE;
    let frame = Frame::new(page_num);
    map_frame(frame, vaddr);
    initialize_page(ptr);
}

// Good: minimal unsafe, pure logic outside
let ptr = unsafe { allocate_page() };
let page_num = ptr as usize / PAGE_SIZE;
let frame = Frame::new(page_num);
unsafe { map_frame(frame, vaddr) };
unsafe { initialize_page(ptr) };
```

---

## Error Handling

### Error Types in `no_std`

The `Error` trait is in `core` as of Rust 1.81. For earlier versions or maximum compatibility, define error enums:

```rust
/// Errors that can occur during memory allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocError {
    /// No more untyped memory available
    OutOfMemory,
    /// Requested alignment is invalid
    InvalidAlignment,
    /// Requested size is too large
    SizeTooLarge,
}

impl fmt::Display for AllocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "out of memory"),
            Self::InvalidAlignment => write!(f, "invalid alignment"),
            Self::SizeTooLarge => write!(f, "size too large"),
        }
    }
}
```

### Result-Based APIs

Prefer `Result` over panicking:

```rust
// Good: fallible
pub fn allocate_frame(&mut self) -> Result<Frame, AllocError> {
    self.free_list.pop().ok_or(AllocError::OutOfMemory)
}

// Avoid: panics
pub fn allocate_frame(&mut self) -> Frame {
    self.free_list.pop().expect("out of memory")
}
```

### Error Conversion

Use `From` implementations for error conversion:

```rust
#[derive(Debug)]
pub enum RuntimeError {
    Alloc(AllocError),
    Capability(CapError),
    Parse(ParseError),
}

impl From<AllocError> for RuntimeError {
    fn from(e: AllocError) -> Self {
        Self::Alloc(e)
    }
}

// Now ? works automatically
fn do_something() -> Result<(), RuntimeError> {
    let frame = allocator.allocate_frame()?; // AllocError -> RuntimeError
    Ok(())
}
```

---

## Panic Handling

### Panic Strategy

Use `panic = "abort"` in release builds (already configured in `Cargo.toml`):

```toml
[profile.release]
panic = "abort"
```

### Custom Panic Handler

Implement a panic handler that outputs diagnostics and halts:

```rust
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Output to UART for debugging
    if let Some(location) = info.location() {
        serial_println!(
            "PANIC at {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    }

    if let Some(message) = info.message() {
        serial_println!("  {}", message);
    }

    // Halt the system
    loop {
        core::hint::spin_loop();
    }
}
```

### Avoiding Panics

Minimize panic paths in production code:

```rust
// Avoid: panics on None
let value = map.get(&key).unwrap();

// Better: handle missing values
let value = map.get(&key).ok_or(Error::KeyNotFound)?;

// Avoid: panics on index out of bounds
let item = slice[index];

// Better: bounds-checked access
let item = slice.get(index).ok_or(Error::IndexOutOfBounds)?;
```

---

## Memory Management

### GlobalAlloc Implementation

Implement `GlobalAlloc` for seL4 untyped memory:

```rust
use core::alloc::{GlobalAlloc, Layout};

pub struct Sel4Allocator {
    // Internal state
}

unsafe impl GlobalAlloc for Sel4Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Layout is guaranteed valid by caller
        // Implementation allocates from seL4 untypeds
        self.inner_alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: ptr was allocated by this allocator with this layout
        self.inner_dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOCATOR: Sel4Allocator = Sel4Allocator::new();
```

### Allocation Initialization Order

The allocator must be initialized before any allocation occurs:

```rust
#[no_mangle]
pub extern "C" fn _start(bootinfo: &sel4::BootInfo) -> ! {
    // 1. Initialize allocator FIRST (no allocations before this)
    unsafe {
        ALLOCATOR.init(bootinfo.untyped_list());
    }

    // 2. Now heap allocation is available
    let config = parse_bootinfo(bootinfo);

    // 3. Continue initialization
    main(config)
}
```

### Heapless Alternatives

Consider `heapless` collections for fixed-size data:

```rust
use heapless::Vec;

// Fixed capacity, no heap allocation
let mut buffer: Vec<u8, 64> = Vec::new();
buffer.push(0x42)?; // Returns Err if full
```

---

## Capability Patterns

### Rust Ownership as Capability

Leverage Rust's ownership system to model capability semantics:

```rust
/// A capability to a seL4 endpoint (owned, unforgeable).
pub struct EndpointCap {
    cptr: sel4::CPtr,
}

impl EndpointCap {
    /// Sends a message (requires ownership or mutable borrow).
    pub fn send(&mut self, msg: &Message) -> Result<(), IpcError> {
        // Only holder of capability can send
        unsafe { sel4::sys::seL4_Send(self.cptr, msg.into()) }
    }

    /// Creates a derived capability with reduced rights.
    pub fn mint_read_only(&self) -> Result<EndpointCap, CapError> {
        // Mint new cap with reduced rights
        let new_cptr = mint_capability(self.cptr, Rights::READ_ONLY)?;
        Ok(EndpointCap { cptr: new_cptr })
    }
}

// Capability cannot be copied (no Clone)
// Capability cannot be forged (private field, controlled construction)
// Capability can be moved (transferred)
```

### Capability Delegation

Model capability delegation with move semantics:

```rust
/// Spawns a process in a new domain, transferring capabilities.
pub fn spawn_isolated(
    entry: fn(),
    capabilities: Vec<Box<dyn Capability>>, // Ownership transferred
) -> Result<ProcessId, SpawnError> {
    // Capabilities are moved into the new domain
    // Caller no longer has access
    create_domain_with_caps(entry, capabilities)
}
```

### Read-Only vs Read-Write

Use Rust's borrow system to enforce access levels:

```rust
/// A shared memory region.
pub struct SharedRegion {
    base: *mut u8,
    len: usize,
}

impl SharedRegion {
    /// Read-only access.
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: region is valid for len bytes
        unsafe { core::slice::from_raw_parts(self.base, self.len) }
    }

    /// Read-write access (requires mutable borrow).
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: region is valid, &mut self ensures exclusivity
        unsafe { core::slice::from_raw_parts_mut(self.base, self.len) }
    }
}
```

---

## Concurrency Patterns

### Green Thread State

Design process state for cooperative/preemptive scheduling:

```rust
pub struct Process {
    pid: ProcessId,
    status: ProcessStatus,
    stack: Stack,
    heap: ProcessHeap,
    mailbox: Mailbox,
    reduction_count: u32,
    context: SavedContext,
}

pub enum ProcessStatus {
    Running,
    Ready,
    Waiting(WaitReason),
    Suspended,
    Terminated(ExitReason),
}

pub enum WaitReason {
    Message,
    Timeout { deadline: Instant },
    Join { target: ProcessId },
}
```

### Yield Points

Insert yield points in long-running operations:

```rust
impl Vm {
    pub fn execute(&mut self, process: &mut Process) -> ExecuteResult {
        loop {
            let instruction = self.fetch(process)?;

            // Reduction counting for preemption
            process.reduction_count += instruction.cost();

            if process.reduction_count >= REDUCTION_LIMIT {
                process.reduction_count = 0;
                return ExecuteResult::Yield;
            }

            match self.execute_instruction(instruction, process)? {
                InstrResult::Continue => {}
                InstrResult::Yield => return ExecuteResult::Yield,
                InstrResult::Exit(reason) => return ExecuteResult::Exit(reason),
            }
        }
    }
}
```

### Message Passing

Design mailboxes for BEAM-style messaging:

```rust
pub struct Mailbox {
    messages: VecDeque<Message>,
    save_queue: VecDeque<Message>, // For selective receive
}

impl Mailbox {
    /// Adds a message to the mailbox.
    pub fn deliver(&mut self, msg: Message) {
        self.messages.push_back(msg);
    }

    /// Attempts to receive a message matching the pattern.
    pub fn receive(&mut self, pattern: &Pattern) -> Option<Message> {
        // Check messages in order
        let pos = self.messages.iter().position(|m| pattern.matches(m))?;
        Some(self.messages.remove(pos).unwrap())
    }
}
```

---

## Hardware Abstraction

### Trait-Based Abstraction

Define traits for hardware interfaces to enable mocking:

```rust
/// Serial port interface.
pub trait Serial {
    type Error;

    fn write_byte(&mut self, byte: u8) -> Result<(), Self::Error>;
    fn read_byte(&mut self) -> Result<u8, Self::Error>;
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        for &byte in bytes {
            self.write_byte(byte)?;
        }
        Ok(())
    }
}

// Real implementation
pub struct Pl011Uart { /* ... */ }

impl Serial for Pl011Uart {
    type Error = UartError;

    fn write_byte(&mut self, byte: u8) -> Result<(), Self::Error> {
        // Actual hardware access
    }

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        // Actual hardware access
    }
}

// Mock for testing
#[cfg(test)]
pub struct MockSerial {
    pub written: Vec<u8>,
    pub to_read: VecDeque<u8>,
}

#[cfg(test)]
impl Serial for MockSerial {
    type Error = core::convert::Infallible;

    fn write_byte(&mut self, byte: u8) -> Result<(), Self::Error> {
        self.written.push(byte);
        Ok(())
    }

    fn read_byte(&mut self) -> Result<u8, Self::Error> {
        Ok(self.to_read.pop_front().unwrap_or(0))
    }
}
```

### MMIO Access

Use volatile operations for memory-mapped I/O:

```rust
use core::ptr::{read_volatile, write_volatile};

/// Memory-mapped register access.
pub struct MmioRegion {
    base: *mut u8,
}

impl MmioRegion {
    /// Reads a 32-bit register.
    ///
    /// # Safety
    ///
    /// - `offset` must be within the MMIO region
    /// - The register at `offset` must be readable
    pub unsafe fn read_u32(&self, offset: usize) -> u32 {
        let ptr = self.base.add(offset) as *const u32;
        // SAFETY: Caller guarantees offset is valid
        unsafe { read_volatile(ptr) }
    }

    /// Writes a 32-bit register.
    ///
    /// # Safety
    ///
    /// - `offset` must be within the MMIO region
    /// - The register at `offset` must be writable
    pub unsafe fn write_u32(&mut self, offset: usize, value: u32) {
        let ptr = self.base.add(offset) as *mut u32;
        // SAFETY: Caller guarantees offset is valid
        unsafe { write_volatile(ptr, value) }
    }
}
```

---

## Style and Conventions

### Naming Conventions

Follow standard Rust conventions with adjustments for seL4 concepts:

| Rust Convention | seL4/Lona Mapping |
|-----------------|-------------------|
| `snake_case` functions | `create_domain`, `send_message` |
| `CamelCase` types | `ProcessId`, `CapabilitySlot` |
| `SCREAMING_CASE` constants | `PAGE_SIZE`, `MAX_PROCESSES` |
| Avoid abbreviations | `capability` not `cap` in public APIs |

### File Headers

Every source file must begin with the SPDX license header:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
```

### Documentation Philosophy

Documentation explains **why** code exists, not **what** it does. The code itself should be self-explanatory through descriptive names. This prevents documentation from becoming outdated.

| Do | Don't |
|----|-------|
| Explain the purpose/goal | Rephrase the implementation logic |
| Describe architectural context | Describe step-by-step what happens |
| Keep under 10 lines | Write exhaustive documentation |
| Use descriptive names | Compensate for bad names with docs |

### Documentation Coverage

**Document all functions, types, and constants** — both public and private. The more self-explanatory the item, the shorter the doc can be:

```rust
/// Returns the number of processes in the run queue.
fn len(&self) -> usize {
    self.queue.len()
}

/// Selects the next process for execution.
///
/// Implements fair round-robin scheduling, cycling through processes
/// while respecting priority levels within each cycle.
fn select_next_process(&mut self) -> Option<&mut Process> {
    // ...
}
```

### Crate Documentation

Each crate's `lib.rs` or `main.rs` must have a `//!` module doc explaining the crate's role:

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Memory allocator for the Lona runtime.
//!
//! Provides heap allocation on top of seL4's untyped memory capabilities.
//! Each Lona process gets an independent heap to enable per-process
//! garbage collection without global pauses.
```

### Function Documentation

Document the **purpose**, not the **implementation**:

```rust
// Good: explains why this exists
/// Finds the next runnable process for scheduling.
///
/// Implements fair scheduling by cycling through processes in round-robin
/// order, respecting priority levels within each cycle.
fn select_next_process(&mut self) -> Option<&mut Process> {
    // Implementation is self-explanatory from the code
}

// Bad: restates what the code does (will become outdated)
/// Iterates through the run queue starting at current_index,
/// wrapping around if necessary, and returns the first process
/// with status == Ready, or None if the queue is empty.
fn select_next_process(&mut self) -> Option<&mut Process> {
    // Now docs must be updated whenever implementation changes
}
```

### Type Documentation

For structs and enums, explain their role in the system:

```rust
/// A lightweight execution context within a Domain.
///
/// Inspired by Erlang/BEAM processes: isolated heap, message-based
/// communication, independent garbage collection.
pub struct Process {
    pid: ProcessId,
    status: ProcessStatus,
    // Field names are self-explanatory
}
```

### Comments vs Documentation

| Syntax | Use For |
|--------|---------|
| `//!` | Crate/module-level docs (at top of file) |
| `///` | Item docs (functions, types, constants) |
| `//` | Implementation notes, SAFETY comments |

Regular `//` comments explain tricky implementation details or non-obvious decisions:

```rust
fn allocate_frame(&mut self) -> Result<Frame, AllocError> {
    // Prefer larger untypeds first to reduce fragmentation
    self.untypeds.sort_by_key(|u| core::cmp::Reverse(u.size()));

    // ... rest of implementation
}
```

### Module Organization

Use `mod.rs` style with clear module hierarchies:

```
src/
├── lib.rs              # Crate root, re-exports
├── engine/
│   ├── mod.rs          # Module declarations
│   ├── value.rs        # Value types
│   ├── vm.rs           # Virtual machine
│   └── gc.rs           # Garbage collector
└── platform/
    ├── mod.rs
    ├── sel4.rs         # seL4 bindings
    └── uart.rs         # UART driver
```

---

## Lints and Checks

The workspace `Cargo.toml` already configures comprehensive lints. Key points:

### Required Lints

- `warnings = "deny"` — No warnings in committed code
- All clippy categories at `deny` level except `nursery` and `restriction`
- `unsafe_op_in_unsafe_fn = "warn"` — Explicit unsafe in unsafe functions

### Running Checks

```bash
# Full check suite
make check

# Individual checks
cargo fmt --check
cargo clippy --all-targets
cargo test --workspace --exclude lona-runtime
```

### Suppressing Lints

Use `#[expect(...)]` over `#[allow(...)]` when suppression is necessary:

```rust
// Good: will warn if the lint no longer triggers
#[expect(clippy::too_many_arguments, reason = "seL4 syscall requires these")]
fn sel4_call(a: u64, b: u64, c: u64, d: u64, e: u64, f: u64) { }

// Avoid: silently continues even if unnecessary
#[allow(clippy::too_many_arguments)]
fn sel4_call(a: u64, b: u64, c: u64, d: u64, e: u64, f: u64) { }
```

---

## References

### Official Documentation

- [The Embedded Rust Book](https://docs.rust-embedded.org/book/)
- [no_std chapter](https://docs.rust-embedded.org/book/intro/no-std.html)
- [Linux Kernel Rust Coding Guidelines](https://docs.kernel.org/rust/coding-guidelines.html)
- [seL4 Rust Support](https://docs.sel4.systems/projects/rust/)
- [rust-sel4 Repository](https://github.com/seL4/rust-sel4)

### Embedded Rust Patterns

- [Concurrency Patterns in Embedded Rust](https://ferrous-systems.com/blog/embedded-concurrency-patterns/)
- [Effective Rust - no_std](https://www.lurklurk.org/effective-rust/no-std.html)
- [Heap Allocation | Writing an OS in Rust](https://os.phil-opp.com/heap-allocation/)

### Capability-Based Security

- [Capability-Security Model in Rust](https://softwarepatternslexicon.com/patterns-rust/24/16/)
- [Object-capability model](https://en.wikipedia.org/wiki/Object-capability_model)

### seL4 Integration

- [Strengthen Your seL4 Userspace Code with Rust](https://www.dornerworks.com/blog/strengthen-your-sel4-userspace-code-with-rust/)
- [seL4 Summit 2024 Rust Presentation](https://sel4.systems/Foundation/Summit/2024/slides/rust-support.pdf)
