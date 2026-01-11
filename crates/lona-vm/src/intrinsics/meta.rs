// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Metadata and namespace intrinsics.

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::realm::{Realm, VisitedTracker, deep_copy_to_realm};
use crate::value::{HeapMap, HeapTuple, Pair, Value, VarContent, VarSlot};

use super::IntrinsicError;

// --- Metadata intrinsics ---

/// Get metadata for an object.
///
/// `(meta obj)` - returns metadata map or nil
///
/// For vars (realm objects), checks the realm's metadata table.
/// For process-heap objects, checks the process's metadata table.
pub fn intrinsic_meta<M: MemorySpace>(proc: &Process, realm: &Realm, _mem: &M) -> Value {
    let obj = proc.x_regs[1];

    // Get the heap address of the object
    let addr = match obj {
        Value::String(addr)
        | Value::Pair(addr)
        | Value::Symbol(addr)
        | Value::Keyword(addr)
        | Value::Tuple(addr)
        | Value::Vector(addr)
        | Value::Map(addr)
        | Value::Namespace(addr)
        | Value::CompiledFn(addr)
        | Value::Closure(addr) => addr,
        // Vars are realm objects - check realm metadata table
        Value::Var(addr) => {
            return realm.get_metadata(addr).map_or(Value::Nil, Value::map);
        }
        // Immediates don't have metadata
        Value::Nil | Value::Bool(_) | Value::Int(_) | Value::NativeFn(_) | Value::Unbound => {
            return Value::Nil;
        }
    };

    // Look up in process metadata table for non-var objects
    proc.get_metadata(addr).map_or(Value::Nil, Value::map)
}

/// Attach metadata to an object.
///
/// `(with-meta obj m)` - returns obj with metadata attached
pub fn intrinsic_with_meta<M: MemorySpace>(
    proc: &mut Process,
    _mem: &M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let obj = proc.x_regs[1];
    let meta = proc.x_regs[2];

    // Metadata must be a map (or nil to clear)
    let meta_addr = match meta {
        Value::Nil => {
            // Clear metadata by not setting any - just return the object
            return Ok(obj);
        }
        Value::Map(addr) => addr,
        _ => {
            return Err(IntrinsicError::TypeError {
                intrinsic: id,
                arg: 1,
                expected: "map",
            });
        }
    };

    // Get the heap address of the object
    let obj_addr = match obj {
        Value::String(addr)
        | Value::Pair(addr)
        | Value::Symbol(addr)
        | Value::Keyword(addr)
        | Value::Tuple(addr)
        | Value::Vector(addr)
        | Value::Map(addr)
        | Value::Namespace(addr)
        | Value::Var(addr)
        | Value::CompiledFn(addr)
        | Value::Closure(addr) => addr,
        // Immediates can't have metadata
        Value::Nil | Value::Bool(_) | Value::Int(_) | Value::NativeFn(_) | Value::Unbound => {
            return Err(IntrinsicError::TypeError {
                intrinsic: id,
                arg: 0,
                expected: "reference type",
            });
        }
    };

    // Store in process metadata table
    proc.set_metadata(obj_addr, meta_addr);

    // Return the same object (metadata doesn't change the value)
    Ok(obj)
}

// --- Namespace intrinsics ---

/// Check if value is a namespace.
///
/// `(namespace? x)` - returns true if x is a namespace
pub const fn intrinsic_is_namespace(proc: &Process) -> Value {
    Value::bool(proc.x_regs[1].is_namespace())
}

/// Create a namespace.
///
/// `(create-ns sym)` - creates or returns existing namespace with given name
pub fn intrinsic_create_ns<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let name = proc.x_regs[1];

    // Name must be a symbol
    let Value::Symbol(_) = name else {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "symbol",
        });
    };

    // Create or find existing namespace
    proc.get_or_create_namespace(mem, name)
        .ok_or(IntrinsicError::OutOfMemory)
}

