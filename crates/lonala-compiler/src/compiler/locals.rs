// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Local variable scope management for the compiler.
//!
//! This module provides the [`LocalEnv`] type which tracks local variable bindings
//! across nested scopes (function parameters, let bindings, etc.).

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use lona_core::symbol;

/// Tracks local variable bindings across nested scopes.
///
/// Used to implement `let` bindings and function parameters. Each scope
/// maps symbol IDs to the register where that variable is stored.
pub struct LocalEnv {
    /// Stack of scopes, each mapping symbol ID to register.
    pub scopes: Vec<BTreeMap<symbol::Id, u8>>,
}

impl LocalEnv {
    /// Creates a new empty local environment.
    #[inline]
    pub const fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    /// Pushes a new scope (entering `let`, `fn`, etc.).
    #[inline]
    pub fn push_scope(&mut self) {
        self.scopes.push(BTreeMap::new());
    }

    /// Pops the current scope (exiting `let`, `fn`, etc.).
    #[inline]
    pub fn pop_scope(&mut self) {
        let _: Option<BTreeMap<symbol::Id, u8>> = self.scopes.pop();
    }

    /// Defines a local variable in the current scope.
    #[inline]
    pub fn define(&mut self, name: symbol::Id, register: u8) {
        if let Some(scope) = self.scopes.last_mut() {
            let _: Option<u8> = scope.insert(name, register);
        }
    }

    /// Looks up a local variable, searching from innermost to outermost scope.
    #[inline]
    pub fn lookup(&self, name: symbol::Id) -> Option<u8> {
        for scope in self.scopes.iter().rev() {
            if let Some(&reg) = scope.get(&name) {
                return Some(reg);
            }
        }
        None
    }
}
