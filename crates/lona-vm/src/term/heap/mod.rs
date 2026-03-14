// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Heap-allocated object types using the new Term representation.
//!
//! All boxed heap objects have a common layout:
//! - 8-byte header (encoding type and size/arity)
//! - Type-specific payload
//!
//! The header is always at the start and can be read to determine
//! the object type and size for GC purposes.
//!
//! See `docs/architecture/term-representation.md` for the full specification.

#[cfg(test)]
mod heap_test;

use core::slice;

use super::Term;
use super::header::Header;

// ============================================================================
// Tuple
// ============================================================================

/// Heap-allocated tuple.
///
/// Layout: header (8 bytes) + N elements (8 bytes each)
/// The header's arity field stores the element count.
///
/// An empty tuple is valid (0 elements, just the header).
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HeapTuple {
    /// Header with object tag TUPLE and arity = element count.
    pub header: Header,
    // Elements follow immediately (accessed via pointer arithmetic)
}

// Compile-time assertion that HeapTuple header is 8 bytes
const _: () = assert!(core::mem::size_of::<HeapTuple>() == 8);

impl HeapTuple {
    /// Size of the header in bytes.
    pub const HEADER_SIZE: usize = 8;

    /// Calculate total allocation size for a tuple of given length.
    #[inline]
    #[must_use]
    pub const fn alloc_size(len: usize) -> usize {
        Self::HEADER_SIZE + len * 8
    }

    /// Create a header for a tuple with the given element count.
    #[inline]
    #[must_use]
    pub const fn make_header(element_count: usize) -> Header {
        Header::tuple(element_count as u64)
    }

    /// Get the number of elements in this tuple.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.header.arity() as usize
    }

    /// Check if this tuple is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get an element by index.
    ///
    /// Returns `None` if the index is out of bounds.
    ///
    /// # Safety
    ///
    /// The tuple must be properly initialized with the correct number of elements.
    #[inline]
    #[must_use]
    pub const unsafe fn get(&self, index: usize) -> Option<Term> {
        if index >= self.len() {
            return None;
        }
        // SAFETY: Caller guarantees the tuple is properly initialized.
        // Index bounds are checked above. Pointer arithmetic is within the allocated object.
        let elements = unsafe { (&raw const *self).add(1).cast::<Term>() };
        Some(unsafe { *elements.add(index) })
    }

    /// Get a slice of all elements.
    ///
    /// # Safety
    ///
    /// The tuple must be properly initialized with the correct number of elements.
    #[inline]
    #[must_use]
    pub const unsafe fn elements(&self) -> &[Term] {
        // SAFETY: Caller guarantees the tuple is properly initialized.
        // Pointer arithmetic is within the allocated object.
        let ptr = unsafe { (&raw const *self).add(1).cast::<Term>() };
        unsafe { slice::from_raw_parts(ptr, self.len()) }
    }
}

// ============================================================================
// Vector
// ============================================================================

/// Heap-allocated vector with separate length and capacity.
///
/// Layout: header (8 bytes) + length (8 bytes) + capacity elements (8 bytes each)
/// The header's arity field stores the capacity.
#[repr(C)]
pub struct HeapVector {
    /// Header with object tag VECTOR and arity = capacity.
    pub header: Header,
    /// Current number of elements (always <= capacity).
    pub length: u64,
    // Elements follow immediately (accessed via pointer arithmetic)
}

// Compile-time assertion that HeapVector prefix is 16 bytes
const _: () = assert!(core::mem::size_of::<HeapVector>() == 16);

impl HeapVector {
    /// Size of the header portion (including length field) in bytes.
    pub const PREFIX_SIZE: usize = 16;

    /// Calculate total allocation size for a vector with given capacity.
    #[inline]
    #[must_use]
    pub const fn alloc_size(capacity: usize) -> usize {
        Self::PREFIX_SIZE + capacity * 8
    }

    /// Create a header for a vector with the given capacity.
    #[inline]
    #[must_use]
    pub const fn make_header(capacity: usize) -> Header {
        Header::vector(capacity as u64)
    }

