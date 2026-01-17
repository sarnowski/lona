// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! IPC message types for realm ↔ memory manager communication.
//!
//! This module defines the protocol between realms and the Lona Memory Manager.
//! Messages use seL4's message registers (MR0-MR3 for fast path).
//!
//! # Message Register Layout
//!
//! All messages use a consistent layout where MR0 contains the message tag.
//!
//! ## `AllocPagesRequest` (realm → MM):
//!
//! | Register | Content |
//! |----------|---------|
//! | MR0 | `MessageTag::AllocPages` |
//! | MR1 | `IpcRegionType` |
//! | MR2 | `page_count` (u64) |
//! | MR3 | `hint_vaddr` (u64, 0 = any) |
//!
//! ## `AllocPagesResponse` (MM → realm):
//!
//! | Register | Content |
//! |----------|---------|
//! | MR0 | `MessageTag` (Success or error) |
//! | MR1 | `vaddr` (u64) |
//! | MR2 | `page_count` (u64) |

use crate::Vaddr;
use core::fmt;

#[cfg(test)]
mod ipc_test;

// =============================================================================
// Message Tags
// =============================================================================

/// IPC message tag identifying the request or response type.
///
/// Tags 1-127 are requests (realm → MM).
/// Tags 128-255 are responses (MM → realm).
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum MessageTag {
    // Requests (realm → MM)
    /// Request to allocate pages in a specific region.
    AllocPages = 1,

    // Responses (MM → realm)
    /// Allocation succeeded.
    Success = 128,
    /// Allocation failed: out of memory.
    ErrorOutOfMemory = 129,
    /// Request was malformed or invalid.
    ErrorInvalidRequest = 130,
}

impl MessageTag {
    /// Number of message registers used by `AllocPagesRequest`.
    pub const ALLOC_PAGES_REQUEST_LEN: usize = 4;

    /// Number of message registers used by `AllocPagesResponse`.
    pub const ALLOC_PAGES_RESPONSE_LEN: usize = 3;

    /// Try to convert from a raw u64 value.
    #[must_use]
    pub const fn from_u64(value: u64) -> Option<Self> {
        match value {
            1 => Some(Self::AllocPages),
            128 => Some(Self::Success),
            129 => Some(Self::ErrorOutOfMemory),
            130 => Some(Self::ErrorInvalidRequest),
            _ => None,
        }
    }

    /// Returns true if this is a request tag (realm → MM).
    #[inline]
    #[must_use]
    pub const fn is_request(self) -> bool {
        (self as u64) < 128
    }

    /// Returns true if this is a response tag (MM → realm).
    #[inline]
    #[must_use]
    pub const fn is_response(self) -> bool {
        (self as u64) >= 128
    }

    /// Returns true if this is a success response.
    #[inline]
    #[must_use]
    pub const fn is_success(self) -> bool {
        matches!(self, Self::Success)
    }

    /// Returns true if this is an error response.
    #[inline]
    #[must_use]
    pub const fn is_error(self) -> bool {
        self.is_response() && !self.is_success()
    }
}

impl fmt::Debug for MessageTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AllocPages => write!(f, "AllocPages"),
            Self::Success => write!(f, "Success"),
            Self::ErrorOutOfMemory => write!(f, "ErrorOutOfMemory"),
            Self::ErrorInvalidRequest => write!(f, "ErrorInvalidRequest"),
        }
    }
}

// =============================================================================
// Region Types for IPC
// =============================================================================

/// Region type for IPC allocation requests.
///
/// This is a simplified view of `RegionType` used in IPC messages.
/// The full `RegionType` enum contains internal-only variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u64)]
pub enum IpcRegionType {
    /// Process pool - process heaps and stacks.
    ProcessPool = 1,
    /// Realm binary heap - large binaries.
    RealmBinary = 2,
    /// Realm-local data - namespace data, atom tables.
    RealmLocal = 3,
}

