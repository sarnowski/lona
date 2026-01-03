# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright 2026 Tobias Sarnowski

"""Entry point for running the MCP server."""

import asyncio
import signal

from .server import cleanup, serve


async def run_with_cleanup() -> None:
    """Run the server with proper cleanup on shutdown."""
    loop = asyncio.get_event_loop()

    # Register signal handlers for graceful shutdown
    for sig in (signal.SIGTERM, signal.SIGINT):
        loop.add_signal_handler(sig, lambda: asyncio.create_task(cleanup()))

    try:
        await serve()
    finally:
        await cleanup()


def main() -> None:
    """Main entry point."""
    asyncio.run(run_with_cleanup())


if __name__ == "__main__":
    main()
