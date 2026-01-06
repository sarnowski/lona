// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Kernel object creation helpers for realm initialization.
//!
//! This module provides functions to create the seL4 kernel objects
//! needed for a realm: `VSpace`, `CNode`, Endpoint, `SchedContext`, and TCB.

#[cfg(feature = "sel4")]
use super::constants::{CNODE_SIZE_BITS, SCHED_CONTEXT_SIZE_BITS};
#[cfg(feature = "sel4")]
use super::types::RealmError;
#[cfg(feature = "sel4")]
use crate::slots::SlotAllocator;
#[cfg(feature = "sel4")]
use crate::untyped::UntypedAllocator;
#[cfg(feature = "sel4")]
use sel4::cap_type::{CNode, Granule, SchedContext, Tcb, VSpace};
#[cfg(feature = "sel4")]
use sel4::{Cap, ObjectBlueprint};

/// Create `VSpace` (root page table).
#[cfg(feature = "sel4")]
pub fn create_vspace(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
) -> Result<usize, RealmError> {
    let dest_slot = slots.alloc().ok_or(RealmError::OutOfSlots)?;

    // VSpace size depends on architecture
    #[cfg(target_arch = "aarch64")]
    let blueprint = ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::SeL4Arch(
        sel4::ObjectBlueprintAArch64::VSpace,
    ));
    #[cfg(target_arch = "x86_64")]
    let blueprint = ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::SeL4Arch(
        sel4::ObjectBlueprintX64::PML4,
    ));

    let size_bits = blueprint.physical_size_bits() as u8;
    let (ut_slot, _, _) = untypeds
        .allocate(size_bits, slots, false)
        .ok_or(RealmError::OutOfMemory)?;

    // ut_slot is an absolute slot number, use Cap::from_bits directly
    let untyped: Cap<sel4::cap_type::Untyped> = Cap::from_bits(ut_slot as u64);
    let cnode = sel4::init_thread::slot::CNODE.cap();

    untyped
        .untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), dest_slot, 1)
        .map_err(|e| {
            sel4::debug_println!("VSpace retype failed: {:?}", e);
            RealmError::ObjectCreation
        })?;

    Ok(dest_slot)
}

/// Assign ASID to `VSpace`.
#[cfg(feature = "sel4")]
pub fn assign_asid(vspace_cap: Cap<VSpace>) -> Result<(), RealmError> {
    let asid_pool = sel4::init_thread::slot::ASID_POOL.cap();
    asid_pool.asid_pool_assign(vspace_cap).map_err(|e| {
        sel4::debug_println!("ASID assignment failed: {:?}", e);
        RealmError::AsidAssignment
    })
}

/// Create `CNode` for realm's `CSpace`.
#[cfg(feature = "sel4")]
pub fn create_cnode(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
) -> Result<usize, RealmError> {
    let dest_slot = slots.alloc().ok_or(RealmError::OutOfSlots)?;
    let blueprint = ObjectBlueprint::CNode {
        size_bits: CNODE_SIZE_BITS,
    };

    let size_bits = blueprint.physical_size_bits() as u8;
    let (ut_slot, _, _) = untypeds
        .allocate(size_bits, slots, false)
        .ok_or(RealmError::OutOfMemory)?;

    // ut_slot is an absolute slot number, use Cap::from_bits directly
    let untyped: Cap<sel4::cap_type::Untyped> = Cap::from_bits(ut_slot as u64);
    let cnode = sel4::init_thread::slot::CNODE.cap();

    untyped
        .untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), dest_slot, 1)
        .map_err(|e| {
            sel4::debug_println!("CNode retype failed: {:?}", e);
            RealmError::ObjectCreation
        })?;

    Ok(dest_slot)
}

/// Create fault endpoint.
#[cfg(feature = "sel4")]
pub fn create_endpoint(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
) -> Result<usize, RealmError> {
    let dest_slot = slots.alloc().ok_or(RealmError::OutOfSlots)?;
    let blueprint = ObjectBlueprint::Endpoint;

    let size_bits = blueprint.physical_size_bits() as u8;
    let (ut_slot, _, _) = untypeds
        .allocate(size_bits, slots, false)
        .ok_or(RealmError::OutOfMemory)?;

    // ut_slot is an absolute slot number, use Cap::from_bits directly
    let untyped: Cap<sel4::cap_type::Untyped> = Cap::from_bits(ut_slot as u64);
    let cnode = sel4::init_thread::slot::CNODE.cap();

    untyped
        .untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), dest_slot, 1)
        .map_err(|e| {
            sel4::debug_println!("Endpoint retype failed: {:?}", e);
            RealmError::ObjectCreation
        })?;

    Ok(dest_slot)
}

