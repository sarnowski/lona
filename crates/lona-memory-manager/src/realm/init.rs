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

use crate::elf::{Elf, ElfError};
use crate::embedded;

/// Information about the VM boot module.
pub struct VmBootModule<'a> {
    /// Parsed ELF file.
    elf: Elf<'a>,
    /// Entry point address.
    pub entry_point: u64,
    /// Number of loadable segments.
    pub segment_count: usize,
    /// Total size of all segments in memory.
    pub total_mem_size: u64,
}

impl<'a> VmBootModule<'a> {
    /// Returns an iterator over loadable segments.
    pub fn segments(&self) -> impl Iterator<Item = crate::elf::LoadableSegment<'a>> + '_ {
        self.elf.loadable_segments()
    }
}

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

/// Find the VM binary in boot modules or embedded data.
#[cfg(feature = "sel4")]
pub fn find_vm_boot_module(
    _bootinfo: &sel4::BootInfoPtr,
) -> Result<VmBootModule<'static>, RealmError> {
    // First, check for embedded VM binary
    if let Some(elf_bytes) = embedded::embedded_vm() {
        sel4::debug_println!("Using embedded VM binary ({} bytes)", elf_bytes.len());
        return parse_vm_elf(elf_bytes);
    }

    // Fall back to boot modules
    sel4::debug_println!("No embedded VM, searching boot modules...");
    Err(RealmError::NoVmBootModule)
}

/// Find the VM binary in boot modules (non-seL4 stub).
///
/// # Errors
///
/// Returns `RealmError::NoVmBootModule` when no VM is available.
#[cfg(not(feature = "sel4"))]
pub fn find_vm_boot_module() -> Result<VmBootModule<'static>, RealmError> {
    // Check for embedded VM binary
    if let Some(elf_bytes) = embedded::embedded_vm() {
        return parse_vm_elf(elf_bytes);
    }
    Err(RealmError::NoVmBootModule)
}

/// Parse VM ELF binary.
fn parse_vm_elf(elf_bytes: &[u8]) -> Result<VmBootModule<'_>, RealmError> {
    let elf = Elf::parse(elf_bytes).map_err(|e| match e {
        ElfError::TooSmall
        | ElfError::InvalidMagic
        | ElfError::Not64Bit
        | ElfError::NotLittleEndian
        | ElfError::NotExecutable
        | ElfError::InvalidPhdrOffset => RealmError::NoVmBootModule,
    })?;

    let entry_point = elf.entry_point();
    let segment_count = elf.loadable_segment_count();
    let total_mem_size: u64 = elf.loadable_segments().map(|s| s.mem_size).sum();

    Ok(VmBootModule {
        elf,
        entry_point,
        segment_count,
        total_mem_size,
    })
}

// =============================================================================
// seL4 Implementation
// =============================================================================

#[cfg(feature = "sel4")]
mod sel4_impl {
    use super::{RealmError, VmBootModule};
    use crate::elf::SegmentPermissions;
    use crate::slots::SlotAllocator;
    use crate::untyped::UntypedAllocator;
    use lona_abi::boot::BootFlags;
    #[cfg(target_arch = "aarch64")]
    use lona_abi::layout::UART_VADDR;
    use lona_abi::layout::{
        INIT_HEAP_SIZE, PAGE_SIZE, PROCESS_POOL_BASE, WORKER_STACK_SIZE, worker_ipc_buffer,
        worker_stack_base,
    };
    use lona_abi::types::{RealmId, WorkerId};
    use sel4::cap_type::{CNode, Endpoint, Granule, SchedContext, Tcb, VSpace};
    use sel4::{Cap, CapRights, ObjectBlueprint, VmAttributes};

    /// Size of the init realm's CSpace in bits (2^8 = 256 slots).
    const CNODE_SIZE_BITS: usize = 8;

    /// Depth to use when addressing slots in the root task's CSpace.
    /// seL4 expects seL4_WordBits (64) for the root CNode.
    #[cfg(target_arch = "x86_64")]
    const ROOT_CNODE_DEPTH: usize = 64;

    /// Size of SchedContext in bits.
    const SCHED_CONTEXT_SIZE_BITS: usize = 12;

    /// TCB priority for init realm worker.
    const TCB_PRIORITY: u64 = 254;

