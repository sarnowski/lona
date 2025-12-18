// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! seL4 page provider implementation.
//!
//! Bridges the gap between Lona's abstract [`PageProvider`] trait and seL4's
//! capability-based memory model. Handles retyping untyped memory into frames
//! and mapping them into the virtual address space.

use core::cell::UnsafeCell;
use core::ptr::NonNull;

use lona_core::allocator::PageProvider;
use sel4::BootInfo;

use super::arch::{self, PagingContext};
use super::slots::SlotAllocator;
use super::untyped::{FRAME_SIZE, UntypedTracker};

/// Virtual address where we start mapping heap pages.
///
/// This is chosen to be well above the initial root task image to avoid
/// conflicts. The actual safe range depends on the seL4 configuration and
/// initial `VSpace` setup.
///
/// TODO: Query bootinfo for safe virtual address ranges instead of hardcoding.
const HEAP_VADDR_START: usize = 0x1_0000_0000; // 4GB mark

/// Maximum virtual address for heap allocation.
///
/// This prevents the heap from growing into other memory regions.
/// On ARM64, this ends at MMIO start to avoid overlap with device memory.
/// On `x86_64`, this is set to 8GB to provide 4GB of heap space.
const HEAP_VADDR_END: usize = 0x2_0000_0000; // 8GB mark

/// seL4-based page provider that allocates from untyped memory.
///
/// Implements [`PageProvider`] by retyping seL4 untyped capabilities into
/// frame capabilities and mapping them into the address space.
pub struct Sel4PageProvider {
    state: UnsafeCell<ProviderState>,
}

/// Internal mutable state for the page provider.
pub struct ProviderState {
    /// Pointer to boot info (valid for lifetime of root task).
    bootinfo: Option<NonNull<BootInfo>>,
    /// Tracks untyped memory allocation.
    untyped_tracker: UntypedTracker,
    /// Tracks `CNode` slot allocation.
    slot_allocator: SlotAllocator,
    /// Next virtual address to map a heap frame at.
    next_vaddr: usize,
    /// Number of frames successfully allocated.
    frames_allocated: usize,
}

impl Sel4PageProvider {
    /// Creates a new uninitialized seL4 page provider.
    ///
    /// The provider must be initialized with [`init`] before use.
    pub const fn new() -> Self {
        Self {
            state: UnsafeCell::new(ProviderState {
                bootinfo: None,
                untyped_tracker: UntypedTracker::new(),
                slot_allocator: SlotAllocator::new(),
                next_vaddr: HEAP_VADDR_START,
                frames_allocated: 0,
            }),
        }
    }

    /// Initializes the page provider with boot information.
    ///
    /// # Safety
    ///
    /// - Must be called exactly once before any allocations
    /// - The bootinfo must remain valid for the lifetime of the allocator
    /// - Must be called in single-threaded context
    pub unsafe fn init(&self, bootinfo: &BootInfo) {
        // SAFETY: Single-threaded initialization
        let state = unsafe { &mut *self.state.get() };
        // Store as NonNull pointer - bootinfo lives for the entire runtime
        state.bootinfo = Some(NonNull::from(bootinfo));
        state.slot_allocator.init(bootinfo);
    }

    /// Returns the number of frames allocated so far.
    pub fn frames_allocated(&self) -> usize {
        // SAFETY: Reading a usize is atomic on supported platforms
        unsafe { (*self.state.get()).frames_allocated }
    }

    /// Allocates a `CNode` slot for a new capability.
    ///
    /// Used by the platform layer for device frame capabilities on ARM64
    /// and `IOPort` capabilities on `x86_64`.
    /// Returns the slot index, or `None` if no slots available.
    pub fn allocate_slot(&self) -> Option<usize> {
        // SAFETY: Single-threaded access in seL4 root task
        let state = unsafe { &mut *self.state.get() };
        state.slot_allocator.allocate()
    }

