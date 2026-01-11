// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Deep copy infrastructure for copying values from process heap to realm code region.
//!
//! When `def` stores a value in the realm, all heap-allocated data must be copied
//! to the realm's code region. This module provides the deep copy machinery.
//!
//! Both process heap and realm code region exist within the same `MemorySpace`.
//! Deep copy allocates in the realm's region and writes to the same `mem`.

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::value::{HeapClosure, HeapCompiledFn, HeapMap, HeapString, HeapTuple, Pair, Value};

use super::Realm;

/// Maximum depth for visited tracking (prevents stack overflow on deeply nested data).
const MAX_VISITED: usize = 256;

/// Tracks visited addresses during deep copy to handle shared structure.
///
/// When the same heap object is referenced multiple times in a value graph,
/// we only copy it once and reuse the destination address for subsequent references.
pub struct VisitedTracker {
    /// Source addresses that have been copied.
    src: [Vaddr; MAX_VISITED],
    /// Corresponding destination addresses.
    dst: [Vaddr; MAX_VISITED],
    /// Number of tracked entries.
    len: usize,
}

impl VisitedTracker {
    /// Create a new empty tracker.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            src: [Vaddr::new(0); MAX_VISITED],
            dst: [Vaddr::new(0); MAX_VISITED],
            len: 0,
        }
    }

    /// Check if a source address has already been copied.
    ///
    /// Returns the destination address if found, `None` otherwise.
    #[must_use]
    pub fn check(&self, src_addr: Vaddr) -> Option<Vaddr> {
        for i in 0..self.len {
            if self.src[i] == src_addr {
                return Some(self.dst[i]);
            }
        }
        None
    }

    /// Record a source→destination mapping.
    ///
    /// Returns `false` if the tracker is full (should not happen in practice
    /// as `MAX_VISITED` is sized for typical use cases).
    pub const fn record(&mut self, src_addr: Vaddr, dst_addr: Vaddr) -> bool {
        if self.len >= MAX_VISITED {
            return false;
        }
        self.src[self.len] = src_addr;
        self.dst[self.len] = dst_addr;
        self.len += 1;
        true
    }
}

