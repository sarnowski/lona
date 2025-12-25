// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Closure support and upvalue tracking for the compiler.
//!
//! This module handles capturing variables from enclosing scopes when compiling
//! nested functions (closures). It tracks:
//!
//! - Which variables need to be captured as upvalues
//! - Whether to capture from a parent's local register or parent's upvalue array
//! - Building capture contexts for nested function compilation

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use lona_core::chunk::UpvalueSource;
use lona_core::symbol;

use super::Compiler;

// =============================================================================
// Upvalue Tracking for Closures
// =============================================================================

/// Tracks an upvalue captured by the function being compiled.
#[derive(Debug, Clone)]
pub struct UpvalueInfo {
    /// The symbol being captured.
    pub symbol: symbol::Id,
    /// How to capture this upvalue at runtime.
    pub source: UpvalueSource,
}

/// Information about a variable available for capture from a parent scope.
#[derive(Debug, Clone)]
pub struct ParentLocal {
    /// The register in the parent where this variable is stored.
    pub register: u8,
}

/// Context for capturing variables from enclosing scopes.
///
/// When compiling a nested function, this describes what variables are
/// available for capture from the immediately enclosing function.
#[derive(Debug, Clone, Default)]
pub struct CaptureContext {
    /// Variables defined as locals in the parent function.
    /// Maps symbol ID to register index.
    pub parent_locals: BTreeMap<symbol::Id, ParentLocal>,
    /// Upvalues available in the parent function.
    /// Maps symbol ID to upvalue index in parent's upvalue array.
    /// Used for nested closures capturing from grandparent+ scopes.
    pub parent_upvalues: BTreeMap<symbol::Id, u8>,
}

impl CaptureContext {
    /// Creates an empty capture context (for top-level or non-closure functions).
    #[inline]
    pub const fn new() -> Self {
        Self {
            parent_locals: BTreeMap::new(),
            parent_upvalues: BTreeMap::new(),
        }
    }
}

/// Result of resolving a symbol during compilation.
#[derive(Debug, Clone, Copy)]
pub enum SymbolResolution {
    /// Symbol is a local variable in the current function.
    Local(u8),
    /// Symbol is captured as an upvalue.
    Upvalue(u8),
    /// Symbol is a global variable.
    Global,
}

// =============================================================================
// Compiler Methods for Upvalue Handling
// =============================================================================

