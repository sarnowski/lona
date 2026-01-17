// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for IPC message types.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::*;

#[test]
fn message_tag_classification() {
    assert!(MessageTag::AllocPages.is_request());
    assert!(!MessageTag::AllocPages.is_response());

    assert!(!MessageTag::Success.is_request());
    assert!(MessageTag::Success.is_response());
    assert!(MessageTag::Success.is_success());
    assert!(!MessageTag::Success.is_error());

    assert!(MessageTag::ErrorOutOfMemory.is_response());
    assert!(MessageTag::ErrorOutOfMemory.is_error());
    assert!(!MessageTag::ErrorOutOfMemory.is_success());
}

#[test]
fn message_tag_round_trip() {
    let tags = [
        MessageTag::AllocPages,
        MessageTag::Success,
        MessageTag::ErrorOutOfMemory,
        MessageTag::ErrorInvalidRequest,
    ];

    for tag in tags {
        let value = tag as u64;
        let decoded = MessageTag::from_u64(value).unwrap();
        assert_eq!(decoded, tag);
    }
}

#[test]
fn message_tag_invalid() {
    assert!(MessageTag::from_u64(0).is_none());
    assert!(MessageTag::from_u64(2).is_none());
    assert!(MessageTag::from_u64(127).is_none());
    assert!(MessageTag::from_u64(131).is_none());
    assert!(MessageTag::from_u64(u64::MAX).is_none());
}

#[test]
fn ipc_region_type_round_trip() {
    let regions = [
        IpcRegionType::ProcessPool,
        IpcRegionType::RealmBinary,
        IpcRegionType::RealmLocal,
    ];

    for region in regions {
        let value = region as u64;
        let decoded = IpcRegionType::from_u64(value).unwrap();
        assert_eq!(decoded, region);
    }
}

#[test]
fn ipc_region_type_invalid() {
    assert!(IpcRegionType::from_u64(0).is_none());
    assert!(IpcRegionType::from_u64(4).is_none());
    assert!(IpcRegionType::from_u64(u64::MAX).is_none());
}

#[test]
fn alloc_pages_request_encode_decode() {
    let request = AllocPagesRequest::new(IpcRegionType::ProcessPool, 10, Vaddr::new(0x1000_0000));

    let mrs = request.to_mrs();
    assert_eq!(mrs[0], MessageTag::AllocPages as u64);
    assert_eq!(mrs[1], IpcRegionType::ProcessPool as u64);
    assert_eq!(mrs[2], 10);
    assert_eq!(mrs[3], 0x1000_0000);

    let decoded = AllocPagesRequest::from_mrs(mrs).unwrap();
    assert_eq!(decoded.tag, MessageTag::AllocPages);
    assert_eq!(decoded.region, IpcRegionType::ProcessPool);
    assert_eq!(decoded.page_count, 10);
    assert_eq!(decoded.hint_vaddr, Vaddr::new(0x1000_0000));
}

#[test]
fn alloc_pages_request_hint_zero() {
    let request = AllocPagesRequest::new(IpcRegionType::RealmBinary, 5, Vaddr::null());

    let mrs = request.to_mrs();
    assert_eq!(mrs[3], 0);

    let decoded = AllocPagesRequest::from_mrs(mrs).unwrap();
    assert!(decoded.hint_vaddr.is_null());
}

#[test]
fn alloc_pages_request_invalid_tag() {
    let mrs = [MessageTag::Success as u64, 1, 10, 0];
    assert!(AllocPagesRequest::from_mrs(mrs).is_none());
}

#[test]
fn alloc_pages_request_invalid_region() {
    let mrs = [MessageTag::AllocPages as u64, 99, 10, 0];
    assert!(AllocPagesRequest::from_mrs(mrs).is_none());
}

#[test]
fn alloc_pages_response_success() {
    let response = AllocPagesResponse::success(Vaddr::new(0x2000_0000), 10);

    assert!(response.is_success());
    assert_eq!(response.tag, MessageTag::Success);
    assert_eq!(response.vaddr, Vaddr::new(0x2000_0000));
    assert_eq!(response.page_count, 10);

    let mrs = response.to_mrs();
    assert_eq!(mrs[0], MessageTag::Success as u64);
    assert_eq!(mrs[1], 0x2000_0000);
    assert_eq!(mrs[2], 10);

    let decoded = AllocPagesResponse::from_mrs(mrs).unwrap();
    assert!(decoded.is_success());
    assert_eq!(decoded.vaddr, Vaddr::new(0x2000_0000));
}

