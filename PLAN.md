# Implementation Plan: Task 2.2 - Parser

This document describes the implementation plan for **Phase 2.2: Parser** from the Lona implementation roadmap.

## Overview

**Task**: Implement a parser that transforms tokens into an Abstract Syntax Tree (AST)

**Description**: Tokens to AST, reader macros `' \` ~ ~@`

**Dependencies**: Phase 2.1 (Lexer) - completed

**Deliverable**: Parser that can parse S-expressions like `(+ 1 2)` into an AST

---

## Current State

### Completed (Phase 2.1 - Lexer)

| File | Description |
|------|-------------|
| `lonala-parser/src/token.rs` | Token kinds: delimiters, literals, symbols, keywords, reader macros |
| `lonala-parser/src/lexer.rs` | Streaming tokenization with `Iterator` trait, peek support |
| `lonala-parser/src/error.rs` | `Span`, `Error`, `Kind` for error reporting |

### Existing Foundation (Phase 1)

| File | Description |
|------|-------------|
| `lona-core/src/value.rs` | `Value` enum (Nil, Bool, Integer, Float, Symbol) |
| `lona-core/src/symbol.rs` | Symbol interning with `Interner` |

---

## Design Decisions

### 1. Separate AST Types (Not Reusing `Value`)

The parser will define its own AST types rather than producing `lona_core::Value` directly because:

- **Separation of concerns**: AST represents parsed source, `Value` represents runtime values
- **Source tracking**: AST nodes need span information for error reporting
- **Future compatibility**: Phase 3.2 extends `Value` with collections; AST should not depend on that
- **Compiler independence**: The bytecode compiler (Phase 2.4) will transform AST to bytecode

### 2. Spanned Nodes

Every AST node is wrapped in a `Spanned<T>` struct that carries source location:

```rust
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,
}
```

This enables precise error messages throughout compilation.

### 3. Reader Macro Expansion

Reader macros are desugared during parsing into their canonical list forms:

| Syntax | Expansion | Symbol |
|--------|-----------|--------|
| `'expr` | `(quote expr)` | `quote` |
| `` `expr `` | `(syntax-quote expr)` | `syntax-quote` |
| `~expr` | `(unquote expr)` | `unquote` |
| `~@expr` | `(unquote-splicing expr)` | `unquote-splicing` |

Example: `'(1 2 3)` becomes `(quote (1 2 3))` in the AST.

### 4. String Processing

The lexer validates escape sequences but preserves the raw lexeme. The parser processes escapes:

| Escape | Character |
|--------|-----------|
| `\\` | Backslash |
| `\"` | Double quote |
| `\n` | Newline |
| `\t` | Tab |
| `\r` | Carriage return |
| `\0` | Null |
| `\uXXXX` | Unicode code point |

### 5. No New Dependencies

The implementation uses only existing dependencies (`alloc` for `Vec`/`String`) and maintains `no_std` compatibility.

---

## AST Structure

### Core Types

```rust
/// Abstract Syntax Tree node for Lonala expressions.
#[derive(Debug, Clone, PartialEq)]
pub enum Ast {
    // Literals
    /// Integer literal (e.g., `42`, `-17`, `0xFF`)
    Integer(i64),
    /// Floating-point literal (e.g., `3.14`, `##NaN`)
    Float(f64),
    /// String literal with escapes processed (e.g., `"hello\nworld"`)
    String(String),
    /// Boolean literal (`true` or `false`)
    Bool(bool),
    /// Nil literal
    Nil,

    // Identifiers
    /// Symbol (e.g., `foo`, `+`, `ns/name`)
    Symbol(String),
    /// Keyword (e.g., `:foo`, `:ns/name`)
    Keyword(String),

    // Collections
    /// List `(...)` - function calls, special forms
    List(Vec<Spanned<Ast>>),
    /// Vector `[...]` - data structure
    Vector(Vec<Spanned<Ast>>),
    /// Map `{...}` - key-value pairs (must have even number of elements)
    Map(Vec<Spanned<Ast>>),
}

