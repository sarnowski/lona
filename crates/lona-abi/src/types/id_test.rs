// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for ID types.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::id::{ProcessId, RealmId, WorkerId};

#[test]
fn realm_id_constants() {
    assert!(RealmId::NULL.is_null());
    assert!(!RealmId::INIT.is_null());
    assert_eq!(RealmId::INIT.as_u64(), 1);
}

#[test]
fn process_id_constants() {
    assert!(ProcessId::NULL.is_null());
    assert!(!ProcessId::INIT.is_null());
    assert_eq!(ProcessId::INIT.as_u64(), 1);
}

#[test]
fn worker_id_bounds() {
    assert!(WorkerId::new(0).is_some());
    assert!(WorkerId::new(255).is_some());
    assert!(WorkerId::new(256).is_none());
}
