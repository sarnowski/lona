// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! BEAM-style tagged word representation for Lonala values.
//!
//! A `Term` is an 8-byte value that encodes type information in the low bits.
//! This enables efficient type dispatch without pointer dereference and supports
//! both immediate values (stored inline) and heap-allocated boxed values.
//!
//! See `docs/architecture/term-representation.md` for the full specification.
//!
//! # Layout
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┬──────┐
//! │                     Payload (62 bits)                           │ Tag  │
//! │                                                                 │ (2b) │
//! └─────────────────────────────────────────────────────────────────┴──────┘
//! ```
//!
//! # Primary Tags
//!
//! - `00` = HEADER (heap object marker, never in registers)
//! - `01` = LIST (pointer to pair cell)
//! - `10` = BOXED (pointer to heap object with header)
//! - `11` = IMMEDIATE (value encoded in word)

pub mod header;
pub mod heap;
pub mod pair;
pub mod printer;
pub mod tag;

#[cfg(test)]
mod term_test;

use core::fmt;

use tag::{immediate, primary, special};

/// A tagged word representing any Lonala value.
///
/// This is the fundamental value type in the VM. It is 8 bytes on 64-bit systems
/// and encodes type information in the low 2 bits.
///
/// # Safety
///
/// When a `Term` contains a pointer (LIST or BOXED tag), the pointer is only
/// valid within the memory space where it was allocated. Copying a Term to a
/// different memory space requires proper handling (e.g., deep copy with pointer
/// remapping during GC).
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Term(u64);

/// Default for Term is NIL, not zero (which would have HEADER tag).
impl Default for Term {
    fn default() -> Self {
        Self::NIL
    }
}

// Compile-time assertion that Term is 8 bytes
const _: () = assert!(core::mem::size_of::<Term>() == 8);

impl Term {
    // ========================================================================
    // Constants - Special Values
    // ========================================================================

    /// The nil value (empty list, falsy).
    pub const NIL: Self = Self(special::NIL);

    /// Boolean true.
    pub const TRUE: Self = Self(special::TRUE);

    /// Boolean false.
    pub const FALSE: Self = Self(special::FALSE);

    /// Uninitialized var sentinel.
    pub const UNBOUND: Self = Self(special::UNBOUND);

    // ========================================================================
    // Primary Tag Operations
    // ========================================================================

    /// Create a Term from a raw u64 value.
    ///
    /// # Safety
    ///
    /// The caller must ensure the raw value is a valid Term encoding.
    /// Invalid encodings may cause undefined behavior when the Term is used.
    #[inline]
    #[must_use]
    pub const unsafe fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Get the raw u64 value.
    #[inline]
    #[must_use]
    pub const fn as_raw(self) -> u64 {
        self.0
    }

    /// Extract the primary tag (bits 0-1).
    #[inline]
    #[must_use]
    pub const fn primary_tag(self) -> u64 {
        self.0 & primary::MASK
    }

    /// Check if this is an immediate value (encoded in the word).
    #[inline]
    #[must_use]
    pub const fn is_immediate(self) -> bool {
        self.primary_tag() == primary::IMMEDIATE
    }

    /// Check if this is a list pointer (points to a pair cell).
    #[inline]
    #[must_use]
    pub const fn is_list(self) -> bool {
        self.primary_tag() == primary::LIST
    }

    /// Check if this is a boxed pointer (points to a heap object with header).
    #[inline]
    #[must_use]
    pub const fn is_boxed(self) -> bool {
        self.primary_tag() == primary::BOXED
    }

    /// Check if this is a header word (only valid at start of heap objects).
    #[inline]
    #[must_use]
    pub const fn is_header(self) -> bool {
        self.primary_tag() == primary::HEADER
    }

    /// Extract the pointer value (masks off tag bits).
    ///
    /// This is valid for LIST and BOXED tags. The returned pointer is
    /// 8-byte aligned.
    #[inline]
    #[must_use]
    pub const fn to_ptr(self) -> *const u8 {
        (self.0 & !primary::MASK) as *const u8
    }

    /// Extract the pointer as a mutable pointer.
    #[inline]
    #[must_use]
    pub const fn to_ptr_mut(self) -> *mut u8 {
        (self.0 & !primary::MASK) as *mut u8
    }

    /// Extract the pointer as a `Vaddr`.
    ///
    /// This is valid for LIST and BOXED tags. For immediate values,
    /// the result is meaningless.
    #[inline]
    #[must_use]
    pub const fn to_vaddr(self) -> crate::Vaddr {
        crate::Vaddr::new(self.0 & !primary::MASK)
    }

    /// Create a boxed Term from a `Vaddr`.
    ///
    /// The address must point to a valid, 8-byte aligned heap object with a header.
    #[inline]
    #[must_use]
    pub const fn boxed_vaddr(addr: crate::Vaddr) -> Self {
        Self(addr.as_u64() | primary::BOXED)
    }

