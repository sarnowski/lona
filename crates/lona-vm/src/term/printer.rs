// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Term printer for displaying values to UART.

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::realm::Realm;
use crate::term::Term;
use crate::term::header::Header;
use crate::term::tag::object;
use crate::uart::{Uart, UartExt};

/// Print a Term to the UART.
pub fn print_term<M: MemorySpace, U: Uart>(
    term: Term,
    proc: &Process,
    realm: &Realm,
    mem: &M,
    uart: &mut U,
) {
    print_term_depth(term, proc, realm, mem, uart, 0);
}

/// Maximum depth for recursive printing (prevents stack overflow).
const MAX_PRINT_DEPTH: usize = 32;

/// Maximum list elements to print (prevents infinite output).
const MAX_LIST_PRINT: usize = 100;

fn print_term_depth<M: MemorySpace, U: Uart>(
    term: Term,
    proc: &Process,
    realm: &Realm,
    mem: &M,
    uart: &mut U,
    depth: usize,
) {
    if depth > MAX_PRINT_DEPTH {
        uart.write_str("...");
        return;
    }

    // Handle special values
    if term.is_nil() {
        uart.write_str("nil");
        return;
    }
    if term.is_true() {
        uart.write_str("true");
        return;
    }
    if term.is_false() {
        uart.write_str("false");
        return;
    }
    if term.is_unbound() {
        uart.write_str("#<unbound>");
        return;
    }

    // Handle small integers
    if term.is_small_int() {
        if let Some(n) = term.as_small_int() {
            print_i64(n, uart);
        }
        return;
    }

    // Handle native functions
    if term.is_native_fn() {
        if let Some(id) = term.as_native_fn_id() {
            uart.write_str("#<native-fn:");
            print_u16(id, uart);
            uart.write_byte(b'>');
        }
        return;
    }

    // Handle immediate symbols (index into realm's symbol table)
    if let Some(idx) = term.as_symbol_index() {
        if let Some(name) = realm.symbol_name(mem, idx) {
            uart.write_str(name);
        } else {
            uart.write_str("#<symbol:");
            print_u32(idx, uart);
            uart.write_byte(b'>');
        }
        return;
    }

    // Handle immediate keywords (index into realm's keyword table)
    if let Some(idx) = term.as_keyword_index() {
        uart.write_byte(b':');
        if let Some(name) = realm.keyword_name(mem, idx) {
            uart.write_str(name);
        } else {
            uart.write_str("<keyword:");
            print_u32(idx, uart);
            uart.write_byte(b'>');
        }
        return;
    }

    // Handle list (pair)
    if term.is_list() {
        print_list(term, proc, realm, mem, uart, depth);
        return;
    }

    // Handle boxed values
    if term.is_boxed() {
        print_boxed(term, proc, realm, mem, uart, depth);
        return;
    }

    // Unknown term
    uart.write_str("#<unknown>");
}

fn print_list<M: MemorySpace, U: Uart>(
    term: Term,
    proc: &Process,
    realm: &Realm,
    mem: &M,
    uart: &mut U,
    depth: usize,
) {
    uart.write_byte(b'(');
    let mut current = term;
    let mut first = true;
    let mut count = 0;

    while let Some((head, tail)) = proc.read_term_pair(mem, current) {
        if !first {
            uart.write_byte(b' ');
        }
        first = false;

        print_term_depth(head, proc, realm, mem, uart, depth + 1);

        if tail.is_nil() {
            break;
        }

        if !tail.is_list() {
            // Improper list
            uart.write_str(" . ");
            print_term_depth(tail, proc, realm, mem, uart, depth + 1);
            break;
        }

        current = tail;
        count += 1;
        if count >= MAX_LIST_PRINT {
            uart.write_str(" ...");
            break;
        }
    }

    uart.write_byte(b')');
}

fn print_boxed<M: MemorySpace, U: Uart>(
    term: Term,
    proc: &Process,
    realm: &Realm,
    mem: &M,
    uart: &mut U,
    depth: usize,
) {
    let addr = term.to_vaddr();
    let header: Header = mem.read(addr);

    match header.object_tag() {
        object::STRING => print_string(proc, mem, uart, term),
        object::SYMBOL => print_symbol(proc, mem, uart, term),
        object::KEYWORD => print_keyword(proc, mem, uart, term),
        object::TUPLE => print_tuple(proc, realm, mem, uart, term, depth),
        object::VECTOR => print_vector(proc, realm, mem, uart, term, depth),
        object::MAP => print_map(proc, realm, mem, uart, term, depth),
        object::FUN => uart.write_str("#<fn>"),
        object::CLOSURE => uart.write_str("#<closure>"),
        object::VAR => uart.write_str("#<var>"),
        object::NAMESPACE => print_namespace(proc, mem, uart, addr),
        _ => {
            uart.write_str("#<boxed:");
            print_u8(header.object_tag(), uart);
            uart.write_byte(b'>');
        }
    }
}

fn print_string<M: MemorySpace, U: Uart>(proc: &Process, mem: &M, uart: &mut U, term: Term) {
    uart.write_byte(b'"');
    if let Some(s) = proc.read_term_string(mem, term) {
        for &b in s.as_bytes() {
            match b {
                b'\n' => uart.write_str("\\n"),
                b'\r' => uart.write_str("\\r"),
                b'\t' => uart.write_str("\\t"),
                b'"' => uart.write_str("\\\""),
                b'\\' => uart.write_str("\\\\"),
                _ if b.is_ascii_graphic() || b == b' ' => uart.write_byte(b),
                _ => {
                    uart.write_str("\\x");
                    print_hex_byte(b, uart);
                }
            }
        }
    }
    uart.write_byte(b'"');
}

