# Testing Strategy

This document defines how we test Lona for fast feedback and high confidence.

## Goals (Non-Negotiable)

- Prefer fast tests: unit > integration.
- Every feature gets happy-path + edge-case coverage.
- Every bug fix starts with a regression test (test-first).
- Speed budgets:
  - Tier 1 (host) suites complete in **< 5s**.
  - Tier 2/3 (QEMU) suites complete in **< 30s**.

## Test Types

### Current (Rust-based)

| Type | Environment | Purpose | Example |
|------|-------------|---------|---------|
| **Rust unit** | Host | Pure logic, single component | Parser, compiler, data structures, VM opcodes |
| **Rust integration (spec)** | Host | Full VM pipeline (parser → compiler → VM) | Language semantics, built-ins, special forms |
| **Rust integration (seL4)** | QEMU + seL4 | seL4 primitives, boot, memory | Capability operations, IPC, memory mapping |

### Future (Lonala-based)

Once the VM is complete, we will implement a Lonala test framework:

| Type | Environment | Purpose |
|------|-------------|---------|
| **Lonala unit** | QEMU + seL4 | Test Lonala functions written in Lonala |
| **Lonala integration** | QEMU + seL4 | Drivers, process supervision, message passing |

**Rule:** Write the *lowest tier* test that can prove the behavior.

## Running Tests

```bash
make test
```

**This is the single command for ALL quality checks.** It must pass with ZERO issues.

`make test` runs the complete verification suite on **both aarch64 and x86_64**:

| Check | What It Does |
|-------|--------------|
| Formatting | Ensures consistent code style |
| Documentation | Fails on broken doc links |
| Compilation | Builds runtime for target architecture |
| Clippy (host) | All lints on host-testable crates |
| Clippy (target) | All lints on runtime with target flags |
| Unit tests | All `#[test]` functions |
| Integration tests | Full system tests in QEMU |

## 1. Rust Unit Tests (Host)

**Location:** `crates/<crate>/src/**/tests/*.rs` or inline `#[cfg(test)]` modules

**Naming:** `test_<behavior>_<case>()`

```rust
// crates/lonala-parser/src/parser/tests/atom_tests.rs
#[test]
fn parse_integer_literal() {
    let ast = parse_ast("42");
    assert!(matches!(ast, Ast::Integer(n) if n == 42));
}

#[test]
fn parse_nested_list() {
    let asts = parse_asts("(+ 1 (* 2 3))");
    assert_eq!(asts.len(), 1);
    // Verify structure...
}
```

## 2. Rust Integration Tests - Spec (Host)

Spec tests evaluate Lonala expressions through the full pipeline (parser → compiler → VM) to verify language semantics.

**Location:** `crates/lona-spec-tests/src/` (one file per spec section)

**Naming:** `test_<section>_<subsection>_<description>()`

**Assertions:** Use `spec_ref()` to tie tests to the language spec.

```rust
// crates/lona-spec-tests/src/data_types/primitives.rs
use crate::{spec_ref, SpecTestContext};

#[test]
fn test_3_2_nil_is_falsy() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_int(
        "(if nil 1 2)",
        2,
        &spec_ref("3.2", "Nil", "nil is falsy in boolean context"),
    );
}

#[test]
fn test_6_1_def_creates_global() {
    let mut ctx = SpecTestContext::new();
    ctx.eval("(def x 42)").unwrap();
    ctx.assert_int("x", 42, &spec_ref("6.1", "def", "creates global binding"));
}

#[test]
fn test_6_1_def_undefined_errors() {
    let mut ctx = SpecTestContext::new();
    ctx.assert_error(
        "undefined_symbol",
        &spec_ref("6.1", "def", "undefined symbols produce errors"),
    );
}
```

### SpecTestContext Quick Reference

