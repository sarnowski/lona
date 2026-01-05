// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Reader for Lonala source code.
//!
//! Converts source code strings into Lonala values.

#[cfg(test)]
mod lexer_test;
#[cfg(test)]
mod parser_test;

mod lexer;
mod parser;

pub use lexer::{LexError, Lexer, Token, TokenString};
pub use parser::{ParseError, Parser, ReadError, read};