/// AST node with source location information.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    /// The AST node
    pub node: T,
    /// Source location
    pub span: Span,
}
```

### Examples

| Source | AST |
|--------|-----|
| `42` | `Integer(42)` |
| `"hello"` | `String("hello")` |
| `foo` | `Symbol("foo")` |
| `:key` | `Keyword("key")` |
| `(+ 1 2)` | `List([Symbol("+"), Integer(1), Integer(2)])` |
| `[1 2 3]` | `Vector([Integer(1), Integer(2), Integer(3)])` |
| `{:a 1}` | `Map([Keyword("a"), Integer(1)])` |
| `'x` | `List([Symbol("quote"), Symbol("x")])` |

---

## Error Types

Extend `error::Kind` with parse-specific errors:

```rust
pub enum Kind {
    // Existing lexer errors...
    UnexpectedCharacter(char),
    UnterminatedString,
    InvalidEscapeSequence(char),
    InvalidNumber,
    InvalidUnicodeEscape,

    // New parser errors
    /// Unexpected token encountered
    UnexpectedToken {
        expected: &'static str,
        found: &'static str,
    },
    /// Closing delimiter doesn't match opener
    UnmatchedDelimiter {
        opener: char,
        expected: char,
        found: char,
    },
    /// Unexpected end of input
    UnexpectedEof {
        expected: &'static str,
    },
    /// Map literal has odd number of elements
    OddMapEntries,
    /// Reader macro not followed by expression
    ReaderMacroMissingExpr,
}
```

---

## Implementation Steps

### Step 1: Create AST Module

**File**: `crates/lonala-parser/src/ast.rs`

- Define `Ast` enum with all variants
- Define `Spanned<T>` wrapper struct
- Implement `Display` for human-readable output
- Implement helper constructors (e.g., `Ast::list()`, `Ast::symbol()`)
- Add unit tests for AST construction and display

### Step 2: Extend Error Types

**File**: `crates/lonala-parser/src/error.rs`

- Add new `Kind` variants for parse errors
- Update `Display` implementation for new variants
- Add tests for error formatting

### Step 3: Create Parser Module

**File**: `crates/lonala-parser/src/parser.rs`

- Define `Parser<'src>` struct wrapping `Lexer<'src>`
- Implement core parsing methods (see below)
- Implement string escape processing
- Add comprehensive tests

### Step 4: Update Crate Exports

**File**: `crates/lonala-parser/src/lib.rs`

- Add `pub mod ast;`
- Add `pub mod parser;`
- Add re-exports for convenience

---

## Parser Implementation Details

### Parser Struct

```rust
pub struct Parser<'src> {
    /// The underlying lexer
    lexer: Lexer<'src>,
    /// The source string (for span extraction)
    source: &'src str,
}
```

### Core Methods

| Method | Description |
|--------|-------------|
| `new(source: &str) -> Parser` | Create parser from source string |
| `parse() -> Result<Vec<Spanned<Ast>>, Error>` | Parse all expressions |
| `parse_one() -> Result<Spanned<Ast>, Error>` | Parse single expression |
| `parse_expr() -> Result<Spanned<Ast>, Error>` | Parse one expression (internal) |

### Parsing Methods (Private)

| Method | Handles |
|--------|---------|
| `parse_list()` | `(` ... `)` |
| `parse_vector()` | `[` ... `]` |
| `parse_map()` | `{` ... `}` |
| `parse_reader_macro(kind)` | `'`, `` ` ``, `~`, `~@` |
| `parse_atom()` | Literals, symbols, keywords |
| `process_string(lexeme)` | Escape sequence processing |

### Parsing Algorithm

Recursive descent parser:

```
parse_expr:
    match peek():
        LeftParen    -> parse_list()
        LeftBracket  -> parse_vector()
        LeftBrace    -> parse_map()
        Quote        -> parse_reader_macro("quote")
        SyntaxQuote  -> parse_reader_macro("syntax-quote")
        Unquote      -> parse_reader_macro("unquote")
        UnquoteSplice-> parse_reader_macro("unquote-splicing")
        Integer/Float/String/Bool/Nil/Symbol/Keyword -> parse_atom()
        RightParen/RightBracket/RightBrace -> error: unexpected closer
        EOF          -> error: unexpected end of input

parse_list:
    consume LeftParen
    elements = []
    while peek() != RightParen and peek() != EOF:
        elements.push(parse_expr())
    expect RightParen
    return List(elements)

