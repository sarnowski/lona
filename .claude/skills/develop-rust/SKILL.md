---
name: develop-rust
description: MUST be loaded before reading, writing, reviewing, or even thinking about Rust code in Lona. Load this skill FIRST whenever Rust is involved - before exploring files, before planning, before any Rust-related reasoning. Contains development principles (YAGNI, KISS, clean code), project-specific coding guidelines (rust-coding-guidelines.md), and the test-first workflow for implementation.
---

# Develop Rust

**STOP. Before you read any Rust file, write any Rust code, or even think about Rust architecture - complete Part 1.**

---

## Part 1: Principles and Context (ALWAYS MANDATORY)

This part is required for ALL Rust-related work: reviews, conceptual discussions, planning, or implementation.

### 1.1 Read the Rust Guide

**Read [docs/development/rust-coding-guidelines.md](../../../docs/development/rust-coding-guidelines.md) NOW.**

This file contains project-specific conventions you must know:
- Project structure and build infrastructure
- Memory layout conventions (Paddr/Vaddr newtypes, struct layout)
- Testing strategy (host-testable vs seL4-only components)
- Platform abstraction patterns (MemorySpace trait, MockVSpace)

**Do not skip this. Do not assume you know the conventions.**

### 1.2 Architecture Principles

All Rust work in this project follows these principles strictly:

**YAGNI (You Aren't Gonna Need It)**
- Only implement what is explicitly required
- No "future-proofing" abstractions
- No configurability unless requested
- Delete unused code, don't comment it out

**KISS (Keep It Simple, Stupid)**
- Straightforward solutions over clever ones
- Minimize indirection and abstraction layers
- Three similar lines > premature abstraction
- No over-engineering

**Clean Code**
- Functions do one thing
- Names reveal intent
- Small, focused functions
- No dead code

---

## Part 2: Writing Code (Only When Implementing)

Skip this part if you're only reviewing or discussing concepts. Follow this part when writing or modifying Rust code.

### 2.1 Test-First Development (MANDATORY)

**Tests come BEFORE implementation. No exceptions.**

#### The Red-Green Workflow

For every piece of functionality:

```
1. WRITE TEST    → Define expected behavior in a test
2. RUN TEST      → Verify it FAILS (red) - this proves the test is meaningful
3. WRITE CODE    → Implement minimal code to make the test pass
4. RUN TEST      → Verify it PASSES (green)
5. REFACTOR      → Clean up while keeping tests green
```

If step 2 passes (test doesn't fail), your test is wrong - it doesn't actually test the new code.

#### Test Requirements by Change Type

| Change | Required Tests | Where |
|--------|----------------|-------|
| New function | Unit test per code path | `src/module/module_test.rs` |
| New feature | Unit + integration | Unit in `_test.rs`, integration in `tests/` |
| Bug fix | Regression test FIRST | `_test.rs` with `regression_` prefix |
| Refactoring | No new tests needed | Existing tests must stay green |

#### Bug Fix Workflow (STRICT)

When fixing a bug, you MUST:

**Step 1: Write the regression test**

```rust
// src/feature/feature_test.rs

#[test]
fn regression_off_by_one_in_vector_nth() {
    // This test demonstrates the bug behavior
    // Before fix: would return wrong element or panic
    // After fix: returns correct element
    let mut vm = setup();
    let result = vm.eval("(nth {1 2 3} 2)");
    assert_eq!(result, Value::int(3)); // Index 2 should return third element
}
```

**Step 2: Run the test - it MUST FAIL**

```bash
cargo test regression_off_by_one
# Expected output: FAILED
# This proves the test actually catches the bug
```

**Step 3: Implement the fix**

Edit the implementation code to fix the bug.

**Step 4: Run the test - it MUST PASS**

```bash
cargo test regression_off_by_one
# Expected output: PASSED
# This proves the fix works
```

**Step 5: The test stays forever**

The regression test remains in the codebase permanently. It prevents the bug from returning.

#### Where to Put Tests

| Test Type | Location | Include Pattern |
|-----------|----------|-----------------|
| Unit tests | `src/foo/foo_test.rs` | `#[cfg(test)] mod foo_test;` in parent `mod.rs` |
| Integration | `tests/*.rs` | Auto-discovered by cargo |
| Test helpers | `tests/common/` | `mod common;` in test file |

#### Test File Template

```rust
// src/feature/feature_test.rs
//! Tests for <module description>.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::platform::MockVSpace;

/// Common test setup.
fn setup() -> (/* fixtures */) {
    // Create MockVSpace, Process, etc.
}

// --- Happy path tests ---

#[test]
fn feature_basic_usage() {
    let (fixtures) = setup();
    // Test the main success case
}

// --- Edge cases ---

#[test]
fn feature_empty_input() {
    // Test boundary: empty input
}

#[test]
fn feature_maximum_size() {
    // Test boundary: maximum allowed size
}

// --- Error cases ---

#[test]
fn feature_rejects_invalid_input() {
    // Test that invalid input is handled correctly
}

// --- Regression tests ---

#[test]
fn regression_issue_123_description() {
    // Test for specific bug that was fixed
}
```

#### Testability

Use the testability matrix from [rust-coding-guidelines.md](../../../docs/development/rust-coding-guidelines.md):

| Host Testable | Not Host Testable |
|---------------|-------------------|
| GC algorithms | seL4 syscalls |
| Bytecode interpreter | VSpace mapping |
| Pattern matching | Real IPC |
| Value encoding | MMIO/DMA |
| Chase-Lev deque | Hardware interaction |

For host-testable code, use `MockVSpace` to simulate memory operations.

### 2.2 Write the Code

Implement the minimal code to make tests pass. Apply YAGNI/KISS/Clean Code from Part 1.

### 2.3 Verify

Run:

```bash
make verify
```

This runs all checks: format, lint, test, build.

**The task is not complete until `make verify` passes.**

---

## Part 3: Finishing Work (MANDATORY)

After `make verify` passes and your implementation is complete, you **MUST** invoke the `finishing-work` skill before claiming the work is done.

The `finishing-work` skill triggers parallel AI agent reviews and ensures all issues are resolved. **No work is complete without it.**
