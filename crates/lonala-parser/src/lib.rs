// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Lexer and parser for the Lonala language.
//!
//! This crate provides lexical analysis (tokenization) for Lonala S-expression
//! source code. It is designed to be `no_std` compatible for use in the Lona
//! runtime.
//!
//! # Modules
//!
//! - [`error`] - Error types for lexing and parsing
//! - [`token`] - Token types produced by the lexer
//! - [`lexer`] - The lexical analyzer
//!
//! # Example
//!
//! ```
//! use lonala_parser::{tokenize, token::Kind};
//!
//! let tokens = tokenize("(+ 1 2)").unwrap();
//! assert_eq!(tokens.len(), 5);
//! assert_eq!(tokens[0].kind, Kind::LeftParen);
//! assert_eq!(tokens[1].kind, Kind::Symbol);
//! assert_eq!(tokens[1].lexeme, "+");
//! ```

#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod error;
pub mod lexer;
pub mod token;

// Re-exports for convenience
pub use error::{Error, Kind as ErrorKind, Span};
pub use lexer::Lexer;
pub use token::{Kind as TokenKind, Token};

#[cfg(feature = "alloc")]
pub use lexer::tokenize;
