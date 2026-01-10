// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Namespace heap type.
//!
//! Namespaces are containers for var bindings, similar to Clojure namespaces.

use super::Value;

/// Heap-allocated namespace header.
///
/// Namespaces are containers for var bindings. The `name` field is a symbol,
/// and `mappings` is a `Value::Map` holding symbol→var mappings.
///
/// Stored in memory as:
/// - 16 bytes: name (`Value::Symbol`)
/// - 16 bytes: mappings (`Value::Map`)
///
/// Example: namespace `my.app` with var `x`:
/// ```text
/// Namespace { name: 'my.app, mappings: %{'x → var-addr} }
/// ```
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Namespace {
    /// The namespace name (a symbol).
    pub name: Value,
    /// Symbol→Vaddr mappings (a map). In the future, this will map to `VarSlot`s.
    /// For now, this is a map of symbol→value.
    pub mappings: Value,
}

impl Namespace {
    /// Size of the namespace header in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();
}
