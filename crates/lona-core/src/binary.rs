// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Binary buffer type with ownership semantics.
//!
//! `Binary` is the ONLY mutable type in Lonala. It provides efficient handling
//! of raw byte buffers for device drivers, network I/O, and DMA operations.
//!
//! # Ownership Model
//!
//! Binary buffers have two access modes:
//! - **Owned**: Full read/write access. The original creator of the buffer.
//! - **View**: Read-only access. Created via `clone()`, `view()`, or slicing a view.
//!
//! Cloning a `Binary` always produces a View, never a copy of the data. This
//! enables zero-copy passing of buffers between processes while maintaining
//! clear ownership semantics.
//!
//! # Zombie State
//!
//! When a buffer is transferred to another process, the original buffer
//! becomes a "zombie" - its data is removed but the structure remains.
//! Any access to a zombie buffer returns an error.

use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::fmt::{self, Debug, Display};
use core::hash::{Hash, Hasher};

/// Errors that can occur during binary buffer operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// Attempted to write to a read-only view.
    ReadOnly,
    /// The buffer has been transferred and is no longer accessible.
    Zombie,
    /// Index is outside the valid range.
    OutOfBounds,
}

impl Display for Error {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::ReadOnly => f.write_str("binary buffer is read-only"),
            Self::Zombie => f.write_str("binary buffer has been transferred"),
            Self::OutOfBounds => f.write_str("index out of bounds"),
        }
    }
}

/// Access mode for a binary buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Access {
    /// Full read/write access.
    Owned,
    /// Read-only access.
    View,
}

/// Internal buffer storage.
///
/// Contains the actual byte data and optional physical address for DMA.
struct Buffer {
    /// The buffer data. `None` indicates a zombie (transferred) buffer.
    data: Option<Vec<u8>>,
    /// Physical address for DMA operations. Set by `dma-alloc` (Task 1.8.10).
    phys_addr: Option<u64>,
}

/// A mutable binary buffer with ownership semantics.
///
/// Binary is the only mutable type in Lonala. Mutation is only possible
/// through the owning reference; views are always read-only.
///
/// # Display Format
///
/// - Owned: `#<binary:N owned>`
/// - View: `#<binary:N view>`
/// - Zombie: `#<binary:zombie>`
pub struct Binary {
    /// Shared reference to the underlying buffer.
    buffer: Rc<RefCell<Buffer>>,
    /// Access mode (owned or view).
    access: Access,
    /// Byte offset into the buffer (for zero-copy slicing).
    offset: usize,
    /// Length of this slice.
    len: usize,
}

impl Binary {
    /// Creates a new owned binary buffer filled with zeros.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let buf = Binary::new(1024);
    /// assert!(buf.is_owner());
    /// assert_eq!(buf.len(), 1024);
    /// ```
    #[inline]
    #[must_use]
    pub fn new(size: usize) -> Self {
        let data = alloc::vec![0_u8; size];
        let buffer = Buffer {
            data: Some(data),
            phys_addr: None,
        };
        Self {
            buffer: Rc::new(RefCell::new(buffer)),
            access: Access::Owned,
            offset: 0_usize,
            len: size,
        }
    }

    /// Creates a new owned binary buffer from existing data.
    #[inline]
    #[must_use]
    pub fn from_vec(data: Vec<u8>) -> Self {
        let len = data.len();
        let buffer = Buffer {
            data: Some(data),
            phys_addr: None,
        };
        Self {
            buffer: Rc::new(RefCell::new(buffer)),
            access: Access::Owned,
            offset: 0_usize,
            len,
        }
    }

