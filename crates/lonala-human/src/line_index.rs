// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Line index for converting byte offsets to line/column positions.
//!
//! This module provides efficient conversion between byte offsets in source
//! text and human-readable line/column positions for error messages.

use alloc::vec::Vec;

/// Line and column position.
///
/// Both line and column are 0-indexed internally but should be displayed
/// as 1-indexed (line 1, column 1) for human consumption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct LineCol {
    /// Line number (0-indexed).
    pub line: u32,
    /// Column number (0-indexed, in bytes).
    pub column: u32,
}

impl LineCol {
    /// Creates a new line/column position.
    #[inline]
    #[must_use]
    pub const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }

    /// Returns the 1-indexed line number for display.
    #[inline]
    #[must_use]
    pub const fn display_line(self) -> u32 {
        self.line.saturating_add(1_u32)
    }

    /// Returns the 1-indexed column number for display.
    #[inline]
    #[must_use]
    pub const fn display_column(self) -> u32 {
        self.column.saturating_add(1_u32)
    }
}

/// Index for converting byte offsets to line/column positions.
///
/// The index pre-computes the byte offset of each line start, enabling
/// O(log n) lookup of line/column positions.
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// Byte offsets of line starts (0 is always the first entry).
    line_starts: Vec<usize>,
}

impl LineIndex {
    /// Creates a new line index for the given source text.
    ///
    /// This scans the text once to find all line boundaries.
    #[inline]
    #[must_use]
    pub fn new(text: &str) -> Self {
        let mut line_starts = Vec::new();
        line_starts.push(0_usize);

        let bytes = text.as_bytes();
        for (offset, &byte) in bytes.iter().enumerate() {
            if byte == b'\n' {
                line_starts.push(offset.saturating_add(1_usize));
            }
        }

        Self { line_starts }
    }

    /// Converts a byte offset to line/column position.
    ///
    /// Returns `None` if the offset is beyond the end of the text.
    ///
    /// Note: Column is measured in bytes, not characters. For ASCII text
    /// this is equivalent to character count, but for UTF-8 text with
    /// multi-byte characters, the column number may be higher than the
    /// visual column.
    #[inline]
    #[must_use]
    pub fn offset_to_line_col(&self, offset: usize) -> Option<LineCol> {
        // Binary search to find the line containing this offset
        let line = match self.line_starts.binary_search(&offset) {
            Ok(exact) => exact,
            Err(insertion_point) => insertion_point.saturating_sub(1_usize),
        };

        let line_start = self.line_starts.get(line)?;
        let column = offset.saturating_sub(*line_start);

        Some(LineCol::new(
            u32::try_from(line).ok()?,
            u32::try_from(column).ok()?,
        ))
    }

