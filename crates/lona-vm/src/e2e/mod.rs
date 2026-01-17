// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! End-to-end test framework for Lona on seL4.
//!
//! This module provides a test runner and test infrastructure for running
//! integration tests within the seL4 environment. Tests run during boot
//! and output results to the serial console for parsing by the host.
//!
//! ## Test Output Protocol
//!
//! Tests output structured results that can be parsed by the host:
//!
//! ```text
//! === LONA E2E TEST RUN ===
//! [TEST] test_name ... PASS
//! [TEST] test_name ... FAIL
//!   Error: description
//! [TEST] test_name ... SKIP
//!   Reason: description
//! === RESULTS: X passed, Y failed, Z skipped ===
//! === E2E_VERDICT: PASS|FAIL ===
//! ```
//!
//! ## Usage
//!
//! The e2e-test feature must be enabled:
//!
//! ```bash
//! cargo build --features sel4,e2e-test
//! ```

mod runner;
mod tests_basic;
mod tests_lmm;
mod tests_lmm_demand;

pub use runner::{TestResult, TestStatus, run_all_tests};
