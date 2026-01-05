// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for platform traits.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::MapError;

#[test]
fn test_map_error_display() {
    let err = MapError::AlreadyMapped;
    assert_eq!(format!("{err}"), "virtual address already mapped");
}
