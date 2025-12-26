# Rust Coding Guidelines for Lona

Coding guidelines for Lona's `no_std` Rust runtime on seL4.

---

## Quick Reference

| Pattern | Correct | Wrong |
|---------|---------|-------|
| Arithmetic | `x.checked_add(1)?` | `x + 1` |
| Indexing | `slice.get(i)?` | `slice[i]` |
| Type conversion | `u16::try_from(x)?` | `x as u16` |
| Widening | `u32::from(byte)` | `byte as u32` |
| Integer literals | `42_u32`, `0_usize` | `42`, `0` |
| Unsafe blocks | One operation per block | Multiple operations |
| Variable names | `index`, `count` | `i`, `n` |
| Module types | `uart::Driver` | `uart::UartDriver` |

---

## File Size Limits

**Target: 500 lines maximum per file.** Files approaching or exceeding this limit must be split.

**Splitting strategies:**

1. **Externalize tests**: Move `#[cfg(test)]` modules to separate `tests/` subdirectories
   ```
   src/compiler/mod.rs (400 lines)
   src/compiler/tests/mod.rs
   src/compiler/tests/expressions.rs
   src/compiler/tests/special_forms.rs
   ```

2. **Extract submodules**: Split logical units into their own files
   ```
   # Before: one large file
   src/value.rs (700 lines)

   # After: module with subfiles
   src/value/mod.rs (100 lines - re-exports)
   src/value/primitives.rs
   src/value/collections.rs
   src/value/display.rs
   ```

3. **Split by concern**: Separate parsing, validation, execution, etc.

**Why this matters:**
- Smaller files are easier to review and understand
- Reduces merge conflicts in collaborative work
- Encourages better modularity and separation of concerns
- Keeps code within reasonable context windows for tooling

---

## Crate Architecture

```
lona-runtime    (seL4-specific, QEMU-tested only)
    ↓
lona-kernel     (abstractions, mostly host-testable)
    ↓
lonala-compiler, lonala-parser  (pure logic, 100% host-testable)
    ↓
lona-core       (foundational types, 100% host-testable)
```

**Principles:**
- Only `lona-runtime` depends on `sel4` crates
- Use traits for hardware abstraction (enables mocking)
- Prefer `core` over `alloc`; feature-gate allocation-dependent code

---

## `no_std` Patterns

### Available Types

| `core` | `alloc` | Unavailable |
|--------|---------|-------------|
| `Option`, `Result`, `Iterator` | `Vec`, `String`, `Box` | `HashMap`*, `std::io` |
| Primitives, slices, atomics | `BTreeMap`, `Rc`, `Arc` | `std::fs`, `std::net` |

*Use `BTreeMap` instead (no random seed required).

### Imports

```rust
use core::fmt::{self, Display};
use core::result::Result;

#[cfg(feature = "alloc")]
use alloc::{string::String, vec::Vec};
```

### Fallible Allocation

```rust
// Preferred: fallible
let mut vec = Vec::new();
vec.try_reserve(100)?;

// Avoid: panics on OOM
let vec = Vec::with_capacity(100);
```

---

## Numeric Safety

The workspace enforces `clippy::arithmetic_side_effects`, `clippy::indexing_slicing`, `clippy::as_conversions`, and `clippy::default_numeric_fallback`.

### Arithmetic

Never use raw `+`, `-`, `*`, `/`, `%`, `<<`, `>>`:

```rust
// Use checked_* when overflow is an error
let size = count.checked_mul(FRAME_SIZE).ok_or(Error::SizeOverflow)?;

// Use saturating_* for counters that shouldn't wrap
self.retry_count = self.retry_count.saturating_add(1);

// Use wrapping_* when wrap semantics are intentional
self.sequence = self.sequence.wrapping_add(1);
```

### Integer Literals

Always add explicit type suffixes:

```rust
for _ in 0_u32..4_u32 { }
let count: usize = 0;
```

### Type Conversions

Never use `as` for numeric conversions:

```rust
// Fallible narrowing
let small = u8::try_from(large_value)?;

// Infallible widening
let wider = u32::from(byte_value);
```

For unavoidable pointer casts (MMIO, allocators), use local `#[expect]`:

```rust
#[expect(clippy::as_conversions, reason = "[approved] MMIO base address")]
let base = region.starting_address as usize;
```

