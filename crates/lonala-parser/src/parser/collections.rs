// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Collection parsing (lists, vectors, maps, sets, anonymous functions).

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::ast::{Ast, Spanned};
use crate::error::{Error, Kind as ErrorKind, Span};
use crate::token::Kind as TokenKind;

use super::Parser;

/// Finds all placeholders in the AST and returns `(max_arg_number, has_rest)`.
///
/// - `%` and `%1` both count as argument 1
/// - `%N` counts as argument N (for N in 1..=9)
/// - `%&` sets `has_rest` to true
fn find_placeholders(elements: &[Spanned<Ast>]) -> (usize, bool) {
    let mut max_arg = 0_usize;
    let mut has_rest = false;

    for elem in elements {
        find_placeholders_in_ast(&elem.node, &mut max_arg, &mut has_rest);
    }

    (max_arg, has_rest)
}

/// Recursively finds placeholders in an AST node.
fn find_placeholders_in_ast(ast: &Ast, max_arg: &mut usize, has_rest: &mut bool) {
    match *ast {
        Ast::Symbol(ref name) => {
            if let Some(arg_num) = parse_placeholder(name) {
                if arg_num == 0 {
                    // %& (rest args)
                    *has_rest = true;
                } else if arg_num > *max_arg {
                    *max_arg = arg_num;
                } else {
                    // arg_num <= *max_arg, nothing to update
                }
            }
        }
        Ast::List(ref elems)
        | Ast::Vector(ref elems)
        | Ast::Set(ref elems)
        | Ast::Map(ref elems) => {
            for elem in elems {
                find_placeholders_in_ast(&elem.node, max_arg, has_rest);
            }
        }
        // WithMeta wraps another node that may contain placeholders
        Ast::WithMeta {
            ref meta,
            ref value,
        } => {
            find_placeholders_in_ast(&meta.node, max_arg, has_rest);
            find_placeholders_in_ast(&value.node, max_arg, has_rest);
        }
        // Other types don't contain nested expressions
        Ast::Integer(_)
        | Ast::Float(_)
        | Ast::String(_)
        | Ast::Bool(_)
        | Ast::Nil
        | Ast::Keyword(_) => {}
    }
}

/// Parses a placeholder symbol and returns the argument number.
///
/// Returns:
/// - `Some(0)` for `%&` (rest args)
/// - `Some(1)` for `%` or `%1`
/// - `Some(N)` for `%N` where N is 2-9
/// - `None` for non-placeholder symbols
fn parse_placeholder(name: &str) -> Option<usize> {
    if name == "%" || name == "%1" {
        Some(1)
    } else if name == "%&" {
        Some(0) // Special marker for rest args
    } else if name.starts_with('%') && name.len() == 2 {
        // Check for %2 through %9
        let digit_char = name.chars().nth(1)?;
        let digit_value = digit_char.to_digit(10)?;
        if (1..=9).contains(&digit_value) {
            Some(usize::try_from(digit_value).ok()?)
        } else {
            None
        }
    } else {
        None
    }
}

/// Transforms placeholder symbols to parameter names in the AST.
fn transform_placeholders(elements: Vec<Spanned<Ast>>) -> Vec<Spanned<Ast>> {
    elements
        .into_iter()
        .map(|elem| Spanned::new(transform_ast(elem.node), elem.span))
        .collect()
}

/// Recursively transforms placeholder symbols in an AST node.
fn transform_ast(ast: Ast) -> Ast {
    match ast {
        Ast::Symbol(name) => parse_placeholder(&name).map_or_else(
            || Ast::Symbol(name),
            |arg_num| {
                if arg_num == 0 {
                    // %& → rest
                    Ast::Symbol(String::from("rest"))
                } else {
                    // % or %N → pN (arg_num guaranteed 1-9 by parse_placeholder)
                    Ast::Symbol(make_param_name(arg_num))
                }
            },
        ),
        Ast::List(elems) => Ast::List(transform_spanned_vec(elems)),
        Ast::Vector(elems) => Ast::Vector(transform_spanned_vec(elems)),
        Ast::Set(elems) => Ast::Set(transform_spanned_vec(elems)),
        Ast::Map(elems) => Ast::Map(transform_spanned_vec(elems)),
        // WithMeta wraps nodes that may contain placeholders
        Ast::WithMeta { meta, value } => Ast::WithMeta {
            meta: Box::new(Spanned::new(transform_ast(meta.node), meta.span)),
            value: Box::new(Spanned::new(transform_ast(value.node), value.span)),
        },
        // Other types pass through unchanged
        Ast::Integer(_)
        | Ast::Float(_)
        | Ast::String(_)
        | Ast::Bool(_)
        | Ast::Nil
        | Ast::Keyword(_) => ast,
    }
}

