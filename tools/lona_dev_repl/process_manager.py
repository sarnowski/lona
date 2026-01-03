# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright 2026 Tobias Sarnowski

"""
Process manager for QEMU instances.

Manages the lifecycle of `make run-$ARCH` processes, supporting multiple
architectures running in parallel with independent idle timeouts.
"""

import asyncio
import atexit
import os
import pty
import re
import signal
import subprocess
import time
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path


# Prompt patterns
PROMPT_PATTERN = re.compile(r"lona> $")

# Timeouts
BOOT_TIMEOUT = 120  # seconds to wait for QEMU to boot
EVAL_TIMEOUT = 30  # seconds to wait for evaluation
IDLE_TIMEOUT = 60  # seconds before killing idle QEMU
IDLE_CHECK_INTERVAL = 10  # seconds between idle checks

# ANSI escape code pattern for cleaning output
ANSI_ESCAPE = re.compile(r"\x1b\[[0-9;]*[a-zA-Z]|\x1b\].*?\x07|\x1b[()][AB012]")


def clean_output(text: str) -> str:
    """Remove ANSI escape codes and normalize line endings."""
    text = ANSI_ESCAPE.sub("", text)
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    return text


def find_project_root() -> Path:
    """Find the project root directory (where Makefile is)."""
    current = Path(__file__).resolve().parent
    while current != current.parent:
        if (current / "Makefile").exists():
            return current
        current = current.parent
    raise RuntimeError("Could not find project root (no Makefile found)")


@dataclass
class ProcessState:
    """State for a single architecture's QEMU process."""

    process: subprocess.Popen | None = None
    master_fd: int | None = None
    start_time: datetime | None = None
    last_activity: float = field(default_factory=time.time)
    lock: asyncio.Lock = field(default_factory=asyncio.Lock)
    output_buffer: str = ""