fn print_symbol<M: MemorySpace, U: Uart>(proc: &Process, mem: &M, uart: &mut U, term: Term) {
    if let Some(s) = proc.read_term_string(mem, term) {
        uart.write_str(s);
    } else {
        uart.write_str("#<symbol>");
    }
}

fn print_keyword<M: MemorySpace, U: Uart>(proc: &Process, mem: &M, uart: &mut U, term: Term) {
    uart.write_byte(b':');
    if let Some(s) = proc.read_term_string(mem, term) {
        uart.write_str(s);
    }
}

fn print_tuple<M: MemorySpace, U: Uart>(
    proc: &Process,
    realm: &Realm,
    mem: &M,
    uart: &mut U,
    term: Term,
    depth: usize,
) {
    uart.write_byte(b'[');
    if let Some(len) = proc.read_term_tuple_len(mem, term) {
        for i in 0..len {
            if i > 0 {
                uart.write_byte(b' ');
            }
            if let Some(elem) = proc.read_term_tuple_element(mem, term, i) {
                print_term_depth(elem, proc, realm, mem, uart, depth + 1);
            }
        }
    }
    uart.write_byte(b']');
}

fn print_vector<M: MemorySpace, U: Uart>(
    proc: &Process,
    realm: &Realm,
    mem: &M,
    uart: &mut U,
    term: Term,
    depth: usize,
) {
    uart.write_byte(b'{');
    if let Some(len) = proc.read_term_vector_len(mem, term) {
        for i in 0..len {
            if i > 0 {
                uart.write_byte(b' ');
            }
            if let Some(elem) = proc.read_term_vector_element(mem, term, i) {
                print_term_depth(elem, proc, realm, mem, uart, depth + 1);
            }
        }
    }
    uart.write_byte(b'}');
}

fn print_map<M: MemorySpace, U: Uart>(
    proc: &Process,
    realm: &Realm,
    mem: &M,
    uart: &mut U,
    term: Term,
    depth: usize,
) {
    uart.write_str("%{");
    if let Some(entries) = proc.read_term_map_entries(mem, term) {
        let mut current = entries;
        let mut first = true;
        while let Some((entry, rest)) = proc.read_term_pair(mem, current) {
            if !first {
                uart.write_byte(b' ');
            }
            first = false;

            // Each entry is a [key value] tuple
            if let Some(key) = proc.read_term_tuple_element(mem, entry, 0) {
                print_term_depth(key, proc, realm, mem, uart, depth + 1);
            }
            uart.write_byte(b' ');
            if let Some(val) = proc.read_term_tuple_element(mem, entry, 1) {
                print_term_depth(val, proc, realm, mem, uart, depth + 1);
            }
            current = rest;
        }
    }
    uart.write_byte(b'}');
}

fn print_namespace<M: MemorySpace, U: Uart>(
    proc: &Process,
    mem: &M,
    uart: &mut U,
    addr: crate::Vaddr,
) {
    uart.write_str("#<ns:");
    // Try to get namespace name
    let name_offset = 8u64; // After header
    let name_term: Term = mem.read(addr.add(name_offset));
    if let Some(s) = proc.read_term_string(mem, name_term) {
        uart.write_str(s);
    }
    uart.write_byte(b'>');
}

/// Print a u8 as decimal.
fn print_u8<U: Uart>(n: u8, uart: &mut U) {
    let mut buf = [0u8; 3];
    let mut i = 0;
    let mut val = n;

    if val == 0 {
        uart.write_byte(b'0');
        return;
    }

    while val > 0 {
        buf[i] = b'0' + (val % 10);
        val /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        uart.write_byte(buf[i]);
    }
}

/// Print a u16 as decimal.
fn print_u16<U: Uart>(n: u16, uart: &mut U) {
    let mut buf = [0u8; 5];
    let mut i = 0;
    let mut val = n;

    if val == 0 {
        uart.write_byte(b'0');
        return;
    }

    while val > 0 {
        buf[i] = b'0' + (val % 10) as u8;
        val /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        uart.write_byte(buf[i]);
    }
}

/// Print a u32 as decimal.
fn print_u32<U: Uart>(n: u32, uart: &mut U) {
    let mut buf = [0u8; 10];
    let mut i = 0;
    let mut val = n;

    if val == 0 {
        uart.write_byte(b'0');
        return;
    }

    while val > 0 {
        buf[i] = b'0' + (val % 10) as u8;
        val /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        uart.write_byte(buf[i]);
    }
}

/// Print an i64 as decimal.
#[expect(
    clippy::cast_sign_loss,
    reason = "value is non-negative in else branch (n >= 0)"
)]
fn print_i64<U: Uart>(n: i64, uart: &mut U) {
    if n < 0 {
        uart.write_byte(b'-');
        if n == i64::MIN {
            uart.write_str("9223372036854775808");
            return;
        }
        print_u64(n.unsigned_abs(), uart);
    } else {
        print_u64(n as u64, uart);
    }
}

/// Print a u64 as decimal.
fn print_u64<U: Uart>(n: u64, uart: &mut U) {
    let mut buf = [0u8; 20];
    let mut i = 0;
    let mut val = n;

    if val == 0 {
        uart.write_byte(b'0');
        return;
    }

    while val > 0 {
        buf[i] = b'0' + (val % 10) as u8;
        val /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        uart.write_byte(buf[i]);
    }
}

/// Print a byte as two hex digits.
fn print_hex_byte<U: Uart>(b: u8, uart: &mut U) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    uart.write_byte(HEX[(b >> 4) as usize]);
    uart.write_byte(HEX[(b & 0x0f) as usize]);
}