    /// Get the capacity of this vector.
    #[inline]
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.header.arity() as usize
    }

    /// Get the current length of this vector.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.length as usize
    }

    /// Check if this vector is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Get an element by index.
    ///
    /// Returns `None` if the index is out of bounds.
    ///
    /// # Safety
    ///
    /// The vector must be properly initialized.
    #[inline]
    #[must_use]
    pub const unsafe fn get(&self, index: usize) -> Option<Term> {
        if index >= self.len() {
            return None;
        }
        // SAFETY: Caller guarantees the vector is properly initialized.
        // Index bounds are checked above. Pointer arithmetic is within the allocated object.
        let elements = unsafe { (&raw const *self).add(1).cast::<Term>() };
        Some(unsafe { *elements.add(index) })
    }

    /// Get a slice of all elements (up to current length).
    ///
    /// # Safety
    ///
    /// The vector must be properly initialized.
    #[inline]
    #[must_use]
    pub const unsafe fn elements(&self) -> &[Term] {
        // SAFETY: Caller guarantees the vector is properly initialized.
        // Pointer arithmetic is within the allocated object.
        let ptr = unsafe { (&raw const *self).add(1).cast::<Term>() };
        unsafe { slice::from_raw_parts(ptr, self.len()) }
    }
}

// ============================================================================
// String
// ============================================================================

/// Heap-allocated UTF-8 string.
///
/// Layout: header (8 bytes) + UTF-8 bytes (padded to 8-byte alignment)
/// The header's arity field stores the byte length.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HeapString {
    /// Header with object tag STRING and arity = byte length.
    pub header: Header,
    // UTF-8 bytes follow immediately (padded to 8-byte alignment)
}

// Compile-time assertion that HeapString header is 8 bytes
const _: () = assert!(core::mem::size_of::<HeapString>() == 8);

impl HeapString {
    /// Size of the header in bytes.
    pub const HEADER_SIZE: usize = 8;

    /// Calculate total allocation size for a string of given byte length.
    /// The data is padded to 8-byte alignment.
    #[inline]
    #[must_use]
    pub const fn alloc_size(byte_len: usize) -> usize {
        Self::HEADER_SIZE + align8(byte_len)
    }

    /// Create a header for a string with the given byte length.
    #[inline]
    #[must_use]
    pub const fn make_header(byte_len: usize) -> Header {
        Header::string(byte_len as u64)
    }

    /// Get the length of this string in bytes.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.header.arity() as usize
    }

    /// Check if this string is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the UTF-8 bytes of this string.
    ///
    /// # Safety
    ///
    /// The string must be properly initialized.
    #[inline]
    #[must_use]
    pub const unsafe fn as_bytes(&self) -> &[u8] {
        // SAFETY: Caller guarantees the string is properly initialized.
        // Pointer arithmetic is within the allocated object.
        let ptr = unsafe { (&raw const *self).add(1).cast::<u8>() };
        unsafe { slice::from_raw_parts(ptr, self.len()) }
    }

    /// Get this string as a str slice.
    ///
    /// Returns `None` if the string contains invalid UTF-8.
    ///
    /// # Safety
    ///
    /// The string must be properly initialized.
    #[inline]
    #[must_use]
    pub unsafe fn as_str(&self) -> Option<&str> {
        // SAFETY: Caller guarantees the string is properly initialized.
        core::str::from_utf8(unsafe { self.as_bytes() }).ok()
    }

    /// Get a pointer to the byte data for writing.
    ///
    /// # Safety
    ///
    /// The string must have been allocated with enough space.
    #[inline]
    #[must_use]
    pub const unsafe fn data_ptr_mut(&mut self) -> *mut u8 {
        // SAFETY: Caller guarantees the string has been allocated with enough space.
        unsafe { (&raw mut *self).add(1).cast::<u8>() }
    }
}

// ============================================================================
// Symbol String Storage
// ============================================================================

/// Symbol string storage for realm interning.
///
/// Layout: header (8 bytes) + UTF-8 bytes (padded to 8-byte alignment)
/// The header's arity field stores the byte length.
///
/// Symbols themselves are immediate values with indices into realm tables.
/// This type stores the actual string data that those indices reference.
/// Allocated in the realm code region, not on process heaps.
#[repr(C)]
pub struct HeapSymbol {
    /// Header with object tag SYMBOL and arity = byte length.
    pub header: Header,
    // UTF-8 bytes follow immediately (padded to 8-byte alignment)
}

// Compile-time assertion that HeapSymbol header is 8 bytes
const _: () = assert!(core::mem::size_of::<HeapSymbol>() == 8);

impl HeapSymbol {
    /// Size of the header in bytes.
    pub const HEADER_SIZE: usize = 8;

    /// Calculate total allocation size for a symbol of given byte length.
    #[inline]
    #[must_use]
    pub const fn alloc_size(byte_len: usize) -> usize {
        Self::HEADER_SIZE + align8(byte_len)
    }

