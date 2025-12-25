// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Helper methods for the VM interpreter.
//!
//! This module provides utility functions for:
//! - Register access (get/set)
//! - Constant loading and conversion
//! - Symbol lookup from constant pool

use lona_core::chunk::{Chunk, Constant};
use lona_core::error_context::TypeExpectation;
use lona_core::integer::Integer;
use lona_core::opcode::{rk_index, rk_is_constant};
use lona_core::symbol;
use lona_core::value::{self, Function, FunctionBody, Value};

use super::Vm;
use crate::vm::error::{Error, Kind as ErrorKind};
use crate::vm::frame::Frame;

impl Vm<'_> {
    // =========================================================================
    // Register Access
    // =========================================================================

    /// Gets a value from a register.
    #[inline]
    pub(super) fn get_register(&self, index: u8, frame: &Frame<'_>) -> Result<Value, Error> {
        let absolute_index = frame.base().saturating_add(usize::from(index));
        self.registers.get(absolute_index).cloned().ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidRegister { index },
                frame.current_location(),
            )
        })
    }

    /// Sets a value in a register.
    #[inline]
    pub(super) fn set_register(
        &mut self,
        index: u8,
        value: Value,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        let absolute_index = frame.base().saturating_add(usize::from(index));
        let reg = self.registers.get_mut(absolute_index).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidRegister { index },
                frame.current_location(),
            )
        })?;
        *reg = value;
        Ok(())
    }

    /// Gets a value from an RK field (register or constant).
    ///
    /// Note: Function constants are not expected in RK operands, so this
    /// method uses the static conversion that doesn't handle them.
    #[inline]
    pub(super) fn get_rk(&self, rk: u8, frame: &Frame<'_>) -> Result<Value, Error> {
        if rk_is_constant(rk) {
            let const_index = u16::from(rk_index(rk));
            Self::rk_constant_to_value(frame.chunk(), const_index, frame)
        } else {
            self.get_register(rk, frame)
        }
    }

    // =========================================================================
    // Constant Loading
    // =========================================================================

    /// Loads a constant and converts it to a value.
    ///
    /// Handles function constants by creating a `Value::Function` with an
    /// `Arc<Chunk>` for the function's bytecode.
    #[inline]
    pub(super) fn load_constant(
        chunk: &Chunk,
        index: u16,
        frame: &Frame<'_>,
    ) -> Result<Value, Error> {
        let constant = chunk.get_constant(index).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidConstant { index },
                frame.current_location(),
            )
        })?;

        Self::convert_constant(constant)
    }

    /// Converts a constant to a value, handling function constants.
    #[inline]
    pub(super) fn convert_constant(constant: &Constant) -> Result<Value, Error> {
        Ok(match *constant {
            Constant::Bool(val) => Value::Bool(val),
            Constant::Integer(num) => Value::Integer(Integer::from_i64(num)),
            Constant::Float(num) => Value::Float(num),
            Constant::Symbol(id) => Value::from(id),
            Constant::Keyword(id) => Value::Keyword(id),
            Constant::String(ref text) => {
                Value::String(lona_core::string::HeapStr::from(text.as_str()))
            }
            Constant::List(ref elements) => {
                let values: Result<alloc::vec::Vec<Value>, Error> =
                    elements.iter().map(Self::convert_constant).collect();
                Value::List(lona_core::list::List::from_vec(values?))
            }
            Constant::Vector(ref elements) => {
                let values: Result<alloc::vec::Vec<Value>, Error> =
                    elements.iter().map(Self::convert_constant).collect();
                Value::Vector(lona_core::vector::Vector::from_vec(values?))
            }
            Constant::Map(ref pairs) => {
                let converted_pairs: Result<alloc::vec::Vec<(Value, Value)>, Error> = pairs
                    .iter()
                    .map(|&(ref key, ref val)| {
                        let key_val = Self::convert_constant(key)?;
                        let val_val = Self::convert_constant(val)?;
                        Ok((key_val, val_val))
                    })
                    .collect();
                Value::Map(lona_core::map::Map::from_pairs(converted_pairs?))
            }
            Constant::Function {
                ref bodies,
                ref name,
            } => {
                // Convert each FunctionBodyData to FunctionBody
                let fn_bodies: alloc::vec::Vec<FunctionBody> = bodies
                    .iter()
                    .map(|body| {
                        let chunk_arc = alloc::sync::Arc::new((*body.chunk).clone());
                        FunctionBody::new(chunk_arc, body.arity, body.has_rest)
                    })
                    .collect();
                Value::Function(Function::new_simple(fn_bodies, name.clone()))
            }
            // Handle Nil and future Constant variants (Constant is #[non_exhaustive])
            Constant::Nil | _ => Value::Nil,
        })
    }

    /// Converts a constant pool entry to a value (static version for RK operands).
    ///
    /// Does not handle function constants since they are not expected in RK positions.
    #[inline]
    pub(super) fn rk_constant_to_value(
        chunk: &Chunk,
        index: u16,
        frame: &Frame<'_>,
    ) -> Result<Value, Error> {
        let constant = chunk.get_constant(index).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidConstant { index },
                frame.current_location(),
            )
        })?;

        Self::convert_simple_constant(constant)
    }

    /// Converts a simple (non-function) constant to a value.
    #[inline]
    pub(super) fn convert_simple_constant(constant: &Constant) -> Result<Value, Error> {
        Ok(match *constant {
            Constant::Bool(val) => Value::Bool(val),
            Constant::Integer(num) => Value::Integer(Integer::from_i64(num)),
            Constant::Float(num) => Value::Float(num),
            Constant::Symbol(id) => Value::from(id),
            Constant::Keyword(id) => Value::Keyword(id),
            Constant::String(ref text) => {
                Value::String(lona_core::string::HeapStr::from(text.as_str()))
            }
            Constant::List(ref elements) => {
                let values: Result<alloc::vec::Vec<Value>, Error> =
                    elements.iter().map(Self::convert_simple_constant).collect();
                Value::List(lona_core::list::List::from_vec(values?))
            }
            Constant::Vector(ref elements) => {
                let values: Result<alloc::vec::Vec<Value>, Error> =
                    elements.iter().map(Self::convert_simple_constant).collect();
                Value::Vector(lona_core::vector::Vector::from_vec(values?))
            }
            Constant::Map(ref pairs) => {
                let converted_pairs: Result<alloc::vec::Vec<(Value, Value)>, Error> = pairs
                    .iter()
                    .map(|&(ref key, ref val)| {
                        let key_val = Self::convert_simple_constant(key)?;
                        let val_val = Self::convert_simple_constant(val)?;
                        Ok((key_val, val_val))
                    })
                    .collect();
                Value::Map(lona_core::map::Map::from_pairs(converted_pairs?))
            }
            // Handle Nil, Function, and future Constant variants
            Constant::Nil | Constant::Function { .. } | _ => Value::Nil,
        })
    }

    // =========================================================================
    // Symbol Lookup
    // =========================================================================

    /// Gets a symbol ID from a constant pool entry.
    #[inline]
    pub(super) fn get_symbol_from_constant(
        chunk: &Chunk,
        index: u16,
        frame: &Frame<'_>,
    ) -> Result<symbol::Id, Error> {
        let constant = chunk.get_constant(index).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidConstant { index },
                frame.current_location(),
            )
        })?;

        match *constant {
            Constant::Symbol(id) => Ok(id),
            Constant::Nil
            | Constant::Bool(_)
            | Constant::Integer(_)
            | Constant::Float(_)
            | Constant::String(_)
            | Constant::Keyword(_)
            | Constant::List(_)
            | Constant::Vector(_)
            | _ => Err(Error::new(
                ErrorKind::TypeError {
                    operation: "symbol lookup",
                    expected: TypeExpectation::Symbol,
                    got: constant_type_name_to_kind(constant),
                    operand: None,
                },
                frame.current_location(),
            )),
        }
    }
}

/// Converts a constant type name to a `value::Kind` for error reporting.
#[inline]
pub(super) const fn constant_type_name_to_kind(constant: &Constant) -> value::Kind {
    match *constant {
        Constant::Nil => value::Kind::Nil,
        Constant::Bool(_) => value::Kind::Bool,
        Constant::Integer(_) => value::Kind::Integer,
        Constant::Float(_) => value::Kind::Float,
        Constant::String(_) => value::Kind::String,
        Constant::Symbol(_) => value::Kind::Symbol,
        Constant::Keyword(_) => value::Kind::Keyword,
        Constant::List(_) => value::Kind::List,
        Constant::Vector(_) => value::Kind::Vector,
        Constant::Map(_) => value::Kind::Map,
        Constant::Function { .. } | _ => value::Kind::Function,
    }
}
