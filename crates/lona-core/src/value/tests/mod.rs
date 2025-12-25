// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for Value types.

mod kind;
mod primitives;
#[cfg(feature = "alloc")]
mod ratio;
#[cfg(feature = "alloc")]
mod string;

use super::*;
#[cfg(feature = "alloc")]
use crate::symbol::Interner;

/// Helper to create an integer value.
#[cfg(feature = "alloc")]
pub(super) fn int(value: i64) -> Value {
    Value::Integer(Integer::from_i64(value))
}

/// Helper to create an integer value (non-alloc).
#[cfg(not(feature = "alloc"))]
pub(super) fn int(value: i64) -> Value {
    Value::Integer(value)
}
