// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright 2026 Tobias Sarnowski

//! Parser for Lonala source code.
//!
//! Converts a token stream into Lonala values.

use super::lexer::{LexError, Lexer, Token};
use crate::platform::MemorySpace;
use crate::process::Process;
use crate::realm::Realm;
use crate::term::Term;
use core::option::Option::{self, None, Some};
use core::result::Result::{self, Err, Ok};

/// Maximum number of elements in a list literal.
///
/// This limit exists because we collect list elements on the stack before
/// building the linked list. A future optimization could build the list
/// incrementally to remove this limit.
const MAX_LIST_ELEMENTS: usize = 64;

/// Maximum number of elements in a tuple literal.
const MAX_TUPLE_ELEMENTS: usize = 64;

/// Maximum number of elements in a vector literal.
const MAX_VECTOR_ELEMENTS: usize = 64;

/// Maximum number of key-value pairs in a map literal.
const MAX_MAP_ENTRIES: usize = 64;

/// Parse error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// Unexpected end of input.
    UnexpectedEof,
    /// Unexpected token.
    UnexpectedToken(Token),
    /// Unmatched right parenthesis.
    UnmatchedRParen,
    /// Unmatched right bracket.
    UnmatchedRBracket,
    /// Unmatched right brace.
    UnmatchedRBrace,
    /// Out of memory.
    OutOfMemory,
    /// List literal exceeds maximum element count.
    ListTooLong,
    /// Tuple literal exceeds maximum element count.
    TupleTooLong,
    /// Vector literal exceeds maximum element count.
    VectorTooLong,
    /// Map literal exceeds maximum entry count.
    MapTooLong,
    /// Map literal has odd number of elements (should be key-value pairs).
    MapOddElements,
    /// Invalid metadata (must be map or keyword).
    InvalidMetadata,
    /// Missing form after metadata.
    MissingFormAfterMetadata,
}

impl core::fmt::Display for ParseError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnexpectedEof => write!(f, "unexpected end of input"),
            Self::UnexpectedToken(t) => write!(f, "unexpected token: {t:?}"),
            Self::UnmatchedRParen => write!(f, "unmatched )"),
            Self::UnmatchedRBracket => write!(f, "unmatched ]"),
            Self::UnmatchedRBrace => write!(f, "unmatched }}"),
            Self::OutOfMemory => write!(f, "out of memory"),
            Self::ListTooLong => write!(f, "list exceeds {MAX_LIST_ELEMENTS} elements"),
            Self::TupleTooLong => write!(f, "tuple exceeds {MAX_TUPLE_ELEMENTS} elements"),
            Self::VectorTooLong => write!(f, "vector exceeds {MAX_VECTOR_ELEMENTS} elements"),
            Self::MapTooLong => write!(f, "map exceeds {MAX_MAP_ENTRIES} entries"),
            Self::MapOddElements => write!(f, "map literal requires even number of elements"),
            Self::InvalidMetadata => write!(f, "metadata must be map or keyword"),
            Self::MissingFormAfterMetadata => write!(f, "expected form after metadata"),
        }
    }
}

/// Combined read error (lexer + parser).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// Lexer error.
    Lex(LexError),
    /// Parser error.
    Parse(ParseError),
}

impl core::fmt::Display for ReadError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Lex(e) => write!(f, "{e}"),
            Self::Parse(e) => write!(f, "{e}"),
        }
    }
}

impl From<LexError> for ReadError {
    fn from(e: LexError) -> Self {
        Self::Lex(e)
    }
}

impl From<ParseError> for ReadError {
    fn from(e: ParseError) -> Self {
        Self::Parse(e)
    }
}

