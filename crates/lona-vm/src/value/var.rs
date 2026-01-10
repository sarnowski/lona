// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Var-related types and flags.
//!
//! Vars are the runtime representation of Clojure-style vars - named references
//! that support dynamic rebinding. This module contains the var slot structure,
//! var content, and associated flags.

use super::Value;
use crate::Vaddr;

/// Var flags for metadata and behavior.
pub mod var_flags {
    /// Var is process-bound (value can be shadowed per-process).
    pub const PROCESS_BOUND: u32 = 0x0001;
    /// Var is a native intrinsic (value is `NativeFn`).
    pub const NATIVE: u32 = 0x0002;
    /// Var is a macro (compile-time expansion).
    pub const MACRO: u32 = 0x0004;
    /// Var is private (not exported from namespace).
    pub const PRIVATE: u32 = 0x0008;
    /// Var is a special form (cannot be used as value).
    pub const SPECIAL_FORM: u32 = 0x0010;
}

/// Var slot - a stable, addressable reference to var content.
///
/// The `VarSlot`'s address serves as the `VarId` for process-bound lookups.
/// Updates create new `VarContent` and atomically swap the content pointer
/// using MVCC (Multi-Version Concurrency Control) semantics.
///
/// **Atomic semantics**: The `content` pointer must be read/written using
/// atomic operations with proper memory ordering:
/// - Reads: Use Acquire ordering (`MemorySpace::read_u64_acquire`)
/// - Writes: Use Release ordering (`MemorySpace::write_u64_release`)
///
/// This ensures readers always see a consistent `VarContent` - either the
/// old or new version, never a partially-written state.
///
/// Stored in memory as:
/// - 8 bytes: content pointer (`Vaddr` pointing to `VarContent`)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VarSlot {
    /// Pointer to the current `VarContent`.
    ///
    /// Must be accessed atomically via `MemorySpace::read_u64_acquire` and
    /// `MemorySpace::write_u64_release` to ensure proper synchronization.
    pub content: Vaddr,
}

impl VarSlot {
    /// Size of the var slot in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

/// Var content - the actual binding data for a var.
///
/// `VarContent` is immutable once created. Updates create a new `VarContent`
/// and atomically swap the `VarSlot`'s pointer.
///
/// Stored in memory as:
/// - 8 bytes: name (Vaddr to interned symbol)
/// - 8 bytes: namespace (Vaddr to containing namespace)
/// - 16 bytes: root (inline Value - the root binding)
/// - 4 bytes: flags (var metadata flags)
/// - 4 bytes: padding for alignment
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VarContent {
    /// Var name (interned symbol address).
    pub name: Vaddr,
    /// Containing namespace (Namespace address).
    pub namespace: Vaddr,
    /// Root binding value (inline, not a pointer).
    ///
    /// For process-bound vars, this is the default value when no
    /// process binding exists. Can be `Value::Unbound` for declared
    /// but uninitialized vars.
    pub root: Value,
    /// Var flags (see `var_flags` module).
    pub flags: u32,
    /// Padding for 8-byte alignment.
    pub padding: u32,
}

impl VarContent {
    /// Size of the var content in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Check if var is process-bound.
    #[inline]
    #[must_use]
    pub const fn is_process_bound(&self) -> bool {
        self.flags & var_flags::PROCESS_BOUND != 0
    }

    /// Check if var is a native intrinsic.
    #[inline]
    #[must_use]
    pub const fn is_native(&self) -> bool {
        self.flags & var_flags::NATIVE != 0
    }

    /// Check if var is a macro.
    #[inline]
    #[must_use]
    pub const fn is_macro(&self) -> bool {
        self.flags & var_flags::MACRO != 0
    }

    /// Check if var is private.
    #[inline]
    #[must_use]
    pub const fn is_private(&self) -> bool {
        self.flags & var_flags::PRIVATE != 0
    }

    /// Check if var is a special form.
    #[inline]
    #[must_use]
    pub const fn is_special_form(&self) -> bool {
        self.flags & var_flags::SPECIAL_FORM != 0
    }
}
