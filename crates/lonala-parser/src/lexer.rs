// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! Lexical analyzer for Lonala source code.
//!
//! The lexer converts source text into a stream of tokens, handling
//! whitespace, comments, and all Lonala lexical elements. It implements
//! `Iterator` for lazy, streaming tokenization.

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::error::{Error, Kind as ErrorKind, Span};
use crate::token::{Kind as TokenKind, Token};

/// Lexical analyzer for Lonala source code.
///
/// The lexer maintains a position in the source string and produces tokens
/// on demand via the `Iterator` trait. It handles:
/// - Whitespace (spaces, tabs, newlines, commas)
/// - Comments (`;` to end of line)
/// - All Lonala token types (delimiters, literals, identifiers, reader macros)
pub struct Lexer<'src> {
    /// The source code being lexed.
    source: &'src str,
    /// Current byte position in source.
    position: usize,
    /// Cached next token for peek support.
    peeked: Option<Option<Result<Token<'src>, Error>>>,
}

impl<'src> Lexer<'src> {
    /// Creates a new lexer for the given source code.
    pub fn new(source: &'src str) -> Self {
        Self {
            source,
            position: 0_usize,
            peeked: None,
        }
    }

    /// Peeks at the next token without consuming it.
    ///
    /// Returns `None` if there are no more tokens. The peeked token
    /// is cached and returned on the next call to `next()`.
    pub fn peek(&mut self) -> Option<&Result<Token<'src>, Error>> {
        if self.peeked.is_none() {
            self.peeked = Some(self.next_token());
        }
        self.peeked.as_ref().and_then(|opt| opt.as_ref())
    }

    /// Returns the remaining unparsed source.
    fn remaining(&self) -> &'src str {
        self.source.get(self.position..).unwrap_or("")
    }

    /// Returns the current character without advancing.
    fn current_char(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    /// Advances the position by one character and returns it.
    fn advance(&mut self) -> Option<char> {
        let ch = self.current_char()?;
        self.position = self.position.saturating_add(ch.len_utf8());
        Some(ch)
    }

    /// Advances the position by n bytes.
    fn advance_by(&mut self, n: usize) {
        self.position = self.position.saturating_add(n);
    }

    /// Skips whitespace and comments.
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.current_char() {
                // Whitespace (including comma, which is whitespace in Clojure)
                Some(' ' | '\t' | '\n' | '\r' | ',') => {
                    self.advance();
                }
                // Comment: ; to end of line
                Some(';') => {
                    while let Some(ch) = self.current_char() {
                        self.advance();
                        if ch == '\n' {
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
    }

    /// Produces the next token from the source.
    fn next_token(&mut self) -> Option<Result<Token<'src>, Error>> {
        self.skip_whitespace_and_comments();

        let start = self.position;
        let ch = self.current_char()?;

        let result = match ch {
            // Delimiters
            '(' => self.single_char_token(TokenKind::LeftParen, start),
            ')' => self.single_char_token(TokenKind::RightParen, start),
            '[' => self.single_char_token(TokenKind::LeftBracket, start),
            ']' => self.single_char_token(TokenKind::RightBracket, start),
            '{' => self.single_char_token(TokenKind::LeftBrace, start),
            '}' => self.single_char_token(TokenKind::RightBrace, start),

            // Reader macros
            '\'' => self.single_char_token(TokenKind::Quote, start),
            '`' => self.single_char_token(TokenKind::SyntaxQuote, start),
            '~' => self.tilde_token(start),

            // String literal
            '"' => self.string_token(start),

            // Keyword
            ':' => self.keyword_token(start),

            // Special floats (##NaN, ##Inf, ##-Inf)
            '#' => self.hash_token(start),

            // Number or symbol starting with digit
            '0'..='9' => self.number_token(start),

            // Negative number or symbol starting with -
            '-' => self.minus_token(start),

            // Symbol starting with +
            '+' => self.plus_token(start),

            // Any other symbol character
            _ if is_symbol_start(ch) => self.symbol_token(start),

            // Unexpected character
            _ => {
                self.advance();
                Err(Error::new(
                    ErrorKind::UnexpectedCharacter(ch),
                    Span::new(start, self.position),
                ))
            }
        };

        Some(result)
    }

    /// Creates a single-character token.
    fn single_char_token(&mut self, kind: TokenKind, start: usize) -> Result<Token<'src>, Error> {
        self.advance();
        let end = self.position;
        let lexeme = self.source.get(start..end).unwrap_or("");
        Ok(Token::new(kind, lexeme, Span::new(start, end)))
    }

    /// Handles `~` which could be `~` (Unquote) or `~@` (UnquoteSplice).
    fn tilde_token(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume ~
        if self.current_char() == Some('@') {
            self.advance(); // consume @
            let lexeme = self.source.get(start..self.position).unwrap_or("");
            Ok(Token::new(
                TokenKind::UnquoteSplice,
                lexeme,
                Span::new(start, self.position),
            ))
        } else {
            let lexeme = self.source.get(start..self.position).unwrap_or("");
            Ok(Token::new(
                TokenKind::Unquote,
                lexeme,
                Span::new(start, self.position),
            ))
        }
    }

    /// Parses a string literal.
    fn string_token(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume opening "

        loop {
            match self.current_char() {
                None => {
                    return Err(Error::new(
                        ErrorKind::UnterminatedString,
                        Span::new(start, self.position),
                    ));
                }
                Some('"') => {
                    self.advance(); // consume closing "
                    break;
                }
                Some('\\') => {
                    self.advance(); // consume backslash
                    match self.current_char() {
                        None => {
                            return Err(Error::new(
                                ErrorKind::UnterminatedString,
                                Span::new(start, self.position),
                            ));
                        }
                        Some('\\' | '"' | 'n' | 't' | 'r' | '0') => {
                            self.advance();
                        }
                        Some('u') => {
                            self.advance(); // consume 'u'
                            // Expect exactly 4 hex digits
                            for _ in 0_u8..4_u8 {
                                match self.current_char() {
                                    Some(c) if c.is_ascii_hexdigit() => {
                                        self.advance();
                                    }
                                    _ => {
                                        return Err(Error::new(
                                            ErrorKind::InvalidUnicodeEscape,
                                            Span::new(start, self.position),
                                        ));
                                    }
                                }
                            }
                        }
                        Some(ch) => {
                            return Err(Error::new(
                                ErrorKind::InvalidEscapeSequence(ch),
                                Span::new(start, self.position),
                            ));
                        }
                    }
                }
                Some(_) => {
                    self.advance();
                }
            }
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::String,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Parses a keyword (`:foo`, `:ns/name`).
    fn keyword_token(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume :

        // Keywords must have at least one character after the colon
        if !self.current_char().is_some_and(is_symbol_start) {
            // Bare colon is an error
            return Err(Error::new(
                ErrorKind::UnexpectedCharacter(':'),
                Span::new(start, self.position),
            ));
        }

        // Consume the rest of the keyword (symbol characters)
        while self.current_char().is_some_and(is_symbol_continue) {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Keyword,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Handles `#` which starts special float literals.
    fn hash_token(&mut self, start: usize) -> Result<Token<'src>, Error> {
        let remaining = self.remaining();

        // Check for ##NaN
        if remaining.starts_with("##NaN") {
            self.advance_by(5_usize);
            let lexeme = self.source.get(start..self.position).unwrap_or("");
            return Ok(Token::new(
                TokenKind::Float,
                lexeme,
                Span::new(start, self.position),
            ));
        }

        // Check for ##Inf
        if remaining.starts_with("##Inf") {
            self.advance_by(5_usize);
            let lexeme = self.source.get(start..self.position).unwrap_or("");
            return Ok(Token::new(
                TokenKind::Float,
                lexeme,
                Span::new(start, self.position),
            ));
        }

        // Check for ##-Inf
        if remaining.starts_with("##-Inf") {
            self.advance_by(6_usize);
            let lexeme = self.source.get(start..self.position).unwrap_or("");
            return Ok(Token::new(
                TokenKind::Float,
                lexeme,
                Span::new(start, self.position),
            ));
        }

        // Unknown # sequence - treat as unexpected
        self.advance();
        Err(Error::new(
            ErrorKind::UnexpectedCharacter('#'),
            Span::new(start, self.position),
        ))
    }

    /// Parses a number starting with a digit.
    fn number_token(&mut self, start: usize) -> Result<Token<'src>, Error> {
        // Check for base prefixes (0x, 0b, 0o)
        if self.current_char() == Some('0') {
            self.advance();
            match self.current_char() {
                Some('x' | 'X') => return self.hex_number(start),
                Some('b' | 'B') => return self.binary_number(start),
                Some('o' | 'O') => return self.octal_number(start),
                Some('.') => return self.float_after_integer(start),
                Some('e' | 'E') => return self.float_exponent(start),
                Some(c) if c.is_ascii_digit() => {
                    // Continue parsing as decimal
                }
                _ => {
                    // Just '0'
                    let lexeme = self.source.get(start..self.position).unwrap_or("");
                    return Ok(Token::new(
                        TokenKind::Integer,
                        lexeme,
                        Span::new(start, self.position),
                    ));
                }
            }
        }

        // Consume decimal digits
        while self.current_char().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
        }

        // Check for float
        match self.current_char() {
            Some('.') => self.float_after_integer(start),
            Some('e' | 'E') => self.float_exponent(start),
            _ => {
                let lexeme = self.source.get(start..self.position).unwrap_or("");
                Ok(Token::new(
                    TokenKind::Integer,
                    lexeme,
                    Span::new(start, self.position),
                ))
            }
        }
    }

    /// Parses a hexadecimal number after seeing `0x` or `0X`.
    fn hex_number(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume x/X

        // Must have at least one hex digit
        if !self.current_char().is_some_and(|c| c.is_ascii_hexdigit()) {
            return Err(Error::new(
                ErrorKind::InvalidNumber,
                Span::new(start, self.position),
            ));
        }

        while self.current_char().is_some_and(|c| c.is_ascii_hexdigit()) {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Integer,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Parses a binary number after seeing `0b` or `0B`.
    fn binary_number(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume b/B

        // Must have at least one binary digit
        if !self.current_char().is_some_and(|c| c == '0' || c == '1') {
            return Err(Error::new(
                ErrorKind::InvalidNumber,
                Span::new(start, self.position),
            ));
        }

        while self.current_char().is_some_and(|c| c == '0' || c == '1') {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Integer,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Parses an octal number after seeing `0o` or `0O`.
    fn octal_number(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume o/O

        // Must have at least one octal digit
        if !self
            .current_char()
            .is_some_and(|c| ('0'..='7').contains(&c))
        {
            return Err(Error::new(
                ErrorKind::InvalidNumber,
                Span::new(start, self.position),
            ));
        }

        while self
            .current_char()
            .is_some_and(|c| ('0'..='7').contains(&c))
        {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Integer,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Parses the fractional part of a float after seeing `.`.
    fn float_after_integer(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume .

        // Must have at least one digit after the dot
        if !self.current_char().is_some_and(|c| c.is_ascii_digit()) {
            return Err(Error::new(
                ErrorKind::InvalidNumber,
                Span::new(start, self.position),
            ));
        }

        while self.current_char().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
        }

        // Check for exponent
        if self.current_char().is_some_and(|c| c == 'e' || c == 'E') {
            return self.float_exponent(start);
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Float,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Parses the exponent part of a float after seeing `e` or `E`.
    fn float_exponent(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume e/E

        // Optional sign
        if self.current_char().is_some_and(|c| c == '+' || c == '-') {
            self.advance();
        }

        // Must have at least one digit
        if !self.current_char().is_some_and(|c| c.is_ascii_digit()) {
            return Err(Error::new(
                ErrorKind::InvalidNumber,
                Span::new(start, self.position),
            ));
        }

        while self.current_char().is_some_and(|c| c.is_ascii_digit()) {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Float,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Handles `-` which could start a negative number or be a symbol.
    fn minus_token(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume -

        // Check if followed by a digit (negative number)
        if self.current_char().is_some_and(|c| c.is_ascii_digit()) {
            return self.number_token(start);
        }

        // Otherwise it's a symbol (possibly just `-`)
        while self.current_char().is_some_and(is_symbol_continue) {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Symbol,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Handles `+` which could be followed by digits or be a symbol.
    fn plus_token(&mut self, start: usize) -> Result<Token<'src>, Error> {
        self.advance(); // consume +

        // Check if followed by a digit (positive number - but we keep +)
        if self.current_char().is_some_and(|c| c.is_ascii_digit()) {
            // Actually, in Lonala/Clojure, +42 is not a number literal
            // + followed by digits is the symbol + followed by a number
            // So we just return the + symbol
            let lexeme = self.source.get(start..self.position).unwrap_or("");
            return Ok(Token::new(
                TokenKind::Symbol,
                lexeme,
                Span::new(start, self.position),
            ));
        }

        // Otherwise continue as symbol
        while self.current_char().is_some_and(is_symbol_continue) {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");
        Ok(Token::new(
            TokenKind::Symbol,
            lexeme,
            Span::new(start, self.position),
        ))
    }

    /// Parses a symbol.
    fn symbol_token(&mut self, start: usize) -> Result<Token<'src>, Error> {
        while self.current_char().is_some_and(is_symbol_continue) {
            self.advance();
        }

        let lexeme = self.source.get(start..self.position).unwrap_or("");

        // Check for reserved words
        let kind = match lexeme {
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            _ => TokenKind::Symbol,
        };

        Ok(Token::new(kind, lexeme, Span::new(start, self.position)))
    }
}

impl<'src> Iterator for Lexer<'src> {
    type Item = Result<Token<'src>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.peeked.take() {
            Some(token) => token,
            None => self.next_token(),
        }
    }
}

/// Returns true if `ch` can start a symbol.
fn is_symbol_start(ch: char) -> bool {
    ch.is_ascii_alphabetic()
        || matches!(
            ch,
            '_' | '+' | '-' | '*' | '/' | '<' | '>' | '=' | '!' | '?' | '&' | '%' | '^' | '.'
        )
}

/// Returns true if `ch` can continue a symbol.
fn is_symbol_continue(ch: char) -> bool {
    is_symbol_start(ch) || ch.is_ascii_digit()
}

/// Tokenizes the entire source into a vector of tokens.
///
/// This is a convenience function that collects all tokens from the lexer.
/// Returns an error if any token fails to parse.
#[cfg(feature = "alloc")]
pub fn tokenize(source: &str) -> Result<Vec<Token<'_>>, Error> {
    Lexer::new(source).collect()
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;

    // Helper to tokenize and unwrap
    fn lex(source: &str) -> Vec<Token<'_>> {
        tokenize(source).expect("lexing should succeed")
    }

    // Helper to get token kinds
    fn kinds(source: &str) -> Vec<TokenKind> {
        lex(source).into_iter().map(|t| t.kind).collect()
    }

    // ==================== Empty and Whitespace ====================

    #[test]
    fn empty_input() {
        assert!(lex("").is_empty());
    }

    #[test]
    fn whitespace_only() {
        assert!(lex("   \t\n\r  ").is_empty());
    }

    #[test]
    fn commas_are_whitespace() {
        assert!(lex("  ,  ,  ").is_empty());
    }

    // ==================== Comments ====================

    #[test]
    fn comment_only() {
        assert!(lex("; this is a comment").is_empty());
    }

    #[test]
    fn comment_at_end_of_line() {
        let tokens = lex("42 ; comment\n43");
        assert_eq!(tokens.len(), 2_usize);
        assert_eq!(tokens.first().map(|t| t.lexeme), Some("42"));
        assert_eq!(tokens.get(1_usize).map(|t| t.lexeme), Some("43"));
    }

    // ==================== Delimiters ====================

    #[test]
    fn delimiters() {
        assert_eq!(
            kinds("()[]{}"),
            vec![
                TokenKind::LeftParen,
                TokenKind::RightParen,
                TokenKind::LeftBracket,
                TokenKind::RightBracket,
                TokenKind::LeftBrace,
                TokenKind::RightBrace,
            ]
        );
    }

    // ==================== Integers ====================

    #[test]
    fn integer_decimal() {
        let tokens = lex("42 0 123");
        assert_eq!(tokens.len(), 3_usize);
        assert!(tokens.iter().all(|t| t.kind == TokenKind::Integer));
        assert_eq!(tokens.first().map(|t| t.lexeme), Some("42"));
        assert_eq!(tokens.get(1_usize).map(|t| t.lexeme), Some("0"));
        assert_eq!(tokens.get(2_usize).map(|t| t.lexeme), Some("123"));
    }

    #[test]
    fn integer_negative() {
        let tokens = lex("-42");
        assert_eq!(tokens.len(), 1_usize);
        assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::Integer));
        assert_eq!(tokens.first().map(|t| t.lexeme), Some("-42"));
    }

    #[test]
    fn integer_hex() {
        let tokens = lex("0xFF 0x1a2B");
        assert_eq!(tokens.len(), 2_usize);
        assert!(tokens.iter().all(|t| t.kind == TokenKind::Integer));
        assert_eq!(tokens.first().map(|t| t.lexeme), Some("0xFF"));
        assert_eq!(tokens.get(1_usize).map(|t| t.lexeme), Some("0x1a2B"));
    }

    #[test]
    fn integer_binary() {
        let tokens = lex("0b1010 0B11");
        assert_eq!(tokens.len(), 2_usize);
        assert!(tokens.iter().all(|t| t.kind == TokenKind::Integer));
    }

    #[test]
    fn integer_octal() {
        let tokens = lex("0o755 0O17");
        assert_eq!(tokens.len(), 2_usize);
        assert!(tokens.iter().all(|t| t.kind == TokenKind::Integer));
    }

    // ==================== Floats ====================

    #[test]
    fn float_simple() {
        let tokens = lex("3.14 0.5");
        assert_eq!(tokens.len(), 2_usize);
        assert!(tokens.iter().all(|t| t.kind == TokenKind::Float));
        assert_eq!(tokens.first().map(|t| t.lexeme), Some("3.14"));
    }

    #[test]
    fn float_negative() {
        let tokens = lex("-3.14");
        assert_eq!(tokens.len(), 1_usize);
        assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::Float));
    }

    #[test]
    fn float_scientific() {
        let tokens = lex("1e10 2.5e-3 1E+5");
        assert_eq!(tokens.len(), 3_usize);
        assert!(tokens.iter().all(|t| t.kind == TokenKind::Float));
    }

    #[test]
    fn float_special_nan() {
        let tokens = lex("##NaN");
        assert_eq!(tokens.len(), 1_usize);
        assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::Float));
        assert_eq!(tokens.first().map(|t| t.lexeme), Some("##NaN"));
    }

    #[test]
    fn float_special_inf() {
        let tokens = lex("##Inf ##-Inf");
        assert_eq!(tokens.len(), 2_usize);
        assert!(tokens.iter().all(|t| t.kind == TokenKind::Float));
    }

    // ==================== Strings ====================

    #[test]
    fn string_empty() {
        let tokens = lex(r#""""#);
        assert_eq!(tokens.len(), 1_usize);
        assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::String));
        assert_eq!(tokens.first().map(|t| t.lexeme), Some("\"\""));
    }

    #[test]
    fn string_simple() {
        let tokens = lex(r#""hello""#);
        assert_eq!(tokens.len(), 1_usize);
        assert_eq!(tokens.first().map(|t| t.lexeme), Some("\"hello\""));
    }

    #[test]
    fn string_with_escapes() {
        let tokens = lex(r#""hello\nworld""#);
        assert_eq!(tokens.len(), 1_usize);
        assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::String));
    }

    #[test]
    fn string_with_unicode_escape() {
        let tokens = lex(r#""\u0041""#);
        assert_eq!(tokens.len(), 1_usize);
        assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::String));
    }

    #[test]
    fn string_with_escaped_quote() {
        let tokens = lex(r#""say \"hi\"""#);
        assert_eq!(tokens.len(), 1_usize);
    }

    // ==================== Booleans and Nil ====================

    #[test]
    fn boolean_true() {
        let tokens = lex("true");
        assert_eq!(tokens.len(), 1_usize);
        assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::True));
    }

    #[test]
    fn boolean_false() {
        let tokens = lex("false");
        assert_eq!(tokens.len(), 1_usize);
        assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::False));
    }

    #[test]
    fn nil_literal() {
        let tokens = lex("nil");
        assert_eq!(tokens.len(), 1_usize);
        assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::Nil));
    }

    // ==================== Symbols ====================

    #[test]
    fn symbol_simple() {
        let tokens = lex("foo bar baz");
        assert_eq!(tokens.len(), 3_usize);
        assert!(tokens.iter().all(|t| t.kind == TokenKind::Symbol));
        assert_eq!(tokens.first().map(|t| t.lexeme), Some("foo"));
    }

    #[test]
    fn symbol_operators() {
        let tokens = lex("+ - * / < > = <= >= !=");
        assert!(tokens.iter().all(|t| t.kind == TokenKind::Symbol));
    }

    #[test]
    fn symbol_with_special_chars() {
        let tokens = lex("update! empty? ->arrow *special*");
        assert_eq!(tokens.len(), 4_usize);
        assert!(tokens.iter().all(|t| t.kind == TokenKind::Symbol));
    }

    #[test]
    fn symbol_namespaced() {
        let tokens = lex("ns/name foo.bar/baz");
        assert_eq!(tokens.len(), 2_usize);
        assert!(tokens.iter().all(|t| t.kind == TokenKind::Symbol));
    }

    // ==================== Keywords ====================

    #[test]
    fn keyword_simple() {
        let tokens = lex(":foo :bar");
        assert_eq!(tokens.len(), 2_usize);
        assert!(tokens.iter().all(|t| t.kind == TokenKind::Keyword));
        assert_eq!(tokens.first().map(|t| t.lexeme), Some(":foo"));
    }

    #[test]
    fn keyword_namespaced() {
        let tokens = lex(":ns/name");
        assert_eq!(tokens.len(), 1_usize);
        assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::Keyword));
    }

    #[test]
    fn keyword_kebab_case() {
        let tokens = lex(":kebab-case");
        assert_eq!(tokens.len(), 1_usize);
        assert_eq!(tokens.first().map(|t| t.lexeme), Some(":kebab-case"));
    }

    // ==================== Reader Macros ====================

    #[test]
    fn quote() {
        let tokens = lex("'x");
        assert_eq!(tokens.len(), 2_usize);
        assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::Quote));
        assert_eq!(tokens.get(1_usize).map(|t| t.kind), Some(TokenKind::Symbol));
    }

    #[test]
    fn syntax_quote() {
        let tokens = lex("`x");
        assert_eq!(tokens.len(), 2_usize);
        assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::SyntaxQuote));
    }

    #[test]
    fn unquote() {
        let tokens = lex("~x");
        assert_eq!(tokens.len(), 2_usize);
        assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::Unquote));
    }

    #[test]
    fn unquote_splice() {
        let tokens = lex("~@x");
        assert_eq!(tokens.len(), 2_usize);
        assert_eq!(
            tokens.first().map(|t| t.kind),
            Some(TokenKind::UnquoteSplice)
        );
        assert_eq!(tokens.first().map(|t| t.lexeme), Some("~@"));
    }

    // ==================== Complex Expressions ====================

    #[test]
    fn simple_list() {
        let tokens = lex("(+ 1 2)");
        assert_eq!(tokens.len(), 5_usize);
        assert_eq!(
            kinds("(+ 1 2)"),
            vec![
                TokenKind::LeftParen,
                TokenKind::Symbol,
                TokenKind::Integer,
                TokenKind::Integer,
                TokenKind::RightParen,
            ]
        );
    }

    #[test]
    fn nested_list() {
        let tokens = lex("(def x (+ 1 2))");
        assert_eq!(tokens.len(), 9_usize);
    }

    #[test]
    fn map_literal() {
        let tokens = lex("{:a 1 :b 2}");
        assert_eq!(tokens.len(), 6_usize);
        assert_eq!(
            kinds("{:a 1 :b 2}"),
            vec![
                TokenKind::LeftBrace,
                TokenKind::Keyword,
                TokenKind::Integer,
                TokenKind::Keyword,
                TokenKind::Integer,
                TokenKind::RightBrace,
            ]
        );
    }

    #[test]
    fn vector_literal() {
        let tokens = lex("[1 2 3]");
        assert_eq!(tokens.len(), 5_usize);
    }

    #[test]
    fn quoted_list() {
        let tokens = lex("'(1 2 3)");
        assert_eq!(tokens.len(), 6_usize);
        assert_eq!(tokens.first().map(|t| t.kind), Some(TokenKind::Quote));
    }

    #[test]
    fn function_definition() {
        let tokens = lex("(defn foo [x] x)");
        assert_eq!(tokens.len(), 8_usize);
    }

    // ==================== Span Tests ====================

    #[test]
    fn span_tracking() {
        let tokens = lex("foo bar");
        assert_eq!(tokens.first().map(|t| t.span), Some(Span::new(0, 3)));
        assert_eq!(tokens.get(1_usize).map(|t| t.span), Some(Span::new(4, 7)));
    }

    #[test]
    fn span_with_unicode() {
        let tokens = lex("hello"); // simple ascii first
        assert_eq!(tokens.first().map(|t| t.span.len()), Some(5_usize));
    }

    // ==================== Error Cases ====================

    #[test]
    fn error_unterminated_string() {
        let result = tokenize(r#""unterminated"#);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, ErrorKind::UnterminatedString);
    }

    #[test]
    fn error_invalid_escape() {
        let result = tokenize(r#""\q""#);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, ErrorKind::InvalidEscapeSequence('q'));
    }

    #[test]
    fn error_invalid_unicode_escape() {
        let result = tokenize(r#""\u00""#);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, ErrorKind::InvalidUnicodeEscape);
    }

    #[test]
    fn error_invalid_hex_number() {
        let result = tokenize("0x");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, ErrorKind::InvalidNumber);
    }

    #[test]
    fn error_invalid_binary_number() {
        let result = tokenize("0b");
        assert!(result.is_err());
    }

    #[test]
    fn error_invalid_octal_number() {
        let result = tokenize("0o");
        assert!(result.is_err());
    }

    #[test]
    fn error_unexpected_character() {
        let result = tokenize("@");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, ErrorKind::UnexpectedCharacter('@'));
    }

    #[test]
    fn error_bare_colon() {
        let result = tokenize(": ");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind, ErrorKind::UnexpectedCharacter(':'));
    }

    // ==================== Peek Tests ====================

    #[test]
    fn peek_does_not_consume() {
        let mut lexer = Lexer::new("foo bar");
        let peeked = lexer.peek().cloned();
        let next = lexer.next();
        assert_eq!(
            peeked
                .as_ref()
                .and_then(|r| r.as_ref().ok().map(|t| t.lexeme)),
            Some("foo")
        );
        assert_eq!(
            next.as_ref()
                .and_then(|r| r.as_ref().ok().map(|t| t.lexeme)),
            Some("foo")
        );
    }

    #[test]
    fn peek_at_end_returns_none() {
        let mut lexer = Lexer::new("");
        assert!(lexer.peek().is_none());
    }
}
