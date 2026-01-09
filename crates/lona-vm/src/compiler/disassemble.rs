// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Bytecode disassembler for debugging.

use crate::bytecode::Chunk;
use crate::bytecode::{decode_a, decode_b, decode_bx, decode_c, decode_opcode, decode_sbx, op};

use std::fmt::Write;

/// Debug helper: disassemble a chunk to a string.
#[must_use]
pub fn disassemble(chunk: &Chunk) -> std::string::String {
    let mut out = std::string::String::new();

    for (i, &instr) in chunk.code.iter().enumerate() {
        let opcode = decode_opcode(instr);
        let a = decode_a(instr);
        let bx = decode_bx(instr);

        let _ = write!(out, "{i:04}: ");

        match opcode {
            op::LOADNIL => {
                let _ = writeln!(out, "LOADNIL   X{a}");
            }
            op::LOADBOOL => {
                let b = if bx != 0 { "true" } else { "false" };
                let _ = writeln!(out, "LOADBOOL  X{a}, {b}");
            }
            op::LOADINT => {
                let sbx = decode_sbx(instr);
                let _ = writeln!(out, "LOADINT   X{a}, {sbx}");
            }
            op::LOADK => {
                let _ = writeln!(out, "LOADK     X{a}, K{bx}");
            }
            op::MOVE => {
                let b = decode_b(instr);
                let _ = writeln!(out, "MOVE      X{a}, X{b}");
            }
            op::INTRINSIC => {
                let b = decode_b(instr);
                let name = crate::intrinsics::intrinsic_name(a).unwrap_or("?");
                let _ = writeln!(out, "INTRINSIC {name}({a}), {b} args");
            }
            op::RETURN => {
                let _ = writeln!(out, "RETURN");
            }
            op::HALT => {
                let _ = writeln!(out, "HALT");
            }
            op::BUILD_TUPLE => {
                let b = decode_b(instr);
                let c = decode_c(instr);
                let _ = writeln!(out, "BUILD_TUPLE X{a}, X{b}, {c}");
            }
            op::BUILD_MAP => {
                let b = decode_b(instr);
                let c = decode_c(instr);
                let _ = writeln!(out, "BUILD_MAP X{a}, X{b}, {c} pairs");
            }
            op::CALL => {
                let b = decode_b(instr);
                let _ = writeln!(out, "CALL      X{a}, {b} args");
            }
            _ => {
                let _ = writeln!(out, "??? opcode={opcode}");
            }
        }
    }

    if !chunk.constants.is_empty() {
        let _ = writeln!(out, "\nConstants:");
        for (i, c) in chunk.constants.iter().enumerate() {
            let _ = writeln!(out, "  K{i}: {c:?}");
        }
    }

    out
}