/// Find a namespace by name.
///
/// `(find-ns sym)` - returns namespace or nil if not found
pub fn intrinsic_find_ns<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let name = proc.x_regs[1];

    // Name must be a symbol
    let Value::Symbol(_) = name else {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "symbol",
        });
    };

    // Find namespace, return nil if not found
    Ok(proc.find_namespace(mem, name).unwrap_or(Value::Nil))
}

/// Get namespace name.
///
/// `(ns-name ns)` - returns the namespace's name symbol
pub fn intrinsic_ns_name<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let ns_val = proc.x_regs[1];

    // Must be a namespace
    let ns = proc
        .read_namespace(mem, ns_val)
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
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let ns_val = proc.x_regs[1];

    // Must be a namespace
    let ns = proc
        .read_namespace(mem, ns_val)
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
pub const fn intrinsic_is_fn(proc: &Process) -> Value {
    Value::bool(proc.x_regs[1].is_fn())
}

// --- Var intrinsics ---

/// Check if value is a var.
///
/// `(var? x)` - returns true if x is a var
pub const fn intrinsic_is_var(proc: &Process) -> Value {
    Value::bool(proc.x_regs[1].is_var())
}

/// Intern a symbol in a namespace, creating or updating a var.
///
/// `(intern ns sym val)` - creates var in namespace with given value
pub fn intrinsic_intern<M: MemorySpace>(
    proc: &mut Process,
    mem: &mut M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let ns = proc.x_regs[1];
    let name = proc.x_regs[2];
    let value = proc.x_regs[3];

    // First arg must be a namespace
    if !ns.is_namespace() {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "namespace",
        });
    }

    // Second arg must be a symbol
    if !matches!(name, Value::Symbol(_)) {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 1,
            expected: "symbol",
        });
    }

    // Intern the var
    proc.intern_var(mem, ns, name, value)
        .ok_or(IntrinsicError::OutOfMemory)
}

