// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Garbage collection for BEAM-style per-process heaps.
//!
//! Lona uses a per-process generational semi-space copying collector
//! following BEAM's proven design. Each process has:
//!
//! - **Young heap**: Where new allocations happen. Collected frequently.
//! - **Old heap**: Where surviving objects are promoted. Collected rarely.
//!
//! # Algorithm
//!
//! ## Minor GC (Young Generation)
//!
//! When young heap is full:
//! 1. Copy live objects from young heap to old heap
//! 2. Reset young heap (htop = heap)
//! 3. Stack stays in place (not moved during minor GC)
//!
//! ## Major GC (Full Collection)
//!
//! When old heap is full or after N minor GCs:
//! 1. Allocate fresh heaps
//! 2. Copy live objects from both heaps to new young heap
//! 3. Free old heaps
//! 4. Old heap starts empty
//!
//! # Forwarding Pointers
//!
//! When an object is copied, a forwarding pointer is left at the original
//! location. This prevents double-copying and allows pointer updates.
//!
//! - **Boxed objects**: Header replaced with forward header (tag=0xFF)
//! - **Pairs**: head field marked with HEADER tag, rest stores new address
//!
//! See `docs/architecture/garbage-collection.md` for the full specification.

pub mod alloc;
pub mod copy;
pub mod growth;
pub mod major;
pub mod minor;
pub mod mso;
pub mod roots;
pub mod utils;

#[cfg(test)]
mod copy_test;
#[cfg(test)]
mod growth_test;
#[cfg(test)]
mod major_test;
#[cfg(test)]
mod minor_test;
#[cfg(test)]
mod mso_test;
#[cfg(test)]
mod roots_test;
#[cfg(test)]
mod utils_test;

// Re-export commonly used items
pub use alloc::{
    AllocResult, CompiledFnSpec, alloc_closure_with_gc, alloc_compiled_fn_with_gc,
    alloc_float_with_gc, alloc_map_with_gc, alloc_pair_with_gc, alloc_string_with_gc,
    alloc_tuple_with_gc, alloc_vector_with_gc, alloc_with_gc,
};
pub use copy::{Copier, copy_term, scan_object_fields};
pub use growth::{
    grow_old_heap, grow_young_heap_with_gc, next_heap_size, update_stack_frame_pointers,
};
pub use major::major_gc;
pub use minor::minor_gc;
pub use mso::{MsoEntry, MsoList};
pub use roots::{
    RootIterator, RootLocation, iterate_roots_with_mem, update_root, update_root_with_mem,
    update_root_y,
};
pub use utils::{
    forward_address_boxed, is_forwarded_boxed, is_in_old_heap, is_in_young_heap,
    make_forward_header, needs_tracing, object_size_from_header,
};

/// GC error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GcError {
    /// Minor GC cannot complete because old heap is full.
    /// Caller should trigger major GC instead.
    NeedsMajorGc,

    /// Out of memory - heap growth failed.
    OutOfMemory,

    /// Internal GC error (bug in GC implementation).
    InternalError,
}

/// Result of a garbage collection operation.
#[derive(Debug, Clone, Copy)]
pub struct GcStats {
    /// Number of bytes promoted to old heap (minor GC) or
    /// total live bytes after collection (major GC).
    pub live_bytes: usize,

    /// Number of bytes reclaimed.
    pub reclaimed_bytes: usize,
}

impl GcStats {
    /// Create stats for a GC that didn't reclaim anything.
    #[must_use]
    pub const fn new(live_bytes: usize, reclaimed_bytes: usize) -> Self {
        Self {
            live_bytes,
            reclaimed_bytes,
        }
    }
}