    /// Create a header for a symbol with the given byte length.
    #[inline]
    #[must_use]
    pub const fn make_header(byte_len: usize) -> Header {
        Header::symbol(byte_len as u64)
    }

    /// Get the length of this symbol in bytes.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.header.arity() as usize
    }

    /// Check if this symbol is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the UTF-8 bytes of this symbol.
    ///
    /// # Safety
    ///
    /// The symbol must be properly initialized.
    #[inline]
    #[must_use]
    pub const unsafe fn as_bytes(&self) -> &[u8] {
        // SAFETY: Caller guarantees the symbol is properly initialized.
        // Pointer arithmetic is within the allocated object.
        let ptr = unsafe { (&raw const *self).add(1).cast::<u8>() };
        unsafe { slice::from_raw_parts(ptr, self.len()) }
    }

    /// Get this symbol as a str slice.
    ///
    /// Returns `None` if the symbol contains invalid UTF-8.
    ///
    /// # Safety
    ///
    /// The symbol must be properly initialized.
    #[inline]
    #[must_use]
    pub unsafe fn as_str(&self) -> Option<&str> {
        // SAFETY: Caller guarantees the symbol is properly initialized.
        core::str::from_utf8(unsafe { self.as_bytes() }).ok()
    }
}

// ============================================================================
// Keyword String Storage
// ============================================================================

/// Keyword string storage for realm interning.
///
/// Layout: header (8 bytes) + UTF-8 bytes (padded to 8-byte alignment)
/// The header's arity field stores the byte length.
///
/// Keywords themselves are immediate values with indices into realm tables.
/// This type stores the actual string data that those indices reference.
/// Allocated in the realm code region, not on process heaps.
#[repr(C)]
pub struct HeapKeyword {
    /// Header with object tag KEYWORD and arity = byte length.
    pub header: Header,
    // UTF-8 bytes follow immediately (padded to 8-byte alignment)
}

// Compile-time assertion that HeapKeyword header is 8 bytes
const _: () = assert!(core::mem::size_of::<HeapKeyword>() == 8);

impl HeapKeyword {
    /// Size of the header in bytes.
    pub const HEADER_SIZE: usize = 8;

    /// Calculate total allocation size for a keyword of given byte length.
    #[inline]
    #[must_use]
    pub const fn alloc_size(byte_len: usize) -> usize {
        Self::HEADER_SIZE + align8(byte_len)
    }

    /// Create a header for a keyword with the given byte length.
    #[inline]
    #[must_use]
    pub const fn make_header(byte_len: usize) -> Header {
        Header::keyword(byte_len as u64)
    }

    /// Get the length of this keyword in bytes.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.header.arity() as usize
    }

    /// Check if this keyword is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get the UTF-8 bytes of this keyword.
    ///
    /// # Safety
    ///
    /// The keyword must be properly initialized.
    #[inline]
    #[must_use]
    pub const unsafe fn as_bytes(&self) -> &[u8] {
        // SAFETY: Caller guarantees the keyword is properly initialized.
        // Pointer arithmetic is within the allocated object.
        let ptr = unsafe { (&raw const *self).add(1).cast::<u8>() };
        unsafe { slice::from_raw_parts(ptr, self.len()) }
    }

    /// Get this keyword as a str slice.
    ///
    /// Returns `None` if the keyword contains invalid UTF-8.
    ///
    /// # Safety
    ///
    /// The keyword must be properly initialized.
    #[inline]
    #[must_use]
    pub unsafe fn as_str(&self) -> Option<&str> {
        // SAFETY: Caller guarantees the keyword is properly initialized.
        core::str::from_utf8(unsafe { self.as_bytes() }).ok()
    }
}

// ============================================================================
// Map
// ============================================================================

/// Heap-allocated map (association list).
///
/// Layout: header (8 bytes) + entries pointer (8 bytes)
/// The header's arity field stores the entry count (for statistics).
/// The entries field is a pair cell chain or NIL for empty map.
#[repr(C)]
pub struct HeapMap {
    /// Header with object tag MAP and arity = entry count.
    pub header: Header,
    /// Head of the association list (pair chain or NIL).
    pub entries: Term,
}

// Compile-time assertion that HeapMap is 16 bytes
const _: () = assert!(core::mem::size_of::<HeapMap>() == 16);

impl HeapMap {
    /// Size of the map in bytes.
    pub const SIZE: usize = 16;

    /// Create a header for a map with the given entry count.
    #[inline]
    #[must_use]
    pub const fn make_header(entry_count: usize) -> Header {
        Header::map(entry_count as u64)
    }

