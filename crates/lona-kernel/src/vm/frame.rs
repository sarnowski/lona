// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Call frame for tracking execution state within a function.

use lonala_compiler::Chunk;
use lonala_parser::Span;

/// A call frame representing execution state within a single function.
///
/// Each function call creates a new frame on the call stack. The frame
/// tracks the bytecode being executed, the program counter, and the
/// base register for this function's local variables.
#[derive(Debug)]
pub struct Frame<'chunk> {
    /// The bytecode chunk being executed.
    chunk: &'chunk Chunk,
    /// Program counter: index of the next instruction to execute.
    pc: usize,
    /// Base register index: where this function's registers start.
    base: usize,
}

impl<'chunk> Frame<'chunk> {
    /// Creates a new call frame.
    ///
    /// # Parameters
    /// - `chunk`: The bytecode to execute
    /// - `base`: The base register index for this frame's locals
    #[inline]
    #[must_use]
    pub const fn new(chunk: &'chunk Chunk, base: usize) -> Self {
        Self { chunk, pc: 0, base }
    }

    /// Returns the chunk being executed.
    #[inline]
    #[must_use]
    pub const fn chunk(&self) -> &'chunk Chunk {
        self.chunk
    }

    /// Returns the current program counter.
    #[inline]
    #[must_use]
    pub const fn pc(&self) -> usize {
        self.pc
    }

    /// Returns the base register index.
    #[inline]
    #[must_use]
    pub const fn base(&self) -> usize {
        self.base
    }

    /// Fetches the next instruction and advances the program counter.
    ///
    /// Returns `None` if the program counter is past the end of the bytecode.
    #[inline]
    pub fn fetch(&mut self) -> Option<u32> {
        let instruction = self.chunk.code().get(self.pc).copied();
        if instruction.is_some() {
            self.pc = self.pc.saturating_add(1);
        }
        instruction
    }

    /// Performs a relative jump by the given signed offset.
    ///
    /// The offset is applied to the current program counter.
    #[inline]
    pub fn jump(&mut self, offset: i16) {
        if offset >= 0 {
            #[expect(
                clippy::as_conversions,
                reason = "i16 >= 0 is safe to convert to usize"
            )]
            let unsigned_offset = offset as usize;
            self.pc = self.pc.saturating_add(unsigned_offset);
        } else {
            #[expect(
                clippy::as_conversions,
                reason = "negating negative i16 gives positive, safe for usize"
            )]
            let unsigned_offset = offset.saturating_neg() as usize;
            self.pc = self.pc.saturating_sub(unsigned_offset);
        }
    }

    /// Returns the source span for the current instruction.
    ///
    /// Returns a default span if the instruction has no span information.
    #[inline]
    #[must_use]
    pub fn current_span(&self) -> Span {
        // The span for the instruction we just executed is at pc - 1
        // since fetch() advances pc after reading
        let instruction_index = self.pc.saturating_sub(1);
        self.chunk
            .span_at(instruction_index)
            .unwrap_or_else(|| Span::new(0_usize, 0_usize))
    }

    /// Returns the source span for the instruction at the given index.
    #[inline]
    #[must_use]
    pub fn span_at(&self, index: usize) -> Span {
        self.chunk
            .span_at(index)
            .unwrap_or_else(|| Span::new(0_usize, 0_usize))
    }

    /// Returns `true` if execution has reached the end of the chunk.
    #[inline]
    #[must_use]
    pub fn is_at_end(&self) -> bool {
        self.pc >= self.chunk.code().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use lonala_compiler::opcode::{Opcode, encode_abc};

    /// Creates a test chunk with the given instructions.
    fn make_chunk(instructions: &[(u32, Span)]) -> Chunk {
        let mut chunk = Chunk::new();
        for &(instr, span) in instructions {
            let _index = chunk.emit(instr, span);
        }
        chunk
    }

    #[test]
    fn new_frame_starts_at_pc_zero() {
        let chunk = make_chunk(&[(
            encode_abc(Opcode::Return, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        )]);
        let frame = Frame::new(&chunk, 0);
        assert_eq!(frame.pc(), 0);
        assert_eq!(frame.base(), 0);
    }

    #[test]
    fn fetch_returns_instruction_and_advances_pc() {
        let instr1 = encode_abc(Opcode::LoadTrue, 0, 0, 0);
        let instr2 = encode_abc(Opcode::Return, 0, 1, 0);
        let chunk = make_chunk(&[
            (instr1, Span::new(0_usize, 4_usize)),
            (instr2, Span::new(4_usize, 10_usize)),
        ]);
        let mut frame = Frame::new(&chunk, 0);

        assert_eq!(frame.fetch(), Some(instr1));
        assert_eq!(frame.pc(), 1);

        assert_eq!(frame.fetch(), Some(instr2));
        assert_eq!(frame.pc(), 2);

        assert_eq!(frame.fetch(), None);
        assert_eq!(frame.pc(), 2); // pc doesn't advance past end
    }

    #[test]
    fn fetch_on_empty_chunk_returns_none() {
        let chunk = Chunk::new();
        let mut frame = Frame::new(&chunk, 0);
        assert_eq!(frame.fetch(), None);
    }

    #[test]
    fn jump_positive_offset() {
        let instructions: Vec<(u32, Span)> = (0_u8..10)
            .map(|i| {
                (
                    encode_abc(Opcode::LoadNil, i, 0, 0),
                    Span::new(usize::from(i), usize::from(i).saturating_add(1)),
                )
            })
            .collect();
        let chunk = make_chunk(&instructions);
        let mut frame = Frame::new(&chunk, 0);

        frame.jump(5);
        assert_eq!(frame.pc(), 5);
    }

    #[test]
    fn jump_negative_offset() {
        let instructions: Vec<(u32, Span)> = (0_u8..10)
            .map(|i| {
                (
                    encode_abc(Opcode::LoadNil, i, 0, 0),
                    Span::new(usize::from(i), usize::from(i).saturating_add(1)),
                )
            })
            .collect();
        let chunk = make_chunk(&instructions);
        let mut frame = Frame::new(&chunk, 0);

        // Move to position 7
        frame.jump(7);
        assert_eq!(frame.pc(), 7);

        // Jump back 3
        frame.jump(-3);
        assert_eq!(frame.pc(), 4);
    }

    #[test]
    fn jump_saturates_at_zero() {
        let chunk = make_chunk(&[(
            encode_abc(Opcode::Return, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        )]);
        let mut frame = Frame::new(&chunk, 0);

        // Try to jump before start - should saturate at 0
        frame.jump(-100);
        assert_eq!(frame.pc(), 0);
    }

    #[test]
    fn current_span_after_fetch() {
        let span1 = Span::new(0_usize, 10_usize);
        let span2 = Span::new(10_usize, 20_usize);
        let chunk = make_chunk(&[
            (encode_abc(Opcode::LoadTrue, 0, 0, 0), span1),
            (encode_abc(Opcode::Return, 0, 1, 0), span2),
        ]);
        let mut frame = Frame::new(&chunk, 0);

        let _instr = frame.fetch();
        assert_eq!(frame.current_span(), span1);

        let _instr = frame.fetch();
        assert_eq!(frame.current_span(), span2);
    }

    #[test]
    fn is_at_end() {
        let chunk = make_chunk(&[(
            encode_abc(Opcode::Return, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        )]);
        let mut frame = Frame::new(&chunk, 0);

        assert!(!frame.is_at_end());
        let _instr = frame.fetch();
        assert!(frame.is_at_end());
    }

    #[test]
    fn base_register_offset() {
        let chunk = make_chunk(&[(
            encode_abc(Opcode::Return, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        )]);
        let frame = Frame::new(&chunk, 10);
        assert_eq!(frame.base(), 10);
    }
}