### Indexing

Never use `[]` indexing:

```rust
let item = slice.get(index).ok_or(Error::OutOfBounds)?;
let range = data.get(start..end).ok_or(Error::InvalidRange)?;
```

---

## Unsafe Code

### SAFETY Comments

Every `unsafe` block requires a preceding `// SAFETY:` comment:

```rust
// SAFETY: ptr is valid from Box::into_raw(), aligned, and exclusively owned
unsafe { Box::from_raw(ptr) }
```

### One Operation Per Block

Each `unsafe` block contains **exactly one** unsafe operation:

```rust
// SAFETY: allocator is initialized
let ptr = unsafe { allocate_page() };

let frame = Frame::new(ptr);

// SAFETY: frame is valid, vaddr is unmapped
unsafe { map_frame(frame, vaddr) };
```

### Safe Abstractions

Encapsulate unsafe behind safe APIs:

```rust
pub struct Uart {
    base: *mut u32,
}

impl Uart {
    /// # Safety
    /// - `base` must point to valid UART MMIO
    /// - Only one instance per physical UART
    pub unsafe fn new(base: *mut u32) -> Self {
        Self { base }
    }

    /// Safe because constructor guarantees validity, `&mut self` guarantees exclusivity.
    pub fn write_byte(&mut self, byte: u8) {
        // SAFETY: constructor guarantees base is valid MMIO
        unsafe {
            core::ptr::write_volatile(self.base, u32::from(byte));
        }
    }
}
```

### Manual Send/Sync

Use `#[expect]` with full safety documentation:

```rust
// SAFETY: Single-threaded seL4 root task - no concurrent access.
#[expect(clippy::non_send_fields_in_send_ty, reason = "[approved] single-threaded root task")]
unsafe impl Send for PageProvider {}
unsafe impl Sync for PageProvider {}
```

### Typestate for Initialization

Prevent "used before init" at compile time:

```rust
pub struct Uart<State> {
    base: *mut u32,
    _state: PhantomData<State>,
}

pub struct Uninit;
pub struct Ready;

impl Uart<Uninit> {
    pub unsafe fn new(base: *mut u32) -> Self { /* ... */ }
    pub fn init(self) -> Uart<Ready> { /* ... */ }
}

impl Uart<Ready> {
    pub fn write(&mut self, byte: u8) { /* ... */ }
}
```

---

## Memory Ordering & Atomics

### Volatile vs Atomic

**Volatile** (`read_volatile`/`write_volatile`): Prevents compiler reordering, required for MMIO.

**Atomic** (`AtomicU32`, etc.): Prevents CPU reordering, provides synchronization between threads.

Volatile is **not** synchronization. For concurrent MMIO access, enforce exclusion via `&mut self` or locks.

### Barriers

```rust
use core::sync::atomic::{compiler_fence, fence, Ordering};

// Compiler fence: prevents compiler reordering only
compiler_fence(Ordering::SeqCst);

// Memory fence: prevents CPU reordering (hardware barrier)
fence(Ordering::SeqCst);

// Architecture-specific (AArch64)
// Use after DMA setup, before device doorbell
core::arch::asm!("dsb sy");
core::arch::asm!("isb");
```

### Interrupt Safety

`RefCell` is **not** interrupt-safe. For data shared with interrupt handlers:

```rust
// Good: Cell for Copy types
static COUNTER: Cell<u32> = Cell::new(0);

// Good: Atomics
static FLAGS: AtomicU32 = AtomicU32::new(0);

// Bad: RefCell panics if ISR borrows while main holds borrow
static STATE: RefCell<...>  // DON'T use with interrupts
```

---

## Error Handling

### Kind + Error Pattern

