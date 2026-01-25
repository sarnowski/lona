// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Namespace allocation methods for Process.
//!
//! For namespace registry operations (find, register, `get_or_create`),
//! use the Realm's methods instead. Process-level allocation creates
//! namespaces on the process heap (temporary), while Realm allocation
//! creates persistent namespaces in the code region.

use crate::platform::MemorySpace;
use crate::value::{Namespace, Value};

use super::Process;

impl Process {
    /// Allocate a namespace on the young heap.
    ///
    /// Creates a namespace with the given name symbol and an empty mappings map.
    /// This is for temporary namespaces on the process heap. For persistent
    /// namespaces, use `Realm::alloc_namespace` instead.
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
}
