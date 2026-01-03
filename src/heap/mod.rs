// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Heap allocator for Lonala values.
//!
//! This is a simple bump allocator that grows downward. No deallocation
//! is supported (no GC per the minimal REPL requirements).
//!
//! Memory layout:
//! ```text
//! base (high address)
//!   │
//!   ▼  ← ptr starts here, moves down with each allocation
//!   │
//!   │ allocated objects
//!   │
//!   ▼
//! limit (low address)
//! ```

#[cfg(test)]
mod heap_test;

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::value::{HeapString, Pair, Value};
use core::option::Option::{self, None, Some};

/// Bump allocator that grows downward.
pub struct Heap {
    /// Top of heap region (high address).
    base: Vaddr,
    /// Current allocation pointer (grows down).
    ptr: Vaddr,
    /// Bottom limit (low address).
    limit: Vaddr,
}

impl Heap {
    /// Create a new heap with the given memory region.
    ///
    /// # Arguments
    /// * `base` - Top of the heap region (high address)
    /// * `size` - Size of the heap in bytes
    ///
    /// The heap grows downward from `base` toward `base - size`.
    #[must_use]
    pub const fn new(base: Vaddr, size: usize) -> Self {
        Self {
            base,
            ptr: base,
            limit: Vaddr::new(base.as_u64().saturating_sub(size as u64)),
        }
    }

    /// Returns the base (top) address of the heap.
    #[must_use]
    pub const fn base(&self) -> Vaddr {
        self.base
    }

    /// Returns the current allocation pointer.
    #[must_use]
    pub const fn ptr(&self) -> Vaddr {
        self.ptr
    }

    /// Returns the limit (bottom) address of the heap.
    #[must_use]
    pub const fn limit(&self) -> Vaddr {
        self.limit
    }

    /// Returns the number of bytes remaining in the heap.
    #[must_use]
    pub const fn remaining(&self) -> usize {
        self.ptr.diff(self.limit) as usize
    }

    /// Returns the number of bytes used.
    #[must_use]
    pub const fn used(&self) -> usize {
        self.base.diff(self.ptr) as usize
    }

    /// Allocate raw bytes with the given size and alignment.
    ///
    /// Returns `None` if there isn't enough space.
    pub fn alloc(&mut self, size: usize, align: usize) -> Option<Vaddr> {
        if size == 0 {
            return Some(self.ptr);
        }

        // Calculate aligned address (growing downward)
        let new_ptr = self.ptr.as_u64().checked_sub(size as u64)?;

        // Align down to the required alignment
        let mask = (align as u64).saturating_sub(1);
        let aligned = new_ptr & !mask;

        // Check if we have enough space
        if aligned < self.limit.as_u64() {
            return None;
        }

        self.ptr = Vaddr::new(aligned);
        Some(self.ptr)
    }

    /// Allocate a string on the heap.
    ///
    /// Returns a `Value::String` pointing to the allocated string, or `None` if OOM.
    pub fn alloc_string<M: MemorySpace>(&mut self, mem: &mut M, s: &str) -> Option<Value> {
        let len = s.len();
        let total_size = HeapString::alloc_size(len);

        // Allocate space (align to 4 bytes for the header)
        let addr = self.alloc(total_size, 4)?;

        // Write header
        let header = HeapString { len: len as u32 };
        mem.write(addr, header);

        // Write string data
        let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
        let dest = mem.slice_mut(data_addr, len);
        dest.copy_from_slice(s.as_bytes());

        Some(Value::string(addr))
    }

    /// Allocate a pair on the heap.
    ///
    /// Returns a `Value::Pair` pointing to the allocated pair, or `None` if OOM.
    pub fn alloc_pair<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        first: Value,
        rest: Value,
    ) -> Option<Value> {
        // Allocate space (align to 8 bytes for Value fields)
        let addr = self.alloc(Pair::SIZE, 8)?;

        // Write the pair
        let pair = Pair::new(first, rest);
        mem.write(addr, pair);

        Some(Value::pair(addr))
    }

    /// Allocate a symbol on the heap (same as string but tagged differently).
    ///
    /// Returns a `Value::Symbol` pointing to the allocated symbol, or `None` if OOM.
    pub fn alloc_symbol<M: MemorySpace>(&mut self, mem: &mut M, name: &str) -> Option<Value> {
        let len = name.len();
        let total_size = HeapString::alloc_size(len);

        // Allocate space (align to 4 bytes for the header)
        let addr = self.alloc(total_size, 4)?;

        // Write header
        let header = HeapString { len: len as u32 };
        mem.write(addr, header);

        // Write string data
        let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
        let dest = mem.slice_mut(data_addr, len);
        dest.copy_from_slice(name.as_bytes());

        Some(Value::symbol(addr))
    }

    /// Read a heap-allocated string.
    ///
    /// Returns `None` if the value is not a string or symbol.
    #[must_use]
    pub fn read_string<'a, M: MemorySpace>(&self, mem: &'a M, value: Value) -> Option<&'a str> {
        let (Value::String(addr) | Value::Symbol(addr)) = value else {
            return None;
        };

        let header: HeapString = mem.read(addr);
        let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
        let bytes = mem.slice(data_addr, header.len as usize);

        // We wrote valid UTF-8 when creating the string, but return None on error
        core::str::from_utf8(bytes).ok()
    }

    /// Read a pair from the heap.
    ///
    /// Returns `None` if the value is not a pair.
    #[must_use]
    pub fn read_pair<M: MemorySpace>(&self, mem: &M, value: Value) -> Option<Pair> {
        let Value::Pair(addr) = value else {
            return None;
        };

        Some(mem.read(addr))
    }
}