User-facing errors use structured data, not strings:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Kind {
    UndefinedSymbol {
        symbol: symbol::Id,
        suggestion: Option<symbol::Id>,
    },
    TypeError {
        operation: &'static str,
        expected: TypeExpectation,
        got: value::Kind,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Error {
    pub kind: Kind,
    pub location: SourceLocation,
}

// NOTE: No Display impl - formatting is in lonala-human crate
```

### Result-Based APIs

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

### Error Parameter Naming

Never use bare `|_|` in `map_err`:

```rust
// Bad
.map_err(|_| MyError::Failed)

// Good
.map_err(|_err| MyError::Failed)

// Better
.map_err(|err| MyError::Failed { cause: err })
```

---

## Style & Naming

### File Headers

```rust
// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
```

### Crate Documentation

```rust
//! Memory allocator for the Lona runtime.
//!
//! Provides heap allocation on top of seL4's untyped memory capabilities.
```

### Naming Conventions

| Convention | Example |
|------------|---------|
| Don't repeat module name | `uart::Driver` not `uart::UartDriver` |
| Descriptive identifiers | `index`, `count` not `i`, `n` |
| No underscore prefix on used items | `print_fmt` not `_print` |

### Variable Shadowing

Avoid shadowing (both `shadow_reuse` and `shadow_same` are denied):

```rust
// Bad
let uart = Uart::new(base);
let uart = uart.init()?;

// Good
let uart = Uart::new(base);
let ready_uart = uart.init()?;
```

### Documentation

Document **why**, not **what**:

```rust
// Good: explains purpose
/// Finds the next runnable process for fair scheduling.

// Bad: restates implementation
/// Iterates through the run queue starting at current_index...
```

Wrap code references in backticks:

```rust
/// Calls `process_message` to handle the `Message`.
```

---

## Testing

### Requirements

- Every feature: happy path + edge case + failure case
- Every bug fix: failing test **first**, then fix
- Coverage targets: 90%+ (parser, compiler, data structures), 85%+ (VM)
- Speed: host tests < 5s, QEMU tests < 30s

### Test Types

| Type | Location | Purpose |
|------|----------|---------|
| Unit | `crates/<crate>/src/**/tests/*.rs` | Single component |
| Spec | `crates/lona-spec-tests/` | Full VM pipeline |
| Integration | `crates/lona-runtime/src/integration_tests.rs` | seL4 primitives |

### Naming

```rust
#[test]
fn test_<behavior>_<case>() { }

#[test]
fn parse_integer_literal() { }

#[test]
fn allocate_returns_error_on_exhaustion() { }
```

### Running Tests

```bash
make test
```

**This is the single command for ALL quality checks.** It must pass with ZERO issues.

`make test` runs the complete verification suite on **both aarch64 and x86_64**:

| Check | Description |
|-------|-------------|
| **Formatting** | `cargo fmt` - consistent code style |
| **Documentation** | Broken doc links fail the build |
| **Compilation** | Builds runtime for target architecture |
| **Clippy (host)** | All lints on host-testable crates |
| **Clippy (target)** | All lints on runtime with target flags |
| **Unit tests** | All `#[test]` functions on host |
| **Integration tests** | Full system tests in QEMU |

Every check must pass. Any failure blocks the build.

### Property-Based Testing

Use `proptest` for data structure invariants (available in `lona-core`):

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn integer_add_commutative(a: i64, b: i64) {
        let x = Integer::from(a);
        let y = Integer::from(b);
        prop_assert_eq!(&x + &y, &y + &x);
    }
}
```

Property tests run as part of `make test`.

---

## Clippy Configuration

The workspace uses **all clippy categories at deny level**, including `restriction` and `nursery`.

### Key Enforced Lints

| Lint | Enforcement |
|------|-------------|
| `arithmetic_side_effects` | Use `checked_*`, `saturating_*`, `wrapping_*` |
| `indexing_slicing` | Use `.get()` instead of `[]` |
| `as_conversions` | Use `try_from()`, `from()` |
| `default_numeric_fallback` | Add type suffixes to literals |
| `undocumented_unsafe_blocks` | Require `// SAFETY:` |
| `multiple_unsafe_ops_per_block` | One unsafe op per block |
| `module_name_repetitions` | Don't repeat module in names |
| `shadow_reuse`, `shadow_same` | Avoid variable shadowing |
| `min_ident_chars` | Use descriptive names |
| `expect_used`, `unwrap_used` | Use `let ... else` or `?` |
| `wildcard_enum_match_arm` | List all variants for `#[non_exhaustive]` enums |
| `exhaustive_structs`, `exhaustive_enums` | Add `#[non_exhaustive]` to public types |
| `missing_inline_in_public_items` | Add `#[inline]` to public functions |
| `allow_attributes` | Use `#[expect]` instead of `#[allow]` |

### Required Patterns

**No `expect()` or `unwrap()`** - Use pattern matching or `?`:

```rust
// Bad
let elem = elements.get(idx).expect("in bounds");

// Good
let Some(elem) = elements.get(idx) else {
    return Err(Error::OutOfBounds);
};
```

**Matching `#[non_exhaustive]` enums** - List all known variants plus wildcard:

```rust
// Bad - clippy::wildcard_enum_match_arm fires
match ast.node {
    Ast::Symbol(ref name) => { /* ... */ }
    _ => { /* ... */ }
}

