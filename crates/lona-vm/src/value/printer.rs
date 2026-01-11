// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Printer for Lonala values.
//!
//! Converts values back to their string representation.

use super::Value;
use crate::platform::MemorySpace;
use crate::process::Process;
use crate::uart::Uart;

/// Print a value to a UART output.
pub fn print_value<M: MemorySpace, U: Uart>(value: Value, proc: &Process, mem: &M, uart: &mut U) {
    match value {
        Value::Nil => uart_write_str(uart, "nil"),
        Value::Bool(true) => uart_write_str(uart, "true"),
        Value::Bool(false) => uart_write_str(uart, "false"),
        Value::Int(n) => print_int(n, uart),
        Value::String(addr) => {
            uart.write_byte(b'"');
            if let Some(s) = proc.read_string(mem, Value::String(addr)) {
                print_string_escaped(s, uart);
            }
            uart.write_byte(b'"');
        }
        Value::Symbol(addr) => {
            if let Some(s) = proc.read_string(mem, Value::Symbol(addr)) {
                uart_write_str(uart, s);
            }
        }
        Value::Keyword(addr) => {
            uart.write_byte(b':');
            if let Some(s) = proc.read_string(mem, Value::Keyword(addr)) {
                uart_write_str(uart, s);
            }
        }
        Value::Pair(_) => print_list(value, proc, mem, uart),
        Value::Tuple(_) => print_tuple(value, proc, mem, uart),
        Value::Vector(_) => print_vector(value, proc, mem, uart),
        Value::Map(_) => print_map(value, proc, mem, uart),
        Value::CompiledFn(_) => print_compiled_fn(value, proc, mem, uart),
        Value::Closure(_) => print_closure(value, proc, mem, uart),
        Value::NativeFn(id) => print_native_fn(id, uart),
        Value::Var(_) => print_var(value, proc, mem, uart),
        Value::Namespace(_) => print_namespace(value, proc, mem, uart),
        Value::Unbound => uart_write_str(uart, "#<unbound>"),
    }
}

fn uart_write_str<U: Uart>(uart: &mut U, s: &str) {
    for byte in s.bytes() {
        uart.write_byte(byte);
    }
}

fn print_int<U: Uart>(n: i64, uart: &mut U) {
    if n == 0 {
        uart.write_byte(b'0');
        return;
    }

    if n < 0 {
        uart.write_byte(b'-');
        // Handle MIN_VALUE edge case
        if n == i64::MIN {
            uart_write_str(uart, "9223372036854775808");
            return;
        }
    }
    print_positive(n.unsigned_abs(), uart);
}

fn print_positive<U: Uart>(n: u64, uart: &mut U) {
    if n == 0 {
        return;
    }
    print_positive(n / 10, uart);
    uart.write_byte(b'0' + (n % 10) as u8);
}

fn print_string_escaped<U: Uart>(s: &str, uart: &mut U) {
    for c in s.chars() {
        match c {
            '\n' => uart_write_str(uart, "\\n"),
            '\t' => uart_write_str(uart, "\\t"),
            '\r' => uart_write_str(uart, "\\r"),
            '\\' => uart_write_str(uart, "\\\\"),
            '"' => uart_write_str(uart, "\\\""),
            c if c.is_ascii_control() => {
                // Print as \xNN
                uart_write_str(uart, "\\x");
                let b = c as u8;
                uart.write_byte(hex_digit(b >> 4));
                uart.write_byte(hex_digit(b & 0xF));
            }
            c => {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                uart_write_str(uart, s);
            }
        }
    }
}

const fn hex_digit(n: u8) -> u8 {
    match n {
        0..=9 => b'0' + n,
        _ => b'a' + (n - 10),
    }
}

fn print_list<M: MemorySpace, U: Uart>(list: Value, proc: &Process, mem: &M, uart: &mut U) {
    uart.write_byte(b'(');

    let mut current = list;
    let mut is_first = true;

    loop {
        match current {
            Value::Nil => break,
            Value::Pair(_) => {
                if !is_first {
                    uart.write_byte(b' ');
                }
                is_first = false;

                if let Some(pair) = proc.read_pair(mem, current) {
                    print_value(pair.first, proc, mem, uart);
                    current = pair.rest;
                } else {
                    break;
                }
            }
            // Improper list (rest is not nil or pair)
            _ => {
                uart_write_str(uart, " . ");
                print_value(current, proc, mem, uart);
                break;
            }
        }
    }

    uart.write_byte(b')');
}

fn print_tuple<M: MemorySpace, U: Uart>(tuple: Value, proc: &Process, mem: &M, uart: &mut U) {
    uart.write_byte(b'[');

    if let Some(len) = proc.read_tuple_len(mem, tuple) {
        for i in 0..len {
            if i > 0 {
                uart.write_byte(b' ');
            }
            if let Some(elem) = proc.read_tuple_element(mem, tuple, i) {
                print_value(elem, proc, mem, uart);
            }
        }
    }

    uart.write_byte(b']');
}

