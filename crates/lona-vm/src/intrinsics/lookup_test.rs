// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for intrinsic lookup functions.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn lookup_arithmetic() {
    assert_eq!(lookup_intrinsic("+"), Some(id::ADD));
    assert_eq!(lookup_intrinsic("-"), Some(id::SUB));
    assert_eq!(lookup_intrinsic("*"), Some(id::MUL));
    assert_eq!(lookup_intrinsic("/"), Some(id::DIV));
    assert_eq!(lookup_intrinsic("mod"), Some(id::MOD));
}

#[test]
fn lookup_comparison() {
    assert_eq!(lookup_intrinsic("="), Some(id::EQ));
    assert_eq!(lookup_intrinsic("<"), Some(id::LT));
    assert_eq!(lookup_intrinsic(">"), Some(id::GT));
    assert_eq!(lookup_intrinsic("<="), Some(id::LE));
    assert_eq!(lookup_intrinsic(">="), Some(id::GE));
}

#[test]
fn lookup_predicates() {
    assert_eq!(lookup_intrinsic("not"), Some(id::NOT));
    assert_eq!(lookup_intrinsic("nil?"), Some(id::IS_NIL));
    assert_eq!(lookup_intrinsic("integer?"), Some(id::IS_INT));
    assert_eq!(lookup_intrinsic("string?"), Some(id::IS_STR));
}

#[test]
fn lookup_str() {
    assert_eq!(lookup_intrinsic("str"), Some(id::STR));
}

#[test]
fn lookup_unknown() {
    assert_eq!(lookup_intrinsic("unknown"), None);
    assert_eq!(lookup_intrinsic("println"), None);
}

#[test]
fn intrinsic_name_roundtrip() {
    for i in 0..INTRINSIC_COUNT as u8 {
        let name = intrinsic_name(i).unwrap();
        assert_eq!(lookup_intrinsic(name), Some(i));
    }
}

#[test]
fn process_intrinsic_ids_contains_expected() {
    let expected = [
        id::PROCESS_INFO,
        id::SPAWN,
        id::SELF,
        id::ALIVE,
        id::IS_PID,
        id::SEND,
        id::IS_REF,
        id::LINK,
        id::UNLINK,
        id::TRAP_EXIT,
        id::MONITOR,
        id::DEMONITOR,
        id::EXIT,
        id::SPAWN_LINK,
        id::SPAWN_MONITOR,
    ];
    assert_eq!(PROCESS_INTRINSIC_IDS.len(), expected.len());
    for &id in &expected {
        assert!(
            PROCESS_INTRINSIC_IDS.contains(&id),
            "PROCESS_INTRINSIC_IDS should contain {:?} ({})",
            intrinsic_name(id),
            id
        );
    }
}

#[test]
fn process_intrinsic_ids_all_valid() {
    for &id in PROCESS_INTRINSIC_IDS {
        assert!(
            (id as usize) < INTRINSIC_COUNT,
            "Process intrinsic ID {id} exceeds INTRINSIC_COUNT ({INTRINSIC_COUNT})",
        );
    }
}
