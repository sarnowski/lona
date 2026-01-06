// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Init realm creation.
//!
//! This module handles creating the init realm by:
//! 1. Creating `VSpace` (page table root)
//! 2. Assigning ASID to `VSpace`
//! 3. Mapping ELF segments with proper page tables
//! 4. Allocating and mapping worker stack
//! 5. Allocating and mapping IPC buffer
//! 6. Creating `CSpace` (`CNode`)
//! 7. Creating fault endpoint
//! 8. Creating `SchedContext`
//! 9. Creating and configuring TCB
//! 10. Starting TCB execution

use super::boot_module::VmBootModule;

// =============================================================================
// seL4 Implementation
// =============================================================================

#[cfg(feature = "sel4")]
mod sel4_impl {
    #[cfg(target_arch = "aarch64")]
    use super::super::device::map_uart;
    #[cfg(target_arch = "x86_64")]
    use super::super::device::setup_ioport_uart;
    use super::super::frame_mapping::{map_rw_frame, map_segment};
    use super::super::kernel_objects::{
        assign_asid, configure_sched_context, configure_tcb, create_cnode, create_endpoint,
        create_sched_context, create_tcb, create_vspace,
    };
    use super::super::types::{Realm, RealmError};
    use super::VmBootModule;
    use crate::slots::SlotAllocator;
    use crate::untyped::UntypedAllocator;
    #[cfg(target_arch = "aarch64")]
    use lona_abi::layout::UART_VADDR;
    use lona_abi::layout::{
        INIT_HEAP_SIZE, PAGE_SIZE, PROCESS_POOL_BASE, WORKER_STACK_SIZE, worker_ipc_buffer,
        worker_stack_base,
    };
    use lona_abi::types::RealmId;
    use sel4::Cap;
    use sel4::cap_type::VSpace;

