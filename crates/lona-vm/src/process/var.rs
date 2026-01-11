// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Var allocation and lookup methods for Process.
//!
//! This module provides methods for creating and manipulating vars within namespaces.

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::value::{HeapMap, HeapString, HeapTuple, Namespace, Value, VarContent, VarSlot};

use super::Process;

impl Process {
    /// Allocate a var (`VarSlot` + `VarContent`) on the process heap.
    ///
    /// Creates a new var with the given name symbol, namespace, root value, and flags.
    /// Returns a `Value::Var` pointing to the allocated `VarSlot`, or `None` if OOM.
    ///
    /// Note: This allocates on the process heap for temporary/REPL use.
    /// For persistent vars (via `def`), use `Realm::alloc_var` instead.
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

    /// Read a `VarSlot` from the heap with atomic Acquire semantics.
    ///
    /// Uses Acquire ordering to ensure we see all writes that happened
    /// before the corresponding Release store to the content pointer.
    ///
    /// Returns `None` if the value is not a var.
    #[must_use]
    pub fn read_var_slot<M: MemorySpace>(&self, mem: &M, value: Value) -> Option<VarSlot> {
        let Value::Var(addr) = value else {
            return None;
        };

        // Read content pointer atomically with Acquire ordering
        let content_raw = mem.read_u64_acquire(addr);
        Some(VarSlot {
            content: Vaddr::new(content_raw),
        })
    }

    /// Read a `VarContent` from the heap via a `VarSlot`.
    ///
    /// Returns `None` if the value is not a var.
    #[must_use]
    pub fn read_var_content<M: MemorySpace>(&self, mem: &M, value: Value) -> Option<VarContent> {
        let slot = self.read_var_slot(mem, value)?;
        Some(mem.read(slot.content))
    }

    /// Get the current value of a var (dereferencing it).
    ///
    /// For process-bound vars, checks process bindings first. If a process
    /// binding exists, returns that value. Otherwise, returns the root value.
    ///
    /// Returns `Value::Unbound` if the var has no value (declared but not initialized)
    /// and no process binding exists.
    #[must_use]
    pub fn var_get<M: MemorySpace>(&self, mem: &M, var: Value) -> Option<Value> {
        let Value::Var(slot_addr) = var else {
            return None;
        };

        let content = self.read_var_content(mem, var)?;

        // Check process bindings first for process-bound vars
        if content.is_process_bound() {
            if let Some(binding) = self.get_binding(slot_addr) {
                return Some(binding);
            }
        }

        Some(content.root)
    }

