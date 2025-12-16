#!/bin/bash
# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
#
# run-integration-tests.sh - Test harness for Lona integration tests
#
# Runs QEMU with the test image and parses serial output for test results.
#
# Exit codes:
#   0 - All tests passed
#   1 - One or more tests failed
#   2 - Timeout (tests didn't complete before printing results)
#   3 - Error (invalid arguments, missing image, etc.)

set -euo pipefail

# Configuration
TIMEOUT="${TIMEOUT:-30}"
IMAGE_FILE="${1:-}"

# QEMU configuration (must match Makefile)
QEMU="${QEMU:-qemu-system-aarch64}"
QEMU_MACHINE="${QEMU_MACHINE:-virt,virtualization=on}"
QEMU_CPU="${QEMU_CPU:-cortex-a57}"
QEMU_MEMORY="${QEMU_MEMORY:-1G}"

# Check arguments
if [[ -z "$IMAGE_FILE" ]]; then
    echo "Usage: $0 <image-file>" >&2
    echo "  image-file: Path to the bootable QEMU image" >&2
    exit 3
fi

if [[ ! -f "$IMAGE_FILE" ]]; then
    echo "Error: Image file not found: $IMAGE_FILE" >&2
    exit 3
fi

# Create temporary file for output
OUTPUT_FILE=$(mktemp)
trap 'rm -f "$OUTPUT_FILE"' EXIT

echo "Running integration tests..."
echo "  Image: $IMAGE_FILE"
echo "  Timeout: ${TIMEOUT}s"
echo ""

# Run QEMU with timeout, capturing output
# Use timeout command to limit execution time
# The -nographic -serial mon:stdio options route serial output to stdout
set +e
timeout "$TIMEOUT" "$QEMU" \
    -machine "$QEMU_MACHINE" \
    -cpu "$QEMU_CPU" \
    -m "$QEMU_MEMORY" \
    -nographic \
    -serial mon:stdio \
    -kernel "$IMAGE_FILE" \
    2>&1 | tee "$OUTPUT_FILE"
QEMU_EXIT=$?
set -e

echo ""
echo "---"

# Parse output for test results first (seL4 root task can't exit, so we always timeout)
# Check for result markers regardless of how QEMU exited
if grep -q '\[LONA-TEST-RESULT:PASS\]' "$OUTPUT_FILE"; then
    echo "RESULT: All tests PASSED"
    exit 0
elif grep -q '\[LONA-TEST-RESULT:FAIL\]' "$OUTPUT_FILE"; then
    echo "RESULT: Tests FAILED"
    # Show which tests failed
    grep '\[FAIL\]' "$OUTPUT_FILE" || true
    exit 1
fi

# No result markers found - check if it was a timeout or other error
if [[ $QEMU_EXIT -eq 124 ]]; then
    echo "TIMEOUT: Tests did not complete within ${TIMEOUT}s"
    echo "No test result markers found in output."
    exit 2
else
    echo "ERROR: QEMU exited with code $QEMU_EXIT"
    echo "No test result marker found in output"
    echo "Expected [LONA-TEST-RESULT:PASS] or [LONA-TEST-RESULT:FAIL]"
    exit 3
fi
