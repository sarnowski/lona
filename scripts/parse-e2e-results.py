#!/usr/bin/env python3
"""
Run QEMU and parse E2E test results from serial output.

Usage: parse-e2e-results.py <arch> <qemu_command...>

The script:
1. Runs QEMU as a subprocess
2. Streams all output to stderr in real-time
3. Monitors serial output for test completion
4. Kills QEMU when E2E_VERDICT is seen or timeout (30s)
5. Parses and reports a compact summary

The test framework outputs structured results:
  === LONA E2E TEST RUN ===
  [TEST] test_name ... PASS
  [TEST] test_name ... FAIL
    Error: description
  [TEST] test_name ... SKIP
    Reason: description
  === RESULTS: X passed, Y failed, Z skipped ===
  === E2E_VERDICT: PASS|FAIL|TIMEOUT ===

Exit codes:
  0 = all tests passed
  1 = one or more tests failed
  2 = timeout or parse error
"""

import os
import re
import signal
import subprocess
import sys
import threading
import time
from dataclasses import dataclass
from enum import Enum
from typing import Optional

# Test timeout in seconds - tests should complete within 10s, 30s is generous
TEST_TIMEOUT = 30


class TestStatus(Enum):
    PASS = "PASS"
    FAIL = "FAIL"
    SKIP = "SKIP"


@dataclass
class TestResult:
    name: str
    status: TestStatus
    details: Optional[str] = None


def parse_test_output(output: str) -> tuple[list[TestResult], Optional[str]]:
    """
    Parse E2E test output and return (results, verdict).

    Returns:
        Tuple of (list of TestResult, verdict string or None)
    """
    results: list[TestResult] = []
    verdict: Optional[str] = None

    lines = output.split('\n')
    i = 0

    while i < len(lines):
        line = lines[i]

        # Parse individual test results
        # Format: [TEST] test_name ... PASS|FAIL|SKIP
        test_match = re.match(r'\[TEST\]\s+(\S+)\s+\.\.\.\s+(PASS|FAIL|SKIP)', line)
        if test_match:
            name = test_match.group(1)
            status = TestStatus(test_match.group(2))
            details = None

            # Look for details on following lines (indented with spaces)
            i += 1
            detail_lines = []
            while i < len(lines) and lines[i].startswith('  '):
                detail_lines.append(lines[i].strip())
                i += 1

            if detail_lines:
                details = '\n'.join(detail_lines)

            results.append(TestResult(name=name, status=status, details=details))
            continue

        # Parse verdict
        # Format: === E2E_VERDICT: PASS|FAIL|TIMEOUT ===
        verdict_match = re.search(r'E2E_VERDICT:\s*(PASS|FAIL|TIMEOUT)', line)
        if verdict_match:
            verdict = verdict_match.group(1)

        i += 1

    return results, verdict


def print_summary(arch: str, results: list[TestResult], verdict: Optional[str]) -> bool:
    """
    Print compact test summary and return success status.

    Returns:
        True if all tests passed, False otherwise
    """
    # Status icons
    icons = {
        TestStatus.PASS: '\033[32m✓\033[0m',  # Green checkmark
        TestStatus.FAIL: '\033[31m✗\033[0m',  # Red X
        TestStatus.SKIP: '\033[33m○\033[0m',  # Yellow circle
    }

    # Check if terminal supports colors
    use_colors = sys.stderr.isatty()
    if not use_colors:
        icons = {
            TestStatus.PASS: '[PASS]',
            TestStatus.FAIL: '[FAIL]',
            TestStatus.SKIP: '[SKIP]',
        }

    print(f"\n{'=' * 60}", file=sys.stderr)
    print(f"E2E Test Summary: {arch}", file=sys.stderr)
    print('=' * 60, file=sys.stderr)

    if not results:
        print("\n  No test results found in output!", file=sys.stderr)
        print("  Check the QEMU output above for boot errors.", file=sys.stderr)
        return False

    # Print individual results
    passed = 0
    failed = 0
    skipped = 0

    for result in results:
        icon = icons[result.status]
        print(f"  {icon} {result.name}", file=sys.stderr)

        if result.details:
            for detail_line in result.details.split('\n'):
                print(f"      {detail_line}", file=sys.stderr)

        if result.status == TestStatus.PASS:
            passed += 1
        elif result.status == TestStatus.FAIL:
            failed += 1
        else:
            skipped += 1

    # Print summary
    print(file=sys.stderr)
    print(f"[{arch}] Summary: {passed} passed, {failed} failed, {skipped} skipped", file=sys.stderr)

    # Print verdict
    if verdict:
        if verdict == "PASS":
            verdict_str = "\033[32mPASS\033[0m" if use_colors else "PASS"
        elif verdict == "FAIL":
            verdict_str = "\033[31mFAIL\033[0m" if use_colors else "FAIL"
        else:  # TIMEOUT
            verdict_str = "\033[31mTIMEOUT\033[0m" if use_colors else "TIMEOUT"
        print(f"[{arch}] Verdict: {verdict_str}", file=sys.stderr)
    else:
        print(f"[{arch}] Verdict: \033[31mUNKNOWN\033[0m (no verdict in output)" if use_colors
              else f"[{arch}] Verdict: UNKNOWN (no verdict in output)", file=sys.stderr)

    print('=' * 60, file=sys.stderr)

    return verdict == "PASS" and failed == 0


