// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for quote special forms: quote and syntax-quote (quasiquote).

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, decode_bx, decode_op};
use lona_core::symbol;

use super::{TEST_SOURCE_ID, compile_source, compile_with_interner};
use crate::compiler::CompileError;
use crate::compiler::compile;
use crate::error::{Error, Kind as ErrorKind};

// =========================================================================
// Special Form: quote
// =========================================================================

#[test]
fn compile_quote_symbol() {
    let (chunk, interner) = compile_with_interner("(quote foo)");
    let code = chunk.code();

    // Should have LoadK instruction
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));

    // Verify symbol constant
    let const_idx = decode_bx(instr0);
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(const_idx) {
        assert_eq!(interner.resolve(*sym_id), "foo");
    } else {
        panic!("expected Symbol constant for quoted symbol");
    }
}

#[test]
fn compile_quote_integer() {
    let chunk = compile_source("(quote 42)");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));

    let const_idx = decode_bx(instr0);
    assert_eq!(chunk.get_constant(const_idx), Some(&Constant::Integer(42)));
}

#[test]
fn compile_quote_list() {
    let chunk = compile_source("(quote (1 2 3))");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));

    let const_idx = decode_bx(instr0);
    if let Some(Constant::List(elements)) = chunk.get_constant(const_idx) {
        assert_eq!(elements.len(), 3);
        assert_eq!(elements.get(0), Some(&Constant::Integer(1)));
        assert_eq!(elements.get(1), Some(&Constant::Integer(2)));
        assert_eq!(elements.get(2), Some(&Constant::Integer(3)));
    } else {
        panic!("expected List constant for quoted list");
    }
}

#[test]
fn compile_quote_vector() {
    let chunk = compile_source("(quote [1 2 3])");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));

    let const_idx = decode_bx(instr0);
    if let Some(Constant::Vector(elements)) = chunk.get_constant(const_idx) {
        assert_eq!(elements.len(), 3);
        assert_eq!(elements.get(0), Some(&Constant::Integer(1)));
        assert_eq!(elements.get(1), Some(&Constant::Integer(2)));
        assert_eq!(elements.get(2), Some(&Constant::Integer(3)));
    } else {
        panic!("expected Vector constant for quoted vector");
    }
}

#[test]
fn compile_quote_nested_list() {
    let chunk = compile_source("(quote (a (b c) d))");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));

    let const_idx = decode_bx(instr0);
    if let Some(Constant::List(elements)) = chunk.get_constant(const_idx) {
        assert_eq!(elements.len(), 3);
        // First element is symbol 'a'
        assert!(matches!(elements.get(0), Some(Constant::Symbol(_))));
        // Second element is list (b c)
        if let Some(Constant::List(inner)) = elements.get(1) {
            assert_eq!(inner.len(), 2);
        } else {
            panic!("expected nested list");
        }
        // Third element is symbol 'd'
        assert!(matches!(elements.get(2), Some(Constant::Symbol(_))));
    } else {
        panic!("expected List constant for quoted nested list");
    }
}

#[test]
fn compile_quote_prevents_evaluation() {
    let (chunk, interner) = compile_with_interner("(quote (+ 1 2))");
    let code = chunk.code();

    // Should NOT have Add instruction - quote prevents evaluation
    let mut has_add = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Add) {
            has_add = true;
            break;
        }
    }
    assert!(!has_add, "quote should prevent evaluation");

    // Should have a List constant with symbol '+' and integers 1, 2
    let instr0 = *code.get(0_usize).unwrap();
    let const_idx = decode_bx(instr0);
    if let Some(Constant::List(elements)) = chunk.get_constant(const_idx) {
        assert_eq!(elements.len(), 3);
        // First element should be symbol '+'
        if let Some(Constant::Symbol(sym_id)) = elements.get(0) {
            assert_eq!(interner.resolve(*sym_id), "+");
        } else {
            panic!("expected symbol '+' in quoted list");
        }
    } else {
        panic!("expected List constant for quoted expression");
    }
}

#[test]
fn compile_quote_invalid_no_args() {
    let interner = symbol::Interner::new();
    let result = compile("(quote)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "quote");
    } else {
        panic!("expected InvalidSpecialForm error for quote");
    }
}

#[test]
fn compile_quote_invalid_two_args() {
    let interner = symbol::Interner::new();
    let result = compile("(quote a b)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "quote");
    } else {
        panic!("expected InvalidSpecialForm error for quote with two args");
    }
}