    /// Get the entry count (stored in header for statistics).
    #[inline]
    #[must_use]
    pub const fn entry_count(&self) -> usize {
        self.header.arity() as usize
    }

    /// Check if this map is empty.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_nil()
    }
}

// ============================================================================
// Float
// ============================================================================

/// Heap-allocated 64-bit floating point number.
///
/// Layout: header (8 bytes) + f64 value (8 bytes)
#[repr(C)]
pub struct HeapFloat {
    /// Header with object tag FLOAT.
    pub header: Header,
    /// The floating point value.
    pub value: f64,
}

// Compile-time assertion that HeapFloat is 16 bytes
const _: () = assert!(core::mem::size_of::<HeapFloat>() == 16);

impl HeapFloat {
    /// Size of a float in bytes.
    pub const SIZE: usize = 16;

    /// Create a header for a float.
    #[inline]
    #[must_use]
    pub const fn make_header() -> Header {
        Header::float()
    }

    /// Get the floating point value.
    #[inline]
    #[must_use]
    pub const fn get(&self) -> f64 {
        self.value
    }
}

// ============================================================================
// Function
// ============================================================================

/// Heap-allocated compiled function.
///
/// Layout: header (8 bytes) + metadata (8 bytes) + bytecode + constants
///
/// IMPORTANT: The header's arity field stores the TOTAL object size in words
/// (not the function's parameter arity). This allows GC to determine object
/// size from the header alone without chasing pointers.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HeapFun {
    /// Header with object tag FUN.
    ///
    /// IMPORTANT: arity = `total_words`, so object size = arity * 8
    pub header: Header,
    /// Number of function parameters (0-255).
    pub fn_arity: u8,
    /// Whether this function is variadic (accepts rest args).
    pub variadic: u8,
    /// Number of local variables used.
    pub locals: u8,
    /// Padding for alignment.
    #[doc(hidden)]
    pub _pad: u8,
    /// Length of bytecode in bytes.
    pub code_len: u16,
    /// Number of constants.
    pub const_count: u16,
    // Bytecode follows, then constants
}

// Compile-time assertion that HeapFun prefix is 16 bytes
const _: () = assert!(core::mem::size_of::<HeapFun>() == 16);

impl HeapFun {
    /// Size of the fixed prefix in bytes.
    pub const PREFIX_SIZE: usize = 16;

    /// Calculate the aligned offset where constants start.
    ///
    /// Constants are Terms (8 bytes) and must be 8-byte aligned.
    /// This rounds up `PREFIX_SIZE + code_len` to the next 8-byte boundary.
    #[inline]
    #[must_use]
    pub const fn constants_offset(code_len: usize) -> usize {
        let unaligned = Self::PREFIX_SIZE + code_len;
        (unaligned + 7) & !7
    }

    /// Calculate total allocation size for a function.
    ///
    /// Returns the size in bytes.
    #[inline]
    #[must_use]
    pub const fn alloc_size(code_len: usize, const_count: usize) -> usize {
        Self::constants_offset(code_len) + const_count * 8
    }

    /// Calculate the total size in words for the header's arity field.
    #[inline]
    #[must_use]
    pub const fn total_words(code_len: usize, const_count: usize) -> u64 {
        let bytes = Self::alloc_size(code_len, const_count);
        bytes.div_ceil(8) as u64
    }

    /// Create a header for a function.
    #[inline]
    #[must_use]
    pub const fn make_header(code_len: usize, const_count: usize) -> Header {
        Header::fun(Self::total_words(code_len, const_count))
    }

    /// Get the function's parameter arity.
    #[inline]
    #[must_use]
    pub const fn arity(&self) -> u8 {
        self.fn_arity
    }

    /// Read the `code_len` field from a `HeapFun` at the given address.
    ///
    /// `base_addr` must point to a fully initialized `HeapFun` in mapped memory.
    #[inline]
    #[must_use]
    pub fn read_code_len<M: crate::platform::MemorySpace>(mem: &M, base_addr: crate::Vaddr) -> u16 {
        mem.read(crate::Vaddr::new(base_addr.as_u64() + 12))
    }

    /// Read the `const_count` field from a `HeapFun` at the given address.
    ///
    /// `base_addr` must point to a fully initialized `HeapFun` in mapped memory.
    #[inline]
    #[must_use]
    pub fn read_const_count<M: crate::platform::MemorySpace>(
        mem: &M,
        base_addr: crate::Vaddr,
    ) -> u16 {
        mem.read(crate::Vaddr::new(base_addr.as_u64() + 14))
    }

