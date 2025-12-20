// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Native function support for the Lonala virtual machine.
//!
//! Provides infrastructure for registering and calling Rust functions
//! from Lonala code. Native functions are used for primitives like
//! `print`, arithmetic operations, and I/O.
//!
//! # Error Handling
//!
//! Native functions use [`NativeError`] for structured error reporting.
//! These errors use typed context (like `value::Kind`) instead of strings,
//! and formatting is centralized in `lonala-human`.

use alloc::collections::BTreeMap;

use lona_core::error_context::{ArityExpectation, TypeExpectation};
use lona_core::symbol::{self, Interner};
use lona_core::value::{self, Value};
use lonala_compiler::MacroRegistry;

/// Context passed to native functions during execution.
///
/// Provides access to VM state that native functions may need, such as
/// the symbol interner for resolving names and the macro registry for
/// introspection functions.
pub struct NativeContext<'vm> {
    /// Symbol interner for resolving symbol names.
    interner: &'vm Interner,
    /// Optional macro registry for introspection functions.
    macros: Option<&'vm MacroRegistry>,
}

impl<'vm> NativeContext<'vm> {
    /// Creates a new native context.
    #[inline]
    #[must_use]
    pub const fn new(interner: &'vm Interner, macros: Option<&'vm MacroRegistry>) -> Self {
        Self { interner, macros }
    }

    /// Returns a reference to the symbol interner.
    #[inline]
    #[must_use]
    pub const fn interner(&self) -> &'vm Interner {
        self.interner
    }

    /// Returns a reference to the macro registry, if available.
    #[inline]
    #[must_use]
    pub const fn macros(&self) -> Option<&'vm MacroRegistry> {
        self.macros
    }
}

/// Signature for native functions.
///
/// Native functions receive their arguments as a slice along with
/// a context providing access to VM state. Returns either a value
/// or an error.
///
/// # Arguments
///
/// * `args` - Slice of argument values passed to the function
/// * `ctx` - Context providing access to interner, macro registry, etc.
///
/// # Returns
///
/// The function result or an error.
pub type NativeFn = fn(args: &[Value], ctx: &NativeContext<'_>) -> Result<Value, NativeError>;

/// Errors that can occur in native functions.
///
/// Uses structured types instead of strings for consistent formatting
/// via `lonala-human`. This type does NOT implement `Display`; all
/// formatting is centralized.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum NativeError {
    /// Wrong number of arguments.
    ArityMismatch {
        /// Expected number of arguments.
        expected: ArityExpectation,
        /// Actual number of arguments provided.
        got: u8,
    },
    /// Type error in argument.
    TypeError {
        /// Expected type(s).
        expected: TypeExpectation,
        /// Actual type encountered.
        got: value::Kind,
        /// Zero-based argument index.
        arg_index: u8,
    },
    /// Division or modulo by zero.
    DivisionByZero,
    /// Generic error with message (for cases where structured data isn't available).
    Error(&'static str),
}

impl NativeError {
    /// Returns the variant name for error identification.
    #[inline]
    #[must_use]
    pub const fn variant_name(&self) -> &'static str {
        match *self {
            Self::ArityMismatch { .. } => "ArityMismatch",
            Self::TypeError { .. } => "TypeError",
            Self::DivisionByZero => "DivisionByZero",
            Self::Error(_) => "Error",
        }
    }
}

/// Registry mapping symbol IDs to native function implementations.
///
/// Used by the VM to look up native functions when executing `Call`
/// instructions on global function symbols.
#[non_exhaustive]
pub struct Registry {
    /// Map from symbol ID to native function.
    functions: BTreeMap<symbol::Id, NativeFn>,
}

impl Registry {
    /// Creates a new empty native function registry.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            functions: BTreeMap::new(),
        }
    }

    /// Registers a native function for a symbol.
    ///
    /// If a function was already registered for this symbol, it is replaced.
    #[inline]
    pub fn register(&mut self, symbol: symbol::Id, func: NativeFn) {
        let _previous = self.functions.insert(symbol, func);
    }

    /// Looks up a native function by symbol ID.
    ///
    /// Returns `Some(func)` if a native function is registered for the symbol,
    /// `None` otherwise.
    #[inline]
    #[must_use]
    pub fn get(&self, symbol: symbol::Id) -> Option<NativeFn> {
        self.functions.get(&symbol).copied()
    }

    /// Returns the number of registered native functions.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.functions.len()
    }

    /// Returns `true` if no native functions are registered.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }
}

