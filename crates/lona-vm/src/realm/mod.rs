// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Realm structure for shared state across processes.
//!
//! A realm represents the persistent state that's shared across all processes
//! within a protection domain. It owns:
//!
//! - A code region for persistent allocations (vars, functions)
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
pub use copy::{VisitedTracker, deep_copy_term_to_realm};

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::process::pool::ProcessPool;
use crate::term::Term;
use crate::term::header::Header;
use crate::term::heap::{HeapKeyword, HeapNamespace, HeapPair, HeapSymbol, HeapTuple, HeapVar};
use crate::term::tag::object;

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
/// - A memory pool for process heap allocations
/// - A code region for persistent allocations (vars, functions)
/// - A namespace registry mapping symbols to namespace addresses
/// - A metadata table mapping object addresses to metadata maps
#[repr(C)]
pub struct Realm {
    // Memory pool for process allocations (and heap growth during GC)
    /// The memory pool for allocating process heaps.
    pool: ProcessPool,

    // Code region (grows up from base)
    /// Base address of the code region.
    pub code_base: Vaddr,
    /// End address of the code region.
    pub code_end: Vaddr,
    /// Current allocation pointer (grows up).
    pub code_top: Vaddr,

    // Interning tables (realm-level, shared across processes)
    /// Interned symbols (addresses of symbol `HeapSymbol`s in code region).
    pub symbol_intern: [Vaddr; MAX_INTERNED_SYMBOLS],
    /// Number of interned symbols.
    pub symbol_intern_len: usize,
    /// Interned keywords (addresses of keyword `HeapKeyword`s in code region).
    pub keyword_intern: [Vaddr; MAX_INTERNED_KEYWORDS],
    /// Number of interned keywords.
    pub keyword_intern_len: usize,

    // Namespace registry
    /// Namespace name symbols as immediate Terms (parallel array).
    pub namespace_names: [Term; MAX_NAMESPACES],
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
    /// Create a new realm with a pre-allocated code region.
    ///
    /// The caller must allocate the code region from the pool before calling
    /// this constructor. This separation keeps construction infallible, enabling
    /// NRVO when used with `Box::new(Realm::new(...))` and avoiding ~180KB of
    /// stack usage from `Option<Realm>` in debug builds.
    ///
    /// # Arguments
    /// * `pool` - The memory pool for process allocations (after code region allocation)
    /// * `code_base` - Base address of the pre-allocated code region
    /// * `code_size` - Size of the code region in bytes
    #[must_use]
    pub const fn new(pool: ProcessPool, code_base: Vaddr, code_size: usize) -> Self {
        let code_end = Vaddr::new(code_base.as_u64() + code_size as u64);

        Self {
            pool,
            code_base,
            code_end,
            code_top: code_base,
            // Interning tables
            symbol_intern: [Vaddr::new(0); MAX_INTERNED_SYMBOLS],
            symbol_intern_len: 0,
            keyword_intern: [Vaddr::new(0); MAX_INTERNED_KEYWORDS],
            keyword_intern_len: 0,
            // Namespace registry
            namespace_names: [Term::NIL; MAX_NAMESPACES],
            namespace_addrs: [Vaddr::new(0); MAX_NAMESPACES],
            namespace_len: 0,
            // Metadata table
            metadata_keys: [Vaddr::new(0); MAX_METADATA_ENTRIES],
            metadata_values: [Vaddr::new(0); MAX_METADATA_ENTRIES],
            metadata_len: 0,
        }
    }

    /// Get a mutable reference to the memory pool.
    ///
    /// Used by GC to allocate new heap regions during growth.
    #[must_use]
    pub const fn pool_mut(&mut self) -> &mut ProcessPool {
        &mut self.pool
    }

    /// Allocate memory for a process's heaps.
    ///
    /// Returns `(young_base, old_base)` or `None` if insufficient space.
    pub fn allocate_process_memory(
        &mut self,
        young_size: usize,
        old_size: usize,
    ) -> Option<(Vaddr, Vaddr)> {
        self.pool
            .allocate_process_memory_with_growth(young_size, old_size)
    }

    // --- Code Region Allocator ---