    /// Read an instruction at the given index from a `HeapFun`'s bytecode.
    ///
    /// Instructions start at `base_addr + PREFIX_SIZE` and are 4 bytes each.
    /// `base_addr` must point to a fully initialized `HeapFun` in mapped memory,
    /// and `ip` must be less than `instruction_count`.
    #[inline]
    #[must_use]
    pub fn read_instruction<M: crate::platform::MemorySpace>(
        mem: &M,
        base_addr: crate::Vaddr,
        ip: usize,
    ) -> u32 {
        let instr_addr =
            crate::Vaddr::new(base_addr.as_u64() + Self::PREFIX_SIZE as u64 + (ip as u64) * 4);
        mem.read(instr_addr)
    }

    /// Read a constant at the given index from a `HeapFun`'s constant pool.
    ///
    /// Constants follow the bytecode at an 8-byte aligned offset.
    /// `base_addr` must point to a fully initialized `HeapFun` in mapped memory,
    /// and `index` must be less than `const_count`.
    #[inline]
    #[must_use]
    pub fn read_constant<M: crate::platform::MemorySpace>(
        mem: &M,
        base_addr: crate::Vaddr,
        code_len: u16,
        index: usize,
    ) -> Term {
        let constants_offset = Self::constants_offset(code_len as usize);
        let const_addr =
            crate::Vaddr::new(base_addr.as_u64() + constants_offset as u64 + (index as u64) * 8);
        mem.read(const_addr)
    }

    /// Read the number of bytecode instructions in a `HeapFun`.
    ///
    /// `code_len` is stored in bytes; instructions are 4 bytes each.
    /// `base_addr` must point to a fully initialized `HeapFun` in mapped memory.
    #[inline]
    #[must_use]
    pub fn instruction_count<M: crate::platform::MemorySpace>(
        mem: &M,
        base_addr: crate::Vaddr,
    ) -> usize {
        let code_len = Self::read_code_len(mem, base_addr);
        code_len as usize / 4
    }

    /// Check if this function is variadic.
    #[inline]
    #[must_use]
    pub const fn is_variadic(&self) -> bool {
        self.variadic != 0
    }

    /// Get a pointer to the bytecode.
    ///
    /// # Safety
    ///
    /// The function must be properly initialized.
    #[inline]
    #[must_use]
    pub const unsafe fn code_ptr(&self) -> *const u8 {
        // SAFETY: Caller guarantees the function is properly initialized.
        unsafe { (&raw const *self).add(1).cast::<u8>() }
    }

    /// Get the bytecode as a slice.
    ///
    /// # Safety
    ///
    /// The function must be properly initialized.
    #[inline]
    #[must_use]
    pub const unsafe fn code(&self) -> &[u8] {
        // SAFETY: Caller guarantees the function is properly initialized.
        unsafe { slice::from_raw_parts(self.code_ptr(), self.code_len as usize) }
    }

    /// Get a pointer to the constants array.
    ///
    /// # Safety
    ///
    /// The function must be properly initialized.
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_ptr_alignment,
        reason = "constants are 8-byte aligned per HeapFun layout in term-representation.md"
    )]
    pub const unsafe fn constants_ptr(&self) -> *const Term {
        // Constants follow bytecode at an 8-byte aligned offset
        // SAFETY: Caller guarantees the function is properly initialized.
        // Pointer arithmetic is within the allocated object.
        let base = (&raw const *self).cast::<u8>();
        unsafe {
            base.add(Self::constants_offset(self.code_len as usize))
                .cast::<Term>()
        }
    }

    /// Get the constants as a slice.
    ///
    /// # Safety
    ///
    /// The function must be properly initialized.
    #[inline]
    #[must_use]
    pub const unsafe fn constants(&self) -> &[Term] {
        // SAFETY: Caller guarantees the function is properly initialized.
        unsafe { slice::from_raw_parts(self.constants_ptr(), self.const_count as usize) }
    }
}

// ============================================================================
// Closure
// ============================================================================

/// Heap-allocated closure (function + captured environment).
///
/// Layout: header (8 bytes) + function pointer (8 bytes) + captures (8 bytes each)
/// The header's arity field stores the capture count.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct HeapClosure {
    /// Header with object tag CLOSURE and arity = capture count.
    pub header: Header,
    /// Pointer to the underlying function.
    pub function: Term,
    // Captured values follow
}

