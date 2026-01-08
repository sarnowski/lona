// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Process memory model for BEAM-style lightweight processes.
//!
//! Each process has its own heap (for allocations) and execution state.
//! The heap uses the BEAM two-heap architecture:
//!
//! ```text
//! YOUNG HEAP (single contiguous block):
//! ┌────────────────────────────────────────────────────────────────────┐
//! │                                                                    │
//! │   HEAP                             FREE                  STACK     │
//! │   (grows up)                      SPACE                 (grows     │
//! │                                                          down)     │
//! │                                                                    │
//! │   [cons][string]◄─htop                   stop─►[frame1][frame0]    │
//! │        ↑                                              ↓            │
//! │                                                                    │
//! └────────────────────────────────────────────────────────────────────┘
//! ▲                                                                    ▲
//! heap (low address)                                      hend (high address)
//!
//! Out of memory when: htop >= stop
//! For M2: Return error (no GC yet)
//!
//! OLD HEAP (separate block, for future GC):
//! ┌────────────────────────────────────────────────────────────────────┐
//! │   [promoted][promoted]                    │         FREE           │
//! │                                           │◄─ old_htop             │
//! └────────────────────────────────────────────────────────────────────┘
//! ▲                                                                    ▲
//! old_heap                                                        old_hend
//!
//! For M2: Allocated but empty (no promotion without GC)
//! ```

pub mod pool;

#[cfg(test)]
mod process_test;

use crate::Vaddr;
use crate::bytecode::Chunk;
use crate::platform::MemorySpace;
use crate::value::{HeapMap, HeapString, HeapTuple, Namespace, Pair, Value};

/// Number of X registers (temporaries).
pub const X_REG_COUNT: usize = 256;

/// Maximum number of interned keywords per process.
///
/// Keywords are interned so that identical keyword literals share the same address.
/// This enables O(1) equality comparison via address comparison.
///
/// Note: This is a temporary per-process table. Once realm-level interning is
/// implemented (Phase 4+), most keywords will be interned at the realm level,
/// and this table will only handle dynamically-constructed keywords.
pub const MAX_INTERNED_KEYWORDS: usize = 1024;

/// Maximum number of metadata entries per process.
///
/// Metadata is stored separately from objects to avoid inline overhead.
/// Most objects don't have metadata, so a separate table is more efficient.
pub const MAX_METADATA_ENTRIES: usize = 1024;

/// Maximum number of namespaces per process.
///
/// Namespaces are stored in a per-process registry. In the future, this will
/// move to the realm level for proper sharing across processes.
pub const MAX_NAMESPACES: usize = 256;

/// Initial young heap size (48 KB).
pub const INITIAL_YOUNG_HEAP_SIZE: usize = 48 * 1024;

/// Initial old heap size (12 KB).
pub const INITIAL_OLD_HEAP_SIZE: usize = 12 * 1024;

/// Process execution status.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessStatus {
    /// Process is ready to run.
    Ready = 0,
    /// Process is currently running.
    Running = 1,
    /// Process has completed execution.
    Completed = 2,
    /// Process encountered an error.
    Error = 3,
}

/// A lightweight process with BEAM-style memory layout.
///
/// Each process owns its heap and execution state. The VM operates on
/// a process reference rather than owning the state itself.
#[repr(C)]
pub struct Process {
    // Identity
    /// Process identifier.
    pub pid: u64,
    /// Current execution status.
    pub status: ProcessStatus,

    // Young heap (stack grows down, heap grows up)
    /// Base (low address) of the young heap.
    pub heap: Vaddr,
    /// End (high address) of the young heap.
    pub hend: Vaddr,
    /// Heap top pointer (grows UP toward hend).
    pub htop: Vaddr,
    /// Stack pointer (grows DOWN toward heap).
    pub stop: Vaddr,

    // Old heap (for future GC)
    /// Base of the old heap.
    pub old_heap: Vaddr,
    /// End of the old heap.
    pub old_hend: Vaddr,
    /// Old heap allocation pointer.
    pub old_htop: Vaddr,

    // Execution state
    /// Instruction pointer (index into bytecode).
    pub ip: usize,
    /// X registers (temporaries).
    pub x_regs: [Value; X_REG_COUNT],
    /// Current bytecode chunk being executed.
    pub chunk: Option<Chunk>,

