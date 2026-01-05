// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the loader module.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

// =============================================================================
// TarSource basic tests
// =============================================================================

#[test]
fn embedded_archive_loads() {
    let source = TarSource::embedded().expect("embedded archive should load");
    assert!(
        source.entries().next().is_some(),
        "archive should contain files"
    );
}

#[test]
fn contains_core_lona() {
    let source = TarSource::embedded().expect("embedded archive should load");
    let has_core = source.entries().any(|entry| {
        entry
            .filename()
            .as_str()
            .is_ok_and(|name| name == "lona/core.lona" || name == "./lona/core.lona")
    });
    assert!(has_core, "archive should contain lona/core.lona");
}

// =============================================================================
// namespace_matches_path tests
// =============================================================================

#[test]
fn namespace_matches_simple() {
    assert!(namespace_matches_path("lona.core", "lona/core.lona"));
}

#[test]
fn namespace_matches_with_dot_prefix() {
    assert!(namespace_matches_path("lona.core", "./lona/core.lona"));
}

#[test]
fn namespace_matches_nested() {
    assert!(namespace_matches_path(
        "my.app.server",
        "my/app/server.lona"
    ));
}

#[test]
fn namespace_matches_single_segment() {
    assert!(namespace_matches_path("core", "core.lona"));
}

#[test]
fn namespace_no_match_different_name() {
    assert!(!namespace_matches_path("lona.core", "lona/other.lona"));
}

#[test]
fn namespace_no_match_missing_extension() {
    assert!(!namespace_matches_path("lona.core", "lona/core"));
}

#[test]
fn namespace_no_match_wrong_extension() {
    assert!(!namespace_matches_path("lona.core", "lona/core.txt"));
}

#[test]
fn namespace_no_match_directory() {
    assert!(!namespace_matches_path("lona", "lona/"));
}

#[test]
fn namespace_no_match_partial() {
    assert!(!namespace_matches_path("lona.core", "lona/core/extra.lona"));
}

#[test]
fn namespace_no_match_dots_in_path() {
    // Dots in namespace must map to slashes, not to dots
    assert!(!namespace_matches_path("lona.core", "lona.core.lona"));
}

#[test]
fn namespace_no_match_empty() {
    assert!(!namespace_matches_path("", ".lona"));
}

// =============================================================================
// NamespaceSource trait tests
// =============================================================================

#[test]
fn tar_source_resolve_core() {
    let source = TarSource::embedded().expect("embedded archive should load");
    let bytes = source.resolve("lona.core");
    assert!(bytes.is_some(), "should resolve lona.core");
    assert!(!bytes.unwrap().is_empty(), "lona.core should not be empty");
}

#[test]
fn tar_source_resolve_nonexistent() {
    let source = TarSource::embedded().expect("embedded archive should load");
    let bytes = source.resolve("nonexistent.namespace");
    assert!(bytes.is_none(), "should not resolve nonexistent namespace");
}

#[test]
fn tar_source_resolve_init() {
    let source = TarSource::embedded().expect("embedded archive should load");
    let bytes = source.resolve("lona.init");
    assert!(bytes.is_some(), "should resolve lona.init");
}

// =============================================================================
// ChainedSource tests
// =============================================================================

/// Mock source for testing `ChainedSource`.
struct MockSource {
    namespace: &'static str,
    data: &'static [u8],
}

impl NamespaceSource for MockSource {
    fn resolve(&self, namespace: &str) -> Option<&[u8]> {
        if namespace == self.namespace {
            Some(self.data)
        } else {
            None
        }
    }
}

#[test]
fn chained_source_finds_in_first() {
    let first = MockSource {
        namespace: "test.one",
        data: b"first",
    };
    let second = MockSource {
        namespace: "test.two",
        data: b"second",
    };
    let sources: [&dyn NamespaceSource; 2] = [&first, &second];
    let chain = ChainedSource::new(&sources);

    assert_eq!(chain.resolve("test.one"), Some(b"first".as_slice()));
}

#[test]
fn chained_source_finds_in_second() {
    let first = MockSource {
        namespace: "test.one",
        data: b"first",
    };
    let second = MockSource {
        namespace: "test.two",
        data: b"second",
    };
    let sources: [&dyn NamespaceSource; 2] = [&first, &second];
    let chain = ChainedSource::new(&sources);

    assert_eq!(chain.resolve("test.two"), Some(b"second".as_slice()));
}

#[test]
fn chained_source_first_wins() {
    let first = MockSource {
        namespace: "test.same",
        data: b"first",
    };
    let second = MockSource {
        namespace: "test.same",
        data: b"second",
    };
    let sources: [&dyn NamespaceSource; 2] = [&first, &second];
    let chain = ChainedSource::new(&sources);

    assert_eq!(
        chain.resolve("test.same"),
        Some(b"first".as_slice()),
        "first source should win"
    );
}

#[test]
fn chained_source_not_found() {
    let first = MockSource {
        namespace: "test.one",
        data: b"first",
    };
    let sources: [&dyn NamespaceSource; 1] = [&first];
    let chain = ChainedSource::new(&sources);

    assert_eq!(chain.resolve("nonexistent"), None);
}

#[test]
fn chained_source_empty() {
    let sources: &[&dyn NamespaceSource] = &[];
    let chain = ChainedSource::new(sources);

    assert_eq!(chain.resolve("anything"), None);
}
