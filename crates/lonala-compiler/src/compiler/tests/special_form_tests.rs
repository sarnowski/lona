// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for special form compilation: do, if, def, let, quote, syntax-quote.

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, decode_bx, decode_op};
use lona_core::symbol;

use super::{compile_source, compile_with_interner};
use crate::compiler::{CompileError, compile};
use crate::error::Error;

// =========================================================================
// Special Form: do
// =========================================================================

#[test]
fn compile_do_empty() {
    let chunk = compile_source("(do)");
    let code = chunk.code();

    // Empty do: LoadNil R0; Return R0, 1
    assert_eq!(code.len(), 2);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadNil));
}

#[test]
fn compile_do_single() {
    let chunk = compile_source("(do 42)");
    let code = chunk.code();

    // (do 42): LoadK R0, K0; Return R0, 1
    assert_eq!(code.len(), 2);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    let const_idx = decode_bx(instr0);
    assert_eq!(chunk.get_constant(const_idx), Some(&Constant::Integer(42)));
}

#[test]
fn compile_do_multiple() {
    let chunk = compile_source("(do 1 2 3)");
    let code = chunk.code();

    // Should compile all three, but only last matters for return
    // LoadK R0, K0 (1) - discarded
    // LoadK R0, K1 (2) - discarded
    // LoadK R0, K2 (3) - returned
    // Return R0, 1
    assert_eq!(code.len(), 4);

    // Last LoadK should load 3
    let instr2 = *code.get(2_usize).unwrap();
    assert_eq!(decode_op(instr2), Some(Opcode::LoadK));
    let const_idx = decode_bx(instr2);
    assert_eq!(chunk.get_constant(const_idx), Some(&Constant::Integer(3)));
}

#[test]
fn compile_do_with_side_effects() {
    let (chunk, interner) = compile_with_interner("(do (print 1) (+ 2 3))");
    let code = chunk.code();

    // GetGlobal R0, K0 (print)
    // LoadK R1, K1 (1)
    // Call R0, 1, 1
    // Add R0, K2, K3 (2 + 3)
    // Return R0, 1
    assert_eq!(code.len(), 5);

    // Verify print symbol
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(0) {
        assert_eq!(interner.resolve(*sym_id), "print");
    } else {
        panic!("expected Symbol constant");
    }

    // Last instruction before Return should be Add
    let add_instr = *code.get(3_usize).unwrap();
    assert_eq!(decode_op(add_instr), Some(Opcode::Add));
}

// =========================================================================
// Special Form: if
// =========================================================================

#[test]
fn compile_if_true_branch() {
    let chunk = compile_source("(if true 1 2)");
    let code = chunk.code();

    // LoadTrue R0 (test)
    // JumpIfNot R0, +offset_to_else
    // LoadK R1, K0 (1)
    // Move R0, R1
    // Jump +offset_to_end
    // LoadK R1, K1 (2)
    // Move R0, R1
    // Return R0, 1
    assert_eq!(code.len(), 8);

    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadTrue));

    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::JumpIfNot));

    // Then branch loads 1 and moves to dest
    let instr2 = *code.get(2_usize).unwrap();
    assert_eq!(decode_op(instr2), Some(Opcode::LoadK));
    let then_const = decode_bx(instr2);
    assert_eq!(chunk.get_constant(then_const), Some(&Constant::Integer(1)));

    let instr3 = *code.get(3_usize).unwrap();
    assert_eq!(decode_op(instr3), Some(Opcode::Move));

    let instr4 = *code.get(4_usize).unwrap();
    assert_eq!(decode_op(instr4), Some(Opcode::Jump));

    // Else branch loads 2 and moves to dest
    let instr5 = *code.get(5_usize).unwrap();
    assert_eq!(decode_op(instr5), Some(Opcode::LoadK));
    let else_const = decode_bx(instr5);
    assert_eq!(chunk.get_constant(else_const), Some(&Constant::Integer(2)));

    let instr6 = *code.get(6_usize).unwrap();
    assert_eq!(decode_op(instr6), Some(Opcode::Move));
}

#[test]
fn compile_if_no_else() {
    let chunk = compile_source("(if false 1)");
    let code = chunk.code();

    // LoadFalse R0 (test)
    // JumpIfNot R0, +offset
    // LoadK R1, K0 (1)
    // Move R0, R1
    // Jump +offset
    // LoadNil R0 (implicit else)
    // Return R0, 1
    assert_eq!(code.len(), 7);

    // Else branch should be LoadNil
    let else_instr = *code.get(5_usize).unwrap();
    assert_eq!(decode_op(else_instr), Some(Opcode::LoadNil));
}

