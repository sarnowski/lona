// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for sequential destructuring bytecode compilation.
//!
//! These tests verify bytecode emission patterns for sequential destructuring.
//! They directly call compile_sequential_binding to test the compilation logic.

use alloc::vec;
use alloc::vec::Vec;

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, decode_bx, decode_op};
use lona_core::span::Span;
use lonala_parser::Ast;

use super::{setup_compiler, source_id, spanned};
use crate::compiler::destructure::parse_sequential_pattern;

#[test]
fn compile_simple_pattern_emits_get_global_for_first() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern [a]
    let ast = spanned(Ast::Vector(vec![spanned(Ast::Symbol("a".into()))]));
    let pattern = parse_sequential_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_sequential_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Extract the chunk and verify GetGlobal for "first" was emitted
    let chunk = core::mem::take(&mut compiler.chunk);
    let code = chunk.code();

    // Find GetGlobal instructions
    let get_globals: Vec<_> = code
        .iter()
        .copied()
        .enumerate()
        .filter(|(_idx, instr)| decode_op(*instr) == Some(Opcode::GetGlobal))
        .collect();

    // Should have at least 2 GetGlobal: one for "first", one for "rest"
    assert!(
        get_globals.len() >= 2_usize,
        "expected at least 2 GetGlobal, got {}",
        get_globals.len()
    );

    // Verify first GetGlobal loads "first" function
    let Some(&(_idx, first_get_global)) = get_globals.first() else {
        panic!("expected at least one GetGlobal instruction");
    };
    let const_idx = decode_bx(first_get_global);
    let Some(Constant::Symbol(sym_id)) = chunk.get_constant(const_idx) else {
        panic!("expected Symbol constant for GetGlobal");
    };
    assert_eq!(
        compiler.interner.resolve(*sym_id),
        "first",
        "first GetGlobal should load 'first' function"
    );
}

#[test]
fn compile_simple_pattern_emits_get_global_for_rest() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern [a]
    let ast = spanned(Ast::Vector(vec![spanned(Ast::Symbol("a".into()))]));
    let pattern = parse_sequential_pattern(compiler.interner, &ast, source_id(), 0)
        .expect("pattern should parse");

    // Compile the pattern
    compiler
        .compile_sequential_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Extract the chunk and verify GetGlobal for "rest" was emitted
    let chunk = core::mem::take(&mut compiler.chunk);
    let code = chunk.code();

    // Find GetGlobal instructions
    let get_globals: Vec<_> = code
        .iter()
        .copied()
        .enumerate()
        .filter(|(_idx, instr)| decode_op(*instr) == Some(Opcode::GetGlobal))
        .collect();

    // Should have at least 2 GetGlobal
    assert!(
        get_globals.len() >= 2_usize,
        "expected at least 2 GetGlobal"
    );

    // Verify second GetGlobal loads "rest" function
    let (_idx, rest_get_global) = *get_globals.get(1_usize).unwrap();
    let const_idx = decode_bx(rest_get_global);
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(const_idx) {
        assert_eq!(
            compiler.interner.resolve(*sym_id),
            "rest",
            "second GetGlobal should load 'rest' function"
        );
    } else {
        panic!("expected Symbol constant for GetGlobal");
    }
}

#[test]
fn compile_simple_pattern_emits_call_instructions() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern [a]
    let ast = spanned(Ast::Vector(vec![spanned(Ast::Symbol("a".into()))]));
    let pattern = parse_sequential_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_sequential_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Extract the chunk and verify Call instructions
    let chunk = core::mem::take(&mut compiler.chunk);
    let code = chunk.code();

    // Count Call instructions
    let call_count = code
        .iter()
        .filter(|&&instr| decode_op(instr) == Some(Opcode::Call))
        .count();

    // Should have at least 2 Call: one for (first ...), one for (rest ...)
    assert!(
        call_count >= 2_usize,
        "expected at least 2 Call instructions, got {call_count}"
    );
}

#[test]
fn compile_pattern_defines_local_bindings() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern [a b c]
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Symbol("b".into())),
        spanned(Ast::Symbol("c".into())),
    ]));
    let pattern = parse_sequential_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_sequential_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Verify that locals were defined for a, b, c
    let a_sym = compiler.interner.intern("a");
    let b_sym = compiler.interner.intern("b");
    let c_sym = compiler.interner.intern("c");

    assert!(
        compiler.locals.lookup(a_sym).is_some(),
        "local 'a' should be defined"
    );
    assert!(
        compiler.locals.lookup(b_sym).is_some(),
        "local 'b' should be defined"
    );
    assert!(
        compiler.locals.lookup(c_sym).is_some(),
        "local 'c' should be defined"
    );
}

#[test]
fn compile_pattern_with_rest_defines_rest_local() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern [a & r]
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Symbol("&".into())),
        spanned(Ast::Symbol("r".into())),
    ]));
    let pattern = parse_sequential_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_sequential_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Verify both a and r are defined
    let a_sym = compiler.interner.intern("a");
    let r_sym = compiler.interner.intern("r");

    assert!(
        compiler.locals.lookup(a_sym).is_some(),
        "local 'a' should be defined"
    );
    assert!(
        compiler.locals.lookup(r_sym).is_some(),
        "local 'r' (rest binding) should be defined"
    );
}

