// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Source location spans for error reporting and debugging.
//!
//! A `Span` represents a byte range in source code, enabling precise error
//! messages and source-level debugging information in compiled bytecode.

use core::fmt;

/// A byte range in source code.
///
/// Spans are half-open intervals `[start, end)` representing byte offsets
/// into the source string. They enable precise error messages that can
/// highlight the problematic portion of the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub struct Span {
    /// Byte offset of the start (inclusive).
    pub start: usize,
    /// Byte offset of the end (exclusive).
    pub end: usize,
}

impl Span {
    /// Creates a new span from start to end byte offsets.
    #[inline]
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Returns the length of this span in bytes.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns true if this span has zero length.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

impl fmt::Display for Span {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { start, end } = *self;
        write!(f, "{start}..{end}")
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::format;

    use super::*;

    #[test]
    fn span_new_and_accessors() {
        let span = Span::new(10_usize, 20_usize);
        assert_eq!(span.start, 10_usize);
        assert_eq!(span.end, 20_usize);
        assert_eq!(span.len(), 10_usize);
        assert!(!span.is_empty());
    }

    #[test]
    fn span_empty() {
        let span = Span::new(5_usize, 5_usize);
        assert!(span.is_empty());
        assert_eq!(span.len(), 0_usize);
    }

    #[test]
    fn span_display() {
        let span = Span::new(10_usize, 20_usize);
        assert_eq!(format!("{span}"), "10..20");
    }

    #[test]
    fn span_default() {
        let span = Span::default();
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 0);
        assert!(span.is_empty());
    }
}