impl Compiler<'_, '_, '_> {
    /// Resolves a symbol, returning how to access it.
    ///
    /// Resolution order:
    /// 1. Local variables in the current function
    /// 2. Already-captured upvalues
    /// 3. Attempt to capture from parent scope
    /// 4. Fall back to global lookup
    #[inline]
    pub(crate) fn resolve_symbol(&mut self, symbol: symbol::Id) -> SymbolResolution {
        // 1. Check locals in current function
        if let Some(reg) = self.locals.lookup(symbol) {
            return SymbolResolution::Local(reg);
        }

        // 2. Check if already captured as upvalue
        if let Some(idx) = self.lookup_upvalue(symbol) {
            return SymbolResolution::Upvalue(idx);
        }

        // 3. Try to capture from enclosing scope
        if let Some(idx) = self.try_capture_upvalue(symbol) {
            return SymbolResolution::Upvalue(idx);
        }

        // 4. Must be a global
        SymbolResolution::Global
    }

    /// Checks if a symbol is already captured as an upvalue.
    ///
    /// Returns the upvalue index if found.
    #[inline]
    fn lookup_upvalue(&self, symbol: symbol::Id) -> Option<u8> {
        for (idx, upvalue) in self.upvalues.iter().enumerate() {
            if upvalue.symbol == symbol {
                return u8::try_from(idx).ok();
            }
        }
        None
    }

    /// Attempts to capture a variable from an enclosing scope.
    ///
    /// Returns the upvalue index if the variable was successfully captured.
    #[inline]
    fn try_capture_upvalue(&mut self, symbol: symbol::Id) -> Option<u8> {
        // First check if it's a local in the parent scope
        if let Some(parent_local) = self.capture_context.parent_locals.get(&symbol) {
            // Capture from parent's local register
            return self.add_upvalue(symbol, UpvalueSource::Local(parent_local.register));
        }

        // Then check if it's an upvalue in the parent scope (for nested closures)
        if let Some(&parent_upvalue_idx) = self.capture_context.parent_upvalues.get(&symbol) {
            // Capture from parent's upvalue array
            return self.add_upvalue(symbol, UpvalueSource::ParentUpvalue(parent_upvalue_idx));
        }

        None
    }

    /// Adds a new upvalue to the current function.
    ///
    /// Returns the upvalue index, or `None` if the upvalue array is full.
    #[inline]
    fn add_upvalue(&mut self, symbol: symbol::Id, source: UpvalueSource) -> Option<u8> {
        // Check if already captured (shouldn't happen if called correctly, but be safe)
        if let Some(idx) = self.lookup_upvalue(symbol) {
            return Some(idx);
        }

        // Check limit (max 256 upvalues)
        let idx = u8::try_from(self.upvalues.len()).ok()?;
        self.upvalues.push(UpvalueInfo { symbol, source });
        Some(idx)
    }

    /// Returns the collected upvalue sources for the current function.
    ///
    /// Called after compilation to get the upvalue descriptors for `FunctionBodyData`.
    #[inline]
    pub(crate) fn take_upvalue_sources(&self) -> Vec<UpvalueSource> {
        self.upvalues.iter().map(|info| info.source).collect()
    }

    /// Builds a capture context from the current compiler's state.
    ///
    /// Used when creating a child compiler for a nested function.
    /// The context includes all locals in the current function and any upvalues
    /// that the current function has captured.
    ///
    /// For transitive closure support, this also preemptively captures any
    /// variables available through our own capture context. This ensures that
    /// deeply nested functions can access variables from grandparent+ scopes.
    #[inline]
    pub(crate) fn build_capture_context(&mut self) -> CaptureContext {
        let mut parent_locals = BTreeMap::new();

        // Collect all locals from all scopes in the current function
        for scope in &self.locals.scopes {
            for (&symbol, &register) in scope {
                let _existing = parent_locals.insert(symbol, ParentLocal { register });
            }
        }

        // Preemptively capture variables from our capture_context that we haven't
        // captured yet. This enables transitive capture for nested closures.
        // For example, in (fn [a] (fn [] (fn [] a))):
        //   - The middle function (fn [] ...) has access to `a` via its capture_context
        //   - The inner function (fn [] a) needs to capture `a` from the middle function
        //   - So the middle function must first capture `a` to make it available
        for (&symbol, parent_local) in &self.capture_context.parent_locals.clone() {
            if !parent_locals.contains_key(&symbol) {
                // Capture from our parent's local register
                let _idx = self.add_upvalue(symbol, UpvalueSource::Local(parent_local.register));
            }
        }
        for (&symbol, &upvalue_idx) in &self.capture_context.parent_upvalues.clone() {
            if !parent_locals.contains_key(&symbol) && self.lookup_upvalue(symbol).is_none() {
                // Capture from our parent's upvalue array
                let _idx = self.add_upvalue(symbol, UpvalueSource::ParentUpvalue(upvalue_idx));
            }
        }

        // Now collect all upvalues (including newly captured ones)
        let mut parent_upvalues = BTreeMap::new();
        for (idx, upvalue) in self.upvalues.iter().enumerate() {
            if let Ok(idx_u8) = u8::try_from(idx) {
                let _existing = parent_upvalues.insert(upvalue.symbol, idx_u8);
            }
        }

        CaptureContext {
            parent_locals,
            parent_upvalues,
        }
    }

    /// Sets the capture context for this compiler.
    ///
    /// Called when creating a child compiler to enable upvalue capture.
    #[inline]
    pub(crate) fn set_capture_context(&mut self, context: CaptureContext) {
        self.capture_context = context;
    }
}
