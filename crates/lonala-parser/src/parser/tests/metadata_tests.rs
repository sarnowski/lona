// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for metadata reader syntax (`^`).

extern crate alloc;

use crate::ast::Ast;

use super::parse_ast;

// ==================== Basic Metadata Syntax ====================

#[test]
fn parse_metadata_with_map() {
    let ast = parse_ast("^{:a 1} x");
    match ast {
        Ast::WithMeta {
            ref meta,
            ref value,
        } => {
            assert!(matches!(meta.node, Ast::Map(_)));
            assert!(matches!(value.node, Ast::Symbol(_)));
        }
        _ => panic!("expected WithMeta, got {ast:?}"),
    }
}

#[test]
fn parse_metadata_keyword_shorthand() {
    let ast = parse_ast("^:private x");
    match ast {
        Ast::WithMeta {
            ref meta,
            ref value,
        } => {
            if let Ast::Map(ref pairs) = meta.node {
                assert_eq!(pairs.len(), 2_usize);
                assert!(
                    matches!(&pairs.first().unwrap().node, Ast::Keyword(kw) if kw == "private")
                );
                assert!(matches!(&pairs.get(1_usize).unwrap().node, Ast::Bool(true)));
            } else {
                panic!("expected Map in meta");
            }
            assert!(matches!(value.node, Ast::Symbol(ref sym) if sym == "x"));
        }
        _ => panic!("expected WithMeta"),
    }
}

#[test]
fn parse_metadata_multiple_keywords() {
    let ast = parse_ast("^:a ^:b x");
    match ast {
        Ast::WithMeta {
            ref meta,
            ref value,
        } => {
            if let Ast::Map(ref pairs) = meta.node {
                // Should have 4 elements: :a true :b true
                assert_eq!(pairs.len(), 4_usize);
            } else {
                panic!("expected Map in meta");
            }
            assert!(matches!(value.node, Ast::Symbol(_)));
        }
        _ => panic!("expected WithMeta"),
    }
}

#[test]
fn parse_metadata_override_duplicate_key() {
    let ast = parse_ast("^{:a 1} ^{:a 2} x");
    match ast {
        Ast::WithMeta { ref meta, .. } => {
            if let Ast::Map(ref pairs) = meta.node {
                // Should have 2 elements after dedup: :a 2
                assert_eq!(pairs.len(), 2_usize);
                assert!(matches!(
                    &pairs.get(1_usize).unwrap().node,
                    Ast::Integer(2_i64)
                ));
            } else {
                panic!("expected Map in meta");
            }
        }
        _ => panic!("expected WithMeta"),
    }
}

#[test]
fn parse_metadata_on_vector() {
    let ast = parse_ast("^{:doc \"test\"} [1 2 3]");
    match ast {
        Ast::WithMeta {
            ref meta,
            ref value,
        } => {
            assert!(matches!(meta.node, Ast::Map(_)));
            assert!(matches!(value.node, Ast::Vector(_)));
        }
        _ => panic!("expected WithMeta"),
    }
}

#[test]
fn parse_metadata_on_quoted_form() {
    let ast = parse_ast("^:tag 'x");
    match ast {
        Ast::WithMeta { ref value, .. } => {
            // value should be (quote x)
            assert!(matches!(value.node, Ast::List(_)));
        }
        _ => panic!("expected WithMeta"),
    }
}

#[test]
fn parse_metadata_nested_in_vector() {
    let ast = parse_ast("[^:a x ^:b y]");
    if let Ast::Vector(ref elements) = ast {
        assert_eq!(elements.len(), 2_usize);
        assert!(matches!(
            elements.first().unwrap().node,
            Ast::WithMeta { .. }
        ));
        assert!(matches!(
            elements.get(1_usize).unwrap().node,
            Ast::WithMeta { .. }
        ));
    } else {
        panic!("expected Vector");
    }
}

#[test]
fn parse_metadata_empty_map() {
    let ast = parse_ast("^{} x");
    match ast {
        Ast::WithMeta { ref meta, .. } => {
            if let Ast::Map(ref pairs) = meta.node {
                assert!(pairs.is_empty());
            } else {
                panic!("expected Map in meta");
            }
        }
        _ => panic!("expected WithMeta"),
    }
}

// ==================== Metadata Display ====================

#[test]
fn display_metadata_with_map() {
    let ast = parse_ast("^{:a 1} x");
    let display = alloc::format!("{ast}");
    assert_eq!(display, "^{:a 1} x");
}

#[test]
fn display_metadata_keyword_shorthand() {
    // The keyword shorthand expands to a map internally
    let ast = parse_ast("^:private x");
    let display = alloc::format!("{ast}");
    assert_eq!(display, "^{:private true} x");
}

// ==================== Metadata Type Name ====================

#[test]
fn type_name_with_meta() {
    let ast = parse_ast("^:foo x");
    assert_eq!(ast.type_name(), "with-meta");
}

// ==================== Mixed Metadata Forms ====================

#[test]
fn parse_metadata_mixed_forms() {
    // Mix of keyword shorthand and map
    let ast = parse_ast("^:private ^{:doc \"hi\"} x");
    match ast {
        Ast::WithMeta { ref meta, .. } => {
            if let Ast::Map(ref pairs) = meta.node {
                // Should have 4 elements: :private true :doc "hi"
                assert_eq!(pairs.len(), 4_usize);
            } else {
                panic!("expected Map in meta");
            }
        }
        _ => panic!("expected WithMeta"),
    }
}

#[test]
fn parse_metadata_on_list() {
    let ast = parse_ast("^:pure (fn [x] x)");
    match ast {
        Ast::WithMeta {
            ref meta,
            ref value,
        } => {
            assert!(matches!(meta.node, Ast::Map(_)));
            assert!(matches!(value.node, Ast::List(_)));
        }
        _ => panic!("expected WithMeta"),
    }
}

#[test]
fn parse_metadata_on_map() {
    let ast = parse_ast("^:const {:a 1}");
    match ast {
        Ast::WithMeta {
            ref meta,
            ref value,
        } => {
            assert!(matches!(meta.node, Ast::Map(_)));
            assert!(matches!(value.node, Ast::Map(_)));
        }
        _ => panic!("expected WithMeta"),
    }
}