#[test]
fn compile_pattern_with_as_defines_as_local() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern [a :as all]
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Keyword("as".into())),
        spanned(Ast::Symbol("all".into())),
    ]));
    let pattern = parse_sequential_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_sequential_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Verify both a and all are defined
    let a_sym = compiler.interner.intern("a");
    let all_sym = compiler.interner.intern("all");

    assert!(
        compiler.locals.lookup(a_sym).is_some(),
        "local 'a' should be defined"
    );
    assert!(
        compiler.locals.lookup(all_sym).is_some(),
        "local 'all' (:as binding) should be defined"
    );
}

#[test]
fn compile_ignore_does_not_define_local() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern [a _ c]
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Symbol("_".into())),
        spanned(Ast::Symbol("c".into())),
    ]));
    let pattern = parse_sequential_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_sequential_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Verify a and c are defined, but _ is not
    let a_sym = compiler.interner.intern("a");
    let underscore_sym = compiler.interner.intern("_");
    let c_sym = compiler.interner.intern("c");

    assert!(
        compiler.locals.lookup(a_sym).is_some(),
        "local 'a' should be defined"
    );
    assert!(
        compiler.locals.lookup(underscore_sym).is_none(),
        "local '_' should NOT be defined (it's ignore)"
    );
    assert!(
        compiler.locals.lookup(c_sym).is_some(),
        "local 'c' should be defined"
    );
}

#[test]
fn compile_nested_pattern_emits_recursive_calls() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern [[x y] z]
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Vector(vec![
            spanned(Ast::Symbol("x".into())),
            spanned(Ast::Symbol("y".into())),
        ])),
        spanned(Ast::Symbol("z".into())),
    ]));
    let pattern = parse_sequential_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_sequential_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Extract the chunk
    let chunk = core::mem::take(&mut compiler.chunk);
    let code = chunk.code();

    // Count GetGlobal for "first" - should be 3 total:
    // 1 for outer first element, 2 for inner [x y]
    let first_count = code
        .iter()
        .copied()
        .filter(|instr| {
            if decode_op(*instr) != Some(Opcode::GetGlobal) {
                return false;
            }
            let const_idx = decode_bx(*instr);
            matches!(
                chunk.get_constant(const_idx),
                Some(Constant::Symbol(sym_id)) if compiler.interner.resolve(*sym_id) == "first"
            )
        })
        .count();

    assert!(
        first_count >= 3_usize,
        "expected at least 3 'first' calls for nested pattern, got {first_count}"
    );

    // Verify all locals are defined
    let x_sym = compiler.interner.intern("x");
    let y_sym = compiler.interner.intern("y");
    let z_sym = compiler.interner.intern("z");

    assert!(
        compiler.locals.lookup(x_sym).is_some(),
        "local 'x' should be defined"
    );
    assert!(
        compiler.locals.lookup(y_sym).is_some(),
        "local 'y' should be defined"
    );
    assert!(
        compiler.locals.lookup(z_sym).is_some(),
        "local 'z' should be defined"
    );
}

#[test]
fn compile_preserves_binding_registers() {
    // Regression test: binding registers must not be clobbered by
    // subsequent allocations within the same destructuring pattern.
    //
    // Pattern [a b c] should allocate distinct registers for a, b, c
    // that remain valid after destructuring completes.
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern [a b c]
    let ast = spanned(Ast::Vector(vec![
        spanned(Ast::Symbol("a".into())),
        spanned(Ast::Symbol("b".into())),
        spanned(Ast::Symbol("c".into())),
    ]));
    let pattern = parse_sequential_pattern(compiler.interner, &ast, source_id(), 0)
        .expect("pattern should parse");

    // Compile the pattern
    compiler
        .compile_sequential_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .expect("compilation should succeed");

    // Get the registers assigned to each binding
    let a_sym = compiler.interner.intern("a");
    let b_sym = compiler.interner.intern("b");
    let c_sym = compiler.interner.intern("c");

    let Some(a_reg) = compiler.locals.lookup(a_sym) else {
        panic!("local 'a' should be defined");
    };
    let Some(b_reg) = compiler.locals.lookup(b_sym) else {
        panic!("local 'b' should be defined");
    };
    let Some(c_reg) = compiler.locals.lookup(c_sym) else {
        panic!("local 'c' should be defined");
    };

    // All binding registers must be distinct
    assert_ne!(a_reg, b_reg, "a and b must have different registers");
    assert_ne!(b_reg, c_reg, "b and c must have different registers");
    assert_ne!(a_reg, c_reg, "a and c must have different registers");

    // All binding registers must be >= source_reg (not overwriting source)
    assert!(a_reg > source_reg, "a's register must be above source_reg");
    assert!(b_reg > source_reg, "b's register must be above source_reg");
    assert!(c_reg > source_reg, "c's register must be above source_reg");

    // The allocator's next_register should be above all bindings
    // This ensures subsequent allocations won't clobber our bindings
    let max_binding_reg = a_reg.max(b_reg).max(c_reg);
    assert!(
        compiler.next_register > max_binding_reg,
        "next_register ({}) should be > max binding register ({})",
        compiler.next_register,
        max_binding_reg
    );
}
