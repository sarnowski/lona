// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Integration test utilities for Lona.
//!
//! This crate provides utilities for running integration tests in QEMU.
//! Tests emit structured markers to stdout that can be parsed by the
//! test harness script to determine pass/fail status.
//!
//! # Test Protocol
//!
//! Tests communicate results via serial output markers:
//! ```text
//! [LONA-TEST-START]
//! test_boot... [PASS]
//! test_arithmetic... [PASS]
//! [LONA-TEST-RESULT:PASS]
//! [LONA-TEST-END]
//! ```
//!
//! The harness script parses `[LONA-TEST-RESULT:PASS]` or `[LONA-TEST-RESULT:FAIL]`
//! to determine the final exit code.

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

/// Test outcome status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Status {
    /// Test passed successfully.
    Pass,
    /// Test failed.
    Fail,
}

impl Status {
    /// Returns the marker string for this status.
    #[inline]
    #[must_use]
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Pass => "[PASS]",
            Self::Fail => "[FAIL]",
        }
    }
}

/// A named test case with a function to execute.
#[non_exhaustive]
pub struct Test {
    /// Name of the test (displayed in output).
    pub name: &'static str,
    /// Function that runs the test and returns pass/fail status.
    pub run: fn() -> Status,
}

impl Test {
    /// Creates a new test case.
    #[inline]
    #[must_use]
    pub const fn new(name: &'static str, run: fn() -> Status) -> Self {
        Self { name, run }
    }
}

/// Runs a suite of tests and returns the overall status.
///
/// Emits structured markers for the test harness to parse.
///
/// # Arguments
///
/// * `tests` - Slice of test cases to run
/// * `output` - Callback for writing output (typically UART)
///
/// # Returns
///
/// `Status::Pass` if all tests passed, `Status::Fail` if any failed.
#[inline]
pub fn run_tests<F>(tests: &[Test], mut output: F) -> Status
where
    F: FnMut(&str),
{
    output("[LONA-TEST-START]\n");

    let mut all_passed = true;

    for test in tests {
        output(test.name);
        output("... ");

        let status = (test.run)();

        output(status.marker());
        output("\n");

        if status == Status::Fail {
            all_passed = false;
        }
    }

    let final_status = if all_passed {
        Status::Pass
    } else {
        Status::Fail
    };

    output("[LONA-TEST-RESULT:");
    output(if all_passed { "PASS" } else { "FAIL" });
    output("]\n");

    output("[LONA-TEST-END]\n");

    final_status
}

/// Macro to define a test that passes if a condition is true.
///
/// # Example
///
/// ```ignore
/// let test = lona_test::test_case!("arithmetic", {
///     let result = 1 + 2;
///     result == 3
/// });
/// ```
#[macro_export]
macro_rules! test_case {
    ($name:expr, $body:expr) => {
        $crate::Test::new($name, || {
            if $body {
                $crate::Status::Pass
            } else {
                $crate::Status::Fail
            }
        })
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "alloc")]
    use alloc::string::String;

    #[test]
    fn status_markers_are_correct() {
        assert_eq!(Status::Pass.marker(), "[PASS]");
        assert_eq!(Status::Fail.marker(), "[FAIL]");
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn run_tests_emits_markers() {
        let tests = [
            Test::new("passing_test", || Status::Pass),
            Test::new("another_passing", || Status::Pass),
        ];

        let mut output = String::new();
        let status = run_tests(&tests, |s| output.push_str(s));

        assert_eq!(status, Status::Pass);
        assert!(output.contains("[LONA-TEST-START]"));
        assert!(output.contains("passing_test... [PASS]"));
        assert!(output.contains("another_passing... [PASS]"));
        assert!(output.contains("[LONA-TEST-RESULT:PASS]"));
        assert!(output.contains("[LONA-TEST-END]"));
    }

    #[test]
    #[cfg(feature = "alloc")]
    fn run_tests_reports_failure() {
        let tests = [
            Test::new("passing", || Status::Pass),
            Test::new("failing", || Status::Fail),
            Test::new("also_passing", || Status::Pass),
        ];

        let mut output = String::new();
        let status = run_tests(&tests, |s| output.push_str(s));

        assert_eq!(status, Status::Fail);
        assert!(output.contains("[LONA-TEST-RESULT:FAIL]"));
        assert!(output.contains("passing... [PASS]"));
        assert!(output.contains("failing... [FAIL]"));
        assert!(output.contains("also_passing... [PASS]"));
    }

    #[test]
    fn test_case_macro_works() {
        let test = test_case!("simple", true);
        assert_eq!(test.name, "simple");
        assert_eq!((test.run)(), Status::Pass);

        let failing = test_case!("failing", false);
        assert_eq!((failing.run)(), Status::Fail);
    }
}
