// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! N-ary operator compilation.
//!
//! This module handles compilation of operators with more than two arguments:
//! - N-ary arithmetic: `(+ a b c ...)` chains binary operations left-to-right
//! - N-ary comparison: `(= a b c ...)` chains pairwise comparisons with short-circuit

use alloc::vec::Vec;

use lona_core::opcode::{Opcode, encode_abc, encode_asbx, rk_register};
use lona_core::span::Span;
use lonala_parser::{Ast, Spanned};

use super::{Compiler, ExprResult};
use crate::error::{Error, Kind as ErrorKind};

impl Compiler<'_, '_, '_> {
    /// Compiles n-ary arithmetic by chaining binary operations left-to-right.
    pub(super) fn compile_nary_arithmetic(
        &mut self,
        opcode: Opcode,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let arg1 = elements
            .get(1_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
        let arg2 = elements
            .get(2_usize)
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

        let checkpoint = self.next_register;
        let rk_b = self.try_compile_rk_operand(arg1)?;
        let rk_c = self.try_compile_rk_operand(arg2)?;

        self.free_registers_to(checkpoint);
        let acc = self.alloc_register(span)?;
        self.chunk.emit(encode_abc(opcode, acc, rk_b, rk_c), span);

        // Chain remaining arguments: acc = acc op arg
        for arg in elements.get(3_usize..).unwrap_or(&[]) {
            let arg_checkpoint = self.next_register;
            let rk_arg = self.try_compile_rk_operand(arg)?;
            self.free_registers_to(arg_checkpoint);

            let rk_acc = rk_register(acc)
                .ok_or_else(|| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
            self.chunk
                .emit(encode_abc(opcode, acc, rk_acc, rk_arg), span);
        }

        Ok(ExprResult { register: acc })
    }

    /// Compiles n-ary comparison by chaining binary comparisons with short-circuit AND.
    ///
    /// `(= a b c)` compiles to: `(and (= a b) (= b c))`
    ///
    /// Strategy:
    /// 1. Evaluate first pair (a, b), check comparison
    /// 2. If false, jump to end with false result
    /// 3. For each subsequent pair (b, c), (c, d), etc:
    ///    - Reuse previous right operand as left operand
    ///    - Check comparison
    ///    - If false, jump to end
    /// 4. If all pass, result is true
    pub(super) fn compile_nary_comparison(
        &mut self,
        opcode: Opcode,
        elements: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        let args = elements.get(1_usize..).unwrap_or(&[]);
        if args.len() < 2_usize {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "comparison",
                    message: "requires at least two arguments",
                },
                self.location(span),
            ));
        }

        // We'll compile each argument into its own register and keep them around
        // for pairwise comparison. This uses more registers but simplifies the logic.
        let checkpoint = self.next_register;

        // Compile all arguments into consecutive registers
        let mut arg_registers = Vec::new();
        for arg in args {
            let result = self.compile_expr(arg)?;
            arg_registers.push(result.register);
        }

        // Destination register for the final boolean result
        let dest = self.alloc_register(span)?;

        // Collect jump offsets that need to be patched to the false case
        let mut false_jumps = Vec::new();

        // Compare adjacent pairs: (a, b), (b, c), (c, d), ...
        for pair in arg_registers.windows(2_usize) {
            let left_reg = *pair
                .first()
                .ok_or_else(|| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
            let right_reg = *pair
                .get(1_usize)
                .ok_or_else(|| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;

            let rk_left = rk_register(left_reg)
                .ok_or_else(|| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
            let rk_right = rk_register(right_reg)
                .ok_or_else(|| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;

            // Emit comparison into dest register
            self.chunk
                .emit(encode_abc(opcode, dest, rk_left, rk_right), span);

            // If comparison is false, jump to false case
            // JumpIfNot: if not R[A] then PC += sBx
            let jump_offset = self.chunk.code().len();
            // Use placeholder offset 0, will be patched later
            self.chunk
                .emit(encode_asbx(Opcode::JumpIfNot, dest, 0), span);
            false_jumps.push(jump_offset);
        }

        // All comparisons passed: load true into dest
        self.chunk
            .emit(encode_abc(Opcode::LoadTrue, dest, 0, 0), span);

        // Jump over the false case
        let skip_false_offset = self.chunk.code().len();
        self.chunk.emit(encode_asbx(Opcode::Jump, 0, 0), span);

        // Record current offset for patching false case
        let false_case_offset = self.chunk.code().len();

        // Load false into dest (jumped to from false_jumps)
        self.chunk
            .emit(encode_abc(Opcode::LoadFalse, dest, 0, 0), span);

        // End label - where skip_false should jump to
        let end_offset = self.chunk.code().len();

        // Patch all false jumps to jump to false_case_offset
        for jump_offset in false_jumps {
            let false_offset_i32 = i32::try_from(false_case_offset)
                .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
            let jump_plus_one = i32::try_from(jump_offset.saturating_add(1))
                .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
            let relative_jump = i16::try_from(false_offset_i32.saturating_sub(jump_plus_one))
                .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
            self.chunk.patch(
                jump_offset,
                encode_asbx(Opcode::JumpIfNot, dest, relative_jump),
            );
        }

        // Patch skip_false to jump to end
        let end_offset_i32 = i32::try_from(end_offset)
            .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
        let skip_plus_one = i32::try_from(skip_false_offset.saturating_add(1))
            .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
        let skip_relative = i16::try_from(end_offset_i32.saturating_sub(skip_plus_one))
            .map_err(|_err| Error::new(ErrorKind::TooManyRegisters, self.location(span)))?;
        self.chunk.patch(
            skip_false_offset,
            encode_asbx(Opcode::Jump, 0, skip_relative),
        );

        // Free temporary argument registers
        self.free_registers_to(checkpoint);

        // Move result to final register if needed
        let final_dest = self.alloc_register(span)?;
        if final_dest != dest {
            self.chunk
                .emit(encode_abc(Opcode::Move, final_dest, dest, 0), span);
        }

        Ok(ExprResult {
            register: final_dest,
        })
    }
}
