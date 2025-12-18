// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

// This module uses `.ok()` to discard `fmt::Result` from `writeln!` calls when writing
// to a `String`. Writing to a `String` is infallible (the `fmt::Write` impl for `String`
// never returns an error), so discarding the result is safe. We use `.ok()` rather than
// `let _ =` because `let_underscore_must_use` would flag that pattern, and these two
// lints conflict with each other.
#![expect(
    clippy::unused_result_ok,
    reason = "[approved] writeln! to String is infallible; .ok() explicitly acknowledges the Result"
)]

//! Error formatting system for Rust-style diagnostic output.
//!
//! This module provides the core formatting functionality that converts
//! structured errors into human-readable diagnostic messages with source
//! context, underlines, and helpful notes.
//!
//! # Output Format
//!
//! ```text
//! error[VariantName]: error message here
//!   --> source_name:1:5
//!    |
//!  1 |   (fooo 42)
//!    |    ^^^^
//!    |
//!    = help: did you mean 'foo'?
//! ```

use alloc::string::String;
use core::fmt::Write as _;

use lona_core::source::Registry as SourceRegistry;
use lona_core::symbol::Interner;

use crate::diagnostic::{Diagnostic, Note};
use crate::line_index::LineIndex;

/// Configuration for error formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Config {
    /// Number of context lines to show before the error line.
    pub context_before: u32,
    /// Number of context lines to show after the error line.
    pub context_after: u32,
}

impl Config {
    /// Creates a new format configuration with default settings.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            context_before: 0_u32,
            context_after: 0_u32,
        }
    }

    /// Sets the number of context lines before the error.
    #[inline]
    #[must_use]
    pub const fn with_context_before(mut self, lines: u32) -> Self {
        self.context_before = lines;
        self
    }

    /// Sets the number of context lines after the error.
    #[inline]
    #[must_use]
    pub const fn with_context_after(mut self, lines: u32) -> Self {
        self.context_after = lines;
        self
    }
}

