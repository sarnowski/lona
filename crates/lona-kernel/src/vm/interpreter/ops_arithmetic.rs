// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Arithmetic and comparison operations.

use lona_core::opcode::{decode_a, decode_b, decode_c};
use lona_core::value::Value;

use super::Vm;
use crate::vm::error::Error;
use crate::vm::frame::Frame;
use crate::vm::helpers::{value_type_name, values_equal};
use crate::vm::numeric;

impl Vm<'_> {
    // =========================================================================
    // Arithmetic Operations
    // =========================================================================

    /// `Add`: `R[A] = RK[B] + RK[C]`
    pub(super) fn op_add(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = numeric::add(&left, &right, frame)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Sub`: `R[A] = RK[B] - RK[C]`
    pub(super) fn op_sub(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = numeric::sub(&left, &right, frame)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Mul`: `R[A] = RK[B] * RK[C]`
    pub(super) fn op_mul(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = numeric::mul(&left, &right, frame)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Div`: `R[A] = RK[B] / RK[C]`
    pub(super) fn op_div(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = numeric::div(&left, &right, frame)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Mod`: `R[A] = RK[B] % RK[C]`
    pub(super) fn op_mod(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = numeric::modulo(&left, &right, frame)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Neg`: `R[A] = -R[B]`
    #[expect(
        clippy::arithmetic_side_effects,
        reason = "[approved] Integer/Ratio negation is safe with arbitrary precision"
    )]
    pub(super) fn op_neg(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let src = decode_b(instruction);

        let operand = self.get_register(src, frame)?;
        let result = match operand {
            Value::Integer(ref int_val) => Value::Integer(-int_val),
            Value::Float(float_val) => Value::Float(-float_val),
            Value::Ratio(ref ratio_val) => Value::Ratio(-ratio_val),
            other @ (Value::Nil | Value::Bool(_) | Value::Symbol(_) | Value::String(_) | _) => {
                return Err(Error::TypeError {
                    expected: "number",
                    got: value_type_name(&other),
                    span: frame.current_span(),
                });
            }
        };
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    // =========================================================================
    // Comparison Operations
    // =========================================================================

    /// `Eq`: `R[A] = RK[B] == RK[C]`
    pub(super) fn op_eq(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = Value::Bool(values_equal(&left, &right));
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Lt`: `R[A] = RK[B] < RK[C]`
    pub(super) fn op_lt(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = numeric::compare(&left, &right, frame, |lv, rv| lv < rv)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Le`: `R[A] = RK[B] <= RK[C]`
    pub(super) fn op_le(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = numeric::compare(&left, &right, frame, |lv, rv| lv <= rv)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Gt`: `R[A] = RK[B] > RK[C]`
    pub(super) fn op_gt(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = numeric::compare(&left, &right, frame, |lv, rv| lv > rv)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Ge`: `R[A] = RK[B] >= RK[C]`
    pub(super) fn op_ge(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let lhs_idx = decode_b(instruction);
        let rhs_idx = decode_c(instruction);

        let left = self.get_rk(lhs_idx, frame)?;
        let right = self.get_rk(rhs_idx, frame)?;
        let result = numeric::compare(&left, &right, frame, |lv, rv| lv >= rv)?;
        self.set_register(dest, result, frame)?;
        Ok(())
    }

    /// `Not`: `R[A] = not R[B]`
    pub(super) fn op_not(&mut self, instruction: u32, frame: &Frame<'_>) -> Result<(), Error> {
        let dest = decode_a(instruction);
        let src = decode_b(instruction);

        let operand = self.get_register(src, frame)?;
        let result = Value::Bool(!operand.is_truthy());
        self.set_register(dest, result, frame)?;
        Ok(())
    }
}
