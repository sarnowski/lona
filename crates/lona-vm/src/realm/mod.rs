// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Realm structure for shared state across processes.
//!
//! A realm represents the persistent state that's shared across all processes
//! within a protection domain. It owns:
//!
//! - A code region for persistent allocations (`VarSlot`s, `VarContent`s, functions)
//! - Interning tables for symbols and keywords
//! - A namespace registry mapping symbols to namespace addresses
//! - A metadata table mapping object addresses to metadata maps
//!
//! The realm's code region exists within the same `MemorySpace` as process heaps,
//! just at a different base address. All processes in a realm can read from the
//! code region, but only `def` operations write to it.

mod bootstrap;
mod copy;

#[cfg(test)]
mod bootstrap_test;
#[cfg(test)]
mod copy_test;
#[cfg(test)]
mod realm_test;

pub use bootstrap::{BootstrapResult, bootstrap, get_core_ns, get_ns_var, lookup_var_in_ns};
pub use copy::{VisitedTracker, deep_copy_to_realm};

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::value::{HeapMap, HeapString, HeapTuple, Namespace, Pair, Value, VarContent, VarSlot};

/// Maximum number of interned symbols per realm.
///
/// Limited to 2000 to keep the array under 16KB (2000 * 8 = 16000 bytes).
pub const MAX_INTERNED_SYMBOLS: usize = 2000;

/// Maximum number of interned keywords per realm.
pub const MAX_INTERNED_KEYWORDS: usize = 1024;

/// Maximum number of namespaces per realm.
pub const MAX_NAMESPACES: usize = 256;

/// Maximum number of metadata entries per realm.
///
/// Limited to 2000 to keep each array under 16KB (2000 * 8 = 16000 bytes).
pub const MAX_METADATA_ENTRIES: usize = 2000;

/// A realm represents shared state across processes.
///
/// The realm owns:
/// - A code region for persistent allocations (`VarSlot`s, `VarContent`s, functions)
/// - A namespace registry mapping symbols to namespace addresses
/// - A metadata table mapping object addresses to metadata maps
#[repr(C)]
pub struct Realm {
    // Code region (grows up from base)
    /// Base address of the code region.
    pub code_base: Vaddr,
    /// End address of the code region.
    pub code_end: Vaddr,
    /// Current allocation pointer (grows up).
    pub code_top: Vaddr,

    // Interning tables (realm-level, shared across processes)
    /// Interned symbols (addresses of symbol `HeapString`s in code region).
    pub symbol_intern: [Vaddr; MAX_INTERNED_SYMBOLS],
    /// Number of interned symbols.
    pub symbol_intern_len: usize,
    /// Interned keywords (addresses of keyword `HeapString`s in code region).
    pub keyword_intern: [Vaddr; MAX_INTERNED_KEYWORDS],
    /// Number of interned keywords.
    pub keyword_intern_len: usize,

    // Namespace registry
    /// Namespace name symbols (parallel array).
    pub namespace_names: [Vaddr; MAX_NAMESPACES],
    /// Namespace addresses (parallel array).
    pub namespace_addrs: [Vaddr; MAX_NAMESPACES],
    /// Number of registered namespaces.
    pub namespace_len: usize,

    // Metadata table
    /// Object addresses (parallel array).
    pub metadata_keys: [Vaddr; MAX_METADATA_ENTRIES],
    /// Metadata map addresses (parallel array).
    pub metadata_values: [Vaddr; MAX_METADATA_ENTRIES],
    /// Number of metadata entries.
    pub metadata_len: usize,
}

impl Realm {
    /// Create a new realm with the given code region.
    ///
    /// # Arguments
    /// * `code_base` - Base address of the code region
    /// * `code_size` - Size of the code region in bytes
    #[must_use]
    pub const fn new(code_base: Vaddr, code_size: usize) -> Self {
        let code_end = Vaddr::new(code_base.as_u64() + code_size as u64);

        Self {
            code_base,
            code_end,
            code_top: code_base,
            // Interning tables
            symbol_intern: [Vaddr::new(0); MAX_INTERNED_SYMBOLS],
            symbol_intern_len: 0,
            keyword_intern: [Vaddr::new(0); MAX_INTERNED_KEYWORDS],
            keyword_intern_len: 0,
            // Namespace registry
            namespace_names: [Vaddr::new(0); MAX_NAMESPACES],
            namespace_addrs: [Vaddr::new(0); MAX_NAMESPACES],
            namespace_len: 0,
            // Metadata table
            metadata_keys: [Vaddr::new(0); MAX_METADATA_ENTRIES],
            metadata_values: [Vaddr::new(0); MAX_METADATA_ENTRIES],
            metadata_len: 0,
        }
    }

    // --- Code Region Allocator ---

