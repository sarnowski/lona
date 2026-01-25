// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Header words for heap-allocated boxed values.
//!
//! Every boxed heap object (those pointed to by BOXED-tagged Terms) starts
//! with an 8-byte header word. The header encodes:
//!
//! - Primary tag (bits 0-1): Always `00` (HEADER)
//! - Object tag (bits 2-9): Type of heap object (TUPLE, VECTOR, STRING, etc.)
//! - Arity (bits 10-63): Size/count information, interpretation varies by type
//!
//! Layout:
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┬────────┬────┐
//! │                      Arity (54 bits)                            │ObjTag  │ 00 │
//! │                                                                 │(8 bits)│    │
//! └─────────────────────────────────────────────────────────────────┴────────┴────┘
//! ```
//!
//! During GC, a forwarding header has object tag `FORWARD` and stores the
//! new address in the arity field (shifted to recover the 8-byte aligned address).

#[cfg(test)]
mod header_test;

use super::Term;
use super::tag::{object, primary};

/// A header word at the start of a heap object.
///
/// The header is 8 bytes and encodes:
/// - Primary tag (bits 0-1): Always `00` (HEADER)
/// - Object tag (bits 2-9): Type of object
/// - Arity (bits 10-63): Size/count, meaning varies by object type
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Header(u64);

// Compile-time assertion that Header is exactly 8 bytes
const _: () = assert!(core::mem::size_of::<Header>() == 8);

impl Header {
    /// Create a header with object tag and arity.
    ///
    /// # Arguments
    /// * `object_tag` - The type of heap object (from `tag::object`)
    /// * `arity` - Size/count information (interpretation depends on object type)
    #[inline]
    #[must_use]
    pub const fn new(object_tag: u8, arity: u64) -> Self {
        Self((arity << 10) | ((object_tag as u64) << 2) | primary::HEADER)
    }

    /// Get the raw u64 value.
    #[inline]
    #[must_use]
    pub const fn as_raw(self) -> u64 {
        self.0
    }

    /// Create a Header from a raw u64 value.
    #[inline]
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Get the object tag (type of heap object).
    #[inline]
    #[must_use]
    pub const fn object_tag(self) -> u8 {
        ((self.0 >> 2) & 0xFF) as u8
    }

    /// Get the arity/size field.
    ///
    /// The meaning of arity depends on the object type:
    /// - TUPLE: number of elements
    /// - VECTOR: capacity
    /// - STRING: byte length
    /// - MAP: entry count
    /// - CLOSURE: capture count
    /// - FUN: total object size in words
    /// - etc.
    #[inline]
    #[must_use]
    pub const fn arity(self) -> u64 {
        self.0 >> 10
    }

    /// Check if this is a forwarding pointer.
    #[inline]
    #[must_use]
    pub const fn is_forward(self) -> bool {
        self.object_tag() == object::FORWARD
    }

    /// Get forwarding address (if this is a forward header).
    ///
    /// # Safety
    ///
    /// Caller must ensure this is a forwarding header (`is_forward()` returns true).
    #[inline]
    #[must_use]
    pub const fn forward_address(self) -> *const u8 {
        debug_assert!(self.object_tag() == object::FORWARD);
        // Arity field contains address >> 3, so we shift left by 3 to recover
        ((self.0 >> 10) << 3) as *const u8
    }

    /// Create a forwarding header pointing to a new address.
    ///
    /// The address must be 8-byte aligned (low 3 bits are zero).
    #[inline]
    #[must_use]
    pub fn forward(new_addr: *const u8) -> Self {
        debug_assert!(
            (new_addr as u64).trailing_zeros() >= 3,
            "Forwarding address not 8-byte aligned"
        );
        // Remove low 3 bits and store in arity field
        let addr_bits = (new_addr as u64) >> 3;
        Self((addr_bits << 10) | (u64::from(object::FORWARD) << 2) | primary::HEADER)
    }