| Method | Use for |
|--------|---------|
| `eval(source) -> Result<Value, String>` | Execute and get result |
| `assert_int(source, expected, spec_ref)` | Integer result |
| `assert_bool(source, expected, spec_ref)` | Boolean result |
| `assert_nil(source, spec_ref)` | Nil result |
| `assert_float(source, expected, spec_ref)` | Float result |
| `assert_ratio(source, numer, denom, spec_ref)` | Ratio result |
| `assert_string(source, expected, spec_ref)` | String result |
| `assert_symbol_eq(source, expected_name, spec_ref)` | Symbol with name |
| `assert_keyword_eq(source, expected_name, spec_ref)` | Keyword with name |
| `assert_list_eq(source, expected_source, spec_ref)` | List equality |
| `assert_vector_eq(source, expected_source, spec_ref)` | Vector equality |
| `assert_map_eq(source, expected_source, spec_ref)` | Map equality |
| `assert_list_len(source, expected_len, spec_ref)` | List length |
| `assert_vector_len(source, expected_len, spec_ref)` | Vector length |
| `assert_map_len(source, expected_len, spec_ref)` | Map length |
| `assert_set_len(source, expected_len, spec_ref)` | Set length |
| `assert_function(source, spec_ref)` | Function type |
| `assert_error(source, spec_ref)` | Expects any error |
| `assert_error_contains(source, contains, spec_ref)` | Error with substring |

## 3. Rust Integration Tests - seL4 (QEMU)

**Location:** `crates/lona-runtime/src/integration_tests.rs`

**Naming:** Return `lona_test::Status::Pass` or `Status::Fail`

```rust
// crates/lona-runtime/src/integration_tests.rs
fn test_boot() -> Status {
    // If we're executing, boot succeeded
    Status::Pass
}

fn test_arithmetic() -> Status {
    let mut interner = Interner::new();
    let chunk = compile("(+ 1 2)", TEST_SOURCE_ID, &mut interner).ok()?;
    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(n)) if n == Integer::from_i64(3) => Status::Pass,
        _ => Status::Fail,
    }
}

// Register in run_integration_tests():
let tests = [
    Test::new("boot", test_boot),
    Test::new("arithmetic", test_arithmetic),
];
```

## Test-First Bug Fix Workflow (Mandatory)

1. **Reproduce** at the lowest tier that can show the bug (prefer host tests).
2. **Write a failing test** (smallest repro; include the triggering edge case).
3. **Verify it fails** with the current (buggy) code.
4. **Fix the bug.**
5. **Verify the test passes.**
6. **Keep the test forever** as a regression guard.

```rust
// Example: Parser was crashing on empty lists
#[test]
fn test_parser_handles_empty_list() {
    // This crashed before the fix
    let ast = parse_ast("()");
    assert!(matches!(ast, Ast::List(items) if items.is_empty()));
}
```

If the bug only manifests in QEMU, still try to extract a host-level test for the pure logic part, then keep the QEMU test as end-to-end regression.

## Coverage Expectations

| Component | Target | Notes |
|-----------|--------|-------|
| Parser, Compiler | 90%+ | Critical for correctness |
| Data structures | 90%+ | Foundation for everything |
| VM interpreter | 85%+ | Complex but deterministic |
| Spec tests | 100% of spec sections | Language contract |
| seL4 integration | 70%+ | Hardware-dependent |
| Drivers | 60%+ | External dependencies |

Every feature adds tests for: happy path + at least one edge case + at least one failure case (when applicable).

## Guidelines

1. **Isolation:** Tests must not depend on order or shared state.
2. **Determinism:** No flaky tests. Seed randomness if needed.
3. **Speed:** Unit tests should be < 10ms each.
4. **Clarity:** Use descriptive assertions; include spec references.
5. **Minimal repro:** Use the smallest expression that proves the behavior.

## Property-Based Testing (proptest)

Use `proptest` for testing invariants with random inputs. Available in `lona-core`.

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn hamt_insert_get_roundtrip(key: u64, value: i32) {
        let map = Map::new().insert(key, value);
        prop_assert_eq!(map.get(&key), Some(&value));
    }

    #[test]
    fn integer_addition_commutative(a: i64, b: i64) {
        let x = Integer::from(a);
        let y = Integer::from(b);
        prop_assert_eq!(&x + &y, &y + &x);
    }
}
```

Property tests run as part of `make test`. Use for:

- Data structure invariants (HAMT, pvec, Integer, Ratio)
- Arithmetic properties (commutativity, associativity)
- Round-trip serialization/parsing