/// Create `SchedContext`.
#[cfg(feature = "sel4")]
pub fn create_sched_context(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
) -> Result<usize, RealmError> {
    let dest_slot = slots.alloc().ok_or(RealmError::OutOfSlots)?;
    let blueprint = ObjectBlueprint::SchedContext {
        size_bits: SCHED_CONTEXT_SIZE_BITS,
    };

    let size_bits = blueprint.physical_size_bits() as u8;
    let (ut_slot, _, _) = untypeds
        .allocate(size_bits, slots, false)
        .ok_or(RealmError::OutOfMemory)?;

    // ut_slot is an absolute slot number, use Cap::from_bits directly
    let untyped: Cap<sel4::cap_type::Untyped> = Cap::from_bits(ut_slot as u64);
    let cnode = sel4::init_thread::slot::CNODE.cap();

    untyped
        .untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), dest_slot, 1)
        .map_err(|e| {
            sel4::debug_println!("SchedContext retype failed: {:?}", e);
            RealmError::ObjectCreation
        })?;

    Ok(dest_slot)
}

/// Configure `SchedContext` with CPU budget.
#[cfg(feature = "sel4")]
pub fn configure_sched_context(
    bootinfo: &sel4::BootInfoPtr,
    sched_slot: usize,
) -> Result<(), RealmError> {
    let sched_cap: Cap<SchedContext> = Cap::from_bits(sched_slot as u64);
    let sched_control = bootinfo.sched_control().index(0).cap();

    // Configure with generous budget: 10ms period, 10ms budget (100% of time slice)
    const PERIOD_US: u64 = 10_000;
    const BUDGET_US: u64 = 10_000;

    sched_control
        .sched_control_configure_flags(
            sched_cap, BUDGET_US, PERIOD_US, 0, // extra_refills
            0, // badge
            0, // flags
        )
        .map_err(|e| {
            sel4::debug_println!("SchedContext configure failed: {:?}", e);
            RealmError::ObjectCreation
        })
}

/// Create TCB.
#[cfg(feature = "sel4")]
pub fn create_tcb(
    slots: &mut SlotAllocator,
    untypeds: &mut UntypedAllocator,
) -> Result<usize, RealmError> {
    let dest_slot = slots.alloc().ok_or(RealmError::OutOfSlots)?;
    let blueprint = ObjectBlueprint::Tcb;

    let size_bits = blueprint.physical_size_bits() as u8;
    let (ut_slot, _, _) = untypeds
        .allocate(size_bits, slots, false)
        .ok_or(RealmError::OutOfMemory)?;

    // ut_slot is an absolute slot number, use Cap::from_bits directly
    let untyped: Cap<sel4::cap_type::Untyped> = Cap::from_bits(ut_slot as u64);
    let cnode = sel4::init_thread::slot::CNODE.cap();

    untyped
        .untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), dest_slot, 1)
        .map_err(|e| {
            sel4::debug_println!("TCB retype failed: {:?}", e);
            RealmError::ObjectCreation
        })?;

    Ok(dest_slot)
}

/// Configure TCB with `CSpace`, `VSpace`, and IPC buffer.
///
/// In MCS mode, fault endpoint and `SchedContext` are set via `tcb_set_sched_params`.
#[cfg(feature = "sel4")]
pub fn configure_tcb(
    tcb_slot: usize,
    cspace_slot: usize,
    vspace_slot: usize,
    ipc_vaddr: u64,
    ipc_frame_slot: usize,
) -> Result<(), RealmError> {
    let tcb_cap: Cap<Tcb> = Cap::from_bits(tcb_slot as u64);
    let cspace_cap: Cap<CNode> = Cap::from_bits(cspace_slot as u64);
    let vspace_cap: Cap<VSpace> = Cap::from_bits(vspace_slot as u64);
    let ipc_frame_cap: Cap<Granule> = Cap::from_bits(ipc_frame_slot as u64);

    tcb_cap
        .tcb_configure(
            cspace_cap,
            sel4::CNodeCapData::new(0, 64 - CNODE_SIZE_BITS),
            vspace_cap,
            ipc_vaddr,
            ipc_frame_cap,
        )
        .map_err(|e| {
            sel4::debug_println!("TCB configure failed: {:?}", e);
            RealmError::TcbConfiguration
        })
}
