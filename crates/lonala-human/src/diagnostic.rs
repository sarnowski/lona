// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Diagnostic trait and supporting types for error formatting.
//!
//! This module defines the interface that error types must implement to be
//! formatted as human-readable diagnostics. All actual formatting is done
//! by the `format` module using these types.

use alloc::string::String;
use alloc::vec::Vec;

use lona_core::source::Location as SourceLocation;
use lona_core::symbol::Interner;

/// Error severity level.
///
/// Determines the visual presentation and prefix of the diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Severity {
    /// A hard error that prevents compilation or execution.
    Error,
    /// A warning about potentially problematic code.
    Warning,
    /// An informational note attached to another diagnostic.
    Note,
}

impl Severity {
    /// Returns the display prefix for this severity.
    #[inline]
    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

/// Additional context attached to an error.
///
/// Notes provide supplementary information like suggestions, explanations,
/// or references to related locations in the code.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Note {
    /// Plain text note with explanatory information.
    ///
    /// Displayed as: `= note: {text}`
    Text(String),

    /// A help suggestion for fixing the error.
    ///
    /// Displayed as: `= help: {text}`
    Help(String),

    /// Reference to where something was defined.
    ///
    /// Displayed with source context at the referenced location.
    DefinedAt {
        /// Description of what was defined (e.g., "function 'foo' defined here").
        description: String,
        /// Location where the definition occurs.
        location: SourceLocation,
    },
}

impl Note {
    /// Creates a plain text note from a string.
    #[inline]
    #[must_use]
    pub const fn text(message: String) -> Self {
        Self::Text(message)
    }

    /// Creates a plain text note from a static string.
    #[inline]
    #[must_use]
    pub fn text_static(message: &'static str) -> Self {
        Self::Text(String::from(message))
    }

    /// Creates a help suggestion from a string.
    #[inline]
    #[must_use]
    pub const fn help(message: String) -> Self {
        Self::Help(message)
    }

    /// Creates a help suggestion from a static string.
    #[inline]
    #[must_use]
    pub fn help_static(message: &'static str) -> Self {
        Self::Help(String::from(message))
    }

    /// Creates a "defined at" reference.
    #[inline]
    #[must_use]
    pub const fn defined_at(description: String, location: SourceLocation) -> Self {
        Self::DefinedAt {
            description,
            location,
        }
    }
}

/// Trait for errors that can be formatted as diagnostics.
///
/// This trait provides the interface between structured error types and the
/// formatting system. Error types implement this trait to provide all the
/// information needed to generate Rust-style error messages.
///
/// # Design Notes
///
/// - Errors should NOT implement `Display` themselves
/// - All human-readable text generation happens through this trait
/// - The `interner` parameter enables resolving symbol IDs to names
pub trait Diagnostic {
    /// Returns the source location of this error.
    ///
    /// This is the primary location that will be highlighted in the output.
    fn location(&self) -> SourceLocation;

    /// Returns the error severity.
    ///
    /// Most errors return `Severity::Error`. Warnings are rare in the
    /// current implementation.
    fn severity(&self) -> Severity;

    /// Returns the variant name for error identification.
    ///
    /// This appears in brackets after the severity, e.g., `error[TypeError]`.
    /// The variant name should be stable and searchable.
    fn variant_name(&self) -> &'static str;

    /// Generates the primary error message.
    ///
    /// This is the main description that appears after the error identifier.
    /// Symbol IDs should be resolved to names using the provided interner.
    ///
    /// # Example
    ///
    /// For an undefined symbol error, this might return:
    /// `"undefined symbol 'fooo'"`
    fn message(&self, interner: &Interner) -> String;

    /// Generates additional notes, suggestions, and help text.
    ///
    /// Notes appear after the source context and provide additional
    /// information to help the user understand and fix the error.
    ///
    /// Returns an empty vector if there are no additional notes.
    fn notes(&self, interner: &Interner) -> Vec<Note>;
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::string::ToString;

    use lona_core::source::{Id as SourceId, Location};
    use lona_core::span::Span;

    use super::*;

    #[test]
    fn severity_prefix() {
        assert_eq!(Severity::Error.prefix(), "error");
        assert_eq!(Severity::Warning.prefix(), "warning");
        assert_eq!(Severity::Note.prefix(), "note");
    }

    #[test]
    fn note_text() {
        let note = Note::text("expected numeric type".to_string());
        assert_eq!(note, Note::Text("expected numeric type".to_string()));
    }

    #[test]
    fn note_text_static() {
        let note = Note::text_static("expected numeric type");
        assert_eq!(note, Note::Text("expected numeric type".to_string()));
    }

    #[test]
    fn note_help() {
        let note = Note::help("did you mean 'foo'?".to_string());
        assert_eq!(note, Note::Help("did you mean 'foo'?".to_string()));
    }

    #[test]
    fn note_help_static() {
        let note = Note::help_static("did you mean 'foo'?");
        assert_eq!(note, Note::Help("did you mean 'foo'?".to_string()));
    }

    #[test]
    fn note_defined_at() {
        let location = Location::new(SourceId::new(0_u32), Span::new(10_usize, 20_usize));
        let note = Note::defined_at("function defined here".to_string(), location);
        assert_eq!(
            note,
            Note::DefinedAt {
                description: "function defined here".to_string(),
                location,
            }
        );
    }
}