// Good - explicit variants plus wildcard for future variants
match ast.node {
    Ast::Symbol(ref name) => { /* ... */ }
    // Other node types handled uniformly (wildcard covers future variants)
    Ast::Integer(_)
    | Ast::Float(_)
    | Ast::String(_)
    | Ast::Bool(_)
    | Ast::Nil
    | Ast::Keyword(_)
    | Ast::List(_)
    | Ast::Vector(_)
    | Ast::Map(_)
    | Ast::Set(_)
    | Ast::WithMeta { .. }
    | _ => { /* ... */ }
}
```

This pattern satisfies both constraints:
- `clippy::wildcard_enum_match_arm` requires explicit handling of known variants
- `#[non_exhaustive]` requires a wildcard `_` for compiler exhaustiveness

**Before modifying existing match patterns, check how other code in the same file or module handles the same enum type.**

**Public types require `#[non_exhaustive]`**:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Binding {
    Symbol(symbol::Id),
    Ignore,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Pattern {
    pub items: Vec<Binding>,
}
```

**Public functions require `#[inline]`**:

```rust
#[inline]
pub fn parse_pattern(ast: &Ast) -> Result<Pattern, Error> {
    // ...
}
```

**Doc code blocks** - Use ` ```text ` for non-Rust examples:

```rust
/// # Examples
///
/// ```text
/// [a b c]     -> 3 symbol bindings
/// [a & rest]  -> rest binding
/// ```
```

### Suppressing Lints

**CRITICAL: You MUST NOT suppress any clippy lint without EXPLICIT user approval.**

When encountering a clippy error:

1. **Always fix the issue correctly first** - Most lints have proper solutions
2. **If truly unfixable**, explain the issue and wait for explicit approval
3. **Never add `#[allow]`, `#[expect]`, or `clippy.toml` overrides without approval**

Only after receiving explicit approval, use `#[expect]` with `[approved]` marker:

```rust
#[expect(clippy::as_conversions, reason = "[approved] MMIO pointer conversion")]
let base = addr as *mut u32;
```

The `[approved]` marker is enforced by a pre-commit hook. Without it, the commit will be rejected.

Remove `#[expect]` when the lint no longer triggers.

---

## Panic Handling

### Configuration

```toml
[profile.release]
panic = "abort"
```

### Custom Handler

```rust
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        serial_println!("PANIC at {}:{}:{}", location.file(), location.line(), location.column());
    }
    loop { core::hint::spin_loop(); }
}
```

### Avoiding Panics

```rust
// Avoid
let value = map.get(&key).unwrap();

// Prefer
let value = map.get(&key).ok_or(Error::KeyNotFound)?;
```

---

## Stack Discipline

In kernel code, stack overflow is often silent corruption.

**Rules:**
- Avoid recursion in runtime paths; use iterative algorithms
- No large stack buffers; use heap or fixed-size `heapless` collections
- The VM uses explicit `Vec<Frame>` call stack, not Rust recursion

---

## Binary Data

Never use `transmute` or pointer casts for parsing binary data:

```rust
// Bad: undefined behavior, endianness issues
let value: u32 = unsafe { *(data.as_ptr() as *const u32) };

// Good: explicit endianness, alignment-safe
let bytes = data.get(0..4).ok_or(Error::TooShort)?;
let value = u32::from_le_bytes(bytes.try_into().unwrap());
```

---

## References

- [The Embedded Rust Book](https://docs.rust-embedded.org/book/)
- [Writing an OS in Rust](https://os.phil-opp.com/)
- [seL4 Rust Support](https://docs.sel4.systems/projects/rust/)
- [Linux Kernel Rust Guidelines](https://docs.kernel.org/rust/coding-guidelines.html)