/// Parser state.
pub struct Parser<'a> {
    lexer: Lexer<'a>,
    /// Lookahead token.
    lookahead: Option<Token>,
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given input.
    #[must_use]
    pub fn new(input: &'a str) -> Self {
        Self {
            lexer: Lexer::new(input),
            lookahead: None,
        }
    }

    /// Read one expression.
    ///
    /// Returns `None` if at end of input.
    ///
    /// Symbols and keywords are interned in the Realm (persistent, shared).
    /// Other heap allocations (strings, pairs, tuples, etc.) go on the process heap.
    ///
    /// # Errors
    ///
    /// Returns `ReadError` if the input contains invalid syntax or if
    /// memory allocation fails.
    pub fn read<M: MemorySpace>(
        &mut self,
        proc: &mut Process,
        realm: &mut Realm,
        mem: &mut M,
    ) -> Result<Option<Term>, ReadError> {
        let token = match self.peek()? {
            Some(t) => t.clone(),
            None => return Ok(None),
        };
        self.advance();

        match token {
            Token::Nil => Ok(Some(Term::NIL)),
            Token::True => Ok(Some(Term::TRUE)),
            Token::False => Ok(Some(Term::FALSE)),
            Token::Int(n) => {
                // For small integers, use immediate encoding
                // TODO: Support bignums for large integers
                let term = Term::small_int(n).ok_or(ParseError::OutOfMemory)?;
                Ok(Some(term))
            }
            Token::String(s) => {
                let term = proc
                    .alloc_term_string(mem, s.as_str())
                    .ok_or(ParseError::OutOfMemory)?;
                Ok(Some(term))
            }
            Token::Symbol(s) => {
                let value = realm
                    .intern_symbol(mem, s.as_str())
                    .ok_or(ParseError::OutOfMemory)?;
                Ok(Some(value))
            }
            Token::Keyword(s) => {
                let value = realm
                    .intern_keyword(mem, s.as_str())
                    .ok_or(ParseError::OutOfMemory)?;
                Ok(Some(value))
            }
            Token::Quote => {
                // 'expr => (quote expr)
                let expr = self
                    .read(proc, realm, mem)?
                    .ok_or(ParseError::UnexpectedEof)?;
                let quote_sym = realm
                    .intern_symbol(mem, "quote")
                    .ok_or(ParseError::OutOfMemory)?;
                // Build (quote expr) = Pair(quote, Pair(expr, nil))
                let inner = proc
                    .alloc_term_pair(mem, expr, Term::NIL)
                    .ok_or(ParseError::OutOfMemory)?;
                let outer = proc
                    .alloc_term_pair(mem, quote_sym, inner)
                    .ok_or(ParseError::OutOfMemory)?;
                Ok(Some(outer))
            }
            Token::VarQuote => {
                // #'expr => (var expr)
                // var is a special form that returns the var object itself
                let expr = self
                    .read(proc, realm, mem)?
                    .ok_or(ParseError::UnexpectedEof)?;
                let var_sym = realm
                    .intern_symbol(mem, "var")
                    .ok_or(ParseError::OutOfMemory)?;
                // Build (var expr) = Pair(var, Pair(expr, nil))
                let inner = proc
                    .alloc_term_pair(mem, expr, Term::NIL)
                    .ok_or(ParseError::OutOfMemory)?;
                let outer = proc
                    .alloc_term_pair(mem, var_sym, inner)
                    .ok_or(ParseError::OutOfMemory)?;
                Ok(Some(outer))
            }
            Token::LParen => self.read_list(proc, realm, mem),
            Token::RParen => Err(ParseError::UnmatchedRParen.into()),
            Token::LBracket => self.read_tuple(proc, realm, mem),
            Token::RBracket => Err(ParseError::UnmatchedRBracket.into()),
            Token::LBrace => self.read_vector(proc, realm, mem),
            Token::MapStart => self.read_map(proc, realm, mem),
            Token::RBrace => Err(ParseError::UnmatchedRBrace.into()),
            Token::Caret => self.read_with_metadata(proc, realm, mem),
        }
    }

    fn read_list<M: MemorySpace>(
        &mut self,
        proc: &mut Process,
        realm: &mut Realm,
        mem: &mut M,
    ) -> Result<Option<Term>, ReadError> {
        // Collect elements on stack before building linked list
        let mut elements = [Term::NIL; MAX_LIST_ELEMENTS];
        let mut count = 0;

        loop {
            match self.peek()? {
                None => return Err(ParseError::UnexpectedEof.into()),
                Some(Token::RParen) => {
                    self.advance();
                    break;
                }
                Some(_) => {
                    if count >= elements.len() {
                        return Err(ParseError::ListTooLong.into());
                    }
                    let elem = self
                        .read(proc, realm, mem)?
                        .ok_or(ParseError::UnexpectedEof)?;
                    elements[count] = elem;
                    count += 1;
                }
            }
        }

        // Build list from back to front: (a b c) = Pair(a, Pair(b, Pair(c, nil)))
        let mut result = Term::NIL;
        for i in (0..count).rev() {
            result = proc
                .alloc_term_pair(mem, elements[i], result)
                .ok_or(ParseError::OutOfMemory)?;
        }

        Ok(Some(result))
    }

    fn read_tuple<M: MemorySpace>(
        &mut self,
        proc: &mut Process,
        realm: &mut Realm,
        mem: &mut M,
    ) -> Result<Option<Term>, ReadError> {
        // Collect elements on stack before building tuple
        let mut elements = [Term::NIL; MAX_TUPLE_ELEMENTS];
        let mut count = 0;

        loop {
            match self.peek()? {
                None => return Err(ParseError::UnexpectedEof.into()),
                Some(Token::RBracket) => {
                    self.advance();
                    break;
                }
                Some(_) => {
                    if count >= elements.len() {
                        return Err(ParseError::TupleTooLong.into());
                    }
                    let elem = self
                        .read(proc, realm, mem)?
                        .ok_or(ParseError::UnexpectedEof)?;
                    elements[count] = elem;
                    count += 1;
                }
            }
        }

        // Allocate and build the tuple
        let tuple = proc
            .alloc_term_tuple(mem, &elements[..count])
            .ok_or(ParseError::OutOfMemory)?;

        Ok(Some(tuple))
    }

    fn read_vector<M: MemorySpace>(
        &mut self,
        proc: &mut Process,
        realm: &mut Realm,
        mem: &mut M,
    ) -> Result<Option<Term>, ReadError> {
        // Collect elements on stack before building vector
        let mut elements = [Term::NIL; MAX_VECTOR_ELEMENTS];
        let mut count = 0;

        loop {
            match self.peek()? {
                None => return Err(ParseError::UnexpectedEof.into()),
                Some(Token::RBrace) => {
                    self.advance();
                    break;
                }
                Some(_) => {
                    if count >= elements.len() {
                        return Err(ParseError::VectorTooLong.into());
                    }
                    let elem = self
                        .read(proc, realm, mem)?
                        .ok_or(ParseError::UnexpectedEof)?;
                    elements[count] = elem;
                    count += 1;
                }
            }
        }

        // Allocate and build the vector
        let vector = proc
            .alloc_term_vector(mem, &elements[..count])
            .ok_or(ParseError::OutOfMemory)?;

        Ok(Some(vector))
    }

    fn read_map<M: MemorySpace>(
        &mut self,
        proc: &mut Process,
        realm: &mut Realm,
        mem: &mut M,
    ) -> Result<Option<Term>, ReadError> {
        // Collect key-value pairs on stack before building map
        // Each entry is 2 Terms: key, value
        let mut elements = [Term::NIL; MAX_MAP_ENTRIES * 2];
        let mut count = 0;

        loop {
            match self.peek()? {
                None => return Err(ParseError::UnexpectedEof.into()),
                Some(Token::RBrace) => {
                    self.advance();
                    break;
                }
                Some(_) => {
                    if count >= elements.len() {
                        return Err(ParseError::MapTooLong.into());
                    }
                    let elem = self
                        .read(proc, realm, mem)?
                        .ok_or(ParseError::UnexpectedEof)?;
                    elements[count] = elem;
                    count += 1;
                }
            }
        }

        // Must have even number of elements (key-value pairs)
        if count % 2 != 0 {
            return Err(ParseError::MapOddElements.into());
        }

        // Build the map as association list from back to front
        // %{:a 1 :b 2} → Pair([:a 1], Pair([:b 2], nil))
        let mut entries = Term::NIL;
        let entry_count = count / 2;
        for i in (0..count).step_by(2).rev() {
            // Build [key value] tuple
            let pair_elements = [elements[i], elements[i + 1]];
            let kv_tuple = proc
                .alloc_term_tuple(mem, &pair_elements)
                .ok_or(ParseError::OutOfMemory)?;

            // Prepend to entries list
            entries = proc
                .alloc_term_pair(mem, kv_tuple, entries)
                .ok_or(ParseError::OutOfMemory)?;
        }

        // Allocate the map with entries
        let map = proc
            .alloc_term_map(mem, entries, entry_count)
            .ok_or(ParseError::OutOfMemory)?;

        Ok(Some(map))
    }

    /// Read a form with metadata attached.
    ///
    /// Handles both `^%{:k v} form` and `^:keyword form` syntax.
    /// Multiple metadata prefixes are merged, with later values overriding earlier.
    fn read_with_metadata<M: MemorySpace>(
        &mut self,
        proc: &mut Process,
        realm: &mut Realm,
        mem: &mut M,
    ) -> Result<Option<Term>, ReadError> {
        // Collect all metadata maps/keywords before the form
        // We need to merge multiple metadata: ^:a ^:b foo → {:a true :b true}
        let mut meta_entries = [Term::NIL; MAX_MAP_ENTRIES * 2];
        let mut meta_count = 0;

        // Read the first metadata token (we already consumed the ^)
        let first_meta = self.read_metadata_value(proc, realm, mem)?;
        add_metadata_entries(first_meta, &mut meta_entries, &mut meta_count, proc, mem)?;

        // Check for additional metadata prefixes
        loop {
            match self.peek()? {
                Some(Token::Caret) => {
                    self.advance(); // consume ^
                    let next_meta = self.read_metadata_value(proc, realm, mem)?;
                    add_metadata_entries(next_meta, &mut meta_entries, &mut meta_count, proc, mem)?;
                }
                Some(_) => break,
                None => return Err(ParseError::MissingFormAfterMetadata.into()),
            }
        }

        // Read the actual form
        let form = self
            .read(proc, realm, mem)?
            .ok_or(ParseError::MissingFormAfterMetadata)?;

        // Build the merged metadata map
        let mut entries = Term::NIL;
        let entry_count = meta_count / 2;
        for i in (0..meta_count).step_by(2).rev() {
            // Build [key value] tuple
            let pair_elements = [meta_entries[i], meta_entries[i + 1]];
            let kv_tuple = proc
                .alloc_term_tuple(mem, &pair_elements)
                .ok_or(ParseError::OutOfMemory)?;

            entries = proc
                .alloc_term_pair(mem, kv_tuple, entries)
                .ok_or(ParseError::OutOfMemory)?;
        }

        let meta_map = proc
            .alloc_term_map(mem, entries, entry_count)
            .ok_or(ParseError::OutOfMemory)?;

        // Store the metadata for the form in the realm's metadata table
        // The actual form is returned; the caller (or later phases) can retrieve meta
        if let (Some(map_addr), Some(addr)) = (get_heap_addr(meta_map), get_heap_addr(form)) {
            realm
                .set_metadata(addr, map_addr)
                .ok_or(ParseError::OutOfMemory)?;
        }

        Ok(Some(form))
    }

    /// Read the value after ^ - either a map or keyword shorthand
    fn read_metadata_value<M: MemorySpace>(
        &mut self,
        proc: &mut Process,
        realm: &mut Realm,
        mem: &mut M,
    ) -> Result<Term, ReadError> {
        match self.peek()? {
            None => Err(ParseError::MissingFormAfterMetadata.into()),
            Some(Token::MapStart) => {
                self.advance();
                let map = self.read_map(proc, realm, mem)?;
                map.ok_or_else(|| ParseError::MissingFormAfterMetadata.into())
            }
            Some(Token::Keyword(_)) => {
                // ^:keyword is shorthand for ^%{:keyword true}
                let kw = self.read(proc, realm, mem)?;
                kw.ok_or_else(|| ParseError::MissingFormAfterMetadata.into())
            }
            Some(_) => Err(ParseError::InvalidMetadata.into()),
        }
    }

    fn peek(&mut self) -> Result<Option<&Token>, LexError> {
        if self.lookahead.is_none() {
            self.lookahead = self.lexer.next_token()?;
        }
        Ok(self.lookahead.as_ref())
    }

    const fn advance(&mut self) {
        self.lookahead = None;
    }
}