// Compile-time assertion that HeapClosure prefix is 16 bytes
const _: () = assert!(core::mem::size_of::<HeapClosure>() == 16);

impl HeapClosure {
    /// Size of the fixed prefix in bytes.
    pub const PREFIX_SIZE: usize = 16;

    /// Calculate total allocation size for a closure with given capture count.
    #[inline]
    #[must_use]
    pub const fn alloc_size(capture_count: usize) -> usize {
        Self::PREFIX_SIZE + capture_count * 8
    }

    /// Create a header for a closure with the given capture count.
    #[inline]
    #[must_use]
    pub const fn make_header(capture_count: usize) -> Header {
        Header::closure(capture_count as u64)
    }

    /// Get the number of captures.
    #[inline]
    #[must_use]
    pub const fn capture_count(&self) -> usize {
        self.header.arity() as usize
    }

    /// Get a captured value by index.
    ///
    /// # Safety
    ///
    /// The closure must be properly initialized.
    #[inline]
    #[must_use]
    pub const unsafe fn get_capture(&self, index: usize) -> Option<Term> {
        if index >= self.capture_count() {
            return None;
        }
        // SAFETY: Caller guarantees the closure is properly initialized.
        // Index bounds are checked above. Pointer arithmetic is within the allocated object.
        let captures = unsafe { (&raw const *self).add(1).cast::<Term>() };
        Some(unsafe { *captures.add(index) })
    }

    /// Get all captures as a slice.
    ///
    /// # Safety
    ///
    /// The closure must be properly initialized.
    #[inline]
    #[must_use]
    pub const unsafe fn captures(&self) -> &[Term] {
        // SAFETY: Caller guarantees the closure is properly initialized.
        // Pointer arithmetic is within the allocated object.
        let ptr = unsafe { (&raw const *self).add(1).cast::<Term>() };
        unsafe { slice::from_raw_parts(ptr, self.capture_count()) }
    }
}

// ============================================================================
// PID (Process ID)
// ============================================================================

/// Heap-allocated process identifier.
///
/// Layout: header (8 bytes) + process data (8 bytes)
#[repr(C)]
pub struct HeapPid {
    /// Header with object tag PID.
    pub header: Header,
    /// Process index and generation packed into 64 bits.
    pub data: u64,
}

// Compile-time assertion that HeapPid is 16 bytes
const _: () = assert!(core::mem::size_of::<HeapPid>() == 16);

impl HeapPid {
    /// Size of a PID in bytes.
    pub const SIZE: usize = 16;

    /// Create a header for a PID.
    #[inline]
    #[must_use]
    pub const fn make_header() -> Header {
        Header::pid()
    }
}

// ============================================================================
// Ref (Unique Reference)
// ============================================================================

/// Heap-allocated unique reference.
///
/// Layout: header (8 bytes) + id data (8 bytes)
#[repr(C)]
pub struct HeapRef {
    /// Header with object tag REF.
    pub header: Header,
    /// Unique reference ID.
    pub id: u64,
}

// Compile-time assertion that HeapRef is 16 bytes
const _: () = assert!(core::mem::size_of::<HeapRef>() == 16);

impl HeapRef {
    /// Size of a ref in bytes.
    pub const SIZE: usize = 16;

    /// Create a header for a ref.
    #[inline]
    #[must_use]
    pub const fn make_header() -> Header {
        Header::reference()
    }
}

// ============================================================================
// Bignum (Arbitrary Precision Integer)
// ============================================================================

/// Heap-allocated arbitrary precision integer.
///
/// Layout: header (8 bytes) + sign/length (8 bytes) + limbs (8 bytes each)
/// The header's arity field stores the limb count.
#[repr(C)]
pub struct HeapBignum {
    /// Header with object tag BIGNUM and arity = limb count.
    pub header: Header,
    /// Sign (0 = positive, 1 = negative) packed with reserved bits.
    pub sign: u64,
    // Limbs follow (u64 array)
}

// Compile-time assertion that HeapBignum prefix is 16 bytes
const _: () = assert!(core::mem::size_of::<HeapBignum>() == 16);

impl HeapBignum {
    /// Size of the fixed prefix in bytes.
    pub const PREFIX_SIZE: usize = 16;

    /// Calculate total allocation size for a bignum with given limb count.
    #[inline]
    #[must_use]
    pub const fn alloc_size(limb_count: usize) -> usize {
        Self::PREFIX_SIZE + limb_count * 8
    }