    /// A created realm with all its kernel objects.
    pub struct Realm {
        /// Realm identifier.
        pub id: RealmId,
        /// VSpace (root page table) capability slot (stored for future realm teardown).
        _vspace_slot: usize,
        /// CSpace (CNode) capability slot (stored for future realm teardown).
        _cspace_slot: usize,
        /// TCB capability slot.
        tcb_slot: usize,
        /// SchedContext capability slot.
        sched_context_slot: usize,
        /// Fault endpoint capability slot.
        endpoint_slot: usize,
        /// IPC buffer frame capability slot (stored for future realm teardown).
        _ipc_frame_slot: usize,
        /// Entry point address from ELF.
        entry_point: u64,
    }

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

    /// Start a worker TCB in a realm.
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

    // =========================================================================
    // Helper Functions
    // =========================================================================

    /// Create VSpace (root page table).
    fn create_vspace(
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

    /// Assign ASID to VSpace.
    fn assign_asid(vspace_cap: Cap<VSpace>) -> Result<(), RealmError> {
        let asid_pool = sel4::init_thread::slot::ASID_POOL.cap();
        asid_pool.asid_pool_assign(vspace_cap).map_err(|e| {
            sel4::debug_println!("ASID assignment failed: {:?}", e);
            RealmError::AsidAssignment
        })
    }

    /// Create CNode for realm's CSpace.
    fn create_cnode(
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
    fn create_endpoint(
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

    /// Create SchedContext.
    fn create_sched_context(
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

    /// Configure SchedContext with CPU budget.
    fn configure_sched_context(
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
    fn create_tcb(
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

    /// Configure TCB with CSpace, VSpace, and IPC buffer.
    /// Note: In MCS mode, fault endpoint and SchedContext are set via tcb_set_sched_params.
    fn configure_tcb(
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

    /// Write initial registers to TCB.
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

    /// Map a segment from the ELF file into the realm's VSpace.
    fn map_segment(
        slots: &mut SlotAllocator,
        untypeds: &mut UntypedAllocator,
        vspace: Cap<VSpace>,
        vaddr: u64,
        mem_size: u64,
        data: &[u8],
        permissions: SegmentPermissions,
    ) -> Result<(), RealmError> {
        let page_size = PAGE_SIZE;
        let num_pages = ((mem_size + page_size - 1) / page_size) as usize;

        for i in 0..num_pages {
            let page_vaddr = (vaddr & !(page_size - 1)) + (i as u64) * page_size;
            let page_offset = i * (page_size as usize);

            // Determine how much data to copy to this page
            let data_start = if vaddr > page_vaddr {
                0
            } else {
                page_offset.saturating_sub((vaddr - (vaddr & !(page_size - 1))) as usize)
            };
            let data_end = (data_start + page_size as usize).min(data.len());
            let page_data = if data_start < data.len() {
                &data[data_start..data_end]
            } else {
                &[]
            };

            // Allocate frame
            let frame_slot = allocate_frame(slots, untypeds)?;

            // Copy data to frame via temporary mapping in root task
            if !page_data.is_empty() {
                copy_data_to_frame(slots, untypeds, frame_slot, page_data)?;
            }

            // Ensure page tables exist and map frame
            map_frame_with_page_tables(
                slots,
                untypeds,
                vspace,
                frame_slot,
                page_vaddr,
                permissions,
            )?;
        }

        Ok(())
    }

    /// Allocate a frame (4KB page).
    fn allocate_frame(
        slots: &mut SlotAllocator,
        untypeds: &mut UntypedAllocator,
    ) -> Result<usize, RealmError> {
        let dest_slot = slots.alloc().ok_or(RealmError::OutOfSlots)?;
        #[cfg(target_arch = "aarch64")]
        let blueprint = ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::SmallPage);
        #[cfg(target_arch = "x86_64")]
        let blueprint = ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::_4k);

        let (ut_slot, _, _) = untypeds
            .allocate(12, slots, false) // 4KB = 2^12
            .ok_or(RealmError::OutOfMemory)?;

        // ut_slot is an absolute slot number, use Cap::from_bits directly
        let untyped: Cap<sel4::cap_type::Untyped> = Cap::from_bits(ut_slot as u64);
        let cnode = sel4::init_thread::slot::CNODE.cap();

        untyped
            .untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), dest_slot, 1)
            .map_err(|e| {
                sel4::debug_println!("Frame retype failed: {:?}", e);
                RealmError::ObjectCreation
            })?;

        Ok(dest_slot)
    }

    /// Copy data to a frame via temporary mapping.
    fn copy_data_to_frame(
        slots: &mut SlotAllocator,
        untypeds: &mut UntypedAllocator,
        frame_slot: usize,
        data: &[u8],
    ) -> Result<(), RealmError> {
        // For simplicity, we use a fixed temporary mapping address
        // In a production system, this would be managed more carefully
        const TEMP_MAP_VADDR: u64 = 0x0000_0000_4000_0000;

        let frame_cap: Cap<Granule> = Cap::from_bits(frame_slot as u64);
        let root_vspace = sel4::init_thread::slot::VSPACE.cap();

        // Map frame temporarily in root task's VSpace
        ensure_page_tables_exist(slots, untypeds, root_vspace, TEMP_MAP_VADDR)?;

        frame_cap
            .frame_map(
                root_vspace,
                TEMP_MAP_VADDR as usize,
                CapRights::read_write(),
                VmAttributes::default(),
            )
            .map_err(|e| {
                sel4::debug_println!("Temp frame map failed: {:?}", e);
                RealmError::MappingFailed
            })?;

        // Copy data
        // SAFETY: We just mapped this address
        unsafe {
            let dst = TEMP_MAP_VADDR as *mut u8;
            core::ptr::copy_nonoverlapping(data.as_ptr(), dst, data.len());
            // Zero-fill the rest of the page
            let remaining = (PAGE_SIZE as usize) - data.len();
            if remaining > 0 {
                core::ptr::write_bytes(dst.add(data.len()), 0, remaining);
            }
        }

        // Unmap from root task
        frame_cap.frame_unmap().map_err(|e| {
            sel4::debug_println!("Temp frame unmap failed: {:?}", e);
            RealmError::MappingFailed
        })?;

        Ok(())
    }

    /// Map a RW frame at the given address (for stack/IPC buffer).
    fn map_rw_frame(
        slots: &mut SlotAllocator,
        untypeds: &mut UntypedAllocator,
        vspace: Cap<VSpace>,
        vaddr: u64,
    ) -> Result<usize, RealmError> {
        let frame_slot = allocate_frame(slots, untypeds)?;

        // Zero the frame
        copy_data_to_frame(slots, untypeds, frame_slot, &[])?;

        let permissions = SegmentPermissions {
            read: true,
            write: true,
            execute: false,
        };

        map_frame_with_page_tables(slots, untypeds, vspace, frame_slot, vaddr, permissions)?;

        Ok(frame_slot)
    }

    /// Map a frame into VSpace, creating page tables as needed.
    fn map_frame_with_page_tables(
        slots: &mut SlotAllocator,
        untypeds: &mut UntypedAllocator,
        vspace: Cap<VSpace>,
        frame_slot: usize,
        vaddr: u64,
        permissions: SegmentPermissions,
    ) -> Result<(), RealmError> {
        let frame_cap: Cap<Granule> = Cap::from_bits(frame_slot as u64);

        // Determine rights and attributes based on permissions
        // Note: On ARM, execute-never is typically controlled via different mechanisms
        // For now, we use default attributes for all mappings
        let attrs = VmAttributes::default();

        // Try mapping, creating page tables as needed (up to 4 levels on aarch64)
        for _ in 0..4 {
            let rights = if permissions.write {
                CapRights::read_write()
            } else {
                CapRights::read_only()
            };

            match frame_cap.frame_map(vspace, vaddr as usize, rights, attrs) {
                Ok(()) => return Ok(()),
                Err(sel4::Error::FailedLookup) => {
                    // Missing page table - create and map one
                    create_and_map_page_table(slots, untypeds, vspace, vaddr)?;
                }
                Err(e) => {
                    sel4::debug_println!("Frame map at 0x{:x} failed: {:?}", vaddr, e);
                    return Err(RealmError::MappingFailed);
                }
            }
        }

        sel4::debug_println!("Failed to map frame after creating 4 page tables");
        Err(RealmError::MappingFailed)
    }

    /// Ensure page tables exist for a virtual address.
    fn ensure_page_tables_exist(
        slots: &mut SlotAllocator,
        untypeds: &mut UntypedAllocator,
        vspace: Cap<VSpace>,
        vaddr: u64,
    ) -> Result<(), RealmError> {
        // Create page tables up to 4 levels as needed
        for _ in 0..4 {
            // Use the architecture-aware page table creation
            match create_and_map_page_table(slots, untypeds, vspace, vaddr) {
                Ok(()) => continue,                      // Created one level, might need more
                Err(RealmError::MappingFailed) => break, // All levels exist
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }

    /// Create a page table (ARM) or intermediate translation table (x86_64).
    ///
    /// On ARM, creates a PT object that works at all levels.
    /// On x86_64, creates a PageTable object (level 3). For higher levels,
    /// use `create_page_directory`, `create_pdpt`.
    #[cfg(target_arch = "aarch64")]
    fn create_page_table(
        slots: &mut SlotAllocator,
        untypeds: &mut UntypedAllocator,
    ) -> Result<usize, RealmError> {
        let dest_slot = slots.alloc().ok_or(RealmError::OutOfSlots)?;
        let blueprint = ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::PT);

        let (ut_slot, _, _) = untypeds
            .allocate(12, slots, false) // Page tables are 4KB
            .ok_or(RealmError::OutOfMemory)?;

        let untyped: Cap<sel4::cap_type::Untyped> = Cap::from_bits(ut_slot as u64);
        let cnode = sel4::init_thread::slot::CNODE.cap();

        untyped
            .untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), dest_slot, 1)
            .map_err(|e| {
                sel4::debug_println!("Page table retype failed: {:?}", e);
                RealmError::ObjectCreation
            })?;

        Ok(dest_slot)
    }

    /// Map a page table into VSpace (ARM).
    #[cfg(target_arch = "aarch64")]
    fn map_page_table(pt_slot: usize, vspace: Cap<VSpace>, vaddr: u64) -> Result<(), RealmError> {
        use sel4::cap_type::PT;
        let pt_cap: Cap<PT> = Cap::from_bits(pt_slot as u64);

        match pt_cap.pt_map(vspace, vaddr as usize, VmAttributes::default()) {
            Ok(()) => Ok(()),
            Err(sel4::Error::DeleteFirst) => Ok(()), // Already exists
            Err(e) => {
                sel4::debug_println!("Page table map failed: {:?}", e);
                Err(RealmError::MappingFailed)
            }
        }
    }

    /// x86_64 page table level indicator.
    #[cfg(target_arch = "x86_64")]
    #[derive(Clone, Copy, Debug)]
    enum X86PageTableLevel {
        /// Page Directory Pointer Table (level 1, covers 512GB)
        Pdpt,
        /// Page Directory (level 2, covers 1GB)
        PageDirectory,
        /// Page Table (level 3, covers 2MB)
        PageTable,
    }

    /// Create an x86_64 translation structure at the specified level.
    #[cfg(target_arch = "x86_64")]
    fn create_x86_translation_table(
        slots: &mut SlotAllocator,
        untypeds: &mut UntypedAllocator,
        level: X86PageTableLevel,
    ) -> Result<usize, RealmError> {
        let dest_slot = slots.alloc().ok_or(RealmError::OutOfSlots)?;
        let blueprint = match level {
            X86PageTableLevel::Pdpt => ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::SeL4Arch(
                sel4::ObjectBlueprintX64::PDPT,
            )),
            X86PageTableLevel::PageDirectory => {
                ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::PageDirectory)
            }
            X86PageTableLevel::PageTable => {
                ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::PageTable)
            }
        };

