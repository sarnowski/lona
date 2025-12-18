// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Human-readable error formatting for the Lonala language.
//!
//! This crate is the **single source of truth** for converting errors to
//! human-readable text. It provides Rust-style error messages with source
//! context, precise locations, and helpful suggestions.
//!
//! # Design Philosophy
//!
//! - **Centralized formatting**: All error formatting happens here, not in
//!   individual error types
//! - **Structured input**: Errors provide structured data (symbol IDs, type
//!   enums); this crate resolves and formats them
//! - **Consistent output**: All errors follow the same visual format
//! - **Source context**: Errors display relevant source lines with underlines
//!
//! # Modules
//!
//! - [`diagnostic`] - Trait and types for error diagnostics
//! - [`format`] - Error formatting system
//! - [`line_index`] - Byte offset to line/column conversion
//!
//! # Error Implementation Modules
//!
//! - [`parser_errors`] - `Diagnostic` impl for parser errors
//! - [`compiler_errors`] - `Diagnostic` impl for compiler errors
//! - [`vm_errors`] - `Diagnostic` impl for VM errors

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub mod compiler_errors;
#[cfg(feature = "alloc")]
pub mod diagnostic;
#[cfg(feature = "alloc")]
pub mod format;
pub mod line_index;
#[cfg(feature = "alloc")]
pub mod parser_errors;
#[cfg(feature = "alloc")]
pub mod vm_errors;

#[cfg(feature = "alloc")]
pub use diagnostic::{Diagnostic, Note, Severity};
#[cfg(feature = "alloc")]
pub use format::{Config, render};
pub use line_index::{LineCol, LineIndex};
