---
name: develop-rust
description: Develops Rust code following project standards. ALWAYS use this skill before writing, editing, or creating any Rust code (.rs files), implementing kernel modules, writing drivers, or any bare-metal Rust. Also use when the user asks to write Rust, implement kernel functionality, or work on any Rust component of Lona.
---

# Develop Rust Code

This skill ensures all Rust code follows the project's coding standards and no_std requirements.

## Before Writing Any Rust Code

**MANDATORY**: Read the coding guidelines before writing or editing Rust:

```
docs/development/rust-coding-guidelines.md
```

This document defines all standards for no_std development, unsafe code, documentation, and more.

## Workflow

### 1. Read the Guidelines

Read `docs/development/rust-coding-guidelines.md` in full. Do not proceed without understanding the standards.

### 2. Understand the Context

Before writing code:
- Identify the module structure and where new code belongs
- Check existing patterns in the codebase for consistency
- Determine if the code requires `unsafe` and plan safety invariants
- Identify dependencies on `core` or `alloc` features

### 3. Plan the Implementation

For each new file or module:
- Define the public API surface (`pub` vs `pub(crate)`)
- Plan error types and how they integrate with existing `Error` enum
- Design safe abstractions around any required unsafe operations
- **Plan tests**: Identify pure logic that can be unit tested on the host

### 3a. Design for Testability

Extract pure logic from hardware interaction so it can be tested:
- Separate calculations, parsing, and data structure operations from MMIO
- Pure functions should be testable with `cargo test` on the host
- Hardware-dependent code needs kernel tests (when the test infrastructure is available)

### 4. Write the Code

Follow this order when creating a new file:

1. **License header** - SPDX identifier and copyright
2. **Module attributes** - `#![no_std]`, features, lints
3. **Imports** - `use` statements, grouped logically
4. **Constants** - Named constants, no magic numbers
5. **Types** - Structs, enums, type aliases
6. **Implementations** - impl blocks with documentation
7. **Functions** - Standalone functions
8. **Tests** - `#[cfg(test)] mod tests` with unit tests for pure logic

### 5. Verify Compliance

Before presenting code to the user, verify:

- [ ] License header with SPDX identifier present
- [ ] All public items have documentation comments
- [ ] All `unsafe` blocks have `// SAFETY:` comments
- [ ] No `unwrap()`, `expect()`, or panicking operations in kernel paths (tests excepted)
- [ ] No magic numbers - all values are named constants
- [ ] Error handling uses `Result<T, Error>` pattern
- [ ] Assembly options are as restrictive as possible (`nomem`, `nostack`, etc.)
- [ ] Pure logic has unit tests in `#[cfg(test)] mod tests`
- [ ] Tests cover edge cases and error conditions

### 6. Build, Test, and Lint

After writing Rust code, run the standard build commands:

```bash
make build    # Verify code quality (runs fmt + clippy) and compile
make image    # Build the complete bootable OS image
make run      # Build and run in QEMU
make clean    # Remove build artifacts for a fresh build
```

Fix any issues before presenting the code. **All quality checks must pass.**

> **Note**: On macOS, use `gmake` instead of `make`.

## Quick Reference

### License Header
```rust
// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) <year> Tobias Sarnowski <tobias@sarnowski.cloud>
//
// <filename> - <brief description>
```

### Safety Comment
```rust
unsafe {
    // SAFETY: <specific justification explaining why this is sound>
    operation();
}
```

### Error Handling
```rust
// Return Result, never panic
pub fn operation() -> Result<T> {
    let value = try_something()?;
    Ok(value)
}
```

### Unit Test
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        assert_eq!(my_function(input), expected);
    }
}
```

## Common Mistakes to Avoid

1. **Missing license header** - Every file starts with the SPDX header
2. **Vague safety comments** - Be specific about invariants, not "this is safe"
3. **Using `unwrap()`** - Use `?` operator or explicit error handling (tests excepted)
4. **Missing documentation** - All public items need doc comments
5. **Overly broad unsafe** - Keep unsafe blocks minimal
6. **Missing assembly options** - Always specify `nomem`, `nostack`, etc. when applicable
7. **Magic numbers** - Use named constants in a `consts` module
8. **Missing tests** - Pure logic should have unit tests
9. **Untestable code** - Separate pure logic from hardware for testability
10. **Using `#[allow(...)]` to suppress clippy warnings** - Fix the issue instead (see below)

## Clippy Allow Directive Policy

**CRITICAL: `#[allow(clippy::...)]` directives are FORBIDDEN without explicit user approval.**

When clippy reports a warning, you MUST fix it, not suppress it. This policy exists because suppressing warnings defeats the purpose of strict lint configuration.

### What to do instead of `#[allow(...)]`:

| Lint | Fix |
|------|-----|
| `arithmetic_side_effects` | Use `.checked_add()`, `.saturating_sub()`, `.wrapping_mul()` |
| `indexing_slicing` | Use `.get()` or `.get_mut()` with proper error handling |
| `cast_possible_truncation` | Use `TryFrom::try_from()` with error handling |
| `dead_code` | Remove unused code, or use `#[cfg(feature = "...")]` |
| `unused_imports` | Remove the unused import |
| `unused_assignments` | Restructure code to avoid the assignment |

### If suppression is truly unavoidable:

1. **Stop and ask the user** - Explain why you cannot satisfy the lint
2. **Get explicit approval** - The user must approve each exception
3. **Document thoroughly** - Use this format:

```rust
// LINT-EXCEPTION: clippy::lint_name
// Reason: <why this specific case cannot satisfy the lint>
// Safety: <what invariants ensure correctness>
#[allow(clippy::lint_name)]
```

**NEVER add `#[allow(...)]` without user approval.** This is a hard requirement.

## Existing Code Reference

Review existing Rust files (if they exist) for examples of correct style:
- `src/main.rs` - Kernel entry point
- `src/arch/aarch64/` - Architecture-specific code
- `src/arch/aarch64/exceptions.rs` - Example of unit tests for pure logic

**Note**: This project is in early development. The `src/` directory may not exist yet. When it does, use existing code as style references.

## Related Documentation

- `docs/goals.md` - Project vision and core concepts
- `docs/development/rust-coding-guidelines.md` - Full coding standards
- `docs/development/adr.md` - Architecture Decision Records index