    /// Intern a symbol in a namespace, creating a new var if needed.
    ///
    /// If a var with the given name already exists in the namespace, updates its value.
    /// Otherwise, creates a new var and adds it to the namespace mappings.
    ///
    /// Returns the var, or `None` if OOM.
    pub fn intern_var<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        ns_value: Value,
        name_symbol: Value,
        root_value: Value,
    ) -> Option<Value> {
        let Value::Namespace(ns_addr) = ns_value else {
            return None;
        };
        let Value::Symbol(name_addr) = name_symbol else {
            return None;
        };

        // Read namespace
        let ns: Namespace = mem.read(ns_addr);

        // Look for existing var in namespace mappings
        if let Some(existing_var) = self.ns_lookup_var(mem, ns.mappings, name_symbol) {
            // Var exists - update its root value
            self.var_set_root(mem, existing_var, root_value)?;
            return Some(existing_var);
        }

        // Create new var
        let var = self.alloc_var(mem, name_addr, ns_addr, root_value, 0)?;

        // Add to namespace mappings
        let Value::Var(var_addr) = var else {
            return None;
        };
        self.ns_add_mapping(mem, ns_addr, name_symbol, Value::var(var_addr))?;

        Some(var)
    }

    /// Update a var's root value atomically (MVCC pattern).
    ///
    /// Creates a new `VarContent` with the updated value and atomically swaps
    /// the `VarSlot`'s content pointer using Release ordering. This ensures
    /// that readers using Acquire ordering will see either the old or new
    /// content, never a partially-written state.
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

        // Allocate new VarContent with updated root
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
        // This ensures all writes to new_content are visible before the pointer is published
        mem.write_u64_release(slot_addr, new_content_addr.as_u64());

        Some(var)
    }

    /// Look up a var by name in namespace mappings.
    ///
    /// The mappings is a `Value::Map` where keys are symbols and values are vars.
    /// Returns the var if found, `None` otherwise.
    #[must_use]
    pub fn ns_lookup_var<M: MemorySpace>(
        &self,
        mem: &M,
        mappings: Value,
        name: Value,
    ) -> Option<Value> {
        let Value::Map(map_addr) = mappings else {
            return None;
        };

        let map: HeapMap = mem.read(map_addr);

        // Walk the association list
        let mut entries = map.entries;
        while let Value::Pair(pair_addr) = entries {
            let pair = mem.read::<crate::value::Pair>(pair_addr);

            // Each entry is a [key value] tuple
            if let Value::Tuple(tuple_addr) = pair.first {
                let header: HeapTuple = mem.read(tuple_addr);
                if header.len >= 2 {
                    let key_addr = tuple_addr.add(HeapTuple::HEADER_SIZE as u64);
                    let value_addr = key_addr.add(core::mem::size_of::<Value>() as u64);

                    let key: Value = mem.read(key_addr);
                    let value: Value = mem.read(value_addr);

                    // Compare symbol addresses (symbols are interned)
                    if let (Value::Symbol(k_addr), Value::Symbol(n_addr)) = (key, name) {
                        if k_addr == n_addr {
                            // Found matching key - return the var
                            return Some(value);
                        }
                    }

                    // Also compare by string content for non-interned symbols
                    if key.is_symbol() && name.is_symbol() && Self::symbols_equal(mem, key, name) {
                        return Some(value);
                    }
                }
            }

            entries = pair.rest;
        }

        None
    }

    /// Compare two symbols by their string content.
    fn symbols_equal<M: MemorySpace>(mem: &M, a: Value, b: Value) -> bool {
        let (Value::Symbol(a_addr), Value::Symbol(b_addr)) = (a, b) else {
            return false;
        };

        // Fast path: same address means same symbol
        if a_addr == b_addr {
            return true;
        }

        // Compare by content
        let a_header: HeapString = mem.read(a_addr);
        let b_header: HeapString = mem.read(b_addr);

        if a_header.len != b_header.len {
            return false;
        }

        let a_data = a_addr.add(HeapString::HEADER_SIZE as u64);
        let b_data = b_addr.add(HeapString::HEADER_SIZE as u64);

        let a_bytes = mem.slice(a_data, a_header.len as usize);
        let b_bytes = mem.slice(b_data, b_header.len as usize);

        a_bytes == b_bytes
    }

    /// Add a symbol→var mapping to a namespace.
    ///
    /// Prepends the new mapping to the namespace's association list.
    fn ns_add_mapping<M: MemorySpace>(
        &mut self,
        mem: &mut M,
        ns_addr: Vaddr,
        name: Value,
        var: Value,
    ) -> Option<()> {
        let ns: Namespace = mem.read(ns_addr);
        let Value::Map(map_addr) = ns.mappings else {
            return None;
        };
        let map: HeapMap = mem.read(map_addr);

        // Create [name var] tuple
        let kv_elements = [name, var];
        let kv_tuple = self.alloc_tuple(mem, &kv_elements)?;

        // Prepend to entries list
        let new_entries = self.alloc_pair(mem, kv_tuple, map.entries)?;

        // Update map's entries
        let new_map = HeapMap {
            entries: new_entries,
        };
        mem.write(map_addr, new_map);

        Some(())
    }
}
