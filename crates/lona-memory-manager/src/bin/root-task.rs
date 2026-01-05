// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Lona Memory Manager - seL4 Root Task
//!
//! This is the initial task that seL4 loads at boot.
//! It creates the init realm and starts the Lona VM.

#![no_std]
#![no_main]

use lona_abi::types::WorkerId;
use lona_memory_manager::realm;
use sel4_root_task::root_task;

/// Entry point for the Lona Memory Manager.
#[root_task]
fn main(bootinfo: &sel4::BootInfoPtr) -> ! {
    sel4::debug_println!("Lona Memory Manager {}", lona_memory_manager::VERSION);
    sel4::debug_println!("Starting...");

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

    // Create init realm
    sel4::debug_println!("Creating init realm...");
    let init_realm = match realm::create_init_realm(bootinfo, &vm_module) {
        Ok(r) => {
            sel4::debug_println!("Init realm created: {:?}", r.id);
            r
        }
        Err(e) => {
            sel4::debug_println!("ERROR: Failed to create init realm: {:?}", e);
            sel4::init_thread::suspend_self()
        }
    };

    // Start first worker
    sel4::debug_println!("Starting init realm worker...");
    if let Err(e) = realm::start_worker(&init_realm, WorkerId::FIRST) {
        sel4::debug_println!("ERROR: Failed to start worker: {:?}", e);
        sel4::init_thread::suspend_self()
    }

    sel4::debug_println!("Init realm started, entering event loop...");

    // Event loop - for now just suspend
    // Future: wait on fault endpoint, handle page faults and IPC requests
    loop {
        sel4::init_thread::suspend_self()
    }
}