    /// Allocate bytes from the code region (append-only, grows up).
    ///
    /// Returns `None` if the code region is full.
    ///
    /// Callers must request appropriate alignment for the type being allocated.
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
    /// If a symbol with the same name already exists, returns the existing
    /// immediate Term. Otherwise, allocates string storage in the code region
    /// and returns a new immediate Term with the index.
    ///
    /// Returns an immediate `Term::symbol(index)` where index is the position
    /// in the intern table. The string content is stored separately for lookup.
    ///
    /// Returns `None` if the code region is full or intern table is full.
    pub fn intern_symbol<M: MemorySpace>(&mut self, mem: &mut M, name: &str) -> Option<Term> {
        // Check intern table for existing symbol
        for i in 0..self.symbol_intern_len {
            let addr = self.symbol_intern[i];
            let header: Header = mem.read(addr);
            let len = header.arity() as usize;
            if len == name.len() {
                let data_addr = addr.add(HeapSymbol::HEADER_SIZE as u64);
                let bytes = mem.slice(data_addr, len);
                if bytes_eq(bytes, name.as_bytes()) {
                    // Found existing interned symbol - return immediate with index
                    return Some(Term::symbol(i as u32));
                }
            }
        }

        // Check intern table capacity
        if self.symbol_intern_len >= MAX_INTERNED_SYMBOLS {
            return None;
        }

        // Allocate string storage in code region
        let len = name.len();
        let total_size = HeapSymbol::alloc_size(len);

        // Allocate space (align to 8 bytes for the header)
        let addr = self.alloc(total_size, 8)?;

        // Write header with proper SYMBOL object tag (for string storage)
        let header = HeapSymbol::make_header(len);
        mem.write(addr, header);

        // Write string data
        let data_addr = addr.add(HeapSymbol::HEADER_SIZE as u64);
        let dest = mem.slice_mut(data_addr, len);
        dest.copy_from_slice(name.as_bytes());

        // Add to intern table and return immediate Term with the new index
        let index = self.symbol_intern_len;
        self.symbol_intern[index] = addr;
        self.symbol_intern_len += 1;

        Some(Term::symbol(index as u32))
    }

    /// Find an existing interned symbol by name (read-only lookup).
    ///
    /// Returns the immediate symbol Term if found, `None` otherwise.
    #[must_use]
    pub fn find_symbol<M: MemorySpace>(&self, mem: &M, name: &str) -> Option<Term> {
        for i in 0..self.symbol_intern_len {
            let addr = self.symbol_intern[i];
            let header: Header = mem.read(addr);
            let len = header.arity() as usize;
            if len == name.len() {
                let data_addr = addr.add(HeapSymbol::HEADER_SIZE as u64);
                let bytes = mem.slice(data_addr, len);
                if bytes_eq(bytes, name.as_bytes()) {
                    return Some(Term::symbol(i as u32));
                }
            }
        }
        None
    }

