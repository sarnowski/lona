// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! seL4 implementation of the event loop.

use crate::realm::constants::{SCHED_BUDGET_US, SCHED_PERIOD_US};
use crate::realm::create_reply;
use crate::realm::frame_mapping::map_rw_frame;
use crate::slots::SlotAllocator;
use crate::untyped::UntypedAllocator;
use lona_abi::Vaddr;
use lona_abi::fault::{SEL4_FAULT_TIMEOUT, SEL4_FAULT_VM_FAULT, VmFaultInfo};
use lona_abi::ipc::{AllocPagesRequest, AllocPagesResponse, IpcRegionType, MessageTag};
use lona_abi::layout::{
    FaultRegion, PAGE_SIZE, PROCESS_POOL_BASE, REALM_BINARY_BASE, REALM_LOCAL_BASE,
    is_inherited_region,
};
use lona_abi::types::RealmId;
use sel4::Cap;
use sel4::cap_type::{Endpoint, Reply, SchedContext, SchedControl, VSpace};

/// Maximum number of realms we can track.
const MAX_REALMS: usize = 16;

/// Error during event loop operation.
#[derive(Debug, Clone, Copy)]
pub enum EventLoopError {
    /// Out of physical memory.
    OutOfMemory,
    /// Out of capability slots.
    OutOfSlots,
    /// Failed to create kernel object.
    ObjectCreation,
    /// Failed to map frame.
    MappingFailed,
    /// Too many realms registered.
    TooManyRealms,
    /// Unknown realm.
    UnknownRealm,
    /// Invalid request.
    InvalidRequest,
}

/// A registered realm's state for the event loop.
pub struct RealmEntry {
    /// Realm identifier.
    pub id: RealmId,
    /// `VSpace` capability (for mapping frames into this realm).
    pub vspace: Cap<VSpace>,
    /// Endpoint capability (for receiving IPC from this realm).
    pub endpoint: Cap<Endpoint>,
    /// `SchedContext` capability (for budget replenishment on Timeout faults).
    pub sched_context: Cap<SchedContext>,
    /// Scheduling budget in microseconds.
    pub budget_us: u64,
    /// Scheduling period in microseconds.
    pub period_us: u64,
    /// Next process pool address for this realm.
    pub next_process_pool: u64,
    /// Next realm binary heap address.
    pub next_realm_binary: u64,
    /// Next realm local data address.
    pub next_realm_local: u64,
    /// Total pages allocated to this realm.
    pub pages_allocated: u64,
}

impl RealmEntry {
    /// Create a new realm entry.
    #[must_use]
    pub fn new(
        id: RealmId,
        vspace: Cap<VSpace>,
        endpoint: Cap<Endpoint>,
        sched_context: Cap<SchedContext>,
    ) -> Self {
        Self {
            id,
            vspace,
            endpoint,
            sched_context,
            budget_us: SCHED_BUDGET_US,
            period_us: SCHED_PERIOD_US,
            next_process_pool: PROCESS_POOL_BASE,
            next_realm_binary: REALM_BINARY_BASE,
            next_realm_local: REALM_LOCAL_BASE,
            pages_allocated: 0,
        }
    }

    /// Get the next allocation address for a region and advance the pointer.
    fn allocate_region(&mut self, region: IpcRegionType, page_count: u64) -> Option<Vaddr> {
        let next_ptr = match region {
            IpcRegionType::ProcessPool => &mut self.next_process_pool,
            IpcRegionType::RealmBinary => &mut self.next_realm_binary,
            IpcRegionType::RealmLocal => &mut self.next_realm_local,
        };

        // Use lona-abi validation to check bounds
        let new_next = region.allocate_check(*next_ptr, page_count)?;

        let addr = Vaddr::new(*next_ptr);
        *next_ptr = new_next;
        Some(addr)
    }

    /// Advance the region pointer if the hint extends beyond current position.
    fn advance_pointer_for_hint(&mut self, region: IpcRegionType, hint: Vaddr, page_count: u64) {
        let next_ptr = match region {
            IpcRegionType::ProcessPool => &mut self.next_process_pool,
            IpcRegionType::RealmBinary => &mut self.next_realm_binary,
            IpcRegionType::RealmLocal => &mut self.next_realm_local,
        };

        // Use lona-abi function to calculate new pointer position
        *next_ptr = region.advance_pointer(*next_ptr, hint, page_count);
    }
}