impl Default for Registry {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Arithmetic Native Function Registration
// =============================================================================

/// The names of all arithmetic primitives.
pub const ARITHMETIC_PRIMITIVE_NAMES: &[&str] = &["+", "-", "*", "/", "mod"];

/// Pre-interns all arithmetic primitive symbols.
///
/// This must be called before creating the VM to avoid borrow conflicts.
/// Returns a vector of symbol IDs in the same order as `ARITHMETIC_PRIMITIVE_NAMES`.
#[inline]
pub fn intern_arithmetic_primitives(
    interner: &mut symbol::Interner,
) -> alloc::vec::Vec<symbol::Id> {
    ARITHMETIC_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.intern(name))
        .collect()
}

/// Looks up arithmetic primitive symbols from an immutable interner.
///
/// This is used when primitives should already be interned (e.g., by the REPL)
/// and we only have an immutable reference to the interner.
///
/// Returns `Some(symbols)` if all primitives are found, `None` otherwise.
#[inline]
#[must_use]
pub fn lookup_arithmetic_primitives(
    interner: &symbol::Interner,
) -> Option<alloc::vec::Vec<symbol::Id>> {
    ARITHMETIC_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.get(name))
        .collect()
}

/// Registers all arithmetic primitives with the VM using pre-interned symbols.
///
/// `symbols` must be the result of calling `intern_arithmetic_primitives` with
/// the same interner.
///
/// Each arithmetic function is registered:
/// - As a native function in the registry (for execution)
/// - As a `NativeFunction` value in globals (for first-class use)
#[inline]
pub fn register_arithmetic_primitives(vm: &mut super::interpreter::Vm<'_>, symbols: &[symbol::Id]) {
    let funcs: &[NativeFn] = &[native_add, native_sub, native_mul, native_div, native_mod];

    for (sym, func) in symbols.iter().zip(funcs.iter()) {
        vm.register_native(*sym, *func);
        // Use NativeFunction for first-class function support
        vm.set_global(*sym, Value::NativeFunction(*sym));
    }
}

// =============================================================================
// Built-in Native Functions for Arithmetic
// =============================================================================

use super::numeric;
use lona_core::integer::Integer;

/// Native implementation of `+` (addition).
///
/// Handles all arities:
/// - `(+)` → 0 (identity)
/// - `(+ x)` → x
/// - `(+ a b ...)` → sum of all arguments
#[inline]
pub fn native_add(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    match args.len() {
        0 => Ok(Value::Integer(Integer::from_i64(0))),
        1 => {
            // Validate that the single argument is numeric
            let arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            if !arg.kind().is_numeric() {
                return Err(NativeError::TypeError {
                    expected: TypeExpectation::Numeric,
                    got: arg.kind(),
                    arg_index: 0_u8,
                });
            }
            Ok(arg.clone())
        }
        _ => {
            let first_arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            let mut acc = first_arg.clone();
            for (idx, arg) in args.iter().skip(1).enumerate() {
                acc = numeric::add_values(&acc, arg).map_err(|err| {
                    // Adjust arg_index for the actual position in the original args.
                    // If error.arg_index is 0, the left operand (accumulator) was wrong.
                    // On first iteration, this means the first argument is wrong (index 0).
                    // If error.arg_index is 1, the right operand (current arg) was wrong.
                    if let NativeError::TypeError {
                        expected,
                        got,
                        arg_index: original_index,
                    } = err
                    {
                        NativeError::TypeError {
                            expected,
                            got,
                            arg_index: if original_index == 0 {
                                0 // Error in first argument
                            } else {
                                u8::try_from(idx.saturating_add(1)).unwrap_or(u8::MAX)
                            },
                        }
                    } else {
                        err
                    }
                })?;
            }
            Ok(acc)
        }
    }
}