impl Default for Config {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

/// Formats any diagnostic error to a string.
///
/// This is the main entry point for error formatting. It takes a structured
/// error and produces a Rust-style diagnostic message with source context.
///
/// # Arguments
///
/// * `error` - The error to format (implements `Diagnostic`)
/// * `registry` - Source registry for looking up source text
/// * `interner` - Symbol interner for resolving symbol IDs to names
/// * `config` - Formatting configuration
///
/// # Returns
///
/// A formatted error string ready for display.
#[inline]
#[must_use]
pub fn render<E: Diagnostic>(
    error: &E,
    registry: &SourceRegistry,
    interner: &Interner,
    config: &Config,
) -> String {
    let mut output = String::new();
    write_to(&mut output, error, registry, interner, config);
    output
}

/// Formats an error to an existing string buffer.
///
/// This is useful when you want to append multiple errors to a single buffer.
#[inline]
pub fn write_to<E: Diagnostic>(
    output: &mut String,
    error: &E,
    registry: &SourceRegistry,
    interner: &Interner,
    config: &Config,
) {
    // Get error information
    let severity = error.severity();
    let variant_name = error.variant_name();
    let message = error.message(interner);
    let location = error.location();
    let notes = error.notes(interner);

    // Format the header line
    // error[VariantName]: message
    // Writing to String is infallible, so we explicitly acknowledge the Result
    writeln!(output, "{}[{variant_name}]: {message}", severity.prefix(),).ok();

    // Try to get source information
    let source = registry.get(location.source);

    if let Some(source_entry) = source {
        // Build line index for the source
        let line_index = LineIndex::new(&source_entry.content);

        // Get start position
        if let Some(start_pos) = line_index.offset_to_line_col(location.span.start) {
            // Format location line
            //   --> source_name:line:column
            writeln!(
                output,
                "  --> {}:{}:{}",
                source_entry.name,
                start_pos.display_line(),
                start_pos.display_column()
            )
            .ok();

            // Calculate the width needed for line numbers
            let start_line = start_pos.line;
            let end_line = line_index
                .offset_to_line_col(location.span.end.saturating_sub(1_usize))
                .map_or(start_line, |pos| pos.line);

            // Calculate display range with context
            let first_display_line = start_line.saturating_sub(config.context_before);
            let last_display_line = end_line.saturating_add(config.context_after);

            // Calculate max line number width (at least 1)
            let max_line_num = last_display_line.saturating_add(1_u32);
            let line_num_width = calculate_line_num_width(max_line_num);

            // Print separator line
            writeln!(output, "{:line_num_width$} |", "").ok();

            // Print context lines before, error line(s), and context after
            let mut current_line = first_display_line;
            while current_line <= last_display_line {
                if let Some(line_content) =
                    line_index.line_content(&source_entry.content, current_line)
                {
                    // Print the source line
                    let line_num = current_line.saturating_add(1_u32);
                    writeln!(output, "{line_num:>line_num_width$} | {line_content}",).ok();

                    // If this line contains the error, print underline
                    if current_line >= start_line && current_line <= end_line {
                        let underline = generate_underline(
                            line_content,
                            &line_index,
                            current_line,
                            location.span.start,
                            location.span.end,
                        );
                        writeln!(output, "{:line_num_width$} | {underline}", "",).ok();
                    }
                }
                current_line = current_line.saturating_add(1_u32);
            }

            // Print separator line before notes
            if !notes.is_empty() {
                writeln!(output, "{:line_num_width$} |", "").ok();
            }

            // Format notes
            for note in &notes {
                format_note(output, note, line_num_width);
            }
        } else {
            // Couldn't get line/column, just show source name
            writeln!(output, "  --> {}", source_entry.name).ok();
            format_notes_without_context(output, &notes);
        }
    } else {
        // No source available
        writeln!(output, "  --> <unknown source>").ok();
        format_notes_without_context(output, &notes);
    }
}

/// Generates the underline string for a span within a line.
fn generate_underline(
    line_content: &str,
    line_index: &LineIndex,
    line_num: u32,
    span_start: usize,
    span_end: usize,
) -> String {
    let line_start = line_index.line_start(line_num).unwrap_or(0_usize);
    let line_end = line_start.saturating_add(line_content.len());

    // Calculate where the underline starts and ends within this line
    let underline_start = if span_start >= line_start {
        span_start.saturating_sub(line_start)
    } else {
        0_usize
    };

    let underline_end = if span_end <= line_end {
        span_end.saturating_sub(line_start)
    } else {
        line_content.len()
    };

    // Build the underline string
    let mut result = String::new();

    // Add leading spaces
    for (idx, ch) in line_content.chars().enumerate() {
        if idx >= underline_start {
            break;
        }
        // Use space for all characters, preserving visual alignment
        if ch == '\t' {
            result.push('\t');
        } else {
            result.push(' ');
        }
    }

    // Add carets for the span
    let caret_count = underline_end.saturating_sub(underline_start).max(1_usize);
    for _ in 0_usize..caret_count {
        result.push('^');
    }

    result
}

/// Formats a note.
#[inline]
fn format_note(output: &mut String, note: &Note, line_num_width: usize) {
    match *note {
        Note::Text(ref text) => {
            writeln!(output, "{:line_num_width$} = note: {text}", "",).ok();
        }
        Note::Help(ref text) => {
            writeln!(output, "{:line_num_width$} = help: {text}", "",).ok();
        }
        Note::DefinedAt {
            ref description, ..
        } => {
            // For now, just show the description without the source context
            // Full implementation would show the definition location too
            writeln!(output, "{:line_num_width$} = note: {description}", "",).ok();
        }
    }
}

/// Formats notes when no source context is available.
#[inline]
fn format_notes_without_context(output: &mut String, notes: &[Note]) {
    for note in notes {
        match *note {
            Note::Text(ref text) => {
                writeln!(output, "   = note: {text}").ok();
            }
            Note::Help(ref text) => {
                writeln!(output, "   = help: {text}").ok();
            }
            Note::DefinedAt {
                ref description, ..
            } => {
                writeln!(output, "   = note: {description}").ok();
            }
        }
    }
}

/// Calculates the width needed to display line numbers.
const fn calculate_line_num_width(max_line: u32) -> usize {
    if max_line == 0_u32 {
        return 1_usize;
    }
    let mut width = 0_usize;
    let mut num = max_line;
    while num > 0_u32 {
        width = width.saturating_add(1_usize);
        num = num.wrapping_div(10_u32);
    }
    width
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::string::ToString;
    use alloc::vec::Vec;

    use lona_core::source::{Id as SourceId, Location, Registry};
    use lona_core::span::Span;
    use lona_core::symbol::Interner;

    use super::*;
    use crate::diagnostic::Severity;

    /// Test implementation of `Diagnostic` for unit tests.
    struct TestError {
        location: Location,
        severity: Severity,
        variant: &'static str,
        message: String,
        notes: Vec<Note>,
    }

    impl TestError {
        fn new(location: Location, message: impl Into<String>) -> Self {
            Self {
                location,
                severity: Severity::Error,
                variant: "TestError",
                message: message.into(),
                notes: Vec::new(),
            }
        }

        fn with_help(mut self, help: impl Into<String>) -> Self {
            self.notes.push(Note::Help(help.into()));
            self
        }

        fn with_note(mut self, note: impl Into<String>) -> Self {
            self.notes.push(Note::Text(note.into()));
            self
        }
    }

    impl Diagnostic for TestError {
        fn location(&self) -> Location {
            self.location
        }

        fn severity(&self) -> Severity {
            self.severity
        }

        fn variant_name(&self) -> &'static str {
            self.variant
        }

        fn message(&self, _interner: &Interner) -> String {
            self.message.clone()
        }

        fn notes(&self, _interner: &Interner) -> Vec<Note> {
            self.notes.clone()
        }
    }