impl IpcRegionType {
    /// Try to convert from a raw u64 value.
    #[must_use]
    pub const fn from_u64(value: u64) -> Option<Self> {
        match value {
            1 => Some(Self::ProcessPool),
            2 => Some(Self::RealmBinary),
            3 => Some(Self::RealmLocal),
            _ => None,
        }
    }

    /// Returns the (base, limit) bounds for this region.
    ///
    /// The limit is exclusive (base + size).
    #[must_use]
    pub const fn bounds(self) -> (u64, u64) {
        use crate::layout::{
            PROCESS_POOL_BASE, PROCESS_POOL_SIZE, REALM_BINARY_BASE, REALM_BINARY_SIZE,
            REALM_LOCAL_BASE, REALM_LOCAL_SIZE,
        };

        match self {
            Self::ProcessPool => (PROCESS_POOL_BASE, PROCESS_POOL_BASE + PROCESS_POOL_SIZE),
            Self::RealmBinary => (REALM_BINARY_BASE, REALM_BINARY_BASE + REALM_BINARY_SIZE),
            Self::RealmLocal => (REALM_LOCAL_BASE, REALM_LOCAL_BASE + REALM_LOCAL_SIZE),
        }
    }

    /// Validate that a hint address is valid for this region.
    ///
    /// A valid hint must be:
    /// - Page-aligned (4KB boundary)
    /// - Within the region's bounds (start >= base, end <= limit)
    ///
    /// Returns `true` if the hint is valid.
    #[must_use]
    pub const fn validate_hint(self, hint: Vaddr, page_count: u64) -> bool {
        use crate::layout::PAGE_SIZE;

        let hint_start = hint.as_u64();

        // Check page alignment
        if hint_start & (PAGE_SIZE - 1) != 0 {
            return false;
        }

        // Calculate end address with overflow check
        let Some(size) = page_count.checked_mul(PAGE_SIZE) else {
            return false;
        };
        let Some(hint_end) = hint_start.checked_add(size) else {
            return false;
        };

        // Check bounds
        let (base, limit) = self.bounds();
        hint_start >= base && hint_end <= limit
    }

    /// Calculate the new pointer position after a hinted allocation.
    ///
    /// If the hint extends beyond `current_ptr`, returns the new position.
    /// Otherwise, returns `current_ptr` unchanged.
    #[must_use]
    pub const fn advance_pointer(self, current_ptr: u64, hint: Vaddr, page_count: u64) -> u64 {
        use crate::layout::PAGE_SIZE;

        let hint_end = hint
            .as_u64()
            .saturating_add(page_count.saturating_mul(PAGE_SIZE));

        if hint_end > current_ptr {
            hint_end
        } else {
            current_ptr
        }
    }

    /// Check if an allocation of `page_count` pages starting at `current_ptr` would exceed bounds.
    ///
    /// Returns `Some(new_ptr)` if valid, `None` if it would exceed bounds.
    #[must_use]
    pub const fn allocate_check(self, current_ptr: u64, page_count: u64) -> Option<u64> {
        use crate::layout::PAGE_SIZE;

        let Some(size) = page_count.checked_mul(PAGE_SIZE) else {
            return None;
        };
        let Some(new_ptr) = current_ptr.checked_add(size) else {
            return None;
        };

        let (base, limit) = self.bounds();
        if new_ptr < base || new_ptr > limit {
            return None;
        }

        Some(new_ptr)
    }
}

// =============================================================================
// Request/Response Structures
// =============================================================================

/// Request to allocate pages in a specific region.
///
/// Sent from realm to Lona Memory Manager via `seL4_Call`.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AllocPagesRequest {
    /// Message tag (must be `MessageTag::AllocPages`).
    pub tag: MessageTag,
    /// Which region to allocate in.
    pub region: IpcRegionType,
    /// Number of 4KB pages to allocate.
    pub page_count: u64,
    /// Suggested virtual address (0 = let MM choose).
    pub hint_vaddr: Vaddr,
}