/// Event loop for handling realm IPC requests and faults.
///
/// The event loop receives messages on realm endpoints and handles:
/// - IPC requests (label=0): `AllocPages` for explicit memory allocation
/// - VMFault (label=5): Lazy mapping for inherited regions only
/// - Timeout (label=6): Budget replenishment + check for interrupted VMFault
/// - Other faults: Log error, don't reply (thread stays blocked)
pub struct EventLoop {
    /// Registered realms.
    realms: [Option<RealmEntry>; MAX_REALMS],
    /// Number of registered realms.
    realm_count: usize,
    /// Slot allocator for new kernel objects.
    slots: SlotAllocator,
    /// Untyped allocator for physical memory.
    untypeds: UntypedAllocator,
    /// Reply capability for replying to messages.
    reply: Cap<Reply>,
    /// `SchedControl` capability for budget replenishment.
    ///
    /// Required to handle Timeout faults correctly: when a thread's budget
    /// expires during a page fault, we must replenish before replying.
    sched_control: Cap<SchedControl>,
}

impl EventLoop {
    /// Create a new event loop with the given allocators and `SchedControl`.
    ///
    /// # Arguments
    ///
    /// * `slots` - Slot allocator for creating kernel objects
    /// * `untypeds` - Untyped memory allocator for physical frames
    /// * `sched_control` - `SchedControl` capability for budget replenishment
    pub fn new(
        mut slots: SlotAllocator,
        mut untypeds: UntypedAllocator,
        sched_control: Cap<SchedControl>,
    ) -> Self {
        const NONE: Option<RealmEntry> = None;

        // Create a Reply capability - this is needed in seL4 MCS for receiving IPC
        let reply_slot = create_reply(&mut slots, &mut untypeds)
            .expect("EventLoop::new: failed to create reply capability");
        let reply: Cap<Reply> = Cap::from_bits(reply_slot as u64);

        sel4::debug_println!("Event loop: Reply capability at slot {}", reply_slot);

        Self {
            realms: [NONE; MAX_REALMS],
            realm_count: 0,
            slots,
            untypeds,
            reply,
            sched_control,
        }
    }

    /// Register a realm with the event loop.
    ///
    /// # Errors
    ///
    /// Returns an error if too many realms are registered.
    pub fn register_realm(&mut self, realm: RealmEntry) -> Result<(), EventLoopError> {
        if self.realm_count >= MAX_REALMS {
            return Err(EventLoopError::TooManyRealms);
        }
        self.realms[self.realm_count] = Some(realm);
        self.realm_count += 1;
        Ok(())
    }

    /// Find a realm by its endpoint capability slot.
    fn find_realm_by_endpoint(&mut self, endpoint_bits: u64) -> Option<&mut RealmEntry> {
        for realm_opt in &mut self.realms[..self.realm_count] {
            if let Some(realm) = realm_opt {
                if realm.endpoint.bits() == endpoint_bits {
                    return Some(realm);
                }
            }
        }
        None
    }