/// Get the value of a var.
///
/// `(var-get var)` - returns the var's current value
pub fn intrinsic_var_get<M: MemorySpace>(
    proc: &Process,
    mem: &M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let var = proc.x_regs[1];

    // Must be a var
    if !var.is_var() {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "var",
        });
    }

    // Get the var's value
    let value = proc.var_get(mem, var).ok_or(IntrinsicError::OutOfMemory)?;

    // Check for unbound var
    if matches!(value, Value::Unbound) {
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
    proc: &Process,
    realm: &mut Realm,
    mem: &mut M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let var = proc.x_regs[1];
    let value = proc.x_regs[2];

    // Must be a var
    if !var.is_var() {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "var",
        });
    }

    // Deep copy the value to the realm's code region
    let mut visited = VisitedTracker::new();
    let realm_value =
        deep_copy_to_realm(value, realm, mem, &mut visited).ok_or(IntrinsicError::OutOfMemory)?;

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
    proc: &mut Process,
    _mem: &M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let var = proc.x_regs[1];
    let value = proc.x_regs[2];

    // Extract var slot address
    let Value::Var(var_id) = var else {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "var",
        });
    };

    // Set process binding (value stays on process heap)
    proc.set_binding(var_id, value)
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
    proc: &Process,
    realm: &mut Realm,
    mem: &mut M,
    id: u8,
) -> Result<Value, IntrinsicError> {
    let var = proc.x_regs[1];
    let meta = proc.x_regs[2];

    // First arg must be a var
    let Value::Var(var_addr) = var else {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 0,
            expected: "var",
        });
    };

    // Read the var's content to get name and namespace
    let slot: VarSlot = mem.read(var_addr);
    let content: VarContent = mem.read(slot.content);

    // Second arg must be a map (or nil for no metadata)
    if meta.is_nil() {
        // No user metadata, but still add :name and :ns
        let final_meta_addr =
            build_compiler_metadata(realm, mem, Value::Nil, content.name, content.namespace)?;
        realm
            .set_metadata(var_addr, final_meta_addr)
            .ok_or(IntrinsicError::OutOfMemory)?;
        return Ok(var);
    }

    let Value::Map(_) = meta else {
        return Err(IntrinsicError::TypeError {
            intrinsic: id,
            arg: 1,
            expected: "map",
        });
    };

    // Deep copy the user metadata map to the realm's code region
    let mut visited = VisitedTracker::new();
    let realm_meta =
        deep_copy_to_realm(meta, realm, mem, &mut visited).ok_or(IntrinsicError::OutOfMemory)?;

    // Get the user entries
    let Value::Map(user_meta_addr) = realm_meta else {
        return Err(IntrinsicError::OutOfMemory);
    };
    let user_map: HeapMap = mem.read(user_meta_addr);

    // Build final metadata with compiler keys prepended
    let final_meta_addr = build_compiler_metadata(
        realm,
        mem,
        user_map.entries,
        content.name,
        content.namespace,
    )?;

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
    existing_entries: Value,
    name_sym_addr: crate::Vaddr,
    namespace_addr: crate::Vaddr,
) -> Result<crate::Vaddr, IntrinsicError> {
    // Intern :name and :ns keywords
    let name_kw = realm
        .intern_keyword(mem, "name")
        .ok_or(IntrinsicError::OutOfMemory)?;
    let ns_kw = realm
        .intern_keyword(mem, "ns")
        .ok_or(IntrinsicError::OutOfMemory)?;

    // Create values for name and namespace
    let name_sym = Value::symbol(name_sym_addr);
    let namespace = Value::namespace(namespace_addr);

    // Create [:name name_sym] tuple
    let name_tuple_size = HeapTuple::alloc_size(2);
    let name_tuple_addr = realm
        .alloc(name_tuple_size, 8)
        .ok_or(IntrinsicError::OutOfMemory)?;
    let name_tuple_header = HeapTuple { len: 2, padding: 0 };
    mem.write(name_tuple_addr, name_tuple_header);
    let name_elem0 = name_tuple_addr.add(HeapTuple::HEADER_SIZE as u64);
    let name_elem1 = name_elem0.add(core::mem::size_of::<Value>() as u64);
    mem.write(name_elem0, name_kw);
    mem.write(name_elem1, name_sym);
    let name_tuple = Value::tuple(name_tuple_addr);

    // Create [:ns namespace] tuple
    let ns_tuple_size = HeapTuple::alloc_size(2);
    let ns_tuple_addr = realm
        .alloc(ns_tuple_size, 8)
        .ok_or(IntrinsicError::OutOfMemory)?;
    let ns_tuple_header = HeapTuple { len: 2, padding: 0 };
    mem.write(ns_tuple_addr, ns_tuple_header);
    let ns_elem0 = ns_tuple_addr.add(HeapTuple::HEADER_SIZE as u64);
    let ns_elem1 = ns_elem0.add(core::mem::size_of::<Value>() as u64);
    mem.write(ns_elem0, ns_kw);
    mem.write(ns_elem1, namespace);
    let ns_tuple = Value::tuple(ns_tuple_addr);

    // Build entries chain: ([:ns namespace] . ([:name name_sym] . existing_entries))
    let name_pair_addr = realm
        .alloc(Pair::SIZE, 8)
        .ok_or(IntrinsicError::OutOfMemory)?;
    mem.write(
        name_pair_addr,
        Pair {
            first: name_tuple,
            rest: existing_entries,
        },
    );

    let ns_pair_addr = realm
        .alloc(Pair::SIZE, 8)
        .ok_or(IntrinsicError::OutOfMemory)?;
    mem.write(
        ns_pair_addr,
        Pair {
            first: ns_tuple,
            rest: Value::pair(name_pair_addr),
        },
    );

    // Create final HeapMap
    let final_map_addr = realm
        .alloc(HeapMap::SIZE, 8)
        .ok_or(IntrinsicError::OutOfMemory)?;
    mem.write(
        final_map_addr,
        HeapMap {
            entries: Value::pair(ns_pair_addr),
        },
    );

    Ok(final_map_addr)
}
