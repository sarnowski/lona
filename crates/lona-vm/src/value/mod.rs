// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Value representation for the Lonala language.
//!
//! Values are the runtime representation of Lonala expressions.
//! Immediate values (nil, bool, int) fit in registers.
//! Compound values (strings, pairs, symbols) are heap-allocated.

#[cfg(test)]
mod mod_test;
#[cfg(test)]
mod printer_test;

mod printer;

pub use printer::print_value;

use crate::Vaddr;
use core::fmt;

/// A Lonala value.
///
/// The value representation uses 16 bytes (tag + payload).
/// Immediate values are stored inline, heap values store a `Vaddr` pointer.
///
/// Tags follow the VM specification (see `docs/architecture/virtual-machine.md`):
/// - 0x0-0x8: Basic types (nil, bool, int, string, pair, symbol, keyword, tuple, map)
/// - 0x9-0xB: Callable types (function, closure, native function)
/// - 0xC-0xD: Reference types (var, namespace)
/// - 0xE: Unbound sentinel
#[derive(Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Value {
    /// The nil value (empty list, false-ish).
    #[default]
    Nil = 0,
    /// Boolean true or false.
    Bool(bool) = 1,
    /// 64-bit signed integer.
    Int(i64) = 2,
    /// Heap-allocated string (pointer to `HeapString`).
    String(Vaddr) = 3,
    /// Heap-allocated pair (pointer to `Pair`).
    Pair(Vaddr) = 4,
    /// Heap-allocated symbol (pointer to `HeapString`).
    Symbol(Vaddr) = 5,
    /// Heap-allocated keyword (pointer to `HeapString`).
    Keyword(Vaddr) = 6,
    /// Heap-allocated tuple (pointer to `HeapTuple`).
    Tuple(Vaddr) = 7,
    /// Heap-allocated map (pointer to `HeapMap`).
    Map(Vaddr) = 8,
    /// Compiled function without captures (pointer to `HeapCompiledFn`).
    CompiledFn(Vaddr) = 9,
    /// Function with captured values (pointer to `HeapClosure`).
    Closure(Vaddr) = 10,
    /// Native function (immediate value: intrinsic ID).
    NativeFn(u16) = 11,
    /// Var reference (pointer to `VarSlot` in code region).
    Var(Vaddr) = 12,
    /// Heap-allocated namespace (pointer to `Namespace`).
    Namespace(Vaddr) = 13,
    /// Sentinel for uninitialized vars (immediate, no payload).
    Unbound = 14,
}

impl Value {
    /// Create a nil value.
    #[inline]
    #[must_use]
    pub const fn nil() -> Self {
        Self::Nil
    }

    /// Create a boolean value.
    #[inline]
    #[must_use]
    pub const fn bool(b: bool) -> Self {
        Self::Bool(b)
    }

    /// Create an integer value.
    #[inline]
    #[must_use]
    pub const fn int(n: i64) -> Self {
        Self::Int(n)
    }

    /// Create a string value from a heap address.
    #[inline]
    #[must_use]
    pub const fn string(addr: Vaddr) -> Self {
        Self::String(addr)
    }

    /// Create a pair value from a heap address.
    #[inline]
    #[must_use]
    pub const fn pair(addr: Vaddr) -> Self {
        Self::Pair(addr)
    }

    /// Create a symbol value from a heap address.
    #[inline]
    #[must_use]
    pub const fn symbol(addr: Vaddr) -> Self {
        Self::Symbol(addr)
    }

    /// Create a keyword value from a heap address.
    #[inline]
    #[must_use]
    pub const fn keyword(addr: Vaddr) -> Self {
        Self::Keyword(addr)
    }

    /// Create a tuple value from a heap address.
    #[inline]
    #[must_use]
    pub const fn tuple(addr: Vaddr) -> Self {
        Self::Tuple(addr)
    }

    /// Create a map value from a heap address.
    #[inline]
    #[must_use]
    pub const fn map(addr: Vaddr) -> Self {
        Self::Map(addr)
    }

