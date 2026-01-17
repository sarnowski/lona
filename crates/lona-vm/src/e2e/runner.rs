// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! E2E test runner for seL4 environment.
//!
//! Executes all registered tests and outputs results to serial console.
//! Tests receive the same process, memory space, and UART that the REPL uses,
//! ensuring they exercise the exact same code paths.

use core::option::Option;
use core::result::Result::{self, Err, Ok};

use crate::platform::MemorySpace;
use crate::process::Process;
use crate::uart::Uart;

use super::{tests_basic, tests_lmm, tests_lmm_demand};

/// Status of a single test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    /// Test passed successfully.
    Pass,
    /// Test failed with an error.
    Fail,
    /// Test was skipped.
    Skip,
}

/// Result of a single test execution.
#[derive(Debug)]
pub struct TestResult {
    /// Name of the test.
    pub name: &'static str,
    /// Status after execution.
    pub status: TestStatus,
    /// Error message if failed, skip reason if skipped.
    pub message: Option<&'static str>,
}

/// A test function signature.
///
/// Tests receive the same process, memory space, and UART that the REPL uses.
type TestFn<M, U> = fn(&mut Process, &mut M, &mut U) -> Result<(), &'static str>;

/// A registered test with metadata.
struct TestCase<M: MemorySpace, U: Uart> {
    name: &'static str,
    func: TestFn<M, U>,
}

/// Get all registered test cases.
fn get_test_cases<M: MemorySpace, U: Uart>() -> [TestCase<M, U>; 19] {
    [
        // Basic VM tests
        TestCase {
            name: "test_vm_init",
            func: tests_basic::test_vm_init,
        },
        TestCase {
            name: "test_serial_output",
            func: tests_basic::test_serial_output,
        },
        TestCase {
            name: "test_memory_types",
            func: tests_basic::test_memory_types,
        },
        TestCase {
            name: "test_address_types",
            func: tests_basic::test_address_types,
        },
        TestCase {
            name: "test_read_quoted_list",
            func: tests_basic::test_read_quoted_list,
        },
        // Memory allocation IPC tests
        TestCase {
            name: "test_lmm_alloc_single_page",
            func: tests_lmm::test_lmm_alloc_single_page,
        },
        TestCase {
            name: "test_lmm_alloc_multiple_pages",
            func: tests_lmm::test_lmm_alloc_multiple_pages,
        },
        TestCase {
            name: "test_lmm_alloc_memory_usable",
            func: tests_lmm::test_lmm_alloc_memory_usable,
        },
        TestCase {
            name: "test_lmm_alloc_with_hint",
            func: tests_lmm::test_lmm_alloc_with_hint,
        },
        TestCase {
            name: "test_lmm_alloc_sequential",
            func: tests_lmm::test_lmm_alloc_sequential,
        },
        TestCase {
            name: "test_lmm_alloc_regions",
            func: tests_lmm::test_lmm_alloc_regions,
        },
        TestCase {
            name: "test_lmm_alloc_large",
            func: tests_lmm::test_lmm_alloc_large,
        },
        TestCase {
            name: "test_lmm_alloc_invalid_hint",
            func: tests_lmm::test_lmm_alloc_invalid_hint,
        },
        // Pool growth tests
        TestCase {
            name: "test_lmm_pool_growth",
            func: tests_lmm::test_lmm_pool_growth,
        },
        TestCase {
            name: "test_lmm_process_allocation_pattern",
            func: tests_lmm::test_lmm_process_allocation_pattern,
        },
        TestCase {
            name: "test_lmm_stress_allocations",
            func: tests_lmm::test_lmm_stress_allocations,
        },
        // Production allocation tests (explicit IPC + pre-mapped stacks)
        TestCase {
            name: "test_explicit_ipc_allocation",
            func: tests_lmm_demand::test_explicit_ipc_allocation,
        },
        TestCase {
            name: "test_premapped_stack",
            func: tests_lmm_demand::test_premapped_stack,
        },
        TestCase {
            name: "test_interleaved_explicit_allocation",
            func: tests_lmm_demand::test_interleaved_explicit_allocation,
        },
    ]
}

/// Run all registered E2E tests and output results.
///
/// Tests receive the same process, memory space, and UART that the REPL uses,
/// ensuring they exercise the exact same code paths as production.
///
/// Returns `true` if all tests passed, `false` otherwise.
///
/// # Output Format
///
/// Results are printed to serial console in a structured format that
/// can be parsed by the host test runner.
pub fn run_all_tests<M: MemorySpace, U: Uart>(
    proc: &mut Process,
    mem: &mut M,
    uart: &mut U,
) -> bool {
    sel4::debug_println!("=== LONA E2E TEST RUN ===");

    let test_cases = get_test_cases::<M, U>();
    let mut passed: u32 = 0;
    let mut failed: u32 = 0;
    let skipped: u32 = 0;

    let mut results: [Option<TestResult>; 32] = [const { None }; 32];

    for (i, test) in test_cases.iter().enumerate() {
        sel4::debug_print!("[TEST] {} ... ", test.name);

        let result = match (test.func)(proc, mem, uart) {
            Ok(()) => {
                sel4::debug_println!("PASS");
                passed += 1;
                TestResult {
                    name: test.name,
                    status: TestStatus::Pass,
                    message: None,
                }
            }
            Err(msg) => {
                sel4::debug_println!("FAIL");
                sel4::debug_println!("  Error: {}", msg);
                failed += 1;
                TestResult {
                    name: test.name,
                    status: TestStatus::Fail,
                    message: Some(msg),
                }
            }
        };

        if i < results.len() {
            results[i] = Some(result);
        }
    }

    // Print summary
    sel4::debug_println!(
        "=== RESULTS: {} passed, {} failed, {} skipped ===",
        passed,
        failed,
        skipped
    );

    // Print verdict
    let all_passed = failed == 0;
    if all_passed {
        sel4::debug_println!("=== E2E_VERDICT: PASS ===");
    } else {
        sel4::debug_println!("=== E2E_VERDICT: FAIL ===");
    }

    all_passed
}
