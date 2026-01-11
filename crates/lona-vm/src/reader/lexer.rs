// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Lexer for Lonala source code.
//!
//! Converts a string of source code into a stream of tokens.

use core::iter::Peekable;
use core::option::Option::{self, None, Some};
use core::result::Result::{self, Err, Ok};
use core::str::Chars;

/// A token in the Lonala language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    /// Left parenthesis `(`
    LParen,
    /// Right parenthesis `)`
    RParen,
    /// Left bracket `[`
    LBracket,
    /// Right bracket `]`
    RBracket,
    /// Left brace `{`
    LBrace,
    /// Right brace `}`
    RBrace,
    /// Map start `%{`
    MapStart,
    /// Quote `'`
    Quote,
    /// Var quote `#'`
    VarQuote,
    /// Metadata prefix `^`
    Caret,
    /// The `nil` literal
    Nil,
    /// The `true` literal
    True,
    /// The `false` literal
    False,
    /// Integer literal
    Int(i64),
    /// String literal (contents without quotes)
    String(TokenString),
    /// Symbol (identifier)
    Symbol(TokenString),
    /// Keyword (e.g., `:foo` or `:ns/bar`)
    Keyword(TokenString),
}

/// A string stored inline for `no_std` compatibility.
///
/// Stores up to 63 bytes of UTF-8 data inline.
#[derive(Clone, PartialEq, Eq)]
pub struct TokenString {
    len: u8,
    data: [u8; 63],
}

impl TokenString {
    /// Maximum capacity in bytes.
    pub const MAX_LEN: usize = 63;

    /// Create a new empty token string.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            len: 0,
            data: [0; 63],
        }
    }

    /// Create a token string from a string slice.
    ///
    /// Returns `None` if the string is too long.
    #[must_use]
    pub fn try_from_str(s: &str) -> Option<Self> {
        if s.len() > Self::MAX_LEN {
            return None;
        }
        let mut data = [0u8; 63];
        data[..s.len()].copy_from_slice(s.as_bytes());
        Some(Self {
            len: s.len() as u8,
            data,
        })
    }

    /// Get the string as a slice.
    ///
    /// Returns an empty string if the stored bytes are not valid UTF-8
    /// (should not happen if only using the safe constructors).
    #[must_use]
    pub fn as_str(&self) -> &str {
        // We only store valid UTF-8, but return empty string on error
        core::str::from_utf8(&self.data[..self.len as usize]).unwrap_or("")
    }

    /// Push a character to the string.
    ///
    /// Returns `false` if the string is full.
    pub fn push(&mut self, c: char) -> bool {
        let mut buf = [0u8; 4];
        let encoded = c.encode_utf8(&mut buf);
        if self.len as usize + encoded.len() > Self::MAX_LEN {
            return false;
        }
        self.data[self.len as usize..self.len as usize + encoded.len()]
            .copy_from_slice(encoded.as_bytes());
        self.len += encoded.len() as u8;
        true
    }
}

impl Default for TokenString {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for TokenString {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "TokenString({:?})", self.as_str())
    }
}

/// Lexer error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LexError {
    /// Unterminated string literal.
    UnterminatedString,
    /// Invalid escape sequence in string.
    InvalidEscape(char),
    /// Invalid number format.
    InvalidNumber,
    /// String or symbol too long.
    TooLong,
    /// Unexpected character.
    UnexpectedChar(char),
}

impl core::fmt::Display for LexError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnterminatedString => write!(f, "unterminated string"),
            Self::InvalidEscape(c) => write!(f, "invalid escape sequence: \\{c}"),
            Self::InvalidNumber => write!(f, "invalid number"),
            Self::TooLong => write!(f, "string or symbol too long"),
            Self::UnexpectedChar(c) => write!(f, "unexpected character: {c}"),
        }
    }
}

