// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Source tracking for error reporting.
//!
//! This module provides types for tracking source locations across multiple
//! source files or REPL inputs, enabling precise error messages with context.
//!
//! # Types
//!
//! - [`Id`] - Identifies a source (use as `source::Id`)
//! - [`Location`] - Combines source ID with byte span (use as `source::Location`)
//! - [`Entry`] - Source metadata with name and content
//! - [`Registry`] - Collection of sources (use as `source::Registry`)

use alloc::string::String;
use alloc::vec::Vec;

use crate::span::Span;

/// Identifies a source (file, REPL input, etc.).
///
/// Each source in the registry is assigned a unique `Id` that can be
/// used to look up the source content and metadata. The ID is stable for
/// the lifetime of the registry.
///
/// Use as `source::Id` for clear code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(u32);

impl Id {
    /// Creates a new source ID from a raw index.
    ///
    /// This is primarily for internal use by [`Registry`].
    #[inline]
    #[must_use]
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    /// Returns the raw index of this source ID.
    #[inline]
    #[must_use]
    pub const fn index(self) -> u32 {
        self.0
    }
}

/// Complete source location: which source and where within it.
///
/// Combines an [`Id`] (identifying which source) with a [`Span`]
/// (identifying the byte range within that source).
///
/// Use as `source::Location` for clear code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Location {
    /// Which source this location refers to.
    pub source: Id,
    /// Byte range within the source.
    pub span: Span,
}

impl Location {
    /// Creates a new source location.
    #[inline]
    #[must_use]
    pub const fn new(source: Id, span: Span) -> Self {
        Self { source, span }
    }
}

/// Metadata about a source.
///
/// Contains both the human-readable name (for display in error messages)
/// and the actual source content (for extracting context lines).
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Entry {
    /// Human-readable name: "<repl>", "<repl:3>", "main.lona", etc.
    pub name: String,
    /// The actual source text.
    pub content: String,
}

impl Entry {
    /// Creates a new source entry with the given name and content.
    #[inline]
    #[must_use]
    pub const fn new(name: String, content: String) -> Self {
        Self { name, content }
    }

    /// Computes the 1-indexed line and column for a byte offset.
    ///
    /// Returns `(line, column)` where both are 1-indexed. If the offset is
    /// beyond the end of the content, returns the position at EOF.
    ///
    /// # Examples
    ///
    /// ```
    /// use lona_core::source::Entry;
    ///
    /// let entry = Entry::new("test".into(), "abc\ndef".into());
    /// assert_eq!(entry.line_col(0), (1, 1));  // 'a'
    /// assert_eq!(entry.line_col(3), (1, 4));  // '\n'
    /// assert_eq!(entry.line_col(4), (2, 1));  // 'd'
    /// ```
    #[inline]
    #[must_use]
    pub fn line_col(&self, offset: usize) -> (u32, u32) {
        let mut line = 1_u32;
        let mut col = 1_u32;

        for (idx, ch) in self.content.char_indices() {
            if idx >= offset {
                break;
            }
            if ch == '\n' {
                line = line.saturating_add(1);
                col = 1;
            } else {
                col = col.saturating_add(1);
            }
        }

        (line, col)
    }
}

/// Registry of all sources.
///
/// The registry maintains a collection of sources that can be referenced
/// by [`Id`]. This enables error messages to display the source name
/// and extract context lines for display.
///
/// Use as `source::Registry` for clear code.
#[derive(Debug, Default)]
pub struct Registry {
    /// Collection of sources indexed by their `Id`.
    sources: Vec<Entry>,
}

impl Registry {
    /// Creates a new empty source registry.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Adds a source to the registry and returns its ID.
    ///
    /// Returns `None` if the registry is full (more than `u32::MAX` sources).
    #[inline]
    pub fn add(&mut self, name: String, content: String) -> Option<Id> {
        let id = Id::new(u32::try_from(self.sources.len()).ok()?);
        self.sources.push(Entry::new(name, content));
        Some(id)
    }

    /// Retrieves a source by its ID.
    ///
    /// Returns `None` if the ID is invalid (e.g., from a different registry).
    #[inline]
    #[must_use]
    pub fn get(&self, id: Id) -> Option<&Entry> {
        self.sources.get(usize::try_from(id.0).ok()?)
    }

