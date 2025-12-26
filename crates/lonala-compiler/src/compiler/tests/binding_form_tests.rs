// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for binding special forms: def and let.

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, decode_bx, decode_op};
use lona_core::symbol;

use super::{TEST_SOURCE_ID, compile_source, compile_with_interner};
use crate::compiler::CompileError;
use crate::compiler::compile;
use crate::error::{Error, Kind as ErrorKind};

// =========================================================================
// Special Form: def
// =========================================================================

#[test]
fn compile_def_simple() {
    let (chunk, interner) = compile_with_interner("(def x 42)");
    let code = chunk.code();

    // LoadK R0, K0 (42)
    // SetGlobal R0, K1 (x)
    // LoadK R1, K2 (metadata map)
    // SetGlobalMeta R1, K1 (x)
    // LoadK R0, K1 (x) - return the symbol
    // Return R0, 1
    assert_eq!(code.len(), 6);

    // First instruction loads the value
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::LoadK));
    let value_const = decode_bx(instr0);
    assert_eq!(
        chunk.get_constant(value_const),
        Some(&Constant::Integer(42))
    );

    // Second instruction is SetGlobal (symbol is namespace-qualified)
    let instr1 = *code.get(1_usize).unwrap();
    assert_eq!(decode_op(instr1), Some(Opcode::SetGlobal));
    let sym_const = decode_bx(instr1);
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(sym_const) {
        assert_eq!(interner.resolve(*sym_id), "user/x");
    } else {
        panic!("expected Symbol constant for def name");
    }

    // Third instruction loads the metadata map
    let instr2 = *code.get(2_usize).unwrap();
    assert_eq!(decode_op(instr2), Some(Opcode::LoadK));

    // Fourth instruction is SetGlobalMeta
    let instr3 = *code.get(3_usize).unwrap();
    assert_eq!(decode_op(instr3), Some(Opcode::SetGlobalMeta));

    // Fifth instruction loads the symbol to return it
    let instr4 = *code.get(4_usize).unwrap();
    assert_eq!(decode_op(instr4), Some(Opcode::LoadK));
}

#[test]
fn compile_def_with_expression() {
    let (chunk, interner) = compile_with_interner("(def y (+ 1 2))");
    let code = chunk.code();

    // Add R0, K0, K1 (1 + 2)
    // SetGlobal R0, K2 (y)
    // LoadK R1, K3 (metadata map)
    // SetGlobalMeta R1, K2 (y)
    // LoadK R0, K2 (y) - return the symbol
    // Return R0, 1
    assert_eq!(code.len(), 6);

    // First instruction should be Add
    let instr0 = *code.get(0_usize).unwrap();
    assert_eq!(decode_op(instr0), Some(Opcode::Add));

    // Check symbol name (namespace-qualified)
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(2) {
        assert_eq!(interner.resolve(*sym_id), "user/y");
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
    // LoadK R1, K2 (metadata map)
    // SetGlobalMeta R1, K1 (x)
    // LoadK R0, K1 (x) - return from def (discarded by do)
    // GetGlobal R0, K1 (x) - lookup x
    // Return R0, 1
    assert_eq!(code.len(), 7);

    // Instruction before Return should be GetGlobal (at index 5)
    let get_global = *code.get(5_usize).unwrap();
    assert_eq!(decode_op(get_global), Some(Opcode::GetGlobal));

    // Symbol lookup is qualified with current namespace (user/x)
    let sym_const = decode_bx(get_global);
    if let Some(Constant::Symbol(sym_id)) = chunk.get_constant(sym_const) {
        assert_eq!(interner.resolve(*sym_id), "user/x");
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
    let interner = symbol::Interner::new();
    let result = compile("(def)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "def");
    } else {
        panic!("expected InvalidSpecialForm error for def");
    }
}

#[test]
fn compile_def_invalid_one_arg() {
    let interner = symbol::Interner::new();
    let result = compile("(def x)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "def");
    } else {
        panic!("expected InvalidSpecialForm error for def");
    }
}

#[test]
fn compile_def_invalid_three_args() {
    let interner = symbol::Interner::new();
    let result = compile("(def x 1 2)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "def");
    } else {
        panic!("expected InvalidSpecialForm error for def");
    }
}

#[test]
fn compile_def_invalid_non_symbol_name() {
    let interner = symbol::Interner::new();
    let result = compile("(def 42 1)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, message },
        ..
    })) = result
    {
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
    let interner = symbol::Interner::new();
    let result = compile("(let)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, .. },
        ..
    })) = result
    {
        assert_eq!(form, "let");
    } else {
        panic!("expected InvalidSpecialForm error for let");
    }
}