    /// Allocate bytes from the code region (append-only, grows up).
    ///
    /// Returns `None` if the code region is full.
    pub const fn alloc(&mut self, size: usize, align: usize) -> Option<Vaddr> {
        if size == 0 {
            return Some(self.code_top);
        }

        // Align code_top up
        let mask = (align as u64).wrapping_sub(1);
        let aligned = (self.code_top.as_u64() + mask) & !mask;
        let new_top = aligned + size as u64;

        // Check bounds
        if new_top > self.code_end.as_u64() {
            return None; // OOM
        }

        let result = Vaddr::new(aligned);
        self.code_top = Vaddr::new(new_top);
        Some(result)
    }

    /// Returns the number of bytes used in the code region.
    #[must_use]
    pub const fn code_used(&self) -> usize {
        self.code_top
            .as_u64()
            .saturating_sub(self.code_base.as_u64()) as usize
    }

    /// Returns the remaining free space in the code region.
    #[must_use]
    pub const fn code_free(&self) -> usize {
        self.code_end
            .as_u64()
            .saturating_sub(self.code_top.as_u64()) as usize
    }

    // --- Symbol Interning ---

    /// Intern a symbol in the realm's code region.
    ///
    /// If a symbol with the same name already exists, returns it.
    /// Otherwise, allocates a new symbol in the code region and interns it.
    ///
    /// Returns `None` if the code region is full or intern table is full.
    pub fn intern_symbol<M: MemorySpace>(&mut self, mem: &mut M, name: &str) -> Option<Value> {
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

        // Check intern table capacity
        if self.symbol_intern_len >= MAX_INTERNED_SYMBOLS {
            return None;
        }

        // Allocate new symbol in code region
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

        // Add to intern table
        self.symbol_intern[self.symbol_intern_len] = addr;
        self.symbol_intern_len += 1;

        Some(Value::symbol(addr))
    }

    /// Find an existing interned symbol by name (read-only lookup).
    ///
    /// Returns the symbol if found in the intern table, `None` otherwise.
    #[must_use]
    pub fn find_symbol<M: MemorySpace>(&self, mem: &M, name: &str) -> Option<Value> {
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

    // --- Keyword Interning ---

    /// Intern a keyword in the realm's code region.
    ///
    /// If a keyword with the same name already exists, returns it.
    /// Otherwise, allocates a new keyword in the code region and interns it.
    ///
    /// Returns `None` if the code region is full or intern table is full.
    pub fn intern_keyword<M: MemorySpace>(&mut self, mem: &mut M, name: &str) -> Option<Value> {
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

        // Check intern table capacity
        if self.keyword_intern_len >= MAX_INTERNED_KEYWORDS {
            return None;
        }

        // Allocate new keyword in code region
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

        // Add to intern table
        self.keyword_intern[self.keyword_intern_len] = addr;
        self.keyword_intern_len += 1;

        Some(Value::keyword(addr))
    }

    /// Find an existing interned keyword by name (read-only lookup).
    ///
    /// Returns the keyword if found in the intern table, `None` otherwise.
    #[must_use]
    pub fn find_keyword<M: MemorySpace>(&self, mem: &M, name: &str) -> Option<Value> {
        for i in 0..self.keyword_intern_len {
            let addr = self.keyword_intern[i];
            let header: HeapString = mem.read(addr);
            if header.len as usize == name.len() {
                let data_addr = addr.add(HeapString::HEADER_SIZE as u64);
                let bytes = mem.slice(data_addr, header.len as usize);
                if bytes == name.as_bytes() {
                    return Some(Value::keyword(addr));
                }
            }
        }
        None
    }

    // --- Namespace Registry ---

    /// Find a namespace by its name symbol.
    ///
    /// Compares namespace names by symbol address. Returns the namespace value
    /// if found, `None` otherwise.
    #[must_use]
    pub fn find_namespace(&self, name: Value) -> Option<Value> {
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
    ///
    /// Returns `None` if the registry is full.
    pub fn register_namespace(&mut self, name_addr: Vaddr, ns_addr: Vaddr) -> Option<()> {
        // Check if already exists - update in place
        for i in 0..self.namespace_len {
            if self.namespace_names[i] == name_addr {
                self.namespace_addrs[i] = ns_addr;
                return Some(());
            }
        }

        // Add new entry if table not full
        if self.namespace_len >= MAX_NAMESPACES {
            return None;
        }

        self.namespace_names[self.namespace_len] = name_addr;
        self.namespace_addrs[self.namespace_len] = ns_addr;
        self.namespace_len += 1;
        Some(())
    }

    /// Allocate a namespace in the code region.
    ///
    /// Creates a namespace with the given name symbol and an empty mappings map.
    ///
    /// Returns a `Value::Namespace` pointing to the allocated namespace, or `None` if OOM.
    pub fn alloc_namespace<M: MemorySpace>(&mut self, mem: &mut M, name: Value) -> Option<Value> {
        // Create empty mappings map in code region
        let map_addr = self.alloc(HeapMap::SIZE, 8)?;
        let empty_map = HeapMap {
            entries: Value::Nil,
        };
        mem.write(map_addr, empty_map);
        let mappings = Value::map(map_addr);

        // Allocate namespace in code region
        let ns_addr = self.alloc(Namespace::SIZE, 8)?;
        let ns = Namespace { name, mappings };
        mem.write(ns_addr, ns);

        Some(Value::namespace(ns_addr))
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
        if let Some(ns) = self.find_namespace(name) {
            return Some(ns);
        }

        // Create new namespace
        let ns = self.alloc_namespace(mem, name)?;

        // Register it
        if let (Value::Symbol(name_addr), Value::Namespace(ns_addr)) = (name, ns) {
            self.register_namespace(name_addr, ns_addr)?;
        }

        Some(ns)
    }

    // --- Var Allocation ---

    /// Allocate a var (`VarSlot` + `VarContent`) in the code region.
    ///
    /// Creates a new var with the given name symbol, namespace, root value, and flags.
    /// Returns a `Value::Var` pointing to the allocated `VarSlot`, or `None` if OOM.
    pub fn alloc_var<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        name: Vaddr,
        namespace: Vaddr,
        root: Value,
        flags: u32,
    ) -> Option<Value> {
        // Allocate VarContent first
        let content_addr = self.alloc(VarContent::SIZE, 8)?;
        let content = VarContent {
            name,
            namespace,
            root,
            flags,
            padding: 0,
        };
        mem.write(content_addr, content);

        // Allocate VarSlot
        let slot_addr = self.alloc(VarSlot::SIZE, 8)?;
        let slot = VarSlot {
            content: content_addr,
        };
        mem.write(slot_addr, slot);

        Some(Value::var(slot_addr))
    }

    /// Update a var's root value atomically (MVCC pattern).
    ///
    /// Creates a new `VarContent` with the updated value and atomically swaps
    /// the `VarSlot`'s content pointer using Release ordering.
    ///
    /// Returns the var, or `None` if the value is not a var or OOM.
    pub fn var_set_root<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        var: Value,
        new_root: Value,
    ) -> Option<Value> {
        let Value::Var(slot_addr) = var else {
            return None;
        };

        // Read current content with Acquire ordering
        let content_raw = mem.read_u64_acquire(slot_addr);
        let old_content: VarContent = mem.read(Vaddr::new(content_raw));

        // Allocate new VarContent with updated root in code region
        let new_content_addr = self.alloc(VarContent::SIZE, 8)?;
        let new_content = VarContent {
            name: old_content.name,
            namespace: old_content.namespace,
            root: new_root,
            flags: old_content.flags,
            padding: 0,
        };
        mem.write(new_content_addr, new_content);

        // Atomically update VarSlot to point to new content with Release ordering
        mem.write_u64_release(slot_addr, new_content_addr.as_u64());

        Some(var)
    }