    /// Maps a frame capability at the given virtual address.
    ///
    /// Creates intermediate page tables as needed. The `frame_slot` must
    /// contain a valid frame capability (already retyped from untyped).
    ///
    /// Used by the platform layer for MMIO device mapping on ARM64.
    ///
    /// Returns `true` if mapping succeeded, `false` on failure.
    ///
    /// # Safety
    ///
    /// - The provider must be initialized
    /// - `frame_slot` must contain a valid frame capability
    /// - Must be called in single-threaded context
    #[cfg(target_arch = "aarch64")]
    pub unsafe fn map_frame(&self, bootinfo: &BootInfo, frame_slot: usize, vaddr: usize) -> bool {
        // SAFETY: Single-threaded access in seL4 root task
        let state = unsafe { &mut *self.state.get() };
        let mut ctx = PagingContext {
            bootinfo,
            slot_allocator: &mut state.slot_allocator,
            untyped_tracker: &mut state.untyped_tracker,
        };
        arch::map_frame_with_page_tables(&mut ctx, frame_slot, vaddr)
    }

    /// Creates a frame from untyped memory.
    ///
    /// Returns the slot index where the new frame capability was placed.
    fn create_frame(state: &mut ProviderState, bootinfo: &BootInfo) -> Option<usize> {
        // Find an untyped region with available space
        let allocation = state.untyped_tracker.find_next_frame_untyped(bootinfo)?;

        // Allocate a CNode slot for the new frame capability
        let slot = state.slot_allocator.allocate()?;

        // Get the untyped capability
        let untyped = bootinfo.untyped().index(allocation.untyped_index).cap();

        // Get the root CNode for placing new capabilities
        let cnode = sel4::init_thread::slot::CNODE.cap();

        // Retype the untyped memory into a frame (4KB page)
        let blueprint = arch::frame_blueprint();

        let result = untyped.untyped_retype(
            &blueprint,
            &cnode.absolute_cptr_for_self(),
            slot,
            1, // Create 1 capability
        );

        match result {
            Ok(()) => Some(slot),
            Err(err) => {
                sel4::debug_println!("Failed to retype untyped into frame: {:?}", err);
                None
            }
        }
    }
}

// TODO: Replace with proper synchronization when multi-threading is added.
#[expect(
    clippy::non_send_fields_in_send_ty,
    reason = "[approved] Single-threaded seL4 root task - no concurrent access to UnsafeCell"
)]
// SAFETY: Sel4PageProvider uses UnsafeCell for interior mutability but is only
// used in single-threaded context (seL4 root task). The UnsafeCell<ProviderState>
// field is not accessed concurrently - all access is serialized by the single-threaded
// execution model of the root task.
unsafe impl Send for Sel4PageProvider {}
// SAFETY: Same as Send - single-threaded access only in seL4 root task.
unsafe impl Sync for Sel4PageProvider {}

impl PageProvider for Sel4PageProvider {
    fn allocate_page(&self) -> Option<*mut u8> {
        // SAFETY: Single-threaded access in seL4 root task
        let state = unsafe { &mut *self.state.get() };

        let bootinfo_ptr = state.bootinfo?;
        // SAFETY: The bootinfo pointer was validated during init() and remains
        // valid for the lifetime of the root task
        let bootinfo = unsafe { bootinfo_ptr.as_ref() };

        let vaddr = state.next_vaddr;

        // Check bounds to prevent heap from growing into unsafe regions
        if vaddr >= HEAP_VADDR_END {
            sel4::debug_println!(
                "Heap exhausted: reached maximum virtual address 0x{:x}",
                HEAP_VADDR_END
            );
            return None;
        }

        // Step 1: Create a frame from untyped memory
        let Some(frame_slot) = Self::create_frame(state, bootinfo) else {
            sel4::debug_println!("Failed to create frame");
            return None;
        };

        // Step 2: Map the frame, creating page tables as needed
        let mut ctx = PagingContext {
            bootinfo,
            slot_allocator: &mut state.slot_allocator,
            untyped_tracker: &mut state.untyped_tracker,
        };
        if !arch::map_frame_with_page_tables(&mut ctx, frame_slot, vaddr) {
            sel4::debug_println!("Failed to map frame at 0x{:x}", vaddr);
            return None;
        }

        // Step 3: Update state
        state.next_vaddr = vaddr.saturating_add(FRAME_SIZE);
        state.frames_allocated = state.frames_allocated.saturating_add(1);

        // Return pointer to the mapped memory
        #[expect(
            clippy::as_conversions,
            reason = "[approved] usize to pointer is required for allocators"
        )]
        Some(vaddr as *mut u8)
    }

    fn page_size(&self) -> usize {
        FRAME_SIZE
    }
}
