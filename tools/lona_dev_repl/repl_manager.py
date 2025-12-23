# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
"""QEMU process and REPL interaction management.

This module handles:
- Starting/stopping QEMU via Docker Compose
- Building the Lona image via make debug-aarch64
- Communicating with the Lona REPL over serial console
- Parsing REPL output and detecting prompts
"""

import asyncio
import atexit
import re
import subprocess
from datetime import datetime, timezone
from pathlib import Path

# REPL prompt patterns
PROMPT_READY = "lona> "
PROMPT_CONTINUATION = "...> "

# Timeout for eval operations (seconds)
EVAL_TIMEOUT = 30.0

# Timeout for REPL boot (seconds)
BOOT_TIMEOUT = 60.0

# Idle shutdown settings
IDLE_TIMEOUT = 60.0  # Shutdown after 60 seconds of inactivity
IDLE_CHECK_INTERVAL = 10.0  # Check for idle every 10 seconds

# ANSI escape code pattern
ANSI_ESCAPE = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


def strip_ansi(text: str) -> str:
    """Remove ANSI escape codes from text."""
    return ANSI_ESCAPE.sub("", text)


def clean_output(raw: str, sent_command: str) -> str:
    """Clean REPL output by removing echo, ANSI codes, and extra whitespace.

    Args:
        raw: Raw output from REPL (between command send and next prompt)
        sent_command: The command that was sent (to strip echo)

    Returns:
        Cleaned result string
    """
    # Strip ANSI codes
    cleaned = strip_ansi(raw)

    # Normalize line endings
    cleaned = cleaned.replace("\r\n", "\n").replace("\r", "\n")

    # Strip the echoed command from the beginning
    # The REPL echoes back what we type, so we need to remove it
    lines = cleaned.split("\n")
    result_lines = []
    found_command = False

    for line in lines:
        stripped = line.strip()
        if not found_command:
            # Check if this line contains the echoed command
            # Handle potential partial echoes across lines
            if sent_command.strip() in stripped or stripped in sent_command.strip():
                found_command = True
                continue
        result_lines.append(line)

    result = "\n".join(result_lines).strip()
    return result