    /// Create a namespace value from a heap address.
    #[inline]
    #[must_use]
    pub const fn namespace(addr: Vaddr) -> Self {
        Self::Namespace(addr)
    }

    /// Create a var value from a code region address.
    #[inline]
    #[must_use]
    pub const fn var(addr: Vaddr) -> Self {
        Self::Var(addr)
    }

    /// Create a compiled function value from a heap address.
    #[inline]
    #[must_use]
    pub const fn compiled_fn(addr: Vaddr) -> Self {
        Self::CompiledFn(addr)
    }

    /// Create a closure value from a heap address.
    #[inline]
    #[must_use]
    pub const fn closure(addr: Vaddr) -> Self {
        Self::Closure(addr)
    }

    /// Create a native function value from an intrinsic ID.
    #[inline]
    #[must_use]
    pub const fn native_fn(id: u16) -> Self {
        Self::NativeFn(id)
    }

    /// Create an unbound sentinel value.
    #[inline]
    #[must_use]
    pub const fn unbound() -> Self {
        Self::Unbound
    }

    /// Check if this value is nil.
    #[inline]
    #[must_use]
    pub const fn is_nil(&self) -> bool {
        matches!(self, Self::Nil)
    }

    /// Check if this value is truthy (not nil and not false).
    #[inline]
    #[must_use]
    pub const fn is_truthy(&self) -> bool {
        !matches!(self, Self::Nil | Self::Bool(false))
    }

    /// Check if this value is a string.
    #[inline]
    #[must_use]
    pub const fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    /// Check if this value is a pair.
    #[inline]
    #[must_use]
    pub const fn is_pair(&self) -> bool {
        matches!(self, Self::Pair(_))
    }

    /// Check if this value is a proper list (nil or pair ending in nil).
    /// Note: This doesn't traverse the list, just checks immediate structure.
    #[inline]
    #[must_use]
    pub const fn is_list_head(&self) -> bool {
        matches!(self, Self::Nil | Self::Pair(_))
    }

    /// Check if this value is a keyword.
    #[inline]
    #[must_use]
    pub const fn is_keyword(&self) -> bool {
        matches!(self, Self::Keyword(_))
    }

    /// Check if this value is a symbol.
    #[inline]
    #[must_use]
    pub const fn is_symbol(&self) -> bool {
        matches!(self, Self::Symbol(_))
    }

    /// Check if this value is a tuple.
    #[inline]
    #[must_use]
    pub const fn is_tuple(&self) -> bool {
        matches!(self, Self::Tuple(_))
    }

    /// Check if this value is a map.
    #[inline]
    #[must_use]
    pub const fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }

    /// Check if this value is a namespace.
    #[inline]
    #[must_use]
    pub const fn is_namespace(&self) -> bool {
        matches!(self, Self::Namespace(_))
    }

    /// Check if this value is a var.
    #[inline]
    #[must_use]
    pub const fn is_var(&self) -> bool {
        matches!(self, Self::Var(_))
    }

    /// Check if this value is a compiled function.
    #[inline]
    #[must_use]
    pub const fn is_compiled_fn(&self) -> bool {
        matches!(self, Self::CompiledFn(_))
    }

    /// Check if this value is a closure.
    #[inline]
    #[must_use]
    pub const fn is_closure(&self) -> bool {
        matches!(self, Self::Closure(_))
    }

    /// Check if this value is a native function.
    #[inline]
    #[must_use]
    pub const fn is_native_fn(&self) -> bool {
        matches!(self, Self::NativeFn(_))
    }

    /// Check if this value is callable (function, closure, or native function).
    #[inline]
    #[must_use]
    pub const fn is_fn(&self) -> bool {
        matches!(
            self,
            Self::CompiledFn(_) | Self::Closure(_) | Self::NativeFn(_)
        )
    }

    /// Check if this value is the unbound sentinel.
    #[inline]
    #[must_use]
    pub const fn is_unbound(&self) -> bool {
        matches!(self, Self::Unbound)
    }

    /// Get the type name of this value for error messages.
    #[inline]
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Nil => "nil",
            Self::Bool(_) => "boolean",
            Self::Int(_) => "integer",
            Self::String(_) => "string",
            Self::Pair(_) => "pair",
            Self::Symbol(_) => "symbol",
            Self::Keyword(_) => "keyword",
            Self::Tuple(_) => "tuple",
            Self::Map(_) => "map",
            Self::CompiledFn(_) => "function",
            Self::Closure(_) => "closure",
            Self::NativeFn(_) => "native-function",
            Self::Var(_) => "var",
            Self::Namespace(_) => "namespace",
            Self::Unbound => "unbound",
        }
    }
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nil => write!(f, "Nil"),
            Self::Bool(b) => write!(f, "Bool({b})"),
            Self::Int(n) => write!(f, "Int({n})"),
            Self::String(addr) => write!(f, "String({addr:?})"),
            Self::Pair(addr) => write!(f, "Pair({addr:?})"),
            Self::Symbol(addr) => write!(f, "Symbol({addr:?})"),
            Self::Keyword(addr) => write!(f, "Keyword({addr:?})"),
            Self::Tuple(addr) => write!(f, "Tuple({addr:?})"),
            Self::Map(addr) => write!(f, "Map({addr:?})"),
            Self::CompiledFn(addr) => write!(f, "CompiledFn({addr:?})"),
            Self::Closure(addr) => write!(f, "Closure({addr:?})"),
            Self::NativeFn(id) => write!(f, "NativeFn({id})"),
            Self::Var(addr) => write!(f, "Var({addr:?})"),
            Self::Namespace(addr) => write!(f, "Namespace({addr:?})"),
            Self::Unbound => write!(f, "Unbound"),
        }
    }
}