fn print_vector<M: MemorySpace, U: Uart>(vector: Value, proc: &Process, mem: &M, uart: &mut U) {
    uart.write_byte(b'{');

    if let Some(len) = proc.read_tuple_len(mem, vector) {
        for i in 0..len {
            if i > 0 {
                uart.write_byte(b' ');
            }
            if let Some(elem) = proc.read_tuple_element(mem, vector, i) {
                print_value(elem, proc, mem, uart);
            }
        }
    }

    uart.write_byte(b'}');
}

fn print_map<M: MemorySpace, U: Uart>(map: Value, proc: &Process, mem: &M, uart: &mut U) {
    uart_write_str(uart, "%{");

    if let Some(map_val) = proc.read_map(mem, map) {
        let mut is_first = true;
        let mut current = map_val.entries;

        while let Some(pair) = proc.read_pair(mem, current) {
            if !is_first {
                uart.write_byte(b' ');
            }
            is_first = false;

            // Each pair.first is a [key value] tuple
            if let Some(key) = proc.read_tuple_element(mem, pair.first, 0) {
                print_value(key, proc, mem, uart);
                uart.write_byte(b' ');
            }
            if let Some(val) = proc.read_tuple_element(mem, pair.first, 1) {
                print_value(val, proc, mem, uart);
            }

            current = pair.rest;
        }
    }

    uart.write_byte(b'}');
}

fn print_namespace<M: MemorySpace, U: Uart>(ns: Value, proc: &Process, mem: &M, uart: &mut U) {
    uart_write_str(uart, "#namespace[");

    if let Some(ns_val) = proc.read_namespace(mem, ns) {
        // Print the namespace name (symbol)
        if let Some(name_str) = proc.read_string(mem, ns_val.name) {
            uart_write_str(uart, name_str);
        }
    }

    uart.write_byte(b']');
}

fn print_var<M: MemorySpace, U: Uart>(var: Value, proc: &Process, mem: &M, uart: &mut U) {
    uart_write_str(uart, "#'");

    if let Some(content) = proc.read_var_content(mem, var) {
        // Print namespace/name
        let ns_addr = content.namespace;
        if !ns_addr.is_null() {
            let ns: super::Namespace = mem.read(ns_addr);
            if let Some(ns_name) = proc.read_string(mem, ns.name) {
                uart_write_str(uart, ns_name);
                uart.write_byte(b'/');
            }
        }

        // Print var name
        if !content.name.is_null() {
            let name = Value::Symbol(content.name);
            if let Some(name_str) = proc.read_string(mem, name) {
                uart_write_str(uart, name_str);
            }
        }
    }
}

fn print_compiled_fn<M: MemorySpace, U: Uart>(func: Value, proc: &Process, mem: &M, uart: &mut U) {
    uart_write_str(uart, "#<fn");

    if let Some(fn_val) = proc.read_compiled_fn(mem, func) {
        uart.write_byte(b'/');
        print_int(i64::from(fn_val.arity), uart);
        if fn_val.variadic {
            uart.write_byte(b'+');
        }
    }

    uart.write_byte(b'>');
}

fn print_closure<M: MemorySpace, U: Uart>(closure: Value, proc: &Process, mem: &M, uart: &mut U) {
    uart_write_str(uart, "#<closure");

    if let Some(closure_val) = proc.read_closure(mem, closure) {
        // Read the underlying function to get arity
        let fn_val: crate::value::HeapCompiledFn = mem.read(closure_val.function);
        uart.write_byte(b'/');
        print_int(i64::from(fn_val.arity), uart);
        if fn_val.variadic {
            uart.write_byte(b'+');
        }
    }

    uart.write_byte(b'>');
}

fn print_native_fn<U: Uart>(id: u16, uart: &mut U) {
    uart_write_str(uart, "#<native-fn ");
    // Try to get the intrinsic name
    if let Some(name) = crate::intrinsics::intrinsic_name(id as u8) {
        uart_write_str(uart, name);
    } else {
        print_int(i64::from(id), uart);
    }
    uart.write_byte(b'>');
}

/// Print a value to a string buffer.
///
/// Returns the printed string representation.
#[cfg(test)]
pub fn print_to_string<M: MemorySpace>(
    value: Value,
    proc: &Process,
    mem: &M,
) -> std::string::String {
    use crate::uart::MockUart;

    let mut uart = MockUart::new();
    print_value(value, proc, mem, &mut uart);
    std::string::String::from_utf8_lossy(uart.output()).into_owned()
}
