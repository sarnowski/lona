// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for `ProcessPool`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::pool::ProcessPool;
use crate::Vaddr;

#[test]
fn pool_initial_state() {
    let base = Vaddr::new(0x1_0000);
    let pool = ProcessPool::new(base, 1024);

    assert_eq!(pool.next(), base);
    assert_eq!(pool.limit(), base.add(1024));
    assert_eq!(pool.remaining(), 1024);
}

#[test]
fn pool_allocate_process() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 1024);

    let (young_base, old_base) = pool.allocate_process_memory(512, 256).unwrap();

    assert_eq!(young_base, base);
    assert_eq!(old_base, base.add(512));
    assert_eq!(pool.remaining(), 256);
}

#[test]
fn pool_allocate_multiple() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 2048);

    // First process
    let (young1, old1) = pool.allocate_process_memory(512, 256).unwrap();
    assert_eq!(young1, base);
    assert_eq!(old1, base.add(512));

    // Second process
    let (young2, old2) = pool.allocate_process_memory(512, 256).unwrap();
    assert_eq!(young2, base.add(768));
    assert_eq!(old2, base.add(1280));

    assert_eq!(pool.remaining(), 512);
}

#[test]
fn pool_oom() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 100);

    // Should fail - not enough space
    let result = pool.allocate_process_memory(80, 40);
    assert!(result.is_none());
}

#[test]
fn pool_extend() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 1024);

    assert_eq!(pool.remaining(), 1024);

    // Extend the pool
    pool.extend(2048);
    assert_eq!(pool.remaining(), 1024 + 2048);
    assert_eq!(pool.limit(), base.add(1024 + 2048));
}

#[test]
fn pool_allocate_with_growth_fails_in_mock() {
    // In mock mode, LMM requests fail, so growth should fail
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 100);

    // This allocation requires growth, which will fail in mock mode
    let result = pool.allocate_process_memory_with_growth(80, 40);
    assert!(result.is_none());
}

#[test]
fn pool_try_grow_fails_in_mock() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 100);

    // try_grow should fail in mock mode
    assert!(!pool.try_grow(1000));

    // Pool should be unchanged
    assert_eq!(pool.remaining(), 100);
}

// =============================================================================
// Alignment Tests
// =============================================================================

#[test]
fn pool_allocate_aligned() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 4096);

    // Allocate with 16-byte alignment
    let addr = pool.allocate(64, 16).unwrap();
    assert_eq!(
        addr.as_u64() % 16,
        0,
        "allocation should be 16-byte aligned"
    );

    // Allocate with 256-byte alignment
    let addr = pool.allocate(64, 256).unwrap();
    assert_eq!(
        addr.as_u64() % 256,
        0,
        "allocation should be 256-byte aligned"
    );
}

#[test]
fn pool_allocate_alignment_causes_gap() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 4096);

    // First allocation - unaligned size
    let first = pool.allocate(100, 1).unwrap();
    assert_eq!(first, base);

    // Second allocation with high alignment requirement
    // Pool pointer is now at 0x1_0064, next 256-aligned address is 0x1_0100
    let second = pool.allocate(64, 256).unwrap();
    assert_eq!(second.as_u64() % 256, 0, "should be 256-byte aligned");
    assert!(
        second.as_u64() > first.as_u64() + 100,
        "should have gap for alignment"
    );
}

#[test]
fn pool_allocate_1_byte_alignment() {
    let base = Vaddr::new(0x1_0001); // Intentionally unaligned base
    let mut pool = ProcessPool::new(base, 4096);

    // 1-byte alignment should work at any address
    let addr = pool.allocate(10, 1).unwrap();
    assert_eq!(addr, base);
}

#[test]
fn pool_allocate_page_alignment() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 65536);

    // Page-aligned allocation (4KB)
    let addr = pool.allocate(4096, 4096).unwrap();
    assert_eq!(addr.as_u64() % 4096, 0, "should be page-aligned");
}

// =============================================================================
// Boundary and Edge Case Tests
// =============================================================================

#[test]
fn pool_allocate_exact_fit() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 1024);

    // Allocate exactly the remaining space
    let addr = pool.allocate(1024, 1).unwrap();
    assert_eq!(addr, base);
    assert_eq!(pool.remaining(), 0);

    // Next allocation should fail
    assert!(pool.allocate(1, 1).is_none());
}

