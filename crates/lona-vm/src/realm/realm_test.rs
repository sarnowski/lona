// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for the Realm module.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::platform::MockVSpace;
use crate::term::Term;
use crate::term::header::Header;
use crate::term::heap::HeapVar;
use crate::term::tag::object;

/// Check if a term is a namespace by reading its header.
fn is_term_namespace(mem: &MockVSpace, term: Term) -> bool {
    if !term.is_boxed() {
        return false;
    }
    let header: Header = mem.read(term.to_vaddr());
    header.object_tag() == object::NAMESPACE
}

/// Create a test realm with a 64KB code region.
fn create_test_realm() -> (Realm, MockVSpace) {
    let code_base = Vaddr::new(0x1000_0000);
    let code_size = 64 * 1024; // 64KB
    let realm = Realm::new(code_base, code_size);
    let mem = MockVSpace::new(code_size, code_base);
    (realm, mem)
}

// --- Allocation Tests ---

#[test]
fn test_realm_alloc_basic() {
    let (mut realm, _mem) = create_test_realm();

    // Allocate 100 bytes
    let addr1 = realm.alloc(100, 8).unwrap();
    assert!(addr1.as_u64() >= realm.code_base.as_u64());
    assert!(addr1.as_u64() < realm.code_end.as_u64());

    // Allocate another 100 bytes
    let addr2 = realm.alloc(100, 8).unwrap();
    assert!(addr2.as_u64() > addr1.as_u64());
    assert!(addr2.as_u64() >= addr1.as_u64() + 100);
}

#[test]
fn test_realm_alloc_alignment() {
    let (mut realm, _mem) = create_test_realm();

    // Allocate 1 byte (unaligned)
    let addr1 = realm.alloc(1, 1).unwrap();

    // Allocate 16 bytes with 8-byte alignment
    let addr2 = realm.alloc(16, 8).unwrap();
    assert_eq!(addr2.as_u64() % 8, 0, "address should be 8-byte aligned");

    // Allocate 32 bytes with 16-byte alignment
    let addr3 = realm.alloc(32, 16).unwrap();
    assert_eq!(addr3.as_u64() % 16, 0, "address should be 16-byte aligned");

    // Verify no overlap
    assert!(addr2.as_u64() > addr1.as_u64());
    assert!(addr3.as_u64() >= addr2.as_u64() + 16);
}

#[test]
fn test_realm_alloc_zero_size() {
    let (mut realm, _mem) = create_test_realm();

    // Zero-size allocation returns current top
    let top_before = realm.code_top;
    let addr = realm.alloc(0, 8).unwrap();
    assert_eq!(addr, top_before);
    assert_eq!(realm.code_top, top_before); // Top doesn't change
}

#[test]
fn test_realm_alloc_oom() {
    let code_base = Vaddr::new(0x1000_0000);
    let code_size = 256; // Very small region
    let mut realm = Realm::new(code_base, code_size);

    // First allocation should succeed
    assert!(realm.alloc(100, 8).is_some());

    // Second allocation should also succeed
    assert!(realm.alloc(100, 8).is_some());

    // Third allocation should fail (OOM)
    assert!(realm.alloc(100, 8).is_none());
}

#[test]
fn test_realm_code_used_and_free() {
    let (mut realm, _mem) = create_test_realm();

    let initial_free = realm.code_free();
    assert_eq!(realm.code_used(), 0);
    assert_eq!(initial_free, 64 * 1024);

    realm.alloc(1000, 8).unwrap();

    assert!(realm.code_used() >= 1000);
    assert!(realm.code_free() < initial_free);
    assert_eq!(realm.code_used() + realm.code_free(), 64 * 1024);
}

// --- Symbol Interning Tests ---

#[test]
fn test_realm_intern_symbol() {
    let (mut realm, mut mem) = create_test_realm();

    // Intern a symbol
    let sym1 = realm.intern_symbol(&mut mem, "foo").unwrap();
    assert!(sym1.is_symbol());

    // Interning the same symbol returns the same address
    let sym2 = realm.intern_symbol(&mut mem, "foo").unwrap();
    assert_eq!(sym1, sym2);

    // Different symbol gets different address
    let sym3 = realm.intern_symbol(&mut mem, "bar").unwrap();
    assert_ne!(sym1, sym3);
}

#[test]
fn test_realm_find_symbol() {
    let (mut realm, mut mem) = create_test_realm();

    // Symbol not found before interning
    assert!(realm.find_symbol(&mem, "foo").is_none());

    // Intern the symbol
    let sym = realm.intern_symbol(&mut mem, "foo").unwrap();

    // Now it can be found
    let found = realm.find_symbol(&mem, "foo").unwrap();
    assert_eq!(sym, found);

    // Other symbols still not found
    assert!(realm.find_symbol(&mem, "bar").is_none());
}