    /// Get the string name of an interned symbol by index.
    ///
    /// Returns the string slice if the index is valid, `None` otherwise.
    #[must_use]
    pub fn symbol_name<'a, M: MemorySpace>(&self, mem: &'a M, index: u32) -> Option<&'a str> {
        let i = index as usize;
        if i >= self.symbol_intern_len {
            return None;
        }
        let addr = self.symbol_intern[i];
        let header: Header = mem.read(addr);
        let len = header.arity() as usize;
        let data_addr = addr.add(HeapSymbol::HEADER_SIZE as u64);
        let bytes = mem.slice(data_addr, len);
        core::str::from_utf8(bytes).ok()
    }

    // --- Keyword Interning ---

    /// Intern a keyword in the realm's code region.
    ///
    /// If a keyword with the same name already exists, returns the existing
    /// immediate Term. Otherwise, allocates string storage in the code region
    /// and returns a new immediate Term with the index.
    ///
    /// Returns an immediate `Term::keyword(index)` where index is the position
    /// in the intern table. The string content is stored separately for lookup.
    ///
    /// Returns `None` if the code region is full or intern table is full.
    pub fn intern_keyword<M: MemorySpace>(&mut self, mem: &mut M, name: &str) -> Option<Term> {
        // Check intern table for existing keyword
        for i in 0..self.keyword_intern_len {
            let addr = self.keyword_intern[i];
            let header: Header = mem.read(addr);
            let len = header.arity() as usize;
            if len == name.len() {
                let data_addr = addr.add(HeapKeyword::HEADER_SIZE as u64);
                let bytes = mem.slice(data_addr, len);
                if bytes_eq(bytes, name.as_bytes()) {
                    // Found existing interned keyword - return immediate with index
                    return Some(Term::keyword(i as u32));
                }
            }
        }

        // Check intern table capacity
        if self.keyword_intern_len >= MAX_INTERNED_KEYWORDS {
            return None;
        }

        // Allocate string storage in code region
        let len = name.len();
        let total_size = HeapKeyword::alloc_size(len);

        // Allocate space (align to 8 bytes for the header)
        let addr = self.alloc(total_size, 8)?;

        // Write header with proper KEYWORD object tag (for string storage)
        let header = HeapKeyword::make_header(len);
        mem.write(addr, header);

        // Write string data
        let data_addr = addr.add(HeapKeyword::HEADER_SIZE as u64);
        let dest = mem.slice_mut(data_addr, len);
        dest.copy_from_slice(name.as_bytes());

        // Add to intern table and return immediate Term with the new index
        let index = self.keyword_intern_len;
        self.keyword_intern[index] = addr;
        self.keyword_intern_len += 1;

        Some(Term::keyword(index as u32))
    }

    /// Find an existing interned keyword by name (read-only lookup).
    ///
    /// Returns the immediate keyword Term if found, `None` otherwise.
    #[must_use]
    pub fn find_keyword<M: MemorySpace>(&self, mem: &M, name: &str) -> Option<Term> {
        for i in 0..self.keyword_intern_len {
            let addr = self.keyword_intern[i];
            let header: Header = mem.read(addr);
            let len = header.arity() as usize;
            if len == name.len() {
                let data_addr = addr.add(HeapKeyword::HEADER_SIZE as u64);
                let bytes = mem.slice(data_addr, len);
                if bytes_eq(bytes, name.as_bytes()) {
                    return Some(Term::keyword(i as u32));
                }
            }
        }
        None
    }

    /// Get the string name of an interned keyword by index.
    ///
    /// Returns the string slice if the index is valid, `None` otherwise.
    #[must_use]
    pub fn keyword_name<'a, M: MemorySpace>(&self, mem: &'a M, index: u32) -> Option<&'a str> {
        let i = index as usize;
        if i >= self.keyword_intern_len {
            return None;
        }
        let addr = self.keyword_intern[i];
        let header: Header = mem.read(addr);
        let len = header.arity() as usize;
        let data_addr = addr.add(HeapKeyword::HEADER_SIZE as u64);
        let bytes = mem.slice(data_addr, len);
        core::str::from_utf8(bytes).ok()
    }

    // --- Namespace Registry ---

    /// Find a namespace by its name symbol.
    ///
    /// Compares namespace names by immediate symbol Term equality.
    #[must_use]
    pub fn find_namespace(&self, name: Term) -> Option<Term> {
        // Symbols are now immediate values - simple Term equality check
        if !name.is_symbol() {
            return None;
        }

        for i in 0..self.namespace_len {
            if self.namespace_names[i] == name {
                return Some(Term::boxed_vaddr(self.namespace_addrs[i]));
            }
        }
        None
    }

    /// Register a namespace in the registry.
    ///
    /// Associates the namespace with its name (immediate symbol Term) for lookup.
    /// If the namespace is already registered, updates the address.
    ///
    /// Returns `None` if the registry is full.
    pub fn register_namespace(&mut self, name: Term, ns_addr: Vaddr) -> Option<()> {
        // Check if already exists - update in place
        for i in 0..self.namespace_len {
            if self.namespace_names[i] == name {
                self.namespace_addrs[i] = ns_addr;
                return Some(());
            }
        }

        // Add new entry if table not full
        if self.namespace_len >= MAX_NAMESPACES {
            return None;
        }

        self.namespace_names[self.namespace_len] = name;
        self.namespace_addrs[self.namespace_len] = ns_addr;
        self.namespace_len += 1;
        Some(())
    }

    /// Allocate a namespace in the code region.
    ///
    /// Creates a namespace with the given name symbol and empty mappings.
    ///
    /// Returns a boxed Term pointing to the allocated namespace, or `None` if OOM.
    pub fn alloc_namespace<M: MemorySpace>(&mut self, mem: &mut M, name: Term) -> Option<Term> {
        // Allocate namespace in code region
        let ns_addr = self.alloc(HeapNamespace::SIZE, 8)?;
        let ns = HeapNamespace {
            header: HeapNamespace::make_header(),
            name,
            mappings: Term::NIL, // Empty mappings list
        };
        mem.write(ns_addr, ns);

        Some(Term::boxed_vaddr(ns_addr))
    }

    /// Create or find a namespace by name.
    ///
    /// If a namespace with the given name exists, returns it.
    /// Otherwise, creates a new namespace, registers it, and returns it.
    ///
    /// The `name` parameter should be an immediate symbol Term.
    pub fn get_or_create_namespace<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        name: Term,
    ) -> Option<Term> {
        // Check if namespace already exists
        if let Some(ns) = self.find_namespace(name) {
            return Some(ns);
        }

        // Create new namespace
        let ns = self.alloc_namespace(mem, name)?;

        // Register it (name is an immediate symbol Term)
        let ns_addr = ns.to_vaddr();
        self.register_namespace(name, ns_addr)?;

        Some(ns)
    }

    // --- Var Allocation ---

    /// Allocate a var in the code region.
    ///
    /// Creates a new var with the given name symbol, namespace, and root value.
    /// Returns a boxed Term pointing to the allocated var, or `None` if OOM.
    pub fn alloc_var<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        name: Term,
        namespace: Term,
        root: Term,
    ) -> Option<Term> {
        // Allocate var in code region
        let var_addr = self.alloc(HeapVar::SIZE, 8)?;
        let var = HeapVar {
            header: HeapVar::make_header(),
            name,
            namespace,
            root,
        };
        mem.write(var_addr, var);

        Some(Term::boxed_vaddr(var_addr))
    }

    /// Update a var's root value.
    ///
    /// Updates the var's root binding in place.
    ///
    /// Returns `Some(())` on success, `None` if the value is not a var.
    pub fn var_set_root<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        var: Term,
        new_root: Term,
    ) -> Option<()> {
        if !var.is_boxed() {
            return None;
        }

        let var_addr = var.to_vaddr();
        let header: Header = mem.read(var_addr);
        if header.object_tag() != object::VAR {
            return None;
        }

        // Read current var
        let mut var_struct: HeapVar = mem.read(var_addr);

        // Update root
        var_struct.root = new_root;

        // Write back
        mem.write(var_addr, var_struct);

        Some(())
    }

    /// Add a symbol→var mapping to a namespace.
    ///
    /// Prepends the new mapping to the namespace's mappings list.
    ///
    /// Returns `None` if allocation fails.
    pub fn add_ns_mapping<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        ns: Term,
        name: Term,
        var: Term,
    ) -> Option<()> {
        if !ns.is_boxed() {
            return None;
        }

        let ns_addr = ns.to_vaddr();
        let mut ns_struct: HeapNamespace = mem.read(ns_addr);

        // Create [name var] tuple in realm
        let tuple_size = HeapTuple::alloc_size(2);
        let tuple_addr = self.alloc(tuple_size, 8)?;

        let tuple_header = HeapTuple::make_header(2);
        mem.write(tuple_addr, tuple_header);

        let elem0_addr = tuple_addr.add(HeapTuple::HEADER_SIZE as u64);
        let elem1_addr = elem0_addr.add(core::mem::size_of::<Term>() as u64);
        mem.write(elem0_addr, name);
        mem.write(elem1_addr, var);

        let kv_tuple = Term::boxed_vaddr(tuple_addr);

        // Create new pair: (kv_tuple . old_mappings)
        let pair_addr = self.alloc(HeapPair::SIZE, 8)?;
        let new_pair = HeapPair {
            head: kv_tuple,
            tail: ns_struct.mappings,
        };
        mem.write(pair_addr, new_pair);

        // Update namespace mappings
        ns_struct.mappings = Term::list_vaddr(pair_addr);
        mem.write(ns_addr, ns_struct);

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

    /// Get metadata for a Term.
    ///
    /// Returns the metadata Term (usually a map) if the value has metadata,
    /// `Term::NIL` otherwise.
    #[must_use]
    pub fn get_metadata_term(&self, term: Term) -> Term {
        // Only boxed values can have metadata
        if !term.is_boxed() && !term.is_list() {
            return Term::NIL;
        }

        let obj_addr = term.to_vaddr();
        self.get_metadata(obj_addr)
            .map_or(Term::NIL, Term::boxed_vaddr)
    }

    /// Read a string from a symbol/keyword/string Term.
    ///
    /// Returns the string slice if the term is a string-like type.
    pub fn read_string<'a, M: MemorySpace>(&self, mem: &'a M, term: Term) -> Option<&'a str> {
        if !term.is_boxed() {
            return None;
        }

        let addr = term.to_vaddr();
        let header: Header = mem.read(addr);
        let tag = header.object_tag();

        if tag != object::STRING && tag != object::SYMBOL && tag != object::KEYWORD {
            return None;
        }

        let len = header.arity() as usize;
        let data_addr = addr.add(HeapSymbol::HEADER_SIZE as u64);
        let bytes = mem.slice(data_addr, len);

        core::str::from_utf8(bytes).ok()
    }
}

/// Compare two byte slices byte-by-byte.
///
/// This avoids SIMD-optimized comparisons that may have alignment requirements
/// not met in the seL4 environment.
fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for i in 0..a.len() {
        if a[i] != b[i] {
            return false;
        }
    }
    true
}

// ============================================================================
// Test Helpers
// ============================================================================

#[cfg(test)]
impl Realm {
    /// Create a realm for testing with a mock memory pool.
    ///
    /// The pool is created from the provided base address with 256KB of space.
    /// This is enough for most tests.
    ///
    /// # Returns
    ///
    /// `Some(Realm)` if creation succeeded, `None` if the pool couldn't
    /// allocate the code region (should never happen with the default sizes).
    #[must_use]
    pub fn new_for_test(pool_base: Vaddr) -> Option<Self> {
        const TEST_POOL_SIZE: usize = 256 * 1024;
        const TEST_CODE_SIZE: usize = 64 * 1024;
        let mut pool = ProcessPool::new(pool_base, TEST_POOL_SIZE);
        let code_base = pool.allocate(TEST_CODE_SIZE, 8)?;
        Some(Self::new(pool, code_base, TEST_CODE_SIZE))
    }
}
