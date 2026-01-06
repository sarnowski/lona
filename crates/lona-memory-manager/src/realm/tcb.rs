// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! TCB configuration and worker startup.
//!
//! This module handles starting a worker thread in a realm by:
//! - Writing initial registers (entry point, stack, arguments)
//! - Binding `SchedContext` and fault endpoint
//! - Resuming the TCB

#[cfg(feature = "sel4")]
use super::constants::TCB_PRIORITY;
#[cfg(feature = "sel4")]
use super::types::{Realm, RealmError};
#[cfg(feature = "sel4")]
use lona_abi::boot::BootFlags;
#[cfg(feature = "sel4")]
use lona_abi::layout::{INIT_HEAP_SIZE, PROCESS_POOL_BASE, WORKER_STACK_SIZE, worker_stack_base};
#[cfg(feature = "sel4")]
use lona_abi::types::WorkerId;
#[cfg(feature = "sel4")]
use sel4::Cap;
#[cfg(feature = "sel4")]
use sel4::cap_type::{Endpoint, SchedContext, Tcb};

/// Start a worker TCB in a realm.
#[cfg(feature = "sel4")]
pub fn start_worker(realm: &Realm, worker_id: WorkerId) -> Result<(), RealmError> {
    let tcb_cap: Cap<Tcb> = Cap::from_bits(realm.tcb_slot as u64);
    let sched_cap: Cap<SchedContext> = Cap::from_bits(realm.sched_context_slot as u64);

    // Step 12: Write initial registers
    sel4::debug_println!("Writing TCB registers...");
    let worker_idx = worker_id.as_u16();
    let stack_top = worker_stack_base(worker_idx) + WORKER_STACK_SIZE;
    let heap_start = PROCESS_POOL_BASE;
    let heap_size = INIT_HEAP_SIZE;
    let flags = BootFlags::NONE
        .with(BootFlags::IS_INIT_REALM)
        .with(BootFlags::HAS_UART)
        .as_u64();

    write_tcb_registers(
        tcb_cap,
        realm.entry_point,
        stack_top,
        realm.id.as_u64(),
        worker_idx as u64,
        heap_start,
        heap_size,
        flags,
    )?;

    // Step 13: Bind SchedContext and fault endpoint to TCB via set_sched_params (MCS)
    sel4::debug_println!("Binding SchedContext via set_sched_params...");
    let endpoint_cap: Cap<Endpoint> = Cap::from_bits(realm.endpoint_slot as u64);
    tcb_cap
        .tcb_set_sched_params(
            sel4::init_thread::slot::TCB.cap(),
            TCB_PRIORITY,
            TCB_PRIORITY,
            sched_cap,
            endpoint_cap,
        )
        .map_err(|e| {
            sel4::debug_println!("TCB set_sched_params failed: {:?}", e);
            RealmError::TcbConfiguration
        })?;

    // Step 14: Resume TCB
    sel4::debug_println!("Resuming TCB...");
    tcb_cap.tcb_resume().map_err(|e| {
        sel4::debug_println!("TCB resume failed: {:?}", e);
        RealmError::TcbConfiguration
    })?;

    sel4::debug_println!("Worker started!");
    Ok(())
}

/// Write initial registers to TCB.
#[cfg(feature = "sel4")]
fn write_tcb_registers(
    tcb_cap: Cap<Tcb>,
    entry_point: u64,
    stack_top: u64,
    realm_id: u64,
    worker_id: u64,
    heap_start: u64,
    heap_size: u64,
    flags: u64,
) -> Result<(), RealmError> {
    let mut regs = sel4::UserContext::default();

    #[cfg(target_arch = "aarch64")]
    {
        *regs.pc_mut() = entry_point;
        *regs.sp_mut() = stack_top;
        *regs.gpr_mut(0) = realm_id;
        *regs.gpr_mut(1) = worker_id;
        *regs.gpr_mut(2) = heap_start;
        *regs.gpr_mut(3) = heap_size;
        *regs.gpr_mut(4) = flags;
    }

    #[cfg(target_arch = "x86_64")]
    {
        *regs.pc_mut() = entry_point;
        *regs.sp_mut() = stack_top;
        *regs.c_param_mut(0) = realm_id; // RDI
        *regs.c_param_mut(1) = worker_id; // RSI
        *regs.c_param_mut(2) = heap_start; // RDX
        *regs.c_param_mut(3) = heap_size; // RCX
        *regs.c_param_mut(4) = flags; // R8
    }

    tcb_cap
        .tcb_write_all_registers(false, &mut regs)
        .map_err(|e| {
            sel4::debug_println!("TCB write registers failed: {:?}", e);
            RealmError::TcbConfiguration
        })
}