/// Read a single expression from a string.
///
/// Symbols and keywords are interned in the Realm (persistent, shared).
///
/// # Errors
///
/// Returns an error if the input contains invalid syntax.
pub fn read<M: MemorySpace>(
    input: &str,
    proc: &mut Process,
    realm: &mut Realm,
    mem: &mut M,
) -> Result<Option<Term>, ReadError> {
    let mut parser = Parser::new(input);
    parser.read(proc, realm, mem)
}

/// Add entries from a metadata value (map or keyword) to the entries array.
fn add_metadata_entries<M: MemorySpace>(
    meta: Term,
    entries: &mut [Term],
    count: &mut usize,
    proc: &Process,
    mem: &M,
) -> Result<(), ReadError> {
    if meta.is_keyword() {
        // ^:keyword → {:keyword true}
        if *count + 2 > entries.len() {
            return Err(ParseError::MapTooLong.into());
        }
        entries[*count] = meta;
        entries[*count + 1] = Term::TRUE;
        *count += 2;
    } else if proc.is_term_map(mem, meta) {
        // Copy all entries from the map
        if let Some(mut current) = proc.read_term_map_entries(mem, meta) {
            while let Some((entry, rest)) = proc.read_term_pair(mem, current) {
                // Each entry is a [key value] tuple
                if let Some(kv_key) = proc.read_term_tuple_element(mem, entry, 0) {
                    if let Some(kv_val) = proc.read_term_tuple_element(mem, entry, 1) {
                        if *count + 2 > entries.len() {
                            return Err(ParseError::MapTooLong.into());
                        }
                        entries[*count] = kv_key;
                        entries[*count + 1] = kv_val;
                        *count += 2;
                    }
                }
                current = rest;
            }
        }
    } else {
        return Err(ParseError::InvalidMetadata.into());
    }
    Ok(())
}

/// Get the heap address of a term if it has one.
///
/// Returns `Some(addr)` for heap-allocated terms (boxed or list), `None` for immediates.
const fn get_heap_addr(term: Term) -> Option<crate::Vaddr> {
    if term.is_boxed() || term.is_list() {
        Some(term.to_vaddr())
    } else {
        None
    }
}