#[test]
fn compile_if_nested() {
    let chunk = compile_source("(if true (if false 1 2) 3)");
    let code = chunk.code();

    // This will have nested if structure
    // The outer then branch contains another if
    assert!(code.len() > 6);

    // First instruction is LoadTrue for outer test
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadTrue));
}

#[test]
fn compile_if_with_expressions() {
    let chunk = compile_source("(if (> 5 3) (+ 1 2) (- 10 5))");
    let code = chunk.code();

    // Test is (> 5 3)
    // Then is (+ 1 2)
    // Else is (- 10 5)

    // First instruction should be Gt for the test
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Gt));
}

#[test]
fn compile_if_invalid_no_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(if)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "if");
    } else {
        panic!("expected InvalidSpecialForm error for if");
    }
}

#[test]
fn compile_if_invalid_one_arg() {
    let mut interner = symbol::Interner::new();
    let result = compile("(if true)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "if");
    } else {
        panic!("expected InvalidSpecialForm error for if");
    }
}

#[test]
fn compile_if_invalid_four_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(if true 1 2 3)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "if");
    } else {
        panic!("expected InvalidSpecialForm error for if");
    }
}

// =========================================================================
// Special Form: def
// =========================================================================

#[test]
fn compile_def_simple() {
    let (chunk, interner) = compile_with_interner("(def x 42)");
    let code = chunk.code();

    // LoadK R0, K0 (42)
    // SetGlobal R0, K1 (x)
    // LoadK R0, K1 (x) - return the symbol
    // Return R0, 1
    assert_eq!(code.len(), 4);

    // First instruction loads the value
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    let value_const = decode_bx(instr0);
    assert_eq!(
        chunk.get_constant(value_const),
        Some(&Constant::Integer(42))
    );

    // Second instruction is SetGlobal
    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::SetGlobal));
    let sym_const = decode_bx(instr1);
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(sym_const) {
        assert_eq!(interner.resolve(*sym_id), "x");
    } else {
        panic!("expected Symbol constant for def name");
    }

    // Third instruction loads the symbol to return it
    let instr2 = *code.get(2_usize).unwrap();
    assert_eq!(decode_op(instr2), Some(Opcode::LoadK));
}

#[test]
fn compile_def_with_expression() {
    let (chunk, interner) = compile_with_interner("(def y (+ 1 2))");
    let code = chunk.code();

    // Add R0, K0, K1 (1 + 2)
    // SetGlobal R0, K2 (y)
    // LoadK R0, K2 (y) - return the symbol
    // Return R0, 1
    assert_eq!(code.len(), 4);

    // First instruction should be Add
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Add));

    // Check symbol name
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(2) {
        assert_eq!(interner.resolve(*sym_id), "y");
    } else {
        panic!("expected Symbol constant at K2");
    }
}

#[test]
fn compile_def_then_use() {
    let (chunk, interner) = compile_with_interner("(do (def x 10) x)");
    let code = chunk.code();

    // LoadK R0, K0 (10)
    // SetGlobal R0, K1 (x)
    // LoadK R0, K1 (x) - return from def (discarded by do)
    // GetGlobal R0, K1 (x) - lookup x
    // Return R0, 1
    assert_eq!(code.len(), 5);

    // Last instruction before Return should be GetGlobal
    let get_global = *code.get(3_usize).unwrap();
    assert_eq!(decode_op(get_global), Some(Opcode::GetGlobal));

    let sym_const = decode_bx(get_global);
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(sym_const) {
        assert_eq!(interner.resolve(*sym_id), "x");
    } else {
        panic!("expected Symbol constant");
    }
}

#[test]
fn compile_def_multiple() {
    let chunk = compile_source("(do (def a 1) (def b 2) (+ a b))");
    let code = chunk.code();

    // Should have SetGlobal for both a and b, then Add
    let mut set_global_count = 0_usize;
    let mut has_add = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::SetGlobal) {
            set_global_count = set_global_count.saturating_add(1);
        }
        if decode_op(instr) == Some(Opcode::Add) {
            has_add = true;
        }
    }
    assert_eq!(set_global_count, 2);
    assert!(has_add);
}

#[test]
fn compile_def_invalid_no_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(def)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "def");
    } else {
        panic!("expected InvalidSpecialForm error for def");
    }
}

#[test]
fn compile_def_invalid_one_arg() {
    let mut interner = symbol::Interner::new();
    let result = compile("(def x)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "def");
    } else {
        panic!("expected InvalidSpecialForm error for def");
    }
}

#[test]
fn compile_def_invalid_three_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(def x 1 2)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "def");
    } else {
        panic!("expected InvalidSpecialForm error for def");
    }
}

