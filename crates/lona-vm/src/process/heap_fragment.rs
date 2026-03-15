// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Heap fragments for cross-worker message delivery.
//!
//! When a message is sent to a process that is currently taken (running on
//! another worker), the message cannot be deep-copied directly to the
//! receiver's heap. Instead, a heap fragment is allocated from the realm's
//! process pool, the message is deep-copied into the fragment, and the
//! fragment is linked to the receiver's slot inbox in the `ProcessTable`.
//!
//! During GC (or when the receiver processes its inbox), fragment data is
//! consolidated into the process's main heap and the fragment is freed.
//!
//! ```text
//! HeapFragment (linked list):
//! ┌──────────────────┐    ┌──────────────────┐
//! │ base: Vaddr      │    │ base: Vaddr      │
//! │ size: usize      │    │ size: usize      │
//! │ top: usize       │    │ top: usize       │
//! │ message: Term    │───►│ message: Term    │───► None
//! │ next: Option<..> │    │ next: Option<..> │
//! └──────────────────┘    └──────────────────┘
//! ```

extern crate alloc;

use alloc::boxed::Box;

use crate::Vaddr;
use crate::term::Term;

/// A heap fragment holding a single message and its heap data.
///
/// The fragment owns a contiguous region of memory (allocated from the realm's
/// process pool) where the message's heap-allocated data is stored. The `message`
/// field is the top-level Term; its boxed/list pointers reference addresses within
/// `[base, base + top)`.
pub struct HeapFragment {
    /// Next fragment in the linked list.
    pub next: Option<Box<Self>>,
    /// Base address of the fragment's memory region (in Vaddr space).
    base: Vaddr,
    /// Total size of the fragment's memory region.
    size: usize,
    /// Bump allocation pointer (offset from base).
    top: usize,
    /// The message term stored in this fragment.
    ///
    /// Its heap data (if any) lives in `[base, base + top)`.
    message: Term,
}

impl HeapFragment {
    /// Create a new heap fragment with the given memory region.
    ///
    /// The `base` address must be a valid Vaddr allocated from the realm's
    /// process pool. The `size` is the total capacity in bytes.
    #[must_use]
    pub const fn new(base: Vaddr, size: usize) -> Self {
        Self {
            next: None,
            base,
            size,
            top: 0,
            message: Term::NIL,
        }
    }

    /// Bump-allocate space within the fragment.
    ///
    /// Returns the Vaddr of the allocated region, or `None` if insufficient space.
    pub const fn alloc(&mut self, size: usize, align: usize) -> Option<Vaddr> {
        if size == 0 {
            return Some(Vaddr::new(self.base.as_u64() + self.top as u64));
        }

        // Align top up
        let current = self.base.as_u64() + self.top as u64;
        let mask = (align as u64).wrapping_sub(1);
        let aligned = (current + mask) & !mask;
        let offset = (aligned - self.base.as_u64()) as usize;
        let new_top = offset + size;

        if new_top > self.size {
            return None;
        }

        self.top = new_top;
        Some(Vaddr::new(aligned))
    }

    /// Set the message stored in this fragment.
    pub const fn set_message(&mut self, msg: Term) {
        self.message = msg;
    }

    /// Get the message stored in this fragment.
    #[must_use]
    pub const fn message(&self) -> Term {
        self.message
    }

    /// Base address of the fragment's memory region.
    #[must_use]
    pub const fn base(&self) -> Vaddr {
        self.base
    }

    /// Bytes used in this fragment.
    #[must_use]
    pub const fn used(&self) -> usize {
        self.top
    }

    /// Total capacity of this fragment.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.size
    }

    /// End address of used data (base + top).
    #[must_use]
    pub const fn top_addr(&self) -> Vaddr {
        Vaddr::new(self.base.as_u64() + self.top as u64)
    }
}
