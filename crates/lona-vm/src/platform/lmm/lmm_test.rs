// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for LMM IPC interface.
//!
//! These tests verify the mock implementation behavior. In mock mode,
//! all LMM requests return `OutOfMemory` error since there's no real
//! memory manager to communicate with.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::lmm_request_pages;
use lona_abi::Vaddr;
use lona_abi::ipc::{IpcRegionType, LmmError};

// =============================================================================
// Mock Mode Basic Tests
// =============================================================================

#[test]
fn mock_lmm_returns_error() {
    // In mock mode, LMM requests should fail gracefully
    let result = lmm_request_pages(IpcRegionType::ProcessPool, 10, None);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), LmmError::OutOfMemory);
}

#[test]
fn mock_lmm_all_regions_return_error() {
    // All region types should return the same error in mock mode
    let regions = [
        IpcRegionType::ProcessPool,
        IpcRegionType::RealmBinary,
        IpcRegionType::RealmLocal,
    ];

    for region in regions {
        let result = lmm_request_pages(region, 1, None);
        assert!(result.is_err(), "Expected error for region {region:?}");
        assert_eq!(
            result.unwrap_err(),
            LmmError::OutOfMemory,
            "Expected OutOfMemory for region {region:?}"
        );
    }
}

#[test]
fn mock_lmm_zero_pages() {
    // Zero-page request should still fail in mock mode
    let result = lmm_request_pages(IpcRegionType::ProcessPool, 0, None);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), LmmError::OutOfMemory);
}

#[test]
fn mock_lmm_large_page_count() {
    // Large page count should fail gracefully
    let result = lmm_request_pages(IpcRegionType::ProcessPool, usize::MAX, None);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), LmmError::OutOfMemory);
}

// =============================================================================
// Hint Address Tests (Mock Mode)
// =============================================================================

#[test]
fn mock_lmm_with_hint_returns_error() {
    // Even with a valid hint, mock mode returns error
    let hint = Vaddr::new(0x0000_0020_0000_0000); // ProcessPool base
    let result = lmm_request_pages(IpcRegionType::ProcessPool, 1, Some(hint));
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), LmmError::OutOfMemory);
}

#[test]
fn mock_lmm_various_hints() {
    // Various hint addresses should all result in OutOfMemory in mock mode
    let hints = [
        Some(Vaddr::null()),
        Some(Vaddr::new(0x1000)),
        Some(Vaddr::new(0x0000_0020_0000_0000)),
        Some(Vaddr::new(u64::MAX)),
        None,
    ];

    for hint in hints {
        let result = lmm_request_pages(IpcRegionType::ProcessPool, 1, hint);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), LmmError::OutOfMemory);
    }
}

// =============================================================================
// Error Type Tests
// =============================================================================

#[test]
fn lmm_error_is_copy() {
    // LmmError should be Copy for ergonomic error handling
    let err = LmmError::OutOfMemory;
    let err_copy = err;
    assert_eq!(err, err_copy);
}

#[test]
fn lmm_error_debug_impl() {
    // Debug impl should work for logging
    let err = LmmError::OutOfMemory;
    let debug_str = format!("{err:?}");
    assert!(debug_str.contains("OutOfMemory"));
}

// =============================================================================
// Multiple Request Tests (Mock Mode)
// =============================================================================

#[test]
fn mock_lmm_multiple_requests() {
    // Multiple consecutive requests should all fail consistently
    for i in 0..10 {
        let result = lmm_request_pages(IpcRegionType::ProcessPool, i + 1, None);
        assert!(result.is_err(), "Request {i} should fail");
        assert_eq!(
            result.unwrap_err(),
            LmmError::OutOfMemory,
            "Request {i} should return OutOfMemory"
        );
    }
}

#[test]
fn mock_lmm_alternating_regions() {
    // Alternating between regions should all fail consistently
    let patterns = [
        IpcRegionType::ProcessPool,
        IpcRegionType::RealmBinary,
        IpcRegionType::ProcessPool,
        IpcRegionType::RealmLocal,
        IpcRegionType::RealmBinary,
    ];

    for (i, region) in patterns.iter().enumerate() {
        let result = lmm_request_pages(*region, 1, None);
        assert!(result.is_err(), "Request {i} should fail");
    }
}

// =============================================================================
// API Ergonomics Tests
// =============================================================================

#[test]
fn lmm_request_pages_returns_result() {
    // Verify the function returns a proper Result type
    let result: Result<Vaddr, LmmError> = lmm_request_pages(IpcRegionType::ProcessPool, 1, None);

    // Pattern matching should work
    match result {
        Ok(_vaddr) => panic!("Expected error in mock mode"),
        Err(LmmError::OutOfMemory) => {} // Expected
        Err(e) => panic!("Unexpected error: {e:?}"),
    }
}

#[test]
fn lmm_request_pages_hint_is_option() {
    // Both Some and None should be valid for hint parameter
    let _ = lmm_request_pages(IpcRegionType::ProcessPool, 1, None);
    let _ = lmm_request_pages(IpcRegionType::ProcessPool, 1, Some(Vaddr::null()));
    // No compile errors means the API is correct
}
