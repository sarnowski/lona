// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Architecture-specific paging support for seL4.
//!
//! This module provides a unified interface for paging operations across
//! different CPU architectures. Each architecture has different page table
//! hierarchies and capability types.
//!
//! # Page Table Hierarchies
//!
//! ## ARM64
//! ARM64 uses a 4-level page table structure where all intermediate levels
//! use the same PT (Page Table) object type.
//!
//! ## `x86_64`
//! `x86_64` uses a 4-level page table structure with different object types
//! at each level:
//! - PML4 (level 4) - 512GB per entry (`VSpace` root)
//! - PDPT (level 3) - 1GB per entry
//! - `PageDirectory` (level 2) - 2MB per entry
//! - `PageTable` (level 1) - 4KB per entry

#[cfg(target_arch = "aarch64")]
mod aarch64;
#[cfg(target_arch = "x86_64")]
mod x86_64;

use sel4::{BootInfo, ObjectBlueprint};

use super::slots::SlotAllocator;
use super::untyped::UntypedTracker;

// Re-export architecture-specific implementations
#[cfg(target_arch = "aarch64")]
pub use aarch64::*;
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

/// Context for paging operations.
///
/// Provides access to slot allocation and untyped memory tracking
/// needed for creating paging structures.
pub struct PagingContext<'ctx> {
    /// Boot info for accessing untyped memory.
    pub bootinfo: &'ctx BootInfo,
    /// Slot allocator for capability slots.
    pub slot_allocator: &'ctx mut SlotAllocator,
    /// Untyped memory tracker.
    pub untyped_tracker: &'ctx mut UntypedTracker,
}

/// Creates a paging structure from untyped memory using the given blueprint.
///
/// Returns the slot index where the new capability was placed.
pub fn create_paging_structure(
    ctx: &mut PagingContext<'_>,
    blueprint: ObjectBlueprint,
    name: &str,
) -> Option<usize> {
    // Allocate a CNode slot for the capability
    let slot = ctx.slot_allocator.allocate()?;

    // Find untyped memory for the object
    let allocation = ctx.untyped_tracker.find_next_frame_untyped(ctx.bootinfo)?;

    let untyped = ctx.bootinfo.untyped().index(allocation.untyped_index).cap();
    let cnode = sel4::init_thread::slot::CNODE.cap();

    match untyped.untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), slot, 1) {
        Ok(()) => Some(slot),
        Err(err) => {
            sel4::debug_println!("Failed to retype untyped into {}: {:?}", name, err);
            None
        }
    }
}
