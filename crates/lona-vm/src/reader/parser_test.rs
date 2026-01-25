// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the Lonala parser.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::{ParseError, ReadError, read};
use crate::Vaddr;
use crate::platform::MockVSpace;
use crate::process::Process;
use crate::realm::Realm;
use crate::term::Term;

fn setup() -> (Process, Realm, MockVSpace) {
    let base = Vaddr::new(0x1_0000);
    let mem = MockVSpace::new(256 * 1024, base);
    let young_base = base;
    let young_size = 64 * 1024;
    let old_base = base.add(young_size as u64);
    let old_size = 16 * 1024;
    let proc = Process::new(young_base, young_size, old_base, old_size);
    // Create realm at a higher address for symbol/keyword interning
    let realm_base = base.add(128 * 1024);
    let realm = Realm::new(realm_base, 64 * 1024);
    (proc, realm, mem)
}

/// Get the string name of a keyword term using the realm's intern table.
fn keyword_str<'a>(term: Term, realm: &Realm, mem: &'a MockVSpace) -> &'a str {
    let index = term.as_keyword_index().expect("not a keyword");
    realm
        .keyword_name(mem, index)
        .expect("keyword not found in intern table")
}

/// Get the string name of a symbol term using the realm's intern table.
fn symbol_str<'a>(term: Term, realm: &Realm, mem: &'a MockVSpace) -> &'a str {
    let index = term.as_symbol_index().expect("not a symbol");
    realm
        .symbol_name(mem, index)
        .expect("symbol not found in intern table")
}