    // Interning tables
    /// Interned keywords (addresses of keyword `HeapString`s on the heap).
    /// Keywords are interned so that identical keyword literals share the same address.
    keyword_intern: [Vaddr; MAX_INTERNED_KEYWORDS],
    /// Number of interned keywords.
    keyword_intern_len: usize,

    // Metadata table
    /// Metadata table: maps object addresses to metadata map addresses.
    /// Stored as parallel arrays: `metadata_keys[i]` → `metadata_values[i]`.
    metadata_keys: [Vaddr; MAX_METADATA_ENTRIES],
    metadata_values: [Vaddr; MAX_METADATA_ENTRIES],
    /// Number of metadata entries.
    metadata_len: usize,

    // Namespace registry
    /// Namespace registry: maps namespace name symbols to namespace addresses.
    /// Stored as parallel arrays: `namespace_names[i]` → `namespace_addrs[i]`.
    namespace_names: [Vaddr; MAX_NAMESPACES],
    namespace_addrs: [Vaddr; MAX_NAMESPACES],
    /// Number of registered namespaces.
    namespace_len: usize,
}

impl Process {
    /// Create a new process with the given heap regions.
    ///
    /// # Arguments
    /// * `pid` - Process identifier
    /// * `young_base` - Base address of young heap (low address)
    /// * `young_size` - Size of young heap in bytes
    /// * `old_base` - Base address of old heap
    /// * `old_size` - Size of old heap in bytes
    #[must_use]
    pub const fn new(
        pid: u64,
        young_base: Vaddr,
        young_size: usize,
        old_base: Vaddr,
        old_size: usize,
    ) -> Self {
        let young_end = Vaddr::new(young_base.as_u64() + young_size as u64);
        let old_end = Vaddr::new(old_base.as_u64() + old_size as u64);

        Self {
            pid,
            status: ProcessStatus::Ready,
            // Young heap: htop starts at base (grows up), stop starts at end (grows down)
            heap: young_base,
            hend: young_end,
            htop: young_base,
            stop: young_end,
            // Old heap: empty, htop at base
            old_heap: old_base,
            old_hend: old_end,
            old_htop: old_base,
            // Execution state
            ip: 0,
            x_regs: [Value::Nil; X_REG_COUNT],
            chunk: None,
            // Interning tables
            keyword_intern: [Vaddr::new(0); MAX_INTERNED_KEYWORDS],
            keyword_intern_len: 0,
            // Metadata table
            metadata_keys: [Vaddr::new(0); MAX_METADATA_ENTRIES],
            metadata_values: [Vaddr::new(0); MAX_METADATA_ENTRIES],
            metadata_len: 0,
            // Namespace registry
            namespace_names: [Vaddr::new(0); MAX_NAMESPACES],
            namespace_addrs: [Vaddr::new(0); MAX_NAMESPACES],
            namespace_len: 0,
        }
    }

    /// Allocate bytes from the young heap (grows up).
    ///
    /// Returns `None` if there isn't enough space.
    pub const fn alloc(&mut self, size: usize, align: usize) -> Option<Vaddr> {
        if size == 0 {
            return Some(self.htop);
        }

        // Align htop up
        let mask = (align as u64).wrapping_sub(1);
        let aligned = (self.htop.as_u64() + mask) & !mask;
        let new_htop = aligned + size as u64;

        // Check collision with stack
        if new_htop > self.stop.as_u64() {
            return None; // OOM - in future, trigger GC
        }

        let result = Vaddr::new(aligned);
        self.htop = Vaddr::new(new_htop);
        Some(result)
    }

    /// Push bytes onto the stack (grows down).
    ///
    /// Returns `None` if there isn't enough space.
    pub const fn stack_push(&mut self, size: usize, align: usize) -> Option<Vaddr> {
        // Align stop down
        let mask = (align as u64).wrapping_sub(1);
        let new_stop = (self.stop.as_u64() - size as u64) & !mask;

        // Check collision with heap
        if new_stop < self.htop.as_u64() {
            return None; // OOM
        }

        self.stop = Vaddr::new(new_stop);
        Some(self.stop)
    }

    /// Pop bytes from the stack (grows down).
    pub fn stack_pop(&mut self, size: usize) {
        let new_stop = self.stop.as_u64() + size as u64;
        // Don't grow past hend
        self.stop = Vaddr::new(new_stop.min(self.hend.as_u64()));
    }

