// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tag constants for the BEAM-style tagged word representation.
//!
//! Lonala uses a tagged word representation where type information is encoded
//! in the low bits of a 64-bit machine word. This module defines all tag constants.
//!
//! See `docs/architecture/term-representation.md` for the full specification.

/// Primary tags (bits 0-1).
///
/// The primary tag determines the basic category of a value:
/// - `HEADER`: Heap object marker (only appears at start of heap objects)
/// - `LIST`: Pointer to a pair cell (headerless, 16 bytes)
/// - `BOXED`: Pointer to a heap object with a header
/// - `IMMEDIATE`: Value encoded directly in the word
pub mod primary {
    /// Header word marker - only appears at the start of heap objects.
    pub const HEADER: u64 = 0b00;
    /// Pointer to a pair cell (cons cell).
    pub const LIST: u64 = 0b01;
    /// Pointer to a boxed heap object with header.
    pub const BOXED: u64 = 0b10;
    /// Immediate value encoded in the word.
    pub const IMMEDIATE: u64 = 0b11;
    /// Mask for extracting primary tag (bits 0-1).
    pub const MASK: u64 = 0b11;
}

/// Immediate subtags (bits 0-3, when primary tag is IMMEDIATE).
///
/// When the primary tag is `11` (IMMEDIATE), the subtag in bits 2-3
/// distinguishes different immediate types.
pub mod immediate {
    /// 60-bit signed small integer.
    pub const SMALL_INT: u64 = 0b0011; // primary=11, subtag=00
    /// Interned symbol (index into realm table).
    pub const SYMBOL: u64 = 0b0111; // primary=11, subtag=01
    /// Interned keyword (index into realm table).
    pub const KEYWORD: u64 = 0b1011; // primary=11, subtag=10
    /// Special values (nil, true, false, unbound).
    pub const SPECIAL: u64 = 0b1111; // primary=11, subtag=11
    /// Mask for extracting immediate tag (bits 0-3).
    pub const MASK: u64 = 0b1111;
}

/// Special value encodings (when immediate subtag is SPECIAL).
///
/// Special values have a tertiary tag in bits 4-7.
pub mod special {
    /// The nil value (empty list, falsy).
    pub const NIL: u64 = 0x0F; // 0000_1111
    /// Boolean true.
    pub const TRUE: u64 = 0x1F; // 0001_1111
    /// Boolean false.
    pub const FALSE: u64 = 0x2F; // 0010_1111
    /// Uninitialized var sentinel.
    pub const UNBOUND: u64 = 0x3F; // 0011_1111
    /// Native function (intrinsic) with ID in upper bits.
    /// Format: bits 0-7 = 0x4F, bits 8-23 = intrinsic ID (16 bits).
    pub const NATIVE_FN_PREFIX: u64 = 0x4F; // 0100_1111
    /// Mask for extracting native function prefix (bits 0-7).
    pub const NATIVE_FN_MASK: u64 = 0xFF;
}

/// Object tags for heap objects (bits 2-9 of header word).
///
/// Every boxed heap object starts with a header word that contains
/// an object tag identifying the type of the object.
pub mod object {
    /// Fixed-size indexed tuple.
    pub const TUPLE: u8 = 0x00;
    /// Persistent vector with capacity.
    pub const VECTOR: u8 = 0x01;
    /// Key-value map (association list).
    pub const MAP: u8 = 0x02;
    /// UTF-8 string.
    pub const STRING: u8 = 0x03;
    /// Raw byte sequence.
    pub const BINARY: u8 = 0x04;
    /// Arbitrary precision integer.
    pub const BIGNUM: u8 = 0x05;
    /// 64-bit IEEE 754 float.
    pub const FLOAT: u8 = 0x06;
    /// Compiled function bytecode.
    pub const FUN: u8 = 0x07;
    /// Function with captured environment.
    pub const CLOSURE: u8 = 0x08;
    /// Process identifier.
    pub const PID: u8 = 0x09;
    /// Unique reference.
    pub const REF: u8 = 0x0A;
    /// Reference to large binary (off-heap).
    pub const PROCBIN: u8 = 0x0B;
    /// Sub-binary view into existing binary.
    pub const SUBBIN: u8 = 0x0C;
    /// Namespace.
    pub const NAMESPACE: u8 = 0x0D;
    /// Var slot.
    pub const VAR: u8 = 0x0E;
    /// Symbol string storage (used by realm interning for string lookup).
    /// Symbols themselves are immediate values; this type stores their string data.
    pub const SYMBOL: u8 = 0x0F;
    /// Keyword string storage (used by realm interning for string lookup).
    /// Keywords themselves are immediate values; this type stores their string data.
    pub const KEYWORD: u8 = 0x10;
    /// Forwarding pointer (GC only).
    pub const FORWARD: u8 = 0xFF;
}

#[cfg(test)]
mod tag_test;