/// Heap-allocated string header.
///
/// Stored in memory as:
/// - 4 bytes: length (u32)
/// - `len` bytes: UTF-8 data (no null terminator)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeapString {
    /// Length of the string in bytes.
    pub len: u32,
    // Followed by `len` UTF-8 bytes (not represented in struct)
}

impl HeapString {
    /// Size of the header in bytes.
    pub const HEADER_SIZE: usize = core::mem::size_of::<Self>();

    /// Calculate total allocation size for a string of given length.
    #[inline]
    #[must_use]
    pub const fn alloc_size(len: usize) -> usize {
        Self::HEADER_SIZE + len
    }
}

/// Heap-allocated pair.
///
/// Used to build lists: (1 2 3) = Pair(1, Pair(2, Pair(3, Nil)))
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Pair {
    /// First element of the pair.
    pub first: Value,
    /// Rest of the list (second element of the pair).
    pub rest: Value,
}

impl Pair {
    /// Size of a pair in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Create a new pair.
    #[inline]
    #[must_use]
    pub const fn new(first: Value, rest: Value) -> Self {
        Self { first, rest }
    }
}

/// Heap-allocated tuple header.
///
/// Stored in memory as:
/// - 4 bytes: length (u32)
/// - `len * size_of::<Value>()` bytes: elements (array of Values)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeapTuple {
    /// Number of elements in the tuple.
    pub len: u32,
    // Followed by `len` Values (not represented in struct)
}

impl HeapTuple {
    /// Size of the header in bytes.
    pub const HEADER_SIZE: usize = core::mem::size_of::<Self>();

    /// Calculate total allocation size for a tuple of given length.
    #[inline]
    #[must_use]
    pub const fn alloc_size(len: usize) -> usize {
        Self::HEADER_SIZE + len * core::mem::size_of::<Value>()
    }
}

/// Heap-allocated map header.
///
/// Maps are implemented as association lists: linked lists of `[key value]` tuples.
/// The `entries` field points to a Pair chain where each `first` is a 2-element tuple.
///
/// Stored in memory as:
/// - 16 bytes: entries (`Value::Pair` or `Value::Nil` for empty map)
///
/// Example structure for `%{:a 1 :b 2}`:
/// ```text
/// HeapMap { entries } → Pair([:a 1], Pair([:b 2], nil))
/// ```
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeapMap {
    /// Head of the association list (Pair chain or nil).
    pub entries: Value,
}