    /// Add a symbol→var mapping to a namespace.
    ///
    /// Prepends the new mapping to the namespace's association list.
    /// This is used during bootstrap to register vars in `lona.core`.
    ///
    /// Returns `None` if allocation fails.
    pub fn add_ns_mapping<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        ns: Value,
        name: Value,
        var: Value,
    ) -> Option<()> {
        let Value::Namespace(ns_addr) = ns else {
            return None;
        };

        let ns_struct: Namespace = mem.read(ns_addr);
        let Value::Map(map_addr) = ns_struct.mappings else {
            return None;
        };
        let map: HeapMap = mem.read(map_addr);

        // Create [name var] tuple in realm
        let tuple_size = HeapTuple::alloc_size(2);
        let tuple_addr = self.alloc(tuple_size, 8)?;

        let tuple_header = HeapTuple { len: 2, padding: 0 };
        mem.write(tuple_addr, tuple_header);

        let elem0_addr = tuple_addr.add(HeapTuple::HEADER_SIZE as u64);
        let elem1_addr = elem0_addr.add(core::mem::size_of::<Value>() as u64);
        mem.write(elem0_addr, name);
        mem.write(elem1_addr, var);

        let kv_tuple = Value::tuple(tuple_addr);

        // Create new pair: (kv_tuple . old_entries)
        let pair_addr = self.alloc(Pair::SIZE, 8)?;
        let new_pair = Pair {
            first: kv_tuple,
            rest: map.entries,
        };
        mem.write(pair_addr, new_pair);

        // Update map's entries
        let new_map = HeapMap {
            entries: Value::pair(pair_addr),
        };
        mem.write(map_addr, new_map);

        Some(())
    }

    // --- Metadata Table ---

    /// Set metadata for an object.
    ///
    /// Associates the given metadata map address with the object address.
    /// If the object already has metadata, it is replaced.
    ///
    /// Returns `None` if the metadata table is full.
    pub fn set_metadata(&mut self, obj_addr: Vaddr, meta_addr: Vaddr) -> Option<()> {
        // Check if already exists - update in place
        for i in 0..self.metadata_len {
            if self.metadata_keys[i] == obj_addr {
                self.metadata_values[i] = meta_addr;
                return Some(());
            }
        }

        // Add new entry if table not full
        if self.metadata_len >= MAX_METADATA_ENTRIES {
            return None;
        }

        self.metadata_keys[self.metadata_len] = obj_addr;
        self.metadata_values[self.metadata_len] = meta_addr;
        self.metadata_len += 1;
        Some(())
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
