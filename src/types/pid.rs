//! Process identifier type.
//!
//! PIDs in Lona are compound identifiers containing both a realm ID and
//! a local process ID within that realm.

use core::default::Default;
use core::fmt;

/// A process identifier containing realm and local IDs.
///
/// PIDs are used to identify processes for message passing and linking.
/// They encode both which realm the process belongs to and its local ID
/// within that realm.
///
/// # Structure
///
/// The PID is packed as a 64-bit value:
/// - Upper 32 bits: realm ID
/// - Lower 32 bits: local process ID
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Pid(u64);

impl Pid {
    /// Create a new PID from realm and local IDs.
    #[inline]
    #[must_use]
    pub const fn new(realm_id: u32, local_id: u32) -> Self {
        Self(((realm_id as u64) << 32) | (local_id as u64))
    }

    /// Create a PID from a raw 64-bit value.
    #[inline]
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Create a null/invalid PID.
    #[inline]
    #[must_use]
    pub const fn null() -> Self {
        Self(0)
    }

    /// Check if this is a null PID.
    #[inline]
    #[must_use]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Get the realm ID component.
    #[inline]
    #[must_use]
    pub const fn realm_id(self) -> u32 {
        (self.0 >> 32) as u32
    }

    /// Get the local process ID component.
    #[inline]
    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Intentional: extracting lower 32 bits
    pub const fn local_id(self) -> u32 {
        self.0 as u32
    }

    /// Get the raw 64-bit value.
    #[inline]
    #[must_use]
    pub const fn as_raw(self) -> u64 {
        self.0
    }

    /// Check if this PID is in the same realm as another.
    #[inline]
    #[must_use]
    pub const fn same_realm(self, other: Self) -> bool {
        self.realm_id() == other.realm_id()
    }

    /// Check if this is a local PID (realm ID 0 indicates current realm).
    #[inline]
    #[must_use]
    pub const fn is_local(self) -> bool {
        self.realm_id() == 0
    }
}

impl fmt::Debug for Pid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pid({}, {})", self.realm_id(), self.local_id())
    }
}

impl fmt::Display for Pid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}.{}>", self.realm_id(), self.local_id())
    }
}

impl Default for Pid {
    fn default() -> Self {
        Self::null()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_construction() {
        let pid = Pid::new(5, 42);
        assert_eq!(pid.realm_id(), 5);
        assert_eq!(pid.local_id(), 42);
    }

    #[test]
    fn test_pid_null() {
        let null = Pid::null();
        assert!(null.is_null());
        assert_eq!(null.realm_id(), 0);
        assert_eq!(null.local_id(), 0);

        let not_null = Pid::new(1, 1);
        assert!(!not_null.is_null());
    }

    #[test]
    fn test_pid_raw_roundtrip() {
        let pid = Pid::new(0x1234, 0x5678);
        let raw = pid.as_raw();
        let restored = Pid::from_raw(raw);
        assert_eq!(pid, restored);
        assert_eq!(restored.realm_id(), 0x1234);
        assert_eq!(restored.local_id(), 0x5678);
    }

    #[test]
    fn test_pid_same_realm() {
        let pid1 = Pid::new(5, 1);
        let pid2 = Pid::new(5, 2);
        let pid3 = Pid::new(6, 1);

        assert!(pid1.same_realm(pid2));
        assert!(!pid1.same_realm(pid3));
    }

    #[test]
    fn test_pid_is_local() {
        let local = Pid::new(0, 42);
        let remote = Pid::new(5, 42);

        assert!(local.is_local());
        assert!(!remote.is_local());
    }

    #[test]
    fn test_pid_display() {
        let pid = Pid::new(5, 42);
        assert_eq!(format!("{pid}"), "<5.42>");
        assert_eq!(format!("{pid:?}"), "Pid(5, 42)");
    }

    #[test]
    fn test_pid_max_values() {
        let pid = Pid::new(u32::MAX, u32::MAX);
        assert_eq!(pid.realm_id(), u32::MAX);
        assert_eq!(pid.local_id(), u32::MAX);
    }
}
