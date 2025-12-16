// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Parser for Lonala S-expressions.
//!
//! The parser transforms a stream of tokens into an Abstract Syntax Tree (AST).
//! It handles reader macros (quote, syntax-quote, unquote, unquote-splicing)
//! by expanding them to their canonical list forms.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use crate::ast::{Ast, Spanned};
use crate::error::{Error, Kind as ErrorKind, Span};
use crate::lexer::Lexer;
use crate::token::Kind as TokenKind;

/// Parser for Lonala S-expressions.
///
/// Transforms tokens from the lexer into an AST. Uses recursive descent
/// parsing to handle nested expressions and collections.
pub struct Parser<'src> {
    /// The underlying lexer.
    lexer: Lexer<'src>,
    /// The source string (for span extraction).
    source: &'src str,
}

impl<'src> Parser<'src> {
    /// Creates a new parser for the given source code.
    #[inline]
    #[must_use]
    pub const fn new(source: &'src str) -> Self {
        Self {
            lexer: Lexer::new(source),
            source,
        }
    }

    /// Parses all expressions from the source.
    ///
    /// Returns a vector of spanned AST nodes, one for each top-level expression.
    #[inline]
    #[must_use = "parsing result should be used"]
    pub fn parse(&mut self) -> Result<Vec<Spanned<Ast>>, Error> {
        let mut exprs = Vec::new();
        while self.lexer.peek().is_some() {
            exprs.push(self.parse_expr()?);
        }
        Ok(exprs)
    }

    /// Parses a single expression from the source.
    ///
    /// Returns an error if there are no expressions or if parsing fails.
    #[inline]
    #[must_use = "parsing result should be used"]
    pub fn parse_one(&mut self) -> Result<Spanned<Ast>, Error> {
        if self.lexer.peek().is_none() {
            return Err(Error::new(
                ErrorKind::UnexpectedEof {
                    expected: "expression",
                },
                Span::new(self.source.len(), self.source.len()),
            ));
        }
        self.parse_expr()
    }

