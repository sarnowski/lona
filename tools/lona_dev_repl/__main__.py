# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
"""Entry point for the Lona development REPL MCP server.

Usage:
    python -m tools.lona_dev_repl
"""

import asyncio
import signal

from .server import cleanup, serve


async def run_with_cleanup() -> None:
    """Run the MCP server with proper cleanup on shutdown."""
    loop = asyncio.get_running_loop()

    # Set up signal handlers for graceful shutdown
    def signal_handler() -> None:
        asyncio.create_task(cleanup())

    for sig in (signal.SIGTERM, signal.SIGINT):
        loop.add_signal_handler(sig, signal_handler)

    try:
        await serve()
    finally:
        # Ensure cleanup on any exit
        await cleanup()


def main() -> None:
    """Run the MCP server."""
    asyncio.run(run_with_cleanup())


if __name__ == "__main__":
    main()
