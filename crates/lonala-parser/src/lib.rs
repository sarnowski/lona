// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Lexer and parser for the Lonala language.
//!
//! This crate provides lexical analysis (tokenization) and parsing for Lonala
//! S-expression source code. It is designed to be `no_std` compatible for use
//! in the Lona runtime.
//!
//! # Modules
//!
//! - [`error`] - Error types for lexing and parsing
//! - [`token`] - Token types produced by the lexer
//! - [`lexer`] - The lexical analyzer
//! - [`ast`] - Abstract syntax tree types
//! - [`parser`] - The parser that transforms tokens into AST
//!
//! # Example
//!
//! ```
//! use lonala_parser::{parse, parse_one, Ast};
//!
//! // Parse multiple expressions
//! let exprs = parse("(+ 1 2) (- 3 4)").unwrap();
//! assert_eq!(exprs.len(), 2);
//!
//! // Parse single expression
//! let expr = parse_one("'(1 2 3)").unwrap();
//! // expr.node is Ast::List([Symbol("quote"), List([Integer(1), ...])])
//!
//! // Access span information
//! println!("Expression spans bytes {}..{}", expr.span.start, expr.span.end);
//! ```

#![no_std]
// Allow single-char identifiers in closure parameters and loop indices, which is
// conventional Rust style (e.g., `|c| c.is_digit()`, `for (i, x) in iter`). The
// `min_ident_chars` lint is overly strict for these common patterns.
#![expect(
    clippy::min_ident_chars,
    reason = "Single-char closure params and loop indices are idiomatic Rust"
)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
pub mod ast;
pub mod error;
pub mod lexer;
#[cfg(feature = "alloc")]
pub mod parser;
pub mod token;

// Re-exports for convenience
pub use error::{Error, Kind as ErrorKind, Span};
pub use lexer::Lexer;
pub use token::{Kind as TokenKind, Token};

#[cfg(feature = "alloc")]
pub use ast::{Ast, Spanned};
#[cfg(feature = "alloc")]
pub use lexer::tokenize;
#[cfg(feature = "alloc")]
pub use parser::{Parser, parse, parse_one};