    /// Create a header for a bignum with the given limb count.
    #[inline]
    #[must_use]
    pub const fn make_header(limb_count: usize) -> Header {
        Header::bignum(limb_count as u64)
    }

    /// Get the number of limbs.
    #[inline]
    #[must_use]
    pub const fn limb_count(&self) -> usize {
        self.header.arity() as usize
    }

    /// Check if this bignum is negative.
    #[inline]
    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.sign != 0
    }

    /// Get the limbs as a slice.
    ///
    /// # Safety
    ///
    /// The bignum must be properly initialized.
    #[inline]
    #[must_use]
    pub const unsafe fn limbs(&self) -> &[u64] {
        // SAFETY: Caller guarantees the bignum is properly initialized.
        // Pointer arithmetic is within the allocated object.
        let ptr = unsafe { (&raw const *self).add(1).cast::<u64>() };
        unsafe { slice::from_raw_parts(ptr, self.limb_count()) }
    }
}

// ============================================================================
// Namespace
// ============================================================================

/// Heap-allocated namespace.
///
/// Layout: header (8 bytes) + name pointer (8 bytes) + mappings pointer (8 bytes)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HeapNamespace {
    /// Header with object tag NAMESPACE.
    pub header: Header,
    /// Name symbol (interned).
    pub name: Term,
    /// Symbol->var mappings (pair chain of [symbol var] tuples).
    pub mappings: Term,
}

// Compile-time assertion that HeapNamespace is 24 bytes
const _: () = assert!(core::mem::size_of::<HeapNamespace>() == 24);

impl HeapNamespace {
    /// Size of a namespace in bytes.
    pub const SIZE: usize = 24;

    /// Create a header for a namespace.
    #[inline]
    #[must_use]
    pub const fn make_header() -> Header {
        Header::namespace()
    }
}

// ============================================================================
// Var
// ============================================================================

/// Heap-allocated var.
///
/// Layout: header (8 bytes) + name (8 bytes) + namespace (8 bytes) + root (8 bytes)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HeapVar {
    /// Header with object tag VAR.
    pub header: Header,
    /// Var name (interned symbol).
    pub name: Term,
    /// Containing namespace.
    pub namespace: Term,
    /// Root binding value.
    pub root: Term,
}

// Compile-time assertion that HeapVar is 32 bytes
const _: () = assert!(core::mem::size_of::<HeapVar>() == 32);

impl HeapVar {
    /// Size of a var in bytes.
    pub const SIZE: usize = 32;

    /// Create a header for a var.
    #[inline]
    #[must_use]
    pub const fn make_header() -> Header {
        Header::var()
    }
}

// ============================================================================
// Pair (Cons Cell)
// ============================================================================

/// Heap-allocated cons cell for lists.
///
/// Layout: head (8 bytes) + tail (8 bytes), NO header (identified by LIST tag).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct HeapPair {
    /// First element of the pair.
    pub head: Term,
    /// Rest of the list (another pair or NIL).
    pub tail: Term,
}

// Compile-time assertion that HeapPair is 16 bytes
const _: () = assert!(core::mem::size_of::<HeapPair>() == 16);

impl HeapPair {
    /// Size of a pair in bytes.
    pub const SIZE: usize = 16;
}

// ============================================================================
// ProcBin (Process Binary Reference)
// ============================================================================

/// Process-local reference to a reference-counted binary in the realm binary heap.
///
/// Layout: header (8 bytes) + `binary_addr` (8 bytes) + offset (4 bytes) + size (4 bytes)
///
/// Large binaries (>= 64 bytes) are stored in the realm's binary heap with
/// reference counting. Each process that references a binary has a `HeapProcBin`
/// on its heap that tracks the reference.
///
/// The MSO list tracks all `HeapProcBin` objects so GC can decrement refcounts
/// when they become unreachable.
#[repr(C)]
pub struct HeapProcBin {
    /// Header with object tag PROCBIN.
    pub header: Header,
    /// Address of the `RefcBinary` in the realm binary heap.
    pub binary_addr: crate::Vaddr,
    /// Offset into the binary (for sub-binary views).
    pub offset: u32,
    /// Size of this reference (may be less than full binary for sub-binaries).
    pub size: u32,
}

// Compile-time assertion that HeapProcBin is 24 bytes
const _: () = assert!(core::mem::size_of::<HeapProcBin>() == 24);

impl HeapProcBin {
    /// Size of a `ProcBin` in bytes.
    pub const SIZE: usize = 24;

