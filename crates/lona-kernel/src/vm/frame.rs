// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Call frame for tracking execution state within a function.

use alloc::sync::Arc;

use lona_core::chunk::Chunk;
use lona_core::source::{self, Location as SourceLocation};
use lona_core::span::Span;
use lona_core::value::Value;

/// A call frame representing execution state within a single function.
///
/// Each function call creates a new frame on the call stack. The frame
/// tracks the bytecode being executed, the program counter, the base
/// register for this function's local variables, the source ID for
/// error reporting, and captured values for closures.
#[derive(Debug)]
pub struct Frame<'chunk> {
    /// The bytecode chunk being executed.
    chunk: &'chunk Chunk,
    /// Program counter: index of the next instruction to execute.
    pc: usize,
    /// Base register index: where this function's registers start.
    base: usize,
    /// Source ID for error reporting.
    source: source::Id,
    /// Captured values for this closure. Empty for non-closures.
    upvalues: Arc<[Value]>,
}

impl<'chunk> Frame<'chunk> {
    /// Creates a new call frame without upvalues (for top-level code).
    ///
    /// # Parameters
    /// - `chunk`: The bytecode to execute
    /// - `base`: The base register index for this frame's locals
    /// - `source`: The source ID for error reporting
    #[inline]
    #[must_use]
    pub fn new(chunk: &'chunk Chunk, base: usize, source: source::Id) -> Self {
        Self {
            chunk,
            pc: 0,
            base,
            source,
            upvalues: Arc::from([]),
        }
    }