    /// Parses a single expression.
    fn parse_expr(&mut self) -> Result<Spanned<Ast>, Error> {
        let token = match self.lexer.peek() {
            Some(&Ok(ref token)) => token.clone(),
            Some(&Err(ref err)) => return Err(err.clone()),
            None => {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof {
                        expected: "expression",
                    },
                    Span::new(self.source.len(), self.source.len()),
                ));
            }
        };

        match token.kind {
            // Delimiters - start collections
            TokenKind::LeftParen => self.parse_list(),
            TokenKind::LeftBracket => self.parse_vector(),
            TokenKind::LeftBrace => self.parse_map(),

            // Reader macros
            TokenKind::Quote => self.parse_reader_macro("quote"),
            TokenKind::SyntaxQuote => self.parse_reader_macro("syntax-quote"),
            TokenKind::Unquote => self.parse_reader_macro("unquote"),
            TokenKind::UnquoteSplice => self.parse_reader_macro("unquote-splicing"),

            // Atoms
            TokenKind::Integer
            | TokenKind::Float
            | TokenKind::String
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Nil
            | TokenKind::Symbol
            | TokenKind::Keyword => self.parse_atom(),

            // Unexpected closing delimiters
            TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace => {
                Err(Error::new(
                    ErrorKind::UnexpectedToken {
                        expected: "expression",
                        found: token.kind.description(),
                    },
                    token.span,
                ))
            }
        }
    }

    /// Parses a list `(...)`.
    fn parse_list(&mut self) -> Result<Spanned<Ast>, Error> {
        self.parse_collection(TokenKind::LeftParen, TokenKind::RightParen, '(', ')')
            .map(|(elements, span)| Spanned::new(Ast::list(elements), span))
    }

    /// Parses a vector `[...]`.
    fn parse_vector(&mut self) -> Result<Spanned<Ast>, Error> {
        self.parse_collection(TokenKind::LeftBracket, TokenKind::RightBracket, '[', ']')
            .map(|(elements, span)| Spanned::new(Ast::vector(elements), span))
    }

    /// Parses a map `{...}`.
    fn parse_map(&mut self) -> Result<Spanned<Ast>, Error> {
        let (elements, span) =
            self.parse_collection(TokenKind::LeftBrace, TokenKind::RightBrace, '{', '}')?;

        // Maps must have an even number of elements
        if !elements.len().is_multiple_of(2_usize) {
            return Err(Error::new(ErrorKind::OddMapEntries, span));
        }

        Ok(Spanned::new(Ast::map(elements), span))
    }

    /// Helper to parse a collection with the given delimiters.
    fn parse_collection(
        &mut self,
        open_kind: TokenKind,
        close_kind: TokenKind,
        open_char: char,
        close_char: char,
    ) -> Result<(Vec<Spanned<Ast>>, Span), Error> {
        // Consume opening delimiter
        let open_token = self.expect_token(open_kind)?;
        let start = open_token.span.start;

        let mut elements = Vec::new();

        loop {
            match self.lexer.peek() {
                None => {
                    return Err(Error::new(
                        ErrorKind::UnexpectedEof {
                            expected: close_kind.description(),
                        },
                        Span::new(self.source.len(), self.source.len()),
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
                        return Err(Error::new(
                            ErrorKind::UnmatchedDelimiter {
                                opener: open_char,
                                expected: close_char,
                                found: found_char,
                            },
                            token.span,
                        ));
                    }

                    // Parse element
                    elements.push(self.parse_expr()?);
                }
            }
        }
    }

    /// Parses a reader macro and expands it to its canonical form.
    ///
    /// For example, `'x` becomes `(quote x)`.
    fn parse_reader_macro(&mut self, symbol_name: &str) -> Result<Spanned<Ast>, Error> {
        // Consume the reader macro token
        let macro_token = self.advance()?;
        let start = macro_token.span.start;

        // Check that an expression follows
        match self.lexer.peek() {
            None => {
                return Err(Error::new(
                    ErrorKind::ReaderMacroMissingExpr,
                    macro_token.span,
                ));
            }
            Some(&Err(ref err)) => return Err(err.clone()),
            Some(&Ok(ref token)) => {
                // Closing delimiters are not valid here
                if matches!(
                    token.kind,
                    TokenKind::RightParen | TokenKind::RightBracket | TokenKind::RightBrace
                ) {
                    return Err(Error::new(
                        ErrorKind::ReaderMacroMissingExpr,
                        macro_token.span,
                    ));
                }
            }
        }

        // Parse the inner expression
        let inner = self.parse_expr()?;
        let end = inner.span.end;

        // Create the symbol for the reader macro
        let symbol_span = macro_token.span;
        let symbol = Spanned::new(Ast::symbol(symbol_name), symbol_span);

        // Build the list form: (symbol inner)
        let elements = alloc::vec![symbol, inner];
        let span = Span::new(start, end);

        Ok(Spanned::new(Ast::list(elements), span))
    }

    /// Parses an atom (literal, symbol, or keyword).
    fn parse_atom(&mut self) -> Result<Spanned<Ast>, Error> {
        let token = self.advance()?;

        let ast = match token.kind {
            TokenKind::Integer => {
                let value = Self::parse_integer(token.lexeme)?;
                Ast::integer(value)
            }
            TokenKind::Float => {
                let value = Self::parse_float(token.lexeme)?;
                Ast::float(value)
            }
            TokenKind::String => {
                let value = Self::process_string(token.lexeme, token.span)?;
                Ast::string(value)
            }
            TokenKind::True => Ast::bool(true),
            TokenKind::False => Ast::bool(false),
            TokenKind::Nil => Ast::nil(),
            TokenKind::Symbol => Ast::symbol(token.lexeme),
            TokenKind::Keyword => {
                // Remove the leading colon from the keyword
                let name = token.lexeme.get(1_usize..).unwrap_or("");
                Ast::keyword(name)
            }
            // parse_atom is only called for atom token kinds from parse_expr
            TokenKind::LeftParen
            | TokenKind::LeftBracket
            | TokenKind::LeftBrace
            | TokenKind::RightParen
            | TokenKind::RightBracket
            | TokenKind::RightBrace
            | TokenKind::Quote
            | TokenKind::SyntaxQuote
            | TokenKind::Unquote
            | TokenKind::UnquoteSplice => {
                return Err(Error::new(
                    ErrorKind::UnexpectedToken {
                        expected: "atom",
                        found: token.kind.description(),
                    },
                    token.span,
                ));
            }
        };

        Ok(Spanned::new(ast, token.span))
    }

    /// Parses an integer literal from its lexeme.
    fn parse_integer(lexeme: &str) -> Result<i64, Error> {
        let make_err = || Error::new(ErrorKind::InvalidNumber, Span::new(0_usize, lexeme.len()));

        // Handle different bases
        if lexeme.len() >= 2_usize {
            let prefix = lexeme.get(..2_usize).unwrap_or("");
            let (radix, skip) = match prefix.to_ascii_lowercase().as_str() {
                "0x" => (16_u32, 2_usize),
                "0b" => (2_u32, 2_usize),
                "0o" => (8_u32, 2_usize),
                _ => (10_u32, 0_usize),
            };

            if skip > 0_usize {
                let digits = lexeme.get(skip..).unwrap_or("");
                return i64::from_str_radix(digits, radix).map_err(|_err| make_err());
            }
        }

        // Decimal
        lexeme.parse::<i64>().map_err(|_err| make_err())
    }

    /// Parses a float literal from its lexeme.
    fn parse_float(lexeme: &str) -> Result<f64, Error> {
        // Handle special float literals
        match lexeme {
            "##NaN" => return Ok(f64::NAN),
            "##Inf" => return Ok(f64::INFINITY),
            "##-Inf" => return Ok(f64::NEG_INFINITY),
            _ => {}
        }

        lexeme
            .parse::<f64>()
            .map_err(|_err| Error::new(ErrorKind::InvalidNumber, Span::new(0_usize, lexeme.len())))
    }

    /// Processes escape sequences in a string literal.
    ///
    /// The lexeme includes the surrounding quotes. This function returns
    /// the string content with escapes processed. The `token_span` is used
    /// to calculate accurate error positions within the source.
    fn process_string(lexeme: &str, token_span: Span) -> Result<String, Error> {
        // Remove surrounding quotes
        let content = lexeme
            .get(1_usize..lexeme.len().saturating_sub(1_usize))
            .unwrap_or("");

        let mut result = String::new();
        let mut chars = content.char_indices();

        while let Some((byte_offset, ch)) = chars.next() {
            if ch == '\\' {
                // Calculate source position: token start + opening quote + offset
                let escape_start = token_span
                    .start
                    .saturating_add(1_usize)
                    .saturating_add(byte_offset);

                match chars.next() {
                    Some((_, '\\')) => result.push('\\'),
                    Some((_, '"')) => result.push('"'),
                    Some((_, 'n')) => result.push('\n'),
                    Some((_, 't')) => result.push('\t'),
                    Some((_, 'r')) => result.push('\r'),
                    Some((_, '0')) => result.push('\0'),
                    Some((u_offset, 'u')) => {
                        // Unicode escape: \uXXXX
                        let unicode_start = token_span
                            .start
                            .saturating_add(1_usize)
                            .saturating_add(u_offset);
                        let mut hex = String::new();
                        let mut hex_end = unicode_start.saturating_add(1_usize);
                        for _ in 0_u8..4_u8 {
                            if let Some((offset, digit)) = chars.next() {
                                hex.push(digit);
                                hex_end = token_span
                                    .start
                                    .saturating_add(1_usize)
                                    .saturating_add(offset)
                                    .saturating_add(digit.len_utf8());
                            }
                        }
                        let error_span = Span::new(escape_start, hex_end);
                        let code_point = u32::from_str_radix(&hex, 16_u32).map_err(|_err| {
                            Error::new(ErrorKind::InvalidUnicodeEscape, error_span)
                        })?;
                        let ch = char::from_u32(code_point).ok_or_else(|| {
                            Error::new(ErrorKind::InvalidUnicodeEscape, error_span)
                        })?;
                        result.push(ch);
                    }
                    Some((end_offset, other)) => {
                        // This shouldn't happen if the lexer validated escapes correctly
                        let escape_end = token_span
                            .start
                            .saturating_add(1_usize)
                            .saturating_add(end_offset)
                            .saturating_add(other.len_utf8());
                        return Err(Error::new(
                            ErrorKind::InvalidEscapeSequence(other),
                            Span::new(escape_start, escape_end),
                        ));
                    }
                    None => {
                        // This shouldn't happen if the lexer validated the string
                        return Err(Error::new(
                            ErrorKind::UnterminatedString,
                            Span::new(escape_start, token_span.end),
                        ));
                    }
                }
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }

    /// Consumes and returns the next token, returning an error if at EOF.
    fn advance(&mut self) -> Result<crate::token::Token<'src>, Error> {
        match self.lexer.next() {
            Some(Ok(token)) => Ok(token),
            Some(Err(err)) => Err(err),
            None => Err(Error::new(
                ErrorKind::UnexpectedEof { expected: "token" },
                Span::new(self.source.len(), self.source.len()),
            )),
        }
    }

    /// Consumes the next token, expecting it to be of the given kind.
    fn expect_token(&mut self, expected: TokenKind) -> Result<crate::token::Token<'src>, Error> {
        let token = self.advance()?;
        if token.kind == expected {
            Ok(token)
        } else {
            Err(Error::new(
                ErrorKind::UnexpectedToken {
                    expected: expected.description(),
                    found: token.kind.description(),
                },
                token.span,
            ))
        }
    }
}

