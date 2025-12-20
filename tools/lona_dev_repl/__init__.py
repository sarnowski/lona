# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>
"""Lona development REPL MCP server.

This MCP server provides tools for interacting with a Lona REPL running in QEMU:
- eval: Evaluate Lonala expressions and return results
- restart: Rebuild and restart QEMU to apply code changes
"""

from .server import serve

__all__ = ["serve"]
