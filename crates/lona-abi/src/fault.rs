// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Fault information structures.
//!
//! When a realm encounters a fault (page fault, capability fault, etc.),
//! seL4 delivers it via IPC to the Lona Memory Manager. This module defines
//! the structures used to represent fault information.

use crate::types::Vaddr;

// =============================================================================
// seL4 Fault Label Constants
// =============================================================================

/// seL4 fault label for VM faults (page faults).
///
/// When a thread accesses unmapped memory, seL4 sends a `VMFault` IPC message
/// to the thread's fault endpoint with this label.
pub const SEL4_FAULT_VM_FAULT: u64 = 5;

/// seL4 fault label for Timeout faults (MCS scheduler).
///
/// In seL4 MCS, when a thread's scheduling budget expires, a Timeout fault
/// is delivered to its fault endpoint. This can also occur when budget
/// expires during or near a page fault, causing Timeout to be delivered
/// instead of `VMFault`.
///
/// Timeout fault message format:
/// - MR0: Instruction pointer at timeout
/// - MR1: Fault address (if interrupted a `VMFault`) or 0
/// - MR2: Data (architecture-specific)
pub const SEL4_FAULT_TIMEOUT: u64 = 6;

// =============================================================================
// VM Fault Info (seL4 Message Parsing)
// =============================================================================

/// VM fault info extracted from seL4 message registers.
///
/// seL4 `VMFault` message format:
/// - MR0: Instruction pointer (where the fault occurred)
/// - MR1: Fault address (the unmapped address accessed)
/// - MR2: Prefetch fault flag (1 = instruction fetch, 0 = data access)
/// - MR3: Fault status register (architecture-specific)
#[derive(Debug, Clone, Copy)]
pub struct VmFaultInfo {
    /// Faulting instruction pointer.
    pub ip: u64,
    /// Faulting virtual address.
    pub addr: Vaddr,
    /// True if this was a prefetch (instruction) fault.
    pub is_prefetch: bool,
    /// Fault status register (architecture-specific).
    pub fsr: u64,
}

impl VmFaultInfo {
    /// Parse VM fault from seL4 message registers.
    ///
    /// # Arguments
    ///
    /// * `mrs` - The four message registers [MR0, MR1, MR2, MR3]
    #[inline]
    #[must_use]
    pub const fn from_mrs(mrs: [u64; 4]) -> Self {
        Self {
            ip: mrs[0],
            addr: Vaddr::new(mrs[1]),
            is_prefetch: mrs[2] != 0,
            fsr: mrs[3],
        }
    }
}

// =============================================================================
// Generic Fault Info (Higher-Level Abstraction)
// =============================================================================

/// Information about a fault that occurred in a realm.
///
/// The Lona Memory Manager receives this information from seL4's fault IPC
/// and uses it to determine how to handle the fault.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct FaultInfo {
    /// Type of fault that occurred.
    pub fault_type: FaultType,

    /// Virtual address that caused the fault (for page faults).
    pub fault_addr: u64,

    /// Instruction pointer at the time of fault.
    pub ip: u64,

    /// True if this was a write access (vs read).
    pub is_write: bool,

    /// True if this was an instruction fetch (vs data access).
    pub is_instruction: bool,
}

impl FaultInfo {
    /// Creates a new page fault info.
    #[inline]
    #[must_use]
    pub const fn page_fault(fault_addr: u64, ip: u64, is_write: bool) -> Self {
        Self {
            fault_type: FaultType::PageFault,
            fault_addr,
            ip,
            is_write,
            is_instruction: false,
        }
    }

    /// Creates a new instruction fetch fault info.
    #[inline]
    #[must_use]
    pub const fn instruction_fault(fault_addr: u64, ip: u64) -> Self {
        Self {
            fault_type: FaultType::PageFault,
            fault_addr,
            ip,
            is_write: false,
            is_instruction: true,
        }
    }
}

/// Type of fault that can occur in a realm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FaultType {
    /// Page fault - access to unmapped virtual address.
    ///
    /// This is the most common fault. The Lona Memory Manager will:
    /// 1. Check if the address is in a valid region
    /// 2. Check permissions (read/write/execute)
    /// 3. Map a frame if valid, or terminate the realm if invalid
    PageFault = 0,

    /// Capability fault - invalid capability operation.
    ///
    /// Occurs when a realm tries to use a capability it doesn't have
    /// or uses a capability incorrectly.
    CapFault = 1,

    /// Unknown syscall - realm invoked an unknown seL4 syscall.
    UnknownSyscall = 2,

    /// User exception - CPU exception (e.g., divide by zero, alignment).
    UserException = 3,
}