impl HeapMap {
    /// Size of the header in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

/// Heap-allocated namespace header.
///
/// Namespaces are containers for var bindings. The `name` field is a symbol,
/// and `mappings` is a `Value::Map` holding symbol→var mappings.
///
/// Stored in memory as:
/// - 16 bytes: name (`Value::Symbol`)
/// - 16 bytes: mappings (`Value::Map`)
///
/// Example: namespace `my.app` with var `x`:
/// ```text
/// Namespace { name: 'my.app, mappings: %{'x → var-addr} }
/// ```
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Namespace {
    /// The namespace name (a symbol).
    pub name: Value,
    /// Symbol→Vaddr mappings (a map). In the future, this will map to `VarSlot`s.
    /// For now, this is a map of symbol→value.
    pub mappings: Value,
}

impl Namespace {
    /// Size of the namespace header in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

/// Var flags for metadata and behavior.
pub mod var_flags {
    /// Var is process-bound (value can be shadowed per-process).
    pub const PROCESS_BOUND: u32 = 0x0001;
    /// Var is a native intrinsic (value is `NativeFn`).
    pub const NATIVE: u32 = 0x0002;
    /// Var is a macro (compile-time expansion).
    pub const MACRO: u32 = 0x0004;
    /// Var is private (not exported from namespace).
    pub const PRIVATE: u32 = 0x0008;
    /// Var is a special form (cannot be used as value).
    pub const SPECIAL_FORM: u32 = 0x0010;
}

/// Var slot - a stable, addressable reference to var content.
///
/// The `VarSlot`'s address serves as the `VarId` for process-bound lookups.
/// Updates create new `VarContent` and atomically swap the content pointer
/// using MVCC (Multi-Version Concurrency Control) semantics.
///
/// **Atomic semantics**: The `content` pointer must be read/written using
/// atomic operations with proper memory ordering:
/// - Reads: Use Acquire ordering (`MemorySpace::read_u64_acquire`)
/// - Writes: Use Release ordering (`MemorySpace::write_u64_release`)
///
/// This ensures readers always see a consistent `VarContent` - either the
/// old or new version, never a partially-written state.
///
/// Stored in memory as:
/// - 8 bytes: content pointer (`Vaddr` pointing to `VarContent`)
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VarSlot {
    /// Pointer to the current `VarContent`.
    ///
    /// Must be accessed atomically via `MemorySpace::read_u64_acquire` and
    /// `MemorySpace::write_u64_release` to ensure proper synchronization.
    pub content: Vaddr,
}

impl VarSlot {
    /// Size of the var slot in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

/// Var content - the actual binding data for a var.
///
/// `VarContent` is immutable once created. Updates create a new `VarContent`
/// and atomically swap the `VarSlot`'s pointer.
///
/// Stored in memory as:
/// - 8 bytes: name (Vaddr to interned symbol)
/// - 8 bytes: namespace (Vaddr to containing namespace)
/// - 16 bytes: root (inline Value - the root binding)
/// - 4 bytes: flags (var metadata flags)
/// - 4 bytes: padding for alignment
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct VarContent {
    /// Var name (interned symbol address).
    pub name: Vaddr,
    /// Containing namespace (Namespace address).
    pub namespace: Vaddr,
    /// Root binding value (inline, not a pointer).
    ///
    /// For process-bound vars, this is the default value when no
    /// process binding exists. Can be `Value::Unbound` for declared
    /// but uninitialized vars.
    pub root: Value,
    /// Var flags (see `var_flags` module).
    pub flags: u32,
    /// Padding for 8-byte alignment.
    pub padding: u32,
}

impl VarContent {
    /// Size of the var content in bytes.
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Check if var is process-bound.
    #[inline]
    #[must_use]
    pub const fn is_process_bound(&self) -> bool {
        self.flags & var_flags::PROCESS_BOUND != 0
    }

    /// Check if var is a native intrinsic.
    #[inline]
    #[must_use]
    pub const fn is_native(&self) -> bool {
        self.flags & var_flags::NATIVE != 0
    }

    /// Check if var is a macro.
    #[inline]
    #[must_use]
    pub const fn is_macro(&self) -> bool {
        self.flags & var_flags::MACRO != 0
    }