    /// Returns remaining free space (between htop and stop).
    #[must_use]
    pub const fn free_space(&self) -> usize {
        self.stop.as_u64().saturating_sub(self.htop.as_u64()) as usize
    }

    /// Returns the number of bytes used in the young heap.
    #[must_use]
    pub const fn heap_used(&self) -> usize {
        self.htop.as_u64().saturating_sub(self.heap.as_u64()) as usize
    }

    /// Returns the number of bytes used in the stack.
    #[must_use]
    pub const fn stack_used(&self) -> usize {
        self.hend.as_u64().saturating_sub(self.stop.as_u64()) as usize
    }

    /// Set the bytecode chunk to execute.
    pub fn set_chunk(&mut self, chunk: Chunk) {
        self.chunk = Some(chunk);
        self.ip = 0;
    }

    /// Reset execution state for a new evaluation.
    pub const fn reset(&mut self) {
        self.ip = 0;
        self.x_regs = [Value::Nil; X_REG_COUNT];
        self.status = ProcessStatus::Ready;
    }

    // --- Value allocation helpers ---

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

    // --- Namespace methods ---

    /// Allocate a namespace on the young heap.
    ///
    /// Creates a namespace with the given name symbol and an empty mappings map.
    ///
    /// Returns a `Value::Namespace` pointing to the allocated namespace, or `None` if OOM.
    pub fn alloc_namespace<M: MemorySpace>(&mut self, mem: &mut M, name: Value) -> Option<Value> {
        // Create empty mappings map
        let empty_entries = Value::Nil;
        let mappings = self.alloc_map(mem, empty_entries)?;

        // Allocate space (align to 8 bytes for Value fields)
        let addr = self.alloc(Namespace::SIZE, 8)?;

        // Write the namespace
        let ns = Namespace { name, mappings };
        mem.write(addr, ns);

        Some(Value::namespace(addr))
    }

    /// Read a namespace from the heap.
    ///
    /// Returns `None` if the value is not a namespace.
    #[must_use]
    pub fn read_namespace<M: MemorySpace>(&self, mem: &M, value: Value) -> Option<Namespace> {
        let Value::Namespace(addr) = value else {
            return None;
        };

        Some(mem.read(addr))
    }

    /// Find a namespace by its name symbol.
    ///
    /// Compares namespace names by symbol address. Returns the namespace value
    /// if found, `None` otherwise.
    #[must_use]
    pub fn find_namespace<M: MemorySpace>(&self, _mem: &M, name: Value) -> Option<Value> {
        let Value::Symbol(name_addr) = name else {
            return None;
        };

        for i in 0..self.namespace_len {
            if self.namespace_names[i] == name_addr {
                return Some(Value::namespace(self.namespace_addrs[i]));
            }
        }
        None
    }

    /// Register a namespace in the registry.
    ///
    /// Associates the namespace with its name for later lookup via `find_namespace`.
    /// If the namespace is already registered, updates the address.
    /// If the registry is full, this is a silent no-op.
    pub fn register_namespace(&mut self, name_addr: Vaddr, ns_addr: Vaddr) {
        // Check if already exists - update in place
        for i in 0..self.namespace_len {
            if self.namespace_names[i] == name_addr {
                self.namespace_addrs[i] = ns_addr;
                return;
            }
        }

        // Add new entry if table not full
        if self.namespace_len < MAX_NAMESPACES {
            self.namespace_names[self.namespace_len] = name_addr;
            self.namespace_addrs[self.namespace_len] = ns_addr;
            self.namespace_len += 1;
        }
    }

    /// Create or find a namespace by name.
    ///
    /// If a namespace with the given name exists, returns it.
    /// Otherwise, creates a new namespace, registers it, and returns it.
    pub fn get_or_create_namespace<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        name: Value,
    ) -> Option<Value> {
        // Check if namespace already exists
        if let Some(ns) = self.find_namespace(mem, name) {
            return Some(ns);
        }

        // Create new namespace
        let ns = self.alloc_namespace(mem, name)?;

        // Register it
        if let (Value::Symbol(name_addr), Value::Namespace(ns_addr)) = (name, ns) {
            self.register_namespace(name_addr, ns_addr);
        }

        Some(ns)
    }
}
