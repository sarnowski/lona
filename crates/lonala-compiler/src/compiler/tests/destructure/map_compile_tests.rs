// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for map destructuring bytecode compilation.
//!
//! These tests verify bytecode emission patterns for map destructuring.
//! They directly call compile_map_binding to test the compilation logic.

use alloc::vec;
use alloc::vec::Vec;

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, decode_bx, decode_op};
use lona_core::span::Span;
use lonala_parser::Ast;

use super::{map_elements, setup_compiler, source_id, spanned};
use crate::compiler::destructure::parse_map_pattern;

#[test]
fn compile_map_keys_emits_get_global_for_get() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern {:keys [a]}
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("keys".into()),
        Ast::Vector(vec![spanned(Ast::Symbol("a".into()))]),
    )])));
    let pattern = parse_map_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_map_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Extract the chunk and verify GetGlobal for "get" was emitted
    let chunk = core::mem::take(&mut compiler.chunk);
    let code = chunk.code();

    // Find GetGlobal instructions
    let get_globals: Vec<_> = code
        .iter()
        .copied()
        .enumerate()
        .filter(|(_idx, instr)| decode_op(*instr) == Some(Opcode::GetGlobal))
        .collect();

    // Should have at least 1 GetGlobal for "get"
    assert!(
        !get_globals.is_empty(),
        "expected at least 1 GetGlobal, got 0"
    );

    // Verify first GetGlobal loads "get" function
    let Some(&(_idx, first_get_global)) = get_globals.first() else {
        panic!("expected at least one GetGlobal instruction");
    };
    let const_idx = decode_bx(first_get_global);
    let Some(Constant::Symbol(sym_id)) = chunk.get_constant(const_idx) else {
        panic!("expected Symbol constant for GetGlobal");
    };
    assert_eq!(
        compiler.interner.resolve(*sym_id),
        "get",
        "GetGlobal should load 'get' function"
    );
}

#[test]
fn compile_map_keys_emits_loadk_for_keyword() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern {:keys [a]}
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("keys".into()),
        Ast::Vector(vec![spanned(Ast::Symbol("a".into()))]),
    )])));
    let pattern = parse_map_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_map_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Extract the chunk and verify LoadK for keyword was emitted
    let chunk = core::mem::take(&mut compiler.chunk);
    let code = chunk.code();

    // Find LoadK instructions
    let load_ks: Vec<_> = code
        .iter()
        .copied()
        .enumerate()
        .filter(|(_idx, instr)| decode_op(*instr) == Some(Opcode::LoadK))
        .collect();

    // Should have at least 1 LoadK for the keyword
    assert!(!load_ks.is_empty(), "expected at least 1 LoadK, got 0");

    // Verify LoadK loads keyword :a
    let Some(&(_idx, load_k)) = load_ks.first() else {
        panic!("expected at least one LoadK instruction");
    };
    let const_idx = decode_bx(load_k);
    let Some(Constant::Keyword(sym_id)) = chunk.get_constant(const_idx) else {
        panic!(
            "expected Keyword constant for LoadK, got {:?}",
            chunk.get_constant(const_idx)
        );
    };
    assert_eq!(
        compiler.interner.resolve(*sym_id),
        "a",
        "LoadK should load keyword :a"
    );
}

#[test]
fn compile_map_keys_emits_call_instruction() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern {:keys [a]}
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("keys".into()),
        Ast::Vector(vec![spanned(Ast::Symbol("a".into()))]),
    )])));
    let pattern = parse_map_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_map_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Extract the chunk and verify Call instruction
    let chunk = core::mem::take(&mut compiler.chunk);
    let code = chunk.code();

    // Count Call instructions
    let call_count = code
        .iter()
        .filter(|&&instr| decode_op(instr) == Some(Opcode::Call))
        .count();

    // Should have at least 1 Call for (get map key)
    assert!(
        call_count >= 1_usize,
        "expected at least 1 Call instruction, got {call_count}"
    );
}

#[test]
fn compile_map_pattern_defines_local_bindings() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern {:keys [a b]}
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("keys".into()),
        Ast::Vector(vec![
            spanned(Ast::Symbol("a".into())),
            spanned(Ast::Symbol("b".into())),
        ]),
    )])));
    let pattern = parse_map_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_map_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Verify that locals were defined for a, b
    let a_sym = compiler.interner.intern("a");
    let b_sym = compiler.interner.intern("b");

    assert!(
        compiler.locals.lookup(a_sym).is_some(),
        "local 'a' should be defined"
    );
    assert!(
        compiler.locals.lookup(b_sym).is_some(),
        "local 'b' should be defined"
    );
}

#[test]
fn compile_map_pattern_with_as_defines_as_local() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern {:keys [a] :as m}
    let ast = spanned(Ast::Map(map_elements(vec![
        (
            Ast::Keyword("keys".into()),
            Ast::Vector(vec![spanned(Ast::Symbol("a".into()))]),
        ),
        (Ast::Keyword("as".into()), Ast::Symbol("m".into())),
    ])));
    let pattern = parse_map_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_map_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Verify both a and m are defined
    let a_sym = compiler.interner.intern("a");
    let m_sym = compiler.interner.intern("m");

    assert!(
        compiler.locals.lookup(a_sym).is_some(),
        "local 'a' should be defined"
    );
    assert!(
        compiler.locals.lookup(m_sym).is_some(),
        "local 'm' (:as binding) should be defined"
    );
}