    /// Returns the length of this binary buffer.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if this binary buffer is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len == 0_usize
    }

    /// Returns `true` if this is an owned buffer with write access.
    #[inline]
    #[must_use]
    pub const fn is_owner(&self) -> bool {
        matches!(self.access, Access::Owned)
    }

    /// Returns `true` if this buffer has been transferred (zombie state).
    #[inline]
    #[must_use]
    pub fn is_zombie(&self) -> bool {
        self.buffer.borrow().data.is_none()
    }

    /// Gets a byte at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    /// Returns `Err(Error::Zombie)` if the buffer has been transferred.
    #[inline]
    pub fn get(&self, index: usize) -> Result<Option<u8>, Error> {
        if index >= self.len {
            return Ok(None);
        }

        let buffer = self.buffer.borrow();
        let data = buffer.data.as_ref().ok_or(Error::Zombie)?;
        let actual_index = self.offset.checked_add(index).ok_or(Error::OutOfBounds)?;
        Ok(data.get(actual_index).copied())
    }

    /// Sets a byte at the given index.
    ///
    /// Returns `Err(Error::ReadOnly)` if this is a view.
    /// Returns `Err(Error::Zombie)` if the buffer has been transferred.
    /// Returns `Err(Error::OutOfBounds)` if the index is out of bounds.
    #[inline]
    pub fn set(&self, index: usize, value: u8) -> Result<(), Error> {
        if !self.is_owner() {
            return Err(Error::ReadOnly);
        }

        if index >= self.len {
            return Err(Error::OutOfBounds);
        }

        let mut buffer = self.buffer.borrow_mut();
        let data = buffer.data.as_mut().ok_or(Error::Zombie)?;
        let actual_index = self.offset.checked_add(index).ok_or(Error::OutOfBounds)?;
        let byte = data.get_mut(actual_index).ok_or(Error::OutOfBounds)?;
        *byte = value;
        Ok(())
    }

    /// Creates a slice of this buffer.
    ///
    /// The slice inherits the access mode from the parent:
    /// - Owned parent → Owned slice
    /// - View parent → View slice
    ///
    /// Returns `None` if the slice would exceed the buffer bounds.
    #[inline]
    #[must_use]
    pub fn slice(&self, start: usize, slice_len: usize) -> Option<Self> {
        // Check bounds
        let end = start.checked_add(slice_len)?;
        if end > self.len {
            return None;
        }

        let new_offset = self.offset.checked_add(start)?;

        Some(Self {
            buffer: Rc::clone(&self.buffer),
            access: self.access,
            offset: new_offset,
            len: slice_len,
        })
    }

    /// Creates a read-only view of this buffer.
    ///
    /// Always returns a View, regardless of whether this is Owned or View.
    #[inline]
    #[must_use]
    pub fn view(&self) -> Self {
        Self {
            buffer: Rc::clone(&self.buffer),
            access: Access::View,
            offset: self.offset,
            len: self.len,
        }
    }

    /// Returns the buffer contents as a slice.
    ///
    /// # Errors
    ///
    /// Returns `Err(Error::Zombie)` if the buffer has been transferred.
    ///
    /// # Note
    ///
    /// This borrows the internal `RefCell`, so the returned reference must
    /// not outlive this borrow. For longer-lived access, copy the data.
    #[inline]
    pub fn as_bytes(&self) -> Result<Ref<'_>, Error> {
        let buffer = self.buffer.borrow();
        if buffer.data.is_none() {
            return Err(Error::Zombie);
        }
        Ok(Ref {
            buffer,
            offset: self.offset,
            len: self.len,
        })
    }

    /// Returns the access mode of this buffer.
    #[inline]
    #[must_use]
    pub const fn access(&self) -> Access {
        self.access
    }

    /// Returns the physical address of this buffer, if known.
    ///
    /// Physical addresses are set by DMA allocation and are used for
    /// hardware DMA operations. Regular buffers have no physical address.
    #[inline]
    #[must_use]
    pub fn phys_addr(&self) -> Option<u64> {
        self.buffer.borrow().phys_addr
    }

    /// Sets the physical address for DMA operations.
    ///
    /// This is called by the DMA allocator when allocating DMA-capable memory.
    ///
    /// # Errors
    ///
    /// Returns `Err(Error::ReadOnly)` if this is a view (not owned).
    /// Returns `Err(Error::Zombie)` if the buffer has been transferred.
    #[inline]
    pub fn set_phys_addr(&self, addr: u64) -> Result<(), Error> {
        if !self.is_owner() {
            return Err(Error::ReadOnly);
        }

        let mut buffer = self.buffer.borrow_mut();
        if buffer.data.is_none() {
            return Err(Error::Zombie);
        }
        buffer.phys_addr = Some(addr);
        Ok(())
    }
}

/// A borrowed reference to binary data.
///
/// This type holds the `RefCell` borrow and provides access to the slice.
pub struct Ref<'borrow> {
    buffer: core::cell::Ref<'borrow, Buffer>,
    offset: usize,
    len: usize,
}

impl Ref<'_> {
    /// Returns the borrowed bytes as a slice.
    #[inline]
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        // We verified data.is_some() before creating Ref, so use empty slice as fallback
        self.buffer
            .data
            .as_ref()
            .and_then(|data| data.get(self.offset..self.offset.saturating_add(self.len)))
            .unwrap_or(&[])
    }
}

impl core::ops::Deref for Ref<'_> {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl Clone for Binary {
    /// Clones this buffer, always producing a View.
    ///
    /// This enables zero-copy sharing of buffers - the clone shares the
    /// same underlying data but can only read, not write.
    #[inline]
    fn clone(&self) -> Self {
        self.view()
    }
}