#[test]
fn pool_allocate_one_byte_remaining() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 1024);

    // Allocate all but one byte
    pool.allocate(1023, 1).unwrap();
    assert_eq!(pool.remaining(), 1);

    // Should be able to allocate that last byte
    let addr = pool.allocate(1, 1).unwrap();
    assert!(addr.as_u64() > 0);
    assert_eq!(pool.remaining(), 0);
}

#[test]
fn pool_allocate_zero_size() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 1024);

    // Zero-size allocation should succeed (returns current pointer)
    let addr = pool.allocate(0, 1).unwrap();
    assert_eq!(addr, base);

    // Pool should be unchanged
    assert_eq!(pool.remaining(), 1024);
    assert_eq!(pool.next(), base);
}

#[test]
fn pool_allocate_process_memory_zero_sizes() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 1024);

    // Zero young, non-zero old
    let (young, old) = pool.allocate_process_memory(0, 100).unwrap();
    assert_eq!(young, base);
    assert_eq!(old, base);

    // Non-zero young, zero old
    let (young2, old2) = pool.allocate_process_memory(100, 0).unwrap();
    assert_eq!(old2, young2.add(100));
}

#[test]
fn pool_new_zero_size() {
    let base = Vaddr::new(0x1_0000);
    let pool = ProcessPool::new(base, 0);

    assert_eq!(pool.remaining(), 0);
    assert_eq!(pool.next(), base);
    assert_eq!(pool.limit(), base);
}

#[test]
fn pool_new_max_size() {
    // Large pool (but not overflowing)
    let base = Vaddr::new(0x1000);
    let pool = ProcessPool::new(base, usize::MAX - 0x2000);

    // Should handle large sizes without panic
    assert!(pool.remaining() > 0);
}

// =============================================================================
// Overflow Protection Tests
// =============================================================================

#[test]
fn pool_allocate_size_overflow() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 1024);

    // Allocation larger than pool - should return None
    assert!(pool.allocate(2048, 1).is_none());

    // Pool should be unchanged
    assert_eq!(pool.remaining(), 1024);
}

#[test]
fn pool_allocate_near_limit_fails_gracefully() {
    // Test that allocations fail gracefully when near pool limits
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 1000);

    // Allocation that exceeds remaining space should return None
    let result = pool.allocate(2000, 1);
    assert!(result.is_none());

    // Pool should be unchanged after failed allocation
    assert_eq!(pool.remaining(), 1000);
}

#[test]
fn pool_allocate_alignment_exceeds_limit() {
    // Test that alignment that pushes allocation past limit fails
    // Base is NOT aligned to 4096, so aligning up will push past limit
    let base = Vaddr::new(0x1_0001); // Not 4096-aligned
    let mut pool = ProcessPool::new(base, 1000);

    // Aligning 0x1_0001 up to 4096 gives 0x1_1000, which is far beyond limit
    let result = pool.allocate(100, 4096);
    assert!(result.is_none());
}

#[test]
fn pool_allocate_process_memory_overflow() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 1024);

    // young + old would overflow usize
    let result = pool.allocate_process_memory(usize::MAX, 1);
    assert!(result.is_none());

    // Pool should be unchanged
    assert_eq!(pool.remaining(), 1024);
}

#[test]
fn pool_new_saturating_limit() {
    // Base + size would overflow u64
    let base = Vaddr::new(u64::MAX - 100);
    let pool = ProcessPool::new(base, 1000);

    // Limit should be saturated to u64::MAX, not wrap around
    assert!(pool.limit().as_u64() >= base.as_u64());
}

#[test]
fn pool_extend_saturating() {
    let base = Vaddr::new(u64::MAX - 100);
    let mut pool = ProcessPool::new(base, 50);

    // Extending by a large amount should saturate
    pool.extend(usize::MAX);

    // Limit should be saturated, not wrapped
    assert!(pool.limit().as_u64() >= base.as_u64());
}

// =============================================================================
// Sequential Allocation Tests
// =============================================================================

#[test]
fn pool_sequential_allocations_contiguous() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 4096);

    let a1 = pool.allocate(100, 1).unwrap();
    let a2 = pool.allocate(100, 1).unwrap();
    let a3 = pool.allocate(100, 1).unwrap();

    // Allocations should be contiguous
    assert_eq!(a2.as_u64(), a1.as_u64() + 100);
    assert_eq!(a3.as_u64(), a2.as_u64() + 100);
}

