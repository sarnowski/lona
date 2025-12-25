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
    /// Returns `None` for zombie (transferred) buffers to prevent stale address leaks.
    #[inline]
    #[must_use]
    pub fn phys_addr(&self) -> Option<u64> {
        let buffer = self.buffer.borrow();
        // Don't leak physical addresses for zombie buffers
        buffer.data.as_ref()?;
        buffer.phys_addr
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
#[path = "binary_tests.rs"]
mod tests;