#[test]
fn alloc_pages_response_error_oom() {
    let response = AllocPagesResponse::error_out_of_memory();

    assert!(!response.is_success());
    assert_eq!(response.tag, MessageTag::ErrorOutOfMemory);
    assert!(response.vaddr.is_null());
    assert_eq!(response.page_count, 0);

    let mrs = response.to_mrs();
    let decoded = AllocPagesResponse::from_mrs(mrs).unwrap();
    assert_eq!(decoded.tag, MessageTag::ErrorOutOfMemory);
}

#[test]
fn alloc_pages_response_error_invalid() {
    let response = AllocPagesResponse::error_invalid_request();

    assert!(!response.is_success());
    assert_eq!(response.tag, MessageTag::ErrorInvalidRequest);
}

#[test]
fn alloc_pages_response_invalid_tag() {
    let mrs = [MessageTag::AllocPages as u64, 0x1000, 10];
    assert!(AllocPagesResponse::from_mrs(mrs).is_none());
}

#[test]
fn lmm_error_from_tag() {
    assert_eq!(
        LmmError::from_tag(MessageTag::ErrorOutOfMemory),
        Some(LmmError::OutOfMemory)
    );
    assert_eq!(
        LmmError::from_tag(MessageTag::ErrorInvalidRequest),
        Some(LmmError::InvalidRequest)
    );
    assert_eq!(LmmError::from_tag(MessageTag::Success), None);
    assert_eq!(LmmError::from_tag(MessageTag::AllocPages), None);
}

// =============================================================================
// Region Validation Tests
// =============================================================================

use crate::layout::{
    PAGE_SIZE, PROCESS_POOL_BASE, PROCESS_POOL_SIZE, REALM_BINARY_BASE, REALM_BINARY_SIZE,
    REALM_LOCAL_BASE, REALM_LOCAL_SIZE,
};

#[test]
fn region_bounds_correct() {
    // ProcessPool
    let (base, limit) = IpcRegionType::ProcessPool.bounds();
    assert_eq!(base, PROCESS_POOL_BASE);
    assert_eq!(limit, PROCESS_POOL_BASE + PROCESS_POOL_SIZE);

    // RealmBinary
    let (base, limit) = IpcRegionType::RealmBinary.bounds();
    assert_eq!(base, REALM_BINARY_BASE);
    assert_eq!(limit, REALM_BINARY_BASE + REALM_BINARY_SIZE);

    // RealmLocal
    let (base, limit) = IpcRegionType::RealmLocal.bounds();
    assert_eq!(base, REALM_LOCAL_BASE);
    assert_eq!(limit, REALM_LOCAL_BASE + REALM_LOCAL_SIZE);
}

#[test]
fn validate_hint_valid_at_base() {
    // Valid hint at region base
    assert!(IpcRegionType::ProcessPool.validate_hint(Vaddr::new(PROCESS_POOL_BASE), 1));
    assert!(IpcRegionType::RealmBinary.validate_hint(Vaddr::new(REALM_BINARY_BASE), 1));
    assert!(IpcRegionType::RealmLocal.validate_hint(Vaddr::new(REALM_LOCAL_BASE), 1));
}

#[test]
fn validate_hint_valid_multi_page() {
    // Valid multi-page allocation
    assert!(IpcRegionType::ProcessPool.validate_hint(Vaddr::new(PROCESS_POOL_BASE), 100));
}

#[test]
fn validate_hint_rejects_unaligned() {
    // Unaligned address (not on 4KB boundary)
    let unaligned = PROCESS_POOL_BASE + 1;
    assert!(!IpcRegionType::ProcessPool.validate_hint(Vaddr::new(unaligned), 1));

    let unaligned = PROCESS_POOL_BASE + 0x100; // 256 bytes, not 4KB
    assert!(!IpcRegionType::ProcessPool.validate_hint(Vaddr::new(unaligned), 1));
}

#[test]
fn validate_hint_rejects_below_base() {
    // Address below region base
    let below = PROCESS_POOL_BASE - PAGE_SIZE;
    assert!(!IpcRegionType::ProcessPool.validate_hint(Vaddr::new(below), 1));
}

#[test]
fn validate_hint_rejects_exceeds_limit() {
    // Allocation that would exceed region limit
    let (_, limit) = IpcRegionType::ProcessPool.bounds();
    let near_limit = limit - PAGE_SIZE;
    // Request 2 pages starting near limit - would exceed
    assert!(!IpcRegionType::ProcessPool.validate_hint(Vaddr::new(near_limit), 2));
}