impl PartialEq for Binary {
    /// Compares two binary buffers by content.
    ///
    /// Two buffers are equal if they have the same bytes, regardless of
    /// access mode or whether they share the same underlying buffer.
    ///
    /// Zombie buffers are never equal to anything (including other zombies).
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        // Zombie buffers are never equal
        let Ok(self_ref) = self.as_bytes() else {
            return false;
        };
        let Ok(other_ref) = other.as_bytes() else {
            return false;
        };
        self_ref.as_slice() == other_ref.as_slice()
    }
}

impl Eq for Binary {}

impl Hash for Binary {
    /// Hashes the binary buffer.
    ///
    /// **Warning**: Binary is a mutable type and should not be used as map keys.
    /// All Binary values hash to the same constant, causing deliberate collisions.
    /// This makes Binary unusable as a map key in practice - use immutable types
    /// for map keys instead.
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        // All Binary values hash to the same constant, making them unusable
        // as map keys (which is intentional - mutable types shouldn't be keys).
        0_u8.hash(state);
    }
}

impl Display for Binary {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_zombie() {
            f.write_str("#<binary:zombie>")
        } else {
            let mode = match self.access {
                Access::Owned => "owned",
                Access::View => "view",
            };
            write!(f, "#<binary:{} {mode}>", self.len)
        }
    }
}

impl Debug for Binary {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Binary")
            .field("access", &self.access)
            .field("offset", &self.offset)
            .field("len", &self.len)
            .field("is_zombie", &self.is_zombie())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation_produces_owned_zeroed_buffer() {
        let buf = Binary::new(10_usize);
        assert!(buf.is_owner());
        assert!(!buf.is_zombie());
        assert_eq!(buf.len(), 10_usize);

        // Verify all bytes are zero
        for i in 0_usize..10_usize {
            assert_eq!(buf.get(i).unwrap(), Some(0_u8));
        }
    }

    #[test]
    fn from_vec_preserves_data() {
        let data = alloc::vec![1_u8, 2_u8, 3_u8, 4_u8, 5_u8];
        let buf = Binary::from_vec(data);
        assert!(buf.is_owner());
        assert_eq!(buf.len(), 5_usize);
        assert_eq!(buf.get(0_usize).unwrap(), Some(1_u8));
        assert_eq!(buf.get(4_usize).unwrap(), Some(5_u8));
    }

    #[test]
    fn len_and_is_empty() {
        let buf = Binary::new(10_usize);
        assert_eq!(buf.len(), 10_usize);
        assert!(!buf.is_empty());

        let empty = Binary::new(0_usize);
        assert_eq!(empty.len(), 0_usize);
        assert!(empty.is_empty());
    }

    #[test]
    fn is_owner_returns_correct_access_mode() {
        let owned = Binary::new(10_usize);
        assert!(owned.is_owner());

        let view = owned.view();
        assert!(!view.is_owner());
    }

    #[test]
    fn get_and_set_with_owned_buffer() {
        let buf = Binary::new(10_usize);

        // Set some values
        assert!(buf.set(0_usize, 42_u8).is_ok());
        assert!(buf.set(9_usize, 99_u8).is_ok());

        // Read them back
        assert_eq!(buf.get(0_usize).unwrap(), Some(42_u8));
        assert_eq!(buf.get(9_usize).unwrap(), Some(99_u8));
    }

    #[test]
    fn get_with_view_works() {
        let owned = Binary::new(10_usize);
        owned.set(5_usize, 123_u8).unwrap();

        let view = owned.view();
        assert_eq!(view.get(5_usize).unwrap(), Some(123_u8));
    }

    #[test]
    fn set_with_view_returns_read_only_error() {
        let owned = Binary::new(10_usize);
        let view = owned.view();

        assert_eq!(view.set(0_usize, 42_u8), Err(Error::ReadOnly));
    }

    #[test]
    fn get_out_of_bounds_returns_none() {
        let buf = Binary::new(10_usize);
        assert_eq!(buf.get(10_usize).unwrap(), None);
        assert_eq!(buf.get(100_usize).unwrap(), None);
    }

    #[test]
    fn set_out_of_bounds_returns_error() {
        let buf = Binary::new(10_usize);
        assert_eq!(buf.set(10_usize, 42_u8), Err(Error::OutOfBounds));
        assert_eq!(buf.set(100_usize, 42_u8), Err(Error::OutOfBounds));
    }

    #[test]
    fn slice_of_owned_returns_owned() {
        let owned = Binary::new(100_usize);
        owned.set(50_usize, 42_u8).unwrap();

        let slice = owned.slice(25_usize, 50_usize).unwrap();
        assert!(slice.is_owner());
        assert_eq!(slice.len(), 50_usize);

        // Index 50 in original is index 25 in slice
        assert_eq!(slice.get(25_usize).unwrap(), Some(42_u8));

        // Slice can still write (it's owned)
        assert!(slice.set(0_usize, 99_u8).is_ok());
    }