        let size_bits = blueprint.physical_size_bits() as u8;
        let (ut_slot, _, _) = untypeds
            .allocate(size_bits, slots, false)
            .ok_or(RealmError::OutOfMemory)?;

        let untyped: Cap<sel4::cap_type::Untyped> = Cap::from_bits(ut_slot as u64);
        let cnode = sel4::init_thread::slot::CNODE.cap();

        untyped
            .untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), dest_slot, 1)
            .map_err(|e| {
                sel4::debug_println!("{:?} retype failed: {:?}", level, e);
                RealmError::ObjectCreation
            })?;

        Ok(dest_slot)
    }

    /// Result of mapping a translation table.
    #[cfg(target_arch = "x86_64")]
    enum MapResult {
        /// Successfully mapped a new table.
        Mapped,
        /// Table already exists at this level.
        AlreadyExists,
    }

    /// Map an x86_64 translation structure into VSpace at the specified level.
    #[cfg(target_arch = "x86_64")]
    fn map_x86_translation_table(
        slot: usize,
        vspace: Cap<VSpace>,
        vaddr: u64,
        level: X86PageTableLevel,
    ) -> Result<MapResult, RealmError> {
        let attrs = VmAttributes::default();
        let result = match level {
            X86PageTableLevel::Pdpt => {
                let cap: Cap<sel4::cap_type::PDPT> = Cap::from_bits(slot as u64);
                cap.pdpt_map(vspace, vaddr as usize, attrs)
            }
            X86PageTableLevel::PageDirectory => {
                let cap: Cap<sel4::cap_type::PageDirectory> = Cap::from_bits(slot as u64);
                cap.page_directory_map(vspace, vaddr as usize, attrs)
            }
            X86PageTableLevel::PageTable => {
                let cap: Cap<sel4::cap_type::PageTable> = Cap::from_bits(slot as u64);
                cap.page_table_map(vspace, vaddr as usize, attrs)
            }
        };

        match result {
            Ok(()) => Ok(MapResult::Mapped),
            Err(sel4::Error::DeleteFirst) => Ok(MapResult::AlreadyExists),
            Err(e) => {
                sel4::debug_println!("{:?} map at 0x{:x} failed: {:?}", level, vaddr, e);
                Err(RealmError::MappingFailed)
            }
        }
    }

    /// Create and map translation tables as needed for x86_64.
    ///
    /// Tries each level in order (PDPT -> PageDirectory -> PageTable) until
    /// the frame can be mapped.
    #[cfg(target_arch = "x86_64")]
    fn create_and_map_page_table(
        slots: &mut SlotAllocator,
        untypeds: &mut UntypedAllocator,
        vspace: Cap<VSpace>,
        vaddr: u64,
    ) -> Result<(), RealmError> {
        // On x86_64, we need to try each level in order.
        // The kernel returns FailedLookup when any level is missing.
        let levels = [
            X86PageTableLevel::Pdpt,
            X86PageTableLevel::PageDirectory,
            X86PageTableLevel::PageTable,
        ];

        for level in levels {
            let slot = create_x86_translation_table(slots, untypeds, level)?;
            match map_x86_translation_table(slot, vspace, vaddr, level) {
                Ok(MapResult::Mapped) => {
                    // Successfully mapped this level, return success
                    return Ok(());
                }
                Ok(MapResult::AlreadyExists) => {
                    // This level already exists, try next level
                    continue;
                }
                Err(RealmError::MappingFailed) => {
                    // Actual mapping failure, try next level
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        // If we get here, all levels already exist
        Err(RealmError::MappingFailed)
    }

    /// Create and map page table (ARM version).
    #[cfg(target_arch = "aarch64")]
    fn create_and_map_page_table(
        slots: &mut SlotAllocator,
        untypeds: &mut UntypedAllocator,
        vspace: Cap<VSpace>,
        vaddr: u64,
    ) -> Result<(), RealmError> {
        let pt_slot = create_page_table(slots, untypeds)?;
        map_page_table(pt_slot, vspace, vaddr)?;
        Ok(())
    }

    /// Set up IOPort capability for UART (x86_64 only).
    ///
    /// Issues an IOPort capability for COM1 (0x3F8-0x3FF) and copies it
    /// to the child realm's CSpace at the well-known slot.
    #[cfg(target_arch = "x86_64")]
    fn setup_ioport_uart(
        slots: &mut SlotAllocator,
        child_cspace_slot: usize,
    ) -> Result<(), RealmError> {
        use lona_abi::types::CapSlot;

        // COM1 port range
        const COM1_FIRST_PORT: u64 = 0x3F8;
        const COM1_LAST_PORT: u64 = 0x3FF;

        // Get IOPortControl capability
        let ioport_control = sel4::init_thread::slot::IO_PORT_CONTROL.cap();
        let root_cnode = sel4::init_thread::slot::CNODE.cap();

        // Allocate a slot in root CSpace for the IOPort capability
        let ioport_slot = slots.alloc().ok_or(RealmError::OutOfSlots)?;

        // Issue IOPort capability for COM1 into root CSpace
        let ioport_dst =
            root_cnode.absolute_cptr_from_bits_with_depth(ioport_slot as u64, ROOT_CNODE_DEPTH);

        ioport_control
            .ioport_control_issue(COM1_FIRST_PORT, COM1_LAST_PORT, &ioport_dst)
            .map_err(|e| {
                sel4::debug_println!("IOPort issue failed: {:?}", e);
                RealmError::ObjectCreation
            })?;

        // Copy IOPort capability to child's CSpace at the well-known slot
        // Source: the IOPort cap we just created in root CSpace
        let src = sel4::init_thread::slot::CNODE
            .cap()
            .absolute_cptr_from_bits_with_depth(ioport_slot as u64, ROOT_CNODE_DEPTH);

        // Destination: slot 6 in child's CSpace (the child CNode is at child_cspace_slot in root CSpace)
        let child_cnode: Cap<CNode> = Cap::from_bits(child_cspace_slot as u64);
        let child_dst = child_cnode
            .absolute_cptr_from_bits_with_depth(CapSlot::IOPORT_UART.as_u64(), CNODE_SIZE_BITS);

        child_dst.copy(&src, CapRights::all()).map_err(|e| {
            sel4::debug_println!("IOPort copy to child CSpace failed: {:?}", e);
            RealmError::ObjectCreation
        })?;

        Ok(())
    }

    /// Map UART device memory (aarch64 only).
    #[cfg(target_arch = "aarch64")]
    fn map_uart(
        bootinfo: &sel4::BootInfoPtr,
        slots: &mut SlotAllocator,
        untypeds: &mut UntypedAllocator,
        vspace: Cap<VSpace>,
    ) -> Result<(), RealmError> {
        // UART physical address depends on platform
        // QEMU virt: 0x0900_0000
        const UART_PADDR: usize = 0x0900_0000;

        // Find device untyped containing UART
        let untyped_list = bootinfo.untyped_list();
        let mut uart_untyped_idx = None;

        for (idx, desc) in untyped_list.iter().enumerate() {
            if !desc.is_device() {
                continue;
            }
            let base = desc.paddr();
            let size = 1_usize << desc.size_bits();
            if UART_PADDR >= base && UART_PADDR < base + size {
                uart_untyped_idx = Some(idx);
                break;
            }
        }

        let uart_idx = uart_untyped_idx.ok_or_else(|| {
            sel4::debug_println!("UART device untyped not found");
            RealmError::MappingFailed
        })?;

        // Retype device untyped into frame
        let frame_slot = slots.alloc().ok_or(RealmError::OutOfSlots)?;
        let untyped = bootinfo.untyped().index(uart_idx).cap();
        let cnode = sel4::init_thread::slot::CNODE.cap();
        let blueprint = ObjectBlueprint::Arch(sel4::ObjectBlueprintArch::SmallPage);

        untyped
            .untyped_retype(&blueprint, &cnode.absolute_cptr_for_self(), frame_slot, 1)
            .map_err(|e| {
                sel4::debug_println!("UART frame retype failed: {:?}", e);
                RealmError::ObjectCreation
            })?;

        // Ensure page tables exist
        ensure_page_tables_exist(slots, untypeds, vspace, UART_VADDR)?;

        // Map UART frame with device memory attributes
        let frame_cap: Cap<Granule> = Cap::from_bits(frame_slot as u64);
        frame_cap
            .frame_map(
                vspace,
                UART_VADDR as usize,
                CapRights::read_write(),
                VmAttributes::default(), // Device memory doesn't need special attrs on ARM
            )
            .map_err(|e| {
                sel4::debug_println!("UART frame map failed: {:?}", e);
                RealmError::MappingFailed
            })?;

        Ok(())
    }
}

#[cfg(feature = "sel4")]
pub use sel4_impl::{Realm, create_init_realm, start_worker};

// =============================================================================
// Non-seL4 Stubs (for testing)
// =============================================================================

#[cfg(not(feature = "sel4"))]
pub use non_sel4_impl::{Realm, create_init_realm, start_worker};

#[cfg(not(feature = "sel4"))]
mod non_sel4_impl {
    use super::{RealmError, VmBootModule};
    use lona_abi::types::{RealmId, WorkerId};

    /// A created realm (stub).
    pub struct Realm {
        /// Realm identifier.
        pub id: RealmId,
    }

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
