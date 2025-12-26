// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Display implementations for Value.

use core::fmt::{self, Display, Write as _};

use super::Value;
use crate::symbol::Interner;

impl Display for Value {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Nil => write!(f, "nil"),
            Self::Bool(true) => write!(f, "true"),
            Self::Bool(false) => write!(f, "false"),
            Self::Integer(ref value) => write!(f, "{value}"),
            Self::Float(value) => format_float(value, f),
            #[cfg(feature = "alloc")]
            Self::Ratio(ref value) => write!(f, "{value}"),
            #[cfg(feature = "alloc")]
            Self::Symbol(ref sym) => write!(f, "#<symbol:{}>", sym.id().as_u32()),
            #[cfg(not(feature = "alloc"))]
            Self::Symbol(id) => write!(f, "#<symbol:{}>", id.as_u32()),
            Self::Keyword(id) => write!(f, "#<keyword:{}>", id.as_u32()),
            Self::NativeFunction(id) => write!(f, "#<native-fn:{}>", id.as_u32()),
            #[cfg(feature = "alloc")]
            Self::String(ref string) => write_escaped_string(string.as_str(), f),
            #[cfg(feature = "alloc")]
            Self::List(ref list) => write!(f, "{list}"),
            #[cfg(feature = "alloc")]
            Self::Vector(ref vector) => write!(f, "{vector}"),
            #[cfg(feature = "alloc")]
            Self::Map(ref map) => write!(f, "{map}"),
            #[cfg(feature = "alloc")]
            Self::Set(ref set) => write!(f, "{set}"),
            #[cfg(feature = "alloc")]
            Self::Binary(ref binary) => write!(f, "{binary}"),
            #[cfg(feature = "alloc")]
            Self::Function(ref func) => write!(f, "{func}"),
            #[cfg(feature = "alloc")]
            Self::Var(ref var) => {
                if let Some(ns) = var.namespace() {
                    write!(f, "#<var:{}/{}>", ns.as_u32(), var.name().as_u32())
                } else {
                    write!(f, "#<var:{}>", var.name().as_u32())
                }
            }
        }
    }
}

/// Formats a float in Lonala syntax.
///
/// Ensures that whole numbers still show as floats (e.g., "1.0" not "1").
pub(super) fn format_float(value: f64, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
    if value.is_nan() {
        write!(formatter, "##NaN")
    } else if value.is_infinite() {
        if value.is_sign_positive() {
            write!(formatter, "##Inf")
        } else {
            write!(formatter, "##-Inf")
        }
    } else {
        // Check if the number is a whole number within i64 range
        // A float is whole if converting to i64 and back gives the same value
        #[expect(
            clippy::as_conversions,
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::float_cmp,
            reason = "[approved] checking if float is representable as i64 - exact equality is intentional"
        )]
        let is_whole = value >= (i64::MIN as f64)
            && value <= (i64::MAX as f64)
            && (value as i64 as f64) == value;

        if is_whole {
            // Whole number - ensure decimal point is shown
            write!(formatter, "{value:.1}")
        } else {
            // Has fractional part or is very large - use default formatting
            write!(formatter, "{value}")
        }
    }
}

/// Writes a string with quotes and escape sequences.
///
/// Escapes:
/// - Backslash (`\\`)
/// - Double quote (`\"`)
/// - Standard control characters: newline (`\n`), carriage return (`\r`), tab (`\t`)
/// - Other ASCII control characters (0x00-0x1F, 0x7F) as hex escapes (`\xNN`)
#[cfg(feature = "alloc")]
pub(super) fn write_escaped_string(
    string: &str,
    formatter: &mut fmt::Formatter<'_>,
) -> fmt::Result {
    formatter.write_char('"')?;
    for ch in string.chars() {
        match ch {
            '\\' => formatter.write_str("\\\\")?,
            '"' => formatter.write_str("\\\"")?,
            '\n' => formatter.write_str("\\n")?,
            '\r' => formatter.write_str("\\r")?,
            '\t' => formatter.write_str("\\t")?,
            // Other ASCII control characters (0x00-0x1F except \t\n\r, and 0x7F)
            '\x00'..='\x08' | '\x0b'..='\x0c' | '\x0e'..='\x1f' | '\x7f' => {
                // Safe: matched chars are all ≤ 0x7F, so u32::from fits in 2 hex digits
                write!(formatter, "\\x{:02x}", u32::from(ch))?;
            }
            other => formatter.write_char(other)?,
        }
    }
    formatter.write_char('"')
}

/// A wrapper for displaying a [`Value`] with symbol name resolution.
///
/// Created via [`Value::display`].
#[cfg(feature = "alloc")]
pub struct Displayable<'interner> {
    value: &'interner Value,
    interner: &'interner Interner,
}

#[cfg(feature = "alloc")]
impl<'interner> Displayable<'interner> {
    /// Creates a new displayable wrapper.
    #[inline]
    #[must_use]
    pub(super) const fn new(value: &'interner Value, interner: &'interner Interner) -> Self {
        Self { value, interner }
    }
}

#[cfg(feature = "alloc")]
impl Display for Displayable<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self.value {
            Value::Symbol(ref sym) => write!(f, "{}", self.interner.resolve(sym.id())),
            Value::Keyword(id) => write!(f, ":{}", self.interner.resolve(id)),
            Value::NativeFunction(id) => {
                write!(f, "#<native-fn:{}>", self.interner.resolve(id))
            }
            Value::Nil => write!(f, "nil"),
            Value::Bool(true) => write!(f, "true"),
            Value::Bool(false) => write!(f, "false"),
            Value::Integer(ref value) => write!(f, "{value}"),
            Value::Float(value) => format_float(value, f),
            Value::Ratio(ref value) => write!(f, "{value}"),
            Value::String(ref string) => write_escaped_string(string.as_str(), f),
            Value::List(ref list) => write!(f, "{}", list.display(self.interner)),
            Value::Vector(ref vector) => write!(f, "{}", vector.display(self.interner)),
            Value::Map(ref map) => write!(f, "{}", map.display(self.interner)),
            Value::Set(ref set) => write!(f, "{}", set.display(self.interner)),
            Value::Binary(ref binary) => write!(f, "{binary}"),
            Value::Function(ref func) => write!(f, "{func}"),
            Value::Var(ref var) => {
                if let Some(ns) = var.namespace() {
                    write!(
                        f,
                        "#<var:{}/{}>",
                        self.interner.resolve(ns),
                        self.interner.resolve(var.name())
                    )
                } else {
                    write!(f, "#<var:{}>", self.interner.resolve(var.name()))
                }
            }
        }
    }
}