impl FaultType {
    /// Returns true if this fault type can potentially be recovered from.
    ///
    /// Page faults are usually recoverable (by mapping a page).
    /// Other faults typically indicate a bug and may terminate the realm.
    #[inline]
    #[must_use]
    pub const fn is_recoverable(self) -> bool {
        matches!(self, Self::PageFault)
    }

    /// Returns a human-readable name for this fault type.
    #[inline]
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PageFault => "page fault",
            Self::CapFault => "capability fault",
            Self::UnknownSyscall => "unknown syscall",
            Self::UserException => "user exception",
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn page_fault_is_recoverable() {
        assert!(FaultType::PageFault.is_recoverable());
        assert!(!FaultType::CapFault.is_recoverable());
        assert!(!FaultType::UnknownSyscall.is_recoverable());
        assert!(!FaultType::UserException.is_recoverable());
    }

    #[test]
    fn fault_info_constructors() {
        let pf = FaultInfo::page_fault(0x1000, 0x2000, true);
        assert_eq!(pf.fault_type, FaultType::PageFault);
        assert_eq!(pf.fault_addr, 0x1000);
        assert_eq!(pf.ip, 0x2000);
        assert!(pf.is_write);
        assert!(!pf.is_instruction);

        let inf = FaultInfo::instruction_fault(0x3000, 0x3000);
        assert!(inf.is_instruction);
        assert!(!inf.is_write);
    }

    // =========================================================================
    // VmFaultInfo Tests
    // =========================================================================

    #[test]
    fn vm_fault_info_from_mrs_data_fault() {
        // Simulate a data read fault at address 0x1_0000_0000
        let mrs: [u64; 4] = [
            0x0000_0001_0000_1234, // IP
            0x0000_0001_0000_0000, // Fault address
            0,                     // Not a prefetch fault (data access)
            0x0000_0000_0000_0006, // FSR (example value)
        ];

        let fault = VmFaultInfo::from_mrs(mrs);

        assert_eq!(fault.ip, 0x0000_0001_0000_1234);
        assert_eq!(fault.addr.as_u64(), 0x0000_0001_0000_0000);
        assert!(!fault.is_prefetch);
        assert_eq!(fault.fsr, 0x0000_0000_0000_0006);
    }

    #[test]
    fn vm_fault_info_from_mrs_prefetch_fault() {
        // Simulate an instruction fetch fault
        let mrs: [u64; 4] = [
            0x0000_0002_0000_0000, // IP
            0x0000_0002_0000_0000, // Fault address (same as IP for prefetch)
            1,                     // Is a prefetch fault (instruction fetch)
            0x0000_0000_0000_0005, // FSR (example value)
        ];

        let fault = VmFaultInfo::from_mrs(mrs);

        assert_eq!(fault.ip, 0x0000_0002_0000_0000);
        assert_eq!(fault.addr.as_u64(), 0x0000_0002_0000_0000);
        assert!(fault.is_prefetch);
        assert_eq!(fault.fsr, 0x0000_0000_0000_0005);
    }

    #[test]
    fn vm_fault_info_addr_alignment() {
        // Verify non-page-aligned addresses are preserved correctly
        let mrs: [u64; 4] = [
            0x1000,                // IP
            0x0000_0020_0000_0123, // Non-page-aligned address
            0,                     // Data fault
            0,                     // FSR
        ];

        let fault = VmFaultInfo::from_mrs(mrs);

        // The address should be preserved exactly (page alignment is done elsewhere)
        assert_eq!(fault.addr.as_u64(), 0x0000_0020_0000_0123);
    }

    #[test]
    fn sel4_fault_vm_fault_constant() {
        // Verify the constant matches seL4's definition
        assert_eq!(SEL4_FAULT_VM_FAULT, 5);
    }

    #[test]
    fn sel4_fault_timeout_constant() {
        // Verify the constant matches seL4 MCS's definition
        assert_eq!(SEL4_FAULT_TIMEOUT, 6);
    }
}
