// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for capability slot type.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::cap::CapSlot;

#[test]
fn cap_slot_constants() {
    assert!(CapSlot::NULL.is_null());
    assert!(!CapSlot::CSPACE.is_null());
    assert!(CapSlot::FIRST_FREE.as_u64() >= 16);
}
