// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Lona Runtime - The root task for Lona on seL4.
//!
//! This crate implements the initial root task that runs on the seL4 microkernel.
//! It receives all system capabilities from the kernel and is responsible for
//! bootstrapping Lona.
//!
//! # Architecture
//!
//! The Lona runtime is the first userspace process started by seL4. It:
//!
//! 1. Receives the boot info structure containing all initial capabilities
//! 2. Initializes the memory allocator using untyped memory capabilities
//! 3. Sets up the Lonala language compiler/interpreter
//! 4. Spawns the init process and other system services
//! 5. Enters the main scheduler loop
//!
//! # Safety
//!
//! As the root task, this code runs with maximum privilege in userspace.
//! All capability operations must be carefully validated to maintain
//! the security guarantees of the seL4 microkernel.

#![no_std]
#![no_main]

extern crate alloc;

mod memory;
mod platform;
mod repl;

use alloc::vec;

use lona_core::allocator::Allocator;
#[cfg(feature = "integration-test")]
use lona_core::integer::Integer;
use sel4_root_task::{Never, root_task};

#[cfg(feature = "integration-test")]
use lona_core::symbol::Interner;
#[cfg(feature = "integration-test")]
use lona_core::value::Value;
#[cfg(feature = "integration-test")]
use lona_kernel::vm::{Globals, Vm};
#[cfg(feature = "integration-test")]
use lona_test::{Status, Test, run_tests};
#[cfg(feature = "integration-test")]
use lonala_compiler::compile;

use crate::memory::Sel4PageProvider;

/// Global page provider for seL4 memory allocation.
static PAGE_PROVIDER: Sel4PageProvider = Sel4PageProvider::new();

/// Global allocator for Rust's `alloc` crate.
///
/// Initialized in `main` before any heap allocation occurs.
///
/// TODO: This global allocator is a temporary bootstrap solution for Phase 1.
/// In Phase 7 (Process Data Structure) and Phase 9 (Garbage Collection),
/// this will be replaced with per-process heaps to enable independent GC
/// and proper memory isolation between domains.
#[global_allocator]
static ALLOCATOR: Allocator<&Sel4PageProvider> = Allocator::new(&PAGE_PROVIDER);

/// Entry point for the Lona runtime.
///
/// This function is called by the seL4 kernel after boot. It receives the
/// boot info structure containing all initial capabilities and memory
/// information needed to bootstrap the system.
///
/// # Arguments
///
/// * `bootinfo` - Pointer to the seL4 boot info structure containing:
///   - Initial thread's TCB, `CNode`, `VSpace`, and ASID pool capabilities
///   - Untyped memory capabilities for dynamic allocation
///   - Device memory regions
///   - Kernel reserved memory regions
#[root_task]
fn main(bootinfo: &sel4::BootInfoPtr) -> sel4::Result<Never> {
    sel4::debug_println!("Lona runtime starting...");

    // Print basic boot information
    sel4::debug_println!("Boot info at: {:p}", bootinfo.ptr());
    sel4::debug_println!("Untyped memory regions: {}", bootinfo.untyped_list().len());

    // Initialize the memory allocator
    // SAFETY: Called once at startup, bootinfo remains valid, single-threaded
    unsafe {
        PAGE_PROVIDER.init(bootinfo);
    }
    sel4::debug_println!("Memory allocator initialized");

    // Initialize UART for real serial output
    init_uart(bootinfo);

    // Test heap allocation to verify the allocator works
    test_allocation();

    println!("Lona runtime initialized successfully");
    println!("Hello from allocator + UART");

    // Run integration tests if enabled, otherwise start the REPL
    #[cfg(feature = "integration-test")]
    {
        run_integration_tests();
        halt_loop()
    }

    // Start the interactive REPL (never returns)
    #[cfg(not(feature = "integration-test"))]
    {
        let mut interactive = repl::InteractiveRepl::new(repl::UartConsole);
        interactive.run()
    }
}

/// Halts the system in a low-power loop.
///
/// Used after integration tests complete. The loop never exits.
#[cfg(feature = "integration-test")]
fn halt_loop() -> ! {
    loop {
        // SAFETY: WFI (Wait For Interrupt) is safe to execute - it simply
        // puts the CPU into a low-power state until an interrupt occurs.
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack, preserves_flags));
        }
    }
}

/// Initializes the UART driver for serial output.
///
/// Discovers UART address from FDT and maps device memory.
fn init_uart(bootinfo: &sel4::BootInfoPtr) {
    // Discover UART from FDT in bootinfo
    let uart_info = match platform::fdt::discover_uart(bootinfo) {
        Ok(info) => {
            sel4::debug_println!(
                "Found UART at paddr 0x{:x}, size 0x{:x}",
                info.paddr,
                info.size
            );
            info
        }
        Err(err) => {
            sel4::debug_println!("Warning: UART discovery failed: {:?}", err);
            return;
        }
    };

    // Initialize the UART driver
    // SAFETY: PAGE_PROVIDER is initialized, single-threaded context
    let success = unsafe { platform::uart::init(uart_info, &PAGE_PROVIDER) };

    if success {
        // First message via UART!
        println!("UART initialized successfully");
    } else {
        sel4::debug_println!("Warning: UART initialization failed");
    }
}