    /// Run the event loop.
    ///
    /// This function never returns - it loops forever handling:
    /// - IPC requests (label=0): Memory allocation via `AllocPages`
    /// - VMFault (label=5): Lazy mapping for inherited regions only
    /// - Timeout (label=6): Budget replenishment + inherited region check
    /// - Other faults: Log and block thread
    pub fn run(&mut self) -> ! {
        sel4::debug_println!("Event loop: starting (MCS-aware fault handling)");

        // Get the endpoint from the first realm
        let endpoint = match &self.realms[0] {
            Some(realm) => realm.endpoint,
            None => {
                sel4::debug_println!("Event loop: no realms registered, suspending");
                sel4::init_thread::suspend_self()
            }
        };
        let endpoint_bits = endpoint.bits();

        // First receive (no reply needed for the first call)
        let (mut msg_info, mut badge) = endpoint.recv(self.reply);

        loop {
            let label = msg_info.label() as u64;

            // Read message registers for all message types
            let mrs = sel4::with_ipc_buffer(|buf| {
                [
                    buf.msg_regs()[0],
                    buf.msg_regs()[1],
                    buf.msg_regs()[2],
                    buf.msg_regs()[3],
                ]
            });

            // Dispatch based on message label
            let (should_reply, reply_length) = match label {
                0 => {
                    // Normal IPC request
                    let response = self.handle_message(endpoint_bits, mrs);
                    let response_mrs = response.to_mrs();
                    sel4::with_ipc_buffer_mut(|buf| {
                        buf.msg_regs_mut()[0] = response_mrs[0];
                        buf.msg_regs_mut()[1] = response_mrs[1];
                        buf.msg_regs_mut()[2] = response_mrs[2];
                    });
                    (true, 3)
                }

                SEL4_FAULT_VM_FAULT => {
                    // VMFault - only handle inherited regions
                    let should_reply = self.handle_vm_fault(endpoint_bits, mrs);
                    (should_reply, 0)
                }

                SEL4_FAULT_TIMEOUT => {
                    // Timeout fault - replenish budget, check for interrupted VMFault
                    let should_reply = self.handle_timeout_fault(endpoint_bits, mrs);
                    (should_reply, 0)
                }

                _ => {
                    // Other fault type (CapFault=1, UnknownSyscall=2, UserException=3, etc.)
                    sel4::debug_println!(
                        "Event loop: unhandled fault (label={}, badge={})",
                        label,
                        badge
                    );
                    sel4::debug_println!(
                        "  Fault MRs: ip=0x{:x}, addr=0x{:x}, data=0x{:x}",
                        mrs[0],
                        mrs[1],
                        mrs[2]
                    );
                    // Don't reply - thread stays blocked
                    (false, 0)
                }
            };

            // Reply (or not) and wait for next message
            if should_reply {
                let reply_info = sel4::MessageInfoBuilder::default()
                    .length(reply_length)
                    .build();
                (msg_info, badge) = endpoint.reply_recv(reply_info, self.reply);
            } else {
                (msg_info, badge) = endpoint.recv(self.reply);
            }
        }
    }

    /// Handle an incoming IPC message.
    fn handle_message(&mut self, endpoint_bits: u64, mrs: [u64; 4]) -> AllocPagesResponse {
        // Parse the message tag
        let Some(tag) = MessageTag::from_u64(mrs[0]) else {
            sel4::debug_println!("Event loop: invalid message tag: {}", mrs[0]);
            return AllocPagesResponse::error_invalid_request();
        };

        match tag {
            MessageTag::AllocPages => self.handle_alloc_pages(endpoint_bits, mrs),
            _ => {
                sel4::debug_println!("Event loop: unexpected message tag: {:?}", tag);
                AllocPagesResponse::error_invalid_request()
            }
        }
    }

