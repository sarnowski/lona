// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Collection literal compilation (tuples and maps).

#[cfg(any(test, feature = "std"))]
use std::vec::Vec;

#[cfg(not(any(test, feature = "std")))]
use alloc::vec::Vec;

use crate::bytecode::{encode_abc, op};
use crate::platform::MemorySpace;
use crate::value::Value;

use super::{CompileError, Compiler};

impl<M: MemorySpace> Compiler<'_, M> {
    /// Compile a tuple literal.
    ///
    /// `[a b c]` evaluates each element and builds a tuple.
    pub(super) fn compile_tuple(
        &mut self,
        tuple: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Get tuple length and elements
        let len = self
            .proc
            .read_tuple_len(self.mem, tuple)
            .ok_or(CompileError::InvalidSyntax)?;

        if len == 0 {
            // Empty tuple - emit BUILD_TUPLE with 0 elements
            self.chunk.emit(encode_abc(op::BUILD_TUPLE, target, 0, 0));
            return Ok(temp_base);
        }

        // Allocate temp registers for elements
        let elem_count = len as u8;
        let next_temp = temp_base
            .checked_add(elem_count)
            .ok_or(CompileError::ExpressionTooComplex)?;

        // Compile each element to a temp register
        let mut current_next_temp = next_temp;
        for i in 0..len {
            let elem = self
                .proc
                .read_tuple_element(self.mem, tuple, i)
                .ok_or(CompileError::InvalidSyntax)?;
            let temp_reg = temp_base
                .checked_add(i as u8)
                .ok_or(CompileError::ExpressionTooComplex)?;
            current_next_temp = self.compile_expr(elem, temp_reg, current_next_temp)?;
        }

        // Emit BUILD_TUPLE: target := [temp_base..temp_base+len-1]
        self.chunk.emit(encode_abc(
            op::BUILD_TUPLE,
            target,
            u16::from(temp_base),
            len as u16,
        ));

        Ok(current_next_temp)
    }

    /// Compile a vector literal.
    ///
    /// `{a b c}` evaluates each element and builds a vector.
    pub(super) fn compile_vector(
        &mut self,
        vector: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Vectors share the same memory layout as tuples
        let len = self
            .proc
            .read_tuple_len(self.mem, vector)
            .ok_or(CompileError::InvalidSyntax)?;

        if len == 0 {
            // Empty vector - emit BUILD_VECTOR with 0 elements
            self.chunk.emit(encode_abc(op::BUILD_VECTOR, target, 0, 0));
            return Ok(temp_base);
        }

        // Allocate temp registers for elements
        let elem_count = len as u8;
        let next_temp = temp_base
            .checked_add(elem_count)
            .ok_or(CompileError::ExpressionTooComplex)?;

        // Compile each element to a temp register
        let mut current_next_temp = next_temp;
        for i in 0..len {
            let elem = self
                .proc
                .read_tuple_element(self.mem, vector, i)
                .ok_or(CompileError::InvalidSyntax)?;
            let temp_reg = temp_base
                .checked_add(i as u8)
                .ok_or(CompileError::ExpressionTooComplex)?;
            current_next_temp = self.compile_expr(elem, temp_reg, current_next_temp)?;
        }

        // Emit BUILD_VECTOR: target := {temp_base..temp_base+len-1}
        self.chunk.emit(encode_abc(
            op::BUILD_VECTOR,
            target,
            u16::from(temp_base),
            len as u16,
        ));

        Ok(current_next_temp)
    }

    /// Compile a map literal.
    ///
    /// `%{:a 1 :b 2}` evaluates each key and value, then builds a map.
    pub(super) fn compile_map(
        &mut self,
        map: Value,
        target: u8,
        temp_base: u8,
    ) -> Result<u8, CompileError> {
        // Read the map's entries (association list)
        let map_val = self
            .proc
            .read_map(self.mem, map)
            .ok_or(CompileError::InvalidSyntax)?;

        // Count entries and collect key-value pairs
        let mut entries = Vec::new();
        let mut current = map_val.entries;
        while let Some(pair) = self.proc.read_pair(self.mem, current) {
            // Each pair.first is a [key value] tuple
            let kv = pair.first;
            let key = self
                .proc
                .read_tuple_element(self.mem, kv, 0)
                .ok_or(CompileError::InvalidSyntax)?;
            let val = self
                .proc
                .read_tuple_element(self.mem, kv, 1)
                .ok_or(CompileError::InvalidSyntax)?;
            entries.push((key, val));
            current = pair.rest;
        }

        if entries.is_empty() {
            // Empty map - emit BUILD_MAP with 0 pairs
            self.chunk.emit(encode_abc(op::BUILD_MAP, target, 0, 0));
            return Ok(temp_base);
        }

        // Each pair needs 2 registers (key, value)
        let pair_count = entries.len() as u8;
        let elem_count = pair_count
            .checked_mul(2)
            .ok_or(CompileError::ExpressionTooComplex)?;
        let next_temp = temp_base
            .checked_add(elem_count)
            .ok_or(CompileError::ExpressionTooComplex)?;

        // Compile each key and value to temp registers
        let mut current_next_temp = next_temp;
        for (i, (key, val)) in entries.iter().enumerate() {
            let key_reg = temp_base
                .checked_add((i * 2) as u8)
                .ok_or(CompileError::ExpressionTooComplex)?;
            let val_reg = temp_base
                .checked_add((i * 2 + 1) as u8)
                .ok_or(CompileError::ExpressionTooComplex)?;
            current_next_temp = self.compile_expr(*key, key_reg, current_next_temp)?;
            current_next_temp = self.compile_expr(*val, val_reg, current_next_temp)?;
        }

        // Emit BUILD_MAP: target := %{temp_base..temp_base+pair_count*2-1}
        self.chunk.emit(encode_abc(
            op::BUILD_MAP,
            target,
            u16::from(temp_base),
            u16::from(pair_count),
        ));

        Ok(current_next_temp)
    }
}
