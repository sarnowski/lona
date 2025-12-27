// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Data movement and global variable operations.

use alloc::sync::Arc;
use alloc::vec::Vec;

use lona_core::chunk::{Constant, UpvalueSource};
use lona_core::error_context::TypeExpectation;
use lona_core::opcode::{decode_a, decode_b, decode_bx};
use lona_core::value::{Function, FunctionBody, Value};

use super::Vm;
use crate::vm::error::{Error, Kind as ErrorKind};
use crate::vm::frame::Frame;

impl Vm<'_> {
    // =========================================================================
    // Data Movement Operations
    // =========================================================================

    /// `Move`: `R[A] = R[B]`
    pub(super) fn op_move(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let src = decode_b(instruction);
        let value = self.get_register(src, frame)?;
        self.set_register(dest, value, frame)?;
        Ok(())
    }

    /// `LoadK`: `R[A] = K[Bx]`
    pub(super) fn op_load_k(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let const_idx = decode_bx(instruction);
        let value = Self::load_constant(frame.chunk(), const_idx, frame)?;
        self.set_register(dest, value, frame)?;
        Ok(())
    }

    /// `LoadNil`: `R[A]..R[A+B] = nil`
    pub(super) fn op_load_nil(&mut self, instruction: u32, frame: &Frame<'_>) {
        let start = decode_a(instruction);
        let count = decode_b(instruction);
        let base = frame.base();

        for offset in 0_u16..=u16::from(count) {
            let reg_idx = base
                .checked_add(usize::from(start))
                .and_then(|x| x.checked_add(usize::from(offset)));
            if let Some(idx) = reg_idx
                && let Some(reg) = self.registers.get_mut(idx)
            {
                *reg = Value::Nil;
            }
        }
    }

    /// `LoadTrue`: `R[A] = true`
    pub(super) fn op_load_true(
        &mut self,
        instruction: u32,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        let dest = decode_a(instruction);
        self.set_register(dest, Value::Bool(true), frame)?;
        Ok(())
    }

    /// `LoadFalse`: `R[A] = false`
    pub(super) fn op_load_false(
        &mut self,
        instruction: u32,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        let dest = decode_a(instruction);
        self.set_register(dest, Value::Bool(false), frame)?;
        Ok(())
    }

    // =========================================================================
    // Global Variable Operations
    // =========================================================================

    /// `GetGlobal`: `R[A] = globals[K[Bx]]`
    ///
    /// Lookup order:
    /// 1. Try the symbol as-is in globals (for user-defined vars like `user/x`)
    /// 2. If not found and symbol is qualified, check the current namespace's
    ///    refers for the unqualified name (for auto-referred primitives like `first`)
    ///
    /// This supports the `lona.core` auto-refer pattern: primitives are registered
    /// in `lona.core` and automatically referred into all namespaces.
    pub(super) fn op_get_global(
        &mut self,
        instruction: u32,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let const_idx = decode_bx(instruction);

        let symbol = Self::get_symbol_from_constant(frame.chunk(), const_idx, frame)?;

        // 1. Try direct lookup in globals (handles user-defined vars)
        if let Some(val) = self.globals.get(symbol) {
            self.set_register(dest, val, frame)?;
            return Ok(());
        }

        // 2. If symbol is qualified (ns/name), look up in the specified namespace
        let symbol_name = self.interner.resolve(symbol);
        if let Some((ns_name, unqualified)) = symbol_name.rsplit_once('/') {
            let ns_sym = self.interner.intern(ns_name);
            let unqualified_sym = self.interner.intern(unqualified);

            // Look up in the specified namespace (not current namespace)
            if let Some(ns) = self.namespace_registry.get(ns_sym)
                && let Some(value) = ns.lookup(unqualified_sym)
            {
                self.set_register(dest, value, frame)?;
                return Ok(());
            }
        }

        // Not found anywhere
        Err(Error::new(
            ErrorKind::UndefinedGlobal {
                symbol,
                suggestion: None,
            },
            frame.current_location(),
        ))
    }

    /// `SetGlobal`: `globals[K[Bx]] = R[A]`
    ///
    /// Defines a var in the namespace. The symbol is expected to be qualified
    /// (e.g., "user/x"). The var is registered in both:
    /// - The namespace registry (for introspection via `ns-publics`)
    /// - The globals map (for execution lookup)
    pub(super) fn op_set_global(
        &mut self,
        instruction: u32,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        let src = decode_a(instruction);
        let const_idx = decode_bx(instruction);

        let symbol = Self::get_symbol_from_constant(frame.chunk(), const_idx, frame)?;
        let value = self.get_register(src, frame)?;

        // Also register the var in the namespace registry for introspection.
        // The symbol is qualified (e.g., "user/x"), so we extract the namespace
        // and unqualified name.
        let symbol_name = self.interner.resolve(symbol);
        if let Some((ns_name, unqualified)) = symbol_name.rsplit_once('/') {
            let ns_sym = self.interner.intern(ns_name);
            let unqualified_sym = self.interner.intern(unqualified);

            // Ensure the namespace exists and intern the var in it
            let ns = self.namespace_registry.get_or_create(ns_sym);
            let _var = ns.intern(unqualified_sym, value.clone());
        }

        // Also update globals for execution lookup (backward compatibility)
        self.globals.set(symbol, value);
        Ok(())
    }

    /// `SetGlobalMeta`: `globals[K[Bx]].merge_meta(R[A])`
    ///
    /// Merges the metadata map in `R[A]` into the Var's existing metadata.
    /// If `R[A]` is nil, this is a no-op.
    pub(super) fn op_set_global_meta(
        &mut self,
        instruction: u32,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        let meta_reg = decode_a(instruction);
        let const_idx = decode_bx(instruction);

        let symbol = Self::get_symbol_from_constant(frame.chunk(), const_idx, frame)?;
        let meta_value = self.get_register(meta_reg, frame)?;

        // Extract Map from Value, or skip if nil
        match meta_value {
            Value::Map(map) => {
                self.globals.merge_meta(symbol, map);
                Ok(())
            }
            Value::Nil => Ok(()), // No metadata to merge
            // All other types are invalid for metadata
            // Value is #[non_exhaustive], so we list known variants and use _ for future ones
            Value::Bool(_)
            | Value::Integer(_)
            | Value::Float(_)
            | Value::Ratio(_)
            | Value::Symbol(_)
            | Value::Keyword(_)
            | Value::String(_)
            | Value::List(_)
            | Value::Vector(_)
            | Value::Set(_)
            | Value::Binary(_)
            | Value::Function(_)
            | Value::NativeFunction(_)
            | Value::Var(_)
            | _ => Err(Error::new(
                ErrorKind::TypeError {
                    operation: "set-global-meta",
                    expected: lona_core::error_context::TypeExpectation::Single(
                        lona_core::value::Kind::Map,
                    ),
                    got: meta_value.kind(),
                    operand: None,
                },
                frame.current_location(),
            )),
        }
    }

    /// `GetGlobalVar`: `R[A] = globals.get_var(K[Bx])`
    ///
    /// Returns the Var itself (not its value), for `#'symbol` syntax.
    /// Uses the same lookup logic as `GetGlobal`: first globals, then
    /// namespace refers for auto-referred primitives.
    pub(super) fn op_get_global_var(
        &mut self,
        instruction: u32,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let const_idx = decode_bx(instruction);

        let symbol = Self::get_symbol_from_constant(frame.chunk(), const_idx, frame)?;

        // 1. Try direct lookup in globals (handles user-defined vars)
        if let Some(var) = self.globals.get_var(symbol) {
            self.set_register(dest, Value::Var(var.clone()), frame)?;
            return Ok(());
        }

        // 2. If symbol is qualified (ns/name), check namespace refers for the var
        let symbol_name = self.interner.resolve(symbol);
        if let Some((_ns, unqualified)) = symbol_name.rsplit_once('/') {
            let unqualified_sym = self.interner.intern(unqualified);

            // Check current namespace's refers for the var
            if let Some(ns) = self.namespace_registry.current()
                && let Some(var) = ns.get_refer(unqualified_sym)
            {
                self.set_register(dest, Value::Var(var.clone()), frame)?;
                return Ok(());
            }
        }

        // Not found anywhere
        Err(Error::new(
            ErrorKind::UndefinedGlobal {
                symbol,
                suggestion: None,
            },
            frame.current_location(),
        ))
    }

    // =========================================================================
    // Closure Operations
    // =========================================================================

    /// `GetUpvalue`: `R[A] = Upvalues[B]`
    ///
    /// Reads a captured value from the current closure's upvalue array.
    pub(super) fn op_get_upvalue(
        &mut self,
        instruction: u32,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let idx = decode_b(instruction);

        let upvalues = frame.upvalues();
        let value = upvalues.get(usize::from(idx)).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidUpvalue {
                    index: idx,
                    available: upvalues.len(),
                },
                frame.current_location(),
            )
        })?;

        self.set_register(dest, value.clone(), frame)?;
        Ok(())
    }

    // =========================================================================
    // Namespace Operations
    // =========================================================================

    /// `SetNamespace`: `current_ns = K[Bx]`
    ///
    /// Switches the current namespace. This affects how unqualified symbols
    /// are resolved in subsequent REPL evaluations and where namespace
    /// operations like `namespace-add-alias` and `namespace-add-refer` apply.
    pub(super) fn op_set_namespace(
        &mut self,
        instruction: u32,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        let const_idx = decode_bx(instruction);

        let symbol = Self::get_symbol_from_constant(frame.chunk(), const_idx, frame)?;
        self.current_namespace = symbol;
        // Also update the namespace registry's current namespace to ensure
        // namespace operations like `namespace-add-alias` and `namespace-add-refer`
        // operate on the correct namespace.
        self.namespace_registry.switch_to(symbol);
        Ok(())
    }

    // =========================================================================
    // Closure Operations
    // =========================================================================

    /// `Closure`: `R[A] = closure(K[Bx])`
    ///
    /// Creates a closure by:
    /// 1. Loading the function template from K\[Bx\]
    /// 2. Copying captured values according to `upvalue_sources`
    /// 3. Storing the new Function in R\[A\]
    pub(super) fn op_closure(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let const_idx = decode_bx(instruction);

        // Load the function template from constants
        let constant = frame.chunk().get_constant(const_idx).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidConstant { index: const_idx },
                frame.current_location(),
            )
        })?;

        // Verify it's a function constant
        let (bodies, name) = match *constant {
            Constant::Function {
                ref bodies,
                ref name,
            } => (bodies, name),
            // All other constant types are not valid for Closure opcode
            Constant::Nil
            | Constant::Bool(_)
            | Constant::Integer(_)
            | Constant::Float(_)
            | Constant::String(_)
            | Constant::Symbol(_)
            | Constant::Keyword(_)
            | Constant::List(_)
            | Constant::Vector(_)
            | Constant::Map(_)
            | _ => {
                return Err(Error::new(
                    ErrorKind::TypeError {
                        operation: "closure",
                        expected: TypeExpectation::Callable,
                        got: lona_core::value::Kind::Nil, // Approximate; we don't have constant kind
                        operand: None,
                    },
                    frame.current_location(),
                ));
            }
        };

        // Capture upvalues according to sources
        // Use the first body's upvalue_sources (all bodies share the same list)
        let upvalue_sources = bodies
            .first()
            .map_or(&[][..], |body| body.upvalue_sources.as_slice());

        let mut captured_values = Vec::with_capacity(upvalue_sources.len());
        for source in upvalue_sources {
            let value = match *source {
                UpvalueSource::Local(reg) => {
                    // Capture from parent's local register
                    self.get_register(reg, frame)?
                }
                UpvalueSource::ParentUpvalue(idx) => {
                    // Capture from parent's upvalue array
                    frame
                        .upvalues()
                        .get(usize::from(idx))
                        .ok_or_else(|| {
                            Error::new(
                                ErrorKind::InvalidUpvalue {
                                    index: idx,
                                    available: frame.upvalues().len(),
                                },
                                frame.current_location(),
                            )
                        })?
                        .clone()
                }
                // Handle future UpvalueSource variants (it's #[non_exhaustive])
                _ => {
                    return Err(Error::new(
                        ErrorKind::InvalidUpvalue {
                            index: 0,
                            available: 0,
                        },
                        frame.current_location(),
                    ));
                }
            };
            captured_values.push(value);
        }

        // Create runtime function bodies
        let fn_bodies: Vec<FunctionBody> = bodies
            .iter()
            .map(|body| {
                let chunk_arc = Arc::new((*body.chunk).clone());
                FunctionBody::new(chunk_arc, body.arity, body.has_rest)
            })
            .collect();

        // Create the closure with captured values
        let upvalues = Arc::from(captured_values);
        let function = Function::new(fn_bodies, name.clone(), upvalues);

        self.set_register(dest, Value::Function(function), frame)?;
        Ok(())
    }
}
