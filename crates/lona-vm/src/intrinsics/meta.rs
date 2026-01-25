// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Metadata and namespace intrinsics.

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::realm::{Realm, VisitedTracker, deep_copy_term_to_realm};
use crate::term::Term;
use crate::term::header::Header;
use crate::term::heap::{HeapMap, HeapPair, HeapTuple, HeapVar};
use crate::term::tag::object;

use super::{IntrinsicError, XRegs};

// --- Metadata intrinsics ---

/// Get metadata for an object.
///
/// `(meta obj)` - returns metadata map or nil
///
/// All metadata is stored in the realm's metadata table.
pub fn intrinsic_meta<M: MemorySpace>(x_regs: &XRegs, realm: &Realm, _mem: &M) -> Term {
    let obj = x_regs[1];
    realm.get_metadata_term(obj)
}

/// Attach metadata to an object.
///
/// `(with-meta obj m)` - returns obj with metadata attached
///
/// All metadata is stored in the realm's metadata table.
pub fn intrinsic_with_meta<M: MemorySpace>(
    x_regs: &XRegs,
    realm: &mut Realm,
    mem: &M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let obj = x_regs[1];
    let meta = x_regs[2];

    // Metadata must be a map (or nil to clear)
    if meta.is_nil() {
        // Clear metadata by not setting any - just return the object
        return Ok(obj);
    }

    // Must be a map
    if !obj.is_boxed() {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "reference type",
        });
    }

    // Check that meta is a map
    if !meta.is_boxed() {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 1,
            expected: "map",
        });
    }

    // Read the header to check it's a map
    let meta_addr = meta.to_vaddr();
    let header: Header = mem.read(meta_addr);
    if header.object_tag() != object::MAP {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 1,
            expected: "map",
        });
    }

    // Get the heap address of the object
    let obj_addr = obj.to_vaddr();

    // Store in realm metadata table
    realm
        .set_metadata(obj_addr, meta_addr)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Return the same object (metadata doesn't change the value)
    Ok(obj)
}

// --- Namespace intrinsics ---

/// Check if value is a namespace.
///
/// `(namespace? x)` - returns true if x is a namespace
pub fn intrinsic_is_namespace<M: MemorySpace>(x_regs: &XRegs, proc: &Process, mem: &M) -> Term {
    Term::bool(proc.is_term_namespace(mem, x_regs[1]))
}

/// Create a namespace.
///
/// `(create-ns sym)` - creates or returns existing namespace with given name
///
/// Namespaces are stored in the realm's namespace registry.
pub fn intrinsic_create_ns<M: MemorySpace>(
    x_regs: &XRegs,
    realm: &mut Realm,
    mem: &mut M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let name = x_regs[1];

    // Name must be an immediate symbol
    if !name.is_symbol() {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "symbol",
        });
    }

    // Create or find existing namespace in realm
    realm
        .get_or_create_namespace(mem, name)
        .ok_or(IntrinsicError::OutOfMemory)
}

/// Find a namespace by name.
///
/// `(find-ns sym)` - returns namespace or nil if not found
///
/// Looks up namespace in the realm's namespace registry.
pub fn intrinsic_find_ns(x_regs: &XRegs, realm: &Realm, id: u8) -> Result<Term, IntrinsicError> {
    let name = x_regs[1];

    // Name must be an immediate symbol
    if !name.is_symbol() {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "symbol",
        });
    }

    // Find namespace in realm, return nil if not found
    Ok(realm.find_namespace(name).unwrap_or(Term::NIL))
}

/// Get namespace name.
///
/// `(ns-name ns)` - returns the namespace's name symbol
pub fn intrinsic_ns_name<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let ns_term = x_regs[1];

    // Must be a namespace
    let ns = proc
        .read_term_namespace(mem, ns_term)
        .ok_or(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "namespace",
        })?;

    Ok(ns.name)
}

/// Get namespace mappings.
///
/// `(ns-map ns)` - returns the namespace's symbol->var mappings
pub fn intrinsic_ns_map<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let ns_term = x_regs[1];

    // Must be a namespace
    let ns = proc
        .read_term_namespace(mem, ns_term)
        .ok_or(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "namespace",
        })?;

    Ok(ns.mappings)
}

// --- Function intrinsic ---

/// Check if value is callable (function, closure, or native function).
///
/// `(fn? x)` - returns true if x is any callable type
pub fn intrinsic_is_fn<M: MemorySpace>(x_regs: &XRegs, proc: &Process, mem: &M) -> Term {
    Term::bool(proc.is_term_callable(mem, x_regs[1]))
}

// --- Var intrinsics ---

