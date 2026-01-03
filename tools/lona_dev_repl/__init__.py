# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright 2026 Tobias Sarnowski

"""
Lona Development REPL MCP Server.

This MCP server provides tools for interacting with a Lona REPL running in QEMU:
- eval: Evaluate Lonala expressions and return results
- restart: Restart QEMU to apply code changes

Supports multiple architectures (aarch64, x86_64) running in parallel.
"""

from .server import cleanup, serve

__all__ = ["serve", "cleanup"]
