// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Lonala Language Server Protocol implementation.
//!
//! This crate provides LSP support for the Lonala programming language,
//! enabling editor integration for syntax highlighting, diagnostics,
//! and code intelligence.
//!
//! # Public API
//!
//! - [`document::Document`] - Document content with line index
//! - [`document::Manager`] - Thread-safe document storage
//! - [`semantic_tokens`] - Semantic token classification
//!
//! The server implementation (`server` module) is internal and accessed
//! only via the `lonala-lsp` binary.

pub mod document;
pub mod semantic_tokens;