/// Helper to transform a vector of spanned AST nodes.
fn transform_spanned_vec(elems: Vec<Spanned<Ast>>) -> Vec<Spanned<Ast>> {
    elems
        .into_iter()
        .map(|elem| Spanned::new(transform_ast(elem.node), elem.span))
        .collect()
}

/// Creates a parameter name "pN" for the given argument number (1-9).
///
/// Uses a lookup table to avoid fallible conversions.
fn make_param_name(arg_num: usize) -> String {
    // Argument numbers are guaranteed to be 1-9 by parse_placeholder
    const PARAM_NAMES: [&str; 10] = ["p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7", "p8", "p9"];

    PARAM_NAMES.get(arg_num).map_or_else(
        || {
            // Fallback for unexpected values (should never happen)
            let mut name = String::from("p");
            name.push_str(&format_usize(arg_num));
            name
        },
        |name| String::from(*name),
    )
}

/// Formats a usize as a decimal string without std.
///
/// Uses a lookup table for digits 0-9, which covers all valid argument numbers.
fn format_usize(num: usize) -> String {
    const DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

    DIGITS.get(num).map_or_else(
        || {
            // For numbers >= 10, use recursive string building
            // This path should never be hit for placeholder args (max 9)
            let mut result = format_usize(num / 10);
            if let Some(digit) = DIGITS.get(num % 10) {
                result.push_str(digit);
            }
            result
        },
        |digit| String::from(*digit),
    )
}

/// Builds a `(fn [params] (body...))` AST from the transformed body.
///
/// The body elements form a single list expression that is the function body.
/// For example, `#(+ % 1)` becomes `(fn [p1] (+ p1 1))`.
fn build_fn_ast(max_arg: usize, has_rest: bool, body: Vec<Spanned<Ast>>, span: Span) -> Ast {
    // Build parameter vector: [p1 p2 ... pN] or [p1 p2 ... pN & rest]
    let mut params = Vec::new();

    for i in 1..=max_arg {
        params.push(Spanned::new(Ast::Symbol(make_param_name(i)), span));
    }

    if has_rest {
        params.push(Spanned::new(Ast::Symbol(String::from("&")), span));
        params.push(Spanned::new(Ast::Symbol(String::from("rest")), span));
    }

    let params_vec = Ast::Vector(params);

    // Wrap body elements in a list to form the function body expression
    // #(+ % 1) becomes (fn [p1] (+ p1 1))
    let body_expr = Ast::List(body);

    // Build (fn [params] body-expr)
    let fn_elements = vec![
        Spanned::new(Ast::Symbol(String::from("fn")), span),
        Spanned::new(params_vec, span),
        Spanned::new(body_expr, span),
    ];

    Ast::List(fn_elements)
}

