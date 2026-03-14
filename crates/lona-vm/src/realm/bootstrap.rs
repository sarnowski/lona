// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Bootstrap sequence for initializing a realm with essential vars.
//!
//! The bootstrap creates the `lona.core` namespace and seeds it with:
//! - `def` - the var definition special form (hardcoded, required for bootstrap)
//! - `*ns*` - process-bound var for current namespace
//! - Other special forms: `fn*`, `quote`, `do`, `var`, `match`
//! - All intrinsics: `+`, `-`, `*`, `/`, etc.
//!
//! After bootstrap, processes can use `def` to define new vars and
//! symbols resolve via namespace lookup.

use crate::platform::MemorySpace;
use crate::term::Term;
use crate::term::header::Header;
use crate::term::heap::{HeapNamespace, HeapTuple};
use crate::term::tag::object;

use super::Realm;

/// Names of special forms that are hardcoded in the compiler.
///
/// These are recognized by name during compilation and have special evaluation
/// rules. They cannot be used as regular function values.
const SPECIAL_FORM_NAMES: &[&str] = &["def", "fn*", "quote", "do", "var", "match"];

/// Names of intrinsics in order of their IDs.
///
/// These match the order in `intrinsics/mod.rs`. Each intrinsic is registered
/// as a `NativeFn` var in `lona.core`.
const INTRINSIC_NAMES: &[&str] = &[
    "+",               // 0: ADD
    "-",               // 1: SUB
    "*",               // 2: MUL
    "/",               // 3: DIV
    "mod",             // 4: MOD
    "=",               // 5: EQ
    "<",               // 6: LT
    ">",               // 7: GT
    "<=",              // 8: LE
    ">=",              // 9: GE
    "not",             // 10: NOT
    "nil?",            // 11: IS_NIL
    "integer?",        // 12: IS_INT
    "string?",         // 13: IS_STR
    "str",             // 14: STR
    "keyword?",        // 15: IS_KEYWORD
    "keyword",         // 16: KEYWORD
    "name",            // 17: NAME
    "namespace",       // 18: NAMESPACE
    "tuple?",          // 19: IS_TUPLE
    "nth",             // 20: NTH
    "count",           // 21: COUNT
    "symbol?",         // 22: IS_SYMBOL
    "map?",            // 23: IS_MAP
    "get",             // 24: GET
    "put",             // 25: PUT
    "keys",            // 26: KEYS
    "vals",            // 27: VALS
    "meta",            // 28: META
    "with-meta",       // 29: WITH_META
    "namespace?",      // 30: IS_NAMESPACE
    "create-ns",       // 31: CREATE_NS
    "find-ns",         // 32: FIND_NS
    "ns-name",         // 33: NS_NAME
    "ns-map",          // 34: NS_MAP
    "fn?",             // 35: IS_FN
    "var?",            // 36: IS_VAR
    "intern",          // 37: INTERN
    "var-get",         // 38: VAR_GET
    "def-root",        // 39: DEF_ROOT
    "def-binding",     // 40: DEF_BINDING
    "def-meta",        // 41: DEF_META
    "vector?",         // 42: IS_VECTOR
    "first",           // 43: FIRST
    "rest",            // 44: REST
    "empty?",          // 45: IS_EMPTY
    "identical?",      // 46: IDENTICAL
    "contains?",       // 47: CONTAINS
    "garbage-collect", // 48: GARBAGE_COLLECT
    "process-info",    // 49: PROCESS_INFO
];

/// Result of bootstrapping a realm.
pub struct BootstrapResult {
    /// The `lona.core` namespace.
    pub core_ns: Term,
    /// The `*ns*` var (for process initialization).
    pub ns_var: Term,
}

