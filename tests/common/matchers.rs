// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Value matchers for structural assertions in tests.
//!
//! This module provides a [`ValueMatcher`] trait and implementations for
//! verifying the structure of Lonala values in tests.
//!
//! # Example
//!
//! ```ignore
//! let value = vm.read_and_eval("(1 2 3)")?;
//! assert_value_matches(&vm, value, &IsList::of(vec![
//!     Box::new(IsInt(1)),
//!     Box::new(IsInt(2)),
//!     Box::new(IsInt(3)),
//! ]));
//! ```

#![expect(
    dead_code,
    reason = "test infrastructure used via macros in test files"
)]

use super::TestVm;
use lona_vm::Value;

/// A matcher for structural assertions on values.
///
/// Implementations check if a value matches expected criteria,
/// returning an error message if the match fails.
pub trait ValueMatcher {
    /// Check if the value matches this matcher's criteria.
    ///
    /// # Errors
    ///
    /// Returns an error message describing the mismatch if the value doesn't match.
    fn matches(&self, value: Value, vm: &TestVm) -> Result<(), String>;
}

/// Assert that a value matches the expected pattern.
///
/// # Panics
///
/// Panics with a descriptive message if the value doesn't match.
pub fn assert_value_matches(vm: &TestVm, value: Value, matcher: &dyn ValueMatcher) {
    if let Err(msg) = matcher.matches(value, vm) {
        panic!("assertion failed: {msg}\n  actual: {}", vm.print(value));
    }
}

/// Matcher for nil values.
///
/// # Example
///
/// ```ignore
/// assert_value_matches(&vm, value, &IsNil);
/// ```
pub struct IsNil;

impl ValueMatcher for IsNil {
    fn matches(&self, value: Value, _vm: &TestVm) -> Result<(), String> {
        if value.is_nil() {
            Ok(())
        } else {
            Err("expected nil".into())
        }
    }
}

/// Matcher for boolean values.
///
/// # Example
///
/// ```ignore
/// assert_value_matches(&vm, value, &IsBool(true));
/// ```
pub struct IsBool(pub bool);

impl ValueMatcher for IsBool {
    fn matches(&self, value: Value, _vm: &TestVm) -> Result<(), String> {
        match value {
            Value::Bool(b) if b == self.0 => Ok(()),
            Value::Bool(b) => Err(format!("expected {}, got {b}", self.0)),
            _ => Err(format!("expected bool {}, got non-bool", self.0)),
        }
    }
}

/// Matcher for integer values.
///
/// # Example
///
/// ```ignore
/// assert_value_matches(&vm, value, &IsInt(42));
/// ```
pub struct IsInt(pub i64);

impl ValueMatcher for IsInt {
    fn matches(&self, value: Value, _vm: &TestVm) -> Result<(), String> {
        match value {
            Value::Int(n) if n == self.0 => Ok(()),
            Value::Int(n) => Err(format!("expected {}, got {n}", self.0)),
            _ => Err(format!("expected integer {}", self.0)),
        }
    }
}

/// Matcher for string values (exact match).
///
/// # Example
///
/// ```ignore
/// assert_value_matches(&vm, value, &IsString::new("hello"));
/// ```
pub struct IsString(pub String);

impl IsString {
    /// Create a new string matcher.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl ValueMatcher for IsString {
    fn matches(&self, value: Value, vm: &TestVm) -> Result<(), String> {
        match value {
            Value::String(addr) => {
                let s = vm
                    .heap()
                    .read_string(vm.mem(), Value::String(addr))
                    .ok_or_else(|| "failed to read string from heap".to_string())?;
                if s == self.0 {
                    Ok(())
                } else {
                    Err(format!("expected string {:?}, got {:?}", self.0, s))
                }
            }
            _ => Err(format!("expected string {:?}, got non-string", self.0)),
        }
    }
}

/// Matcher for symbol values (exact match).
///
/// # Example
///
/// ```ignore
/// assert_value_matches(&vm, value, &IsSymbol::new("foo"));
/// ```
pub struct IsSymbol(pub String);

impl IsSymbol {
    /// Create a new symbol matcher.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl ValueMatcher for IsSymbol {
    fn matches(&self, value: Value, vm: &TestVm) -> Result<(), String> {
        match value {
            Value::Symbol(addr) => {
                let s = vm
                    .heap()
                    .read_string(vm.mem(), Value::Symbol(addr))
                    .ok_or_else(|| "failed to read symbol from heap".to_string())?;
                if s == self.0 {
                    Ok(())
                } else {
                    Err(format!("expected symbol {}, got {}", self.0, s))
                }
            }
            _ => Err(format!("expected symbol {}", self.0)),
        }
    }
}

/// Matcher for lists with specific elements.
///
/// Verifies that the value is a proper list with exactly the specified elements,
/// each matching its corresponding matcher.
///
/// # Example
///
/// ```ignore
/// assert_value_matches(&vm, value, &IsList::of(vec![
///     Box::new(IsInt(1)),
///     Box::new(IsInt(2)),
/// ]));
/// ```
pub struct IsList(pub Vec<Box<dyn ValueMatcher>>);

impl IsList {
    /// Create a list matcher from element matchers.
    #[must_use]
    pub fn of(matchers: Vec<Box<dyn ValueMatcher>>) -> Self {
        Self(matchers)
    }

    /// Create an empty list matcher (matches nil).
    #[must_use]
    pub fn empty() -> Self {
        Self(vec![])
    }
}

