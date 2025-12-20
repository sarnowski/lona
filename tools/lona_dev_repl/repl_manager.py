# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
"""QEMU process and REPL interaction management.

This module handles:
- Starting/stopping QEMU via Docker Compose
- Building the Lona image via make debug-arm64
- Communicating with the Lona REPL over serial console
- Parsing REPL output and detecting prompts
"""

import asyncio
import atexit
import re
from pathlib import Path

# REPL prompt patterns
PROMPT_READY = "lona> "
PROMPT_CONTINUATION = "...> "

# Timeout for eval operations (seconds)
EVAL_TIMEOUT = 30.0

# Timeout for REPL boot (seconds)
BOOT_TIMEOUT = 60.0

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
        self.buffer: str = ""
        self._lock = asyncio.Lock()

        # Register cleanup on exit
        atexit.register(self._sync_cleanup)

    def _sync_cleanup(self) -> None:
        """Synchronous cleanup for atexit handler."""
        if self.process is not None:
            try:
                self.process.terminate()
            except ProcessLookupError:
                pass

    async def _cleanup_stale_containers(self) -> None:
        """Remove any stale runner containers before starting."""
        proc = await asyncio.create_subprocess_exec(
            "docker",
            "compose",
            "rm",
            "-f",
            "-s",
            "runner-arm64",
            cwd=self.project_root,
            stdout=asyncio.subprocess.DEVNULL,
            stderr=asyncio.subprocess.DEVNULL,
        )
        await proc.wait()

    async def _build(self) -> tuple[bool, str]:
        """Build the Lona image using make debug-arm64.

        Returns:
            Tuple of (success, message)
        """
        proc = await asyncio.create_subprocess_exec(
            "make",
            "debug-arm64",
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

        Returns:
            Tuple of (success, message)
        """
        await self._cleanup_stale_containers()

        # Start QEMU with -T to disable TTY allocation for proper piping
        self.process = await asyncio.create_subprocess_exec(
            "docker",
            "compose",
            "run",
            "--rm",
            "-T",
            "runner-arm64",
            cwd=self.project_root,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.STDOUT,
        )

        self.buffer = ""

        # Wait for the REPL to be ready
        try:
            await self._wait_for_prompt(timeout=BOOT_TIMEOUT)
            return True, "REPL ready"
        except asyncio.TimeoutError:
            await self._stop_qemu()
            return (
                False,
                f"Timeout waiting for REPL (collected output: {self.buffer[:500]})",
            )

    async def _stop_qemu(self) -> None:
        """Stop the QEMU process."""
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
        return self.process is not None and self.process.returncode is None

    async def _ensure_running_locked(self) -> tuple[bool, str]:
        """Ensure QEMU is running (must be called with lock held).

        Returns:
            Tuple of (success, message)
        """
        if self._is_running():
            return True, "Already running"

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

    async def close(self) -> None:
        """Clean up resources."""
        await self._stop_qemu()