    /// Handle an `AllocPages` request.
    fn handle_alloc_pages(&mut self, endpoint_bits: u64, mrs: [u64; 4]) -> AllocPagesResponse {
        // Parse the request
        let Some(request) = AllocPagesRequest::from_mrs(mrs) else {
            sel4::debug_println!("Event loop: invalid AllocPages request");
            return AllocPagesResponse::error_invalid_request();
        };

        sel4::debug_println!(
            "Event loop: AllocPages {{ region: {:?}, pages: {}, hint: {:?} }}",
            request.region,
            request.page_count,
            request.hint_vaddr
        );

        // Find the realm
        let Some(realm) = self.find_realm_by_endpoint(endpoint_bits) else {
            sel4::debug_println!("Event loop: unknown realm for endpoint");
            return AllocPagesResponse::error_invalid_request();
        };

        // Determine virtual address
        let vaddr = if request.hint_vaddr.is_null() {
            // Use next available address for the region
            match realm.allocate_region(request.region, request.page_count) {
                Some(addr) => addr,
                None => {
                    sel4::debug_println!("Event loop: region address overflow or bounds exceeded");
                    return AllocPagesResponse::error_invalid_request();
                }
            }
        } else {
            // Validate hint address is within the correct region bounds (uses lona-abi)
            if !request
                .region
                .validate_hint(request.hint_vaddr, request.page_count)
            {
                sel4::debug_println!(
                    "Event loop: invalid hint address {:?} for region {:?}",
                    request.hint_vaddr,
                    request.region
                );
                return AllocPagesResponse::error_invalid_request();
            }
            // Advance region pointer to prevent future allocations from overlapping
            realm.advance_pointer_for_hint(request.region, request.hint_vaddr, request.page_count);
            request.hint_vaddr
        };

        let vspace = realm.vspace;
        let page_count = request.page_count;

        // Track successful mappings for error reporting
        let mut pages_mapped: u64 = 0;

        // Allocate and map each page
        for i in 0..page_count {
            let page_vaddr = vaddr.as_u64() + i * PAGE_SIZE;

            match map_rw_frame(&mut self.slots, &mut self.untypeds, vspace, page_vaddr) {
                Ok(_frame_slot) => {
                    pages_mapped += 1;
                    // Find realm again (we need mutable access for bookkeeping)
                    if let Some(realm) = self.find_realm_by_endpoint(endpoint_bits) {
                        realm.pages_allocated += 1;
                    }
                }
                Err(e) => {
                    // Partial mapping failure - some pages may have been mapped.
                    // NOTE: Rollback is not implemented - already-mapped pages remain allocated.
                    // They are tracked in pages_allocated for cleanup on realm termination.
                    sel4::debug_println!(
                        "Event loop: failed to map page {} of {}: {:?} ({} pages already mapped)",
                        i,
                        page_count,
                        e,
                        pages_mapped
                    );
                    return AllocPagesResponse::error_out_of_memory();
                }
            }
        }

        sel4::debug_println!("Event loop: allocated {} pages at {:?}", page_count, vaddr);

        AllocPagesResponse::success(vaddr, page_count)
    }

    /// Handle a VM fault.
    ///
    /// IMPORTANT: Only inherited regions use fault-based lazy mapping.
    /// All other regions must use explicit IPC allocation. Faults in
    /// non-inherited regions indicate bugs or invalid memory access.
    ///
    /// Returns `true` if page was mapped (reply to resume thread).
    /// Returns `false` if fault is an error (don't reply - thread stays blocked).
    fn handle_vm_fault(&mut self, endpoint_bits: u64, mrs: [u64; 4]) -> bool {
        let fault = VmFaultInfo::from_mrs(mrs);
        let addr = fault.addr.as_u64();

        sel4::debug_println!(
            "Event loop: VMFault at 0x{:x} (ip=0x{:x}, prefetch={})",
            addr,
            fault.ip,
            fault.is_prefetch
        );

        // ONLY inherited regions use fault-based lazy mapping.
        // All other regions must use explicit IPC allocation.
        if is_inherited_region(addr) {
            let Some(realm) = self.find_realm_by_endpoint(endpoint_bits) else {
                sel4::debug_println!("Event loop: VMFault from unknown realm");
                return false;
            };
            let vspace = realm.vspace;

            // Page-align the faulting address
            let page_addr = addr & !(PAGE_SIZE - 1);

            // FUTURE: Map the page from parent realm's frames (read-only, shared).
            // Currently allocates a fresh zeroed frame as a placeholder. The target
            // implementation will look up the parent's frame for this address and
            // map it into the child's VSpace, enabling code inheritance where children
            // see parent-defined code. This requires tracking parent->child relationships
            // and parent frame mappings.
            match map_rw_frame(&mut self.slots, &mut self.untypeds, vspace, page_addr) {
                Ok(_) => {
                    if let Some(realm) = self.find_realm_by_endpoint(endpoint_bits) {
                        realm.pages_allocated += 1;
                    }
                    sel4::debug_println!("Event loop: mapped inherited page at 0x{:x}", page_addr);
                    return true; // Reply to resume thread
                }
                Err(e) => {
                    sel4::debug_println!("Event loop: failed to map inherited page: {:?}", e);
                    return false; // OOM - thread stays blocked
                }
            }
        }

        // All other faults are ERRORS - memory should be explicitly allocated
        // or pre-mapped at realm creation.
        let region = FaultRegion::from_addr(addr);

        match region {
            FaultRegion::ProcessPool => {
                sel4::debug_println!(
                    "FATAL: ProcessPool fault at 0x{:x} - VM must use lmm_request_pages()",
                    addr
                );
            }
            FaultRegion::RealmBinary => {
                sel4::debug_println!(
                    "FATAL: RealmBinary fault at 0x{:x} - should use explicit IPC",
                    addr
                );
            }
            FaultRegion::RealmLocal => {
                sel4::debug_println!(
                    "FATAL: RealmLocal fault at 0x{:x} - should use explicit IPC",
                    addr
                );
            }
            FaultRegion::WorkerStack(worker) => {
                sel4::debug_println!(
                    "FATAL: Stack overflow for worker {} at 0x{:x} - stacks are pre-mapped",
                    worker,
                    addr
                );
            }
            FaultRegion::Invalid => {
                sel4::debug_println!(
                    "FATAL: Invalid memory access at 0x{:x} (null guard, kernel space, etc.)",
                    addr
                );
            }
        }

        // Don't reply - thread stays blocked (effectively terminated)
        false
    }