    /// Check if var is private.
    #[inline]
    #[must_use]
    pub const fn is_private(&self) -> bool {
        self.flags & var_flags::PRIVATE != 0
    }

    /// Check if var is a special form.
    #[inline]
    #[must_use]
    pub const fn is_special_form(&self) -> bool {
        self.flags & var_flags::SPECIAL_FORM != 0
    }
}

/// Heap-allocated compiled function header.
///
/// A compiled function is a pure function (no captures) produced by `(fn* [args] body)`.
/// Contains bytecode and metadata for execution.
///
/// Stored in memory as:
/// - Header: arity, variadic flag, locals count, code length
/// - Followed by: bytecode instructions (array of u32)
/// - Followed by: constants pool (array of Values)
///
/// Note: The constant pool is stored separately to allow variable-length bytecode.
/// Constants follow immediately after bytecode.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeapCompiledFn {
    /// Number of required parameters.
    pub arity: u8,
    /// If true, accepts variable arguments (last param collects rest).
    pub variadic: bool,
    /// Number of Y (local) registers needed.
    pub num_locals: u8,
    /// Padding byte for alignment (always 0).
    pub padding: u8,
    /// Length of bytecode in u32 instructions.
    pub code_len: u32,
    /// Number of constants in the constant pool.
    pub constants_len: u32,
    /// Source line number (0 if unknown).
    pub source_line: u32,
    /// Padding for 8-byte alignment.
    pub padding2: u32,
    /// Source file path (Vaddr to string, or `Vaddr::null()` if unknown).
    pub source_file: crate::Vaddr,
    // Followed by:
    // - `code_len` u32 instructions
    // - `constants_len` Values (constant pool)
}

impl HeapCompiledFn {
    /// Size of the header in bytes.
    pub const HEADER_SIZE: usize = core::mem::size_of::<Self>();

    /// Calculate total allocation size for a compiled function.
    ///
    /// # Arguments
    /// * `code_len` - Number of bytecode instructions (u32)
    /// * `constants_len` - Number of constants in the pool
    #[inline]
    #[must_use]
    pub const fn alloc_size(code_len: usize, constants_len: usize) -> usize {
        Self::HEADER_SIZE
            + code_len * core::mem::size_of::<u32>()
            + constants_len * core::mem::size_of::<Value>()
    }

    /// Offset from header start to the bytecode array.
    #[inline]
    #[must_use]
    pub const fn bytecode_offset() -> usize {
        Self::HEADER_SIZE
    }

    /// Offset from header start to the constants pool.
    #[inline]
    #[must_use]
    pub const fn constants_offset(code_len: usize) -> usize {
        Self::HEADER_SIZE + code_len * core::mem::size_of::<u32>()
    }
}

/// Heap-allocated closure header.
///
/// A closure is a function paired with captured values from its lexical environment.
/// Produced when `fn*` references free variables from enclosing scope.
///
/// Stored in memory as:
/// - Header: function pointer, captures count
/// - Followed by: captured values (array of Values)
///
/// The `function` field points to a `HeapCompiledFn` that contains the bytecode.
#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct HeapClosure {
    /// Pointer to the underlying `HeapCompiledFn`.
    pub function: Vaddr,
    /// Number of captured values.
    pub captures_len: u32,
    /// Padding for alignment (always 0).
    pub padding: u32,
    // Followed by `captures_len` Values (captured environment)
}

impl HeapClosure {
    /// Size of the header in bytes.
    pub const HEADER_SIZE: usize = core::mem::size_of::<Self>();

    /// Calculate total allocation size for a closure.
    ///
    /// # Arguments
    /// * `captures_len` - Number of captured values
    #[inline]
    #[must_use]
    pub const fn alloc_size(captures_len: usize) -> usize {
        Self::HEADER_SIZE + captures_len * core::mem::size_of::<Value>()
    }

    /// Offset from header start to the captures array.
    #[inline]
    #[must_use]
    pub const fn captures_offset() -> usize {
        Self::HEADER_SIZE
    }
}