// --- Keyword Interning Tests ---

#[test]
fn test_realm_intern_keyword() {
    let (mut realm, mut mem) = create_test_realm();

    // Intern a keyword
    let kw1 = realm.intern_keyword(&mut mem, "foo").unwrap();
    assert!(kw1.is_keyword());

    // Interning the same keyword returns the same address
    let kw2 = realm.intern_keyword(&mut mem, "foo").unwrap();
    assert_eq!(kw1, kw2);

    // Different keyword gets different address
    let kw3 = realm.intern_keyword(&mut mem, "bar").unwrap();
    assert_ne!(kw1, kw3);
}

#[test]
fn test_realm_find_keyword() {
    let (mut realm, mut mem) = create_test_realm();

    // Keyword not found before interning
    assert!(realm.find_keyword(&mem, "foo").is_none());

    // Intern the keyword
    let kw = realm.intern_keyword(&mut mem, "foo").unwrap();

    // Now it can be found
    let found = realm.find_keyword(&mem, "foo").unwrap();
    assert_eq!(kw, found);
}

#[test]
fn test_symbol_and_keyword_separate() {
    let (mut realm, mut mem) = create_test_realm();

    // Symbol and keyword with same name are different values
    let sym = realm.intern_symbol(&mut mem, "foo").unwrap();
    let kw = realm.intern_keyword(&mut mem, "foo").unwrap();

    assert!(sym.is_symbol());
    assert!(kw.is_keyword());
    assert_ne!(sym, kw);
}

/// Test that interned symbols satisfy identity semantics.
///
/// Per term-representation.md: "Symbol/keyword identity via pointer equality
/// (index comparison)". Interning the same symbol name multiple times must
/// return the same Term, ensuring (identical? 'foo 'foo) => true.
#[test]
fn test_interned_symbol_identity() {
    let (mut realm, mut mem) = create_test_realm();

    // Intern symbol twice with same name
    let sym1 = realm.intern_symbol(&mut mem, "test-symbol").unwrap();
    let sym2 = realm.intern_symbol(&mut mem, "test-symbol").unwrap();

    // Must be bit-identical (same index in intern table)
    assert_eq!(
        sym1.as_raw(),
        sym2.as_raw(),
        "Interned symbols with same name must have identical bit representation"
    );

    // Extract indices and verify they're the same
    let idx1 = sym1.as_symbol_index().unwrap();
    let idx2 = sym2.as_symbol_index().unwrap();
    assert_eq!(idx1, idx2, "Interned symbols must have same index");
}

/// Test that interned keywords satisfy identity semantics.
///
/// Similar to symbols, keywords must be identical when interned with the same name.
/// This ensures (identical? :foo :foo) => true.
#[test]
fn test_interned_keyword_identity() {
    let (mut realm, mut mem) = create_test_realm();

    // Intern keyword twice with same name
    let kw1 = realm.intern_keyword(&mut mem, "test-keyword").unwrap();
    let kw2 = realm.intern_keyword(&mut mem, "test-keyword").unwrap();

    // Must be bit-identical
    assert_eq!(
        kw1.as_raw(),
        kw2.as_raw(),
        "Interned keywords with same name must have identical bit representation"
    );

    // Extract indices and verify they're the same
    let idx1 = kw1.as_keyword_index().unwrap();
    let idx2 = kw2.as_keyword_index().unwrap();
    assert_eq!(idx1, idx2, "Interned keywords must have same index");
}

// --- Namespace Registry Tests ---

#[test]
fn test_realm_namespace_registry() {
    let (mut realm, mut mem) = create_test_realm();

    // Intern namespace name
    let name = realm.intern_symbol(&mut mem, "my.ns").unwrap();

    // Create namespace
    let ns = realm.alloc_namespace(&mut mem, name).unwrap();
    assert!(is_term_namespace(&mem, ns));

    // Register it - name is now an immediate symbol Term
    let ns_addr = ns.to_vaddr();
    realm.register_namespace(name, ns_addr).unwrap();

    // Find it
    let found = realm.find_namespace(name).unwrap();
    assert_eq!(ns, found);
}

#[test]
fn test_realm_get_or_create_namespace() {
    let (mut realm, mut mem) = create_test_realm();

    // Intern namespace name
    let name = realm.intern_symbol(&mut mem, "lona.core").unwrap();

    // First call creates namespace
    let ns1 = realm.get_or_create_namespace(&mut mem, name).unwrap();
    assert!(is_term_namespace(&mem, ns1));

    // Second call returns the same namespace
    let ns2 = realm.get_or_create_namespace(&mut mem, name).unwrap();
    assert_eq!(ns1, ns2);
}

