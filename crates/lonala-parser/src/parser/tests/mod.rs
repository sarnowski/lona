// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Tests for the Lonala parser.

mod atom_tests;
mod collection_tests;
mod reader_macro_tests;

extern crate alloc;

use alloc::vec::Vec;

use crate::ast::Ast;
use crate::parser::{parse, parse_one};

/// Helper to parse and return the AST node, ignoring spans.
pub fn parse_ast(source: &str) -> Ast {
    parse_one(source).expect("parse should succeed").node
}

/// Helper to parse and return all AST nodes.
pub fn parse_asts(source: &str) -> Vec<Ast> {
    parse(source)
        .expect("parse should succeed")
        .into_iter()
        .map(|spanned| spanned.node)
        .collect()
}