/// Tests that heap allocation is working correctly.
fn test_allocation() {
    sel4::debug_println!("Testing heap allocation...");

    // Create a vector to test allocation
    let test_vec = vec![1_i32, 2_i32, 3_i32, 4_i32, 5_i32];

    // Verify the contents
    sel4::debug_println!("Allocated vector: {:?}", test_vec.as_slice());

    // Check allocator stats
    let stats = ALLOCATOR.stats();
    sel4::debug_println!(
        "Allocator stats: {} bytes in {} pages ({} bytes reserved)",
        stats.total_allocated,
        stats.pages_allocated,
        stats.total_reserved()
    );

    // Verify page provider stats match
    sel4::debug_println!("Page provider frames: {}", PAGE_PROVIDER.frames_allocated());

    // Allocate some more to verify ongoing allocation works
    let another_vec: alloc::vec::Vec<u32> = (0..100).collect();
    sel4::debug_println!("Second allocation: {} elements", another_vec.len());

    let stats = ALLOCATOR.stats();
    sel4::debug_println!(
        "Final allocator stats: {} bytes in {} pages",
        stats.total_allocated,
        stats.pages_allocated
    );
}

/// Runs integration tests and outputs results via UART.
///
/// Tests are executed when the `integration-test` feature is enabled.
/// Results are output in a structured format for the test harness to parse.
#[cfg(feature = "integration-test")]
fn run_integration_tests() {
    println!("Running integration tests...");

    let tests = [
        Test::new("boot", test_boot),
        Test::new("arithmetic", test_arithmetic),
        Test::new("subtraction", test_subtraction),
        Test::new("multiplication", test_multiplication),
        Test::new("comparison", test_comparison),
        Test::new("boolean_not", test_boolean_not),
        Test::new("nested_expr", test_nested_expression),
        Test::new("string_literal", test_string_literal),
        // Special form tests
        Test::new("do_empty", test_do_empty),
        Test::new("do_single", test_do_single),
        Test::new("do_multiple", test_do_multiple),
        Test::new("if_true", test_if_true),
        Test::new("if_false", test_if_false),
        Test::new("if_no_else", test_if_no_else),
        Test::new("def_simple", test_def_simple),
        // REPL-like persistent state tests
        Test::new("repl_def_persist", test_repl_def_persist),
        Test::new("repl_def_use_in_if", test_repl_def_use_in_if),
        // Test using actual Repl struct
        Test::new("actual_repl_test", test_actual_repl_def_use_in_if),
        // Error handling test
        Test::new("incomplete_input", test_incomplete_input_error),
    ];

    let status = run_tests(&tests, |s| print!("{s}"));

    // Report final status
    println!(
        "Integration tests {}",
        if status == Status::Pass {
            "PASSED"
        } else {
            "FAILED"
        }
    );
}

/// Tests that the system booted successfully.
///
/// If we reach this code, boot has succeeded (implicit pass).
#[cfg(feature = "integration-test")]
fn test_boot() -> Status {
    // If we're executing this code, boot succeeded
    Status::Pass
}

/// Tests basic arithmetic: (+ 1 2) should evaluate to 3.
#[cfg(feature = "integration-test")]
fn test_arithmetic() -> Status {
    let mut interner = Interner::new();

    // Compile a simple arithmetic expression
    let chunk = match compile("(+ 1 2)", &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    // Execute it
    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(3) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests subtraction: (- 10 3) should evaluate to 7.
#[cfg(feature = "integration-test")]
fn test_subtraction() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(- 10 3)", &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(7) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests multiplication: (* 6 7) should evaluate to 42.
#[cfg(feature = "integration-test")]
fn test_multiplication() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(* 6 7)", &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(42) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests comparison: (< 1 2) should evaluate to true.
#[cfg(feature = "integration-test")]
fn test_comparison() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(< 1 2)", &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Bool(result)) if result => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests boolean not: (not false) should evaluate to true.
#[cfg(feature = "integration-test")]
fn test_boolean_not() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(not false)", &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Bool(result)) if result => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests nested expression: (+ (* 2 3) (- 10 5)) should evaluate to 11.
#[cfg(feature = "integration-test")]
fn test_nested_expression() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(+ (* 2 3) (- 10 5))", &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(11) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests string literal: "hello" should evaluate to a string value.
#[cfg(feature = "integration-test")]
fn test_string_literal() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("\"hello\"", &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::String(ref string)) if string.as_str() == "hello" => Status::Pass,
        _ => Status::Fail,
    }
}

// =============================================================================
// Special Form Tests
// =============================================================================

