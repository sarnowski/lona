// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Built-in primitive functions for the Lonala language.
//!
//! Provides core primitives like `print` that require special handling
//! by the VM due to their interaction with I/O or other VM internals.

use alloc::string::String;

use core::fmt::Write as _;

use lona_core::symbol::Interner;
use lona_core::value::Value;

use super::natives::NativeError;

/// Formats values for print output.
///
/// Formats each value separated by spaces, with a trailing newline.
/// Returns the formatted string.
///
/// # Arguments
///
/// * `args` - Values to format
/// * `interner` - Symbol interner for resolving symbol names
#[inline]
#[must_use]
pub fn format_print_args(args: &[Value], interner: &Interner) -> String {
    let mut output = String::new();

    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            output.push(' ');
        }
        // Format the value using its display implementation
        let _err = write!(output, "{}", arg.display(interner));
    }

    output.push('\n');
    output
}

/// Print callback type.
///
/// Called by the VM when `print` is invoked. Receives the formatted
/// output string.
pub type PrintCallback = fn(&str);

/// Native print implementation.
///
/// Formats the arguments and returns them as a formatted string.
/// The actual output is handled by the VM's print callback.
///
/// Always returns `Value::Nil`.
///
/// # Errors
///
/// This function does not return errors - any arguments are accepted.
#[inline]
pub fn native_print(args: &[Value], interner: &Interner) -> Result<Value, NativeError> {
    // Format but don't output - the VM handles output via callback
    // This function exists for signature compatibility; actual print
    // is handled specially by the VM
    let _formatted = format_print_args(args, interner);
    Ok(Value::Nil)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lona_core::integer::Integer;
    use lona_core::ratio::Ratio;
    use lona_core::symbol::Interner;

    /// Helper to create an integer value.
    fn int(value: i64) -> Value {
        Value::Integer(Integer::from_i64(value))
    }

    #[test]
    fn format_print_single_integer() {
        let interner = Interner::new();
        let result = format_print_args(&[int(42)], &interner);
        assert_eq!(result, "42\n");
    }

    #[test]
    fn format_print_multiple_integers() {
        let interner = Interner::new();
        let result = format_print_args(&[int(1), int(2), int(3)], &interner);
        assert_eq!(result, "1 2 3\n");
    }

    #[test]
    fn format_print_float() {
        let interner = Interner::new();
        let result = format_print_args(&[Value::Float(3.14)], &interner);
        assert_eq!(result, "3.14\n");
    }

    #[test]
    fn format_print_float_whole() {
        let interner = Interner::new();
        let result = format_print_args(&[Value::Float(4.0)], &interner);
        assert_eq!(result, "4.0\n");
    }

    #[test]
    fn format_print_bool() {
        let interner = Interner::new();

        let result = format_print_args(&[Value::Bool(true)], &interner);
        assert_eq!(result, "true\n");

        let result = format_print_args(&[Value::Bool(false)], &interner);
        assert_eq!(result, "false\n");
    }

    #[test]
    fn format_print_nil() {
        let interner = Interner::new();
        let result = format_print_args(&[Value::Nil], &interner);
        assert_eq!(result, "nil\n");
    }

    #[test]
    fn format_print_symbol() {
        let mut interner = Interner::new();
        let sym_id = interner.intern("my-symbol");

        let result = format_print_args(&[Value::Symbol(sym_id)], &interner);
        assert_eq!(result, "my-symbol\n");
    }

    #[test]
    fn format_print_mixed() {
        let mut interner = Interner::new();
        let sym_id = interner.intern("x");

        let result = format_print_args(
            &[
                int(1),
                Value::Float(2.5),
                Value::Bool(true),
                Value::Symbol(sym_id),
            ],
            &interner,
        );
        assert_eq!(result, "1 2.5 true x\n");
    }

    #[test]
    fn format_print_empty() {
        let interner = Interner::new();
        let result = format_print_args(&[], &interner);
        assert_eq!(result, "\n");
    }

    #[test]
    fn native_print_returns_nil() {
        let interner = Interner::new();
        let result = native_print(&[int(42)], &interner).unwrap();
        assert_eq!(result, Value::Nil);
    }

    #[test]
    fn format_print_negative_integer() {
        let interner = Interner::new();
        let result = format_print_args(&[int(-5)], &interner);
        assert_eq!(result, "-5\n");
    }

    #[test]
    fn format_print_zero() {
        let interner = Interner::new();
        let result = format_print_args(&[int(0)], &interner);
        assert_eq!(result, "0\n");
    }

    #[test]
    fn format_print_ratio() {
        let interner = Interner::new();
        let ratio = Value::Ratio(Ratio::from_i64(1, 3));
        let result = format_print_args(&[ratio], &interner);
        assert_eq!(result, "1/3\n");
    }

    #[test]
    fn format_print_ratio_whole() {
        let interner = Interner::new();
        // 4/2 should normalize to 2 and display as "2"
        let ratio = Value::Ratio(Ratio::from_i64(4, 2));
        let result = format_print_args(&[ratio], &interner);
        assert_eq!(result, "2\n");
    }
}
