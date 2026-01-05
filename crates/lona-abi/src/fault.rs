// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Fault information structures.
//!
//! When a realm encounters a fault (page fault, capability fault, etc.),
//! seL4 delivers it via IPC to the Lona Memory Manager. This module defines
//! the structures used to represent fault information.

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
}
