// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Value allocation methods for Process.
//!
//! This module provides methods for allocating and reading basic heap values:
//! strings, pairs, symbols, keywords, tuples, and maps.

use crate::platform::MemorySpace;
use crate::value::{HeapMap, HeapString, HeapTuple, Pair, Value};

use super::Process;

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
    /// This allocates without interning. For interned symbols (with identity semantics),
    /// use `Realm::intern_symbol()` instead.
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

    /// Allocate a keyword on the young heap (same as string but tagged differently).
    ///
    /// This allocates without interning. For interned keywords (with identity semantics),
    /// use `Realm::intern_keyword()` instead.
    ///
    /// Returns a `Value::Keyword` pointing to the allocated keyword, or `None` if OOM.
    pub fn alloc_keyword<M: MemorySpace>(&mut self, mem: &mut M, name: &str) -> Option<Value> {
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
        let header = HeapTuple {
            len: len as u32,
            padding: 0,
        };
        mem.write(addr, header);

        // Write elements
        let data_addr = addr.add(HeapTuple::HEADER_SIZE as u64);
        for (i, &elem) in elements.iter().enumerate() {
            let elem_addr = data_addr.add((i * core::mem::size_of::<Value>()) as u64);
            mem.write(elem_addr, elem);
        }

        Some(Value::tuple(addr))
    }

    /// Allocate a vector on the young heap.
    ///
    /// Vectors share the same memory layout as tuples (length + elements).
    /// Returns a `Value::Vector` pointing to the allocated vector, or `None` if OOM.
    pub fn alloc_vector<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        elements: &[Value],
    ) -> Option<Value> {
        let len = elements.len();
        let total_size = HeapTuple::alloc_size(len);

        // Allocate space (align to 8 bytes for Value fields)
        let addr = self.alloc(total_size, 8)?;

        // Write header
        let header = HeapTuple {
            len: len as u32,
            padding: 0,
        };
        mem.write(addr, header);

        // Write elements
        let data_addr = addr.add(HeapTuple::HEADER_SIZE as u64);
        for (i, &elem) in elements.iter().enumerate() {
            let elem_addr = data_addr.add((i * core::mem::size_of::<Value>()) as u64);
            mem.write(elem_addr, elem);
        }

        Some(Value::vector(addr))
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

    /// Read a tuple or vector's length from the heap.
    ///
    /// Returns `None` if the value is not a tuple or vector.
    #[must_use]
    pub fn read_tuple_len<M: MemorySpace>(&self, mem: &M, value: Value) -> Option<usize> {
        let (Value::Tuple(addr) | Value::Vector(addr)) = value else {
            return None;
        };

        let header: HeapTuple = mem.read(addr);
        Some(header.len as usize)
    }

    /// Read a tuple or vector element at the given index.
    ///
    /// Returns `None` if the value is not a tuple/vector or index is out of bounds.
    #[must_use]
    pub fn read_tuple_element<M: MemorySpace>(
        &self,
        mem: &M,
        value: Value,
        index: usize,
    ) -> Option<Value> {
        let (Value::Tuple(addr) | Value::Vector(addr)) = value else {
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

    /// Read a vector's length from the heap.
    ///
    /// Returns `None` if the value is not a vector.
    /// Use this when you need to match specifically on vectors (not tuples).
    #[must_use]
    pub fn read_vector_len<M: MemorySpace>(&self, mem: &M, value: Value) -> Option<usize> {
        let Value::Vector(addr) = value else {
            return None;
        };

        let header: HeapTuple = mem.read(addr);
        Some(header.len as usize)
    }

    /// Read a vector element at the given index.
    ///
    /// Returns `None` if the value is not a vector or index is out of bounds.
    /// Use this when you need to match specifically on vectors (not tuples).
    #[must_use]
    pub fn read_vector_element<M: MemorySpace>(
        &self,
        mem: &M,
        value: Value,
        index: usize,
    ) -> Option<Value> {
        let Value::Vector(addr) = value else {
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

    /// Look up a key in a map.
    ///
    /// Returns `Some(value)` if the key is found, `None` if not found or not a map.
    #[must_use]
    pub fn map_get<M: MemorySpace>(&self, mem: &M, map: Value, key: Value) -> Option<Value> {
        use crate::intrinsics::core_get;

        core_get(self, mem, map, key, Value::Nil)
            .ok()
            .filter(|v: &Value| !v.is_nil())
    }
}