    /// Create a list Term from a `Vaddr`.
    ///
    /// The address must point to a valid, 8-byte aligned pair cell.
    #[inline]
    #[must_use]
    pub const fn list_vaddr(addr: crate::Vaddr) -> Self {
        Self(addr.as_u64() | primary::LIST)
    }

    // ========================================================================
    // Immediate Tag Operations
    // ========================================================================

    /// Extract the immediate tag (bits 0-3, only valid when `is_immediate`).
    #[inline]
    #[must_use]
    pub const fn immediate_tag(self) -> u64 {
        self.0 & immediate::MASK
    }

    /// Check if this is a small integer.
    #[inline]
    #[must_use]
    pub const fn is_small_int(self) -> bool {
        self.immediate_tag() == immediate::SMALL_INT
    }

    /// Check if this is an interned symbol.
    #[inline]
    #[must_use]
    pub const fn is_symbol(self) -> bool {
        self.immediate_tag() == immediate::SYMBOL
    }

    /// Check if this is an interned keyword.
    #[inline]
    #[must_use]
    pub const fn is_keyword(self) -> bool {
        self.immediate_tag() == immediate::KEYWORD
    }

    /// Check if this is a special value (nil, true, false, unbound).
    #[inline]
    #[must_use]
    pub const fn is_special(self) -> bool {
        self.immediate_tag() == immediate::SPECIAL
    }

    // ========================================================================
    // Special Value Checks
    // ========================================================================

    /// Check if this is nil.
    #[inline]
    #[must_use]
    pub const fn is_nil(self) -> bool {
        self.0 == special::NIL
    }

    /// Check if this is true.
    #[inline]
    #[must_use]
    pub const fn is_true(self) -> bool {
        self.0 == special::TRUE
    }

    /// Check if this is false.
    #[inline]
    #[must_use]
    pub const fn is_false(self) -> bool {
        self.0 == special::FALSE
    }

    /// Check if this is a boolean (true or false).
    #[inline]
    #[must_use]
    pub const fn is_boolean(self) -> bool {
        self.is_true() || self.is_false()
    }

    /// Check if this is the unbound sentinel.
    #[inline]
    #[must_use]
    pub const fn is_unbound(self) -> bool {
        self.0 == special::UNBOUND
    }

    /// Check if this value is truthy (not nil and not false).
    #[inline]
    #[must_use]
    pub const fn is_truthy(self) -> bool {
        !self.is_nil() && !self.is_false()
    }

    /// Check if this is a native function (intrinsic).
    #[inline]
    #[must_use]
    pub const fn is_native_fn(self) -> bool {
        (self.0 & special::NATIVE_FN_MASK) == special::NATIVE_FN_PREFIX
    }

    // ========================================================================
    // Small Integer Operations
    // ========================================================================