class ReplManager:
    """Manages a QEMU instance running the Lona REPL.

    Provides async methods for:
    - eval(expression): Evaluate a Lonala expression
    - restart(): Rebuild and restart QEMU
    - ensure_running(): Start QEMU if not already running
    """

    def __init__(self, project_root: Path | None = None):
        """Initialize the REPL manager.

        Args:
            project_root: Path to the Lona project root. If None, uses current directory.
        """
        self.project_root = project_root or Path.cwd()
        self.process: asyncio.subprocess.Process | None = None
        self.container_id: str | None = None  # Track our specific Docker container
        self.buffer: str = ""
        self.started_at: datetime | None = None
        self._lock = asyncio.Lock()
        self._last_activity: datetime | None = None
        self._idle_monitor_task: asyncio.Task | None = None

        # Register cleanup on exit
        atexit.register(self._sync_cleanup)

    def _sync_cleanup(self) -> None:
        """Synchronous cleanup for atexit handler."""
        # Kill the Docker container directly (this is the reliable way)
        if self.container_id is not None:
            try:
                subprocess.run(
                    ["docker", "kill", self.container_id],
                    stdout=subprocess.DEVNULL,
                    stderr=subprocess.DEVNULL,
                    timeout=5.0,
                )
            except (subprocess.TimeoutExpired, FileNotFoundError):
                pass
        # Also terminate the attach process
        if self.process is not None:
            try:
                self.process.terminate()
            except ProcessLookupError:
                pass


    async def _build(self) -> tuple[bool, str]:
        """Build the Lona image using make debug-aarch64.

        Returns:
            Tuple of (success, message)
        """
        proc = await asyncio.create_subprocess_exec(
            "make",
            "debug-aarch64",
            cwd=self.project_root,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.STDOUT,
        )
        stdout, _ = await proc.communicate()
        output = stdout.decode("utf-8", errors="replace")

        if proc.returncode != 0:
            return False, f"Build failed:\n{output}"
        return True, "Build successful"

    async def _start_qemu(self) -> tuple[bool, str]:
        """Start QEMU via docker compose.

        Uses detached mode to get the container ID, then attaches to it.
        This allows us to reliably kill exactly our container without
        affecting other parallel sessions.

        Returns:
            Tuple of (success, message)
        """
        # Start container in detached mode to get its ID
        # -T disables TTY allocation for proper piping
        proc = await asyncio.create_subprocess_exec(
            "docker",
            "compose",
            "run",
            "--rm",
            "-d",
            "-T",
            "runner-aarch64",
            cwd=self.project_root,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        stdout, stderr = await proc.communicate()

        if proc.returncode != 0:
            error_msg = stderr.decode("utf-8", errors="replace")
            return False, f"Failed to start container: {error_msg}"

        self.container_id = stdout.decode("utf-8", errors="replace").strip()

        # Attach to the container's stdin/stdout
        # --sig-proxy=false prevents signals from being forwarded (we handle shutdown ourselves)
        self.process = await asyncio.create_subprocess_exec(
            "docker",
            "attach",
            "--sig-proxy=false",
            self.container_id,
            cwd=self.project_root,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.STDOUT,
        )

        self.buffer = ""

        # Wait for the REPL to be ready
        try:
            await self._wait_for_prompt(timeout=BOOT_TIMEOUT)
            self.started_at = datetime.now(timezone.utc)
            self._last_activity = datetime.now(timezone.utc)
            self._start_idle_monitor()
            return True, "REPL ready"
        except asyncio.TimeoutError:
            await self._stop_qemu()
            return (
                False,
                f"Timeout waiting for REPL (collected output: {self.buffer[:500]})",
            )

    async def _stop_qemu(self) -> None:
        """Stop the QEMU process by killing our specific container."""
        # Kill the Docker container directly by ID
        # This is the only reliable way to stop QEMU when running via docker compose
        if self.container_id is not None:
            proc = await asyncio.create_subprocess_exec(
                "docker",
                "kill",
                self.container_id,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )
            await proc.wait()
            self.container_id = None

        # Terminate the attach process
        if self.process is not None:
            try:
                self.process.terminate()
                await asyncio.wait_for(self.process.wait(), timeout=5.0)
            except (ProcessLookupError, asyncio.TimeoutError):
                try:
                    self.process.kill()
                except ProcessLookupError:
                    pass
            self.process = None

        self.buffer = ""
        self.started_at = None
        self._last_activity = None

        # Cancel idle monitor if it's not the current task (avoid self-cancel)
        if self._idle_monitor_task is not None:
            try:
                current = asyncio.current_task()
            except RuntimeError:
                current = None
            if self._idle_monitor_task is not current:
                self._idle_monitor_task.cancel()
            self._idle_monitor_task = None

    async def _read_chunk(self, timeout: float) -> str | None:
        """Read a chunk of output from QEMU.

        Returns:
            The chunk of text, or None if EOF/process dead
        """
        if self.process is None or self.process.stdout is None:
            return None

        try:
            chunk = await asyncio.wait_for(
                self.process.stdout.read(4096),
                timeout=timeout,
            )
            if not chunk:
                return None
            return chunk.decode("utf-8", errors="replace")
        except asyncio.TimeoutError:
            raise

    async def _wait_for_prompt(self, timeout: float) -> str:
        """Wait for a REPL prompt, accumulating output.

        Returns:
            The type of prompt found ("ready" or "continuation")

        Raises:
            asyncio.TimeoutError: If no prompt found within timeout
        """
        deadline = asyncio.get_event_loop().time() + timeout

        while True:
            remaining = deadline - asyncio.get_event_loop().time()
            if remaining <= 0:
                raise asyncio.TimeoutError()

            chunk = await self._read_chunk(min(remaining, 1.0))
            if chunk is None:
                raise asyncio.TimeoutError()

            self.buffer += chunk

            # Check for prompts (check ready first since continuation might appear during boot)
            clean_buffer = strip_ansi(self.buffer)
            if PROMPT_READY in clean_buffer:
                return "ready"
            if PROMPT_CONTINUATION in clean_buffer:
                return "continuation"

    def _is_running(self) -> bool:
        """Check if QEMU process is running."""
        # Both the container and the attach process must be alive
        return (
            self.container_id is not None
            and self.process is not None
            and self.process.returncode is None
        )

    def _start_idle_monitor(self) -> None:
        """Start the idle monitor task if not already running."""
        if self._idle_monitor_task is None or self._idle_monitor_task.done():
            self._idle_monitor_task = asyncio.create_task(self._idle_monitor())

    async def _idle_monitor(self) -> None:
        """Background task that shuts down QEMU after idle timeout."""
        try:
            while True:
                await asyncio.sleep(IDLE_CHECK_INTERVAL)
                async with self._lock:
                    if not self._is_running():
                        # QEMU not running, stop monitoring
                        break
                    if self._last_activity is None:
                        continue
                    elapsed = (
                        datetime.now(timezone.utc) - self._last_activity
                    ).total_seconds()
                    if elapsed >= IDLE_TIMEOUT:
                        await self._stop_qemu()
                        break
        except asyncio.CancelledError:
            pass  # Clean shutdown

    def get_started_at_iso(self) -> str | None:
        """Get the QEMU start time as an ISO 8601 string."""
        if self.started_at is None:
            return None
        return self.started_at.isoformat()

    async def _ensure_running_locked(self) -> tuple[bool, str]:
        """Ensure QEMU is running (must be called with lock held).

        Returns:
            Tuple of (success, message)
        """
        if self._is_running():
            return True, "Already running"

        # Clean up stale state if process died externally (e.g., container killed)
        # This ensures orphan containers are killed and state is reset
        if self.container_id is not None or self.process is not None:
            await self._stop_qemu()

        # Build first
        success, msg = await self._build()
        if not success:
            return False, msg

        # Then start QEMU
        return await self._start_qemu()

    async def ensure_running(self) -> tuple[bool, str]:
        """Ensure QEMU is running, starting it if necessary.

        Returns:
            Tuple of (success, message)
        """
        async with self._lock:
            return await self._ensure_running_locked()

    async def restart(self) -> tuple[bool, str]:
        """Stop QEMU, rebuild, and restart.

        Returns:
            Tuple of (success, message)
        """
        async with self._lock:
            # Stop existing instance
            await self._stop_qemu()

            # Rebuild
            success, msg = await self._build()
            if not success:
                return False, msg

            # Start fresh
            return await self._start_qemu()

    async def eval(self, expression: str) -> tuple[bool, str]:
        """Evaluate a Lonala expression.

        Args:
            expression: The Lonala expression to evaluate

        Returns:
            Tuple of (success, result_or_error)
        """
        async with self._lock:
            # Record activity to reset idle timer (also updated at end via finally)
            self._last_activity = datetime.now(timezone.utc)

            try:
                # Ensure QEMU is running (under the same lock to prevent races)
                success, msg = await self._ensure_running_locked()
                if not success:
                    return False, msg

                if self.process is None or self.process.stdin is None:
                    return False, "QEMU stdin not available"

                # Clear buffer from previous operations
                self.buffer = ""

                # Send the expression
                # Add newline to submit
                command = expression + "\n"
                self.process.stdin.write(command.encode("utf-8"))
                await self.process.stdin.drain()

                # Wait for response
                try:
                    prompt_type = await self._wait_for_prompt(timeout=EVAL_TIMEOUT)
                except asyncio.TimeoutError:
                    return False, "Timeout waiting for response"

                if prompt_type == "continuation":
                    # Expression is incomplete
                    return False, "Incomplete expression (missing closing delimiter)"

                # Extract result from buffer
                # Buffer contains: echo + result + prompt
                clean_buffer = strip_ansi(self.buffer)

                # Find the last prompt and take everything before it
                if PROMPT_READY in clean_buffer:
                    result_part = clean_buffer.rsplit(PROMPT_READY, 1)[0]
                else:
                    result_part = clean_buffer

                # Clean up the result
                result = clean_output(result_part, expression)

                return True, result
            finally:
                # Always update activity at end to prevent immediate shutdown
                # after long-running evaluations
                self._last_activity = datetime.now(timezone.utc)

    async def close(self) -> None:
        """Clean up resources."""
        # Acquire lock to safely modify shared state and prevent races
        async with self._lock:
            # Cancel idle monitor (can't await - it may be waiting for this lock)
            if self._idle_monitor_task is not None:
                self._idle_monitor_task.cancel()
                self._idle_monitor_task = None
            await self._stop_qemu()
