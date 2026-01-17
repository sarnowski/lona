// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! IPC interface to the Lona Memory Manager.
//!
//! This module provides functions for requesting memory from the LMM via seL4 IPC.
//! The VM uses these to grow its process pool when needed.

// =============================================================================
// seL4 Implementation
// =============================================================================

#[cfg(all(not(test), not(feature = "std")))]
mod sel4_impl {
    use lona_abi::ipc::{
        AllocPagesRequest, AllocPagesResponse, IpcRegionType, LmmError, MessageTag,
    };
    use lona_abi::{CapSlot, Vaddr};
    use sel4::Cap;
    use sel4::cap_type::Endpoint;

    /// Request pages from the Lona Memory Manager.
    ///
    /// This function sends an IPC request to the LMM, blocks until the reply,
    /// and returns the virtual address of the newly mapped pages.
    ///
    /// # Arguments
    ///
    /// * `region` - Which memory region to allocate in
    /// * `page_count` - Number of 4KB pages to allocate
    /// * `hint_vaddr` - Suggested virtual address (`None` = let LMM choose)
    ///
    /// # Errors
    ///
    /// Returns an error if the allocation fails (out of memory, invalid request).
    pub fn lmm_request_pages(
        region: IpcRegionType,
        page_count: usize,
        hint_vaddr: Option<Vaddr>,
    ) -> Result<Vaddr, LmmError> {
        // Build request
        let request = AllocPagesRequest::new(
            region,
            page_count as u64,
            hint_vaddr.unwrap_or(Vaddr::null()),
        );
        let mrs = request.to_mrs();

        // Get LMM endpoint capability
        let lmm_endpoint: Cap<Endpoint> = Cap::from_bits(CapSlot::LMM_ENDPOINT.as_u64());

        // Build message info
        let msg_info = sel4::MessageInfoBuilder::default()
            .length(MessageTag::ALLOC_PAGES_REQUEST_LEN)
            .build();

        // Set message registers
        sel4::with_ipc_buffer_mut(|ipc_buffer| {
            ipc_buffer.msg_regs_mut()[0] = mrs[0];
            ipc_buffer.msg_regs_mut()[1] = mrs[1];
            ipc_buffer.msg_regs_mut()[2] = mrs[2];
            ipc_buffer.msg_regs_mut()[3] = mrs[3];
        });

        // Call LMM endpoint (blocking)
        let _reply = lmm_endpoint.call(msg_info);

        // Read response from message registers
        let (mr0, mr1, mr2) = sel4::with_ipc_buffer(|ipc_buffer| {
            (
                ipc_buffer.msg_regs()[0],
                ipc_buffer.msg_regs()[1],
                ipc_buffer.msg_regs()[2],
            )
        });

        // Parse response
        let Some(response) = AllocPagesResponse::from_mrs([mr0, mr1, mr2]) else {
            return Err(LmmError::InvalidResponse);
        };

        if response.is_success() {
            Ok(response.vaddr)
        } else {
            match LmmError::from_tag(response.tag) {
                Some(e) => Err(e),
                None => Err(LmmError::InvalidResponse),
            }
        }
    }
}

#[cfg(all(not(test), not(feature = "std")))]
pub use sel4_impl::lmm_request_pages;

// =============================================================================
// Mock Implementation (for testing)
// =============================================================================

#[cfg(any(test, feature = "std"))]
mod mock_impl {
    use lona_abi::Vaddr;
    use lona_abi::ipc::{IpcRegionType, LmmError};

    /// Mock implementation that always fails.
    ///
    /// In tests, the process pool should be sized appropriately or use
    /// a mock allocator that doesn't require IPC.
    ///
    /// # Errors
    ///
    /// Always returns `LmmError::OutOfMemory` in mock mode.
    pub const fn lmm_request_pages(
        _region: IpcRegionType,
        _page_count: usize,
        _hint_vaddr: Option<Vaddr>,
    ) -> Result<Vaddr, LmmError> {
        // In mock mode, we don't have a real LMM
        Err(LmmError::OutOfMemory)
    }
}

#[cfg(any(test, feature = "std"))]
pub use mock_impl::lmm_request_pages;

#[cfg(test)]
mod lmm_test;