/// Check if value is a var.
///
/// `(var? x)` - returns true if x is a var
pub fn intrinsic_is_var<M: MemorySpace>(x_regs: &XRegs, proc: &Process, mem: &M) -> Term {
    Term::bool(proc.is_term_var(mem, x_regs[1]))
}

/// Intern a symbol in a namespace, creating or updating a var.
///
/// `(intern ns sym val)` - creates var in namespace with given value
pub fn intrinsic_intern<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &mut Process,
    mem: &mut M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let ns = x_regs[1];
    let name = x_regs[2];
    let value = x_regs[3];

    // First arg must be a namespace
    if !proc.is_term_namespace(mem, ns) {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "namespace",
        });
    }

    // Second arg must be a symbol
    if !proc.is_term_symbol(name) {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 1,
            expected: "symbol",
        });
    }

    // Intern the var
    proc.intern_var_term(mem, ns, name, value)
        .ok_or(IntrinsicError::OutOfMemory)
}

/// Get the value of a var.
///
/// `(var-get var)` - returns the var's current value
pub fn intrinsic_var_get<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let var = x_regs[1];

    // Must be a var
    if !proc.is_term_var(mem, var) {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "var",
        });
    }

    // Get the var's value
    let value = proc
        .var_get_term(mem, var)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Check for unbound var (Term::UNBOUND)
    if value.is_unbound() {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "bound var",
        });
    }

    Ok(value)
}

/// Define var root value (deep copies value to realm).
///
/// `(def-root var value)` - deep copies value to realm and sets as var root
///
/// This intrinsic is used by the `def` special form for non-process-bound vars.
/// It deep copies the value from the process heap to the realm's code region,
/// then atomically updates the var's root binding.
pub fn intrinsic_def_root<M: MemorySpace>(
    x_regs: &XRegs,
    _proc: &Process,
    realm: &mut Realm,
    mem: &mut M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let var = x_regs[1];
    let value = x_regs[2];

    // Must be a var
    if !var.is_boxed() {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "var",
        });
    }

    // Check it's actually a var
    let header: Header = mem.read(var.to_vaddr());
    if header.object_tag() != object::VAR {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "var",
        });
    }

    // Deep copy the value to the realm's code region
    let mut visited = VisitedTracker::new();
    let realm_value = deep_copy_term_to_realm(value, realm, mem, &mut visited)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Set the var's root binding to the copied value
    realm
        .var_set_root(mem, var, realm_value)
        .ok_or(IntrinsicError::OutOfMemory)?;

    Ok(var)
}

/// Define var binding (sets process-local binding).
///
/// `(def-binding var value)` - sets process-local binding for a process-bound var
///
/// This intrinsic is used by the `def` special form for process-bound vars.
/// It sets the process-local binding without copying to the realm.
pub fn intrinsic_def_binding<M: MemorySpace>(
    x_regs: &XRegs,
    proc: &mut Process,
    mem: &M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let var = x_regs[1];
    let value = x_regs[2];

    // Extract var slot address
    if !var.is_boxed() {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "var",
        });
    }

    // Check it's actually a var
    let header: Header = mem.read(var.to_vaddr());
    if header.object_tag() != object::VAR {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "var",
        });
    }

    let var_addr = var.to_vaddr();

    // Set process binding (value stays on process heap)
    proc.set_binding_term(var_addr, value)
        .ok_or(IntrinsicError::OutOfMemory)?;

    Ok(var)
}

/// Define var metadata (stores metadata in realm).
///
/// `(def-meta var meta)` - deep copies metadata to realm and stores in realm's metadata table
///
/// This intrinsic is used by the `def` special form when metadata is present.
/// It deep copies the metadata map from the process heap to the realm's code region,
/// adds compiler keys (`:name` and `:ns`), then stores the mapping in the realm's
/// metadata table.
pub fn intrinsic_def_meta<M: MemorySpace>(
    x_regs: &XRegs,
    _proc: &Process,
    realm: &mut Realm,
    mem: &mut M,
    id: u8,
) -> Result<Term, IntrinsicError> {
    let var = x_regs[1];
    let meta = x_regs[2];

    // First arg must be a var
    if !var.is_boxed() {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "var",
        });
    }

    let var_addr = var.to_vaddr();
    let var_header: Header = mem.read(var_addr);
    if var_header.object_tag() != object::VAR {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "var",
        });
    }

    // Read the var's content to get name and namespace
    let slot: HeapVar = mem.read(var_addr);

    // Second arg must be a map (or nil for no metadata)
    if meta.is_nil() {
        // No user metadata, but still add :name and :ns
        let final_meta_addr =
            build_compiler_metadata(realm, mem, Term::NIL, slot.name, slot.namespace)?;
        realm
            .set_metadata(var_addr, final_meta_addr)
            .ok_or(IntrinsicError::OutOfMemory)?;
        return Ok(var);
    }

    // Check that meta is a map
    if !meta.is_boxed() {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 1,
            expected: "map",
        });
    }

    let meta_header: Header = mem.read(meta.to_vaddr());
    if meta_header.object_tag() != object::MAP {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 1,
            expected: "map",
        });
    }

    // Deep copy the user metadata map to the realm's code region
    let mut visited = VisitedTracker::new();
    let realm_meta = deep_copy_term_to_realm(meta, realm, mem, &mut visited)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Get the user entries (8-byte header + entries)
    let user_meta_addr = realm_meta.to_vaddr();
    let entries_addr = user_meta_addr.add(8);
    let user_entries: Term = mem.read(entries_addr);

    // Build final metadata with compiler keys prepended
    let final_meta_addr =
        build_compiler_metadata(realm, mem, user_entries, slot.name, slot.namespace)?;

    // Store in realm's metadata table
    realm
        .set_metadata(var_addr, final_meta_addr)
        .ok_or(IntrinsicError::OutOfMemory)?;

    Ok(var)
}

