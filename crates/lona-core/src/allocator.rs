// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Memory allocation primitives for the Lona runtime.
//!
//! Provides a trait-based abstraction for page allocation and a bump
//! allocator that can work with any page provider. This design enables
//! host-based testing with mock page providers while using seL4 untyped
//! memory in production.

use core::alloc::{GlobalAlloc, Layout};
use core::cell::UnsafeCell;
use core::ptr;

// ============================================================================
// PageProvider trait
// ============================================================================

/// Provides memory pages to the allocator on demand.
///
/// Implementations are responsible for obtaining physical memory and mapping
/// it into the current address space. On seL4, this involves retyping untyped
/// memory into frames and mapping them via the `VSpace`.
///
/// Implementations must ensure that:
/// - Returned pointers are valid and properly aligned to `page_size()`
/// - Each page is only returned once (no double-allocation)
/// - The memory remains valid for the lifetime of the allocator
pub trait PageProvider: Send + Sync {
    /// Allocates a new page of memory.
    ///
    /// Returns a pointer to the start of a newly allocated page, or `None`
    /// if no more pages are available.
    fn allocate_page(&self) -> Option<*mut u8>;

    /// Returns the page size in bytes (typically 4096).
    fn page_size(&self) -> usize;
}

/// Blanket implementation for references to page providers.
///
/// This enables using `&'static Provider` in const contexts for global allocators.
impl<P: PageProvider> PageProvider for &P {
    #[inline]
    fn allocate_page(&self) -> Option<*mut u8> {
        (*self).allocate_page()
    }

    #[inline]
    fn page_size(&self) -> usize {
        (*self).page_size()
    }
}

// ============================================================================
// Bump Allocator
// ============================================================================

/// A simple bump allocator that allocates memory linearly.
///
/// Memory is allocated by incrementing a pointer ("bumping" it forward).
/// When the current page is exhausted, a new page is requested from the
/// underlying [`PageProvider`].
///
/// # Thread Safety
///
/// This allocator uses interior mutability with `UnsafeCell` and is marked
/// `Sync` because seL4 runs a single thread per address space in our initial
/// implementation. For multi-threaded use, wrap in a spinlock.
pub struct Allocator<P: PageProvider> {
    /// The page provider that supplies memory pages.
    provider: P,
    /// Internal mutable state protected by single-threaded access assumption.
    state: UnsafeCell<AllocatorState>,
}

/// Internal mutable state of the bump allocator.
struct AllocatorState {
    /// End of the current page (exclusive).
    current_page_end: usize,
    /// Start of the current page being allocated from.
    current_page_start: usize,
    /// Current allocation pointer (the "bump" pointer).
    next: usize,
    /// Number of pages obtained from the provider.
    pages_allocated: usize,
    /// Total bytes allocated across all pages.
    total_allocated: usize,
}

impl<P: PageProvider> Allocator<P> {
    /// Attempts to allocate memory with the given layout.
    ///
    /// Returns a pointer to the allocated memory, or null if allocation fails.
    /// Supports allocations larger than a single page by allocating multiple
    /// contiguous pages.
    fn alloc_inner(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Single-threaded access is guaranteed because:
        // 1. seL4 root task runs in a single-threaded context
        // 2. This allocator is used as the global allocator (#[global_allocator])
        // 3. seL4's IPC model means no preemptive interrupts allocate memory
        // 4. UnsafeCell requires exclusive access which we have in single-threaded context
        // TODO: For multi-threaded use (Phase 10), wrap in spin::Mutex or similar.
        let state = unsafe { &mut *self.state.get() };

        let page_size = self.provider.page_size();

        // Align the next pointer up to the required alignment
        let alloc_start = align_up(state.next, layout.align());
        let Some(alloc_end) = alloc_start.checked_add(layout.size()) else {
            return ptr::null_mut();
        };

        // Check if we have enough space in the current page(s)
        if alloc_end <= state.current_page_end {
            state.next = alloc_end;
            state.total_allocated = state.total_allocated.saturating_add(layout.size());
            // Converting usize address to pointer - this is the standard pattern for allocators
            #[expect(
                clippy::as_conversions,
                reason = "usize to pointer is required for allocators"
            )]
            return alloc_start as *mut u8;
        }

        // Need more pages - calculate how many
        // For simplicity, start allocation at a fresh page boundary for large allocations
        let pages_needed = layout
            .size()
            .saturating_add(page_size.saturating_sub(1))
            .checked_div(page_size)
            .unwrap_or(0);

        if pages_needed == 0 {
            return ptr::null_mut();
        }

        // Allocate the required number of pages
        // Pages are mapped at contiguous virtual addresses by the provider
        let Some(first_page) = self.provider.allocate_page() else {
            return ptr::null_mut();
        };

        // Converting pointer to usize for arithmetic
        #[expect(
            clippy::as_conversions,
            reason = "pointer to usize is required for address arithmetic"
        )]
        let first_page_start = first_page as usize;

        state.pages_allocated = state.pages_allocated.saturating_add(1);

        // Allocate additional pages if needed (they will be contiguous)
        for _ in 1_usize..pages_needed {
            if self.provider.allocate_page().is_none() {
                // Failed to allocate enough pages
                // Note: In a bump allocator, we can't free the pages we already allocated
                return ptr::null_mut();
            }
            state.pages_allocated = state.pages_allocated.saturating_add(1);
        }

        // Calculate the end of all allocated pages
        let total_page_space = pages_needed.saturating_mul(page_size);
        let page_end = first_page_start.saturating_add(total_page_space);

        // Update state with new page range
        state.current_page_start = first_page_start;
        state.current_page_end = page_end;

        // Align within the new pages
        let new_alloc_start = align_up(first_page_start, layout.align());
        let new_alloc_end = new_alloc_start.saturating_add(layout.size());

        // Verify the allocation fits
        if new_alloc_end > page_end {
            // Alignment pushed us past the end - shouldn't happen with proper page count
            return ptr::null_mut();
        }

        state.next = new_alloc_end;
        state.total_allocated = state.total_allocated.saturating_add(layout.size());

        // Converting usize address to pointer - this is the standard pattern for allocators
        #[expect(
            clippy::as_conversions,
            reason = "usize to pointer is required for allocators"
        )]
        {
            new_alloc_start as *mut u8
        }
    }

    /// Creates a new bump allocator with the given page provider.
    ///
    /// No pages are allocated until the first allocation request.
    #[inline]
    pub const fn new(provider: P) -> Self {
        Self {
            provider,
            state: UnsafeCell::new(AllocatorState {
                current_page_end: 0,
                current_page_start: 0,
                next: 0,
                pages_allocated: 0,
                total_allocated: 0,
            }),
        }
    }

    /// Returns statistics about this allocator's memory usage.
    #[inline]
    pub fn stats(&self) -> Stats {
        // SAFETY: Single-threaded access in seL4 root task
        let state = unsafe { &*self.state.get() };
        Stats {
            page_size: self.provider.page_size(),
            pages_allocated: state.pages_allocated,
            total_allocated: state.total_allocated,
        }
    }
}

