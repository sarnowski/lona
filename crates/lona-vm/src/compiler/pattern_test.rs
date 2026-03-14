// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for pattern parsing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::pattern::Pattern;
use crate::Vaddr;
use crate::compiler::Compiler;
use crate::platform::MockVSpace;
use crate::process::Process;
use crate::reader::read;
use crate::realm::{Realm, bootstrap};
use crate::term::Term;

/// Create a test environment with bootstrapped realm and process.
fn setup() -> Option<(Process, Realm, MockVSpace)> {
    let base = Vaddr::new(0x1_0000);
    let mut mem = MockVSpace::new(512 * 1024, base);
    let mut realm = Realm::new_for_test(base).unwrap();

    let (young_base, old_base) = realm.allocate_process_memory(64 * 1024, 16 * 1024)?;
    let mut proc = Process::new(young_base, 64 * 1024, old_base, 16 * 1024);

    let result = bootstrap(&mut realm, &mut mem)?;
    proc.bootstrap(result.ns_var, result.core_ns);

    Some((proc, realm, mem))
}

/// Parse a pattern string and return the Pattern.
fn parse_pattern(
    src: &str,
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut MockVSpace,
) -> Pattern {
    let expr = read(src, proc, realm, mem)
        .expect("parse error")
        .expect("empty input");
    let compiler = Compiler::new(proc, mem, realm);
    compiler.parse_pattern(expr).expect("pattern parse error")
}

// --- Wildcard tests ---

#[test]
fn parse_wildcard() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("_", &mut proc, &mut realm, &mut mem);
    assert_eq!(pattern, Pattern::Wildcard);
}

// --- Binding tests ---

#[test]
fn parse_simple_binding() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("x", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Binding { name, name_len } => {
            assert_eq!(&name[..name_len as usize], b"x");
        }
        _ => panic!("expected Binding pattern"),
    }
}

#[test]
fn parse_longer_binding_name() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("my-variable", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Binding { name, name_len } => {
            assert_eq!(&name[..name_len as usize], b"my-variable");
        }
        _ => panic!("expected Binding pattern"),
    }
}

// --- Literal tests ---

#[test]
fn parse_literal_nil() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("nil", &mut proc, &mut realm, &mut mem);
    assert_eq!(pattern, Pattern::Literal(Term::NIL));
}

#[test]
fn parse_literal_true() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("true", &mut proc, &mut realm, &mut mem);
    assert_eq!(pattern, Pattern::Literal(Term::TRUE));
}

#[test]
fn parse_literal_false() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("false", &mut proc, &mut realm, &mut mem);
    assert_eq!(pattern, Pattern::Literal(Term::FALSE));
}

#[test]
fn parse_literal_integer() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("42", &mut proc, &mut realm, &mut mem);
    assert_eq!(pattern, Pattern::Literal(Term::small_int(42).unwrap()));
}

#[test]
fn parse_literal_negative_integer() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("-100", &mut proc, &mut realm, &mut mem);
    assert_eq!(pattern, Pattern::Literal(Term::small_int(-100).unwrap()));
}

#[test]
fn parse_literal_keyword() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern(":ok", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Literal(term) if term.is_keyword() => {}
        _ => panic!("expected Literal keyword pattern"),
    }
}

#[test]
fn parse_literal_string() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("\"hello\"", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Literal(term) if proc.is_term_string(&mem, term) => {}
        _ => panic!("expected Literal string pattern"),
    }
}

// --- Tuple pattern tests ---

#[test]
fn parse_empty_tuple() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("[]", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Tuple(elements) => {
            assert_eq!(elements.len(), 0);
        }
        _ => panic!("expected Tuple pattern"),
    }
}

#[test]
fn parse_tuple_with_bindings() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("[a b]", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Tuple(elements) => {
            assert_eq!(elements.len(), 2);
            assert!(matches!(&elements[0], Pattern::Binding { .. }));
            assert!(matches!(&elements[1], Pattern::Binding { .. }));
            assert_eq!(elements[0].binding_name(), Some(b"a".as_slice()));
            assert_eq!(elements[1].binding_name(), Some(b"b".as_slice()));
        }
        _ => panic!("expected Tuple pattern"),
    }
}