    /// Returns the content of a specific line.
    ///
    /// The returned string does not include the trailing newline.
    #[inline]
    #[must_use]
    pub fn line_content<'source>(&self, text: &'source str, line: u32) -> Option<&'source str> {
        let line_idx = usize::try_from(line).ok()?;
        let start = *self.line_starts.get(line_idx)?;
        let end = self
            .line_starts
            .get(line_idx.saturating_add(1_usize))
            .map_or(text.len(), |&next_start| {
                // Exclude the newline character
                next_start.saturating_sub(1_usize).max(start)
            });

        text.get(start..end)
    }

    /// Returns the number of lines in the indexed text.
    #[inline]
    #[must_use]
    pub const fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    /// Returns the byte offset of a line start.
    #[inline]
    #[must_use]
    pub fn line_start(&self, line: u32) -> Option<usize> {
        self.line_starts.get(usize::try_from(line).ok()?).copied()
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use super::*;

    #[test]
    fn line_col_new() {
        let pos = LineCol::new(2_u32, 5_u32);
        assert_eq!(pos.line, 2_u32);
        assert_eq!(pos.column, 5_u32);
    }

    #[test]
    fn line_col_display() {
        let pos = LineCol::new(0_u32, 0_u32);
        assert_eq!(pos.display_line(), 1_u32);
        assert_eq!(pos.display_column(), 1_u32);

        let pos = LineCol::new(4_u32, 10_u32);
        assert_eq!(pos.display_line(), 5_u32);
        assert_eq!(pos.display_column(), 11_u32);
    }

    #[test]
    fn empty_text() {
        let index = LineIndex::new("");
        assert_eq!(index.line_count(), 1_usize);
        assert_eq!(index.offset_to_line_col(0_usize), Some(LineCol::new(0, 0)));
        assert_eq!(index.line_content("", 0_u32), Some(""));
    }

    #[test]
    fn single_line_no_newline() {
        let text = "hello world";
        let index = LineIndex::new(text);

        assert_eq!(index.line_count(), 1_usize);
        assert_eq!(index.offset_to_line_col(0_usize), Some(LineCol::new(0, 0)));
        assert_eq!(index.offset_to_line_col(5_usize), Some(LineCol::new(0, 5)));
        assert_eq!(
            index.offset_to_line_col(11_usize),
            Some(LineCol::new(0, 11))
        );
        assert_eq!(index.line_content(text, 0_u32), Some("hello world"));
    }

    #[test]
    fn single_line_with_newline() {
        let text = "hello\n";
        let index = LineIndex::new(text);

        assert_eq!(index.line_count(), 2_usize);
        assert_eq!(index.offset_to_line_col(0_usize), Some(LineCol::new(0, 0)));
        assert_eq!(index.offset_to_line_col(5_usize), Some(LineCol::new(0, 5)));
        // Offset 6 is on line 1 (the empty line after newline)
        assert_eq!(index.offset_to_line_col(6_usize), Some(LineCol::new(1, 0)));
        assert_eq!(index.line_content(text, 0_u32), Some("hello"));
        assert_eq!(index.line_content(text, 1_u32), Some(""));
    }

    #[test]
    fn multiple_lines() {
        let text = "line1\nline2\nline3";
        let index = LineIndex::new(text);

        assert_eq!(index.line_count(), 3_usize);

        // Line 0
        assert_eq!(index.offset_to_line_col(0_usize), Some(LineCol::new(0, 0)));
        assert_eq!(index.offset_to_line_col(4_usize), Some(LineCol::new(0, 4)));
        assert_eq!(index.line_content(text, 0_u32), Some("line1"));

        // Line 1
        assert_eq!(index.offset_to_line_col(6_usize), Some(LineCol::new(1, 0)));
        assert_eq!(index.offset_to_line_col(10_usize), Some(LineCol::new(1, 4)));
        assert_eq!(index.line_content(text, 1_u32), Some("line2"));

        // Line 2
        assert_eq!(index.offset_to_line_col(12_usize), Some(LineCol::new(2, 0)));
        assert_eq!(index.offset_to_line_col(16_usize), Some(LineCol::new(2, 4)));
        assert_eq!(index.line_content(text, 2_u32), Some("line3"));
    }

    #[test]
    fn offset_at_newline() {
        let text = "ab\ncd";
        let index = LineIndex::new(text);

        // Offset 2 is the newline character itself (still on line 0)
        assert_eq!(index.offset_to_line_col(2_usize), Some(LineCol::new(0, 2)));
        // Offset 3 is the first character of line 1
        assert_eq!(index.offset_to_line_col(3_usize), Some(LineCol::new(1, 0)));
    }

    #[test]
    fn empty_lines() {
        let text = "a\n\nb";
        let index = LineIndex::new(text);

        assert_eq!(index.line_count(), 3_usize);
        assert_eq!(index.line_content(text, 0_u32), Some("a"));
        assert_eq!(index.line_content(text, 1_u32), Some(""));
        assert_eq!(index.line_content(text, 2_u32), Some("b"));

        // Offset 2 is on the empty line
        assert_eq!(index.offset_to_line_col(2_usize), Some(LineCol::new(1, 0)));
    }

    #[test]
    fn line_content_out_of_bounds() {
        let text = "single line";
        let index = LineIndex::new(text);

        assert_eq!(index.line_content(text, 0_u32), Some("single line"));
        assert_eq!(index.line_content(text, 1_u32), None);
        assert_eq!(index.line_content(text, 100_u32), None);
    }

    #[test]
    fn line_start() {
        let text = "abc\ndef\nghi";
        let index = LineIndex::new(text);

        assert_eq!(index.line_start(0_u32), Some(0_usize));
        assert_eq!(index.line_start(1_u32), Some(4_usize));
        assert_eq!(index.line_start(2_u32), Some(8_usize));
        assert_eq!(index.line_start(3_u32), None);
    }

    #[test]
    fn unicode_text() {
        // UTF-8 text: "hello" in Japanese followed by newline and more text
        // Each character is 3 bytes in UTF-8
        let text = "\u{3053}\u{3093}\u{306B}\u{3061}\u{306F}\nworld";
        let index = LineIndex::new(text);

        assert_eq!(index.line_count(), 2_usize);

        // First line starts at 0
        assert_eq!(index.offset_to_line_col(0_usize), Some(LineCol::new(0, 0)));

        // Each Japanese character is 3 bytes, so 5 characters = 15 bytes
        // The newline is at offset 15
        assert_eq!(
            index.offset_to_line_col(15_usize),
            Some(LineCol::new(0, 15))
        );

        // "world" starts at offset 16
        assert_eq!(index.offset_to_line_col(16_usize), Some(LineCol::new(1, 0)));

        // Line content extraction
        assert_eq!(
            index.line_content(text, 0_u32),
            Some("\u{3053}\u{3093}\u{306B}\u{3061}\u{306F}")
        );
        assert_eq!(index.line_content(text, 1_u32), Some("world"));
    }

    #[test]
    fn windows_line_endings() {
        // Windows uses \r\n, but we only split on \n
        // The \r will be included in the line content
        let text = "line1\r\nline2";
        let index = LineIndex::new(text);

        assert_eq!(index.line_count(), 2_usize);
        // Line 0 includes the \r
        assert_eq!(index.line_content(text, 0_u32), Some("line1\r"));
        assert_eq!(index.line_content(text, 1_u32), Some("line2"));
    }

    #[test]
    fn trailing_newlines() {
        let text = "a\n\n\n";
        let index = LineIndex::new(text);

        assert_eq!(index.line_count(), 4_usize);
        assert_eq!(index.line_content(text, 0_u32), Some("a"));
        assert_eq!(index.line_content(text, 1_u32), Some(""));
        assert_eq!(index.line_content(text, 2_u32), Some(""));
        assert_eq!(index.line_content(text, 3_u32), Some(""));
    }
}