/// Tests empty do: (do) should return nil.
#[cfg(feature = "integration-test")]
fn test_do_empty() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(do)", &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Nil) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests single do: (do 42) should return 42.
#[cfg(feature = "integration-test")]
fn test_do_single() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(do 42)", &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(42) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests multiple do: (do 1 2 3) should return 3.
#[cfg(feature = "integration-test")]
fn test_do_multiple() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(do 1 2 3)", &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(3) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests if true branch: (if true 1 2) should return 1.
#[cfg(feature = "integration-test")]
fn test_if_true() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(if true 1 2)", &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(1) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests if false branch: (if false 1 2) should return 2.
#[cfg(feature = "integration-test")]
fn test_if_false() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(if false 1 2)", &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(2) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests if without else: (if false 1) should return nil.
#[cfg(feature = "integration-test")]
fn test_if_no_else() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(if false 1)", &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Nil) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests simple def: (def x 42) should define x and return symbol.
#[cfg(feature = "integration-test")]
fn test_def_simple() -> Status {
    let mut interner = Interner::new();

    let chunk = match compile("(do (def x 42) x)", &mut interner) {
        Ok(chunk) => chunk,
        Err(_err) => return Status::Fail,
    };

    let mut vm = Vm::new(&interner);
    match vm.execute(&chunk) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(42) => Status::Pass,
        _ => Status::Fail,
    }
}

// =============================================================================
// REPL-like Persistent State Tests
// =============================================================================

/// Helper to evaluate source with persistent state (like the REPL does).
#[cfg(feature = "integration-test")]
fn eval_with_state(
    source: &str,
    interner: &mut Interner,
    globals: &mut Globals,
) -> Result<Value, ()> {
    let chunk = compile(source, interner).map_err(|err| {
        println!("Compile error: {err}");
    })?;

    let mut vm = Vm::new(interner);
    *vm.globals_mut() = globals.clone();

    // Register print function like the REPL does
    if let Some(print_sym) = interner.get("print") {
        vm.update_print_symbol(print_sym);
        vm.set_global(print_sym, Value::Symbol(print_sym));
    }

    let result = vm.execute(&chunk).map_err(|err| {
        println!("Runtime error: {err:?}");
    })?;

    *globals = vm.globals().clone();

    Ok(result)
}

/// Tests that def persists across evaluations.
#[cfg(feature = "integration-test")]
fn test_repl_def_persist() -> Status {
    let mut interner = Interner::new();
    let mut globals = Globals::new();

    // First evaluation: define x
    if eval_with_state("(def x 42)", &mut interner, &mut globals).is_err() {
        return Status::Fail;
    }

    // Second evaluation: use x
    match eval_with_state("x", &mut interner, &mut globals) {
        Ok(Value::Integer(result)) if result == Integer::from_i64(42) => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests the exact failing scenario: def x, then use x in if condition.
#[cfg(feature = "integration-test")]
fn test_repl_def_use_in_if() -> Status {
    let mut interner = Interner::new();
    let mut globals = Globals::new();

    // First evaluation: define x
    if eval_with_state("(def x 42)", &mut interner, &mut globals).is_err() {
        return Status::Fail;
    }

    // Second evaluation: use x in if condition
    match eval_with_state(
        "(if (> x 10) \"big\" \"small\")",
        &mut interner,
        &mut globals,
    ) {
        Ok(Value::String(ref s)) if s.as_str() == "big" => Status::Pass,
        _ => Status::Fail,
    }
}

/// Tests using the core Repl struct - this is the real end-to-end test.
///
/// Uses `Repl::eval()` which is the same core function used by the interactive console.
#[cfg(feature = "integration-test")]
fn test_actual_repl_def_use_in_if() -> Status {
    let mut repl_instance = repl::Repl::new();

    // First evaluation: define x
    if repl_instance.eval("(def x 42)").is_err() {
        return Status::Fail;
    }

    // Second evaluation: use x in if condition
    match repl_instance.eval("(if (> x 10) \"big\" \"small\")") {
        Ok(Value::String(ref s)) if s.as_str() == "big" => Status::Pass,
        other => {
            // Print debug info
            println!("test_actual_repl_def_use_in_if failed: {other:?}");
            Status::Fail
        }
    }
}

/// Tests that incomplete input returns an appropriate error.
///
/// Incomplete input (unbalanced parentheses) should result in a parse error,
/// not a crash or hang.
#[cfg(feature = "integration-test")]
fn test_incomplete_input_error() -> Status {
    let mut repl_instance = repl::Repl::new();

    // This input has unbalanced parentheses - it's incomplete
    match repl_instance.eval("(def x") {
        Err(ref msg) if msg.contains("Compile error") => Status::Pass,
        Ok(value) => {
            println!("test_incomplete_input_error: expected error, got value: {value:?}");
            Status::Fail
        }
        Err(ref msg) => {
            println!("test_incomplete_input_error: unexpected error format: {msg}");
            Status::Fail
        }
    }
}