#[test]
fn parse_tuple_with_literals() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("[1 :ok true]", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Tuple(elements) => {
            assert_eq!(elements.len(), 3);
            match &elements[0] {
                Pattern::Literal(term) => assert_eq!(*term, Term::small_int(1).unwrap()),
                _ => panic!("expected literal"),
            }
            match &elements[1] {
                Pattern::Literal(term) => assert!(term.is_keyword()),
                _ => panic!("expected keyword literal"),
            }
            match &elements[2] {
                Pattern::Literal(term) => assert_eq!(*term, Term::TRUE),
                _ => panic!("expected true literal"),
            }
        }
        _ => panic!("expected Tuple pattern"),
    }
}

#[test]
fn parse_tuple_mixed() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("[:ok x]", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Tuple(elements) => {
            assert_eq!(elements.len(), 2);
            match &elements[0] {
                Pattern::Literal(term) => assert!(term.is_keyword()),
                _ => panic!("expected keyword literal"),
            }
            assert!(matches!(&elements[1], Pattern::Binding { .. }));
        }
        _ => panic!("expected Tuple pattern"),
    }
}

#[test]
fn parse_tuple_with_wildcard() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("[a _ c]", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Tuple(elements) => {
            assert_eq!(elements.len(), 3);
            assert!(matches!(&elements[0], Pattern::Binding { .. }));
            assert_eq!(elements[1], Pattern::Wildcard);
            assert!(matches!(&elements[2], Pattern::Binding { .. }));
        }
        _ => panic!("expected Tuple pattern"),
    }
}

#[test]
fn parse_nested_tuple() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("[[a b] c]", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Tuple(outer) => {
            assert_eq!(outer.len(), 2);
            match &outer[0] {
                Pattern::Tuple(inner) => {
                    assert_eq!(inner.len(), 2);
                    assert_eq!(inner[0].binding_name(), Some(b"a".as_slice()));
                    assert_eq!(inner[1].binding_name(), Some(b"b".as_slice()));
                }
                _ => panic!("expected inner Tuple pattern"),
            }
            assert_eq!(outer[1].binding_name(), Some(b"c".as_slice()));
        }
        _ => panic!("expected Tuple pattern"),
    }
}

// --- Tuple rest pattern tests ---

#[test]
fn parse_tuple_rest() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("[h & t]", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::TupleRest { head, rest } => {
            assert_eq!(head.len(), 1);
            assert_eq!(head[0].binding_name(), Some(b"h".as_slice()));
            assert_eq!(rest.binding_name(), Some(b"t".as_slice()));
        }
        _ => panic!("expected TupleRest pattern"),
    }
}

#[test]
fn parse_tuple_rest_multiple_head() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("[a b & rest]", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::TupleRest { head, rest } => {
            assert_eq!(head.len(), 2);
            assert_eq!(head[0].binding_name(), Some(b"a".as_slice()));
            assert_eq!(head[1].binding_name(), Some(b"b".as_slice()));
            assert_eq!(rest.binding_name(), Some(b"rest".as_slice()));
        }
        _ => panic!("expected TupleRest pattern"),
    }
}

#[test]
fn parse_tuple_rest_with_wildcard() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("[h & _]", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::TupleRest { head, rest } => {
            assert_eq!(head.len(), 1);
            assert_eq!(*rest, Pattern::Wildcard);
        }
        _ => panic!("expected TupleRest pattern"),
    }
}

#[test]
fn parse_tuple_rest_empty_head() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("[& all]", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::TupleRest { head, rest } => {
            assert_eq!(head.len(), 0);
            assert_eq!(rest.binding_name(), Some(b"all".as_slice()));
        }
        _ => panic!("expected TupleRest pattern"),
    }
}

// --- Vector pattern tests ---

#[test]
fn parse_empty_vector() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("{}", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Vector(elements) => {
            assert_eq!(elements.len(), 0);
        }
        _ => panic!("expected Vector pattern"),
    }
}

#[test]
fn parse_vector_with_bindings() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("{a b c}", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Vector(elements) => {
            assert_eq!(elements.len(), 3);
            assert_eq!(elements[0].binding_name(), Some(b"a".as_slice()));
            assert_eq!(elements[1].binding_name(), Some(b"b".as_slice()));
            assert_eq!(elements[2].binding_name(), Some(b"c".as_slice()));
        }
        _ => panic!("expected Vector pattern"),
    }
}