#[test]
fn compile_map_pattern_with_or_emits_conditional() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern {:keys [a] :or {a 0}}
    let ast = spanned(Ast::Map(map_elements(vec![
        (
            Ast::Keyword("keys".into()),
            Ast::Vector(vec![spanned(Ast::Symbol("a".into()))]),
        ),
        (
            Ast::Keyword("or".into()),
            Ast::Map(map_elements(vec![(
                Ast::Symbol("a".into()),
                Ast::Integer(0),
            )])),
        ),
    ])));
    let pattern = parse_map_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_map_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Extract the chunk
    let chunk = core::mem::take(&mut compiler.chunk);
    let code = chunk.code();

    // Verify Eq opcode was emitted (for nil check)
    let eq_count = code
        .iter()
        .filter(|&&instr| decode_op(instr) == Some(Opcode::Eq))
        .count();
    assert!(
        eq_count >= 1_usize,
        "expected at least 1 Eq instruction for nil check, got {eq_count}"
    );

    // Verify JumpIfNot was emitted (for conditional)
    let jump_count = code
        .iter()
        .filter(|&&instr| decode_op(instr) == Some(Opcode::JumpIfNot))
        .count();
    assert!(
        jump_count >= 1_usize,
        "expected at least 1 JumpIfNot instruction for conditional, got {jump_count}"
    );
}

#[test]
fn compile_map_strs_emits_string_constant() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern {:strs [name]}
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("strs".into()),
        Ast::Vector(vec![spanned(Ast::Symbol("name".into()))]),
    )])));
    let pattern = parse_map_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_map_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Extract the chunk and verify LoadK for string was emitted
    let chunk = core::mem::take(&mut compiler.chunk);
    let code = chunk.code();

    // Find LoadK instructions
    let load_ks: Vec<_> = code
        .iter()
        .copied()
        .enumerate()
        .filter(|(_idx, instr)| decode_op(*instr) == Some(Opcode::LoadK))
        .collect();

    // Should have at least 1 LoadK for the string
    assert!(!load_ks.is_empty(), "expected at least 1 LoadK, got 0");

    // Verify LoadK loads string "name"
    let Some(&(_idx, load_k)) = load_ks.first() else {
        panic!("expected at least one LoadK instruction");
    };
    let const_idx = decode_bx(load_k);
    let Some(Constant::String(s)) = chunk.get_constant(const_idx) else {
        panic!(
            "expected String constant for LoadK, got {:?}",
            chunk.get_constant(const_idx)
        );
    };
    assert_eq!(s, "name", "LoadK should load string \"name\"");
}

#[test]
fn compile_map_syms_emits_symbol_constant() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern {:syms [x]}
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("syms".into()),
        Ast::Vector(vec![spanned(Ast::Symbol("x".into()))]),
    )])));
    let pattern = parse_map_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_map_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Extract the chunk and verify LoadK for symbol was emitted
    let chunk = core::mem::take(&mut compiler.chunk);
    let code = chunk.code();

    // Find LoadK instructions
    let load_ks: Vec<_> = code
        .iter()
        .copied()
        .enumerate()
        .filter(|(_idx, instr)| decode_op(*instr) == Some(Opcode::LoadK))
        .collect();

    // Should have at least 1 LoadK for the symbol
    assert!(!load_ks.is_empty(), "expected at least 1 LoadK, got 0");

    // Verify LoadK loads symbol 'x
    let Some(&(_idx, load_k)) = load_ks.first() else {
        panic!("expected at least one LoadK instruction");
    };
    let const_idx = decode_bx(load_k);
    let Some(Constant::Symbol(sym_id)) = chunk.get_constant(const_idx) else {
        panic!(
            "expected Symbol constant for LoadK, got {:?}",
            chunk.get_constant(const_idx)
        );
    };
    assert_eq!(
        compiler.interner.resolve(*sym_id),
        "x",
        "LoadK should load symbol 'x"
    );
}

#[test]
fn compile_map_explicit_binding() {
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern {a :key-a}
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Symbol("a".into()),
        Ast::Keyword("key-a".into()),
    )])));
    let pattern = parse_map_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_map_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

    // Verify local 'a' is defined
    let a_sym = compiler.interner.intern("a");
    assert!(
        compiler.locals.lookup(a_sym).is_some(),
        "local 'a' should be defined"
    );

    // Extract chunk and verify GetGlobal for get was emitted
    let chunk = core::mem::take(&mut compiler.chunk);
    let code = chunk.code();

    let get_globals: Vec<_> = code
        .iter()
        .copied()
        .filter(|instr| decode_op(*instr) == Some(Opcode::GetGlobal))
        .collect();

    assert!(!get_globals.is_empty(), "expected GetGlobal for 'get'");
}

#[test]
fn compile_map_preserves_binding_registers() {
    // Regression test: binding registers must not be clobbered
    let (mut compiler, source_reg) = setup_compiler();

    // Parse pattern {:keys [a b c]}
    let ast = spanned(Ast::Map(map_elements(vec![(
        Ast::Keyword("keys".into()),
        Ast::Vector(vec![
            spanned(Ast::Symbol("a".into())),
            spanned(Ast::Symbol("b".into())),
            spanned(Ast::Symbol("c".into())),
        ]),
    )])));
    let pattern = parse_map_pattern(compiler.interner, &ast, source_id(), 0).unwrap();

    // Compile the pattern
    compiler
        .compile_map_binding(&pattern, source_reg, Span::new(0_usize, 1_usize))
        .unwrap();

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
}
