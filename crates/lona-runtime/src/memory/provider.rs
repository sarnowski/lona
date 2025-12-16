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
use sel4::cap_type::{PT, SmallPage};
use sel4::{BootInfo, Cap, CapRights, ObjectBlueprint, ObjectBlueprintArm, VmAttributes};

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
/// This prevents the heap from growing into kernel space or device memory.
/// Set to 128GB to leave plenty of room while staying safely in user space.
const HEAP_VADDR_END: usize = 0x20_0000_0000; // 128GB mark

/// seL4-based page provider that allocates from untyped memory.
///
/// Implements [`PageProvider`] by retyping seL4 untyped capabilities into
/// frame capabilities and mapping them into the address space.
pub struct Sel4PageProvider {
    state: UnsafeCell<ProviderState>,
}

/// Internal mutable state for the page provider.
struct ProviderState {
    /// Pointer to boot info (valid for lifetime of root task).
    bootinfo: Option<NonNull<BootInfo>>,
    /// Tracks untyped memory allocation.
    untyped_tracker: UntypedTracker,
    /// Tracks `CNode` slot allocation.
    slot_allocator: SlotAllocator,
    /// Next virtual address to map a frame at.
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

        // Retype the untyped memory into a frame (SmallPage = 4KB on ARM64)
        let blueprint = ObjectBlueprint::Arch(ObjectBlueprintArm::SmallPage);

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

    /// Creates a page table from untyped memory.
    ///
    /// Returns the slot index where the new page table capability was placed.
    fn create_page_table(state: &mut ProviderState, bootinfo: &BootInfo) -> Option<usize> {
        // Allocate a CNode slot for the page table capability
        let pt_slot = state.slot_allocator.allocate()?;

        // Find untyped memory for the page table
        let allocation = state.untyped_tracker.find_next_frame_untyped(bootinfo)?;

        let untyped = bootinfo.untyped().index(allocation.untyped_index).cap();
        let cnode = sel4::init_thread::slot::CNODE.cap();

        // Create a page table object
        let blueprint = ObjectBlueprint::Arch(ObjectBlueprintArm::PT);

        match untyped.untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), pt_slot, 1) {
            Ok(()) => Some(pt_slot),
            Err(err) => {
                sel4::debug_println!("Failed to retype untyped into page table: {:?}", err);
                None
            }
        }
    }

    /// Maps a page table at the given virtual address.
    ///
    /// Returns `Ok(true)` if the page table was mapped successfully.
    /// Returns `Ok(false)` if a page table already exists (`DeleteFirst` error).
    /// Returns `Err(())` on other failures.
    fn map_page_table(pt_slot: usize, vaddr: usize) -> Result<bool, ()> {
        let pt_cap: Cap<PT> = Cap::from_bits(u64::try_from(pt_slot).unwrap_or(0));
        let vspace = sel4::init_thread::slot::VSPACE.cap();

        match pt_cap.pt_map(vspace, vaddr, VmAttributes::default()) {
            Ok(()) => {
                sel4::debug_println!("Mapped page table at 0x{:x}", vaddr);
                Ok(true)
            }
            Err(sel4::Error::DeleteFirst) => {
                // A page table already exists at this level - that's fine
                Ok(false)
            }
            Err(err) => {
                sel4::debug_println!("Page table map failed at 0x{:x}: {:?}", vaddr, err);
                Err(())
            }
        }
    }

    /// Tries to map a frame, creating page tables as needed.
    ///
    /// On ARM64, we may need to create intermediate page table structures
    /// (up to 4 levels) before we can map a frame. This function attempts
    /// to map the frame and creates page tables only when necessary.
    fn map_frame_with_page_tables(
        state: &mut ProviderState,
        bootinfo: &BootInfo,
        frame_slot: usize,
        vaddr: usize,
    ) -> bool {
        let vspace = sel4::init_thread::slot::VSPACE.cap();
        let frame_cap: Cap<SmallPage> = Cap::from_bits(u64::try_from(frame_slot).unwrap_or(0));

        // ARM64 has up to 4 levels of page tables. We try mapping the frame,
        // and if it fails due to missing page tables, we create them one at a time.
        for _ in 0..4 {
            let result = frame_cap.frame_map(
                vspace,
                vaddr,
                CapRights::read_write(),
                VmAttributes::default(),
            );

            match result {
                Ok(()) => return true,
                Err(sel4::Error::FailedLookup) => {
                    // Missing page table - create and map one
                    let Some(pt_slot) = Self::create_page_table(state, bootinfo) else {
                        sel4::debug_println!("Failed to create page table");
                        return false;
                    };

                    if Self::map_page_table(pt_slot, vaddr).is_err() {
                        return false;
                    }
                    // Page table mapped (or already existed), retry frame mapping
                }
                Err(err) => {
                    sel4::debug_println!("Frame map failed at 0x{:x}: {:?}", vaddr, err);
                    return false;
                }
            }
        }

        // If we get here, we created 4 page tables but still can't map - something is wrong
        sel4::debug_println!(
            "Failed to map frame after creating 4 page tables at 0x{:x}",
            vaddr
        );
        false
    }
}

// SAFETY: Sel4PageProvider uses UnsafeCell for interior mutability but is only
// used in single-threaded context (seL4 root task).
// TODO: Replace with proper synchronization when multi-threading is added.
unsafe impl Send for Sel4PageProvider {}
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
        if !Self::map_frame_with_page_tables(state, bootinfo, frame_slot, vaddr) {
            sel4::debug_println!("Failed to map frame at 0x{:x}", vaddr);
            return None;
        }

        sel4::debug_println!(
            "Allocated and mapped frame {} at 0x{:x}",
            state.frames_allocated,
            vaddr
        );

        // Step 3: Update state
        state.next_vaddr = vaddr.saturating_add(FRAME_SIZE);
        state.frames_allocated = state.frames_allocated.saturating_add(1);

        // Return pointer to the mapped memory
        #[expect(
            clippy::as_conversions,
            reason = "usize to pointer is required for allocators"
        )]
        Some(vaddr as *mut u8)
    }

    fn page_size(&self) -> usize {
        FRAME_SIZE
    }
}
