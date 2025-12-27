// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Source code loading abstraction for namespace imports.
//!
//! The [`SourceLoader`] trait decouples namespace loading from storage,
//! allowing different implementations for embedded sources (bootstrap),
//! filesystem access (future), and testing.

use alloc::collections::BTreeMap;
use alloc::string::String;

/// Trait for loading namespace source code.
///
/// Implementations provide source code given a namespace name. This allows
/// the runtime to load namespaces from different sources:
///
/// - [`MemorySourceLoader`]: In-memory sources for bundled/embedded code
/// - Future: Filesystem-based loader for runtime code loading
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` to allow the loader to be shared
/// across the runtime (though currently single-threaded, this prepares for
/// future multi-domain scenarios).
pub trait SourceLoader: Send + Sync {
    /// Load source code for a namespace.
    ///
    /// Returns `Some(&str)` with the source code if the namespace is found,
    /// or `None` if the namespace source is not available.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace name (e.g., "lona.core", "user.utils")
    fn load_source(&self, namespace: &str) -> Option<&str>;
}

/// In-memory source loader for bundled/embedded sources.
///
/// This loader stores namespace sources in memory, useful for:
/// - Bootstrap code that must be available before filesystem access
/// - Testing scenarios with controlled source content
/// - Embedded systems without filesystem support
///
/// # Example
///
/// ```
/// use lona_kernel::namespace::{MemorySourceLoader, SourceLoader};
///
/// let mut loader = MemorySourceLoader::new();
/// loader.add("my.namespace".into(), "(def x 42)".into());
///
/// assert_eq!(loader.load_source("my.namespace"), Some("(def x 42)"));
/// assert_eq!(loader.load_source("unknown"), None);
/// ```
#[derive(Debug, Default)]
pub struct MemorySourceLoader {
    /// Namespace name → source code.
    sources: BTreeMap<String, String>,
}

impl MemorySourceLoader {
    /// Creates a new empty memory source loader.
    #[inline]
    #[must_use]
    pub const fn new() -> Self {
        Self {
            sources: BTreeMap::new(),
        }
    }

    /// Adds a namespace source to the loader.
    ///
    /// If the namespace already exists, its source is replaced.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace name (e.g., "lona.core")
    /// * `source` - The complete source code for the namespace
    #[inline]
    pub fn add(&mut self, namespace: String, source: String) {
        let _previous = self.sources.insert(namespace, source);
    }

    /// Returns the number of namespaces in the loader.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Returns true if no namespaces are loaded.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Returns an iterator over namespace names.
    #[inline]
    pub fn namespaces(&self) -> impl Iterator<Item = &str> {
        self.sources.keys().map(String::as_str)
    }
}

impl SourceLoader for MemorySourceLoader {
    #[inline]
    fn load_source(&self, namespace: &str) -> Option<&str> {
        self.sources.get(namespace).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::*;

    #[test]
    fn test_memory_loader_new_is_empty() {
        let loader = MemorySourceLoader::new();
        assert!(loader.is_empty());
        assert_eq!(loader.len(), 0);
    }

    #[test]
    fn test_memory_loader_add_and_load() {
        let mut loader = MemorySourceLoader::new();
        loader.add("my.namespace".into(), "(def x 42)".into());

        assert_eq!(loader.load_source("my.namespace"), Some("(def x 42)"));
        assert_eq!(loader.len(), 1);
        assert!(!loader.is_empty());
    }

    #[test]
    fn test_memory_loader_load_nonexistent() {
        let loader = MemorySourceLoader::new();
        assert_eq!(loader.load_source("nonexistent"), None);
    }

    #[test]
    fn test_memory_loader_replace_source() {
        let mut loader = MemorySourceLoader::new();
        loader.add("my.namespace".into(), "(def x 1)".into());
        loader.add("my.namespace".into(), "(def x 2)".into());

        assert_eq!(loader.load_source("my.namespace"), Some("(def x 2)"));
        assert_eq!(loader.len(), 1);
    }

    #[test]
    fn test_memory_loader_multiple_namespaces() {
        let mut loader = MemorySourceLoader::new();
        loader.add("ns.one".into(), "(def a 1)".into());
        loader.add("ns.two".into(), "(def b 2)".into());
        loader.add("ns.three".into(), "(def c 3)".into());

        assert_eq!(loader.load_source("ns.one"), Some("(def a 1)"));
        assert_eq!(loader.load_source("ns.two"), Some("(def b 2)"));
        assert_eq!(loader.load_source("ns.three"), Some("(def c 3)"));
        assert_eq!(loader.len(), 3);
    }

    #[test]
    fn test_memory_loader_namespaces_iterator() {
        let mut loader = MemorySourceLoader::new();
        loader.add("a.ns".into(), "".into());
        loader.add("b.ns".into(), "".into());

        let names: alloc::vec::Vec<_> = loader.namespaces().collect();
        assert_eq!(names.len(), 2);
        // BTreeMap maintains sorted order
        assert_eq!(names, vec!["a.ns", "b.ns"]);
    }

    #[test]
    fn test_source_loader_trait_object() {
        let mut loader = MemorySourceLoader::new();
        loader.add("test".into(), "(+ 1 2)".into());

        // Verify it works as a trait object
        let trait_obj: &dyn SourceLoader = &loader;
        assert_eq!(trait_obj.load_source("test"), Some("(+ 1 2)"));
        assert_eq!(trait_obj.load_source("missing"), None);
    }

    #[test]
    fn test_memory_loader_default() {
        let loader = MemorySourceLoader::default();
        assert!(loader.is_empty());
    }
}
