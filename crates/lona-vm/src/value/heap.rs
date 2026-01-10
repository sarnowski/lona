// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Heap-allocated value types.
//!
//! This module contains the header structures for heap-allocated Lonala values:
//! strings, pairs (cons cells), tuples, and maps.

use super::Value;

/// Heap-allocated string header.
///
/// Stored in memory as:
/// - 4 bytes: length (u32)
/// - `len` bytes: UTF-8 data (no null terminator)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeapString {
    /// Length of the string in bytes.
    pub len: u32,
    // Followed by `len` UTF-8 bytes (not represented in struct)
}

impl HeapString {
    /// Size of the header in bytes.
    pub const HEADER_SIZE: usize = core::mem::size_of::<Self>();

    /// Calculate total allocation size for a string of given length.
    #[inline]
    #[must_use]
    pub const fn alloc_size(len: usize) -> usize {
        Self::HEADER_SIZE + len
    }
}

/// Heap-allocated pair.
///
/// Used to build lists: (1 2 3) = Pair(1, Pair(2, Pair(3, Nil)))
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Pair {
    /// First element of the pair.
    pub first: Value,
    /// Rest of the list (second element of the pair).
    pub rest: Value,
}

impl Pair {
    /// Size of a pair in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Create a new pair.
    #[inline]
    #[must_use]
    pub const fn new(first: Value, rest: Value) -> Self {
        Self { first, rest }
    }
}

/// Heap-allocated tuple header.
///
/// Stored in memory as:
/// - 4 bytes: length (u32)
/// - `len * size_of::<Value>()` bytes: elements (array of Values)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeapTuple {
    /// Number of elements in the tuple.
    pub len: u32,
    // Followed by `len` Values (not represented in struct)
}

impl HeapTuple {
    /// Size of the header in bytes.
    pub const HEADER_SIZE: usize = core::mem::size_of::<Self>();

    /// Calculate total allocation size for a tuple of given length.
    #[inline]
    #[must_use]
    pub const fn alloc_size(len: usize) -> usize {
        Self::HEADER_SIZE + len * core::mem::size_of::<Value>()
    }
}

/// Heap-allocated map header.
///
/// Maps are implemented as association lists: linked lists of `[key value]` tuples.
/// The `entries` field points to a Pair chain where each `first` is a 2-element tuple.
///
/// Stored in memory as:
/// - 16 bytes: entries (`Value::Pair` or `Value::Nil` for empty map)
///
/// Example structure for `%{:a 1 :b 2}`:
/// ```text
/// HeapMap { entries } → Pair([:a 1], Pair([:b 2], nil))
/// ```
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeapMap {
    /// Head of the association list (Pair chain or nil).
    pub entries: Value,
}

impl HeapMap {
    /// Size of the header in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();
}
