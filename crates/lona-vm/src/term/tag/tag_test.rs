// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for tag constants.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

/// Primary tags should not overlap.
#[test]
fn primary_tags_are_distinct() {
    let tags = [
        primary::HEADER,
        primary::LIST,
        primary::BOXED,
        primary::IMMEDIATE,
    ];

    // All tags should fit in 2 bits
    for &tag in &tags {
        assert!(tag <= 0b11, "Tag {tag:#04b} exceeds 2 bits");
    }

    // All tags should be distinct
    for i in 0..tags.len() {
        for j in (i + 1)..tags.len() {
            assert_ne!(tags[i], tags[j], "Tags at {i} and {j} are equal");
        }
    }
}

/// Immediate subtags should have correct primary tag bits.
#[test]
fn immediate_subtags_have_correct_primary() {
    let subtags = [
        immediate::SMALL_INT,
        immediate::SYMBOL,
        immediate::KEYWORD,
        immediate::SPECIAL,
    ];

    for &subtag in &subtags {
        assert_eq!(
            subtag & primary::MASK,
            primary::IMMEDIATE,
            "Subtag {subtag:#06b} does not have IMMEDIATE primary"
        );
    }
}

/// Immediate subtags should be distinct.
#[test]
fn immediate_subtags_are_distinct() {
    let subtags = [
        immediate::SMALL_INT,
        immediate::SYMBOL,
        immediate::KEYWORD,
        immediate::SPECIAL,
    ];

    for i in 0..subtags.len() {
        for j in (i + 1)..subtags.len() {
            assert_ne!(subtags[i], subtags[j], "Subtags at {i} and {j} are equal");
        }
    }
}

/// Special values should have correct immediate subtag.
#[test]
fn special_values_have_correct_subtag() {
    let specials = [
        special::NIL,
        special::TRUE,
        special::FALSE,
        special::UNBOUND,
    ];

    for &val in &specials {
        assert_eq!(
            val & immediate::MASK,
            immediate::SPECIAL,
            "Special value {val:#04x} does not have SPECIAL subtag"
        );
    }
}

/// Special values should be distinct.
#[test]
fn special_values_are_distinct() {
    let specials = [
        special::NIL,
        special::TRUE,
        special::FALSE,
        special::UNBOUND,
    ];

    for i in 0..specials.len() {
        for j in (i + 1)..specials.len() {
            assert_ne!(
                specials[i], specials[j],
                "Special values at {i} and {j} are equal"
            );
        }
    }
}

/// Object tags should fit in 8 bits.
#[test]
fn object_tags_fit_in_8_bits() {
    let tags: [u8; 15] = [
        object::TUPLE,
        object::VECTOR,
        object::MAP,
        object::STRING,
        object::BINARY,
        object::BIGNUM,
        object::FLOAT,
        object::FUN,
        object::CLOSURE,
        object::PID,
        object::REF,
        object::PROCBIN,
        object::SUBBIN,
        object::NAMESPACE,
        object::VAR,
    ];

    // All fit in u8 by construction, but verify FORWARD is special
    assert_eq!(object::FORWARD, 0xFF, "FORWARD should be 0xFF");

    // Verify main object tags don't conflict with FORWARD
    for &tag in &tags {
        assert_ne!(tag, object::FORWARD, "Object tag conflicts with FORWARD");
    }
}
