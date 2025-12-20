# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
"""MCP server for Lona development REPL.

Provides two tools:
- eval: Evaluate Lonala expressions in a running REPL
- restart: Rebuild and restart QEMU to apply code changes
"""

from pathlib import Path

from mcp.server.fastmcp import FastMCP

from .repl_manager import ReplManager

# Create the MCP server
mcp = FastMCP(
    "lona-dev-repl",
    instructions="""Lona Development REPL Server.

This server provides access to a Lona REPL running in QEMU. Use it to:
- Test Lonala code interactively
- Verify behavior after making changes to the runtime
- Experiment with language features

The REPL maintains state between evaluations, so you can define functions
and variables that persist across calls.

Use `restart` after making changes to Rust code to rebuild and apply them.
""",
)

# Global REPL manager instance
_manager: ReplManager | None = None


def get_manager() -> ReplManager:
    """Get or create the REPL manager singleton."""
    global _manager
    if _manager is None:
        # Find project root (where Makefile is)
        project_root = Path(__file__).parent.parent.parent
        _manager = ReplManager(project_root)
    return _manager


@mcp.tool()
async def eval(expression: str) -> str:
    """Evaluate a Lonala expression in the REPL.

    The expression is sent to a running Lona REPL in QEMU. If QEMU is not
    running, it will be started automatically (including building the image).

    Multi-line expressions are supported. The REPL will wait for complete
    expressions before evaluating.

    Args:
        expression: A Lonala expression to evaluate. Can be multi-line.

    Returns:
        The result of evaluating the expression, or an error message.

    Examples:
        eval("(+ 1 2)")  # Returns: "3"
        eval("(def x 42)")  # Returns: "" (nil result)
        eval("x")  # Returns: "42"
        eval("(defn square [x] (* x x))")  # Define a function
        eval("(square 5)")  # Returns: "25"
    """
    manager = get_manager()
    success, result = await manager.eval(expression)

    if success:
        if result:
            return result
        return "(nil)"
    return f"Error: {result}"


@mcp.tool()
async def restart() -> str:
    """Rebuild Lona and restart the QEMU instance.

    This tool:
    1. Stops the currently running QEMU instance (if any)
    2. Runs `make debug-arm64` to rebuild the Lona image
    3. Starts a fresh QEMU instance with the new image

    Use this after making changes to Rust code in the runtime, kernel,
    or compiler crates. The new REPL will start with a fresh state.

    Returns:
        A status message indicating success or failure.
    """
    manager = get_manager()
    success, message = await manager.restart()

    if success:
        return f"Success: {message}"
    return f"Error: {message}"


async def cleanup() -> None:
    """Clean up resources if manager exists."""
    if _manager is not None:
        await _manager.close()


async def serve() -> None:
    """Run the MCP server over stdio."""
    await mcp.run_stdio_async()
