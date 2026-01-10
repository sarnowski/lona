// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Value allocation methods for Process.
//!
//! This module provides methods for allocating and reading basic heap values:
//! strings, pairs, symbols, keywords, tuples, and maps.

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::value::{HeapMap, HeapString, HeapTuple, Pair, Value};

use super::{MAX_INTERNED_KEYWORDS, MAX_INTERNED_SYMBOLS, MAX_METADATA_ENTRIES, Process};

impl Process {
    /// Allocate a string on the young heap.
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

    /// Allocate a pair on the young heap.
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

    /// Allocate a symbol on the young heap (same as string but tagged differently).
    ///
    /// Symbols are interned: the same symbol literal will return the same address.
    /// This enables O(1) equality comparison via address comparison and is required
    /// for namespace lookups (which compare symbol addresses).
    ///
    /// Returns a `Value::Symbol` pointing to the allocated symbol, or `None` if OOM.
    pub fn alloc_symbol<M: MemorySpace>(&mut self, mem: &mut M, name: &str) -> Option<Value> {
        // Check intern table for existing symbol
        for i in 0..self.symbol_intern_len {
            let addr = self.symbol_intern[i];
            let header: HeapString = mem.read(addr);
            if header.len as usize == name.len() {
                let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
                let bytes = mem.slice(data_addr, header.len as usize);
                if bytes == name.as_bytes() {
                    // Found existing interned symbol
                    return Some(Value::symbol(addr));
                }
            }
        }

        // Not found, allocate new symbol
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

        // Add to intern table if not full
        if self.symbol_intern_len < MAX_INTERNED_SYMBOLS {
            self.symbol_intern[self.symbol_intern_len] = addr;
            self.symbol_intern_len += 1;
        }

        Some(Value::symbol(addr))
    }

    /// Find an existing interned symbol by name (read-only lookup).
    ///
    /// Returns the symbol if found in the intern table, `None` otherwise.
    /// Unlike `alloc_symbol`, this does not allocate or modify the intern table.
    #[must_use]
    pub fn find_interned_symbol<M: MemorySpace>(&self, mem: &M, name: &str) -> Option<Value> {
        for i in 0..self.symbol_intern_len {
            let addr = self.symbol_intern[i];
            let header: HeapString = mem.read(addr);
            if header.len as usize == name.len() {
                let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
                let bytes = mem.slice(data_addr, header.len as usize);
                if bytes == name.as_bytes() {
                    return Some(Value::symbol(addr));
                }
            }
        }
        None
    }

    /// Allocate a keyword on the young heap (same as string but tagged differently).
    ///
    /// Keywords are interned: the same keyword literal will return the same address.
    /// This enables O(1) equality comparison via address comparison.
    ///
    /// Returns a `Value::Keyword` pointing to the allocated keyword, or `None` if OOM.
    pub fn alloc_keyword<M: MemorySpace>(&mut self, mem: &mut M, name: &str) -> Option<Value> {
        // Check intern table for existing keyword
        for i in 0..self.keyword_intern_len {
            let addr = self.keyword_intern[i];
            let header: HeapString = mem.read(addr);
            if header.len as usize == name.len() {
                let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
                let bytes = mem.slice(data_addr, header.len as usize);
                if bytes == name.as_bytes() {
                    // Found existing interned keyword
                    return Some(Value::keyword(addr));
                }
            }
        }

        // Not found, allocate new keyword
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

        // Add to intern table if not full
        if self.keyword_intern_len < MAX_INTERNED_KEYWORDS {
            self.keyword_intern[self.keyword_intern_len] = addr;
            self.keyword_intern_len += 1;
        }

        Some(Value::keyword(addr))
    }