/// Native implementation of `-` (subtraction).
///
/// Handles arities:
/// - `(-)` → Error (requires at least one argument)
/// - `(- x)` → -x (negation)
/// - `(- a b ...)` → a - b - ...
#[inline]
pub fn native_sub(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    match args.len() {
        0 => Err(NativeError::ArityMismatch {
            expected: ArityExpectation::AtLeast(1_u8),
            got: 0_u8,
        }),
        1 => {
            let arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            numeric::negate_value(arg)
        }
        _ => {
            let first_arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            let mut acc = first_arg.clone();
            for (idx, arg) in args.iter().skip(1).enumerate() {
                acc = numeric::sub_values(&acc, arg).map_err(|err| {
                    // Adjust arg_index for the actual position in the original args.
                    // If error.arg_index is 0, the left operand (accumulator) was wrong.
                    // On first iteration, this means the first argument is wrong (index 0).
                    // If error.arg_index is 1, the right operand (current arg) was wrong.
                    if let NativeError::TypeError {
                        expected,
                        got,
                        arg_index: original_index,
                    } = err
                    {
                        NativeError::TypeError {
                            expected,
                            got,
                            arg_index: if original_index == 0 {
                                0 // Error in first argument
                            } else {
                                u8::try_from(idx.saturating_add(1)).unwrap_or(u8::MAX)
                            },
                        }
                    } else {
                        err
                    }
                })?;
            }
            Ok(acc)
        }
    }
}

/// Native implementation of `*` (multiplication).
///
/// Handles all arities:
/// - `(*)` → 1 (identity for multiplication)
/// - `(* x)` → x (validates numeric type)
/// - `(* a b ...)` → product of all arguments
#[inline]
pub fn native_mul(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    match args.len() {
        0 => Ok(Value::Integer(Integer::from_i64(1))),
        1 => {
            // Validate that the single argument is numeric
            let arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            if !arg.kind().is_numeric() {
                return Err(NativeError::TypeError {
                    expected: TypeExpectation::Numeric,
                    got: arg.kind(),
                    arg_index: 0_u8,
                });
            }
            Ok(arg.clone())
        }
        _ => {
            let first_arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            let mut acc = first_arg.clone();
            for (idx, arg) in args.iter().skip(1).enumerate() {
                acc = numeric::mul_values(&acc, arg).map_err(|err| {
                    // Adjust arg_index for the actual position in the original args.
                    // If error.arg_index is 0, the left operand (accumulator) was wrong.
                    // On first iteration, this means the first argument is wrong (index 0).
                    // If error.arg_index is 1, the right operand (current arg) was wrong.
                    if let NativeError::TypeError {
                        expected,
                        got,
                        arg_index: original_index,
                    } = err
                    {
                        NativeError::TypeError {
                            expected,
                            got,
                            arg_index: if original_index == 0 {
                                0 // Error in first argument
                            } else {
                                u8::try_from(idx.saturating_add(1)).unwrap_or(u8::MAX)
                            },
                        }
                    } else {
                        err
                    }
                })?;
            }
            Ok(acc)
        }
    }
}

/// Native implementation of `/` (division).
///
/// Handles arities:
/// - `(/)` → Error (requires at least one argument)
/// - `(/ x)` → 1/x (reciprocal)
/// - `(/ a b ...)` → a / b / ...
#[inline]
pub fn native_div(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    match args.len() {
        0 => Err(NativeError::ArityMismatch {
            expected: ArityExpectation::AtLeast(1_u8),
            got: 0_u8,
        }),
        1 => {
            let arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            numeric::inverse_value(arg)
        }
        _ => {
            let first_arg = args.first().ok_or(NativeError::Error("missing argument"))?;
            let mut acc = first_arg.clone();
            for (idx, arg) in args.iter().skip(1).enumerate() {
                acc = numeric::div_values(&acc, arg).map_err(|err| {
                    // Adjust arg_index for the actual position in the original args.
                    // If error.arg_index is 0, the left operand (accumulator) was wrong.
                    // On first iteration, this means the first argument is wrong (index 0).
                    // If error.arg_index is 1, the right operand (current arg) was wrong.
                    if let NativeError::TypeError {
                        expected,
                        got,
                        arg_index: original_index,
                    } = err
                    {
                        NativeError::TypeError {
                            expected,
                            got,
                            arg_index: if original_index == 0 {
                                0 // Error in first argument
                            } else {
                                u8::try_from(idx.saturating_add(1)).unwrap_or(u8::MAX)
                            },
                        }
                    } else {
                        err
                    }
                })?;
            }
            Ok(acc)
        }
    }
}

