// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Metadata support for Lonala values.
//!
//! Metadata is a map of data about a value that does not affect
//! its equality or hash code. This follows Clojure's metadata semantics.
//!
//! # Implementing Types
//!
//! Types implementing [`Meta`] must ensure that:
//! 1. `PartialEq` ignores metadata
//! 2. `Hash` ignores metadata
//! 3. `with_meta` preserves structural sharing where possible

use crate::map::Map;

/// Trait for values that can carry metadata.
///
/// Implementing types must ensure that metadata does not affect equality
/// or hashing. The underlying data structure should be shared (not deep
/// cloned) when attaching new metadata.
pub trait Meta: Sized {
    /// Returns the attached metadata, if any.
    fn meta(&self) -> Option<&Map>;

    /// Returns a new instance with the given metadata attached.
    ///
    /// The underlying data structure is shared (not deep cloned).
    /// Passing `None` clears metadata.
    #[must_use]
    fn with_meta(self, meta: Option<Map>) -> Self;
}
