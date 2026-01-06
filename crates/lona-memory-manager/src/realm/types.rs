// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Core types for realm creation.

use lona_abi::types::RealmId;

/// Error during realm creation.
#[derive(Debug, Clone, Copy)]
pub enum RealmError {
    /// Not enough untyped memory.
    OutOfMemory,
    /// No more capability slots available.
    OutOfSlots,
    /// Failed to create kernel object.
    ObjectCreation,
    /// Failed to assign ASID.
    AsidAssignment,
    /// Failed to map frame.
    MappingFailed,
    /// Failed to configure TCB.
    TcbConfiguration,
    /// No boot module found for VM.
    NoVmBootModule,
}

/// A created realm with all its kernel objects.
#[cfg(feature = "sel4")]
pub struct Realm {
    /// Realm identifier.
    pub id: RealmId,
    /// VSpace (root page table) capability slot (stored for future realm teardown).
    pub(crate) _vspace_slot: usize,
    /// CSpace (CNode) capability slot (stored for future realm teardown).
    pub(crate) _cspace_slot: usize,
    /// TCB capability slot.
    pub(crate) tcb_slot: usize,
    /// SchedContext capability slot.
    pub(crate) sched_context_slot: usize,
    /// Fault endpoint capability slot.
    pub(crate) endpoint_slot: usize,
    /// IPC buffer frame capability slot (stored for future realm teardown).
    pub(crate) _ipc_frame_slot: usize,
    /// Entry point address from ELF.
    pub(crate) entry_point: u64,
}

/// A created realm (stub for non-seL4 builds).
#[cfg(not(feature = "sel4"))]
pub struct Realm {
    /// Realm identifier.
    pub id: RealmId,
}
