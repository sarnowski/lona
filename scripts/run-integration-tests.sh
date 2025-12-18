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
ARCH="${1:-}"
IMAGE_PATH="${2:-}"

# Check arguments
if [[ -z "$ARCH" ]] || [[ -z "$IMAGE_PATH" ]]; then
    echo "Usage: $0 <arch> <image-path>" >&2
    echo "  arch:       Target architecture (aarch64 or x86_64)" >&2
    echo "  image-path: Path to the bootable image (file for aarch64, directory for x86_64)" >&2
    exit 3
fi

# Architecture-specific QEMU configuration
case "$ARCH" in
    aarch64)
        QEMU="${QEMU:-qemu-system-aarch64}"
        QEMU_MACHINE="${QEMU_MACHINE:-virt,virtualization=on}"
        QEMU_CPU="${QEMU_CPU:-cortex-a57}"
        QEMU_MEMORY="${QEMU_MEMORY:-1G}"

        if [[ ! -f "$IMAGE_PATH" ]]; then
            echo "Error: Image file not found: $IMAGE_PATH" >&2
            exit 3
        fi

        QEMU_BOOT_ARGS=(-kernel "$IMAGE_PATH")
        ;;

    x86_64)
        QEMU="${QEMU:-qemu-system-x86_64}"
        QEMU_MACHINE="${QEMU_MACHINE:-q35}"
        QEMU_CPU="${QEMU_CPU:-Cascadelake-Server}"
        QEMU_MEMORY="${QEMU_MEMORY:-512M}"
        OVMF_CODE="${OVMF_CODE:-/usr/share/OVMF/OVMF_CODE.fd}"

        if [[ ! -d "$IMAGE_PATH" ]]; then
            echo "Error: Image directory not found: $IMAGE_PATH" >&2
            exit 3
        fi

        if [[ ! -f "$OVMF_CODE" ]]; then
            echo "Error: OVMF firmware not found: $OVMF_CODE" >&2
            exit 3
        fi

        QEMU_BOOT_ARGS=(-bios "$OVMF_CODE" -drive "format=raw,file=fat:rw:$IMAGE_PATH")
        ;;

    *)
        echo "Error: Unknown architecture: $ARCH" >&2
        echo "Supported architectures: aarch64, x86_64" >&2
        exit 3
        ;;
esac

# Create temporary file for output
OUTPUT_FILE=$(mktemp)
trap 'rm -f "$OUTPUT_FILE"' EXIT

echo "Running $ARCH integration tests..."
echo "  Image: $IMAGE_PATH"
echo "  Timeout: ${TIMEOUT}s"
echo ""

# Run QEMU in background, capturing output to file
"$QEMU" \
    -machine "$QEMU_MACHINE" \
    -cpu "$QEMU_CPU" \
    -m "$QEMU_MEMORY" \
    -nographic \
    -serial "file:$OUTPUT_FILE" \
    -monitor none \
    "${QEMU_BOOT_ARGS[@]}" \
    2>&1 &
QEMU_PID=$!

# Poll for test completion or timeout
ELAPSED=0

while [[ $ELAPSED -lt $TIMEOUT ]]; do
    sleep 1
    ELAPSED=$((ELAPSED + 1))

    # Check if QEMU is still running
    if ! kill -0 "$QEMU_PID" 2>/dev/null; then
        # QEMU exited unexpectedly
        break
    fi

    # Check for test result markers
    if grep -q '\[LONA-TEST-RESULT:PASS\]\|\[LONA-TEST-RESULT:FAIL\]' "$OUTPUT_FILE" 2>/dev/null; then
        break
    fi
done

# Kill QEMU if still running
if kill -0 "$QEMU_PID" 2>/dev/null; then
    kill "$QEMU_PID" 2>/dev/null || true
    wait "$QEMU_PID" 2>/dev/null || true
fi

# Display the captured output
cat "$OUTPUT_FILE"

echo ""
echo "---"
echo "Completed in ${ELAPSED}s"

# Parse output for test results
if grep -q '\[LONA-TEST-RESULT:PASS\]' "$OUTPUT_FILE"; then
    echo "RESULT: All tests PASSED"
    exit 0
elif grep -q '\[LONA-TEST-RESULT:FAIL\]' "$OUTPUT_FILE"; then
    echo "RESULT: Tests FAILED"
    # Show which tests failed
    grep '\[FAIL\]' "$OUTPUT_FILE" || true
    exit 1
fi

# No result markers found
if [[ $ELAPSED -ge $TIMEOUT ]]; then
    echo "TIMEOUT: Tests did not complete within ${TIMEOUT}s"
    echo "No test result markers found in output."
    exit 2
else
    echo "ERROR: QEMU exited unexpectedly"
    echo "No test result marker found in output"
    echo "Expected [LONA-TEST-RESULT:PASS] or [LONA-TEST-RESULT:FAIL]"
    exit 3
fi
