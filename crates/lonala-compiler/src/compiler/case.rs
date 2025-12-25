// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Case special form compilation.
//!
//! This module handles compilation of the `case` special form for
//! efficient value-based dispatch on compile-time constants.
//!
//! # Syntax
//!
//! ```text
//! (case expr
//!   pattern1 result1
//!   pattern2 result2
//!   ...
//!   :else default-result)
//! ```
//!
//! # Valid Patterns
//!
//! - Integer literals (e.g., `1`, `42`, `-5`)
//! - Keywords (e.g., `:ok`, `:error`)
//! - Strings (e.g., `"hello"`)
//! - `nil`
//! - Booleans (`true`, `false`)
//!
//! Symbols, lists, vectors, maps, sets, and floats are NOT valid patterns.

extern crate alloc;

use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

use lona_core::chunk::Constant;
use lona_core::opcode::{Opcode, encode_abc, encode_abx, encode_asbx};
use lona_core::span::Span;
use lonala_parser::{Ast, Spanned};

use super::{Compiler, ExprResult};
use crate::error::{Error, Kind as ErrorKind};

/// Parsed case clauses: list of (pattern, result) pairs and optional default.
type CaseClauses<'args> = (
    Vec<(&'args Spanned<Ast>, &'args Spanned<Ast>)>,
    Option<&'args Spanned<Ast>>,
);

/// A pattern value used for duplicate detection.
///
/// Since patterns are compile-time constants, we can use a simpler
/// representation for equality checking during compilation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PatternValue {
    Nil,
    Bool(bool),
    Integer(i64),
    Keyword(String),
    String(String),
}

