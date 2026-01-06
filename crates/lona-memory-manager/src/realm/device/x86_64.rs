// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! `x86_64` UART device setup.
//!
//! Sets up IOPort capability for COM1 serial port access.

use super::super::constants::{CNODE_SIZE_BITS, ROOT_CNODE_DEPTH};
use super::super::types::RealmError;
use crate::slots::SlotAllocator;
use lona_abi::types::CapSlot;
use sel4::cap_type::CNode;
use sel4::{Cap, CapRights};

/// Set up IOPort capability for UART.
///
/// Issues an IOPort capability for COM1 (0x3F8-0x3FF) and copies it
/// to the child realm's `CSpace` at the well-known slot.
pub fn setup_ioport_uart(
    slots: &mut SlotAllocator,
    child_cspace_slot: usize,
) -> Result<(), RealmError> {
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
