// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Tests for bytecode encoding and decoding.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_sign_loss)]

use super::*;

#[test]
fn encode_decode_abc_roundtrip() {
    // Test various combinations of A, B, C values
    let cases = [
        (0u8, 0u8, 0u16, 0u16),
        (op::MOVE, 5, 10, 15),
        (op::INTRINSIC, 255, 511, 511), // max values
        (op::LOADNIL, 128, 256, 128),
    ];

    for (opcode, a, b, c) in cases {
        let instr = encode_abc(opcode, a, b, c);
        assert_eq!(decode_opcode(instr), opcode, "opcode mismatch");
        assert_eq!(decode_a(instr), a, "A mismatch");
        assert_eq!(decode_b(instr), b, "B mismatch");
        assert_eq!(decode_c(instr), c, "C mismatch");
    }
}

#[test]
fn encode_decode_abx_roundtrip() {
    // Test various combinations of A, Bx values
    let cases = [
        (op::LOADINT, 0u8, 0u32),
        (op::LOADK, 5, 12345),
        (op::LOADBOOL, 255, 0x3FFFF), // max Bx
        (op::LOADNIL, 128, 0x20000),  // middle value
    ];

    for (opcode, a, bx) in cases {
        let instr = encode_abx(opcode, a, bx);
        assert_eq!(decode_opcode(instr), opcode, "opcode mismatch");
        assert_eq!(decode_a(instr), a, "A mismatch");
        assert_eq!(decode_bx(instr), bx, "Bx mismatch");
    }
}

#[test]
fn decode_signed_bx_positive() {
    // Positive values (bit 17 = 0)
    let instr = encode_abx(op::LOADINT, 0, 12345);
    assert_eq!(decode_sbx(instr), 12345);

    let instr = encode_abx(op::LOADINT, 0, 0);
    assert_eq!(decode_sbx(instr), 0);

    // Max positive (0x1FFFF = 131071)
    let instr = encode_abx(op::LOADINT, 0, MAX_SIGNED_BX as u32);
    assert_eq!(decode_sbx(instr), MAX_SIGNED_BX);
}

#[test]
fn decode_signed_bx_negative() {
    // -1 in 18-bit two's complement = 0x3FFFF
    let instr = encode_abx(op::LOADINT, 0, 0x3FFFF);
    assert_eq!(decode_sbx(instr), -1);

    // -2 = 0x3FFFE
    let instr = encode_abx(op::LOADINT, 0, 0x3FFFE);
    assert_eq!(decode_sbx(instr), -2);

    // Min negative (0x20000 = -131072)
    let instr = encode_abx(op::LOADINT, 0, MIN_SIGNED_BX as u32 & BX_MASK);
    assert_eq!(decode_sbx(instr), MIN_SIGNED_BX);
}

#[test]
fn chunk_add_constant() {
    let mut chunk = Chunk::new();

    let idx0 = chunk.add_constant(Value::int(42)).unwrap();
    assert_eq!(idx0, 0);

    let idx1 = chunk.add_constant(Value::nil()).unwrap();
    assert_eq!(idx1, 1);

    let idx2 = chunk.add_constant(Value::bool(true)).unwrap();
    assert_eq!(idx2, 2);

    assert_eq!(chunk.constants.len(), 3);
    assert_eq!(chunk.constants[0], Value::int(42));
    assert_eq!(chunk.constants[1], Value::nil());
    assert_eq!(chunk.constants[2], Value::bool(true));
}

#[test]
fn chunk_emit() {
    let mut chunk = Chunk::new();

    chunk.emit(encode_abx(op::LOADINT, 0, 42));
    chunk.emit(encode_abc(op::INTRINSIC, 0, 2, 0));
    chunk.emit(encode_abx(op::HALT, 0, 0));

    assert_eq!(chunk.code_len(), 3);
    assert_eq!(decode_opcode(chunk.code[0]), op::LOADINT);
    assert_eq!(decode_opcode(chunk.code[1]), op::INTRINSIC);
    assert_eq!(decode_opcode(chunk.code[2]), op::HALT);
}

#[test]
fn instruction_field_boundaries() {
    // Verify fields don't overlap
    // opcode at bits 31-26
    // A at bits 25-18
    // B at bits 17-9
    // C at bits 8-0

    let instr = encode_abc(0x3F, 0xFF, 0x1FF, 0x1FF);
    assert_eq!(instr, 0xFFFF_FFFF); // All bits set

    // Only opcode set
    let instr = encode_abc(0x3F, 0, 0, 0);
    assert_eq!(instr, 0xFC00_0000);

    // Only A set
    let instr = encode_abc(0, 0xFF, 0, 0);
    assert_eq!(instr, 0x03FC_0000);

    // Only B set
    let instr = encode_abc(0, 0, 0x1FF, 0);
    assert_eq!(instr, 0x0003_FE00);

    // Only C set
    let instr = encode_abc(0, 0, 0, 0x1FF);
    assert_eq!(instr, 0x0000_01FF);
}

#[test]
fn bx_field_full_range() {
    // Bx uses bits 17-0
    let instr = encode_abx(0, 0, 0x3FFFF);
    assert_eq!(decode_bx(instr), 0x3FFFF);

    let instr = encode_abx(0, 0, 0);
    assert_eq!(decode_bx(instr), 0);
}