    /// Allocate a tuple on the young heap.
    ///
    /// Returns a `Value::Tuple` pointing to the allocated tuple, or `None` if OOM.
    pub fn alloc_tuple<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        elements: &[Value],
    ) -> Option<Value> {
        let len = elements.len();
        let total_size = HeapTuple::alloc_size(len);

        // Allocate space (align to 8 bytes for Value fields)
        let addr = self.alloc(total_size, 8)?;

        // Write header
        let header = HeapTuple { len: len as u32 };
        mem.write(addr, header);

        // Write elements
        let data_addr = addr.add(HeapTuple::HEADER_SIZE as u64);
        for (i, &elem) in elements.iter().enumerate() {
            let elem_addr = data_addr.add((i * core::mem::size_of::<Value>()) as u64);
            mem.write(elem_addr, elem);
        }

        Some(Value::tuple(addr))
    }

    /// Read a heap-allocated string.
    ///
    /// Returns `None` if the value is not a string, symbol, or keyword.
    #[must_use]
    pub fn read_string<'a, M: MemorySpace>(&self, mem: &'a M, value: Value) -> Option<&'a str> {
        let (Value::String(addr) | Value::Symbol(addr) | Value::Keyword(addr)) = value else {
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

    /// Read a tuple's length from the heap.
    ///
    /// Returns `None` if the value is not a tuple.
    #[must_use]
    pub fn read_tuple_len<M: MemorySpace>(&self, mem: &M, value: Value) -> Option<usize> {
        let Value::Tuple(addr) = value else {
            return None;
        };

        let header: HeapTuple = mem.read(addr);
        Some(header.len as usize)
    }

    /// Read a tuple element at the given index.
    ///
    /// Returns `None` if the value is not a tuple or index is out of bounds.
    #[must_use]
    pub fn read_tuple_element<M: MemorySpace>(
        &self,
        mem: &M,
        value: Value,
        index: usize,
    ) -> Option<Value> {
        let Value::Tuple(addr) = value else {
            return None;
        };

        let header: HeapTuple = mem.read(addr);
        if index >= header.len as usize {
            return None;
        }

        let data_addr = addr.add(HeapTuple::HEADER_SIZE as u64);
        let elem_addr = data_addr.add((index * core::mem::size_of::<Value>()) as u64);
        Some(mem.read(elem_addr))
    }

    /// Allocate a map on the young heap.
    ///
    /// A map is an association list: a Pair chain where each `first` is a `[key value]` tuple.
    ///
    /// Returns a `Value::Map` pointing to the allocated map, or `None` if OOM.
    pub fn alloc_map<M: MemorySpace>(&mut self, mem: &mut M, entries: Value) -> Option<Value> {
        // Allocate space (align to 8 bytes for Value field)
        let addr = self.alloc(HeapMap::SIZE, 8)?;

        // Write the map
        let map = HeapMap { entries };
        mem.write(addr, map);

        Some(Value::map(addr))
    }

    /// Read a map from the heap.
    ///
    /// Returns `None` if the value is not a map.
    #[must_use]
    pub fn read_map<M: MemorySpace>(&self, mem: &M, value: Value) -> Option<HeapMap> {
        let Value::Map(addr) = value else {
            return None;
        };

        Some(mem.read(addr))
    }

    /// Set metadata for an object.
    ///
    /// Associates the given metadata map address with the object address.
    /// If the object already has metadata, it is replaced.
    /// If the metadata table is full, this is a silent no-op.
    pub fn set_metadata(&mut self, obj_addr: Vaddr, meta_addr: Vaddr) {
        // Check if already exists - update in place
        for i in 0..self.metadata_len {
            if self.metadata_keys[i] == obj_addr {
                self.metadata_values[i] = meta_addr;
                return;
            }
        }

        // Add new entry if table not full
        if self.metadata_len < MAX_METADATA_ENTRIES {
            self.metadata_keys[self.metadata_len] = obj_addr;
            self.metadata_values[self.metadata_len] = meta_addr;
            self.metadata_len += 1;
        }
    }

    /// Get metadata for an object.
    ///
    /// Returns the metadata map address if the object has metadata, `None` otherwise.
    #[must_use]
    pub fn get_metadata(&self, obj_addr: Vaddr) -> Option<Vaddr> {
        for i in 0..self.metadata_len {
            if self.metadata_keys[i] == obj_addr {
                return Some(self.metadata_values[i]);
            }
        }
        None
    }
}
