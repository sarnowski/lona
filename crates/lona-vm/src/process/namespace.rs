// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Namespace allocation and registry methods for Process.

use crate::Vaddr;
use crate::platform::MemorySpace;
use crate::value::{Namespace, Value};

use super::{MAX_NAMESPACES, Process};

impl Process {
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
