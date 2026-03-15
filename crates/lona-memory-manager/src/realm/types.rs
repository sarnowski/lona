// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Core types for realm creation.

#[cfg(feature = "sel4")]
use super::constants::MAX_REALM_WORKERS;
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
///
/// Supports multiple worker TCBs per realm. Each worker has its own
/// TCB, `SchedContext`, and IPC buffer. All workers share the same
/// `VSpace` and `CSpace`.
#[cfg(feature = "sel4")]
pub struct Realm {
    /// Realm identifier.
    pub id: RealmId,
    /// `VSpace` (root page table) capability slot.
    pub vspace_slot: usize,
    /// `CSpace` (`CNode`) capability slot.
    pub cspace_slot: usize,
    /// TCB capability slots (one per worker).
    pub tcb_slots: [usize; MAX_REALM_WORKERS],
    /// `SchedContext` capability slots (one per worker).
    pub sched_context_slots: [usize; MAX_REALM_WORKERS],
    /// Endpoint capability slot (shared for both faults and IPC).
    ///
    /// This single endpoint is used for:
    /// - Thread faults (configured in TCB via `tcb_set_sched_params`)
    /// - LMM IPC requests (cap copied to realm's `CSpace`)
    ///
    /// The event loop distinguishes between faults (label != 0) and
    /// IPC requests (label == 0) based on the message label.
    pub endpoint_slot: usize,
    /// IPC buffer frame capability slots (one per worker).
    pub ipc_frame_slots: [usize; MAX_REALM_WORKERS],
    /// Number of active workers.
    pub worker_count: usize,
    /// Entry point address from ELF.
    pub entry_point: u64,
}

/// A created realm (stub for non-seL4 builds).
#[cfg(not(feature = "sel4"))]
pub struct Realm {
    /// Realm identifier.
    pub id: RealmId,
}
