// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! MSO (Mark-Sweep Objects) list for off-heap binary references.
//!
//! The MSO list tracks references to off-heap objects (like large binaries)
//! that need special handling during garbage collection. When a process
//! allocates a large binary, a `HeapProcBin` on the process heap references
//! a `RefcBinary` in the realm's binary heap.
//!
//! During GC, we need to:
//! 1. Update MSO entries when their `HeapProcBin` is forwarded
//! 2. Decrement refcount and remove entry when `HeapProcBin` is dead
//!
//! The MSO list is a singly-linked list stored in the process heap itself.
//! Each entry points to a `HeapProcBin` that holds the actual binary reference.

use crate::Vaddr;
use crate::platform::MemorySpace;

/// Entry in the MSO (Mark-Sweep Objects) list.
///
/// Each entry tracks a `HeapProcBin` object on the process heap.
/// When the `HeapProcBin` is garbage collected, its referenced
/// binary's refcount must be decremented.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MsoEntry {
    /// Next entry in the list (or `Vaddr::null()` for end).
    pub next: Vaddr,
    /// Address of the `HeapProcBin` object on the process heap.
    pub object_addr: Vaddr,
}

// Compile-time assertion that MsoEntry is 16 bytes
const _: () = assert!(core::mem::size_of::<MsoEntry>() == 16);

impl MsoEntry {
    /// Size of an MSO entry in bytes.
    pub const SIZE: usize = 16;
}

/// MSO list head tracker.
///
/// This is a lightweight wrapper around the list head pointer.
/// The actual entries are stored in the process heap.
#[derive(Clone, Copy, Debug)]
pub struct MsoList {
    /// Head of the list (first entry) or `Vaddr::null()` if empty.
    head: Vaddr,
}

impl MsoList {
    /// Create an empty MSO list.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            head: Vaddr::null(),
        }
    }

    /// Check if the list is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    /// Get the head of the list.
    #[must_use]
    pub const fn head(&self) -> Vaddr {
        self.head
    }

    /// Set the head of the list.
    pub const fn set_head(&mut self, head: Vaddr) {
        self.head = head;
    }

    /// Push a new entry to the front of the list.
    ///
    /// The entry is written at `entry_addr` and linked to the front.
    /// The caller must have already allocated space for the entry.
    pub fn push<M: MemorySpace>(&mut self, mem: &mut M, entry_addr: Vaddr, object_addr: Vaddr) {
        let entry = MsoEntry {
            next: self.head,
            object_addr,
        };
        mem.write(entry_addr, entry);
        self.head = entry_addr;
    }
}

impl Default for MsoList {
    fn default() -> Self {
        Self::new()
    }
}