/// Statistics about allocator memory usage.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Stats {
    /// Size of each page in bytes.
    pub page_size: usize,
    /// Number of pages obtained from the provider.
    pub pages_allocated: usize,
    /// Total bytes allocated (not including alignment padding).
    pub total_allocated: usize,
}

impl Stats {
    /// Returns the total memory reserved from the page provider.
    #[inline]
    #[must_use]
    pub const fn total_reserved(&self) -> usize {
        self.pages_allocated.saturating_mul(self.page_size)
    }
}

// SAFETY: Allocator uses UnsafeCell for interior mutability but is only
// used in single-threaded contexts (seL4 root task has one thread per domain).
// For multi-threaded use, this would need synchronization.
unsafe impl<P: PageProvider> Sync for Allocator<P> {}

// SAFETY: GlobalAlloc implementation is safe because:
// - We only access memory through the PageProvider which guarantees valid pages
// - Interior mutability is protected by single-threaded access assumption
// - All pointer arithmetic is bounds-checked
unsafe impl<P: PageProvider> GlobalAlloc for Allocator<P> {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.alloc_inner(layout)
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = self.alloc_inner(layout);
        if !ptr.is_null() {
            // SAFETY: ptr is valid and properly aligned for layout.size() bytes
            unsafe {
                ptr::write_bytes(ptr, 0, layout.size());
            }
        }
        ptr
    }

    #[inline]
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator does not support deallocation.
        // Memory will be reclaimed by the garbage collector at process level.
    }

    #[inline]
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // Simple implementation: allocate new, copy, (don't free old - it's a bump allocator)
        let Ok(new_layout) = Layout::from_size_align(new_size, layout.align()) else {
            return ptr::null_mut();
        };

        let new_ptr = self.alloc_inner(new_layout);
        if new_ptr.is_null() {
            return ptr::null_mut();
        }

        // Copy the old data to the new location
        let copy_size = layout.size().min(new_size);
        // SAFETY: Both pointers are valid, non-overlapping (bump allocator), and properly aligned
        unsafe {
            ptr::copy_nonoverlapping(ptr, new_ptr, copy_size);
        }

        new_ptr
    }
}

/// Aligns `addr` upwards to the given alignment.
///
/// Alignment must be a power of two.
#[inline]
const fn align_up(addr: usize, align: usize) -> usize {
    // This works because align is a power of two:
    // align - 1 creates a mask of the lower bits
    // Adding this mask and then clearing with NOT mask rounds up
    //
    // Using wrapping operations to satisfy clippy's arithmetic_side_effects lint.
    // This is safe because:
    // - align is always a power of two (guaranteed by Layout)
    // - The result is always >= addr (alignment can only increase)
    let mask = align.wrapping_sub(1);
    addr.wrapping_add(mask) & !mask
}

#[cfg(test)]
#[path = "allocator_tests.rs"]
mod tests;