    /// Returns the number of sources in the registry.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.sources.len()
    }

    /// Returns true if the registry contains no sources.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::string::ToString;

    use super::*;

    #[test]
    fn id_new_and_index() {
        let id = Id::new(42_u32);
        assert_eq!(id.index(), 42_u32);
    }

    #[test]
    fn id_equality() {
        let id1 = Id::new(5_u32);
        let id2 = Id::new(5_u32);
        let id3 = Id::new(10_u32);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn location_new() {
        let source_id = Id::new(1_u32);
        let span = Span::new(10_usize, 20_usize);
        let location = Location::new(source_id, span);
        assert_eq!(location.source, source_id);
        assert_eq!(location.span, span);
    }

    #[test]
    fn entry_new() {
        let source = Entry::new("<repl>".to_string(), "(+ 1 2)".to_string());
        assert_eq!(source.name, "<repl>");
        assert_eq!(source.content, "(+ 1 2)");
    }

    #[test]
    fn registry_add_and_get() {
        let mut registry = Registry::new();
        let id1 = registry
            .add("<repl>".to_string(), "(+ 1 2)".to_string())
            .expect("should succeed");
        let id2 = registry
            .add("main.lona".to_string(), "(defn main [] nil)".to_string())
            .expect("should succeed");

        assert_ne!(id1, id2);

        let source1 = registry.get(id1).expect("source1 should exist");
        assert_eq!(source1.name, "<repl>");
        assert_eq!(source1.content, "(+ 1 2)");

        let source2 = registry.get(id2).expect("source2 should exist");
        assert_eq!(source2.name, "main.lona");
        assert_eq!(source2.content, "(defn main [] nil)");
    }

    #[test]
    fn registry_get_invalid_id() {
        let registry = Registry::new();
        let invalid_id = Id::new(999_u32);
        assert!(registry.get(invalid_id).is_none());
    }

    #[test]
    fn registry_len_and_is_empty() {
        let mut registry = Registry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0_usize);

        registry
            .add("<repl>".to_string(), "(+ 1 2)".to_string())
            .expect("should succeed");
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1_usize);

        registry
            .add("<repl:2>".to_string(), "(* 3 4)".to_string())
            .expect("should succeed");
        assert_eq!(registry.len(), 2_usize);
    }

    #[test]
    fn registry_default() {
        let registry = Registry::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn line_col_first_character() {
        let entry = Entry::new("test".to_string(), "abc\ndef".to_string());
        assert_eq!(entry.line_col(0_usize), (1_u32, 1_u32));
    }

    #[test]
    fn line_col_same_line() {
        let entry = Entry::new("test".to_string(), "abc\ndef".to_string());
        assert_eq!(entry.line_col(1_usize), (1_u32, 2_u32)); // 'b'
        assert_eq!(entry.line_col(2_usize), (1_u32, 3_u32)); // 'c'
    }

    #[test]
    fn line_col_newline_char() {
        let entry = Entry::new("test".to_string(), "abc\ndef".to_string());
        // The newline at index 3 is at line 1, column 4
        assert_eq!(entry.line_col(3_usize), (1_u32, 4_u32));
    }

    #[test]
    fn line_col_second_line() {
        let entry = Entry::new("test".to_string(), "abc\ndef".to_string());
        assert_eq!(entry.line_col(4_usize), (2_u32, 1_u32)); // 'd'
        assert_eq!(entry.line_col(5_usize), (2_u32, 2_u32)); // 'e'
        assert_eq!(entry.line_col(6_usize), (2_u32, 3_u32)); // 'f'
    }

    #[test]
    fn line_col_multiple_lines() {
        let entry = Entry::new("test".to_string(), "a\nb\nc".to_string());
        assert_eq!(entry.line_col(0_usize), (1_u32, 1_u32)); // 'a'
        assert_eq!(entry.line_col(2_usize), (2_u32, 1_u32)); // 'b'
        assert_eq!(entry.line_col(4_usize), (3_u32, 1_u32)); // 'c'
    }

    #[test]
    fn line_col_empty_content() {
        let entry = Entry::new("test".to_string(), String::new());
        // Beyond end of content, returns initial position
        assert_eq!(entry.line_col(0_usize), (1_u32, 1_u32));
    }

    #[test]
    fn line_col_beyond_content() {
        let entry = Entry::new("test".to_string(), "abc".to_string());
        // At end of content
        assert_eq!(entry.line_col(3_usize), (1_u32, 4_u32));
        // Beyond end - still reports position at end
        assert_eq!(entry.line_col(100_usize), (1_u32, 4_u32));
    }
}