impl Parser<'_> {
    /// Parses a list `(...)`.
    pub(super) fn parse_list(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
        let (elements, span) =
            self.parse_collection(TokenKind::LeftParen, TokenKind::RightParen, '(', ')')?;
        Ok(Self::spanned_with_trivia(
            Ast::list(elements),
            span,
            trivia_start,
        ))
    }

    /// Parses a vector `[...]`.
    pub(super) fn parse_vector(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
        let (elements, span) =
            self.parse_collection(TokenKind::LeftBracket, TokenKind::RightBracket, '[', ']')?;
        Ok(Self::spanned_with_trivia(
            Ast::vector(elements),
            span,
            trivia_start,
        ))
    }

    /// Parses a map `{...}`.
    pub(super) fn parse_map(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
        let (elements, span) =
            self.parse_collection(TokenKind::LeftBrace, TokenKind::RightBrace, '{', '}')?;

        // Maps must have an even number of elements
        if elements.len() % 2 != 0 {
            return Err(Error::new(ErrorKind::OddMapEntries, self.location(span)));
        }

        // Check for duplicate keys (keys are at indices 0, 2, 4, ...)
        // Uses O(n²) comparison but n is typically small for literal maps
        let keys: Vec<&Spanned<Ast>> = elements.iter().step_by(2_usize).collect();
        for (i, key) in keys.iter().enumerate() {
            for prev in keys.iter().take(i) {
                if key.node == prev.node {
                    return Err(Error::new(
                        ErrorKind::DuplicateMapKey,
                        self.location(key.span),
                    ));
                }
            }
        }

        Ok(Self::spanned_with_trivia(
            Ast::map(elements),
            span,
            trivia_start,
        ))
    }

    /// Parses a set `#{...}`.
    pub(super) fn parse_set(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
        let (elements, span) =
            self.parse_collection(TokenKind::SetStart, TokenKind::RightBrace, '#', '}')?;

        // Check for duplicate AST nodes (O(n²) but n is typically small)
        for (i, elem) in elements.iter().enumerate() {
            for prev in elements.iter().take(i) {
                if elem.node == prev.node {
                    return Err(Error::new(
                        ErrorKind::DuplicateSetElement,
                        self.location(elem.span),
                    ));
                }
            }
        }

        Ok(Self::spanned_with_trivia(
            Ast::set(elements),
            span,
            trivia_start,
        ))
    }

    /// Parses an anonymous function `#(...)`.
    ///
    /// This expands `#(+ % 1)` to `(fn [p1] (+ p1 1))`.
    ///
    /// Placeholder rules:
    /// - `%` or `%1` → first argument
    /// - `%N` → Nth argument
    /// - `%&` → rest arguments
    ///
    /// Nested `#()` is not allowed (unlike Clojure).
    pub(super) fn parse_anon_fn(&mut self, trivia_start: usize) -> Result<Spanned<Ast>, Error> {
        // Check for nested #() - this is not allowed in Lonala
        if self.in_anon_fn {
            // Peek to get the location for the error
            let token = self.lexer.peek();
            let span = match token {
                Some(&Ok(ref tok)) => tok.span,
                Some(&Err(ref err)) => err.span(),
                None => Span::new(self.source.len(), self.source.len()),
            };
            return Err(Error::new(ErrorKind::NestedAnonFn, self.location(span)));
        }

        // Set the flag while parsing the body
        self.in_anon_fn = true;

        // Parse the body elements like a list
        let result = self.parse_collection(TokenKind::AnonFnStart, TokenKind::RightParen, '#', ')');

        // Reset the flag after parsing (whether successful or not)
        self.in_anon_fn = false;

        let (elements, span) = result?;

        // Analyze placeholders and transform to fn form
        let (max_arg, has_rest) = find_placeholders(&elements);
        let transformed = transform_placeholders(elements);
        let fn_ast = build_fn_ast(max_arg, has_rest, transformed, span);

        Ok(Self::spanned_with_trivia(fn_ast, span, trivia_start))
    }

    /// Helper to parse a collection with the given delimiters.
    pub(super) fn parse_collection(
        &mut self,
        open_kind: TokenKind,
        close_kind: TokenKind,
        open_char: char,
        close_char: char,
    ) -> Result<(Vec<Spanned<Ast>>, Span), Error> {
        // Consume opening delimiter
        let open_token = self.expect_token(open_kind)?;
        let start = open_token.span.start;
        let opener_location = self.location(open_token.span);

        let mut elements = Vec::new();

        loop {
            // Handle any discard tokens before checking for closing delimiter
            self.skip_discards()?;

            match self.lexer.peek() {
                None => {
                    return Err(Error::new(
                        ErrorKind::UnexpectedEof {
                            expected: close_kind.description(),
                        },
                        self.location_from(self.source.len(), self.source.len()),
                    ));
                }
                Some(&Err(ref err)) => return Err(err.clone()),
                Some(&Ok(ref token)) if token.kind == close_kind => {
                    // Consume closing delimiter
                    let close_token = self.advance()?;
                    let span = Span::new(start, close_token.span.end);
                    return Ok((elements, span));
                }
                Some(&Ok(ref token)) => {
                    // Check for mismatched delimiters
                    if matches!(
                        token.kind,
                        TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace
                    ) && token.kind != close_kind
                    {
                        let found_char = match token.kind {
                            TokenKind::RightParen => ')',
                            TokenKind::RightBracket => ']',
                            // RightBrace is the only remaining option
                            TokenKind::RightBrace | _ => '}',
                        };
                        // Copy span before calling self.location() to avoid borrow conflict
                        let error_span = token.span;
                        return Err(Error::new(
                            ErrorKind::UnmatchedDelimiter {
                                opener: open_char,
                                opener_location,
                                expected: close_char,
                                found: found_char,
                            },
                            self.location(error_span),
                        ));
                    }

                    // Parse element
                    elements.push(self.parse_expr()?);
                }
            }
        }
    }
}