/// Bootstrap the realm with essential vars.
///
/// This function:
/// 1. Creates the `lona.core` namespace
/// 2. Seeds `def` as a special form var (required for bootstrap)
/// 3. Seeds `*ns*` as a process-bound var (default = `lona.core`)
/// 4. Seeds other special forms (`fn*`, `quote`, `do`, `var`, `match`)
/// 5. Seeds all intrinsics as `NativeFn` vars
///
/// # Returns
///
/// Returns `Some(BootstrapResult)` containing the core namespace and `*ns*` var,
/// or `None` if allocation fails.
///
/// # Panics
///
/// Does not panic.
pub fn bootstrap<M: MemorySpace>(realm: &mut Realm, mem: &mut M) -> Option<BootstrapResult> {
    // Create lona.core namespace
    let core_sym = realm.intern_symbol(mem, "lona.core")?;
    let core_ns = realm.get_or_create_namespace(mem, core_sym)?;

    // === SPECIAL FORMS ===
    // These are hardcoded in the compiler and have SPECIAL_FORM flag

    for &name in SPECIAL_FORM_NAMES {
        let sym = realm.intern_symbol(mem, name)?;

        // Special forms have Unbound root - they can't be called as values
        let var = realm.alloc_var(mem, sym, core_ns, Term::UNBOUND)?;

        realm.add_ns_mapping(mem, core_ns, sym, var)?;
    }

    // === *ns* VAR ===
    // Process-bound var that holds the current namespace
    // Root value is lona.core (default for new processes)

    let ns_sym = realm.intern_symbol(mem, "*ns*")?;
    let ns_var = realm.alloc_var(mem, ns_sym, core_ns, core_ns)?; // Root = lona.core

    realm.add_ns_mapping(mem, core_ns, ns_sym, ns_var)?;

    // === INTRINSICS ===
    // Each intrinsic becomes a NativeFn var

    for (id, &name) in INTRINSIC_NAMES.iter().enumerate() {
        // Skip if it's also a special form (already registered)
        if SPECIAL_FORM_NAMES.contains(&name) {
            continue;
        }

        let sym = realm.intern_symbol(mem, name)?;

        // Root value is the NativeFn with the intrinsic ID
        let native_fn = Term::native_fn(id as u16);
        let var = realm.alloc_var(mem, sym, core_ns, native_fn)?;

        realm.add_ns_mapping(mem, core_ns, sym, var)?;
    }

    Some(BootstrapResult { core_ns, ns_var })
}

/// Look up a var by name in the given namespace.
///
/// This is a helper for finding vars during compilation. It searches
/// the namespace's mappings for a matching symbol name.
///
/// Returns the var if found, `None` otherwise.
pub fn lookup_var_in_ns<M: MemorySpace>(
    realm: &Realm,
    mem: &M,
    ns: Term,
    name: &str,
) -> Option<Term> {
    use crate::term::heap::HeapPair;

    if !ns.is_boxed() {
        return None;
    }

    let ns_addr = ns.to_vaddr();
    let header: Header = mem.read(ns_addr);
    if header.object_tag() != object::NAMESPACE {
        return None;
    }

    let ns_struct: HeapNamespace = mem.read(ns_addr);

    // Walk the mappings list (association list of pairs)
    let mut current = ns_struct.mappings;
    while current.is_list() {
        let pair_addr = current.to_vaddr();
        let pair: HeapPair = mem.read(pair_addr);

        // Each entry is a [key value] tuple
        if pair.head.is_boxed() {
            let tuple_addr = pair.head.to_vaddr();
            let tuple_header: Header = mem.read(tuple_addr);
            if tuple_header.object_tag() == object::TUPLE && tuple_header.arity() >= 2 {
                let key_addr = tuple_addr.add(HeapTuple::HEADER_SIZE as u64);
                let value_addr = key_addr.add(core::mem::size_of::<Term>() as u64);

                let key: Term = mem.read(key_addr);

                // Compare symbol name - symbols are now immediate values with indices
                if let Some(idx) = key.as_symbol_index() {
                    if let Some(sym_name) = realm.symbol_name(mem, idx) {
                        if sym_name == name {
                            let var: Term = mem.read(value_addr);
                            return Some(var);
                        }
                    }
                }
            }
        }

        current = pair.tail;
    }

    None
}

/// Get the `*ns*` var from the realm.
///
/// This is used during process initialization to set up the process binding.
pub fn get_ns_var<M: MemorySpace>(realm: &Realm, mem: &M) -> Option<Term> {
    let core_sym = realm.find_symbol(mem, "lona.core")?;
    let core_ns = realm.find_namespace(core_sym)?;
    lookup_var_in_ns(realm, mem, core_ns, "*ns*")
}

/// Get the `lona.core` namespace from the realm.
pub fn get_core_ns<M: MemorySpace>(realm: &Realm, mem: &M) -> Option<Term> {
    let core_sym = realm.find_symbol(mem, "lona.core")?;
    realm.find_namespace(core_sym)
}