    /// Calculate total object size in bytes (including header).
    ///
    /// Note: No minimum size enforced. Forwarding pointers during GC
    /// replace the 8-byte header with an 8-byte forwarding header,
    /// so even 8-byte objects (empty tuple, empty string) are safe.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub const fn object_size(self) -> usize {
        let arity = self.arity() as usize;
        match self.object_tag() {
            object::TUPLE => 8 + arity * 8,
            object::VECTOR => 16 + arity * 8,
            object::STRING => 8 + align8(arity),
            object::BINARY => 8 + align8(arity),
            object::MAP => 16,
            object::FLOAT => 16,
            object::BIGNUM => 16 + arity * 8,
            object::FUN => arity * 8,
            object::CLOSURE => 16 + arity * 8,
            object::PID => 16,
            object::REF => 16,
            object::PROCBIN => 24,
            object::SUBBIN => 24,
            object::NAMESPACE => 24, // HeapNamespace: header + name + mappings
            object::VAR => 32,       // HeapVar: header + name + namespace + root
            object::SYMBOL => 8 + align8(arity),
            object::KEYWORD => 8 + align8(arity),
            _ => 8,
        }
    }

    // ========================================================================
    // Type-specific header constructors
    // ========================================================================

    /// Create a tuple header.
    #[inline]
    #[must_use]
    pub const fn tuple(element_count: u64) -> Self {
        Self::new(object::TUPLE, element_count)
    }

    /// Create a vector header.
    #[inline]
    #[must_use]
    pub const fn vector(capacity: u64) -> Self {
        Self::new(object::VECTOR, capacity)
    }

    /// Create a string header.
    #[inline]
    #[must_use]
    pub const fn string(byte_length: u64) -> Self {
        Self::new(object::STRING, byte_length)
    }

    /// Create a binary header.
    #[inline]
    #[must_use]
    pub const fn binary(byte_length: u64) -> Self {
        Self::new(object::BINARY, byte_length)
    }

    /// Create a map header.
    #[inline]
    #[must_use]
    pub const fn map(entry_count: u64) -> Self {
        Self::new(object::MAP, entry_count)
    }

    /// Create a float header.
    #[inline]
    #[must_use]
    pub const fn float() -> Self {
        Self::new(object::FLOAT, 0)
    }

    /// Create a bignum header.
    #[inline]
    #[must_use]
    pub const fn bignum(limb_count: u64) -> Self {
        Self::new(object::BIGNUM, limb_count)
    }

    /// Create a function header.
    #[inline]
    #[must_use]
    pub const fn fun(total_words: u64) -> Self {
        Self::new(object::FUN, total_words)
    }

    /// Create a closure header.
    #[inline]
    #[must_use]
    pub const fn closure(capture_count: u64) -> Self {
        Self::new(object::CLOSURE, capture_count)
    }

    /// Create a PID header.
    #[inline]
    #[must_use]
    pub const fn pid() -> Self {
        Self::new(object::PID, 0)
    }

    /// Create a ref header.
    #[inline]
    #[must_use]
    pub const fn reference() -> Self {
        Self::new(object::REF, 0)
    }

    /// Create a procbin header.
    #[inline]
    #[must_use]
    pub const fn procbin() -> Self {
        Self::new(object::PROCBIN, 0)
    }

    /// Create a subbin header.
    #[inline]
    #[must_use]
    pub const fn subbin() -> Self {
        Self::new(object::SUBBIN, 0)
    }

    /// Create a namespace header.
    #[inline]
    #[must_use]
    pub const fn namespace() -> Self {
        Self::new(object::NAMESPACE, 0)
    }

    /// Create a var header.
    #[inline]
    #[must_use]
    pub const fn var() -> Self {
        Self::new(object::VAR, 0)
    }

    /// Create a symbol header (heap-allocated during transition).
    /// TODO: Remove when symbol interning is implemented.
    #[inline]
    #[must_use]
    pub const fn symbol(byte_length: u64) -> Self {
        Self::new(object::SYMBOL, byte_length)
    }

    /// Create a keyword header (heap-allocated during transition).
    /// TODO: Remove when keyword interning is implemented.
    #[inline]
    #[must_use]
    pub const fn keyword(byte_length: u64) -> Self {
        Self::new(object::KEYWORD, byte_length)
    }
}