/// Native implementation of `mod` (modulo).
///
/// Requires exactly 2 arguments:
/// - `(mod a b)` → a % b
#[inline]
pub fn native_mod(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 2_usize {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(2_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let left = args.first().ok_or(NativeError::Error("missing argument"))?;
    let right = args
        .get(1_usize)
        .ok_or(NativeError::Error("missing argument"))?;

    numeric::modulo_values(left, right)
}

// =============================================================================
// Comparison Native Function Registration
// =============================================================================

/// The names of all comparison primitives.
pub const COMPARISON_PRIMITIVE_NAMES: &[&str] = &["=", "<", ">", "<=", ">="];

/// Pre-interns all comparison primitive symbols.
///
/// This must be called before creating the VM to avoid borrow conflicts.
/// Returns a vector of symbol IDs in the same order as `COMPARISON_PRIMITIVE_NAMES`.
#[inline]
pub fn intern_comparison_primitives(
    interner: &mut symbol::Interner,
) -> alloc::vec::Vec<symbol::Id> {
    COMPARISON_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.intern(name))
        .collect()
}

/// Looks up comparison primitive symbols from an immutable interner.
///
/// This is used when primitives should already be interned (e.g., by the REPL)
/// and we only have an immutable reference to the interner.
///
/// Returns `Some(symbols)` if all primitives are found, `None` otherwise.
#[inline]
#[must_use]
pub fn lookup_comparison_primitives(
    interner: &symbol::Interner,
) -> Option<alloc::vec::Vec<symbol::Id>> {
    COMPARISON_PRIMITIVE_NAMES
        .iter()
        .map(|name| interner.get(name))
        .collect()
}

/// Registers all comparison primitives with the VM using pre-interned symbols.
///
/// `symbols` must be the result of calling `intern_comparison_primitives` with
/// the same interner.
///
/// Each comparison function is registered:
/// - As a native function in the registry (for execution)
/// - As a `NativeFunction` value in globals (for first-class use)
#[inline]
pub fn register_comparison_primitives(vm: &mut super::interpreter::Vm<'_>, symbols: &[symbol::Id]) {
    let funcs: &[NativeFn] = &[native_eq, native_lt, native_gt, native_le, native_ge];

    for (sym, func) in symbols.iter().zip(funcs.iter()) {
        vm.register_native(*sym, *func);
        // Use NativeFunction for first-class function support
        vm.set_global(*sym, Value::NativeFunction(*sym));
    }
}

// =============================================================================
// Built-in Native Functions for Comparison
// =============================================================================

use super::helpers::values_equal;

/// Native implementation of `=` (equality).
///
/// Handles all arities:
/// - `(=)` → true (vacuously, but typically error per Clojure spec)
/// - `(= x)` → true (vacuously)
/// - `(= a b ...)` → true if all arguments are semantically equal pairwise
#[inline]
pub fn native_eq(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    // Per Clojure spec: 0 or 1 args returns true (vacuously)
    if args.len() <= 1_usize {
        return Ok(Value::Bool(true));
    }

    // Check all adjacent pairs
    for pair in args.windows(2_usize) {
        let left = pair.first().ok_or(NativeError::Error("missing argument"))?;
        let right = pair
            .get(1_usize)
            .ok_or(NativeError::Error("missing argument"))?;
        if !values_equal(left, right) {
            return Ok(Value::Bool(false));
        }
    }

    Ok(Value::Bool(true))
}

/// Native implementation of `<` (less than).
///
/// Handles arities:
/// - `(<)` → true (vacuously, per Clojure)
/// - `(< x)` → true (vacuously, per Clojure)
/// - `(< a b ...)` → true if args are in strictly increasing order
#[inline]
pub fn native_lt(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    compare_chain(args, |ord| ord == core::cmp::Ordering::Less)
}

/// Native implementation of `>` (greater than).
///
/// Handles arities:
/// - `(>)` → true (vacuously, per Clojure)
/// - `(> x)` → true (vacuously, per Clojure)
/// - `(> a b ...)` → true if args are in strictly decreasing order
#[inline]
pub fn native_gt(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    compare_chain(args, |ord| ord == core::cmp::Ordering::Greater)
}

/// Native implementation of `<=` (less than or equal).
///
/// Handles arities:
/// - `(<=)` → true (vacuously, per Clojure)
/// - `(<= x)` → true (vacuously, per Clojure)
/// - `(<= a b ...)` → true if args are in non-decreasing order
#[inline]
pub fn native_le(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    compare_chain(args, |ord| {
        ord == core::cmp::Ordering::Less || ord == core::cmp::Ordering::Equal
    })
}

/// Native implementation of `>=` (greater than or equal).
///
/// Handles arities:
/// - `(>=)` → true (vacuously, per Clojure)
/// - `(>= x)` → true (vacuously, per Clojure)
/// - `(>= a b ...)` → true if args are in non-increasing order
#[inline]
pub fn native_ge(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    compare_chain(args, |ord| {
        ord == core::cmp::Ordering::Greater || ord == core::cmp::Ordering::Equal
    })
}

/// Performs a chained comparison on numeric values.
///
/// Returns true if all adjacent pairs satisfy the comparison predicate.
fn compare_chain<F>(args: &[Value], pred: F) -> Result<Value, NativeError>
where
    F: Fn(core::cmp::Ordering) -> bool,
{
    // Per Clojure spec: 0 or 1 args returns true (vacuously)
    if args.len() <= 1_usize {
        return Ok(Value::Bool(true));
    }

    // Check all adjacent pairs
    for (idx, pair) in args.windows(2_usize).enumerate() {
        let left = pair.first().ok_or(NativeError::Error("missing argument"))?;
        let right = pair
            .get(1_usize)
            .ok_or(NativeError::Error("missing argument"))?;

        match numeric::compare_values(left, right) {
            Ok(ord) => {
                if !pred(ord) {
                    return Ok(Value::Bool(false));
                }
            }
            Err(err) => {
                // Adjust arg_index: pair[0] is at position idx, pair[1] is at idx+1
                if let NativeError::TypeError {
                    expected,
                    got,
                    arg_index: original_index,
                } = err
                {
                    return Err(NativeError::TypeError {
                        expected,
                        got,
                        arg_index: if original_index == 0 {
                            u8::try_from(idx).unwrap_or(u8::MAX)
                        } else {
                            u8::try_from(idx.saturating_add(1)).unwrap_or(u8::MAX)
                        },
                    });
                }
                return Err(err);
            }
        }
    }

    Ok(Value::Bool(true))
}

// =============================================================================
// Type Predicate Native Function Registration
// =============================================================================

/// The names of all type predicate primitives.
pub const TYPE_PREDICATE_NAMES: &[&str] = &["keyword?"];

/// Pre-interns all type predicate primitive symbols.
///
/// This must be called before creating the VM to avoid borrow conflicts.
/// Returns a vector of symbol IDs in the same order as `TYPE_PREDICATE_NAMES`.
#[inline]
pub fn intern_type_predicates(interner: &mut symbol::Interner) -> alloc::vec::Vec<symbol::Id> {
    TYPE_PREDICATE_NAMES
        .iter()
        .map(|name| interner.intern(name))
        .collect()
}

/// Looks up type predicate primitive symbols from an immutable interner.
///
/// This is used when primitives should already be interned (e.g., by the REPL)
/// and we only have an immutable reference to the interner.
///
/// Returns `Some(symbols)` if all primitives are found, `None` otherwise.
#[inline]
#[must_use]
pub fn lookup_type_predicates(interner: &symbol::Interner) -> Option<alloc::vec::Vec<symbol::Id>> {
    TYPE_PREDICATE_NAMES
        .iter()
        .map(|name| interner.get(name))
        .collect()
}

/// Registers all type predicate primitives with the VM using pre-interned symbols.
///
/// `symbols` must be the result of calling `intern_type_predicates` with
/// the same interner.
///
/// Each type predicate function is registered:
/// - As a native function in the registry (for execution)
/// - As a `NativeFunction` value in globals (for first-class use)
#[inline]
pub fn register_type_predicates(vm: &mut super::interpreter::Vm<'_>, symbols: &[symbol::Id]) {
    let funcs: &[NativeFn] = &[native_keyword_p];

    for (sym, func) in symbols.iter().zip(funcs.iter()) {
        vm.register_native(*sym, *func);
        // Use NativeFunction for first-class function support
        vm.set_global(*sym, Value::NativeFunction(*sym));
    }
}

// =============================================================================
// Built-in Native Functions for Type Predicates
// =============================================================================

/// Native implementation of `keyword?` (type predicate).
///
/// Returns true if the argument is a keyword.
#[inline]
pub fn native_keyword_p(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
    if args.len() != 1_usize {
        return Err(NativeError::ArityMismatch {
            expected: ArityExpectation::Exact(1_u8),
            got: u8::try_from(args.len()).unwrap_or(u8::MAX),
        });
    }

    let arg = args.first().ok_or(NativeError::Error("missing argument"))?;
    Ok(Value::Bool(arg.is_keyword()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lona_core::integer::Integer;
    use lona_core::symbol::Interner;

    /// Simple native function for testing.
    fn test_add(args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
        if args.len() != 2_usize {
            return Err(NativeError::ArityMismatch {
                expected: ArityExpectation::Exact(2_u8),
                got: u8::try_from(args.len()).unwrap_or(u8::MAX),
            });
        }

        let a_val = args.first().ok_or(NativeError::Error("missing argument"))?;
        let left = a_val.as_integer().ok_or(NativeError::TypeError {
            expected: TypeExpectation::Single(value::Kind::Integer),
            got: a_val.kind(),
            arg_index: 0_u8,
        })?;

        let b_val = args
            .get(1_usize)
            .ok_or(NativeError::Error("missing argument"))?;
        let right = b_val.as_integer().ok_or(NativeError::TypeError {
            expected: TypeExpectation::Single(value::Kind::Integer),
            got: b_val.kind(),
            arg_index: 1_u8,
        })?;

        Ok(Value::Integer(left + right))
    }

    /// Native function that returns nil.
    fn test_nil(_args: &[Value], _ctx: &NativeContext<'_>) -> Result<Value, NativeError> {
        Ok(Value::Nil)
    }

    #[test]
    fn register_and_get_native() {
        let mut interner = Interner::new();
        let mut registry = Registry::new();

        let add_sym = interner.intern("add");
        registry.register(add_sym, test_add);

        assert!(registry.get(add_sym).is_some());
    }

    #[test]
    fn get_unknown_symbol_returns_none() {
        let mut interner = Interner::new();
        let registry = Registry::new();

        let unknown_sym = interner.intern("unknown");
        assert!(registry.get(unknown_sym).is_none());
    }

    #[test]
    fn call_registered_native() {
        let mut interner = Interner::new();
        let mut registry = Registry::new();

        let add_sym = interner.intern("add");
        registry.register(add_sym, test_add);

        let native_fn = registry.get(add_sym).unwrap();
        let args = [
            Value::Integer(Integer::from_i64(1)),
            Value::Integer(Integer::from_i64(2)),
        ];
        let ctx = NativeContext::new(&interner, None);
        let result = native_fn(&args, &ctx).unwrap();

        assert_eq!(result, Value::Integer(Integer::from_i64(3)));
    }

    #[test]
    fn native_arity_error() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);
        let result = test_add(&[Value::Integer(Integer::from_i64(1))], &ctx);
        assert!(matches!(
            result,
            Err(NativeError::ArityMismatch {
                expected: ArityExpectation::Exact(2_u8),
                got: 1_u8
            })
        ));
    }

    #[test]
    fn native_type_error() {
        let interner = Interner::new();
        let ctx = NativeContext::new(&interner, None);
        let result = test_add(
            &[Value::Bool(true), Value::Integer(Integer::from_i64(2))],
            &ctx,
        );
        match result {
            Err(NativeError::TypeError {
                expected,
                got,
                arg_index,
            }) => {
                assert_eq!(expected, TypeExpectation::Single(value::Kind::Integer));
                assert_eq!(got, value::Kind::Bool);
                assert_eq!(arg_index, 0_u8);
            }
            _ => panic!("Expected TypeError"),
        }
    }

    #[test]
    fn registry_len_and_is_empty() {
        let mut interner = Interner::new();
        let mut registry = Registry::new();

        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        let sym = interner.intern("test");
        registry.register(sym, test_nil);

        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn register_replaces_existing() {
        let mut interner = Interner::new();
        let mut registry = Registry::new();

        let sym = interner.intern("test");
        registry.register(sym, test_add);
        registry.register(sym, test_nil);

        // Should have replaced, not added
        assert_eq!(registry.len(), 1);

        // Should use the new function
        let native_fn = registry.get(sym).unwrap();
        let ctx = NativeContext::new(&interner, None);
        let result = native_fn(&[], &ctx).unwrap();
        assert_eq!(result, Value::Nil);
    }

    #[test]
    fn native_error_variant_name() {
        assert_eq!(
            NativeError::ArityMismatch {
                expected: ArityExpectation::Exact(2_u8),
                got: 3_u8
            }
            .variant_name(),
            "ArityMismatch"
        );
        assert_eq!(
            NativeError::TypeError {
                expected: TypeExpectation::Numeric,
                got: value::Kind::Bool,
                arg_index: 0_u8
            }
            .variant_name(),
            "TypeError"
        );
        assert_eq!(NativeError::DivisionByZero.variant_name(), "DivisionByZero");
        assert_eq!(NativeError::Error("test").variant_name(), "Error");
    }
}
