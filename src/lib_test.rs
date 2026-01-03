// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the library root.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn test_init_succeeds() {
    let result = init();
    assert!(result.is_ok());
}

#[test]
fn test_version_not_empty() {
    assert!(!VERSION.is_empty());
}