#[test]
fn compile_def_invalid_non_symbol_name() {
    let mut interner = symbol::Interner::new();
    let result = compile("(def 42 1)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, message, .. })) = result {
        assert_eq!(form, "def");
        assert!(message.contains("symbol"));
    } else {
        panic!("expected InvalidSpecialForm error for def with non-symbol name");
    }
}

// =========================================================================
// Special Form: let
// =========================================================================

#[test]
fn compile_let_single_binding() {
    let chunk = compile_source("(let [x 42] x)");
    let code = chunk.code();

    // Should have instructions for: load 42, move to binding, move from binding, return
    assert!(code.len() >= 2);

    // Should use Move opcode for local variable access
    let mut has_move = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Move) {
            has_move = true;
            break;
        }
    }
    assert!(has_move);
}

#[test]
fn compile_let_multiple_bindings() {
    let chunk = compile_source("(let [x 1 y 2] (+ x y))");
    let code = chunk.code();

    // Should have Add instruction
    let mut has_add = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Add) {
            has_add = true;
            break;
        }
    }
    assert!(has_add);
}

#[test]
fn compile_let_forward_reference() {
    let chunk = compile_source("(let [x 1 y (+ x 1)] y)");
    let code = chunk.code();

    // Should have Add instruction for (+ x 1)
    let mut has_add = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Add) {
            has_add = true;
            break;
        }
    }
    assert!(has_add);
}

#[test]
fn compile_let_nested() {
    let chunk = compile_source("(let [x 1] (let [y 2] (+ x y)))");
    let code = chunk.code();

    // Should have Add instruction
    let mut has_add = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::Add) {
            has_add = true;
            break;
        }
    }
    assert!(has_add);
}

#[test]
fn compile_let_empty_body() {
    let chunk = compile_source("(let [x 1])");
    let code = chunk.code();

    // Empty body should produce LoadNil
    let mut has_load_nil = false;
    for &instr in code {
        if decode_op(instr) == Some(Opcode::LoadNil) {
            has_load_nil = true;
            break;
        }
    }
    assert!(has_load_nil);
}

#[test]
fn compile_let_invalid_no_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(let)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "let");
    } else {
        panic!("expected InvalidSpecialForm error for let");
    }
}

#[test]
fn compile_let_invalid_non_vector_bindings() {
    let mut interner = symbol::Interner::new();
    let result = compile("(let (x 1) x)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, message, .. })) = result {
        assert_eq!(form, "let");
        assert!(message.contains("vector"));
    } else {
        panic!("expected InvalidSpecialForm error for let with non-vector bindings");
    }
}

#[test]
fn compile_let_invalid_odd_bindings() {
    let mut interner = symbol::Interner::new();
    let result = compile("(let [x 1 y] x)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, message, .. })) = result {
        assert_eq!(form, "let");
        assert!(message.contains("pairs"));
    } else {
        panic!("expected InvalidSpecialForm error for let with odd bindings");
    }
}

#[test]
fn compile_let_invalid_non_symbol_binding_name() {
    let mut interner = symbol::Interner::new();
    let result = compile("(let [42 1] 0)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, message, .. })) = result {
        assert_eq!(form, "let");
        assert!(message.contains("symbol"));
    } else {
        panic!("expected InvalidSpecialForm error for let with non-symbol binding name");
    }
}

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
    let mut interner = symbol::Interner::new();
    let result = compile("(quote)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "quote");
    } else {
        panic!("expected InvalidSpecialForm error for quote");
    }
}

#[test]
fn compile_quote_invalid_two_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(quote a b)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
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
    let mut interner = symbol::Interner::new();
    let result = compile("~x", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, message, .. })) = result {
        assert_eq!(form, "unquote");
        assert!(message.contains("not inside syntax-quote"));
    } else {
        panic!("expected InvalidSpecialForm error for unquote outside syntax-quote");
    }
}

#[test]
fn compile_unquote_splicing_outside_syntax_quote_error() {
    let mut interner = symbol::Interner::new();
    let result = compile("~@x", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, message, .. })) = result {
        assert_eq!(form, "unquote-splicing");
        assert!(message.contains("not inside syntax-quote"));
    } else {
        panic!("expected InvalidSpecialForm error for unquote-splicing outside syntax-quote");
    }
}

#[test]
fn compile_syntax_quote_invalid_no_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(syntax-quote)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "syntax-quote");
    } else {
        panic!("expected InvalidSpecialForm error for syntax-quote");
    }
}

#[test]
fn compile_syntax_quote_invalid_two_args() {
    let mut interner = symbol::Interner::new();
    let result = compile("(syntax-quote a b)", &mut interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error::InvalidSpecialForm { form, .. })) = result {
        assert_eq!(form, "syntax-quote");
    } else {
        panic!("expected InvalidSpecialForm error for syntax-quote with two args");
    }
}