// =========================================================================
// Special Form: syntax-quote (quasiquote)
// =========================================================================

#[test]
fn compile_syntax_quote_literal() {
    // Literals are quoted
    let chunk = compile_source("`42");
    let code = chunk.code();

    // Should generate (quote 42)
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    let const_idx = decode_bx(instr0);
    assert_eq!(chunk.get_constant(const_idx), Some(&Constant::Integer(42)));
}

#[test]
fn compile_syntax_quote_symbol() {
    // Symbols are quoted
    let (chunk, interner) = compile_with_interner("`foo");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    let const_idx = decode_bx(instr0);
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(const_idx) {
        assert_eq!(interner.resolve(*sym_id), "foo");
    } else {
        panic!("expected Symbol constant for syntax-quoted symbol");
    }
}

#[test]
fn compile_syntax_quote_empty_list() {
    // Empty list stays empty
    let chunk = compile_source("`()");
    let code = chunk.code();

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    let const_idx = decode_bx(instr0);
    if let Some(Constant::List(elements)) = chunk.get_constant(const_idx) {
        assert!(elements.is_empty());
    } else {
        panic!("expected empty List constant");
    }
}

#[test]
fn compile_syntax_quote_simple_list() {
    // `(a b) expands to (list 'a 'b)
    let chunk = compile_source("`(a b)");
    let code = chunk.code();

    // Should have Call instruction for (list ...)
    let mut has_call = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Call) {
            has_call = true;
            break;
        }
    }
    assert!(
        has_call,
        "syntax-quote should generate function calls to list"
    );
}

#[test]
fn compile_unquote_in_syntax_quote() {
    // `(a ~x) with unquote should evaluate x
    let chunk = compile_source("(let [x 1] `(a ~x))");
    let code = chunk.code();

    // Should have Call instruction for list
    let mut has_call = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Call) {
            has_call = true;
            break;
        }
    }
    assert!(
        has_call,
        "unquote should be compiled in syntax-quote context"
    );
}

#[test]
fn compile_unquote_splicing_in_syntax_quote() {
    // `(a ~@xs b) should use concat
    let chunk = compile_source("(let [xs (vector 1 2)] `(a ~@xs b))");
    let code = chunk.code();

    // Should have multiple Call instructions (for concat, list)
    let mut call_count = 0_usize;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Call) {
            call_count = call_count.saturating_add(1);
        }
    }
    // At least 2 calls: vector constructor and the concat/list calls
    assert!(
        call_count >= 2,
        "unquote-splicing should generate multiple function calls"
    );
}

#[test]
fn compile_syntax_quote_vector() {
    // `[a b] should produce a vector
    let chunk = compile_source("`[a b]");
    let code = chunk.code();

    // Should have Call instruction for vec
    let mut has_call = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Call) {
            has_call = true;
            break;
        }
    }
    assert!(has_call, "syntax-quote vector should generate calls to vec");
}

#[test]
fn compile_unquote_outside_syntax_quote_error() {
    let interner = symbol::Interner::new();
    let result = compile("~x", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, message },
        ..
    })) = result
    {
        assert_eq!(form, "unquote");
        assert!(message.contains("not inside syntax-quote"));
    } else {
        panic!("expected InvalidSpecialForm error for unquote outside syntax-quote");
    }
}

#[test]
fn compile_unquote_splicing_outside_syntax_quote_error() {
    let interner = symbol::Interner::new();
    let result = compile("~@x", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, message },
        ..
    })) = result
    {
        assert_eq!(form, "unquote-splicing");
        assert!(message.contains("not inside syntax-quote"));
    } else {
        panic!("expected InvalidSpecialForm error for unquote-splicing outside syntax-quote");
    }
}

#[test]
fn compile_syntax_quote_invalid_no_args() {
    let interner = symbol::Interner::new();
    let result = compile("(syntax-quote)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "syntax-quote");
    } else {
        panic!("expected InvalidSpecialForm error for syntax-quote");
    }
}

#[test]
fn compile_syntax_quote_invalid_two_args() {
    let interner = symbol::Interner::new();
    let result = compile("(syntax-quote a b)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "syntax-quote");
    } else {
        panic!("expected InvalidSpecialForm error for syntax-quote with two args");
    }
}