    /// Handle a Timeout fault by replenishing budget and checking for
    /// interrupted page faults in inherited regions.
    ///
    /// In seL4 MCS, when a thread's budget expires during or near a page fault,
    /// the kernel delivers a Timeout fault instead of VMFault. We must:
    /// 1. Replenish the budget via `SchedControl` (otherwise thread re-times-out)
    /// 2. Check if MR1 contains an inherited region address (interrupted VMFault)
    /// 3. If so, map the page (idempotent - safe even if already mapped)
    /// 4. Reply to resume the thread
    fn handle_timeout_fault(&mut self, endpoint_bits: u64, mrs: [u64; 4]) -> bool {
        // MR0 = IP, MR1 = fault address (if interrupted VMFault), MR2 = data
        let ip = mrs[0];
        let possible_fault_addr = mrs[1];

        sel4::debug_println!(
            "Event loop: Timeout fault (ip=0x{:x}, addr=0x{:x})",
            ip,
            possible_fault_addr
        );

        // Find the realm
        let Some(realm) = self.find_realm_by_endpoint(endpoint_bits) else {
            sel4::debug_println!("Event loop: Timeout fault from unknown realm");
            return false;
        };

        // CRITICAL: Replenish budget before replying.
        // Without this, the thread will immediately timeout again.
        let sched_context = realm.sched_context;
        let budget_us = realm.budget_us;
        let period_us = realm.period_us;

        if let Err(e) = self.sched_control.sched_control_configure_flags(
            sched_context,
            budget_us,
            period_us,
            0, // extra_refills
            0, // badge
            0, // flags
        ) {
            sel4::debug_println!("Event loop: Failed to replenish budget: {:?}", e);
            // Still try to continue - maybe the thread can make progress
        }

        // Check if this looks like an interrupted page fault in inherited region.
        // If MR1 contains a valid inherited region address, map the page.
        if possible_fault_addr != 0 && is_inherited_region(possible_fault_addr) {
            let page_addr = possible_fault_addr & !(PAGE_SIZE - 1);

            // Re-find realm (we need vspace)
            let Some(realm) = self.find_realm_by_endpoint(endpoint_bits) else {
                return true; // Reply anyway - budget was replenished
            };
            let vspace = realm.vspace;

            // Lazy map the inherited region page (idempotent)
            match map_rw_frame(&mut self.slots, &mut self.untypeds, vspace, page_addr) {
                Ok(_) => {
                    if let Some(realm) = self.find_realm_by_endpoint(endpoint_bits) {
                        realm.pages_allocated += 1;
                    }
                    sel4::debug_println!(
                        "Event loop: Timeout+VMFault - mapped inherited page at 0x{:x}",
                        page_addr
                    );
                }
                Err(e) => {
                    // May fail if already mapped - that's OK
                    sel4::debug_println!(
                        "Event loop: Inherited page map result: {:?} (may be already mapped)",
                        e
                    );
                }
            }
        }

        // Always reply to resume the thread with replenished budget
        true
    }
}