// --- Map pattern tests ---

#[test]
fn parse_empty_map() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("%{}", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Map(pairs) => {
            assert_eq!(pairs.len(), 0);
        }
        _ => panic!("expected Map pattern"),
    }
}

#[test]
fn parse_map_single_key() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("%{:k v}", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Map(pairs) => {
            assert_eq!(pairs.len(), 1);
            assert!(pairs[0].0.is_keyword());
            assert_eq!(pairs[0].1.binding_name(), Some(b"v".as_slice()));
        }
        _ => panic!("expected Map pattern"),
    }
}

#[test]
fn parse_map_multiple_keys() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("%{:a x :b y}", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Map(pairs) => {
            assert_eq!(pairs.len(), 2);
            // Both keys should be keywords
            assert!(pairs[0].0.is_keyword());
            assert!(pairs[1].0.is_keyword());
            // Both values should be binding patterns
            assert!(matches!(&pairs[0].1, Pattern::Binding { .. }));
            assert!(matches!(&pairs[1].1, Pattern::Binding { .. }));
        }
        _ => panic!("expected Map pattern"),
    }
}

#[test]
fn parse_map_with_literal_value_pattern() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("%{:status :ok}", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Map(pairs) => {
            assert_eq!(pairs.len(), 1);
            assert!(pairs[0].0.is_keyword());
            match &pairs[0].1 {
                Pattern::Literal(term) => assert!(term.is_keyword()),
                _ => panic!("expected Literal keyword pattern"),
            }
        }
        _ => panic!("expected Map pattern"),
    }
}

#[test]
fn parse_map_with_nested_tuple() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    let pattern = parse_pattern("%{:data [a b]}", &mut proc, &mut realm, &mut mem);

    match pattern {
        Pattern::Map(pairs) => {
            assert_eq!(pairs.len(), 1);
            match &pairs[0].1 {
                Pattern::Tuple(elems) => {
                    assert_eq!(elems.len(), 2);
                }
                _ => panic!("expected nested Tuple pattern"),
            }
        }
        _ => panic!("expected Map pattern"),
    }
}

// --- Pattern helper method tests ---

#[test]
fn binding_name_returns_none_for_non_binding() {
    assert_eq!(Pattern::Wildcard.binding_name(), None);
    assert_eq!(
        Pattern::Literal(Term::small_int(42).unwrap()).binding_name(),
        None
    );
}

#[test]
fn pattern_binding_constructor() {
    let pattern = Pattern::binding(b"test");
    assert_eq!(pattern.binding_name(), Some(b"test".as_slice()));
}

#[test]
fn pattern_literal_constructor() {
    let term = Term::small_int(42).unwrap();
    let pattern = Pattern::literal(term);
    assert_eq!(pattern, Pattern::Literal(term));
}

#[test]
fn pattern_wildcard_constructor() {
    assert_eq!(Pattern::wildcard(), Pattern::Wildcard);
}

// --- Error case tests ---

/// Helper to parse a pattern and expect an error.
fn parse_pattern_err(
    src: &str,
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut MockVSpace,
) -> Result<Pattern, crate::compiler::CompileError> {
    let expr = read(src, proc, realm, mem)
        .expect("parse error")
        .expect("empty input");
    let compiler = Compiler::new(proc, mem, realm);
    compiler.parse_pattern(expr)
}

#[test]
fn error_list_not_valid_pattern() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // Lists (pairs) are not valid patterns
    let result = parse_pattern_err("(a b c)", &mut proc, &mut realm, &mut mem);
    assert!(result.is_err());
}

#[test]
fn error_malformed_rest_multiple_ampersands() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // Multiple & in tuple pattern is invalid
    let result = parse_pattern_err("[a & b & c]", &mut proc, &mut realm, &mut mem);
    assert!(result.is_err());
}

#[test]
fn error_malformed_rest_extra_after() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // Extra elements after & rest is invalid
    let result = parse_pattern_err("[& a b]", &mut proc, &mut realm, &mut mem);
    assert!(result.is_err());
}

#[test]
fn error_malformed_rest_nothing_after() {
    let (mut proc, mut realm, mut mem) = setup().unwrap();
    // & with nothing after is invalid
    let result = parse_pattern_err("[a &]", &mut proc, &mut realm, &mut mem);
    assert!(result.is_err());
}