impl core::fmt::Debug for Header {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let tag = self.object_tag();
        let tag_name = match tag {
            object::TUPLE => "TUPLE",
            object::VECTOR => "VECTOR",
            object::MAP => "MAP",
            object::STRING => "STRING",
            object::BINARY => "BINARY",
            object::BIGNUM => "BIGNUM",
            object::FLOAT => "FLOAT",
            object::FUN => "FUN",
            object::CLOSURE => "CLOSURE",
            object::PID => "PID",
            object::REF => "REF",
            object::PROCBIN => "PROCBIN",
            object::SUBBIN => "SUBBIN",
            object::NAMESPACE => "NAMESPACE",
            object::VAR => "VAR",
            object::SYMBOL => "SYMBOL",
            object::KEYWORD => "KEYWORD",
            object::FORWARD => "FORWARD",
            _ => "UNKNOWN",
        };
        write!(f, "Header::{tag_name}(arity={})", self.arity())
    }
}

// Use the common align8 from parent module
use super::align8;

// ============================================================================
// Term extensions for boxed values
// ============================================================================

impl Term {
    /// Create a boxed pointer to a heap object.
    ///
    /// # Safety
    ///
    /// The pointer must point to a valid, 8-byte aligned header.
    #[inline]
    #[must_use]
    pub fn boxed(ptr: *const Header) -> Self {
        debug_assert!(
            (ptr as u64).trailing_zeros() >= 3,
            "Pointer not 8-byte aligned"
        );
        Self((ptr as u64) | primary::BOXED)
    }

    /// Get pointer to header (if this is boxed).
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_ptr_alignment,
        reason = "boxed pointers are 8-byte aligned per term-representation.md"
    )]
    pub const fn as_header_ptr(self) -> Option<*const Header> {
        if self.is_boxed() {
            Some(self.to_ptr().cast::<Header>())
        } else {
            None
        }
    }

    /// Get mutable pointer to header (if this is boxed).
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_ptr_alignment,
        reason = "boxed pointers are 8-byte aligned per term-representation.md"
    )]
    pub const fn as_header_ptr_mut(self) -> Option<*mut Header> {
        if self.is_boxed() {
            Some(self.to_ptr_mut().cast::<Header>())
        } else {
            None
        }
    }

    /// Read the header of a boxed value.
    ///
    /// # Safety
    ///
    /// The pointer must be valid and the object must not have been deallocated.
    #[inline]
    #[must_use]
    pub unsafe fn header(self) -> Option<Header> {
        // SAFETY: Caller guarantees the pointer is valid and the object is live.
        self.as_header_ptr().map(|p| unsafe { *p })
    }

    /// Get object tag (if boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid and the object must not have been deallocated.
    #[inline]
    #[must_use]
    pub unsafe fn object_tag(self) -> Option<u8> {
        // SAFETY: Caller guarantees the pointer is valid and the object is live.
        unsafe { self.header() }.map(Header::object_tag)
    }

    // ========================================================================
    // Type checking helpers
    // ========================================================================

    /// Check if this is a tuple (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_tuple(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::TUPLE) }
    }

    /// Check if this is a vector (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_vector(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::VECTOR) }
    }

    /// Check if this is a string (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_string(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::STRING) }
    }

    /// Check if this is a binary (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_binary(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::BINARY) }
    }

    /// Check if this is a map (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_map(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::MAP) }
    }

    /// Check if this is a float (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_float(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::FLOAT) }
    }

    /// Check if this is a bignum (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_bignum(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::BIGNUM) }
    }

    /// Check if this is a function (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_fun(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::FUN) }
    }

    /// Check if this is a closure (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_closure(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::CLOSURE) }
    }

    /// Check if this is a PID (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_pid(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::PID) }
    }

    /// Check if this is a reference (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_ref(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::REF) }
    }

    /// Check if this is a procbin (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_procbin(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::PROCBIN) }
    }

    /// Check if this is a subbin (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_subbin(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::SUBBIN) }
    }

    /// Check if this is a namespace (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_namespace(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::NAMESPACE) }
    }

    /// Check if this is a var (boxed).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_var(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::VAR) }
    }

    /// Check if this is a heap-allocated symbol (boxed, transition only).
    /// TODO: Remove when symbol interning is implemented.
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_heap_symbol(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::SYMBOL) }
    }

    /// Check if this is a heap-allocated keyword (boxed, transition only).
    /// TODO: Remove when keyword interning is implemented.
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_heap_keyword(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::KEYWORD) }
    }

    /// Check if this boxed value has been forwarded (GC).
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    pub unsafe fn is_forwarded(self) -> bool {
        // SAFETY: Caller guarantees the pointer is valid.
        unsafe { self.object_tag() == Some(object::FORWARD) }
    }
}
