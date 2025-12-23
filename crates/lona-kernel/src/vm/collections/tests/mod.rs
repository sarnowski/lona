// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for collection primitive functions.

use lona_core::integer::Integer;
use lona_core::string::HeapStr;
use lona_core::symbol::Interner;
use lona_core::value::Value;

use crate::vm::natives::NativeContext;

mod concat_tests;
mod cons_tests;
mod first_tests;
mod get_tests;
mod list_tests;
mod map_tests;
mod rest_tests;
mod vec_tests;
mod vector_tests;

/// Helper to create an integer value.
pub(super) fn int(value: i64) -> Value {
    Value::Integer(Integer::from_i64(value))
}

/// Helper to create a string value.
pub(super) fn string(text: &str) -> Value {
    Value::String(HeapStr::new(text))
}

/// Helper to create a native context for testing.
pub(super) fn ctx(interner: &Interner) -> NativeContext<'_> {
    NativeContext::new(interner, None)
}