    /// Creates a new call frame with captured upvalues (for closures).
    ///
    /// # Parameters
    /// - `chunk`: The bytecode to execute
    /// - `base`: The base register index for this frame's locals
    /// - `source`: The source ID for error reporting
    /// - `upvalues`: Captured values from enclosing scopes
    #[inline]
    #[must_use]
    pub const fn with_upvalues(
        chunk: &'chunk Chunk,
        base: usize,
        source: source::Id,
        upvalues: Arc<[Value]>,
    ) -> Self {
        Self {
            chunk,
            pc: 0,
            base,
            source,
            upvalues,
        }
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

    /// Returns the source ID for this frame.
    #[inline]
    #[must_use]
    pub const fn source(&self) -> source::Id {
        self.source
    }

    /// Returns the captured upvalues for this closure.
    #[inline]
    #[must_use]
    pub fn upvalues(&self) -> &[Value] {
        &self.upvalues
    }

    /// Returns the Arc containing the upvalues (for cloning into child frames).
    #[inline]
    #[must_use]
    pub const fn upvalues_arc(&self) -> &Arc<[Value]> {
        &self.upvalues
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
    pub const fn jump(&mut self, offset: i16) {
        if offset >= 0 {
            #[expect(
                clippy::as_conversions,
                clippy::cast_sign_loss,
                reason = "i16 >= 0 is safe to convert to usize"
            )]
            let unsigned_offset = offset as usize;
            self.pc = self.pc.saturating_add(unsigned_offset);
        } else {
            #[expect(
                clippy::as_conversions,
                clippy::cast_sign_loss,
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

    /// Returns the full source location for the current instruction.
    ///
    /// Combines the source ID with the current span for complete error context.
    #[inline]
    #[must_use]
    pub fn current_location(&self) -> SourceLocation {
        SourceLocation::new(self.source, self.current_span())
    }

    /// Returns the source span for the instruction at the given index.
    #[inline]
    #[must_use]
    pub fn span_at(&self, index: usize) -> Span {
        self.chunk
            .span_at(index)
            .unwrap_or_else(|| Span::new(0_usize, 0_usize))
    }

    /// Returns the full source location for the instruction at the given index.
    #[inline]
    #[must_use]
    pub fn location_at(&self, index: usize) -> SourceLocation {
        SourceLocation::new(self.source, self.span_at(index))
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
    use lona_core::opcode::{Opcode, encode_abc};

    /// Creates a test chunk with the given instructions.
    fn make_chunk(instructions: &[(u32, Span)]) -> Chunk {
        let mut chunk = Chunk::new();
        for &(instr, span) in instructions {
            let _index = chunk.emit(instr, span);
        }
        chunk
    }

    /// Default test source ID.
    fn test_source() -> source::Id {
        source::Id::new(0_u32)
    }

    #[test]
    fn new_frame_starts_at_pc_zero() {
        let chunk = make_chunk(&[(
            encode_abc(Opcode::Return, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        )]);
        let frame = Frame::new(&chunk, 0, test_source());
        assert_eq!(frame.pc(), 0);
        assert_eq!(frame.base(), 0);
        assert_eq!(frame.source(), test_source());
    }

    #[test]
    fn fetch_returns_instruction_and_advances_pc() {
        let instr1 = encode_abc(Opcode::LoadTrue, 0, 0, 0);
        let instr2 = encode_abc(Opcode::Return, 0, 1, 0);
        let chunk = make_chunk(&[
            (instr1, Span::new(0_usize, 4_usize)),
            (instr2, Span::new(4_usize, 10_usize)),
        ]);
        let mut frame = Frame::new(&chunk, 0, test_source());

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
        let mut frame = Frame::new(&chunk, 0, test_source());
        assert_eq!(frame.fetch(), None);
    }

    #[test]
    fn jump_positive_offset() {
        let instructions: Vec<(u32, Span)> = (0_u8..10)
            .map(|iter_idx| {
                (
                    encode_abc(Opcode::LoadNil, iter_idx, 0, 0),
                    Span::new(
                        usize::from(iter_idx),
                        usize::from(iter_idx).saturating_add(1),
                    ),
                )
            })
            .collect();
        let chunk = make_chunk(&instructions);
        let mut frame = Frame::new(&chunk, 0, test_source());

        frame.jump(5);
        assert_eq!(frame.pc(), 5);
    }

    #[test]
    fn jump_negative_offset() {
        let instructions: Vec<(u32, Span)> = (0_u8..10)
            .map(|iter_idx| {
                (
                    encode_abc(Opcode::LoadNil, iter_idx, 0, 0),
                    Span::new(
                        usize::from(iter_idx),
                        usize::from(iter_idx).saturating_add(1),
                    ),
                )
            })
            .collect();
        let chunk = make_chunk(&instructions);
        let mut frame = Frame::new(&chunk, 0, test_source());

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
        let mut frame = Frame::new(&chunk, 0, test_source());

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
        let mut frame = Frame::new(&chunk, 0, test_source());

        let _instr = frame.fetch();
        assert_eq!(frame.current_span(), span1);

        let _instr = frame.fetch();
        assert_eq!(frame.current_span(), span2);
    }

    #[test]
    fn current_location_combines_source_and_span() {
        let span = Span::new(5_usize, 15_usize);
        let source_id = source::Id::new(42_u32);
        let chunk = make_chunk(&[(encode_abc(Opcode::LoadTrue, 0, 0, 0), span)]);
        let mut frame = Frame::new(&chunk, 0, source_id);

        let _instr = frame.fetch();
        let location = frame.current_location();

        assert_eq!(location.source, source_id);
        assert_eq!(location.span, span);
    }

    #[test]
    fn is_at_end() {
        let chunk = make_chunk(&[(
            encode_abc(Opcode::Return, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        )]);
        let mut frame = Frame::new(&chunk, 0, test_source());

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
        let frame = Frame::new(&chunk, 10, test_source());
        assert_eq!(frame.base(), 10);
    }

    #[test]
    fn location_at_specific_index() {
        let span1 = Span::new(0_usize, 5_usize);
        let span2 = Span::new(5_usize, 10_usize);
        let source_id = source::Id::new(7_u32);
        let chunk = make_chunk(&[
            (encode_abc(Opcode::LoadNil, 0, 0, 0), span1),
            (encode_abc(Opcode::Return, 0, 0, 0), span2),
        ]);
        let frame = Frame::new(&chunk, 0, source_id);

        let loc0 = frame.location_at(0);
        assert_eq!(loc0.source, source_id);
        assert_eq!(loc0.span, span1);

        let loc1 = frame.location_at(1);
        assert_eq!(loc1.source, source_id);
        assert_eq!(loc1.span, span2);
    }

    #[test]
    fn new_frame_has_empty_upvalues() {
        let chunk = make_chunk(&[(
            encode_abc(Opcode::Return, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        )]);
        let frame = Frame::new(&chunk, 0, test_source());
        assert!(frame.upvalues().is_empty());
    }

    #[test]
    fn frame_with_upvalues_stores_values() {
        use lona_core::integer::Integer;

        let chunk = make_chunk(&[(
            encode_abc(Opcode::Return, 0, 0, 0),
            Span::new(0_usize, 1_usize),
        )]);
        let upvalues: Arc<[Value]> = Arc::from([
            Value::Integer(Integer::from_i64(42)),
            Value::Bool(true),
            Value::Nil,
        ]);
        let frame = Frame::with_upvalues(&chunk, 0, test_source(), upvalues.clone());

        assert_eq!(frame.upvalues().len(), 3);
        assert_eq!(
            frame.upvalues().first(),
            Some(&Value::Integer(Integer::from_i64(42)))
        );
        assert!(Arc::ptr_eq(frame.upvalues_arc(), &upvalues));
    }
}