def kill_process_tree(proc: subprocess.Popen) -> None:
    """Kill process and all its children (including docker container)."""
    try:
        # First try graceful termination
        proc.terminate()
        try:
            proc.wait(timeout=2)
        except subprocess.TimeoutExpired:
            # Force kill
            proc.kill()
            proc.wait(timeout=2)
    except Exception:
        pass

    # Also try to kill by process group (catches docker container)
    try:
        os.killpg(os.getpgid(proc.pid), signal.SIGKILL)
    except Exception:
        pass


def run_qemu_with_monitor(command: list[str]) -> tuple[str, Optional[str]]:
    """
    Run QEMU, stream output in real-time, and monitor for test completion.

    Timeout is TEST_TIMEOUT seconds (30s by default).

    Returns:
        Tuple of (captured output, verdict or None if timeout)
    """
    output_lines: list[str] = []
    verdict_found = threading.Event()
    timed_out = False

    # Start process in new process group so we can kill the whole tree
    proc = subprocess.Popen(
        command,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,  # Line buffered
        preexec_fn=os.setsid,  # Create new process group
    )

    # Regex to match ANSI escape sequences (CSI sequences and simple escapes)
    # This includes: cursor movement, screen clear, colors, etc.
    ansi_escape_re = re.compile(r'\x1b(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])')

    def read_output():
        """Read output line by line, echo to stderr, and check for verdict."""
        assert proc.stdout is not None
        try:
            for line in proc.stdout:
                output_lines.append(line)

                # Strip ANSI escape sequences for display
                clean_line = ansi_escape_re.sub('', line)
                # Skip lines that were ONLY escape sequences (became empty after stripping)
                # but preserve legitimate empty lines
                had_escapes = line != clean_line
                if not (had_escapes and not clean_line.strip()):
                    print(clean_line, end='', file=sys.stderr, flush=True)

                # Check for verdict marker
                if 'E2E_VERDICT:' in line:
                    verdict_found.set()
                    break
        except Exception:
            pass  # Process was killed

    # Start reader thread
    reader = threading.Thread(target=read_output, daemon=True)
    reader.start()

    # Wait for verdict or timeout
    start_time = time.time()
    while not verdict_found.is_set():
        elapsed = time.time() - start_time
        if elapsed > TEST_TIMEOUT:
            timed_out = True
            print(f"\n[TIMEOUT] Test exceeded {TEST_TIMEOUT}s limit", file=sys.stderr)
            break
        time.sleep(0.1)

    # Kill QEMU and docker container
    kill_process_tree(proc)

    # Wait for reader to finish
    reader.join(timeout=1)

    output = ''.join(output_lines)

    if timed_out:
        return output, "TIMEOUT"
    elif verdict_found.is_set():
        # Parse verdict from output
        match = re.search(r'E2E_VERDICT:\s*(PASS|FAIL)', output)
        return output, match.group(1) if match else None
    else:
        return output, "TIMEOUT"


def main():
    if len(sys.argv) < 3:
        print("Usage: parse-e2e-results.py <arch> <qemu_command...>", file=sys.stderr)
        sys.exit(2)

    arch = sys.argv[1]
    qemu_command = sys.argv[2:]

    print(f"Running E2E tests for {arch} (timeout: {TEST_TIMEOUT}s)...", file=sys.stderr)
    print("-" * 60, file=sys.stderr)

    # Run QEMU and monitor output (streams to stderr in real-time)
    output, verdict = run_qemu_with_monitor(qemu_command)

    # Parse results from captured output
    results, parsed_verdict = parse_test_output(output)

    # Use parsed verdict if available, otherwise use monitor verdict
    final_verdict = parsed_verdict or verdict

    # Print compact summary
    success = print_summary(arch, results, final_verdict)

    if final_verdict == "TIMEOUT":
        sys.exit(2)
    elif success:
        sys.exit(0)
    else:
        sys.exit(1)


if __name__ == '__main__':
    main()