impl ValueMatcher for IsList {
    fn matches(&self, value: Value, vm: &TestVm) -> Result<(), String> {
        // Empty list matcher expects nil
        if self.0.is_empty() {
            return if value.is_nil() {
                Ok(())
            } else {
                Err("expected empty list (nil)".into())
            };
        }

        let mut current = value;
        let mut index = 0;

        for matcher in &self.0 {
            match current {
                Value::Pair(addr) => {
                    let pair = vm
                        .heap()
                        .read_pair(vm.mem(), Value::Pair(addr))
                        .ok_or_else(|| format!("failed to read pair at index {index}"))?;

                    matcher
                        .matches(pair.first, vm)
                        .map_err(|e| format!("at index {index}: {e}"))?;

                    current = pair.rest;
                    index += 1;
                }
                Value::Nil => {
                    return Err(format!(
                        "list too short: expected {} elements, got {index}",
                        self.0.len()
                    ));
                }
                _ => {
                    return Err(format!(
                        "improper list at index {index}: rest is not a pair or nil"
                    ));
                }
            }
        }

        // Verify we've consumed the entire list
        if current.is_nil() {
            Ok(())
        } else {
            Err(format!(
                "list too long: expected {} elements, but more remain",
                self.0.len()
            ))
        }
    }
}

/// Matcher that checks the printed representation of a value.
///
/// # Example
///
/// ```ignore
/// assert_value_matches(&vm, value, &PrintsAs::new("(1 2 3)"));
/// ```
pub struct PrintsAs(pub String);

impl PrintsAs {
    /// Create a new print matcher.
    #[must_use]
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl ValueMatcher for PrintsAs {
    fn matches(&self, value: Value, vm: &TestVm) -> Result<(), String> {
        let printed = vm.print(value);
        if printed == self.0 {
            Ok(())
        } else {
            Err(format!("expected to print {:?}, got {:?}", self.0, printed))
        }
    }
}

// ============================================================================
// Macros for use in test files
//
// These macros are expanded at call sites (in *_test.rs files), so they can
// use unwrap/expect since those files allow such operations.
// ============================================================================

/// Assert that reading and printing produces the expected output.
///
/// # Panics
///
/// Panics if reading fails or the output doesn't match.
#[macro_export]
macro_rules! assert_rep {
    ($vm:expr, $input:expr, $expected:expr) => {
        match $vm.rep($input) {
            Ok(output) => assert_eq!(output, $expected, "input: {:?}", $input),
            Err(e) => panic!("rep failed for {:?}: {}", $input, e),
        }
    };
}

/// Assert that reading and printing a value roundtrips correctly.
///
/// Verifies that `print(read(input)) == input`.
///
/// # Panics
///
/// Panics if reading fails or the output doesn't match the input.
#[macro_export]
macro_rules! assert_roundtrip {
    ($vm:expr, $input:expr) => {
        $crate::assert_rep!($vm, $input, $input)
    };
}

/// Assert that reading a value produces the expected structure and roundtrips.
///
/// This macro:
/// 1. Reads and evaluates the input
/// 2. Verifies the value matches the structural matcher
/// 3. Verifies the printed output equals the input (roundtrip)
/// 4. Returns the value for optional further inspection
///
/// # Panics
///
/// Panics if reading fails, structure doesn't match, or roundtrip fails.
#[macro_export]
macro_rules! assert_reads {
    ($vm:expr, $input:expr, $matcher:expr) => {{
        let value = $vm
            .read_and_eval($input)
            .expect(concat!("failed to read: ", stringify!($input)));
        $crate::common::assert_value_matches(&$vm, value, &$matcher);
        assert_eq!(
            $vm.print(value),
            $input,
            "roundtrip failed for: {:?}",
            $input
        );
        value
    }};
}

/// Assert that reading a value produces the expected structure and output.
///
/// Like `assert_reads!` but allows specifying a different expected output
/// (for cases like quote expansion where input != output).
///
/// # Panics
///
/// Panics if reading fails, structure doesn't match, or output doesn't match.
#[macro_export]
macro_rules! assert_reads_as {
    ($vm:expr, $input:expr, $expected:expr, $matcher:expr) => {{
        let value = $vm
            .read_and_eval($input)
            .expect(concat!("failed to read: ", stringify!($input)));
        $crate::common::assert_value_matches(&$vm, value, &$matcher);
        assert_eq!(
            $vm.print(value),
            $expected,
            "output mismatch for input: {:?}",
            $input
        );
        value
    }};
}

/// Assert that reading produces an error.
///
/// # Panics
///
/// Panics if reading succeeds (expected it to fail).
#[macro_export]
macro_rules! assert_read_error {
    ($vm:expr, $input:expr) => {
        match $vm.read($input) {
            Err(_) => {}
            Ok(v) => panic!(
                "expected read error for {:?}, got {:?}",
                $input,
                v.map(|val| $vm.print(val))
            ),
        }
    };
}

/// Build a list matcher from element matchers.
///
/// # Example
///
/// ```ignore
/// list![]                           // empty list
/// list![IsInt(1), IsInt(2)]         // list of two integers
/// list![IsSymbol::new("foo"), IsInt(42)]  // mixed list
/// ```
#[macro_export]
macro_rules! list {
    () => {
        $crate::common::IsList::empty()
    };
    ($($matcher:expr),+ $(,)?) => {
        $crate::common::IsList::of(vec![$(Box::new($matcher) as Box<dyn $crate::common::ValueMatcher>),+])
    };
}

/// Build a symbol matcher (shorthand for `IsSymbol::new`).
#[macro_export]
macro_rules! sym {
    ($s:expr) => {
        $crate::common::IsSymbol::new($s)
    };
}

/// Build a string matcher (shorthand for `IsString::new`).
#[macro_export]
macro_rules! str {
    ($s:expr) => {
        $crate::common::IsString::new($s)
    };
}