    #[test]
    fn format_config_default() {
        let config = Config::default();
        assert_eq!(config.context_before, 0_u32);
        assert_eq!(config.context_after, 0_u32);
    }

    #[test]
    fn format_config_builder() {
        let config = Config::new()
            .with_context_before(2_u32)
            .with_context_after(1_u32);
        assert_eq!(config.context_before, 2_u32);
        assert_eq!(config.context_after, 1_u32);
    }

    #[test]
    fn calculate_line_num_width_single_digit() {
        assert_eq!(calculate_line_num_width(1_u32), 1_usize);
        assert_eq!(calculate_line_num_width(9_u32), 1_usize);
    }

    #[test]
    fn calculate_line_num_width_double_digit() {
        assert_eq!(calculate_line_num_width(10_u32), 2_usize);
        assert_eq!(calculate_line_num_width(99_u32), 2_usize);
    }

    #[test]
    fn calculate_line_num_width_triple_digit() {
        assert_eq!(calculate_line_num_width(100_u32), 3_usize);
        assert_eq!(calculate_line_num_width(999_u32), 3_usize);
    }

    #[test]
    fn calculate_line_num_width_zero() {
        assert_eq!(calculate_line_num_width(0_u32), 1_usize);
    }

    #[test]
    fn format_simple_error() {
        let mut registry = Registry::new();
        let interner = Interner::new();
        let config = Config::new();

        let source_id = registry
            .add("<repl>".to_string(), "(fooo 42)".to_string())
            .expect("should add source");

        let location = Location::new(source_id, Span::new(1_usize, 5_usize));
        let error = TestError::new(location, "undefined symbol 'fooo'");

        let output = render(&error, &registry, &interner, &config);

        assert!(output.contains("error[TestError]: undefined symbol 'fooo'"));
        assert!(output.contains("--> <repl>:1:2"));
        assert!(output.contains("(fooo 42)"));
        assert!(output.contains("^^^^"));
    }

