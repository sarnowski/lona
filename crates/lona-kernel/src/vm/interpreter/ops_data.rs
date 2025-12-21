// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Data movement and global variable operations.

use lona_core::opcode::{decode_a, decode_b, decode_bx};
use lona_core::value::Value;

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
    pub(super) fn op_get_global(
        &mut self,
        instruction: u32,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let const_idx = decode_bx(instruction);

        let symbol = Self::get_symbol_from_constant(frame.chunk(), const_idx, frame)?;
        let value = self.globals.get(symbol).ok_or_else(|| {
            Error::new(
                ErrorKind::UndefinedGlobal {
                    symbol,
                    suggestion: None, // TODO: implement suggestion lookup
                },
                frame.current_location(),
            )
        })?;
        self.set_register(dest, value, frame)?;
        Ok(())
    }

    /// `SetGlobal`: `globals[K[Bx]] = R[A]`
    pub(super) fn op_set_global(
        &mut self,
        instruction: u32,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        let src = decode_a(instruction);
        let const_idx = decode_bx(instruction);

        let symbol = Self::get_symbol_from_constant(frame.chunk(), const_idx, frame)?;
        let value = self.get_register(src, frame)?;
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
    /// Returns the Var itself (not its value), for metadata access.
    pub(super) fn op_get_global_var(
        &mut self,
        instruction: u32,
        frame: &Frame<'_>,
    ) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let const_idx = decode_bx(instruction);

        let symbol = Self::get_symbol_from_constant(frame.chunk(), const_idx, frame)?;
        let var = self.globals.get_var(symbol).ok_or_else(|| {
            Error::new(
                ErrorKind::UndefinedGlobal {
                    symbol,
                    suggestion: None,
                },
                frame.current_location(),
            )
        })?;
        self.set_register(dest, Value::Var(var.clone()), frame)?;
        Ok(())
    }
}
