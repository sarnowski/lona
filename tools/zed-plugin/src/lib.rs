// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Zed editor extension for the Lonala programming language.
//!
//! This extension provides:
//! - Syntax highlighting via Tree-sitter for `.lona` files
//! - LSP integration via the `lonala-lsp` server

use zed_extension_api as zed;

struct LonalaExtension;

impl zed::Extension for LonalaExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        let command = worktree.which("lonala-lsp").ok_or_else(|| {
            "lonala-lsp not found in PATH. Install with: cargo install --path crates/lonala-lsp"
                .to_string()
        })?;

        Ok(zed::Command {
            command,
            args: vec![],
            env: Default::default(),
        })
    }
}

zed::register_extension!(LonalaExtension);