impl Compiler<'_, '_, '_> {
    /// Compiles a `case` special form.
    ///
    /// Syntax: `(case expr pattern1 result1 pattern2 result2 ... [:else default])`
    ///
    /// Evaluates `expr` once, then compares against each pattern in order.
    /// Returns the result of the first matching pattern's result expression.
    /// If no pattern matches and no `:else` is provided, triggers a runtime error.
    ///
    /// For tail call optimization: the test expression is never in tail position,
    /// but all result expressions inherit the current tail position.
    pub(super) fn compile_case(
        &mut self,
        args: &[Spanned<Ast>],
        span: Span,
    ) -> Result<ExprResult, Error> {
        // Need at least the expression to test
        if args.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "case",
                    message: "expected (case expr pattern1 result1 ...)",
                },
                self.location(span),
            ));
        }

        // First argument is the expression to test
        let test_expr = args
            .first()
            .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

        // Remaining arguments are pattern/result pairs (and optional :else)
        let clauses = args.get(1_usize..).unwrap_or(&[]);

        // Parse clauses into (pattern, result) pairs and optional default
        let (pairs, default_expr) = self.parse_case_clauses(clauses, span)?;

        // Validate all patterns and check for duplicates
        let mut seen_patterns: BTreeSet<PatternValue> = BTreeSet::new();
        for &(pattern_ast, _result_ast) in &pairs {
            let pattern_value = self.validate_case_pattern(pattern_ast)?;
            if !seen_patterns.insert(pattern_value) {
                return Err(Error::new(
                    ErrorKind::InvalidSpecialForm {
                        form: "case",
                        message: "duplicate pattern in case expression",
                    },
                    self.location(pattern_ast.span),
                ));
            }
        }

        // Save tail position - result expressions inherit it, but test does not
        let is_tail = self.in_tail_position;

        // Compile test expression NOT in tail position
        let test_result =
            self.with_tail_position(false, |compiler| compiler.compile_expr(test_expr))?;
        let test_reg = test_result.register;

        // Allocate destination register for result
        let dest = self.alloc_register(span)?;

        // Track jump-to-end instructions that need patching
        let mut jumps_to_end: alloc::vec::Vec<usize> = alloc::vec::Vec::new();

        // Compile each pattern/result pair
        for (pattern_ast, result_ast) in pairs {
            // Load pattern constant into a temp register
            let pattern_const = self.pattern_to_constant(pattern_ast)?;
            let pattern_const_idx = self.add_constant(pattern_const, pattern_ast.span)?;
            let pattern_reg = self.alloc_register(pattern_ast.span)?;
            self.chunk.emit(
                encode_abx(Opcode::LoadK, pattern_reg, pattern_const_idx),
                pattern_ast.span,
            );

            // Compare: result in a temp register
            let cmp_reg = self.alloc_register(pattern_ast.span)?;
            self.chunk.emit(
                encode_abc(Opcode::Eq, cmp_reg, test_reg, pattern_reg),
                pattern_ast.span,
            );

            // JumpIfNot to next clause (will patch offset later)
            let jump_to_next_idx = self
                .chunk
                .emit(encode_asbx(Opcode::JumpIfNot, cmp_reg, 0), pattern_ast.span);

            // Free temp registers for pattern and comparison
            self.free_registers_to(dest.saturating_add(1));

            // Compile result expression (inherits tail position)
            let result_result =
                self.with_tail_position(is_tail, |compiler| compiler.compile_expr(result_ast))?;

            // Move result to dest if needed
            if result_result.register != dest {
                self.chunk.emit(
                    encode_abc(Opcode::Move, dest, result_result.register, 0),
                    result_ast.span,
                );
            }

            // Free any temps from result but keep dest
            self.free_registers_to(dest.saturating_add(1));

            // Jump to end (will patch offset later)
            let jump_to_end_idx = self.chunk.emit(encode_asbx(Opcode::Jump, 0, 0), span);
            jumps_to_end.push(jump_to_end_idx);

            // Patch jump_to_next to point here
            let next_offset = self
                .chunk
                .len()
                .saturating_sub(jump_to_next_idx)
                .saturating_sub(1);
            let next_offset_i16 = i16::try_from(next_offset)
                .map_err(|_err| Error::new(ErrorKind::JumpTooLarge, self.location(span)))?;
            self.chunk.patch(
                jump_to_next_idx,
                encode_asbx(Opcode::JumpIfNot, cmp_reg, next_offset_i16),
            );
        }

        // Compile default or emit CaseFail
        if let Some(default_ast) = default_expr {
            // Compile default expression (inherits tail position)
            let default_result =
                self.with_tail_position(is_tail, |compiler| compiler.compile_expr(default_ast))?;

            // Move result to dest if needed
            if default_result.register != dest {
                self.chunk.emit(
                    encode_abc(Opcode::Move, dest, default_result.register, 0),
                    default_ast.span,
                );
            }
        } else {
            // No default - emit CaseFail with the test value for diagnostics
            self.chunk
                .emit(encode_abc(Opcode::CaseFail, test_reg, 0, 0), span);
        }

        // Free temps but keep dest
        self.free_registers_to(dest.saturating_add(1));

        // Patch all jumps to end
        let end_pos = self.chunk.len();
        for jump_idx in jumps_to_end {
            let end_offset = end_pos.saturating_sub(jump_idx).saturating_sub(1);
            let end_offset_i16 = i16::try_from(end_offset)
                .map_err(|_err| Error::new(ErrorKind::JumpTooLarge, self.location(span)))?;
            self.chunk
                .patch(jump_idx, encode_asbx(Opcode::Jump, 0, end_offset_i16));
        }

        Ok(ExprResult { register: dest })
    }

    /// Parses case clauses into pattern/result pairs and optional default.
    ///
    /// Returns: `(Vec<(pattern, result)>, Option<default_expr>)`
    fn parse_case_clauses<'args>(
        &self,
        clauses: &'args [Spanned<Ast>],
        span: Span,
    ) -> Result<CaseClauses<'args>, Error> {
        let mut pairs = Vec::new();
        let mut default_expr: Option<&'args Spanned<Ast>> = None;
        let mut idx = 0_usize;

        while idx < clauses.len() {
            let pattern = clauses
                .get(idx)
                .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;

            // Check for :else default clause
            if let Ast::Keyword(ref name) = pattern.node
                && name == "else"
            {
                // :else must be followed by exactly one expression
                idx = idx.saturating_add(1);
                if idx >= clauses.len() {
                    return Err(Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "case",
                            message: ":else must be followed by a result expression",
                        },
                        self.location(pattern.span),
                    ));
                }
                default_expr = clauses.get(idx);
                idx = idx.saturating_add(1);

                // :else must be the last clause
                if idx < clauses.len() {
                    return Err(Error::new(
                        ErrorKind::InvalidSpecialForm {
                            form: "case",
                            message: ":else must be the last clause",
                        },
                        self.location(pattern.span),
                    ));
                }
                break;
            }

            // Regular pattern/result pair
            idx = idx.saturating_add(1);
            if idx >= clauses.len() {
                return Err(Error::new(
                    ErrorKind::InvalidSpecialForm {
                        form: "case",
                        message: "pattern must be followed by a result expression",
                    },
                    self.location(pattern.span),
                ));
            }
            let result = clauses
                .get(idx)
                .ok_or_else(|| Error::new(ErrorKind::EmptyCall, self.location(span)))?;
            pairs.push((pattern, result));
            idx = idx.saturating_add(1);
        }

        Ok((pairs, default_expr))
    }

    /// Validates that a pattern is a compile-time constant.
    ///
    /// Returns a `PatternValue` for duplicate detection.
    fn validate_case_pattern(&self, pattern: &Spanned<Ast>) -> Result<PatternValue, Error> {
        match pattern.node {
            Ast::Nil => Ok(PatternValue::Nil),
            Ast::Bool(bool_val) => Ok(PatternValue::Bool(bool_val)),
            Ast::Integer(num) => Ok(PatternValue::Integer(num)),
            Ast::Keyword(ref name) => Ok(PatternValue::Keyword(String::from(name.as_str()))),
            Ast::String(ref text) => Ok(PatternValue::String(String::from(text.as_str()))),

            // Invalid patterns
            Ast::Float(_) => Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "case",
                    message: "float patterns are not allowed (NaN and precision issues)",
                },
                self.location(pattern.span),
            )),
            Ast::Symbol(_) => Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "case",
                    message: "symbol patterns are not allowed (would be ambiguous with bindings)",
                },
                self.location(pattern.span),
            )),
            Ast::List(_) => Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "case",
                    message: "list patterns are not allowed (use cond for complex matching)",
                },
                self.location(pattern.span),
            )),
            Ast::Vector(_) => Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "case",
                    message: "vector patterns are not allowed (use cond for complex matching)",
                },
                self.location(pattern.span),
            )),
            Ast::Map(_) => Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "case",
                    message: "map patterns are not allowed (use cond for complex matching)",
                },
                self.location(pattern.span),
            )),
            Ast::Set(_) => Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "case",
                    message: "set patterns are not allowed (use cond for complex matching)",
                },
                self.location(pattern.span),
            )),
            Ast::WithMeta { ref value, .. } => {
                // For metadata, validate the inner value
                self.validate_case_pattern(value)
            }
            // Wildcard required for #[non_exhaustive] - handles future variants
            _ => Err(Error::new(
                ErrorKind::InvalidSpecialForm {
                    form: "case",
                    message: "invalid pattern type",
                },
                self.location(pattern.span),
            )),
        }
    }

    /// Converts a validated pattern AST to a constant for loading.
    fn pattern_to_constant(&mut self, pattern: &Spanned<Ast>) -> Result<Constant, Error> {
        match pattern.node {
            Ast::Nil => Ok(Constant::Nil),
            Ast::Bool(bool_val) => Ok(Constant::Bool(bool_val)),
            Ast::Integer(num) => Ok(Constant::Integer(num)),
            Ast::Keyword(ref name) => {
                // Keywords are stored with the Keyword constant type
                let id = self.interner.intern(name);
                Ok(Constant::Keyword(id))
            }
            Ast::String(ref text) => Ok(Constant::String(String::from(text.as_str()))),
            Ast::WithMeta { ref value, .. } => self.pattern_to_constant(value),
            // Explicit variants for clippy::wildcard_enum_match_arm compliance
            // These should have been caught by validate_case_pattern
            Ast::Float(_)
            | Ast::Symbol(_)
            | Ast::List(_)
            | Ast::Vector(_)
            | Ast::Map(_)
            | Ast::Set(_)
            // Wildcard required for #[non_exhaustive] - handles future variants
            | _ => Err(Error::new(
                ErrorKind::InternalError {
                    message: "pattern_to_constant called with invalid pattern",
                },
                self.location(pattern.span),
            )),
        }
    }
}