#[test]
fn test_realm_namespace_not_found() {
    let (mut realm, mut mem) = create_test_realm();

    // Intern a name but don't create namespace
    let name = realm.intern_symbol(&mut mem, "nonexistent").unwrap();

    // Namespace not found
    assert!(realm.find_namespace(name).is_none());
}

// --- Var Allocation Tests ---

#[test]
fn test_realm_alloc_var() {
    let (mut realm, mut mem) = create_test_realm();

    // Intern symbol for var name
    let name = realm.intern_symbol(&mut mem, "x").unwrap();

    // Create namespace
    let ns_name = realm.intern_symbol(&mut mem, "test.ns").unwrap();
    let ns = realm.get_or_create_namespace(&mut mem, ns_name).unwrap();

    // Create var - alloc_var now takes Term args directly
    let root = Term::small_int(42).unwrap();
    let var = realm.alloc_var(&mut mem, name, ns, root).unwrap();
    assert!(var.is_boxed()); // Vars are boxed terms

    // Read the var content directly
    let var_addr = var.to_vaddr();
    let heap_var: HeapVar = mem.read(var_addr);

    assert_eq!(heap_var.name, name);
    assert_eq!(heap_var.namespace, ns);
    assert_eq!(heap_var.root, root);
}

#[test]
fn test_realm_var_set_root() {
    let (mut realm, mut mem) = create_test_realm();

    // Create var with initial value
    let name = realm.intern_symbol(&mut mem, "x").unwrap();
    let ns_name = realm.intern_symbol(&mut mem, "test.ns").unwrap();
    let ns = realm.get_or_create_namespace(&mut mem, ns_name).unwrap();

    let root1 = Term::small_int(1).unwrap();
    let var = realm.alloc_var(&mut mem, name, ns, root1).unwrap();

    // Read initial value
    let var_addr = var.to_vaddr();
    let heap_var: HeapVar = mem.read(var_addr);
    assert_eq!(heap_var.root, root1);

    // Update the root value
    let root2 = Term::small_int(2).unwrap();
    realm.var_set_root(&mut mem, var, root2).unwrap();

    // Read updated value
    let heap_var: HeapVar = mem.read(var_addr);
    assert_eq!(heap_var.root, root2);
}

#[test]
fn test_realm_var_basic_allocation() {
    let (mut realm, mut mem) = create_test_realm();

    let name = realm.intern_symbol(&mut mem, "*ns*").unwrap();
    let ns_name = realm.intern_symbol(&mut mem, "lona.core").unwrap();
    let ns = realm.get_or_create_namespace(&mut mem, ns_name).unwrap();

    // Create var with nil root
    let var = realm.alloc_var(&mut mem, name, ns, Term::NIL).unwrap();

    // Verify var was created correctly
    let var_addr = var.to_vaddr();
    let heap_var: HeapVar = mem.read(var_addr);
    assert_eq!(heap_var.name, name);
    assert_eq!(heap_var.namespace, ns);
    assert_eq!(heap_var.root, Term::NIL);
}

// --- Metadata Table Tests ---

#[test]
fn test_realm_metadata() {
    let (mut realm, _mem) = create_test_realm();

    let obj_addr = Vaddr::new(0x1000_1000);
    let meta_addr = Vaddr::new(0x1000_2000);

    // No metadata initially
    assert!(realm.get_metadata(obj_addr).is_none());

    // Set metadata
    realm.set_metadata(obj_addr, meta_addr).unwrap();

    // Get metadata
    let found = realm.get_metadata(obj_addr).unwrap();
    assert_eq!(found, meta_addr);
}

#[test]
fn test_realm_metadata_update() {
    let (mut realm, _mem) = create_test_realm();

    let obj_addr = Vaddr::new(0x1000_1000);
    let meta_addr1 = Vaddr::new(0x1000_2000);
    let meta_addr2 = Vaddr::new(0x1000_3000);

    // Set initial metadata
    realm.set_metadata(obj_addr, meta_addr1).unwrap();
    assert_eq!(realm.get_metadata(obj_addr).unwrap(), meta_addr1);

    // Update metadata
    realm.set_metadata(obj_addr, meta_addr2).unwrap();
    assert_eq!(realm.get_metadata(obj_addr).unwrap(), meta_addr2);
}

#[test]
fn test_realm_multiple_metadata() {
    let (mut realm, _mem) = create_test_realm();

    let obj1 = Vaddr::new(0x1000_1000);
    let obj2 = Vaddr::new(0x1000_2000);
    let meta1 = Vaddr::new(0x1000_3000);
    let meta2 = Vaddr::new(0x1000_4000);

    realm.set_metadata(obj1, meta1).unwrap();
    realm.set_metadata(obj2, meta2).unwrap();

    assert_eq!(realm.get_metadata(obj1).unwrap(), meta1);
    assert_eq!(realm.get_metadata(obj2).unwrap(), meta2);
}
