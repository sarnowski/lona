// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Bootstrap sequence for initializing a realm with essential vars.
//!
//! The bootstrap creates `lona.core` and `lona.process` namespaces:
//! - `lona.core`: special forms, `*ns*`, and all intrinsics (auto-referred)
//! - `lona.process`: process intrinsics (spawn, send, link, monitor, etc.)
//!
//! Process intrinsics are registered in `lona.process` and auto-referred
//! into `lona.core` so they are accessible both qualified and unqualified.
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

use crate::intrinsics::{INTRINSIC_NAMES, PROCESS_INTRINSIC_IDS};

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
/// 1. Creates `lona.core` and `lona.process` namespaces
/// 2. Seeds special forms (`def`, `fn*`, `quote`, `do`, `var`, `match`)
/// 3. Seeds `*ns*` as a process-bound var (default = `lona.core`)
/// 4. Seeds all intrinsics as `NativeFn` vars:
///    - Process intrinsics go in `lona.process`, auto-referred into `lona.core`
///    - All other intrinsics go directly into `lona.core`
///
/// # Returns
///
/// Returns `Some(BootstrapResult)` containing the core namespace and `*ns*` var,
/// or `None` if allocation fails.
pub fn bootstrap<M: MemorySpace>(realm: &mut Realm, mem: &mut M) -> Option<BootstrapResult> {
    // Create lona.core namespace
    let core_sym = realm.intern_symbol(mem, "lona.core")?;
    let core_ns = realm.get_or_create_namespace(mem, core_sym)?;

    // Create lona.process namespace
    let process_sym = realm.intern_symbol(mem, "lona.process")?;
    let process_ns = realm.get_or_create_namespace(mem, process_sym)?;

    // === SPECIAL FORMS ===

    for &name in SPECIAL_FORM_NAMES {
        let sym = realm.intern_symbol(mem, name)?;
        let var = realm.alloc_var(mem, sym, core_ns, Term::UNBOUND)?;
        realm.add_ns_mapping(mem, core_ns, sym, var)?;
    }

    // === *ns* VAR ===

    let ns_sym = realm.intern_symbol(mem, "*ns*")?;
    let ns_var = realm.alloc_var(mem, ns_sym, core_ns, core_ns)?;
    realm.add_ns_mapping(mem, core_ns, ns_sym, ns_var)?;

    // === INTRINSICS ===

    for (id, &name) in INTRINSIC_NAMES.iter().enumerate() {
        if SPECIAL_FORM_NAMES.contains(&name) {
            continue;
        }

        let sym = realm.intern_symbol(mem, name)?;
        let native_fn = Term::native_fn(id as u16);
        let is_process = PROCESS_INTRINSIC_IDS.contains(&(id as u8));

        if is_process {
            // Process intrinsics: register in lona.process, auto-refer into lona.core
            let var = realm.alloc_var(mem, sym, process_ns, native_fn)?;
            realm.add_ns_mapping(mem, process_ns, sym, var)?;
            realm.add_ns_mapping(mem, core_ns, sym, var)?;
        } else {
            // All other intrinsics: register only in lona.core
            let var = realm.alloc_var(mem, sym, core_ns, native_fn)?;
            realm.add_ns_mapping(mem, core_ns, sym, var)?;
        }
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

/// Get the `lona.process` namespace from the realm.
pub fn get_process_ns<M: MemorySpace>(realm: &Realm, mem: &M) -> Option<Term> {
    let sym = realm.find_symbol(mem, "lona.process")?;
    realm.find_namespace(sym)
}