    #[test]
    fn format_error_with_help() {
        let mut registry = Registry::new();
        let interner = Interner::new();
        let config = Config::new();

        let source_id = registry
            .add("<repl>".to_string(), "(fooo 42)".to_string())
            .expect("should add source");

        let location = Location::new(source_id, Span::new(1_usize, 5_usize));
        let error =
            TestError::new(location, "undefined symbol 'fooo'").with_help("did you mean 'foo'?");

        let output = render(&error, &registry, &interner, &config);

        assert!(output.contains("= help: did you mean 'foo'?"));
    }

    #[test]
    fn format_error_with_note() {
        let mut registry = Registry::new();
        let interner = Interner::new();
        let config = Config::new();

        let source_id = registry
            .add("<repl>".to_string(), "(+ \"hello\" 5)".to_string())
            .expect("should add source");

        let location = Location::new(source_id, Span::new(0_usize, 14_usize));
        let error = TestError::new(location, "type error in '+'")
            .with_note("expected numeric type, got string");

        let output = render(&error, &registry, &interner, &config);

        assert!(output.contains("= note: expected numeric type, got string"));
    }

    #[test]
    fn format_multiline_error() {
        let mut registry = Registry::new();
        let interner = Interner::new();
        let config = Config::new().with_context_before(1_u32);

        let source = "(defn add [a b]\n  (+ a b))";
        let source_id = registry
            .add("<repl>".to_string(), source.to_string())
            .expect("should add source");

        // Error on line 2, at the '+'
        let location = Location::new(source_id, Span::new(18_usize, 19_usize));
        let error = TestError::new(location, "error on second line");

        let output = render(&error, &registry, &interner, &config);

        assert!(output.contains("--> <repl>:2:3"));
        // Context line should be shown
        assert!(output.contains("(defn add [a b]"));
        // Error line should be shown
        assert!(output.contains("(+ a b))"));
    }

    #[test]
    fn format_error_unknown_source() {
        let registry = Registry::new();
        let interner = Interner::new();
        let config = Config::new();

        // Use a source ID that doesn't exist in the registry
        let location = Location::new(SourceId::new(999_u32), Span::new(0_usize, 5_usize));
        let error = TestError::new(location, "error with unknown source");

        let output = render(&error, &registry, &interner, &config);

        assert!(output.contains("error[TestError]: error with unknown source"));
        assert!(output.contains("--> <unknown source>"));
    }

    #[test]
    fn generate_underline_at_start() {
        let line_content = "hello world";
        let line_index = LineIndex::new(line_content);
        let underline = generate_underline(line_content, &line_index, 0_u32, 0_usize, 5_usize);
        assert_eq!(underline, "^^^^^");
    }

    #[test]
    fn generate_underline_in_middle() {
        let line_content = "hello world";
        let line_index = LineIndex::new(line_content);
        let underline = generate_underline(line_content, &line_index, 0_u32, 6_usize, 11_usize);
        assert_eq!(underline, "      ^^^^^");
    }

    #[test]
    fn generate_underline_single_char() {
        let line_content = "x + y";
        let line_index = LineIndex::new(line_content);
        let underline = generate_underline(line_content, &line_index, 0_u32, 2_usize, 3_usize);
        assert_eq!(underline, "  ^");
    }

    #[test]
    fn format_error_with_multiple_notes() {
        let mut registry = Registry::new();
        let interner = Interner::new();
        let config = Config::new();

        let source_id = registry
            .add("<repl>".to_string(), "(- \"hello\" 5)".to_string())
            .expect("should add source");

        let location = Location::new(source_id, Span::new(0_usize, 13_usize));
        let error = TestError::new(location, "type error in '-'")
            .with_note("expected numeric type, got string")
            .with_note("'-' requires all arguments to be numbers");

        let output = render(&error, &registry, &interner, &config);

        assert!(output.contains("= note: expected numeric type, got string"));
        assert!(output.contains("= note: '-' requires all arguments to be numbers"));
    }
}