#[test]
fn compile_let_invalid_non_vector_bindings() {
    let interner = symbol::Interner::new();
    let result = compile("(let (x 1) x)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, message },
        ..
    })) = result
    {
        assert_eq!(form, "let");
        assert!(message.contains("vector"));
    } else {
        panic!("expected InvalidSpecialForm error for let with non-vector bindings");
    }
}

#[test]
fn compile_let_invalid_odd_bindings() {
    let interner = symbol::Interner::new();
    let result = compile("(let [x 1 y] x)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, message },
        ..
    })) = result
    {
        assert_eq!(form, "let");
        assert!(message.contains("pairs"));
    } else {
        panic!("expected InvalidSpecialForm error for let with odd bindings");
    }
}

#[test]
fn compile_let_invalid_non_symbol_binding_name() {
    let interner = symbol::Interner::new();
    let result = compile("(let [42 1] 0)", TEST_SOURCE_ID, &interner);
    assert!(result.is_err());

    if let Err(CompileError::Compile(Error {
        kind: ErrorKind::InvalidSpecialForm { form, message },
        ..
    })) = result
    {
        assert_eq!(form, "let");
        assert!(message.contains("symbol") || message.contains("vector pattern"));
    } else {
        panic!("expected InvalidSpecialForm error for let with non-symbol binding name");
    }
}

// =========================================================================
// Special Form: let with destructuring
// =========================================================================

#[test]
fn compile_let_destructure_simple() {
    // (let [[a b] [1 2]] a) should compile successfully
    let chunk = compile_source("(let [[a b] [1 2]] a)");
    let code = chunk.code();

    // Should have GetGlobal for "first" and "rest"
    let get_global_count = code
        .iter()
        .filter(|&&instr| decode_op(instr) == Some(Opcode::GetGlobal))
        .count();
    assert!(
        get_global_count >= 2_usize,
        "expected at least 2 GetGlobal instructions for first/rest, got {get_global_count}"
    );
}

#[test]
fn compile_let_destructure_with_rest() {
    // (let [[a & r] [1 2 3]] r) should compile successfully
    let chunk = compile_source("(let [[a & r] [1 2 3]] r)");
    let code = chunk.code();

    // Should have Call instructions for first/rest
    let call_count = code
        .iter()
        .filter(|&&instr| decode_op(instr) == Some(Opcode::Call))
        .count();
    assert!(
        call_count >= 1_usize,
        "expected at least 1 Call instruction, got {call_count}"
    );
}

#[test]
fn compile_let_destructure_with_ignore() {
    // (let [[a _ c] [1 2 3]] c) should compile successfully
    let chunk = compile_source("(let [[a _ c] [1 2 3]] c)");
    let code = chunk.code();

    // Should have GetGlobal instructions
    let has_get_global = code
        .iter()
        .any(|&instr| decode_op(instr) == Some(Opcode::GetGlobal));
    assert!(
        has_get_global,
        "expected GetGlobal instructions for first/rest"
    );
}

#[test]
fn compile_let_destructure_with_as() {
    // (let [[a :as all] [1 2]] all) should compile successfully
    let chunk = compile_source("(let [[a :as all] [1 2]] all)");
    let code = chunk.code();

    // Should have Move instruction for :as binding
    let move_count = code
        .iter()
        .filter(|&&instr| decode_op(instr) == Some(Opcode::Move))
        .count();
    assert!(
        move_count >= 1_usize,
        "expected at least 1 Move instruction for :as binding"
    );
}

#[test]
fn compile_let_destructure_nested() {
    // (let [[[a b] c] [[1 2] 3]] a) should compile with nested patterns
    let chunk = compile_source("(let [[[a b] c] [[1 2] 3]] a)");
    let code = chunk.code();

    // Should have multiple GetGlobal for "first" (outer + inner destructuring)
    let get_global_count = code
        .iter()
        .filter(|&&instr| decode_op(instr) == Some(Opcode::GetGlobal))
        .count();
    assert!(
        get_global_count >= 4_usize,
        "expected at least 4 GetGlobal instructions for nested destructuring"
    );
}

#[test]
fn compile_let_destructure_mixed_bindings() {
    // Mix simple and destructuring bindings
    let chunk = compile_source("(let [x 1 [a b] [2 3] y 4] (+ x y))");
    let code = chunk.code();

    // Should have GetGlobal for first/rest (from destructuring)
    let has_get_global = code
        .iter()
        .any(|&instr| decode_op(instr) == Some(Opcode::GetGlobal));
    assert!(has_get_global, "expected GetGlobal for destructuring");

    // Should have Add for (+ x y)
    let has_add = code
        .iter()
        .any(|&instr| decode_op(instr) == Some(Opcode::Add));
    assert!(has_add, "expected Add instruction");
}