    /// Create a header for a `ProcBin`.
    #[inline]
    #[must_use]
    pub const fn make_header() -> Header {
        Header::procbin()
    }

    /// Get the binary address in the realm binary heap.
    #[inline]
    #[must_use]
    pub const fn binary_addr(&self) -> crate::Vaddr {
        self.binary_addr
    }

    /// Get the offset into the binary.
    #[inline]
    #[must_use]
    pub const fn offset(&self) -> u32 {
        self.offset
    }

    /// Get the size of this reference.
    #[inline]
    #[must_use]
    pub const fn size(&self) -> u32 {
        self.size
    }
}

// ============================================================================
// Utility functions
// ============================================================================

// Use the common align8 from parent module
use super::align8;

// ============================================================================
// Term extensions for type-safe access
// ============================================================================

impl Term {
    /// Get this term as a tuple pointer.
    ///
    /// Returns `None` if this is not a boxed tuple.
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_ptr_alignment,
        reason = "boxed pointers are 8-byte aligned per term-representation.md"
    )]
    pub unsafe fn as_tuple(&self) -> Option<&HeapTuple> {
        // SAFETY: Caller guarantees the pointer is valid.
        if unsafe { self.is_tuple() } {
            Some(unsafe { &*self.to_ptr().cast::<HeapTuple>() })
        } else {
            None
        }
    }

    /// Get this term as a vector pointer.
    ///
    /// Returns `None` if this is not a boxed vector.
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_ptr_alignment,
        reason = "boxed pointers are 8-byte aligned per term-representation.md"
    )]
    pub unsafe fn as_vector(&self) -> Option<&HeapVector> {
        // SAFETY: Caller guarantees the pointer is valid.
        if unsafe { self.is_vector() } {
            Some(unsafe { &*self.to_ptr().cast::<HeapVector>() })
        } else {
            None
        }
    }

    /// Get this term as a string pointer.
    ///
    /// Returns `None` if this is not a boxed string.
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_ptr_alignment,
        reason = "boxed pointers are 8-byte aligned per term-representation.md"
    )]
    pub unsafe fn as_heap_string(&self) -> Option<&HeapString> {
        // SAFETY: Caller guarantees the pointer is valid.
        if unsafe { self.is_string() } {
            Some(unsafe { &*self.to_ptr().cast::<HeapString>() })
        } else {
            None
        }
    }

    /// Get this term as a map pointer.
    ///
    /// Returns `None` if this is not a boxed map.
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_ptr_alignment,
        reason = "boxed pointers are 8-byte aligned per term-representation.md"
    )]
    pub unsafe fn as_map(&self) -> Option<&HeapMap> {
        // SAFETY: Caller guarantees the pointer is valid.
        if unsafe { self.is_map() } {
            Some(unsafe { &*self.to_ptr().cast::<HeapMap>() })
        } else {
            None
        }
    }

    /// Get this term as a float pointer.
    ///
    /// Returns `None` if this is not a boxed float.
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_ptr_alignment,
        reason = "boxed pointers are 8-byte aligned per term-representation.md"
    )]
    pub unsafe fn as_float(&self) -> Option<&HeapFloat> {
        // SAFETY: Caller guarantees the pointer is valid.
        if unsafe { self.is_float() } {
            Some(unsafe { &*self.to_ptr().cast::<HeapFloat>() })
        } else {
            None
        }
    }

    /// Get this term as a function pointer.
    ///
    /// Returns `None` if this is not a boxed function.
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_ptr_alignment,
        reason = "boxed pointers are 8-byte aligned per term-representation.md"
    )]
    pub unsafe fn as_fun(&self) -> Option<&HeapFun> {
        // SAFETY: Caller guarantees the pointer is valid.
        if unsafe { self.is_fun() } {
            Some(unsafe { &*self.to_ptr().cast::<HeapFun>() })
        } else {
            None
        }
    }

    /// Get this term as a closure pointer.
    ///
    /// Returns `None` if this is not a boxed closure.
    ///
    /// # Safety
    ///
    /// The pointer must be valid if this is a boxed value.
    #[inline]
    #[must_use]
    #[expect(
        clippy::cast_ptr_alignment,
        reason = "boxed pointers are 8-byte aligned per term-representation.md"
    )]
    pub unsafe fn as_closure(&self) -> Option<&HeapClosure> {
        // SAFETY: Caller guarantees the pointer is valid.
        if unsafe { self.is_closure() } {
            Some(unsafe { &*self.to_ptr().cast::<HeapClosure>() })
        } else {
            None
        }
    }
}