parse_reader_macro(symbol_name):
    consume reader macro token
    if peek() == EOF or peek() is closer:
        error: reader macro missing expression
    inner = parse_expr()
    return List([Symbol(symbol_name), inner])
```

---

## File Structure After Implementation

```
crates/lonala-parser/src/
├── lib.rs          # Crate root with exports
├── error.rs        # Error types (extended)
├── token.rs        # Token types (unchanged)
├── lexer.rs        # Lexer (unchanged)
├── ast.rs          # NEW: AST types
└── parser.rs       # NEW: Parser implementation
```

---

## Public API

### New Exports

```rust
// Types
pub use ast::{Ast, Spanned};
pub use parser::Parser;

// Convenience functions
pub use parser::{parse, parse_one};
```

### Usage Examples

```rust
use lonala_parser::{parse, parse_one, Ast};

// Parse multiple expressions
let exprs = parse("(+ 1 2) (- 3 4)")?;
assert_eq!(exprs.len(), 2);

// Parse single expression
let expr = parse_one("'(1 2 3)")?;
// expr.node == Ast::List([
//     Ast::Symbol("quote"),
//     Ast::List([Ast::Integer(1), Ast::Integer(2), Ast::Integer(3)])
// ])

// Access span information
println!("Expression spans bytes {}..{}", expr.span.start, expr.span.end);
```

---

## Test Plan

### Unit Tests (in `ast.rs`)

- AST node construction
- Display formatting
- Equality comparison
- Spanned wrapper

### Unit Tests (in `parser.rs`)

#### Atoms
- Integer literals (decimal, hex, binary, octal, negative)
- Float literals (simple, scientific, special: NaN, Inf, -Inf)
- String literals (simple, with escapes, unicode escapes)
- Boolean literals (true, false)
- Nil literal
- Symbols (simple, operators, namespaced)
- Keywords (simple, namespaced)

#### Collections
- Empty list `()`
- List with elements `(+ 1 2)`
- Nested lists `((a) (b))`
- Empty vector `[]`
- Vector with elements `[1 2 3]`
- Empty map `{}`
- Map with entries `{:a 1 :b 2}`
- Nested collections `{:list (1 2) :vec [3 4]}`

#### Reader Macros
- Quote `'x` → `(quote x)`
- Quote with list `'(1 2 3)`
- Syntax quote `` `x ``
- Unquote `~x`
- Unquote-splicing `~@xs`
- Nested reader macros `''x`

#### Span Tracking
- Single token spans
- Collection spans (include delimiters)
- Nested expression spans

#### Error Cases
- Unexpected EOF in list
- Unexpected EOF in vector
- Unexpected EOF in map
- Mismatched delimiters `(]`
- Odd number of map elements `{:a 1 :b}`
- Reader macro at EOF `'`
- Unexpected closing delimiter `)`

### Integration Tests

- Parse and verify complex expressions
- Round-trip: parse → display → parse → compare

---

## Testing Strategy

Per `docs/development/testing-strategy.md`, this is **Tier 1** testing:

- **100% host-testable** - no QEMU required
- Run with: `cargo test -p lonala-parser`
- All tests run on development machine
- Fast iteration cycle

---

## Acceptance Criteria

The parser is complete when:

1. All token types are correctly parsed into AST nodes
2. Reader macros are expanded to their canonical forms
3. String escape sequences are processed correctly
4. Collections validate structure (balanced delimiters, even map entries)
5. Errors include accurate span information
6. All tests pass
7. `make check` passes (fmt, clippy, tests)
8. Code follows `docs/development/rust-coding-guidelines.md`

---

## Future Considerations

### Phase 2.3 (Bytecode Format)

The AST will be consumed by the bytecode compiler. Consider:
- AST traversal patterns
- Constant pool integration for strings/symbols

### Phase 3.2 (Extended Value Types)

When `Value` gains List/Vector/Map, the compiler may convert AST → Value for evaluation. The AST remains the parsing output.

### Macro Expansion (Phase 4+)

The current parser handles *reader* macros (lexical transformation). *Compile-time* macros will be handled later in the macro expander phase.