    /// Create the init realm.
    pub fn create_init_realm(
        bootinfo: &sel4::BootInfoPtr,
        vm_module: &VmBootModule<'_>,
    ) -> Result<Realm, RealmError> {
        sel4::debug_println!("Loading VM binary:");
        sel4::debug_println!("  Entry point: 0x{:x}", vm_module.entry_point);
        sel4::debug_println!("  Segments: {}", vm_module.segment_count);
        sel4::debug_println!("  Total size: {} bytes", vm_module.total_mem_size);

        let mut slots = SlotAllocator::from_bootinfo(bootinfo);
        let mut untypeds = UntypedAllocator::from_bootinfo(bootinfo);

        // Step 1: Create VSpace
        sel4::debug_println!("Creating VSpace...");
        let vspace_slot = create_vspace(&mut slots, &mut untypeds)?;
        let vspace_cap: Cap<VSpace> = Cap::from_bits(vspace_slot as u64);
        sel4::debug_println!("  VSpace at slot {}", vspace_slot);

        // Step 2: Assign ASID
        sel4::debug_println!("Assigning ASID...");
        assign_asid(vspace_cap)?;
        sel4::debug_println!("  ASID assigned");

        // Step 3: Map ELF segments
        sel4::debug_println!("Mapping ELF segments...");
        for (i, segment) in vm_module.segments().enumerate() {
            sel4::debug_println!(
                "  Segment {}: 0x{:x} ({} bytes, {})",
                i,
                segment.vaddr,
                segment.mem_size,
                segment.permissions.as_str()
            );
            map_segment(
                &mut slots,
                &mut untypeds,
                vspace_cap,
                segment.vaddr,
                segment.mem_size,
                segment.data,
                segment.permissions,
            )?;
        }

        // Step 4: Allocate and map worker stack
        sel4::debug_println!("Mapping worker stack...");
        let stack_base = worker_stack_base(0);
        let stack_pages = (WORKER_STACK_SIZE / PAGE_SIZE) as usize;
        for i in 0..stack_pages {
            let vaddr = stack_base + (i as u64) * PAGE_SIZE;
            map_rw_frame(&mut slots, &mut untypeds, vspace_cap, vaddr)?;
        }
        sel4::debug_println!("  Stack at 0x{:x} ({} pages)", stack_base, stack_pages);

        // Step 5: Allocate and map IPC buffer
        sel4::debug_println!("Mapping IPC buffer...");
        let ipc_vaddr = worker_ipc_buffer(0);
        let ipc_frame_slot = map_rw_frame(&mut slots, &mut untypeds, vspace_cap, ipc_vaddr)?;
        sel4::debug_println!("  IPC buffer at 0x{:x}", ipc_vaddr);

        // Step 5b: Allocate and map heap
        sel4::debug_println!("Mapping heap...");
        let heap_pages = (INIT_HEAP_SIZE / PAGE_SIZE) as usize;
        for i in 0..heap_pages {
            let vaddr = PROCESS_POOL_BASE + (i as u64) * PAGE_SIZE;
            map_rw_frame(&mut slots, &mut untypeds, vspace_cap, vaddr)?;
        }
        sel4::debug_println!(
            "  Heap at 0x{:x} ({} pages, {} bytes)",
            PROCESS_POOL_BASE,
            heap_pages,
            INIT_HEAP_SIZE
        );

        // Step 6: Create CSpace
        sel4::debug_println!("Creating CSpace...");
        let cspace_slot = create_cnode(&mut slots, &mut untypeds)?;
        sel4::debug_println!("  CSpace at slot {}", cspace_slot);

        // Step 7: Create Endpoint
        sel4::debug_println!("Creating endpoint...");
        let endpoint_slot = create_endpoint(&mut slots, &mut untypeds)?;
        sel4::debug_println!("  Endpoint at slot {}", endpoint_slot);

        // Step 8: Create SchedContext
        sel4::debug_println!("Creating SchedContext...");
        let sched_context_slot = create_sched_context(&mut slots, &mut untypeds)?;
        configure_sched_context(bootinfo, sched_context_slot)?;
        sel4::debug_println!("  SchedContext at slot {}", sched_context_slot);

        // Step 9: Create TCB
        sel4::debug_println!("Creating TCB...");
        let tcb_slot = create_tcb(&mut slots, &mut untypeds)?;
        sel4::debug_println!("  TCB at slot {}", tcb_slot);

        // Step 10: Configure TCB
        // Note: In MCS mode, priority/SchedContext/fault endpoint are set in start_worker
        sel4::debug_println!("Configuring TCB...");
        configure_tcb(
            tcb_slot,
            cspace_slot,
            vspace_slot,
            ipc_vaddr,
            ipc_frame_slot,
        )?;

        // Map UART for init realm (aarch64 only - x86_64 uses IOPort)
        #[cfg(target_arch = "aarch64")]
        {
            sel4::debug_println!("Mapping UART...");
            map_uart(bootinfo, &mut slots, &mut untypeds, vspace_cap)?;
            sel4::debug_println!("  UART at 0x{:x}", UART_VADDR);
        }

        // Set up IOPort capability for UART (x86_64 only)
        #[cfg(target_arch = "x86_64")]
        {
            sel4::debug_println!("Setting up IOPort for UART...");
            setup_ioport_uart(&mut slots, cspace_slot)?;
            sel4::debug_println!(
                "  IOPort at CSpace slot {}",
                lona_abi::types::CapSlot::IOPORT_UART.as_u64()
            );
        }

        Ok(Realm {
            id: RealmId::INIT,
            _vspace_slot: vspace_slot,
            _cspace_slot: cspace_slot,
            tcb_slot,
            sched_context_slot,
            endpoint_slot,
            _ipc_frame_slot: ipc_frame_slot,
            entry_point: vm_module.entry_point,
        })
    }
}

#[cfg(feature = "sel4")]
pub use sel4_impl::create_init_realm;

// =============================================================================
// Non-seL4 Stubs (for testing)
// =============================================================================

#[cfg(not(feature = "sel4"))]
pub use non_sel4_impl::{create_init_realm, start_worker};

#[cfg(not(feature = "sel4"))]
mod non_sel4_impl {
    use super::super::types::{Realm, RealmError};
    use super::VmBootModule;
    use lona_abi::types::{RealmId, WorkerId};

    /// Create the init realm (non-seL4 stub).
    ///
    /// # Errors
    ///
    /// This stub always succeeds.
    pub const fn create_init_realm(_vm_module: &VmBootModule<'_>) -> Result<Realm, RealmError> {
        Ok(Realm { id: RealmId::INIT })
    }

    /// Start a worker (non-seL4 stub).
    ///
    /// # Errors
    ///
    /// This stub always succeeds.
    pub const fn start_worker(_realm: &Realm, _worker_id: WorkerId) -> Result<(), RealmError> {
        Ok(())
    }
}