    /// Create a small integer Term.
    ///
    /// Returns `None` if the value doesn't fit in 60 bits (signed).
    /// Values outside this range should be promoted to bignum.
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_sign_loss,
        reason = "i64 to u64 preserves bit pattern for tagged word encoding per term-representation.md"
    )]
    pub const fn small_int(value: i64) -> Option<Self> {
        // Check if value fits in 60 bits (signed)
        // For a 60-bit signed integer, the valid range is -2^59 to 2^59 - 1
        // We check by shifting right by 59 bits: result should be 0 (positive)
        // or -1 (negative, all 1s after sign extension)
        let shifted = value >> 59;
        if shifted != 0 && shifted != -1 {
            return None; // Too large, needs bignum
        }

        // Encode: shift value left by 4, OR with SMALL_INT tag
        // The cast to u64 preserves the bit pattern for signed integers
        Some(Self(((value as u64) << 4) | immediate::SMALL_INT))
    }

    /// Extract small integer value.
    ///
    /// Returns `None` if this is not a small integer.
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_possible_wrap,
        reason = "u64 to i64 for arithmetic right shift to preserve sign per term-representation.md"
    )]
    pub const fn as_small_int(self) -> Option<i64> {
        if self.is_small_int() {
            // Arithmetic shift right by 4 to preserve sign
            // The cast to i64 enables sign extension during the shift
            Some((self.0 as i64) >> 4)
        } else {
            None
        }
    }

    // ========================================================================
    // Symbol Operations
    // ========================================================================

    /// Create an interned symbol Term from a table index.
    #[inline]
    #[must_use]
    pub const fn symbol(index: u32) -> Self {
        Self(((index as u64) << 4) | immediate::SYMBOL)
    }

    /// Extract symbol table index.
    ///
    /// Returns `None` if this is not a symbol.
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "upper bits are guaranteed zero since we shifted u32 left by 4 during encoding"
    )]
    pub const fn as_symbol_index(self) -> Option<u32> {
        if self.is_symbol() {
            // Index fits in 32 bits by design (we shifted u32 left by 4)
            Some((self.0 >> 4) as u32)
        } else {
            None
        }
    }

    // ========================================================================
    // Keyword Operations
    // ========================================================================

    /// Create an interned keyword Term from a table index.
    #[inline]
    #[must_use]
    pub const fn keyword(index: u32) -> Self {
        Self(((index as u64) << 4) | immediate::KEYWORD)
    }

    /// Extract keyword table index.
    ///
    /// Returns `None` if this is not a keyword.
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "upper bits are guaranteed zero since we shifted u32 left by 4 during encoding"
    )]
    pub const fn as_keyword_index(self) -> Option<u32> {
        if self.is_keyword() {
            // Index fits in 32 bits by design (we shifted u32 left by 4)
            Some((self.0 >> 4) as u32)
        } else {
            None
        }
    }

    // ========================================================================
    // Boolean Operations
    // ========================================================================

    /// Create a boolean Term.
    #[inline]
    #[must_use]
    pub const fn bool(value: bool) -> Self {
        if value { Self::TRUE } else { Self::FALSE }
    }

    /// Extract boolean value.
    ///
    /// Returns `None` if this is not a boolean.
    #[inline]
    #[must_use]
    pub const fn as_bool(self) -> Option<bool> {
        if self.is_true() {
            Some(true)
        } else if self.is_false() {
            Some(false)
        } else {
            None
        }
    }

    // ========================================================================
    // Native Function Operations
    // ========================================================================

    /// Create a native function (intrinsic) Term.
    ///
    /// The ID identifies which intrinsic function this represents.
    #[inline]
    #[must_use]
    pub const fn native_fn(id: u16) -> Self {
        // Encode: bits 0-7 = NATIVE_FN_PREFIX, bits 8-23 = ID
        Self(special::NATIVE_FN_PREFIX | ((id as u64) << 8))
    }

    /// Extract native function ID.
    ///
    /// Returns `None` if this is not a native function.
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "upper bits are guaranteed zero since we shifted u16 left by 8 during encoding"
    )]
    pub const fn as_native_fn_id(self) -> Option<u16> {
        if self.is_native_fn() {
            Some((self.0 >> 8) as u16)
        } else {
            None
        }
    }

    // ========================================================================
    // Type Names
    // ========================================================================

    /// Get the type name for error messages.
    ///
    /// Note: For boxed values, this returns a generic name. Use `object_type_name`
    /// with a memory space to get the specific heap object type.
    #[must_use]
    pub const fn type_name(self) -> &'static str {
        if self.is_nil() {
            "nil"
        } else if self.is_boolean() {
            "boolean"
        } else if self.is_small_int() {
            "integer"
        } else if self.is_symbol() {
            "symbol"
        } else if self.is_keyword() {
            "keyword"
        } else if self.is_unbound() {
            "unbound"
        } else if self.is_list() {
            "pair"
        } else if self.is_boxed() {
            "boxed"
        } else if self.is_header() {
            "header"
        } else {
            "unknown"
        }
    }
}

impl fmt::Debug for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_nil() {
            write!(f, "Term::NIL")
        } else if self.is_true() {
            write!(f, "Term::TRUE")
        } else if self.is_false() {
            write!(f, "Term::FALSE")
        } else if self.is_unbound() {
            write!(f, "Term::UNBOUND")
        } else if let Some(n) = self.as_small_int() {
            write!(f, "Term::small_int({n})")
        } else if let Some(idx) = self.as_symbol_index() {
            write!(f, "Term::symbol({idx})")
        } else if let Some(idx) = self.as_keyword_index() {
            write!(f, "Term::keyword({idx})")
        } else if self.is_list() {
            write!(f, "Term::list({:p})", self.to_ptr())
        } else if self.is_boxed() {
            write!(f, "Term::boxed({:p})", self.to_ptr())
        } else if self.is_header() {
            write!(f, "Term::header({:#018x})", self.0)
        } else {
            write!(f, "Term({:#018x})", self.0)
        }
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_nil() {
            write!(f, "nil")
        } else if self.is_true() {
            write!(f, "true")
        } else if self.is_false() {
            write!(f, "false")
        } else if self.is_unbound() {
            write!(f, "#<unbound>")
        } else if let Some(n) = self.as_small_int() {
            write!(f, "{n}")
        } else if self.as_symbol_index().is_some() {
            write!(f, "#<symbol>")
        } else if self.as_keyword_index().is_some() {
            write!(f, "#<keyword>")
        } else if self.is_list() {
            write!(f, "#<pair>")
        } else if self.is_boxed() {
            write!(f, "#<boxed>")
        } else {
            write!(f, "#<unknown>")
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Align a size up to the next 8-byte boundary.
///
/// This is used throughout the term module for memory alignment calculations.
#[inline]
#[must_use]
pub const fn align8(n: usize) -> usize {
    (n + 7) & !7
}

/// Extract a `Vaddr` from a Term (works for both boxed and list terms).
///
/// This function is provided for convenience when working with the VM's
/// memory operations that use `Vaddr` for addressing.
#[inline]
#[must_use]
pub const fn term_to_vaddr(term: Term) -> crate::Vaddr {
    term.to_vaddr()
}
