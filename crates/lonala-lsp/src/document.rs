// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Document management for the LSP server.
//!
//! This module tracks open files and provides access to their content
//! and line indices.

use dashmap::DashMap;
use lonala_human::LineIndex;
use tower_lsp::lsp_types::Url;

/// A managed document with content and line index.
#[non_exhaustive]
pub struct Document {
    /// Document content.
    pub content: String,
    /// Line index for offset conversion.
    pub line_index: LineIndex,
    /// Document version (from editor).
    pub version: i32,
}

impl Document {
    /// Creates a new document with the given content and version.
    #[inline]
    #[must_use]
    pub fn new(content: String, version: i32) -> Self {
        let line_index = LineIndex::new(&content);
        Self {
            content,
            line_index,
            version,
        }
    }
}

/// Thread-safe document storage.
pub struct Manager {
    documents: DashMap<Url, Document>,
}

impl Manager {
    /// Creates a new empty document manager.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        Self {
            documents: DashMap::new(),
        }
    }

    /// Opens a document with the given content.
    #[inline]
    pub fn open(&self, uri: Url, content: String, version: i32) {
        self.documents.insert(uri, Document::new(content, version));
    }

    /// Updates a document with new content.
    #[inline]
    pub fn update(&self, uri: &Url, content: String, version: i32) {
        if let Some(mut doc) = self.documents.get_mut(uri) {
            doc.line_index = LineIndex::new(&content);
            doc.content = content;
            doc.version = version;
        }
    }

    /// Closes a document.
    #[inline]
    pub fn close(&self, uri: &Url) {
        self.documents.remove(uri);
    }

    /// Gets a reference to a document.
    #[inline]
    #[must_use]
    pub fn get(&self, uri: &Url) -> Option<dashmap::mapref::one::Ref<'_, Url, Document>> {
        self.documents.get(uri)
    }
}

impl Default for Manager {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
