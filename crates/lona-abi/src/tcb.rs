// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! TCB (Thread Control Block) configuration types for realm creation.

use crate::layout::{SHARED_CODE_BASE, WORKER_STACK_SIZE, worker_ipc_buffer, worker_stack_base};
use crate::types::WorkerId;

/// Initial register state for starting a realm worker.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct InitialRegisters {
    /// Program counter - entry point address.
    pub pc: u64,
    /// Stack pointer - top of worker stack.
    pub sp: u64,
    /// Argument registers: `[realm_id, worker_id, heap_start, heap_size, flags]`.
    pub args: [u64; 5],
}

impl InitialRegisters {
    /// Create initial registers for a realm worker.
    #[must_use]
    pub const fn for_worker(
        worker_id: WorkerId,
        realm_id: u64,
        heap_start: u64,
        heap_size: u64,
        flags: u64,
    ) -> Self {
        let worker_idx = worker_id.as_u16();
        Self {
            pc: SHARED_CODE_BASE,
            sp: worker_stack_base(worker_idx) + WORKER_STACK_SIZE,
            args: [realm_id, worker_idx as u64, heap_start, heap_size, flags],
        }
    }
}

/// Calculate IPC buffer virtual address for a worker.
#[inline]
#[must_use]
pub const fn ipc_buffer_vaddr(worker_id: WorkerId) -> u64 {
    worker_ipc_buffer(worker_id.as_u16())
}
