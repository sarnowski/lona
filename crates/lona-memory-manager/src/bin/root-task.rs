// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Lona Memory Manager - seL4 Root Task
//!
//! This is the initial task that seL4 loads at boot.
//! It creates the init realm and starts the Lona VM.

#![no_std]
#![no_main]

use lona_abi::layout::{INIT_HEAP_SIZE, PAGE_SIZE, PROCESS_POOL_BASE};
use lona_abi::types::WorkerId;
use lona_memory_manager::event_loop::{EventLoop, RealmEntry};
use lona_memory_manager::realm;
use lona_memory_manager::slots::SlotAllocator;
use lona_memory_manager::untyped::UntypedAllocator;
use sel4::Cap;
use sel4::cap_type::{Endpoint, SchedContext, SchedControl, Tcb, VSpace};
use sel4_root_task::root_task;

/// Entry point for the Lona Memory Manager.
#[root_task]
fn main(bootinfo: &sel4::BootInfoPtr) -> ! {
    sel4::debug_println!("Lona Memory Manager {}", lona_memory_manager::VERSION);
    sel4::debug_println!("Starting...");

    // Initialize allocators
    let mut slots = SlotAllocator::from_bootinfo(bootinfo);
    let mut untypeds = UntypedAllocator::from_bootinfo(bootinfo);

    // Find VM boot module (embedded or from bootinfo)
    sel4::debug_println!("Looking for VM binary...");
    let vm_module = match realm::find_vm_boot_module(bootinfo) {
        Ok(m) => {
            sel4::debug_println!(
                "Found VM: {} segments, {} bytes total",
                m.segment_count,
                m.total_mem_size
            );
            m
        }
        Err(e) => {
            sel4::debug_println!("ERROR: Failed to find VM binary: {:?}", e);
            sel4::init_thread::suspend_self()
        }
    };

    // Create init realm (with multiple worker TCBs)
    sel4::debug_println!("Creating init realm...");
    let init_realm = match realm::create_init_realm(bootinfo, &vm_module, &mut slots, &mut untypeds)
    {
        Ok(r) => {
            sel4::debug_println!(
                "Init realm created: {:?} ({} workers)",
                r.id,
                r.worker_count
            );
            r
        }
        Err(e) => {
            sel4::debug_println!("ERROR: Failed to create init realm: {:?}", e);
            sel4::init_thread::suspend_self()
        }
    };

    // Start all workers
    sel4::debug_println!("Starting {} init realm workers...", init_realm.worker_count);
    for worker_idx in 0..init_realm.worker_count {
        let worker_id = WorkerId::new(worker_idx as u16).expect("worker_idx within MAX_WORKERS");
        if let Err(e) = realm::start_worker(&init_realm, worker_id) {
            sel4::debug_println!("ERROR: Failed to start worker {}: {:?}", worker_idx, e);
            sel4::init_thread::suspend_self()
        }
    }

    sel4::debug_println!("All workers started, entering event loop...");

    // Get SchedControl capability for budget replenishment
    let sched_control: Cap<SchedControl> = bootinfo.sched_control().index(0).cap();

    // Create event loop with remaining allocators and SchedControl
    let mut event_loop = EventLoop::new(slots, untypeds, sched_control);

    // Register init realm with event loop
    // Use Worker 0's caps as the primary for fault/IPC handling
    let vspace: Cap<VSpace> = Cap::from_bits(init_realm.vspace_slot as u64);
    let endpoint: Cap<Endpoint> = Cap::from_bits(init_realm.endpoint_slot as u64);
    let sched_context: Cap<SchedContext> = Cap::from_bits(init_realm.sched_context_slots[0] as u64);
    let tcb: Cap<Tcb> = Cap::from_bits(init_realm.tcb_slots[0] as u64);

    // Calculate initial process pool address (after INIT_HEAP_SIZE already mapped)
    let init_heap_pages = (INIT_HEAP_SIZE / PAGE_SIZE) as u64;
    let mut realm_entry = RealmEntry::new(init_realm.id, vspace, endpoint, sched_context, tcb);
    realm_entry.next_process_pool = PROCESS_POOL_BASE + init_heap_pages * PAGE_SIZE;

    // Register additional workers' SchedContexts for timeout replenishment
    for worker_idx in 1..init_realm.worker_count {
        let sc: Cap<SchedContext> =
            Cap::from_bits(init_realm.sched_context_slots[worker_idx] as u64);
        realm_entry.add_worker_sched_context(sc);
    }

    if let Err(e) = event_loop.register_realm(realm_entry) {
        sel4::debug_println!("ERROR: Failed to register realm: {:?}", e);
        sel4::init_thread::suspend_self()
    }

    // Run the event loop
    event_loop.run()
}
