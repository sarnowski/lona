# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright 2026 Tobias Sarnowski

"""
MCP server for Lona Development REPL.

Provides tools for evaluating Lonala code in QEMU and managing QEMU instances.
Supports multiple architectures (aarch64, x86_64) running in parallel.
"""

from mcp.server.fastmcp import FastMCP

from .process_manager import ProcessManager

# Valid architectures
VALID_ARCHS = {"aarch64", "x86_64"}
DEFAULT_ARCH = "aarch64"

# Create MCP server
mcp = FastMCP(
    "lona-dev-repl",
    instructions="""
Lona Development REPL - Test Lonala code interactively in QEMU.

Use 'eval' to evaluate Lonala expressions. QEMU starts automatically on first use.
Use 'restart' after code changes to rebuild and restart with updated code.

Supports multiple architectures running in parallel:
- aarch64 (default): ARM 64-bit
- x86_64: Intel/AMD 64-bit

Each architecture has an independent QEMU instance with a 60-second idle timeout.
""",
)

# Global process manager (lazily initialized)
_manager: ProcessManager | None = None


def _get_manager() -> ProcessManager:
    """Get or create the process manager."""
    global _manager
    if _manager is None:
        _manager = ProcessManager()
    return _manager


def _format_result(arch: str, output: str) -> str:
    """Format result with architecture and timestamp metadata."""
    manager = _get_manager()
    start_time = manager.get_start_time(arch)

    if start_time is not None:
        timestamp = start_time.strftime("%Y-%m-%d %H:%M:%S")
        header = f"[{arch} | started {timestamp}]"
    else:
        header = f"[{arch} | not started]"

    return f"{header}\n\n{output}"


def _validate_arch(arch: str) -> str:
    """Validate and normalize architecture name."""
    arch = arch.lower().strip()
    if arch not in VALID_ARCHS:
        valid = ", ".join(sorted(VALID_ARCHS))
        raise ValueError(f"Invalid architecture '{arch}'. Valid: {valid}")
    return arch


@mcp.tool()
async def eval(code: str, arch: str = DEFAULT_ARCH) -> str:
    """
    Evaluate a Lonala expression in the REPL.

    QEMU is started automatically if not already running.
    Results include architecture and start timestamp for context.

    Args:
        code: Lonala expression to evaluate
        arch: Target architecture (aarch64 or x86_64, default: aarch64)

    Returns:
        Evaluation result with metadata header
    """
    arch = _validate_arch(arch)
    manager = _get_manager()

    try:
        result = await manager.eval(arch, code)
        return _format_result(arch, result)
    except TimeoutError as e:
        return _format_result(arch, f"Error: {e}")
    except RuntimeError as e:
        return _format_result(arch, f"Error: {e}")


@mcp.tool()
async def restart(arch: str = DEFAULT_ARCH) -> str:
    """
    Restart QEMU for the given architecture.

    Kills any running QEMU instance and starts a fresh one.
    Use this after making code changes to rebuild and test.

    Args:
        arch: Target architecture (aarch64 or x86_64, default: aarch64)

    Returns:
        Status message with metadata header
    """
    arch = _validate_arch(arch)
    manager = _get_manager()

    try:
        await manager.restart(arch)
        return _format_result(arch, "QEMU restarted successfully. Ready for input.")
    except TimeoutError as e:
        return _format_result(arch, f"Error during restart: {e}")
    except RuntimeError as e:
        return _format_result(arch, f"Error during restart: {e}")


async def cleanup() -> None:
    """Clean up all resources."""
    global _manager
    if _manager is not None:
        await _manager.close()
        _manager = None


async def serve() -> None:
    """Run the MCP server."""
    await mcp.run_stdio_async()
