// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Pair cells for list representation.
//!
//! A pair cell (also called a cons cell) holds two Terms: head and rest.
//! Unlike boxed values, pair cells have NO HEADER - they are identified
//! by the LIST tag on the pointer, not by an in-memory header word.
//!
//! Layout:
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                         head (8 bytes)                          │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                         rest (8 bytes)                          │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! During GC, forwarded pair cells are detected by checking if `head`
//! has the HEADER tag (which is impossible for a valid Term in a register
//! or on the heap). The forwarding address is stored in `rest`.

// Pointer casts are intentional and required for the low-level memory layout.
// Alignment is verified at runtime via debug_assert.
#![allow(clippy::cast_ptr_alignment)]

#[cfg(test)]
mod pair_test;

use super::Term;
use super::tag::primary;

/// A pair cell on the heap.
///
/// NO HEADER - identified by LIST tag on pointer.
/// Total size: 16 bytes (two 8-byte Terms).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct Pair {
    /// The first element of the pair (head).
    pub head: Term,
    /// The second element of the pair (rest/tail).
    pub rest: Term,
}

// Compile-time assertion that Pair is exactly 16 bytes
const _: () = assert!(core::mem::size_of::<Pair>() == 16);

// Compile-time assertion that Pair has 8-byte alignment
const _: () = assert!(core::mem::align_of::<Pair>() == 8);

impl Pair {
    /// Size of a pair cell in bytes.
    pub const SIZE: usize = 16;

    /// Create a new pair cell.
    #[inline]
    #[must_use]
    pub const fn new(head: Term, rest: Term) -> Self {
        Self { head, rest }
    }

    /// Check if this pair cell has been forwarded during GC.
    ///
    /// During garbage collection, forwarded cells are marked by setting
    /// the head to a value with the HEADER primary tag, which is impossible
    /// for any valid term that would appear in a pair cell.
    #[inline]
    #[must_use]
    pub const fn is_forwarded(&self) -> bool {
        self.head.is_header()
    }

    /// Get the forwarding address (only valid if `is_forwarded()` is true).
    ///
    /// # Safety
    ///
    /// Caller must ensure this pair is forwarded (`is_forwarded()` returns true).
    #[inline]
    #[must_use]
    pub const fn forward_address(&self) -> *const Self {
        debug_assert!(self.head.is_header());
        self.rest.to_ptr().cast::<Self>()
    }

    /// Set the forwarding pointer, marking this cell as forwarded.
    ///
    /// After this call, `is_forwarded()` will return true and
    /// `forward_address()` will return the given address.
    ///
    /// # Safety
    ///
    /// The `new_addr` must be a valid pointer to a Pair in the new heap.
    #[inline]
    pub unsafe fn set_forward(&mut self, new_addr: *const Self) {
        // Mark as forwarded using header tag in head
        // SAFETY: HEADER tag is a valid Term encoding for forwarding marker
        self.head = unsafe { Term::from_raw(primary::HEADER) };
        // Store new address (must be 8-byte aligned, no tag needed)
        // SAFETY: Raw address stored as Term for forwarding pointer
        self.rest = unsafe { Term::from_raw(new_addr as u64) };
    }
}

impl Term {
    /// Create a list pointer to a pair cell.
    ///
    /// # Safety
    ///
    /// The pointer must point to a valid, 8-byte aligned Pair cell.
    #[inline]
    #[must_use]
    pub fn list(ptr: *const Pair) -> Self {
        // Check 8-byte alignment (low 3 bits must be zero)
        // We only use 2 bits for tags, but 8-byte alignment is required
        // for the forwarding pointer address packing to work correctly.
        debug_assert!(
            (ptr as u64).trailing_zeros() >= 3,
            "Pointer not 8-byte aligned"
        );
        Self((ptr as u64) | primary::LIST)
    }

    /// Get pointer to pair cell (if this is a list).
    ///
    /// Returns `None` if this is not a list pointer.
    /// Note: NIL (empty list) returns `None` - use `is_nil()` to check for empty list.
    #[inline]
    #[must_use]
    pub const fn as_pair_ptr(self) -> Option<*const Pair> {
        if self.is_list() {
            Some(self.to_ptr().cast::<Pair>())
        } else {
            None
        }
    }

    /// Get mutable pointer to pair cell (if this is a list).
    #[inline]
    #[must_use]
    pub const fn as_pair_ptr_mut(self) -> Option<*mut Pair> {
        if self.is_list() {
            Some(self.to_ptr_mut().cast::<Pair>())
        } else {
            None
        }
    }

    /// Check if this is the empty list.
    ///
    /// The empty list is represented as NIL, not as a LIST tag pointer.
    #[inline]
    #[must_use]
    pub const fn is_empty_list(self) -> bool {
        self.is_nil()
    }

    /// Get the head of a list (first element).
    ///
    /// Returns `None` if not a list pointer.
    ///
    /// # Safety
    ///
    /// The pointer must be valid and the Pair must not have been deallocated.
    #[inline]
    #[must_use]
    pub unsafe fn head(self) -> Option<Self> {
        self.as_pair_ptr().map(|p| unsafe { (*p).head })
    }

    /// Get the rest of a list (tail).
    ///
    /// Returns `None` if not a list pointer.
    ///
    /// # Safety
    ///
    /// The pointer must be valid and the Pair must not have been deallocated.
    #[inline]
    #[must_use]
    pub unsafe fn rest(self) -> Option<Self> {
        self.as_pair_ptr().map(|p| unsafe { (*p).rest })
    }
}
