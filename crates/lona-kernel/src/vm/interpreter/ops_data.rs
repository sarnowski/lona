// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Data movement and global variable operations.

use lona_core::opcode::{decode_a, decode_b, decode_bx};
use lona_core::value::Value;

use super::Vm;
use crate::vm::error::Error;
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
        let value = self
            .globals
            .get(symbol)
            .ok_or_else(|| Error::UndefinedGlobal {
                symbol,
                span: frame.current_span(),
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
}