#[test]
fn pool_many_small_allocations() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 10000);

    // Allocate 100 small chunks
    for i in 0_u64..100 {
        let addr = pool.allocate(64, 1).unwrap();
        assert_eq!(addr.as_u64(), base.as_u64() + i * 64);
    }

    assert_eq!(pool.remaining(), 10000 - (100 * 64));
}

#[test]
fn pool_allocate_multiple_processes() {
    let base = Vaddr::new(0x1_0000);
    // Need enough space: sum of (1024 + 2048) * (i+1) for i=0..10 = 3072 * 55 = 168,960
    let mut pool = ProcessPool::new(base, 200_000);

    // Simulate allocating memory for multiple processes
    let mut last_old_end = base;

    for i in 0..10 {
        let young_size = 1024 * (i + 1);
        let old_size = 2048 * (i + 1);

        let (young, old) = pool.allocate_process_memory(young_size, old_size).unwrap();

        // Young should be at or after last allocation
        assert!(young.as_u64() >= last_old_end.as_u64());

        // Old should be immediately after young
        assert_eq!(old.as_u64(), young.as_u64() + young_size as u64);

        last_old_end = Vaddr::new(old.as_u64() + old_size as u64);
    }
}

// =============================================================================
// Growth Mechanism Tests (Mock Mode)
// =============================================================================

#[test]
fn pool_allocate_with_growth_succeeds_when_space_available() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 10000);

    // This should succeed without needing growth
    let result = pool.allocate_process_memory_with_growth(1000, 2000);
    assert!(result.is_some());
}

#[test]
fn pool_allocate_with_growth_fails_when_growth_needed() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 100);

    // This needs growth, which fails in mock mode
    let result = pool.allocate_process_memory_with_growth(1000, 2000);
    assert!(result.is_none());
}

#[test]
fn pool_try_grow_various_sizes() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 100);

    // All growth attempts fail in mock mode
    assert!(!pool.try_grow(1));
    assert!(!pool.try_grow(4096));
    assert!(!pool.try_grow(1_000_000));
    assert!(!pool.try_grow(usize::MAX));

    // Pool should remain unchanged
    assert_eq!(pool.remaining(), 100);
}

// =============================================================================
// Extend Tests
// =============================================================================

#[test]
fn pool_extend_increases_capacity() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 1024);

    let original_remaining = pool.remaining();
    pool.extend(2048);

    assert_eq!(pool.remaining(), original_remaining + 2048);
}

#[test]
fn pool_extend_after_allocation() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 1024);

    // Allocate some
    pool.allocate(500, 1).unwrap();
    assert_eq!(pool.remaining(), 524);

    // Extend
    pool.extend(1000);
    assert_eq!(pool.remaining(), 1524);

    // Should be able to allocate more
    let addr = pool.allocate(1000, 1).unwrap();
    assert!(addr.as_u64() > 0);
}

#[test]
fn pool_extend_zero() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 1024);

    let original = pool.remaining();
    pool.extend(0);

    assert_eq!(pool.remaining(), original);
}

// =============================================================================
// State Inspection Tests
// =============================================================================

#[test]
fn pool_next_advances_correctly() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 4096);

    assert_eq!(pool.next(), base);

    pool.allocate(100, 1).unwrap();
    assert_eq!(pool.next().as_u64(), base.as_u64() + 100);

    pool.allocate(200, 1).unwrap();
    assert_eq!(pool.next().as_u64(), base.as_u64() + 300);
}

#[test]
fn pool_limit_unchanged_by_allocation() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 4096);

    let original_limit = pool.limit();

    pool.allocate(100, 1).unwrap();
    assert_eq!(pool.limit(), original_limit);

    pool.allocate(200, 1).unwrap();
    assert_eq!(pool.limit(), original_limit);
}

#[test]
fn pool_remaining_decreases_correctly() {
    let base = Vaddr::new(0x1_0000);
    let mut pool = ProcessPool::new(base, 1000);

    assert_eq!(pool.remaining(), 1000);

    pool.allocate(100, 1).unwrap();
    assert_eq!(pool.remaining(), 900);

    pool.allocate(200, 1).unwrap();
    assert_eq!(pool.remaining(), 700);

    // Extend increases remaining
    pool.extend(500);
    assert_eq!(pool.remaining(), 1200);
}