/// Parses all expressions from the source string.
///
/// This is a convenience function that creates a parser and parses all
/// top-level expressions.
#[inline]
#[must_use = "parsing result should be used"]
pub fn parse(source: &str) -> Result<Vec<Spanned<Ast>>, Error> {
    Parser::new(source).parse()
}

/// Parses a single expression from the source string.
///
/// This is a convenience function that creates a parser and parses one
/// expression. Returns an error if there are no expressions.
#[inline]
#[must_use = "parsing result should be used"]
pub fn parse_one(source: &str) -> Result<Spanned<Ast>, Error> {
    Parser::new(source).parse_one()
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::format;
    use alloc::string::ToString;
    use alloc::vec;

    use super::*;

    // Helper to parse and return the AST node, ignoring spans
    fn parse_ast(source: &str) -> Ast {
        parse_one(source).expect("parse should succeed").node
    }

    // Helper to parse and return all AST nodes
    fn parse_asts(source: &str) -> Vec<Ast> {
        parse(source)
            .expect("parse should succeed")
            .into_iter()
            .map(|s| s.node)
            .collect()
    }

    // ==================== Atoms: Integers ====================

    #[test]
    fn parse_integer_decimal() {
        assert_eq!(parse_ast("42"), Ast::Integer(42_i64));
        assert_eq!(parse_ast("0"), Ast::Integer(0_i64));
        assert_eq!(parse_ast("123456789"), Ast::Integer(123456789_i64));
    }

    #[test]
    fn parse_integer_negative() {
        assert_eq!(parse_ast("-42"), Ast::Integer(-42_i64));
        assert_eq!(parse_ast("-1"), Ast::Integer(-1_i64));
    }

    #[test]
    fn parse_integer_hex() {
        assert_eq!(parse_ast("0xFF"), Ast::Integer(255_i64));
        assert_eq!(parse_ast("0x1a2B"), Ast::Integer(0x1a2B_i64));
        assert_eq!(parse_ast("0X10"), Ast::Integer(16_i64));
    }

    #[test]
    fn parse_integer_binary() {
        assert_eq!(parse_ast("0b1010"), Ast::Integer(10_i64));
        assert_eq!(parse_ast("0B11"), Ast::Integer(3_i64));
    }

    #[test]
    fn parse_integer_octal() {
        assert_eq!(parse_ast("0o755"), Ast::Integer(493_i64));
        assert_eq!(parse_ast("0O17"), Ast::Integer(15_i64));
    }

    // ==================== Atoms: Floats ====================

    #[test]
    fn parse_float_simple() {
        assert_eq!(parse_ast("3.14"), Ast::Float(3.14_f64));
        assert_eq!(parse_ast("0.5"), Ast::Float(0.5_f64));
    }

    #[test]
    fn parse_float_negative() {
        assert_eq!(parse_ast("-3.14"), Ast::Float(-3.14_f64));
    }

    #[test]
    fn parse_float_scientific() {
        assert_eq!(parse_ast("1e10"), Ast::Float(1e10_f64));
        assert_eq!(parse_ast("2.5e-3"), Ast::Float(2.5e-3_f64));
        assert_eq!(parse_ast("1E+5"), Ast::Float(1e5_f64));
    }

    #[test]
    fn parse_float_nan() {
        let ast = parse_ast("##NaN");
        if let Ast::Float(f) = ast {
            assert!(f.is_nan());
        } else {
            panic!("expected Float");
        }
    }

    #[test]
    fn parse_float_infinity() {
        assert_eq!(parse_ast("##Inf"), Ast::Float(f64::INFINITY));
        assert_eq!(parse_ast("##-Inf"), Ast::Float(f64::NEG_INFINITY));
    }

    // ==================== Atoms: Strings ====================

    #[test]
    fn parse_string_empty() {
        assert_eq!(parse_ast(r#""""#), Ast::String(String::new()));
    }

    #[test]
    fn parse_string_simple() {
        assert_eq!(parse_ast(r#""hello""#), Ast::String("hello".to_string()));
    }

    #[test]
    fn parse_string_with_escapes() {
        assert_eq!(
            parse_ast(r#""hello\nworld""#),
            Ast::String("hello\nworld".to_string())
        );
        assert_eq!(
            parse_ast(r#""tab\there""#),
            Ast::String("tab\there".to_string())
        );
        assert_eq!(
            parse_ast(r#""back\\slash""#),
            Ast::String("back\\slash".to_string())
        );
        assert_eq!(
            parse_ast(r#""say \"hi\"""#),
            Ast::String("say \"hi\"".to_string())
        );
        assert_eq!(
            parse_ast(r#""return\r""#),
            Ast::String("return\r".to_string())
        );
        assert_eq!(parse_ast(r#""null\0""#), Ast::String("null\0".to_string()));
    }

    #[test]
    fn parse_string_unicode_escape() {
        assert_eq!(parse_ast(r#""\u0041""#), Ast::String("A".to_string()));
        assert_eq!(
            parse_ast(r#""\u03B1""#),
            Ast::String("\u{03B1}".to_string())
        ); // Greek alpha
    }

    // ==================== Atoms: Booleans and Nil ====================

    #[test]
    fn parse_boolean_true() {
        assert_eq!(parse_ast("true"), Ast::Bool(true));
    }

    #[test]
    fn parse_boolean_false() {
        assert_eq!(parse_ast("false"), Ast::Bool(false));
    }

    #[test]
    fn parse_nil() {
        assert_eq!(parse_ast("nil"), Ast::Nil);
    }

    // ==================== Atoms: Symbols ====================

    #[test]
    fn parse_symbol_simple() {
        assert_eq!(parse_ast("foo"), Ast::Symbol("foo".to_string()));
        assert_eq!(parse_ast("bar"), Ast::Symbol("bar".to_string()));
    }

    #[test]
    fn parse_symbol_operators() {
        assert_eq!(parse_ast("+"), Ast::Symbol("+".to_string()));
        assert_eq!(parse_ast("-"), Ast::Symbol("-".to_string()));
        assert_eq!(parse_ast("*"), Ast::Symbol("*".to_string()));
        assert_eq!(parse_ast("/"), Ast::Symbol("/".to_string()));
        assert_eq!(parse_ast("<="), Ast::Symbol("<=".to_string()));
        assert_eq!(parse_ast(">="), Ast::Symbol(">=".to_string()));
    }

    #[test]
    fn parse_symbol_with_special_chars() {
        assert_eq!(parse_ast("update!"), Ast::Symbol("update!".to_string()));
        assert_eq!(parse_ast("empty?"), Ast::Symbol("empty?".to_string()));
        assert_eq!(parse_ast("->arrow"), Ast::Symbol("->arrow".to_string()));
        assert_eq!(parse_ast("*special*"), Ast::Symbol("*special*".to_string()));
    }

    #[test]
    fn parse_symbol_namespaced() {
        assert_eq!(parse_ast("ns/name"), Ast::Symbol("ns/name".to_string()));
        assert_eq!(
            parse_ast("foo.bar/baz"),
            Ast::Symbol("foo.bar/baz".to_string())
        );
    }

    // ==================== Atoms: Keywords ====================

    #[test]
    fn parse_keyword_simple() {
        assert_eq!(parse_ast(":foo"), Ast::Keyword("foo".to_string()));
        assert_eq!(parse_ast(":bar"), Ast::Keyword("bar".to_string()));
    }

    #[test]
    fn parse_keyword_namespaced() {
        assert_eq!(parse_ast(":ns/name"), Ast::Keyword("ns/name".to_string()));
    }

    #[test]
    fn parse_keyword_kebab_case() {
        assert_eq!(
            parse_ast(":kebab-case"),
            Ast::Keyword("kebab-case".to_string())
        );
    }

    // ==================== Collections: Lists ====================

    #[test]
    fn parse_empty_list() {
        assert_eq!(parse_ast("()"), Ast::List(vec![]));
    }

    #[test]
    fn parse_list_with_elements() {
        let ast = parse_ast("(+ 1 2)");
        match ast {
            Ast::List(elements) => {
                assert_eq!(elements.len(), 3_usize);
                assert_eq!(
                    elements.first().map(|s| &s.node),
                    Some(&Ast::Symbol("+".to_string()))
                );
                assert_eq!(
                    elements.get(1_usize).map(|s| &s.node),
                    Some(&Ast::Integer(1_i64))
                );
                assert_eq!(
                    elements.get(2_usize).map(|s| &s.node),
                    Some(&Ast::Integer(2_i64))
                );
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn parse_nested_lists() {
        let ast = parse_ast("((a) (b))");
        match ast {
            Ast::List(outer) => {
                assert_eq!(outer.len(), 2_usize);
                match &outer.first().map(|s| &s.node) {
                    Some(Ast::List(inner)) => {
                        assert_eq!(inner.len(), 1_usize);
                    }
                    _ => panic!("expected inner List"),
                }
            }
            _ => panic!("expected List"),
        }
    }

    // ==================== Collections: Vectors ====================

    #[test]
    fn parse_empty_vector() {
        assert_eq!(parse_ast("[]"), Ast::Vector(vec![]));
    }

    #[test]
    fn parse_vector_with_elements() {
        let ast = parse_ast("[1 2 3]");
        match ast {
            Ast::Vector(elements) => {
                assert_eq!(elements.len(), 3_usize);
                assert_eq!(
                    elements.first().map(|s| &s.node),
                    Some(&Ast::Integer(1_i64))
                );
                assert_eq!(
                    elements.get(1_usize).map(|s| &s.node),
                    Some(&Ast::Integer(2_i64))
                );
                assert_eq!(
                    elements.get(2_usize).map(|s| &s.node),
                    Some(&Ast::Integer(3_i64))
                );
            }
            _ => panic!("expected Vector"),
        }
    }

    // ==================== Collections: Maps ====================

    #[test]
    fn parse_empty_map() {
        assert_eq!(parse_ast("{}"), Ast::Map(vec![]));
    }

    #[test]
    fn parse_map_with_entries() {
        let ast = parse_ast("{:a 1 :b 2}");
        match ast {
            Ast::Map(elements) => {
                assert_eq!(elements.len(), 4_usize);
                assert_eq!(
                    elements.first().map(|s| &s.node),
                    Some(&Ast::Keyword("a".to_string()))
                );
                assert_eq!(
                    elements.get(1_usize).map(|s| &s.node),
                    Some(&Ast::Integer(1_i64))
                );
                assert_eq!(
                    elements.get(2_usize).map(|s| &s.node),
                    Some(&Ast::Keyword("b".to_string()))
                );
                assert_eq!(
                    elements.get(3_usize).map(|s| &s.node),
                    Some(&Ast::Integer(2_i64))
                );
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn parse_nested_collections() {
        let ast = parse_ast("{:list (1 2) :vec [3 4]}");
        match ast {
            Ast::Map(elements) => {
                assert_eq!(elements.len(), 4_usize);
                assert!(matches!(
                    elements.get(1_usize).map(|s| &s.node),
                    Some(Ast::List(_))
                ));
                assert!(matches!(
                    elements.get(3_usize).map(|s| &s.node),
                    Some(Ast::Vector(_))
                ));
            }
            _ => panic!("expected Map"),
        }
    }

    // ==================== Reader Macros ====================

    #[test]
    fn parse_quote() {
        let ast = parse_ast("'x");
        match ast {
            Ast::List(elements) => {
                assert_eq!(elements.len(), 2_usize);
                assert_eq!(
                    elements.first().map(|s| &s.node),
                    Some(&Ast::Symbol("quote".to_string()))
                );
                assert_eq!(
                    elements.get(1_usize).map(|s| &s.node),
                    Some(&Ast::Symbol("x".to_string()))
                );
            }
            _ => panic!("expected List for quote"),
        }
    }

    #[test]
    fn parse_quote_list() {
        let ast = parse_ast("'(1 2 3)");
        match ast {
            Ast::List(elements) => {
                assert_eq!(elements.len(), 2_usize);
                assert_eq!(
                    elements.first().map(|s| &s.node),
                    Some(&Ast::Symbol("quote".to_string()))
                );
                assert!(matches!(
                    elements.get(1_usize).map(|s| &s.node),
                    Some(Ast::List(_))
                ));
            }
            _ => panic!("expected List for quote"),
        }
    }

    #[test]
    fn parse_syntax_quote() {
        let ast = parse_ast("`x");
        match ast {
            Ast::List(elements) => {
                assert_eq!(elements.len(), 2_usize);
                assert_eq!(
                    elements.first().map(|s| &s.node),
                    Some(&Ast::Symbol("syntax-quote".to_string()))
                );
            }
            _ => panic!("expected List for syntax-quote"),
        }
    }

    #[test]
    fn parse_unquote() {
        let ast = parse_ast("~x");
        match ast {
            Ast::List(elements) => {
                assert_eq!(elements.len(), 2_usize);
                assert_eq!(
                    elements.first().map(|s| &s.node),
                    Some(&Ast::Symbol("unquote".to_string()))
                );
            }
            _ => panic!("expected List for unquote"),
        }
    }

    #[test]
    fn parse_unquote_splice() {
        let ast = parse_ast("~@xs");
        match ast {
            Ast::List(elements) => {
                assert_eq!(elements.len(), 2_usize);
                assert_eq!(
                    elements.first().map(|s| &s.node),
                    Some(&Ast::Symbol("unquote-splicing".to_string()))
                );
            }
            _ => panic!("expected List for unquote-splicing"),
        }
    }

    #[test]
    fn parse_nested_reader_macros() {
        let ast = parse_ast("''x");
        // Should be (quote (quote x))
        match ast {
            Ast::List(outer) => {
                assert_eq!(outer.len(), 2_usize);
                assert_eq!(
                    outer.first().map(|s| &s.node),
                    Some(&Ast::Symbol("quote".to_string()))
                );
                match outer.get(1_usize).map(|s| &s.node) {
                    Some(Ast::List(inner)) => {
                        assert_eq!(inner.len(), 2_usize);
                        assert_eq!(
                            inner.first().map(|s| &s.node),
                            Some(&Ast::Symbol("quote".to_string()))
                        );
                    }
                    _ => panic!("expected inner List"),
                }
            }
            _ => panic!("expected List"),
        }
    }

    // ==================== Multiple Expressions ====================

    #[test]
    fn parse_multiple_expressions() {
        let asts = parse_asts("1 2 3");
        assert_eq!(asts.len(), 3_usize);
        assert_eq!(asts.first(), Some(&Ast::Integer(1_i64)));
        assert_eq!(asts.get(1_usize), Some(&Ast::Integer(2_i64)));
        assert_eq!(asts.get(2_usize), Some(&Ast::Integer(3_i64)));
    }

    #[test]
    fn parse_empty_source() {
        let asts = parse_asts("");
        assert!(asts.is_empty());
    }

    #[test]
    fn parse_whitespace_only() {
        let asts = parse_asts("   \n\t  ");
        assert!(asts.is_empty());
    }

    // ==================== Span Tracking ====================

    #[test]
    fn span_single_token() {
        let spanned = parse_one("foo").expect("parse should succeed");
        assert_eq!(spanned.span, Span::new(0_usize, 3_usize));
    }

    #[test]
    fn span_collection() {
        let spanned = parse_one("(+ 1 2)").expect("parse should succeed");
        assert_eq!(spanned.span, Span::new(0_usize, 7_usize));
    }

    #[test]
    fn span_nested() {
        let spanned = parse_one("((a))").expect("parse should succeed");
        assert_eq!(spanned.span, Span::new(0_usize, 5_usize));
    }

    #[test]
    fn span_reader_macro() {
        let spanned = parse_one("'x").expect("parse should succeed");
        // Spans from quote (0) to end of x (2)
        assert_eq!(spanned.span, Span::new(0_usize, 2_usize));
    }

    // ==================== Error Cases ====================

    #[test]
    fn error_unexpected_eof_in_list() {
        let result = parse_one("(+ 1");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err.kind, ErrorKind::UnexpectedEof { .. }));
    }

    #[test]
    fn error_unexpected_eof_in_vector() {
        let result = parse_one("[1 2");
        assert!(result.is_err());
    }

    #[test]
    fn error_unexpected_eof_in_map() {
        let result = parse_one("{:a 1");
        assert!(result.is_err());
    }

    #[test]
    fn error_mismatched_delimiter() {
        let result = parse_one("(]");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err.kind, ErrorKind::UnmatchedDelimiter { .. }));
    }

    #[test]
    fn error_odd_map_entries() {
        let result = parse_one("{:a 1 :b}");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, ErrorKind::OddMapEntries);
    }

    #[test]
    fn error_reader_macro_at_eof() {
        let result = parse_one("'");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, ErrorKind::ReaderMacroMissingExpr);
    }

    #[test]
    fn error_reader_macro_before_closer() {
        let result = parse_one("(')");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, ErrorKind::ReaderMacroMissingExpr);
    }

    #[test]
    fn error_unexpected_closing_delimiter() {
        let result = parse_one(")");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err.kind, ErrorKind::UnexpectedToken { .. }));
    }

    #[test]
    fn error_parse_one_empty() {
        let result = parse_one("");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err.kind, ErrorKind::UnexpectedEof { .. }));
    }

    // ==================== Complex Expressions ====================

    #[test]
    fn parse_function_definition() {
        let ast = parse_ast("(defn foo [x] x)");
        match ast {
            Ast::List(elements) => {
                assert_eq!(elements.len(), 4_usize);
                assert_eq!(
                    elements.first().map(|s| &s.node),
                    Some(&Ast::Symbol("defn".to_string()))
                );
                assert_eq!(
                    elements.get(1_usize).map(|s| &s.node),
                    Some(&Ast::Symbol("foo".to_string()))
                );
                assert!(matches!(
                    elements.get(2_usize).map(|s| &s.node),
                    Some(Ast::Vector(_))
                ));
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn parse_let_binding() {
        let ast = parse_ast("(let [x 1] x)");
        match ast {
            Ast::List(elements) => {
                assert_eq!(elements.len(), 3_usize);
            }
            _ => panic!("expected List"),
        }
    }

    // ==================== Display Round-trip ====================

    #[test]
    fn display_roundtrip_simple() {
        let ast = parse_ast("42");
        assert_eq!(format!("{ast}"), "42");
    }

    #[test]
    fn display_roundtrip_list() {
        let ast = parse_ast("(+ 1 2)");
        assert_eq!(format!("{ast}"), "(+ 1 2)");
    }

    #[test]
    fn display_roundtrip_vector() {
        let ast = parse_ast("[1 2 3]");
        assert_eq!(format!("{ast}"), "[1 2 3]");
    }

    #[test]
    fn display_roundtrip_map() {
        let ast = parse_ast("{:a 1}");
        assert_eq!(format!("{ast}"), "{:a 1}");
    }
}