    #[test]
    fn slice_of_view_returns_view() {
        let owned = Binary::new(100_usize);
        let view = owned.view();

        let slice = view.slice(25_usize, 50_usize).unwrap();
        assert!(!slice.is_owner());
        assert_eq!(slice.len(), 50_usize);
    }

    #[test]
    fn slice_bounds_checking() {
        let buf = Binary::new(100_usize);

        // Valid slice
        assert!(buf.slice(0_usize, 100_usize).is_some());
        assert!(buf.slice(50_usize, 50_usize).is_some());
        assert!(buf.slice(99_usize, 1_usize).is_some());
        assert!(buf.slice(100_usize, 0_usize).is_some());

        // Invalid slices
        assert!(buf.slice(0_usize, 101_usize).is_none());
        assert!(buf.slice(50_usize, 51_usize).is_none());
        assert!(buf.slice(101_usize, 0_usize).is_none());
    }

    #[test]
    fn view_always_returns_view() {
        let owned = Binary::new(10_usize);
        let view1 = owned.view();
        let view2 = view1.view();

        assert!(!view1.is_owner());
        assert!(!view2.is_owner());
    }

    #[test]
    fn clone_always_produces_view() {
        let owned = Binary::new(10_usize);
        let cloned = owned.clone();

        assert!(owned.is_owner());
        assert!(!cloned.is_owner());

        // Clone of a view is also a view
        let view = owned.view();
        let view_clone = view.clone();
        assert!(!view_clone.is_owner());
    }

    #[test]
    fn equality_based_on_content() {
        let buf1 = Binary::from_vec(alloc::vec![1_u8, 2_u8, 3_u8]);
        let buf2 = Binary::from_vec(alloc::vec![1_u8, 2_u8, 3_u8]);
        let buf3 = Binary::from_vec(alloc::vec![1_u8, 2_u8, 4_u8]);

        assert_eq!(buf1, buf2);
        assert_ne!(buf1, buf3);
    }

    #[test]
    fn equality_ignores_access_mode() {
        let owned = Binary::from_vec(alloc::vec![1_u8, 2_u8, 3_u8]);
        let view = owned.view();

        assert_eq!(owned, view);
    }

    #[test]
    fn display_format() {
        let owned = Binary::new(1024_usize);
        assert_eq!(alloc::format!("{owned}"), "#<binary:1024 owned>");

        let view = owned.view();
        assert_eq!(alloc::format!("{view}"), "#<binary:1024 view>");
    }

    #[test]
    fn display_empty_buffer() {
        let empty = Binary::new(0_usize);
        assert_eq!(alloc::format!("{empty}"), "#<binary:0 owned>");
    }

    #[test]
    fn as_bytes_returns_correct_slice() {
        let buf = Binary::from_vec(alloc::vec![1_u8, 2_u8, 3_u8, 4_u8, 5_u8]);
        let bytes = buf.as_bytes().unwrap();
        assert_eq!(bytes.as_slice(), &[1_u8, 2_u8, 3_u8, 4_u8, 5_u8]);
    }

    #[test]
    fn as_bytes_respects_slice_bounds() {
        let buf = Binary::from_vec(alloc::vec![1_u8, 2_u8, 3_u8, 4_u8, 5_u8]);
        let slice = buf.slice(1_usize, 3_usize).unwrap();
        let bytes = slice.as_bytes().unwrap();
        assert_eq!(bytes.as_slice(), &[2_u8, 3_u8, 4_u8]);
    }

    #[test]
    fn views_share_underlying_data() {
        let owned = Binary::new(10_usize);
        let view = owned.view();

        // Modify through owned
        owned.set(5_usize, 42_u8).unwrap();

        // Change is visible through view
        assert_eq!(view.get(5_usize).unwrap(), Some(42_u8));
    }

    #[test]
    fn slices_share_underlying_data() {
        let owned = Binary::new(100_usize);
        let slice = owned.slice(25_usize, 50_usize).unwrap();

        // Modify through slice (it's also owned)
        slice.set(0_usize, 99_u8).unwrap();

        // Change is visible in original at offset 25
        assert_eq!(owned.get(25_usize).unwrap(), Some(99_u8));
    }

    #[test]
    fn phys_addr_operations() {
        let owned = Binary::new(10_usize);
        assert_eq!(owned.phys_addr(), None);

        // Set physical address
        owned.set_phys_addr(0x1000_u64).unwrap();
        assert_eq!(owned.phys_addr(), Some(0x1000_u64));

        // View can read but not write
        let view = owned.view();
        assert_eq!(view.phys_addr(), Some(0x1000_u64));
        assert_eq!(view.set_phys_addr(0x2000_u64), Err(Error::ReadOnly));
    }
}