impl AllocPagesRequest {
    /// Create a new allocation request.
    #[must_use]
    pub const fn new(region: IpcRegionType, page_count: u64, hint_vaddr: Vaddr) -> Self {
        Self {
            tag: MessageTag::AllocPages,
            region,
            page_count,
            hint_vaddr,
        }
    }

    /// Encode this request into message register values.
    ///
    /// Returns `[MR0, MR1, MR2, MR3]`.
    #[must_use]
    pub const fn to_mrs(self) -> [u64; 4] {
        [
            self.tag as u64,
            self.region as u64,
            self.page_count,
            self.hint_vaddr.as_u64(),
        ]
    }

    /// Decode a request from message register values.
    ///
    /// Returns `None` if the tag or region is invalid.
    #[must_use]
    pub const fn from_mrs(mrs: [u64; 4]) -> Option<Self> {
        let Some(tag) = MessageTag::from_u64(mrs[0]) else {
            return None;
        };
        if !matches!(tag, MessageTag::AllocPages) {
            return None;
        }
        let Some(region) = IpcRegionType::from_u64(mrs[1]) else {
            return None;
        };
        Some(Self {
            tag,
            region,
            page_count: mrs[2],
            hint_vaddr: Vaddr::new(mrs[3]),
        })
    }
}

/// Response to an allocation request.
///
/// Sent from Lona Memory Manager to realm via `seL4_Reply`.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct AllocPagesResponse {
    /// Response tag (Success or error).
    pub tag: MessageTag,
    /// Start of allocated region (only valid if Success).
    pub vaddr: Vaddr,
    /// Actual pages allocated (only valid if Success).
    pub page_count: u64,
}

impl AllocPagesResponse {
    /// Create a success response.
    #[must_use]
    pub const fn success(vaddr: Vaddr, page_count: u64) -> Self {
        Self {
            tag: MessageTag::Success,
            vaddr,
            page_count,
        }
    }

    /// Create an out-of-memory error response.
    #[must_use]
    pub const fn error_out_of_memory() -> Self {
        Self {
            tag: MessageTag::ErrorOutOfMemory,
            vaddr: Vaddr::null(),
            page_count: 0,
        }
    }

    /// Create an invalid request error response.
    #[must_use]
    pub const fn error_invalid_request() -> Self {
        Self {
            tag: MessageTag::ErrorInvalidRequest,
            vaddr: Vaddr::null(),
            page_count: 0,
        }
    }

    /// Encode this response into message register values.
    ///
    /// Returns `[MR0, MR1, MR2]`.
    #[must_use]
    pub const fn to_mrs(self) -> [u64; 3] {
        [self.tag as u64, self.vaddr.as_u64(), self.page_count]
    }

    /// Decode a response from message register values.
    ///
    /// Returns `None` if the tag is invalid.
    #[must_use]
    pub const fn from_mrs(mrs: [u64; 3]) -> Option<Self> {
        let Some(tag) = MessageTag::from_u64(mrs[0]) else {
            return None;
        };
        if tag.is_request() {
            return None;
        }
        Some(Self {
            tag,
            vaddr: Vaddr::new(mrs[1]),
            page_count: mrs[2],
        })
    }

    /// Returns true if the allocation succeeded.
    #[inline]
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.tag.is_success()
    }
}

// =============================================================================
// Error Type
// =============================================================================

/// Error from Lona Memory Manager IPC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LmmError {
    /// Out of physical memory.
    OutOfMemory,
    /// Request was malformed.
    InvalidRequest,
    /// Response could not be parsed.
    InvalidResponse,
}

impl LmmError {
    /// Create from a response tag.
    #[must_use]
    pub const fn from_tag(tag: MessageTag) -> Option<Self> {
        match tag {
            MessageTag::ErrorOutOfMemory => Some(Self::OutOfMemory),
            MessageTag::ErrorInvalidRequest => Some(Self::InvalidRequest),
            _ => None,
        }
    }
}