/// Lexer state.
pub struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given input.
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    /// Get the next token.
    ///
    /// # Errors
    ///
    /// Returns `LexError` if the input contains invalid syntax such as
    /// unterminated strings, invalid escape sequences, or unexpected characters.
    pub fn next_token(&mut self) -> Result<Option<Token>, LexError> {
        self.skip_whitespace();

        let Some(&c) = self.chars.peek() else {
            return Ok(None);
        };

        match c {
            '(' => {
                self.chars.next();
                Ok(Some(Token::LParen))
            }
            ')' => {
                self.chars.next();
                Ok(Some(Token::RParen))
            }
            '[' => {
                self.chars.next();
                Ok(Some(Token::LBracket))
            }
            ']' => {
                self.chars.next();
                Ok(Some(Token::RBracket))
            }
            '{' => {
                self.chars.next();
                Ok(Some(Token::LBrace))
            }
            '}' => {
                self.chars.next();
                Ok(Some(Token::RBrace))
            }
            '%' => {
                // Check for %{ (map start)
                self.chars.next(); // consume '%'
                if matches!(self.chars.peek(), Some('{')) {
                    self.chars.next(); // consume '{'
                    Ok(Some(Token::MapStart))
                } else {
                    // Just a % symbol
                    self.lex_symbol_rest('%')
                }
            }
            '\'' => {
                self.chars.next();
                Ok(Some(Token::Quote))
            }
            '^' => {
                self.chars.next();
                Ok(Some(Token::Caret))
            }
            '"' => self.lex_string(),
            '0'..='9' => self.lex_number(),
            '-' => {
                // Could be negative number or symbol
                self.chars.next();
                if let Some('0'..='9') = self.chars.peek() {
                    self.lex_negative_number()
                } else {
                    // Just a minus symbol
                    self.lex_symbol_rest('-')
                }
            }
            ':' => {
                // Keyword (e.g., :foo or :ns/bar)
                self.chars.next(); // consume ':'
                self.lex_keyword()
            }
            '#' => {
                // Reader macro: #' for var quote
                self.chars.next(); // consume '#'
                if matches!(self.chars.peek(), Some('\'')) {
                    self.chars.next(); // consume '\''
                    Ok(Some(Token::VarQuote))
                } else {
                    // Unknown reader macro
                    Err(LexError::UnexpectedChar('#'))
                }
            }
            _ if is_symbol_start(c) => self.lex_symbol(),
            _ => Err(LexError::UnexpectedChar(c)),
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() || c == ',' {
                self.chars.next();
            } else if c == ';' {
                // Skip comment to end of line
                while let Some(&c) = self.chars.peek() {
                    self.chars.next();
                    if c == '\n' {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    }

    fn lex_string(&mut self) -> Result<Option<Token>, LexError> {
        self.chars.next(); // consume opening quote
        let mut s = TokenString::new();

        loop {
            match self.chars.next() {
                Some('"') => return Ok(Some(Token::String(s))),
                Some('\\') => {
                    let escaped = match self.chars.next() {
                        Some('n') => '\n',
                        Some('t') => '\t',
                        Some('r') => '\r',
                        Some('\\') => '\\',
                        Some('"') => '"',
                        Some(c) => return Err(LexError::InvalidEscape(c)),
                        None => return Err(LexError::UnterminatedString),
                    };
                    if !s.push(escaped) {
                        return Err(LexError::TooLong);
                    }
                }
                Some(c) => {
                    if !s.push(c) {
                        return Err(LexError::TooLong);
                    }
                }
                None => return Err(LexError::UnterminatedString),
            }
        }
    }

    fn lex_number(&mut self) -> Result<Option<Token>, LexError> {
        let mut n: i64 = 0;

        while let Some(&c) = self.chars.peek() {
            if let Some(digit) = c.to_digit(10) {
                self.chars.next();
                n = n.checked_mul(10).ok_or(LexError::InvalidNumber)?;
                n = n
                    .checked_add(i64::from(digit))
                    .ok_or(LexError::InvalidNumber)?;
            } else if is_delimiter(c) {
                break;
            } else {
                return Err(LexError::InvalidNumber);
            }
        }

        Ok(Some(Token::Int(n)))
    }

    fn lex_negative_number(&mut self) -> Result<Option<Token>, LexError> {
        let mut n: i64 = 0;

        while let Some(&c) = self.chars.peek() {
            if let Some(digit) = c.to_digit(10) {
                self.chars.next();
                n = n.checked_mul(10).ok_or(LexError::InvalidNumber)?;
                n = n
                    .checked_add(i64::from(digit))
                    .ok_or(LexError::InvalidNumber)?;
            } else if is_delimiter(c) {
                break;
            } else {
                return Err(LexError::InvalidNumber);
            }
        }

        Ok(Some(Token::Int(-n)))
    }

    fn lex_symbol(&mut self) -> Result<Option<Token>, LexError> {
        // We only call this after peek() confirmed there's a character
        let Some(c) = self.chars.next() else {
            return Ok(None);
        };
        self.lex_symbol_rest(c)
    }

    fn lex_symbol_rest(&mut self, first: char) -> Result<Option<Token>, LexError> {
        let mut s = TokenString::new();
        if !s.push(first) {
            return Err(LexError::TooLong);
        }

        while let Some(&c) = self.chars.peek() {
            if is_symbol_continue(c) {
                self.chars.next();
                if !s.push(c) {
                    return Err(LexError::TooLong);
                }
            } else {
                break;
            }
        }

        // Check for reserved words
        let token = match s.as_str() {
            "nil" => Token::Nil,
            "true" => Token::True,
            "false" => Token::False,
            _ => Token::Symbol(s),
        };

        Ok(Some(token))
    }

    /// Lex a keyword (already consumed the leading ':').
    fn lex_keyword(&mut self) -> Result<Option<Token>, LexError> {
        let mut s = TokenString::new();

        // First character must be a keyword-start character (not digit)
        let Some(&c) = self.chars.peek() else {
            // Just ':' alone is an error
            return Err(LexError::UnexpectedChar(':'));
        };

        if !is_keyword_start(c) {
            return Err(LexError::UnexpectedChar(c));
        }

        self.chars.next();
        if !s.push(c) {
            return Err(LexError::TooLong);
        }

        // Continue reading keyword characters
        while let Some(&c) = self.chars.peek() {
            if is_keyword_continue(c) {
                self.chars.next();
                if !s.push(c) {
                    return Err(LexError::TooLong);
                }
            } else {
                break;
            }
        }

        Ok(Some(Token::Keyword(s)))
    }
}

fn is_symbol_start(c: char) -> bool {
    c.is_alphabetic()
        || matches!(
            c,
            '!' | '$' | '&' | '*' | '+' | '-' | '.' | '/' | '<' | '=' | '>' | '?' | '@' | '_' | '~'
        )
}

fn is_symbol_continue(c: char) -> bool {
    is_symbol_start(c) || c.is_ascii_digit() || c == ':'
}

/// Check if a character can start a keyword name (after the leading ':').
fn is_keyword_start(c: char) -> bool {
    c.is_alphabetic()
        || matches!(
            c,
            '!' | '$' | '&' | '*' | '+' | '-' | '.' | '/' | '<' | '=' | '>' | '?' | '@' | '_' | '~'
        )
}

/// Check if a character can continue a keyword name (allows digits and colons).
fn is_keyword_continue(c: char) -> bool {
    is_keyword_start(c) || c.is_ascii_digit() || c == ':'
}

fn is_delimiter(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\'' | ';' | ',' | '^'
        )
}

/// Tokenize an entire string into a vector of tokens.
#[cfg(test)]
pub fn tokenize(input: &str) -> Result<std::vec::Vec<Token>, LexError> {
    let mut lexer = Lexer::new(input);
    let mut tokens = std::vec::Vec::new();
    while let Some(tok) = lexer.next_token()? {
        tokens.push(tok);
    }
    Ok(tokens)
}