#[test]
fn read_nil() {
    let (mut proc, mut realm, mut mem) = setup();
    let value = read("nil", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(value.is_nil());
}

#[test]
fn read_booleans() {
    let (mut proc, mut realm, mut mem) = setup();

    let t = read("true", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert_eq!(t, Term::TRUE);

    let f = read("false", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert_eq!(f, Term::FALSE);
}

#[test]
fn read_integers() {
    let (mut proc, mut realm, mut mem) = setup();

    assert_eq!(
        read("0", &mut proc, &mut realm, &mut mem).unwrap().unwrap(),
        Term::small_int(0).unwrap()
    );
    assert_eq!(
        read("42", &mut proc, &mut realm, &mut mem)
            .unwrap()
            .unwrap(),
        Term::small_int(42).unwrap()
    );
    assert_eq!(
        read("-123", &mut proc, &mut realm, &mut mem)
            .unwrap()
            .unwrap(),
        Term::small_int(-123).unwrap()
    );
}

#[test]
fn read_strings() {
    let (mut proc, mut realm, mut mem) = setup();

    let value = read("\"hello\"", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    let s = proc.read_term_string(&mem, value).unwrap();
    assert_eq!(s, "hello");
}

#[test]
fn read_empty_list() {
    let (mut proc, mut realm, mut mem) = setup();

    let value = read("()", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(value.is_nil());
}

#[test]
fn read_list() {
    let (mut proc, mut realm, mut mem) = setup();

    let value = read("(1 2 3)", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(value.is_list());

    // Check structure: (1 . (2 . (3 . nil)))
    let (head1, tail1) = proc.read_term_pair(&mem, value).unwrap();
    assert_eq!(head1, Term::small_int(1).unwrap());

    let (head2, tail2) = proc.read_term_pair(&mem, tail1).unwrap();
    assert_eq!(head2, Term::small_int(2).unwrap());

    let (head3, tail3) = proc.read_term_pair(&mem, tail2).unwrap();
    assert_eq!(head3, Term::small_int(3).unwrap());
    assert!(tail3.is_nil());
}

#[test]
fn read_nested_list() {
    let (mut proc, mut realm, mut mem) = setup();

    let value = read("(1 (2 3))", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();

    let (head1, tail1) = proc.read_term_pair(&mem, value).unwrap();
    assert_eq!(head1, Term::small_int(1).unwrap());

    let (head2, _tail2) = proc.read_term_pair(&mem, tail1).unwrap();
    assert!(head2.is_list()); // (2 3)

    let (inner_head, _inner_tail) = proc.read_term_pair(&mem, head2).unwrap();
    assert_eq!(inner_head, Term::small_int(2).unwrap());
}

#[test]
fn read_quote() {
    let (mut proc, mut realm, mut mem) = setup();

    // 'x => (quote x)
    let value = read("'x", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(value.is_list());

    let (head1, tail1) = proc.read_term_pair(&mem, value).unwrap();
    let quote_name = symbol_str(head1, &realm, &mem);
    assert_eq!(quote_name, "quote");

    let (head2, tail2) = proc.read_term_pair(&mem, tail1).unwrap();
    let x_name = symbol_str(head2, &realm, &mem);
    assert_eq!(x_name, "x");
    assert!(tail2.is_nil());
}

#[test]
fn read_quote_list() {
    let (mut proc, mut realm, mut mem) = setup();

    // '(1 2 3) => (quote (1 2 3))
    let value = read("'(1 2 3)", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(value.is_list());

    let (head1, tail1) = proc.read_term_pair(&mem, value).unwrap();
    let quote_name = symbol_str(head1, &realm, &mem);
    assert_eq!(quote_name, "quote");

    let (head2, _tail2) = proc.read_term_pair(&mem, tail1).unwrap();
    assert!(head2.is_list()); // The list (1 2 3)
}

#[test]
fn read_empty_input() {
    let (mut proc, mut realm, mut mem) = setup();
    let value = read("", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(value.is_none());
}

#[test]
fn read_whitespace_only() {
    let (mut proc, mut realm, mut mem) = setup();
    let value = read("   \n\t  ", &mut proc, &mut realm, &mut mem).unwrap();
    assert!(value.is_none());
}

#[test]
fn read_unmatched_rparen() {
    let (mut proc, mut realm, mut mem) = setup();
    let err = read(")", &mut proc, &mut realm, &mut mem).unwrap_err();
    assert!(matches!(err, ReadError::Parse(ParseError::UnmatchedRParen)));
}

#[test]
fn read_unclosed_list() {
    let (mut proc, mut realm, mut mem) = setup();
    let err = read("(1 2", &mut proc, &mut realm, &mut mem).unwrap_err();
    assert!(matches!(err, ReadError::Parse(ParseError::UnexpectedEof)));
}

// --- Keyword parser tests ---

#[test]
fn read_keyword_simple() {
    let (mut proc, mut realm, mut mem) = setup();
    let value = read(":foo", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(value.is_keyword());
    let s = keyword_str(value, &realm, &mem);
    assert_eq!(s, "foo");
}

#[test]
fn read_keyword_qualified() {
    let (mut proc, mut realm, mut mem) = setup();
    let value = read(":ns/bar", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(value.is_keyword());
    let s = keyword_str(value, &realm, &mem);
    assert_eq!(s, "ns/bar");
}

#[test]
fn read_keyword_interning() {
    let (mut proc, mut realm, mut mem) = setup();

    // Parse the same keyword twice
    let k1 = read(":foo", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    let k2 = read(":foo", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();

    // Both should be keywords
    assert!(k1.is_keyword());
    assert!(k2.is_keyword());

    // Due to interning, they should have the same address
    // Both are keywords (verified above), so direct comparison is valid
    assert_eq!(k1, k2, "interned keywords should be equal");
}

#[test]
fn read_keyword_different_not_interned() {
    let (mut proc, mut realm, mut mem) = setup();

    // Parse different keywords
    let k1 = read(":foo", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    let k2 = read(":bar", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();

    // Both are keywords
    assert!(k1.is_keyword());
    assert!(k2.is_keyword());

    // Different keywords should not be equal
    assert_ne!(k1, k2, "different keywords should not be equal");
}

// --- Tuple parser tests ---

#[test]
fn read_tuple_empty() {
    let (mut proc, mut realm, mut mem) = setup();
    let value = read("[]", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(proc.is_term_tuple(&mem, value));
    let len = proc.read_term_tuple_len(&mem, value).unwrap();
    assert_eq!(len, 0);
}

#[test]
fn read_tuple_simple() {
    let (mut proc, mut realm, mut mem) = setup();
    let value = read("[1 2 3]", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(proc.is_term_tuple(&mem, value));

    let len = proc.read_term_tuple_len(&mem, value).unwrap();
    assert_eq!(len, 3);

    assert_eq!(
        proc.read_term_tuple_element(&mem, value, 0).unwrap(),
        Term::small_int(1).unwrap()
    );
    assert_eq!(
        proc.read_term_tuple_element(&mem, value, 1).unwrap(),
        Term::small_int(2).unwrap()
    );
    assert_eq!(
        proc.read_term_tuple_element(&mem, value, 2).unwrap(),
        Term::small_int(3).unwrap()
    );
}

#[test]
fn read_tuple_mixed() {
    let (mut proc, mut realm, mut mem) = setup();
    let value = read("[1 \"hello\" nil]", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(proc.is_term_tuple(&mem, value));

    let len = proc.read_term_tuple_len(&mem, value).unwrap();
    assert_eq!(len, 3);

    assert_eq!(
        proc.read_term_tuple_element(&mem, value, 0).unwrap(),
        Term::small_int(1).unwrap()
    );
    let s = proc.read_term_tuple_element(&mem, value, 1).unwrap();
    assert!(proc.is_term_string(&mem, s));
    assert_eq!(proc.read_term_string(&mem, s).unwrap(), "hello");
    assert!(
        proc.read_term_tuple_element(&mem, value, 2)
            .unwrap()
            .is_nil()
    );
}

#[test]
fn read_tuple_nested() {
    let (mut proc, mut realm, mut mem) = setup();
    let value = read("[[1 2] [3 4]]", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(proc.is_term_tuple(&mem, value));

    let len = proc.read_term_tuple_len(&mem, value).unwrap();
    assert_eq!(len, 2);

    let inner1 = proc.read_term_tuple_element(&mem, value, 0).unwrap();
    assert!(proc.is_term_tuple(&mem, inner1));
    assert_eq!(proc.read_term_tuple_len(&mem, inner1).unwrap(), 2);

    let inner2 = proc.read_term_tuple_element(&mem, value, 1).unwrap();
    assert!(proc.is_term_tuple(&mem, inner2));
    assert_eq!(proc.read_term_tuple_len(&mem, inner2).unwrap(), 2);
}

#[test]
fn read_tuple_with_keywords() {
    let (mut proc, mut realm, mut mem) = setup();
    let value = read("[:a :b]", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(proc.is_term_tuple(&mem, value));

    let len = proc.read_term_tuple_len(&mem, value).unwrap();
    assert_eq!(len, 2);

    let k1 = proc.read_term_tuple_element(&mem, value, 0).unwrap();
    assert!(k1.is_keyword());
    assert_eq!(keyword_str(k1, &realm, &mem), "a");

    let k2 = proc.read_term_tuple_element(&mem, value, 1).unwrap();
    assert!(k2.is_keyword());
    assert_eq!(keyword_str(k2, &realm, &mem), "b");
}

#[test]
fn read_unclosed_tuple() {
    let (mut proc, mut realm, mut mem) = setup();
    let err = read("[1 2", &mut proc, &mut realm, &mut mem).unwrap_err();
    assert!(matches!(err, ReadError::Parse(ParseError::UnexpectedEof)));
}

// --- Map parser tests (Phase 2) ---

#[test]
fn read_map_empty() {
    let (mut proc, mut realm, mut mem) = setup();
    let value = read("%{}", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(proc.is_term_map(&mem, value));
    let entries = proc.read_term_map_entries(&mem, value).unwrap();
    assert!(entries.is_nil());
}

#[test]
fn read_map_simple() {
    let (mut proc, mut realm, mut mem) = setup();
    let value = read("%{:a 1 :b 2}", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(proc.is_term_map(&mem, value));
}

#[test]
fn read_map_odd_elements_error() {
    let (mut proc, mut realm, mut mem) = setup();
    let err = read("%{:a 1 :b}", &mut proc, &mut realm, &mut mem).unwrap_err();
    assert!(matches!(err, ReadError::Parse(ParseError::MapOddElements)));
}

#[test]
fn read_map_unclosed_error() {
    let (mut proc, mut realm, mut mem) = setup();
    let err = read("%{:a 1", &mut proc, &mut realm, &mut mem).unwrap_err();
    assert!(matches!(err, ReadError::Parse(ParseError::UnexpectedEof)));
}

#[test]
fn read_unmatched_rbrace() {
    let (mut proc, mut realm, mut mem) = setup();
    let err = read("}", &mut proc, &mut realm, &mut mem).unwrap_err();
    assert!(matches!(err, ReadError::Parse(ParseError::UnmatchedRBrace)));
}

// --- Metadata parser tests (Phase 2) ---

#[test]
fn read_metadata_keyword() {
    let (mut proc, mut realm, mut mem) = setup();
    // ^:foo bar parses and attaches metadata
    let value = read("^:private foo", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(proc.is_term_symbol(value));
    let idx = value.as_symbol_index().unwrap();
    assert_eq!(realm.symbol_name(&mem, idx).unwrap(), "foo");
}

#[test]
fn read_metadata_map() {
    let (mut proc, mut realm, mut mem) = setup();
    let value = read("^%{:doc \"hello\"} foo", &mut proc, &mut realm, &mut mem)
        .unwrap()
        .unwrap();
    assert!(proc.is_term_symbol(value));
    let idx = value.as_symbol_index().unwrap();
    assert_eq!(realm.symbol_name(&mem, idx).unwrap(), "foo");
}

#[test]
fn read_metadata_invalid() {
    let (mut proc, mut realm, mut mem) = setup();
    let err = read("^123 foo", &mut proc, &mut realm, &mut mem).unwrap_err();
    assert!(matches!(err, ReadError::Parse(ParseError::InvalidMetadata)));
}

#[test]
fn read_metadata_missing_form() {
    let (mut proc, mut realm, mut mem) = setup();
    let err = read("^:foo", &mut proc, &mut realm, &mut mem).unwrap_err();
    assert!(matches!(
        err,
        ReadError::Parse(ParseError::MissingFormAfterMetadata)
    ));
}