#[test]
fn validate_hint_rejects_wrong_region() {
    // ProcessPool address in RealmBinary region request
    assert!(!IpcRegionType::RealmBinary.validate_hint(Vaddr::new(PROCESS_POOL_BASE), 1));

    // RealmBinary address in ProcessPool region request
    assert!(!IpcRegionType::ProcessPool.validate_hint(Vaddr::new(REALM_BINARY_BASE), 1));
}

#[test]
fn validate_hint_rejects_overflow() {
    // Page count that would overflow
    assert!(!IpcRegionType::ProcessPool.validate_hint(Vaddr::new(PROCESS_POOL_BASE), u64::MAX));
}

#[test]
fn advance_pointer_moves_forward() {
    let current = PROCESS_POOL_BASE;
    let hint = Vaddr::new(PROCESS_POOL_BASE + 10 * PAGE_SIZE);
    let page_count = 5;

    let new_ptr = IpcRegionType::ProcessPool.advance_pointer(current, hint, page_count);

    // Should advance to hint + 5 pages
    let expected = PROCESS_POOL_BASE + 10 * PAGE_SIZE + 5 * PAGE_SIZE;
    assert_eq!(new_ptr, expected);
}

#[test]
fn advance_pointer_unchanged_if_behind() {
    let current = PROCESS_POOL_BASE + 100 * PAGE_SIZE;
    let hint = Vaddr::new(PROCESS_POOL_BASE); // Behind current
    let page_count = 5;

    let new_ptr = IpcRegionType::ProcessPool.advance_pointer(current, hint, page_count);

    // Should not change - hint is behind current
    assert_eq!(new_ptr, current);
}

#[test]
fn advance_pointer_handles_exact_match() {
    let current = PROCESS_POOL_BASE + 10 * PAGE_SIZE;
    let hint = Vaddr::new(PROCESS_POOL_BASE + 5 * PAGE_SIZE);
    let page_count = 5; // hint_end = base + 10 pages = current

    let new_ptr = IpcRegionType::ProcessPool.advance_pointer(current, hint, page_count);

    // hint_end == current, should not change
    assert_eq!(new_ptr, current);
}

#[test]
fn allocate_check_valid() {
    let current = PROCESS_POOL_BASE;
    let result = IpcRegionType::ProcessPool.allocate_check(current, 10);

    assert!(result.is_some());
    assert_eq!(result.unwrap(), PROCESS_POOL_BASE + 10 * PAGE_SIZE);
}

#[test]
fn allocate_check_rejects_exceeds_limit() {
    // Try to allocate more pages than the region can hold
    let pages_in_region = PROCESS_POOL_SIZE / PAGE_SIZE;
    let result = IpcRegionType::ProcessPool.allocate_check(PROCESS_POOL_BASE, pages_in_region + 1);

    assert!(result.is_none());
}

#[test]
fn allocate_check_rejects_near_limit() {
    // Start near the limit
    let (_, limit) = IpcRegionType::ProcessPool.bounds();
    let near_limit = limit - PAGE_SIZE;

    // Try to allocate 2 pages - would exceed
    let result = IpcRegionType::ProcessPool.allocate_check(near_limit, 2);
    assert!(result.is_none());

    // Allocate 1 page - should succeed
    let result = IpcRegionType::ProcessPool.allocate_check(near_limit, 1);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), limit);
}

#[test]
fn allocate_check_rejects_overflow() {
    let result = IpcRegionType::ProcessPool.allocate_check(PROCESS_POOL_BASE, u64::MAX);
    assert!(result.is_none());
}

// =============================================================================
// Extended Edge Case Tests
// =============================================================================

#[test]
fn message_tag_debug_format() {
    // Ensure Debug impl doesn't panic and produces expected output
    assert_eq!(format!("{:?}", MessageTag::AllocPages), "AllocPages");
    assert_eq!(format!("{:?}", MessageTag::Success), "Success");
    assert_eq!(
        format!("{:?}", MessageTag::ErrorOutOfMemory),
        "ErrorOutOfMemory"
    );
    assert_eq!(
        format!("{:?}", MessageTag::ErrorInvalidRequest),
        "ErrorInvalidRequest"
    );
}