class ProcessManager:
    """Manages QEMU processes for multiple architectures."""

    def __init__(self) -> None:
        self._states: dict[str, ProcessState] = {}
        self._project_root = find_project_root()
        self._idle_task: asyncio.Task | None = None
        self._shutdown = False

        # Register cleanup on exit
        atexit.register(self._sync_cleanup)

    def _get_state(self, arch: str) -> ProcessState:
        """Get or create state for an architecture."""
        if arch not in self._states:
            self._states[arch] = ProcessState()
        return self._states[arch]

    async def ensure_running(self, arch: str) -> None:
        """Ensure QEMU is running for the given architecture."""
        state = self._get_state(arch)

        async with state.lock:
            if state.process is not None and state.process.poll() is None:
                # Already running
                state.last_activity = time.time()
                return

            # Start new process
            await self._start_process(arch, state)

        # Start idle monitor if not running
        if self._idle_task is None or self._idle_task.done():
            self._idle_task = asyncio.create_task(self._idle_monitor())

    async def _start_process(self, arch: str, state: ProcessState) -> None:
        """Start a new QEMU process for the given architecture."""
        # Create pseudo-terminal for interactive docker
        master_fd, slave_fd = pty.openpty()

        # Start make run-$ARCH
        process = subprocess.Popen(
            ["make", f"run-{arch}"],
            stdin=slave_fd,
            stdout=slave_fd,
            stderr=slave_fd,
            cwd=self._project_root,
            start_new_session=True,  # Create new process group for clean killing
        )

        # Close slave fd in parent
        os.close(slave_fd)

        # Set master to non-blocking
        os.set_blocking(master_fd, False)

        state.process = process
        state.master_fd = master_fd
        state.start_time = datetime.now()
        state.last_activity = time.time()
        state.output_buffer = ""

        # Wait for boot (first prompt)
        await self._wait_for_prompt(state, timeout=BOOT_TIMEOUT)

    async def _wait_for_prompt(self, state: ProcessState, timeout: float) -> str:
        """Wait for the REPL prompt and return accumulated output."""
        start = time.time()

        while time.time() - start < timeout:
            if state.process is None or state.process.poll() is not None:
                raise RuntimeError("Process died while waiting for prompt")

            try:
                chunk = os.read(state.master_fd, 4096).decode("utf-8", errors="replace")
                state.output_buffer += chunk
            except BlockingIOError:
                pass  # No data available

            # Check for prompt
            if PROMPT_PATTERN.search(state.output_buffer):
                output = state.output_buffer
                state.output_buffer = ""
                return clean_output(output)

            await asyncio.sleep(0.05)

        raise TimeoutError(f"Timeout waiting for prompt after {timeout}s")

    async def eval(self, arch: str, code: str) -> str:
        """Evaluate code in the REPL for the given architecture."""
        await self.ensure_running(arch)

        state = self._get_state(arch)

        async with state.lock:
            state.last_activity = time.time()

            # Send code
            os.write(state.master_fd, (code + "\n").encode("utf-8"))

            # Wait for response
            output = await self._wait_for_prompt(state, timeout=EVAL_TIMEOUT)

            # Filter output to extract just the result
            # The REPL echoes input on the prompt line (e.g., "lona> 42"), and
            # prints the result on its own line. We skip all prompt lines.
            lines = output.split("\n")
            result_lines = []
            for line in lines:
                stripped = line.strip()
                # Skip prompt lines (includes echoed input)
                if stripped == "lona>" or stripped.startswith("lona> "):
                    continue
                result_lines.append(line)

            return "\n".join(result_lines).strip()

    async def restart(self, arch: str) -> None:
        """Restart QEMU for the given architecture."""
        state = self._get_state(arch)

        async with state.lock:
            await self._kill_process(state)

        # Start fresh
        await self.ensure_running(arch)

    async def _kill_process(self, state: ProcessState) -> None:
        """Kill a QEMU process and clean up."""
        if state.process is None:
            return

        # Kill entire process group
        try:
            pgid = os.getpgid(state.process.pid)
            os.killpg(pgid, signal.SIGTERM)
        except (ProcessLookupError, OSError):
            pass

        # Wait a bit for graceful shutdown
        for _ in range(10):
            if state.process.poll() is not None:
                break
            await asyncio.sleep(0.1)

        # Force kill if still running
        if state.process.poll() is None:
            try:
                pgid = os.getpgid(state.process.pid)
                os.killpg(pgid, signal.SIGKILL)
            except (ProcessLookupError, OSError):
                pass

        # Close file descriptor
        if state.master_fd is not None:
            try:
                os.close(state.master_fd)
            except OSError:
                pass

        state.process = None
        state.master_fd = None
        state.start_time = None
        state.output_buffer = ""

    async def _idle_monitor(self) -> None:
        """Monitor all processes and kill idle ones."""
        while not self._shutdown:
            await asyncio.sleep(IDLE_CHECK_INTERVAL)

            now = time.time()
            for arch, state in list(self._states.items()):
                if state.process is None or state.process.poll() is not None:
                    continue

                idle_time = now - state.last_activity
                if idle_time > IDLE_TIMEOUT:
                    async with state.lock:
                        await self._kill_process(state)

    def get_start_time(self, arch: str) -> datetime | None:
        """Get the start time for an architecture's QEMU instance."""
        state = self._get_state(arch)
        return state.start_time

    def is_running(self, arch: str) -> bool:
        """Check if QEMU is running for the given architecture."""
        state = self._get_state(arch)
        return state.process is not None and state.process.poll() is None

    async def close(self) -> None:
        """Shut down all processes."""
        self._shutdown = True

        if self._idle_task is not None:
            self._idle_task.cancel()
            try:
                await self._idle_task
            except asyncio.CancelledError:
                pass

        for state in self._states.values():
            async with state.lock:
                await self._kill_process(state)

    def _sync_cleanup(self) -> None:
        """Synchronous cleanup for atexit."""
        for state in self._states.values():
            if state.process is not None and state.process.poll() is None:
                try:
                    pgid = os.getpgid(state.process.pid)
                    os.killpg(pgid, signal.SIGKILL)
                except (ProcessLookupError, OSError):
                    pass
            if state.master_fd is not None:
                try:
                    os.close(state.master_fd)
                except OSError:
                    pass