/// Build metadata map with compiler keys (:name and :ns) prepended to existing entries.
fn build_compiler_metadata<M: MemorySpace>(
    realm: &mut Realm,
    mem: &mut M,
    existing_entries: Term,
    name_sym: Term,
    namespace: Term,
) -> Result<crate::Vaddr, IntrinsicError> {
    // Intern :name and :ns keywords
    let name_kw = realm
        .intern_keyword(mem, "name")
        .ok_or(IntrinsicError::OutOfMemory)?;
    let ns_kw = realm
        .intern_keyword(mem, "ns")
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Create [:name name_sym] tuple
    let name_tuple_size = HeapTuple::alloc_size(2);
    let name_tuple_addr = realm
        .alloc(name_tuple_size, 8)
        .ok_or(IntrinsicError::OutOfMemory)?;
    let name_tuple_header = HeapTuple::make_header(2);
    mem.write(name_tuple_addr, name_tuple_header);
    let name_elem0 = name_tuple_addr.add(HeapTuple::HEADER_SIZE as u64);
    let name_elem1 = name_elem0.add(core::mem::size_of::<Term>() as u64);
    mem.write(name_elem0, name_kw);
    mem.write(name_elem1, name_sym);
    let name_tuple = Term::boxed_vaddr(name_tuple_addr);

    // Create [:ns namespace] tuple
    let ns_tuple_size = HeapTuple::alloc_size(2);
    let ns_tuple_addr = realm
        .alloc(ns_tuple_size, 8)
        .ok_or(IntrinsicError::OutOfMemory)?;
    let ns_tuple_header = HeapTuple::make_header(2);
    mem.write(ns_tuple_addr, ns_tuple_header);
    let ns_elem0 = ns_tuple_addr.add(HeapTuple::HEADER_SIZE as u64);
    let ns_elem1 = ns_elem0.add(core::mem::size_of::<Term>() as u64);
    mem.write(ns_elem0, ns_kw);
    mem.write(ns_elem1, namespace);
    let ns_tuple = Term::boxed_vaddr(ns_tuple_addr);

    // Build entries chain: ([:ns namespace] . ([:name name_sym] . existing_entries))
    let name_pair_size = HeapPair::SIZE;
    let name_pair_addr = realm
        .alloc(name_pair_size, 8)
        .ok_or(IntrinsicError::OutOfMemory)?;
    mem.write(
        name_pair_addr,
        HeapPair {
            head: name_tuple,
            tail: existing_entries,
        },
    );

    let ns_pair_addr = realm
        .alloc(name_pair_size, 8)
        .ok_or(IntrinsicError::OutOfMemory)?;
    mem.write(
        ns_pair_addr,
        HeapPair {
            head: ns_tuple,
            tail: Term::list_vaddr(name_pair_addr),
        },
    );

    // Create final map (8-byte header + entries)
    let final_map_addr = realm
        .alloc(HeapMap::SIZE, 8)
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Count entries (2 new + existing)
    let mut entry_count = 2;
    let mut current = existing_entries;
    while current.is_list() {
        entry_count += 1;
        let pair: HeapPair = mem.read(current.to_vaddr());
        current = pair.tail;
    }

    // Write header
    let map_header = HeapMap::make_header(entry_count);
    mem.write(final_map_addr, map_header);

    // Write entries
    let entries_term = Term::list_vaddr(ns_pair_addr);
    let entries_addr = final_map_addr.add(8);
    mem.write(entries_addr, entries_term);

    Ok(final_map_addr)
}
