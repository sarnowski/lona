// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Constants shared across realm creation modules.

/// Size of the init realm's `CSpace` in bits (2^8 = 256 slots).
#[cfg(feature = "sel4")]
pub const CNODE_SIZE_BITS: usize = 8;

/// Depth to use when addressing slots in the root task's `CSpace`.
/// seL4 expects `seL4_WordBits` (64) for the root `CNode`.
#[cfg(all(feature = "sel4", target_arch = "x86_64"))]
pub const ROOT_CNODE_DEPTH: usize = 64;

/// Size of `SchedContext` in bits.
#[cfg(feature = "sel4")]
pub const SCHED_CONTEXT_SIZE_BITS: usize = 12;

/// TCB priority for init realm worker.
#[cfg(feature = "sel4")]
pub const TCB_PRIORITY: u64 = 254;

/// Fixed temporary mapping address for copying data to frames.
/// This address is in the root task's `VSpace`, below the child realm regions.
#[cfg(feature = "sel4")]
pub const TEMP_MAP_VADDR: u64 = 0x0000_0000_4000_0000;