impl Default for VisitedTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Deep copy a value from process heap to realm code region.
///
/// This is the main entry point for copying values to the realm. It handles
/// all value types, recursively copying heap-allocated structures.
///
/// # Arguments
/// * `value` - The value to copy
/// * `realm` - The realm to allocate in
/// * `mem` - The memory space (shared between process and realm)
/// * `visited` - Tracker for already-copied addresses
///
/// # Returns
/// The copied value with all pointers updated to realm addresses,
/// or `None` if allocation fails.
pub fn deep_copy_to_realm<M: MemorySpace>(
    value: Value,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Value> {
    match value {
        // Immediates: no copy needed, pass through directly
        Value::Nil | Value::Bool(_) | Value::Int(_) | Value::NativeFn(_) | Value::Unbound => {
            Some(value)
        }

        // Realm references: already in realm, pass through
        // (Vars and Namespaces are created in realm, not process heap)
        Value::Var(_) | Value::Namespace(_) => Some(value),

        // Strings: copy bytes to realm
        Value::String(addr) => {
            if let Some(dst) = visited.check(addr) {
                return Some(Value::string(dst));
            }
            deep_copy_string(addr, realm, mem, visited, Value::string)
        }

        // Symbols: re-intern in realm (deduplication)
        Value::Symbol(addr) => {
            // Read the symbol's string content and copy to local buffer
            let header: HeapString = mem.read(addr);
            let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
            let len = header.len as usize;

            // Copy to local buffer to avoid borrow conflict
            let mut buf = [0u8; 256];
            if len > buf.len() {
                return None; // Symbol name too long
            }
            let bytes = mem.slice(data_addr, len);
            buf[..len].copy_from_slice(bytes);

            // Convert to string for intern lookup
            let name = core::str::from_utf8(&buf[..len]).ok()?;
            realm.intern_symbol(mem, name)
        }

        // Keywords: re-intern in realm (deduplication)
        Value::Keyword(addr) => {
            // Read the keyword's string content and copy to local buffer
            let header: HeapString = mem.read(addr);
            let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
            let len = header.len as usize;

            // Copy to local buffer to avoid borrow conflict
            let mut buf = [0u8; 256];
            if len > buf.len() {
                return None; // Keyword name too long
            }
            let bytes = mem.slice(data_addr, len);
            buf[..len].copy_from_slice(bytes);

            // Convert to string for intern lookup
            let name = core::str::from_utf8(&buf[..len]).ok()?;
            realm.intern_keyword(mem, name)
        }

        // Pairs: recursive copy
        Value::Pair(addr) => {
            if let Some(dst) = visited.check(addr) {
                return Some(Value::pair(dst));
            }
            deep_copy_pair(addr, realm, mem, visited)
        }

        // Tuples: recursive copy
        Value::Tuple(addr) => {
            if let Some(dst) = visited.check(addr) {
                return Some(Value::tuple(dst));
            }
            deep_copy_tuple(addr, realm, mem, visited)
        }

        // Vectors: recursive copy (same layout as tuples)
        Value::Vector(addr) => {
            if let Some(dst) = visited.check(addr) {
                return Some(Value::vector(dst));
            }
            deep_copy_vector(addr, realm, mem, visited)
        }

        // Maps: recursive copy
        Value::Map(addr) => {
            if let Some(dst) = visited.check(addr) {
                return Some(Value::map(dst));
            }
            deep_copy_map(addr, realm, mem, visited)
        }

        // Compiled functions: deep copy bytecode and constants
        Value::CompiledFn(addr) => {
            if let Some(dst) = visited.check(addr) {
                return Some(Value::compiled_fn(dst));
            }
            deep_copy_compiled_fn(addr, realm, mem, visited)
        }

        // Closures: deep copy function and captures
        Value::Closure(addr) => {
            if let Some(dst) = visited.check(addr) {
                return Some(Value::closure(dst));
            }
            deep_copy_closure(addr, realm, mem, visited)
        }
    }
}

/// Deep copy a string to the realm's code region.
fn deep_copy_string<M: MemorySpace>(
    src_addr: Vaddr,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
    value_ctor: fn(Vaddr) -> Value,
) -> Option<Value> {
    // Read source header
    let header: HeapString = mem.read(src_addr);
    let len = header.len as usize;
    let total_size = HeapString::alloc_size(len);

    // Allocate in realm
    let dst_addr = realm.alloc(total_size, 4)?;

    // Record before recursing (not that strings recurse, but for consistency)
    visited.record(src_addr, dst_addr);

    // Copy header
    mem.write(dst_addr, header);

    // Copy string data byte by byte to avoid borrow conflict
    let src_data = src_addr.add(HeapString::HEADER_SIZE as u64);
    let dst_data = dst_addr.add(HeapString::HEADER_SIZE as u64);

    for i in 0..len {
        let byte: u8 = mem.read(src_data.add(i as u64));
        mem.write(dst_data.add(i as u64), byte);
    }

    Some(value_ctor(dst_addr))
}

/// Deep copy a pair to the realm's code region.
fn deep_copy_pair<M: MemorySpace>(
    src_addr: Vaddr,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Value> {
    // Read source pair
    let pair: Pair = mem.read(src_addr);

    // Allocate in realm first (record early to handle cycles)
    let dst_addr = realm.alloc(Pair::SIZE, 8)?;
    visited.record(src_addr, dst_addr);

    // Deep copy first and rest
    let dst_first = deep_copy_to_realm(pair.first, realm, mem, visited)?;
    let dst_rest = deep_copy_to_realm(pair.rest, realm, mem, visited)?;

    // Write destination pair
    let dst_pair = Pair {
        first: dst_first,
        rest: dst_rest,
    };
    mem.write(dst_addr, dst_pair);

    Some(Value::pair(dst_addr))
}

/// Deep copy a tuple to the realm's code region.
fn deep_copy_tuple<M: MemorySpace>(
    src_addr: Vaddr,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Value> {
    // Read source header
    let header: HeapTuple = mem.read(src_addr);
    let len = header.len as usize;
    let total_size = HeapTuple::alloc_size(len);

    // Allocate in realm first
    let dst_addr = realm.alloc(total_size, 8)?;
    visited.record(src_addr, dst_addr);

    // Write header
    mem.write(dst_addr, header);

    // Deep copy each element
    let src_elements = src_addr.add(HeapTuple::HEADER_SIZE as u64);
    let dst_elements = dst_addr.add(HeapTuple::HEADER_SIZE as u64);

    for i in 0..len {
        let offset = (i * core::mem::size_of::<Value>()) as u64;
        let src_elem: Value = mem.read(src_elements.add(offset));
        let dst_elem = deep_copy_to_realm(src_elem, realm, mem, visited)?;
        mem.write(dst_elements.add(offset), dst_elem);
    }

    Some(Value::tuple(dst_addr))
}

/// Deep copy a vector to the realm's code region (same layout as tuple).
fn deep_copy_vector<M: MemorySpace>(
    src_addr: Vaddr,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Value> {
    // Read source header (vectors share layout with tuples)
    let header: HeapTuple = mem.read(src_addr);
    let len = header.len as usize;
    let total_size = HeapTuple::alloc_size(len);

    // Allocate in realm first
    let dst_addr = realm.alloc(total_size, 8)?;
    visited.record(src_addr, dst_addr);

    // Write header
    mem.write(dst_addr, header);

    // Deep copy each element
    let src_elements = src_addr.add(HeapTuple::HEADER_SIZE as u64);
    let dst_elements = dst_addr.add(HeapTuple::HEADER_SIZE as u64);

    for i in 0..len {
        let offset = (i * core::mem::size_of::<Value>()) as u64;
        let src_elem: Value = mem.read(src_elements.add(offset));
        let dst_elem = deep_copy_to_realm(src_elem, realm, mem, visited)?;
        mem.write(dst_elements.add(offset), dst_elem);
    }

    Some(Value::vector(dst_addr))
}

/// Deep copy a map to the realm's code region.
fn deep_copy_map<M: MemorySpace>(
    src_addr: Vaddr,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Value> {
    // Read source map
    let map: HeapMap = mem.read(src_addr);

    // Allocate in realm first
    let dst_addr = realm.alloc(HeapMap::SIZE, 8)?;
    visited.record(src_addr, dst_addr);

    // Deep copy the entries list (association list of pairs)
    let dst_entries = deep_copy_to_realm(map.entries, realm, mem, visited)?;

    // Write destination map
    let dst_map = HeapMap {
        entries: dst_entries,
    };
    mem.write(dst_addr, dst_map);

    Some(Value::map(dst_addr))
}

/// Deep copy a compiled function to the realm's code region.
fn deep_copy_compiled_fn<M: MemorySpace>(
    src_addr: Vaddr,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Value> {
    // Read source header
    let header: HeapCompiledFn = mem.read(src_addr);
    let code_len = header.code_len as usize;
    let constants_len = header.constants_len as usize;
    let total_size = HeapCompiledFn::alloc_size(code_len, constants_len);

    // Allocate in realm first
    let dst_addr = realm.alloc(total_size, 8)?;
    visited.record(src_addr, dst_addr);

    // Copy header (will update source_file if needed)
    let mut dst_header = header;

    // Deep copy source_file string if present
    if !header.source_file.is_null() {
        if let Some(Value::String(dst_file)) =
            deep_copy_to_realm(Value::string(header.source_file), realm, mem, visited)
        {
            dst_header.source_file = dst_file;
        }
    }

    mem.write(dst_addr, dst_header);

    // Copy bytecode (no deep copy needed for u32 instructions)
    let src_code = src_addr.add(HeapCompiledFn::bytecode_offset() as u64);
    let dst_code = dst_addr.add(HeapCompiledFn::bytecode_offset() as u64);
    for i in 0..code_len {
        let offset = (i * core::mem::size_of::<u32>()) as u64;
        let instr: u32 = mem.read(src_code.add(offset));
        mem.write(dst_code.add(offset), instr);
    }

    // Deep copy constants
    let src_constants = src_addr.add(HeapCompiledFn::constants_offset(code_len) as u64);
    let dst_constants = dst_addr.add(HeapCompiledFn::constants_offset(code_len) as u64);
    for i in 0..constants_len {
        let offset = (i * core::mem::size_of::<Value>()) as u64;
        let src_const: Value = mem.read(src_constants.add(offset));
        let dst_const = deep_copy_to_realm(src_const, realm, mem, visited)?;
        mem.write(dst_constants.add(offset), dst_const);
    }

    Some(Value::compiled_fn(dst_addr))
}

/// Deep copy a closure to the realm's code region.
fn deep_copy_closure<M: MemorySpace>(
    src_addr: Vaddr,
    realm: &mut Realm,
    mem: &mut M,
    visited: &mut VisitedTracker,
) -> Option<Value> {
    // Read source header
    let header: HeapClosure = mem.read(src_addr);
    let captures_len = header.captures_len as usize;
    let total_size = HeapClosure::alloc_size(captures_len);

    // Allocate in realm first
    let dst_addr = realm.alloc(total_size, 8)?;
    visited.record(src_addr, dst_addr);

    // Deep copy the underlying function
    let dst_func = deep_copy_to_realm(Value::compiled_fn(header.function), realm, mem, visited)?;
    let Value::CompiledFn(dst_func_addr) = dst_func else {
        return None;
    };

    // Write header with new function pointer
    let dst_header = HeapClosure {
        function: dst_func_addr,
        captures_len: header.captures_len,
        padding: 0,
    };
    mem.write(dst_addr, dst_header);

    // Deep copy captures
    let src_captures = src_addr.add(HeapClosure::captures_offset() as u64);
    let dst_captures = dst_addr.add(HeapClosure::captures_offset() as u64);
    for i in 0..captures_len {
        let offset = (i * core::mem::size_of::<Value>()) as u64;
        let src_cap: Value = mem.read(src_captures.add(offset));
        let dst_cap = deep_copy_to_realm(src_cap, realm, mem, visited)?;
        mem.write(dst_captures.add(offset), dst_cap);
    }

    Some(Value::closure(dst_addr))
}