#[test]
fn message_tag_constants() {
    // Verify the message register counts are correct
    assert_eq!(MessageTag::ALLOC_PAGES_REQUEST_LEN, 4);
    assert_eq!(MessageTag::ALLOC_PAGES_RESPONSE_LEN, 3);
}

#[test]
fn alloc_pages_request_zero_pages() {
    // Zero pages is technically valid at the protocol level
    let request = AllocPagesRequest::new(IpcRegionType::ProcessPool, 0, Vaddr::null());
    let mrs = request.to_mrs();
    let decoded = AllocPagesRequest::from_mrs(mrs).unwrap();
    assert_eq!(decoded.page_count, 0);
}

#[test]
fn alloc_pages_request_max_pages() {
    // Large but not overflowing page count
    let request = AllocPagesRequest::new(IpcRegionType::ProcessPool, u64::MAX, Vaddr::null());
    let mrs = request.to_mrs();
    let decoded = AllocPagesRequest::from_mrs(mrs).unwrap();
    assert_eq!(decoded.page_count, u64::MAX);
}

#[test]
fn alloc_pages_request_all_regions() {
    // Test encoding/decoding for all region types
    let regions = [
        IpcRegionType::ProcessPool,
        IpcRegionType::RealmBinary,
        IpcRegionType::RealmLocal,
    ];

    for region in regions {
        let request = AllocPagesRequest::new(region, 42, Vaddr::new(0xDEAD_BEEF));
        let mrs = request.to_mrs();
        let decoded = AllocPagesRequest::from_mrs(mrs).unwrap();
        assert_eq!(decoded.region, region);
        assert_eq!(decoded.page_count, 42);
    }
}

#[test]
fn alloc_pages_response_large_page_count() {
    let response = AllocPagesResponse::success(Vaddr::new(0x1000), u64::MAX);
    let mrs = response.to_mrs();
    let decoded = AllocPagesResponse::from_mrs(mrs).unwrap();
    assert_eq!(decoded.page_count, u64::MAX);
}

#[test]
fn alloc_pages_response_max_address() {
    // Test with maximum valid address
    let response = AllocPagesResponse::success(Vaddr::new(u64::MAX), 1);
    let mrs = response.to_mrs();
    let decoded = AllocPagesResponse::from_mrs(mrs).unwrap();
    assert_eq!(decoded.vaddr.as_u64(), u64::MAX);
}

// =============================================================================
// Region Boundary Tests for All Regions
// =============================================================================

#[test]
fn validate_hint_realm_binary_boundaries() {
    let region = IpcRegionType::RealmBinary;
    let (base, limit) = region.bounds();

    // Valid: at base
    assert!(region.validate_hint(Vaddr::new(base), 1));

    // Valid: at limit - 1 page
    assert!(region.validate_hint(Vaddr::new(limit - PAGE_SIZE), 1));

    // Invalid: at limit (would exceed)
    assert!(!region.validate_hint(Vaddr::new(limit), 1));

    // Invalid: just before base
    if base >= PAGE_SIZE {
        assert!(!region.validate_hint(Vaddr::new(base - PAGE_SIZE), 1));
    }
}

#[test]
fn validate_hint_realm_local_boundaries() {
    let region = IpcRegionType::RealmLocal;
    let (base, limit) = region.bounds();

    // Valid: at base
    assert!(region.validate_hint(Vaddr::new(base), 1));

    // Valid: at limit - 1 page
    assert!(region.validate_hint(Vaddr::new(limit - PAGE_SIZE), 1));

    // Invalid: at limit (would exceed)
    assert!(!region.validate_hint(Vaddr::new(limit), 1));
}

#[test]
fn validate_hint_exact_fit_at_limit() {
    // Allocation that exactly reaches the limit should succeed
    let region = IpcRegionType::ProcessPool;
    let (_, limit) = region.bounds();

    // Start 10 pages before limit, request 10 pages = exactly reaches limit
    let start = limit - 10 * PAGE_SIZE;
    assert!(region.validate_hint(Vaddr::new(start), 10));

    // Start 10 pages before limit, request 11 pages = exceeds limit
    assert!(!region.validate_hint(Vaddr::new(start), 11));
}

#[test]
fn allocate_check_all_regions() {
    // Test allocate_check for all region types
    let regions = [
        IpcRegionType::ProcessPool,
        IpcRegionType::RealmBinary,
        IpcRegionType::RealmLocal,
    ];

    for region in regions {
        let (base, _) = region.bounds();

        // Should succeed at base
        let result = region.allocate_check(base, 1);
        assert!(result.is_some(), "allocate_check failed for {region:?}");
        assert_eq!(result.unwrap(), base + PAGE_SIZE);
    }
}

