// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the memory allocator.

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

use super::{Allocator, PageProvider, align_up};

/// Mock page provider for testing.
struct MockPageProvider {
    /// Index of the next page to allocate.
    next_page: AtomicUsize,
    /// Static pages to allocate from.
    pages: &'static [&'static [u8]],
}

impl MockPageProvider {
    /// Page size used in tests.
    const PAGE_SIZE: usize = 4096;

    /// Creates a new mock provider with the given pages.
    fn new(pages: &'static [&'static [u8]]) -> Self {
        Self {
            next_page: AtomicUsize::new(0),
            pages,
        }
    }
}

impl PageProvider for MockPageProvider {
    fn allocate_page(&self) -> Option<*mut u8> {
        let index = self.next_page.fetch_add(1, Ordering::Relaxed);
        self.pages.get(index).map(|page| {
            #[expect(
                clippy::as_conversions,
                reason = "[approved] slice to pointer for test mock"
            )]
            {
                page.as_ptr() as *mut u8
            }
        })
    }

    fn page_size(&self) -> usize {
        Self::PAGE_SIZE
    }
}

// Static buffers for mock pages (must be properly aligned)
#[repr(align(4096))]
struct AlignedPage([u8; 4096]);

static PAGE1: AlignedPage = AlignedPage([0; 4096]);
static PAGE2: AlignedPage = AlignedPage([0; 4096]);

static PAGES: [&[u8]; 2] = [&PAGE1.0, &PAGE2.0];

#[test]
fn test_align_up() {
    assert_eq!(align_up(0, 8), 0);
    assert_eq!(align_up(1, 8), 8);
    assert_eq!(align_up(7, 8), 8);
    assert_eq!(align_up(8, 8), 8);
    assert_eq!(align_up(9, 8), 16);
    assert_eq!(align_up(4095, 4096), 4096);
    assert_eq!(align_up(4096, 4096), 4096);
}

#[test]
fn test_multi_page_allocation_succeeds() {
    let provider = MockPageProvider::new(&PAGES);
    let allocator = Allocator::new(provider);

    // Allocate more than one page - should succeed with 2 pages available
    // Note: In real seL4, pages are mapped contiguously. The mock doesn't provide
    // truly contiguous memory, but we're only checking allocation success here.
    let large_layout = Layout::from_size_align(6144, 8).unwrap(); // 1.5 pages
    // SAFETY: Test allocation
    let ptr = unsafe { allocator.alloc(large_layout) };
    assert!(!ptr.is_null());

    let stats = allocator.stats();
    assert_eq!(stats.pages_allocated, 2);
    assert_eq!(stats.total_allocated, 6144);
}

#[test]
fn test_allocation_fails_when_not_enough_pages() {
    // Only provide one page
    static SINGLE_PAGE_ONLY: [&[u8]; 1] = [&PAGE1.0];
    let provider = MockPageProvider::new(&SINGLE_PAGE_ONLY);
    let allocator = Allocator::new(provider);

    // Try to allocate more than one page - should fail with only 1 page available
    let huge_layout = Layout::from_size_align(8192, 8).unwrap();
    // SAFETY: Test allocation
    let ptr = unsafe { allocator.alloc(huge_layout) };
    assert!(ptr.is_null());
}

#[test]
fn test_allocation_respects_alignment() {
    let provider = MockPageProvider::new(&PAGES);
    let allocator = Allocator::new(provider);

    // First allocation with small alignment
    let layout1 = Layout::from_size_align(1, 1).unwrap();
    // SAFETY: Test allocation
    let ptr1 = unsafe { allocator.alloc(layout1) };
    assert!(!ptr1.is_null());

    // Second allocation requiring 256-byte alignment
    let layout2 = Layout::from_size_align(64, 256).unwrap();
    // SAFETY: Test allocation
    let ptr2 = unsafe { allocator.alloc(layout2) };
    assert!(!ptr2.is_null());
    #[expect(
        clippy::as_conversions,
        reason = "[approved] pointer to usize for alignment check"
    )]
    let addr = ptr2 as usize;
    assert_eq!(addr % 256, 0, "Pointer not aligned to 256");
}

#[test]
fn test_allocation_spans_pages() {
    let provider = MockPageProvider::new(&PAGES);
    let allocator = Allocator::new(provider);

    // Allocate nearly a full page
    let large_layout = Layout::from_size_align(4000, 8).unwrap();
    // SAFETY: Test allocation
    let ptr1 = unsafe { allocator.alloc(large_layout) };
    assert!(!ptr1.is_null());

    // This allocation should require a new page
    let small_layout = Layout::from_size_align(200, 8).unwrap();
    // SAFETY: Test allocation
    let ptr2 = unsafe { allocator.alloc(small_layout) };
    assert!(!ptr2.is_null());

    let stats = allocator.stats();
    assert_eq!(stats.pages_allocated, 2);
}

#[test]
fn test_first_allocation_requests_page() {
    let provider = MockPageProvider::new(&PAGES);
    let allocator = Allocator::new(provider);

    let stats_before = allocator.stats();
    assert_eq!(stats_before.pages_allocated, 0);

    let layout = Layout::from_size_align(64, 8).unwrap();
    // SAFETY: Test allocation
    let ptr = unsafe { allocator.alloc(layout) };

    assert!(!ptr.is_null());
    let stats_after = allocator.stats();
    assert_eq!(stats_after.pages_allocated, 1);
    assert_eq!(stats_after.total_allocated, 64);
}

#[test]
fn test_multiple_allocations_same_page() {
    let provider = MockPageProvider::new(&PAGES);
    let allocator = Allocator::new(provider);

    let layout = Layout::from_size_align(100, 8).unwrap();

    // Allocate multiple times within one page
    for i in 0..10 {
        // SAFETY: Test allocation
        let ptr = unsafe { allocator.alloc(layout) };
        assert!(!ptr.is_null(), "Allocation {} failed", i);
    }

    let stats = allocator.stats();
    // 10 allocations of 100 bytes each = 1000 bytes minimum
    // With alignment, we should still fit in one 4KB page
    assert_eq!(stats.pages_allocated, 1);
    assert_eq!(stats.total_allocated, 1000);
}

#[test]
fn test_out_of_pages_returns_null() {
    // Only provide one page
    static SINGLE_PAGE: [&[u8]; 1] = [&PAGE1.0];
    let provider = MockPageProvider::new(&SINGLE_PAGE);
    let allocator = Allocator::new(provider);

    // Fill the first page
    let large_layout = Layout::from_size_align(4000, 8).unwrap();
    // SAFETY: Test allocation
    let ptr1 = unsafe { allocator.alloc(large_layout) };
    assert!(!ptr1.is_null());

    // This should fail - no more pages available
    // SAFETY: Test allocation
    let ptr2 = unsafe { allocator.alloc(large_layout) };
    assert!(ptr2.is_null());
}

#[test]
fn test_stats_tracking() {
    let provider = MockPageProvider::new(&PAGES);
    let allocator = Allocator::new(provider);

    let stats = allocator.stats();
    assert_eq!(stats.total_allocated, 0);
    assert_eq!(stats.pages_allocated, 0);
    assert_eq!(stats.page_size, 4096);
    assert_eq!(stats.total_reserved(), 0);

    let layout = Layout::from_size_align(256, 8).unwrap();
    // SAFETY: Test allocation
    unsafe { allocator.alloc(layout) };

    let stats = allocator.stats();
    assert_eq!(stats.total_allocated, 256);
    assert_eq!(stats.pages_allocated, 1);
    assert_eq!(stats.total_reserved(), 4096);
}
