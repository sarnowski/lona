// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Metadata and namespace intrinsics.

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::value::Value;

use super::IntrinsicError;

// --- Metadata intrinsics ---

/// Get metadata for an object.
///
/// `(meta obj)` - returns metadata map or nil
pub fn intrinsic_meta<M: MemorySpace>(proc: &Process, mem: &M) -> Value {
    let obj = proc.x_regs[1];

    // Get the heap address of the object
    let addr = match obj {
        Value::String(addr)
        | Value::Pair(addr)
        | Value::Symbol(addr)
        | Value::Keyword(addr)
        | Value::Tuple(addr)
        | Value::Map(addr)
        | Value::Namespace(addr)
        | Value::CompiledFn(addr)
        | Value::Closure(addr) => addr,
        // Immediates don't have metadata
        Value::Nil | Value::Bool(_) | Value::Int(_) | Value::NativeFn(_) | Value::Unbound => {
            return Value::Nil;
        }
    };

    // Look up in process metadata table
    proc.get_metadata(addr).map_or(Value::Nil, |meta_addr| {
        // Return the metadata map
        let _: crate::value::HeapMap = mem.read(meta_addr);
        Value::map(meta_addr)
    })
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
        | Value::Map(addr)
        | Value::Namespace(addr)
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
