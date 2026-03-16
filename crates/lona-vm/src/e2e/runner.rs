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
use crate::process::{Process, ProcessId, ProcessStatus};
use crate::realm::Realm;
use crate::scheduler::Scheduler;
use crate::uart::Uart;

use super::{spec_runner, tests_basic, tests_lmm, tests_lmm_demand, tests_process};

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
/// Tests receive the same process, realm, memory space, and UART that the REPL uses.
type TestFn<M, U> = fn(&mut Process, &mut Realm, &mut M, &mut U) -> Result<(), &'static str>;

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
/// Tests receive the same process, realm, memory space, and UART that the REPL uses,
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
    realm: &mut Realm,
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

        let result = match (test.func)(proc, realm, mem, uart) {
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

    // Create scheduler for process-aware spec tests
    let ns_var = crate::realm::get_ns_var(realm, mem);
    let core_ns = crate::realm::get_core_ns(realm, mem);
    let scheduler = match (ns_var, core_ns) {
        (Some(nv), Some(cn)) => {
            let sched = Scheduler::new(nv, cn);
            // Register the test process so (self), (alive? (self)) work
            register_test_process(&sched, proc, realm, mem);
            Some(sched)
        }
        _ => None,
    };

    // Run process-related tests (need scheduler)
    let process_failed = scheduler.as_ref().map_or(0, |sched| {
        tests_process::run_process_tests(proc, realm, mem, uart, sched)
    });

    // Run specification tests
    let spec_failed = spec_runner::run_spec_tests(proc, realm, mem, uart, scheduler.as_ref());

    // Print verdict (includes all test results)
    let all_passed = failed == 0 && spec_failed == 0 && process_failed == 0;
    if all_passed {
        sel4::debug_println!("=== E2E_VERDICT: PASS ===");
    } else {
        sel4::debug_println!("=== E2E_VERDICT: FAIL ===");
    }

    all_passed
}

/// Register the test process in the scheduler's process table.
///
/// Allocates a slot so the test process has a valid PID and
/// `alive?` returns true. The process is NOT inserted (it lives
/// outside the table as a local variable in the runner), so the
/// slot appears "taken" — this is the same state as a process being
/// executed by a worker.
fn register_test_process<M: MemorySpace>(
    scheduler: &Scheduler,
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut M,
) {
    // Grow the process table by one segment so we can allocate a slot.
    #[cfg(test)]
    {
        let _ = &mut *realm;
        let segment = crate::scheduler::ProcessTable::alloc_test_segment();
        scheduler.with_process_table_mut(|pt| unsafe { pt.grow_segment(segment) });
    }
    #[cfg(not(test))]
    {
        use crate::scheduler::process_table::{SEGMENT_SIZE, Slot};
        let size = core::mem::size_of::<Slot>() * SEGMENT_SIZE;
        if let Some(vaddr) = realm.pool_mut().allocate_with_growth(size, 8) {
            let segment_ptr = vaddr.as_u64() as *mut Slot;
            scheduler.with_process_table_mut(|pt| unsafe { pt.grow_segment(segment_ptr) });
        }
    }

    if let Some((index, generation)) = scheduler.with_process_table_mut(|pt| pt.allocate()) {
        let pid = ProcessId::new(index, generation);
        proc.pid = pid;
        proc.status = ProcessStatus::Ready;
        if let Some(pid_term) = proc.alloc_term_pid(mem, index, generation) {
            proc.pid_term = Some(pid_term);
        }
        // Slot remains "allocated but no process" (= taken state).
        // is_alive(pid) will return true via is_taken check.
    }
}