#[test]
fn allocate_check_zero_pages() {
    // Zero pages should succeed (returns same pointer)
    let result = IpcRegionType::ProcessPool.allocate_check(PROCESS_POOL_BASE, 0);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), PROCESS_POOL_BASE);
}

#[test]
fn allocate_check_exact_region_capacity() {
    // Allocating exactly the region's capacity from base should succeed
    let region = IpcRegionType::RealmLocal;
    let (base, limit) = region.bounds();
    let pages = (limit - base) / PAGE_SIZE;

    let result = region.allocate_check(base, pages);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), limit);
}

#[test]
fn advance_pointer_saturating_behavior() {
    // Test that advance_pointer uses saturating arithmetic
    let current = PROCESS_POOL_BASE;
    let hint = Vaddr::new(u64::MAX - PAGE_SIZE);

    // This should saturate rather than overflow
    let new_ptr = IpcRegionType::ProcessPool.advance_pointer(current, hint, u64::MAX);

    // Result should be saturated (very large) but not wrap around
    assert!(new_ptr >= current);
}

#[test]
fn advance_pointer_all_regions() {
    let regions = [
        (IpcRegionType::ProcessPool, PROCESS_POOL_BASE),
        (IpcRegionType::RealmBinary, REALM_BINARY_BASE),
        (IpcRegionType::RealmLocal, REALM_LOCAL_BASE),
    ];

    for (region, base) in regions {
        let current = base;
        let hint = Vaddr::new(base + 100 * PAGE_SIZE);
        let page_count = 10;

        let new_ptr = region.advance_pointer(current, hint, page_count);

        // Should advance to hint + pages
        let expected = base + 100 * PAGE_SIZE + 10 * PAGE_SIZE;
        assert_eq!(new_ptr, expected, "advance_pointer failed for {region:?}");
    }
}

// =============================================================================
// Error Type Tests
// =============================================================================

#[test]
fn lmm_error_debug_format() {
    // Ensure Debug impl works for all error variants
    assert!(format!("{:?}", LmmError::OutOfMemory).contains("OutOfMemory"));
    assert!(format!("{:?}", LmmError::InvalidRequest).contains("InvalidRequest"));
    assert!(format!("{:?}", LmmError::InvalidResponse).contains("InvalidResponse"));
}

#[test]
fn lmm_error_equality() {
    assert_eq!(LmmError::OutOfMemory, LmmError::OutOfMemory);
    assert_eq!(LmmError::InvalidRequest, LmmError::InvalidRequest);
    assert_ne!(LmmError::OutOfMemory, LmmError::InvalidRequest);
}

// =============================================================================
// Cross-Region Validation Tests
// =============================================================================

#[test]
fn regions_do_not_overlap() {
    // Verify that all IPC regions have non-overlapping bounds
    let regions = [
        IpcRegionType::ProcessPool,
        IpcRegionType::RealmBinary,
        IpcRegionType::RealmLocal,
    ];

    for i in 0..regions.len() {
        for j in (i + 1)..regions.len() {
            let (base_i, limit_i) = regions[i].bounds();
            let (base_j, limit_j) = regions[j].bounds();

            // Check no overlap: either i is entirely before j, or j is entirely before i
            let no_overlap = limit_i <= base_j || limit_j <= base_i;
            assert!(
                no_overlap,
                "Regions {:?} and {:?} overlap",
                regions[i], regions[j]
            );
        }
    }
}

#[test]
fn validate_hint_rejects_address_in_other_regions() {
    // For each region, verify hints in OTHER regions are rejected
    let all_bases = [
        (IpcRegionType::ProcessPool, PROCESS_POOL_BASE),
        (IpcRegionType::RealmBinary, REALM_BINARY_BASE),
        (IpcRegionType::RealmLocal, REALM_LOCAL_BASE),
    ];

    for (target_region, _) in &all_bases {
        for (_, other_base) in &all_bases {
            let (target_base, target_limit) = target_region.bounds();

            // If other_base is outside target region, it should be rejected
            if *other_base < target_base || *other_base >= target_limit {
                assert!(
                    !target_region.validate_hint(Vaddr::new(*other_base), 1),
                    "Region {target_region:?} should reject hint at {other_base:#x}"
                );
            }
        }
    }
}
