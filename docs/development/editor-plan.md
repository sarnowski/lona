# Editor Support Implementation Plan

**Version**: 2.0.0
**Status**: Draft
**Last Updated**: 2025-12-20

This document defines the roadmap for implementing Language Server Protocol (LSP) support and editor integrations for the Lonala programming language.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Architecture](#2-architecture)
3. [Codebase Reusability](#3-codebase-reusability)
4. [Milestone Summary](#4-milestone-summary)
5. [Milestone 0: Foundation Infrastructure](#5-milestone-0-foundation-infrastructure)
6. [Milestone 1: LSP with Semantic Tokens](#6-milestone-1-lsp-with-semantic-tokens)
7. [Milestone 2: Tree-sitter Grammar](#7-milestone-2-tree-sitter-grammar)
8. [Milestone 3: Zed Editor Extension](#8-milestone-3-zed-editor-extension)
9. [Milestone 4: VS Code Extension](#9-milestone-4-vs-code-extension)
10. [Milestone 5: Neovim/Vim Support](#10-milestone-5-neovimvim-support)
11. [Milestone 6: LSP Diagnostics](#11-milestone-6-lsp-diagnostics)
12. [Milestone 7: Hover and Documentation](#12-milestone-7-hover-and-documentation)
13. [Milestone 8: Go to Definition and References](#13-milestone-8-go-to-definition-and-references)
14. [Milestone 9: Document Symbols and Outline](#14-milestone-9-document-symbols-and-outline)
15. [Milestone 10: Code Completion](#15-milestone-10-code-completion)
16. [Milestone 11: Advanced Features](#16-milestone-11-advanced-features)
17. [Build Infrastructure](#17-build-infrastructure)
18. [Testing Strategy](#18-testing-strategy)

---

## 1. Overview

### 1.1 Goals

- Provide first-class editor support for Lonala development
- **Shared infrastructure**: All human-readable output logic shared between LSP and REPL
- Maximize code reuse from existing `lonala-parser` and `lonala-compiler` crates
- Support major editors: Zed, VS Code, Neovim/Vim, and any LSP-compatible editor
- Enable incremental feature delivery with each milestone providing usable functionality

### 1.2 Non-Goals

- The LSP server does NOT run on seL4/Lona (it runs on developer machines)
- No proprietary editor integrations (all open standards)
- No debugger support in initial milestones (future work)

### 1.3 Design Principles

1. **Structured data, not strings**: Parser/compiler return structured errors with spans; human-readable formatting happens in one place
2. **Single source of truth**: The `lonala-human` crate is the only place that generates human-readable messages
3. **Reuse over rewrite**: Leverage existing lexer, parser, and compiler
4. **Thin LSP layer**: The LSP crate is minimal glue between protocol and shared logic
5. **no_std compatible**: Core infrastructure runs on seL4 for the REPL

### 1.4 File Extension

Lonala source files use the `.lona` extension:
- `lona/core.lona` → `lona.core` namespace
- `math/utils.lona` → `math.utils` namespace

---

## 2. Architecture

### 2.1 Layered Architecture

The key architectural insight: **both the REPL and LSP need human-readable output**. Rather than duplicate logic, we create a shared `lonala-human` crate.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Human Interface Endpoints                        │
├─────────────────────────────────────┬───────────────────────────────────┤
│            lonala-lsp               │           lona-runtime            │
│     (Developer machine, std)        │        (seL4, no_std)             │
│                                     │                                   │
│  - LSP protocol translation         │  - REPL loop                      │
│  - Document management              │  - UART I/O                       │
│  - Maps LSP types ↔ internal types  │  - Prints human text              │
│                                     │                                   │
│  (Thin glue - minimal logic)        │  (Uses lonala-human for output)   │
└─────────────────────────────────────┴───────────────────────────────────┘
                          │                         │
                          └───────────┬─────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                    lonala-human (no_std + alloc)                        │
│                                                                         │
│  THE ONLY PLACE THAT GENERATES HUMAN-READABLE TEXT                      │
│                                                                         │
│  - LineIndex: SourceLocation → line:column                              │
│  - format_error(): Error + SourceRegistry → formatted error message     │
│  - format_hover(): SymbolInfo → hover text                              │
│  - format_completion(): CompletionItem → detail text                    │
│  - format_signature(): FnInfo → signature string                        │
│  - special_form_docs(): name → documentation                            │
│  - builtin_docs(): name → documentation                                 │
│                                                                         │
└─────────────────────────────────────┬───────────────────────────────────┘
                                      │
         ┌────────────────────────────┼────────────────────────────────┐
         │                            │                                │
         ▼                            ▼                                ▼
┌─────────────────┐        ┌──────────────────┐        ┌──────────────────┐
│  lonala-parser  │        │ lonala-compiler  │        │    lona-core     │
│                 │        │                  │        │                  │
│  Returns:       │        │  Returns:        │        │  Defines:        │
│  - Tokens       │        │  - CompileError  │        │  - Span          │
│  - Spanned<Ast> │        │    (structured)  │        │  - SourceId      │
│  - ParseError   │        │  - Chunk         │        │  - SourceLocation│
│    (structured) │        │                  │        │  - SourceRegistry│
│  - Comments     │        │                  │        │  - Symbol        │
│    attached to  │        │                  │        │                  │
│    expressions  │        │                  │        │                  │
└─────────────────┘        └──────────────────┘        └──────────────────┘
```

### 2.2 Source Location Tracking

Every token and AST node tracks its source location:

```rust
// lona-core

/// Identifies a source (file, REPL input, etc.)
pub struct SourceId(pub u32);

/// Byte range within a source
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// Complete source location
pub struct SourceLocation {
    pub source: SourceId,
    pub span: Span,
}

/// Metadata about a source
pub struct Source {
    pub name: String,      // "<repl>", "main.lona", "initrd:/init.lona"
    pub content: String,   // The actual source text
}

/// Registry of all sources
pub struct SourceRegistry {
    sources: Vec<Source>,
}
```

**Source name examples:**

| Context | Source Name |
|---------|-------------|
| REPL input | `<repl>` or `<repl:3>` (session line 3) |
| File on disk | `src/main.lona` |
| Initrd file | `initrd:/init.lona` |
| Network (future) | `https://example.com/lib.lona` |
| Macro expansion (future) | `<macro:when>` |
| Eval string (future) | `<eval>` |

### 2.3 Full Span Tracking

Rather than parsing and attaching comments semantically, we track the **full span** from the first leading comment through the end of the expression. This preserves everything exactly as written.

```rust
// lonala-parser

pub struct Spanned<T> {
    pub node: T,
    pub span: Span,           // Just the expression
    pub full_span: Span,      // From first leading comment to end
    pub source: SourceId,
}
```

Example source:
```clojure
; Adds two numbers together.
; Returns the sum of a and b.
(defn add [a b]
  ; Perform addition
  (+ a b))

; Subtracts b from a.
(defn sub [a b]
  (- a b))
```

For `add`:
- `span`: byte range of `(defn add [a b] ...)`
- `full_span`: byte range from `; Adds two numbers` through closing `)`

For `sub`:
- `span`: byte range of `(defn sub [a b] ...)`
- `full_span`: byte range from `; Subtracts b from a.` through closing `)`

**Benefits:**
- No `Vec<Comment>` storage needed - just slice the source
- Preserves blank lines, inline comments, trailing comments
- Exact original formatting preserved
- Simple implementation

**REPL source lookup:**
```clojure
repl> (source add)
; Adds two numbers together.
; Returns the sum of a and b.
(defn add [a b]
  ; Perform addition
  (+ a b))
```

### 2.4 Rich Error Messages (Rust-style)

Error messages show context lines, preserving comments and blank lines exactly as written. This follows Rust's excellent error message design.

**Example: Unmatched Delimiter**
```
error: unmatched delimiter
  --> lib/math.lona:12:8
   |
10 |   ; Calculate the sum
11 |   ; of two numbers
12 |   (+ 1 2])
   |         ^ expected ')' but found ']'
```

**Example: Undefined Symbol with Suggestion**
```
error: undefined symbol 'fooo'
  --> src/main.lona:15:5
   |
13 | ; Helper function for processing
14 | (defn process [x]
15 |   (fooo x))
   |    ^^^^ did you mean 'foo'?
   |
note: 'foo' defined here
  --> src/main.lona:8:1
   |
 8 | (defn foo [x]
   | ^^^^^^^^^^^^^
```

**Example: Wrong Arity with Help**
```
error: wrong number of arguments to 'if' (expected 2-3, got 1)
  --> app/core.lona:24:3
   |
21 | ; Validate user input
22 | ; Returns true if valid
23 | (defn validate [input]
24 |   (if (empty? input)))
   |   ^^^^^^^^^^^^^^^^^^^^ 'if' requires a 'then' branch
   |
help: add a then branch
   |
24 |   (if (empty? input) false)
   |                      +++++
```

### 2.5 Error Message Flow

```
┌──────────────────────────────────────────────────────────────────────┐
│  Parser returns structured error:                                    │
│                                                                      │
│  ParseError {                                                        │
│      kind: UnmatchedDelimiter { expected: ')', found: ']' },         │
│      location: SourceLocation { source: SourceId(2), span: 145..146 }│
│  }                                                                   │
└────────────────────────────────────┬─────────────────────────────────┘
                                     │
                                     ▼
┌──────────────────────────────────────────────────────────────────────┐
│  lonala-human::format_error(&error, &source_registry)                │
│                                                                      │
│  1. Look up source: registry.get(SourceId(2))                        │
│     → Source { name: "lib/math.lona", content: "..." }               │
│                                                                      │
│  2. Build LineIndex, find error position                             │
│     → offset 145 is line 12, column 8                                │
│                                                                      │
│  3. Extract context lines (2 before, 0 after by default)             │
│     → lines 10, 11, 12 with original comments/whitespace             │
│                                                                      │
│  4. Format Rust-style message with:                                  │
│     - Error description                                              │
│     - Source location (file:line:col)                                │
│     - Context lines with line numbers                                │
│     - Underline/caret pointing to error                              │
│     - Optional: suggestions, notes, help                             │
│                                                                      │
└────────────────────────────────────┬─────────────────────────────────┘
                                     │
           ┌─────────────────────────┴─────────────────────────┐
           │                                                   │
           ▼                                                   ▼
┌──────────────────────────────┐             ┌──────────────────────────────┐
│  REPL prints to UART         │             │  LSP creates Diagnostic      │
│                              │             │                              │
│  repl> (+ 1 2])              │             │  Uses same formatted message │
│  error: unmatched delimiter  │             │  from lonala-human           │
│    --> <repl>:1:8            │             │                              │
│     |                        │             │  Diagnostic {                │
│   1 |   (+ 1 2])             │             │      message: "error: ...",  │
│     |         ^ expected ')' │             │      range: ...,             │
│               but found ']'  │             │  }                           │
└──────────────────────────────┘             └──────────────────────────────┘
```

### 2.5 Directory Structure

```
lona/
├── crates/
│   ├── lona-core/                  # (existing, enhanced)
│   │   └── src/
│   │       ├── span.rs             # Span, SourceId, SourceLocation
│   │       ├── source.rs           # Source, SourceRegistry (NEW)
│   │       └── ...
│   │
│   ├── lonala-parser/              # (existing, enhanced)
│   │   └── src/
│   │       ├── ast.rs              # Spanned<T> with span + full_span
│   │       └── ...
│   │
│   ├── lonala-human/               # Human-readable output (NEW)
│   │   ├── Cargo.toml              # no_std + alloc
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── line_index.rs       # Span → line:column conversion
│   │       ├── error.rs            # Error formatting
│   │       ├── hover.rs            # Hover content generation
│   │       ├── completion.rs       # Completion detail text
│   │       ├── signature.rs        # Function signature display
│   │       └── docs.rs             # Special form & builtin documentation
│   │
│   ├── lonala-lsp/                 # LSP server (NEW, std only)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs             # Entry point
│   │       ├── server.rs           # LanguageServer trait (thin)
│   │       ├── document.rs         # Document manager
│   │       └── convert.rs          # LSP types ↔ internal types
│   │
│   ├── lonala-compiler/            # (existing)
│   └── lona-runtime/               # (existing, uses lonala-human)
│
├── tree-sitter-lona/               # Tree-sitter grammar (renamed)
│   ├── grammar.js
│   ├── package.json
│   └── queries/
│       ├── highlights.scm
│       └── ...
│
└── editors/
    ├── vscode/
    │   ├── package.json
    │   ├── language-configuration.json
    │   └── src/extension.ts
    │
    ├── zed/
    │   ├── extension.toml
    │   ├── Cargo.toml
    │   ├── src/lib.rs
    │   └── languages/lona/
    │       └── config.toml
    │
    └── neovim/
        ├── lua/lona/
        └── queries/lona/
```

### 2.6 Component Responsibilities

| Component | Responsibility | no_std |
|-----------|----------------|--------|
| `lona-core` | Source tracking, spans, symbols | Yes |
| `lonala-parser` | Tokenization, parsing, AST with comments | Yes |
| `lonala-compiler` | Compilation, structured errors | Yes |
| `lonala-human` | **All human-readable text generation** (byte offsets) | Yes |
| `lonala-lsp` | LSP protocol, document management, **UTF-16 conversion** | No |
| `lona-runtime` | REPL, uses lonala-human for output | Yes |
| `tree-sitter-lona` | Grammar for Zed/Neovim highlighting | N/A |

**Note on UTF-16**: The LSP protocol requires UTF-16 code units for column positions. This conversion is handled entirely in `lonala-lsp/src/convert.rs`, keeping the core crates (`lonala-human`, `lona-core`) free of UTF-16 concerns and `no_std` compatible.

---

## 3. Codebase Reusability

### 3.1 Data Flow Principle

**Structured data flows down, human text flows up:**

```
Parser/Compiler (structured data)
        │
        ▼
  lonala-human (formats to human text)
        │
        ▼
  REPL / LSP (presents to user)
```

### 3.2 What Each Layer Does

| Layer | Does | Does NOT |
|-------|------|----------|
| **Parser** | Returns `ParseError { kind, location }` | Format error messages |
| **Compiler** | Returns `CompileError { kind, location }` | Format error messages |
| **lonala-human** | Formats all human-readable output | Know about LSP or UART |
| **LSP** | Translates LSP protocol ↔ internal types | Generate message text |
| **REPL** | Reads input, prints output | Generate message text |

### 3.3 Feature Flag Strategy

```toml
# lonala-human/Cargo.toml
[package]
name = "lonala-human"

[features]
default = ["alloc"]
alloc = []
std = ["alloc"]

[dependencies]
lona-core = { path = "../lona-core" }
lonala-parser = { path = "../lonala-parser" }
```

---

## 4. Milestone Summary

| # | Milestone | Deliverables | Dependencies |
|---|-----------|--------------|--------------|
| **0** | **Foundation** | Source tracking, comment capture, lonala-human crate | None |
| 1 | LSP + Semantic Tokens | Working LSP with syntax highlighting | M0 |
| 2 | Tree-sitter Grammar | Syntax highlighting for Zed/Neovim | None |
| 3 | Zed Extension | Full Zed integration | M1, M2 |
| 4 | VS Code Extension | Full VS Code integration | M1 |
| 5 | Neovim/Vim Support | Neovim and Vim integration | M1, M2 |
| 6 | LSP Diagnostics | Real-time error reporting | M1 |
| 7 | Hover | Documentation on hover | M1, M6 |
| 8 | Go to Definition | Navigate to symbol definitions | M1, M6 |
| 9 | Document Symbols | Outline view, breadcrumbs | M1 |
| 10 | Code Completion | Intelligent autocomplete | M1, M6, M8 |
| 11 | Advanced Features | Rename, references, code actions | M1-M10 |

```
Timeline (suggested order):

M0 (Foundation)
 │
 ├──► M1 (LSP) ──► M6 ──► M7 ──► M8 ──► M9 ──► M10 ──► M11
 │         │
 │         └──► M4 (VS Code)
 │
 └──► M2 (Tree-sitter) ──► M3 (Zed)
                │
                └──► M5 (Neovim)
```

---

## 5. Milestone 0: Foundation Infrastructure

**Goal**: Establish the architectural foundation for both REPL and LSP.

**Rationale**: This infrastructure enables consistent human-readable output across all user-facing components. It must be completed before any editor integration.

> **Status (verified 2025-12-20)**: The core foundation work has been completed as part of the **Standardized Error Handling** initiative.
>
> **Completed**:
> - Task 0.1 (Source Tracking) - `SourceId`, `SourceRegistry` implemented
> - Task 0.2 (Parser API) - Parser accepts `SourceId`, errors include `SourceLocation`
> - Task 0.3 (Full Span Tracking) - `Spanned<T>` has `full_span`, parser uses `trivia_start()` (20+ tests)
> - Task 0.4 (Compiler Source Tracking) - Compiler errors include `SourceLocation`
> - Task 0.5 (lonala-human Crate) - Error formatting with context lines
> - Task 0.6 (REPL Integration) - REPL uses `lonala-human` for output
>
> **Remaining**: Task 0.7 (Foundation Tests)

### 5.1 Tasks

#### Task 0.1: Add Source Tracking to lona-core

**Scope**: Implement source identification and registry.

**Files to modify/create**:
- `crates/lona-core/src/source.rs` (NEW)
- `crates/lona-core/src/span.rs` (MODIFY)
- `crates/lona-core/src/lib.rs` (MODIFY)

**Implementation**:
```rust
// crates/lona-core/src/source.rs

use alloc::string::String;
use alloc::vec::Vec;

/// Identifies a source (file, REPL input, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(pub u32);

/// Complete source location (source + byte range)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    pub source: SourceId,
    pub span: Span,
}

impl SourceLocation {
    pub fn new(source: SourceId, span: Span) -> Self {
        Self { source, span }
    }
}

/// Metadata about a source
#[derive(Debug, Clone)]
pub struct Source {
    /// Human-readable name: "<repl>", "main.lona", "initrd:/init.lona"
    pub name: String,
    /// The actual source text
    pub content: String,
}

impl Source {
    pub fn new(name: String, content: String) -> Self {
        Self { name, content }
    }
}

/// Registry of all sources
#[derive(Debug, Default)]
pub struct SourceRegistry {
    sources: Vec<Source>,
}

impl SourceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a source and return its ID
    pub fn add(&mut self, name: String, content: String) -> SourceId {
        let id = SourceId(self.sources.len() as u32);
        self.sources.push(Source::new(name, content));
        id
    }

    /// Get a source by ID
    pub fn get(&self, id: SourceId) -> Option<&Source> {
        self.sources.get(id.0 as usize)
    }

    /// Get source name by ID
    pub fn name(&self, id: SourceId) -> Option<&str> {
        self.get(id).map(|s| s.name.as_str())
    }

    /// Get source content by ID
    pub fn content(&self, id: SourceId) -> Option<&str> {
        self.get(id).map(|s| s.content.as_str())
    }
}
```

**Acceptance criteria**:
- [x] `SourceId`, `SourceLocation`, `Source`, `SourceRegistry` implemented
- [x] All types are `no_std` + `alloc` compatible
- [x] Unit tests for registry operations
- [x] Documentation with examples

---

#### Task 0.2: Update Parser API for Source Tracking

**Scope**: Modify parser to accept `SourceId` and include it in errors.

**Files to modify**:
- `crates/lonala-parser/src/parser/mod.rs`
- `crates/lonala-parser/src/error.rs`

**Key changes**:

```rust
// crates/lonala-parser/src/parser/mod.rs

pub struct Parser<'src> {
    lexer: Lexer<'src>,
    source_id: SourceId,
}

impl<'src> Parser<'src> {
    pub fn new(source: &'src str, source_id: SourceId) -> Self {
        Self {
            lexer: Lexer::new(source),
            source_id,
        }
    }
}

/// Parse source code with source tracking
pub fn parse(source: &str, source_id: SourceId) -> Result<Vec<Spanned<Ast>>, Vec<Error>> {
    let mut parser = Parser::new(source, source_id);
    parser.parse_program()
}
```

```rust
// crates/lonala-parser/src/error.rs

use lona_core::{SourceId, Span};

#[derive(Debug, Clone)]
pub struct Error {
    pub kind: ErrorKind,
    pub span: Span,
    pub source: SourceId,
}

impl Error {
    pub fn new(kind: ErrorKind, span: Span, source: SourceId) -> Self {
        Self { kind, span, source }
    }
}
```

**Acceptance criteria**:
- [x] Parser accepts `SourceId` parameter *(verified in parser/mod.rs)*
- [x] All errors include `SourceId` and `Span` *(verified in error.rs: `Error { kind, location: SourceLocation }`)*
- [x] Existing tests updated and passing *(verified)*
- [x] API change documented *(verified)*

---

#### Task 0.3: Implement Full Span Tracking

**Scope**: Track full span (including leading comments/whitespace) for each expression. This is simpler than attaching parsed comments - we just track byte ranges and slice the source when needed.

**Files to modify**:
- `crates/lonala-parser/src/ast.rs`
- `crates/lonala-parser/src/parser/mod.rs`

**Updated Spanned structure**:
```rust
// crates/lonala-parser/src/ast.rs

use lona_core::{SourceId, SourceRegistry, Span};

/// AST node with source location tracking
pub struct Spanned<T> {
    pub node: T,
    pub span: Span,           // Just the expression
    pub full_span: Span,      // From first leading comment/whitespace to end
    pub source: SourceId,
}

impl<T> Spanned<T> {
    pub fn new(node: T, source: SourceId, span: Span, full_span: Span) -> Self {
        Self { node, span, full_span, source }
    }

    /// Get the full source text including leading comments
    pub fn full_source<'a>(&self, registry: &'a SourceRegistry) -> Option<&'a str> {
        let source = registry.get(self.source)?;
        Some(&source.content[self.full_span.start..self.full_span.end])
    }

    /// Get just the expression source (no leading comments)
    pub fn source_text<'a>(&self, registry: &'a SourceRegistry) -> Option<&'a str> {
        let source = registry.get(self.source)?;
        Some(&source.content[self.span.start..self.span.end])
    }
}
```

**Parser changes**:
```rust
// crates/lonala-parser/src/parser/mod.rs

impl<'src> Parser<'src> {
    /// Parse a form, tracking both expression span and full span
    fn parse_form(&mut self) -> Result<Spanned<Ast>, Error> {
        // Remember position BEFORE skipping whitespace/comments
        let full_start = self.current_position();

        // Skip whitespace and comments
        self.skip_trivia();

        // Parse the actual expression
        let expr_start = self.current_position();
        let node = self.parse_expr()?;
        let expr_end = self.current_position();

        Ok(Spanned {
            node,
            span: Span::new(expr_start, expr_end),
            full_span: Span::new(full_start, expr_end),
            source: self.source_id,
        })
    }
}
```

**Example behavior**:
```clojure
; Header comment
; More details
(defn foo [x]
  ; inline comment
  x)

; Next function docs
(defn bar [] nil)
```

For `foo`:
- `span`: byte range of `(defn foo [x] ...)`
- `full_span`: from `; Header comment` through closing `)`
- `full_source()`: returns the entire block including comments

For `bar`:
- `span`: byte range of `(defn bar [] nil)`
- `full_span`: from `; Next function docs` through closing `)`

**REPL source lookup**:
```clojure
repl> (source foo)
; Header comment
; More details
(defn foo [x]
  ; inline comment
  x)
```

**Acceptance criteria**:
- [x] `Spanned<T>` has both `span` and `full_span` fields *(implemented in ast.rs)*
- [x] Parser tracks full span including leading whitespace/comments *(via `trivia_start()` and `spanned_with_trivia()`)*
- [x] `full_source()` returns the complete text including comments *(verified)*
- [x] Inline comments and blank lines preserved exactly *(verified)*
- [x] Existing tests updated for new Spanned structure *(verified)*
- [x] Unit tests for full span tracking *(20+ tests in `parser/tests/full_span_tests.rs`)*

---

#### Task 0.4: Update Compiler for Source Tracking

**Scope**: Ensure compiler preserves and uses `SourceLocation`.

**Files to modify**:
- `crates/lonala-compiler/src/error.rs`
- `crates/lonala-compiler/src/compiler/mod.rs`

**Error structure**:
```rust
// crates/lonala-compiler/src/error.rs

use lona_core::SourceLocation;

#[derive(Debug, Clone)]
pub struct Error {
    pub kind: ErrorKind,
    pub location: SourceLocation,
}

impl Error {
    pub fn new(kind: ErrorKind, location: SourceLocation) -> Self {
        Self { kind, location }
    }
}
```

**Acceptance criteria**:
- [x] All compiler errors include `SourceLocation`
- [x] Existing tests updated and passing
- [x] Error kinds remain structured (no human text)

---

#### Task 0.5: Create lonala-human Crate

**Scope**: Create the crate that generates all human-readable output.

**Files to create**:
- `crates/lonala-human/Cargo.toml`
- `crates/lonala-human/src/lib.rs`
- `crates/lonala-human/src/line_index.rs`
- `crates/lonala-human/src/error.rs`
- `crates/lonala-human/src/docs.rs`

**Cargo.toml**:
```toml
[package]
name = "lonala-human"
version = "0.1.0"
edition = "2021"
description = "Human-readable output formatting for Lonala"
license = "MIT OR Apache-2.0"

[features]
default = ["alloc"]
alloc = []
std = ["alloc"]

[dependencies]
lona-core = { path = "../lona-core" }
lonala-parser = { path = "../lonala-parser" }
lonala-compiler = { path = "../lonala-compiler" }
```

**Line index implementation**:
```rust
// crates/lonala-human/src/line_index.rs

use alloc::vec::Vec;
use lona_core::Span;

/// Line and column position (0-indexed)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineCol {
    pub line: u32,
    pub column: u32,
}

/// Index for converting byte offsets to line/column
pub struct LineIndex {
    /// Byte offset of the start of each line
    line_starts: Vec<usize>,
}

impl LineIndex {
    /// Build a line index from source text
    pub fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, c) in text.char_indices() {
            if c == '\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    /// Convert byte offset to line/column
    pub fn offset_to_line_col(&self, offset: usize) -> LineCol {
        let line = self.line_starts
            .binary_search(&offset)
            .unwrap_or_else(|i| i.saturating_sub(1));
        let line_start = self.line_starts[line];
        let column = offset - line_start;
        LineCol {
            line: line as u32,
            column: column as u32,
        }
    }

    /// Convert line/column to byte offset
    pub fn line_col_to_offset(&self, line_col: LineCol) -> usize {
        let line_start = self.line_starts
            .get(line_col.line as usize)
            .copied()
            .unwrap_or(0);
        line_start + line_col.column as usize
    }

    /// Get the content of a specific line
    pub fn line_content<'a>(&self, text: &'a str, line: u32) -> &'a str {
        let start = self.line_starts.get(line as usize).copied().unwrap_or(0);
        let end = self.line_starts
            .get(line as usize + 1)
            .copied()
            .unwrap_or(text.len());
        // Trim trailing newline
        text[start..end].trim_end_matches('\n').trim_end_matches('\r')
    }

    /// Number of lines
    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
}
```

**Error formatting (Rust-style with context)**:
```rust
// crates/lonala-human/src/error.rs

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use lona_core::{SourceId, SourceRegistry, Span};
use lonala_parser::error::Error as ParseError;
use lonala_compiler::error::Error as CompileError;
use crate::line_index::LineIndex;

/// Configuration for error formatting
pub struct ErrorConfig {
    /// Number of context lines before the error
    pub context_lines_before: u32,
    /// Number of context lines after the error
    pub context_lines_after: u32,
}

impl Default for ErrorConfig {
    fn default() -> Self {
        Self {
            context_lines_before: 2,
            context_lines_after: 0,
        }
    }
}

/// Format a parse error with source context (Rust-style)
pub fn format_parse_error(
    error: &ParseError,
    registry: &SourceRegistry,
) -> String {
    format_error(
        &error.kind.to_string(),
        error.span,
        error.source,
        registry,
        &ErrorConfig::default(),
    )
}

/// Format a compile error with source context
pub fn format_compile_error(
    error: &CompileError,
    registry: &SourceRegistry,
) -> String {
    format_error(
        &error.kind.to_string(),
        error.span,
        error.source,
        registry,
        &ErrorConfig::default(),
    )
}

/// Format an error with Rust-style context lines
pub fn format_error(
    message: &str,
    span: Span,
    source_id: SourceId,
    registry: &SourceRegistry,
    config: &ErrorConfig,
) -> String {
    let source = match registry.get(source_id) {
        Some(s) => s,
        None => return format!("error: {}", message),
    };

    let index = LineIndex::new(&source.content);
    let start_pos = index.offset_to_line_col(span.start);
    let end_pos = index.offset_to_line_col(span.end);

    // Calculate line range to display
    let first_line = start_pos.line.saturating_sub(config.context_lines_before);
    let last_line = (end_pos.line + config.context_lines_after)
        .min(index.line_count() as u32 - 1);

    // Width for line numbers
    let line_num_width = (last_line + 1).to_string().len();

    let mut output = String::new();

    // Error header
    output.push_str(&format!("error: {}\n", message));
    output.push_str(&format!(
        "  --> {}:{}:{}\n",
        source.name,
        start_pos.line + 1,
        start_pos.column + 1
    ));

    // Blank line with gutter
    output.push_str(&format!("{:width$} |\n", "", width = line_num_width));

    // Context and error lines
    for line in first_line..=last_line {
        let content = index.line_content(&source.content, line);
        output.push_str(&format!(
            "{:>width$} | {}\n",
            line + 1,
            content,
            width = line_num_width
        ));

        // Add underline on the error line
        if line == start_pos.line {
            let underline_start = start_pos.column as usize;
            let underline_len = if start_pos.line == end_pos.line {
                (end_pos.column - start_pos.column).max(1) as usize
            } else {
                content.len().saturating_sub(underline_start).max(1)
            };

            output.push_str(&format!(
                "{:width$} | {}{}\n",
                "",
                " ".repeat(underline_start),
                "^".repeat(underline_len),
                width = line_num_width
            ));
        }
    }

    output
}
```

**Example output** (showing comments are preserved in context):
```
error: undefined symbol 'fooo'
  --> src/main.lona:15:5
   |
13 | ; Helper function for processing
14 | (defn process [x]
15 |   (fooo x))
   |    ^^^^
```

**Documentation**:
```rust
// crates/lonala-human/src/docs.rs

use alloc::string::String;

/// Get documentation for a special form
pub fn special_form_doc(name: &str) -> Option<&'static str> {
    match name {
        "def" => Some(
            "Binds a value to a global variable.\n\n\
             Syntax: (def name value)\n\n\
             Returns: The symbol name"
        ),
        "let" => Some(
            "Creates local bindings for a body of expressions.\n\n\
             Syntax: (let [bindings*] body*)\n\n\
             Bindings are evaluated left-to-right. Each binding can \
             refer to previously bound names."
        ),
        "fn" => Some(
            "Creates a function.\n\n\
             Syntax: (fn name? [params*] body*)\n\n\
             The optional name enables recursion and debugging."
        ),
        "if" => Some(
            "Conditional branching.\n\n\
             Syntax: (if test then else?)\n\n\
             Evaluates test. If truthy, evaluates and returns then. \
             Otherwise evaluates and returns else (or nil if omitted)."
        ),
        "do" => Some(
            "Sequential execution of multiple expressions.\n\n\
             Syntax: (do exprs*)\n\n\
             Returns the value of the last expression, or nil if empty."
        ),
        "quote" => Some(
            "Returns its argument unevaluated.\n\n\
             Syntax: (quote form) or 'form\n\n\
             Prevents evaluation, returning the form as data."
        ),
        "defmacro" => Some(
            "Defines a macro.\n\n\
             Syntax: (defmacro name [params*] body+)\n\n\
             Macros receive unevaluated arguments and return code \
             to be compiled in their place."
        ),
        "syntax-quote" => Some(
            "Template quoting with unquote support.\n\n\
             Syntax: `form\n\n\
             Like quote, but allows selective evaluation with ~ (unquote) \
             and ~@ (unquote-splicing)."
        ),
        _ => None,
    }
}

/// Get documentation for a built-in function
pub fn builtin_doc(name: &str) -> Option<&'static str> {
    match name {
        "print" => Some(
            "Prints values to output.\n\n\
             Syntax: (print args*)\n\n\
             Prints each argument separated by spaces, followed by newline. \
             Returns nil."
        ),
        // Future built-ins...
        _ => None,
    }
}

/// Check if a symbol is a special form
pub fn is_special_form(name: &str) -> bool {
    matches!(name,
        "def" | "defmacro" | "let" | "fn" | "if" | "do" |
        "quote" | "syntax-quote" | "unquote" | "unquote-splicing"
    )
}

/// Check if a symbol is a built-in function
pub fn is_builtin(name: &str) -> bool {
    matches!(name, "print")
}
```

**Acceptance criteria**:
- [x] `lonala-human` crate created with `no_std` + `alloc` support
- [x] `LineIndex` correctly converts offsets to line/column
- [x] `format_parse_error` produces properly formatted error messages
- [x] `format_compile_error` produces properly formatted error messages
- [ ] Documentation functions return correct content *(partially done)*
- [x] Unit tests for all formatting functions

---

#### Task 0.6: Update REPL to Use lonala-human

**Scope**: Modify the REPL to use `lonala-human` for all output.

**Files to modify**:
- `crates/lona-runtime/src/repl.rs`
- `crates/lona-runtime/Cargo.toml`

**Example usage**:
```rust
// In REPL error handling
use lonala_human::error::format_parse_error;

fn handle_parse_error(&self, error: ParseError) {
    let message = format_parse_error(&error, &self.source_registry);
    self.print_line(&message);
}
```

**Acceptance criteria**:
- [x] REPL uses `lonala-human` for error messages
- [x] Error messages include source location (`<repl>:1:5`)
- [x] Error messages include source context and underline
- [x] All existing REPL functionality preserved

---

#### Task 0.7: Add Foundation Tests

**Scope**: Comprehensive tests for the foundation infrastructure.

**Test files**:
- `crates/lona-core/src/source.rs` (unit tests)
- `crates/lonala-human/src/line_index.rs` (unit tests)
- `crates/lonala-human/src/error.rs` (unit tests)
- `crates/lonala-parser/tests/full_span.rs` (integration tests)

**Test scenarios**:
1. Source registry add/get operations
2. Line index with empty file, single line, multiple lines
3. Line index with unicode characters
4. Error formatting with context lines (comments preserved)
5. Full span tracking includes leading comments and blank lines
6. `full_source()` returns exact original text

**Acceptance criteria**:
- [ ] All tests pass
- [ ] Edge cases covered (empty input, unicode, CRLF line endings)
- [ ] Error messages match expected Rust-style format
- [ ] Tests run in both `std` and `no_std` environments

---

### 5.2 Milestone 0 Deliverables

- [x] Source tracking in `lona-core` (`SourceId`, `SourceRegistry`)
- [x] Parser updated with `full_span` tracking *(Task 0.3 - verified 2025-12-20)*
- [x] Compiler updated for source tracking
- [x] `lonala-human` crate with error formatting and documentation
- [x] REPL updated to use `lonala-human`
- [ ] Comprehensive test coverage

---

## 6. Milestone 1: LSP with Semantic Tokens

**Goal**: Create a minimal LSP server with syntax highlighting.

**Rationale**: Semantic tokens provide immediate visual feedback and establish the LSP infrastructure. The LSP is a thin layer that delegates to `lonala-human` for any text generation.

### 6.1 Tasks

#### Task 1.1: Create lonala-lsp Crate Structure

**Scope**: Set up the LSP crate with dependencies.

**Files to create**:
- `crates/lonala-lsp/Cargo.toml`
- `crates/lonala-lsp/src/main.rs`
- `crates/lonala-lsp/src/lib.rs`

**Cargo.toml**:
```toml
[package]
name = "lonala-lsp"
version = "0.1.0"
edition = "2021"
description = "Language Server Protocol implementation for Lonala"
license = "MIT OR Apache-2.0"

[[bin]]
name = "lonala-lsp"
path = "src/main.rs"

[dependencies]
# LSP framework
tower-lsp = "0.20"
tokio = { version = "1", features = ["full"] }

# Lonala crates (with std features enabled)
lona-core = { path = "../lona-core", features = ["std"] }
lonala-parser = { path = "../lonala-parser", features = ["std"] }
lonala-compiler = { path = "../lonala-compiler", features = ["std"] }
lonala-human = { path = "../lonala-human", features = ["std"] }

# Utilities
dashmap = "6"
log = "0.4"
env_logger = "0.11"

[dev-dependencies]
tokio-test = "0.4"
```

**Acceptance criteria**:
- [ ] Crate compiles successfully
- [ ] Dependencies resolve correctly
- [ ] `cargo build -p lonala-lsp` produces binary

---

#### Task 1.2: Implement Document Manager

**Scope**: Track open documents with source registry integration.

**File**: `crates/lonala-lsp/src/document.rs`

```rust
use std::sync::Arc;
use dashmap::DashMap;
use tower_lsp::lsp_types::Url;
use lona_core::{SourceId, SourceRegistry};
use lonala_human::line_index::LineIndex;
use parking_lot::RwLock;

/// A managed document
pub struct Document {
    pub source_id: SourceId,
    pub content: String,
    pub line_index: LineIndex,
    pub version: i32,
}

/// Manages all open documents
pub struct DocumentManager {
    documents: DashMap<Url, Document>,
    registry: Arc<RwLock<SourceRegistry>>,
}

impl DocumentManager {
    pub fn new() -> Self {
        Self {
            documents: DashMap::new(),
            registry: Arc::new(RwLock::new(SourceRegistry::new())),
        }
    }

    pub fn registry(&self) -> Arc<RwLock<SourceRegistry>> {
        self.registry.clone()
    }

    pub fn open(&self, uri: Url, content: String, version: i32) {
        let source_id = {
            let mut registry = self.registry.write();
            registry.add(uri.to_string(), content.clone())
        };
        let line_index = LineIndex::new(&content);
        self.documents.insert(uri, Document {
            source_id,
            content,
            line_index,
            version,
        });
    }

    pub fn update(&self, uri: &Url, content: String, version: i32) {
        if let Some(mut doc) = self.documents.get_mut(uri) {
            // Update document content in place (no new SourceId allocation).
            // The DocumentManager owns the source content; we only use
            // SourceRegistry for parsing/compiling when needed.
            doc.content = content.clone();
            doc.line_index = LineIndex::new(&content);
            doc.version = version;
        }
    }

    pub fn close(&self, uri: &Url) {
        self.documents.remove(uri);
    }

    pub fn get(&self, uri: &Url) -> Option<dashmap::mapref::one::Ref<Url, Document>> {
        self.documents.get(uri)
    }
}
```

**Acceptance criteria**:
- [ ] Thread-safe document storage
- [ ] Integrates with `SourceRegistry`
- [ ] Line index updated on content change

---

#### Task 1.3: Implement LSP ↔ Internal Type Conversion

**Scope**: Convert between LSP types and internal Lonala types, including UTF-16 position encoding.

**File**: `crates/lonala-lsp/src/convert.rs`

**Important**: The LSP protocol uses UTF-16 code units for column positions. The `LineIndex` in `lonala-human` uses byte offsets (which is correct for `no_std` REPL usage). The UTF-16 conversion is handled entirely in the LSP layer.

```rust
use tower_lsp::lsp_types::{Position, Range};
use lona_core::Span;
use lonala_human::line_index::{LineIndex, LineCol};

/// Convert byte column to UTF-16 code unit column.
///
/// LSP uses UTF-16 code units for character positions. This scans the
/// line content to count UTF-16 code units up to the byte offset.
fn byte_col_to_utf16(line_content: &str, byte_col: usize) -> u32 {
    line_content
        .get(..byte_col.min(line_content.len()))
        .unwrap_or("")
        .chars()
        .map(|c| c.len_utf16() as u32)
        .sum()
}

/// Convert UTF-16 column to byte column.
fn utf16_col_to_byte(line_content: &str, utf16_col: u32) -> usize {
    let mut utf16_count = 0u32;
    let mut byte_offset = 0usize;
    for c in line_content.chars() {
        if utf16_count >= utf16_col {
            break;
        }
        utf16_count += c.len_utf16() as u32;
        byte_offset += c.len_utf8();
    }
    byte_offset
}

/// Convert Span to LSP Range using a LineIndex.
///
/// Handles UTF-16 encoding required by LSP.
pub fn span_to_range(span: Span, index: &LineIndex, source: &str) -> Range {
    let start_lc = index.offset_to_line_col(span.start).unwrap_or_default();
    let end_lc = index.offset_to_line_col(span.end).unwrap_or_default();

    let start_line_content = index.line_content(source, start_lc.line).unwrap_or("");
    let end_line_content = index.line_content(source, end_lc.line).unwrap_or("");

    Range {
        start: Position {
            line: start_lc.line,
            character: byte_col_to_utf16(start_line_content, start_lc.column as usize),
        },
        end: Position {
            line: end_lc.line,
            character: byte_col_to_utf16(end_line_content, end_lc.column as usize),
        },
    }
}

/// Convert LSP Range to Span using a LineIndex.
///
/// Handles UTF-16 encoding required by LSP.
pub fn range_to_span(range: Range, index: &LineIndex, source: &str) -> Span {
    let start_line_content = index.line_content(source, range.start.line).unwrap_or("");
    let end_line_content = index.line_content(source, range.end.line).unwrap_or("");

    let start_byte_col = utf16_col_to_byte(start_line_content, range.start.character);
    let end_byte_col = utf16_col_to_byte(end_line_content, range.end.character);

    let start_offset = index.line_start(range.start.line).unwrap_or(0) + start_byte_col;
    let end_offset = index.line_start(range.end.line).unwrap_or(0) + end_byte_col;

    Span::new(start_offset, end_offset)
}
```

**Acceptance criteria**:
- [ ] Bidirectional conversion works correctly
- [ ] UTF-16 conversion handles multi-byte characters (emoji, CJK, etc.)
- [ ] Unit tests for conversion functions including Unicode edge cases

---

#### Task 1.4: Implement Semantic Tokens Provider

**Scope**: Generate semantic tokens using the lexer.

**File**: `crates/lonala-lsp/src/semantic_tokens.rs`

```rust
use tower_lsp::lsp_types::{
    SemanticToken, SemanticTokenType, SemanticTokensLegend,
    SemanticTokensOptions, SemanticTokensFullOptions,
};
use lonala_parser::{Lexer, TokenKind};
use lonala_human::docs::{is_special_form, is_builtin};
use crate::document::Document;
use crate::convert::line_col_to_position;

/// Token type indices (must match legend order)
pub mod types {
    pub const KEYWORD: u32 = 0;
    pub const FUNCTION: u32 = 1;
    pub const VARIABLE: u32 = 2;
    pub const NUMBER: u32 = 3;
    pub const STRING: u32 = 4;
    pub const ENUM_MEMBER: u32 = 5;  // Keywords like :foo
    pub const COMMENT: u32 = 6;
    pub const OPERATOR: u32 = 7;
}

pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::NUMBER,
            SemanticTokenType::STRING,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::COMMENT,
            SemanticTokenType::OPERATOR,
        ],
        token_modifiers: vec![],
    }
}

pub fn options() -> SemanticTokensOptions {
    SemanticTokensOptions {
        legend: legend(),
        full: Some(SemanticTokensFullOptions::Bool(true)),
        ..Default::default()
    }
}

pub fn compute(doc: &Document) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    let lexer = Lexer::new(&doc.content);

    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for token in lexer {
        let token_type = classify(&token, &doc.content);

        if let Some(token_type) = token_type {
            let pos = doc.line_index.offset_to_line_col(token.span.start);
            let length = (token.span.end - token.span.start) as u32;

            let delta_line = pos.line - prev_line;
            let delta_start = if delta_line == 0 {
                pos.column - prev_start
            } else {
                pos.column
            };

            tokens.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type,
                token_modifiers_bitset: 0,
            });

            prev_line = pos.line;
            prev_start = pos.column;
        }
    }

    tokens
}

fn classify(token: &lonala_parser::Token, _source: &str) -> Option<u32> {
    match token.kind {
        TokenKind::Comment => Some(types::COMMENT),
        TokenKind::String => Some(types::STRING),
        TokenKind::Integer | TokenKind::Float => Some(types::NUMBER),
        TokenKind::True | TokenKind::False | TokenKind::Nil => Some(types::KEYWORD),
        TokenKind::Keyword => Some(types::ENUM_MEMBER),
        TokenKind::Quote | TokenKind::SyntaxQuote |
        TokenKind::Unquote | TokenKind::UnquoteSplice => Some(types::OPERATOR),
        TokenKind::Symbol => {
            let lexeme = token.lexeme;
            if is_special_form(lexeme) {
                Some(types::KEYWORD)
            } else if is_builtin(lexeme) {
                Some(types::FUNCTION)
            } else {
                Some(types::VARIABLE)
            }
        }
        _ => None,  // Delimiters, etc.
    }
}
```

**Acceptance criteria**:
- [ ] All token types correctly classified
- [ ] Delta encoding is correct
- [ ] Uses `lonala-human` for special form/builtin detection

---

#### Task 1.5: Implement LSP Server

**Scope**: Implement the `LanguageServer` trait.

**File**: `crates/lonala-lsp/src/server.rs`

```rust
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::document::DocumentManager;
use crate::semantic_tokens;

pub struct LonalaServer {
    client: Client,
    documents: DocumentManager,
}

impl LonalaServer {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: DocumentManager::new(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LonalaServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        semantic_tokens::options()
                    )
                ),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "lonala-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        self.documents.open(doc.uri, doc.text, doc.version);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents.update(
                &params.text_document.uri,
                change.text,
                params.text_document.version,
            );
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.close(&params.text_document.uri);
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;

        let tokens = self.documents.get(&uri).map(|doc| {
            semantic_tokens::compute(&doc)
        });

        Ok(tokens.map(|data| {
            SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data,
            })
        }))
    }
}
```

**Acceptance criteria**:
- [ ] Server initializes with correct capabilities
- [ ] Document sync works
- [ ] Semantic tokens returned
- [ ] Server shuts down cleanly

---

#### Task 1.6: Implement Server Entry Point

**Scope**: Create the main binary.

**File**: `crates/lonala-lsp/src/main.rs`

```rust
use tower_lsp::{LspService, Server};
use lonala_lsp::server::LonalaServer;

#[tokio::main]
async fn main() {
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| LonalaServer::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}
```

**Acceptance criteria**:
- [ ] Binary starts and listens on stdio
- [ ] Responds to initialize request
- [ ] Logging configurable via `RUST_LOG`

---

#### Task 1.7: Add Makefile Targets

**Scope**: Add build targets for LSP.

**File**: `Makefile` (additions)

```makefile
# =============================================================================
# LSP Targets (uses local cargo, not Docker)
# =============================================================================

.PHONY: lsp lsp-test lsp-build lsp-install lsp-clean

lsp: lsp-test lsp-build  ## Build LSP server (test + release binary)

lsp-test:  ## Run LSP tests
	cargo test -p lonala-human
	cargo test -p lonala-lsp

lsp-build:  ## Build LSP release binary
	cargo build --release -p lonala-lsp
	@echo ""
	@echo "LSP binary built: target/release/lonala-lsp"

lsp-install:  ## Install LSP to ~/.cargo/bin
	cargo install --path crates/lonala-lsp

lsp-clean:  ## Clean LSP build artifacts
	cargo clean -p lonala-lsp
	cargo clean -p lonala-human
```

**Acceptance criteria**:
- [ ] `make lsp` runs tests and builds binary
- [ ] `make lsp-install` installs to PATH

---

#### Task 1.8: Add LSP Tests

**Scope**: Integration tests for the LSP.

**File**: `crates/lonala-lsp/tests/integration.rs`

**Test scenarios**:
1. Initialize/shutdown handshake
2. Document open/change/close
3. Semantic tokens for simple code
4. Semantic tokens for all token types

**Acceptance criteria**:
- [ ] All tests pass
- [ ] Tests cover implemented functionality

---

### 6.2 Milestone 1 Deliverables

- [ ] Working `lonala-lsp` binary
- [ ] Semantic token highlighting
- [ ] Document synchronization
- [ ] Integration with `lonala-human` for text generation
- [ ] Test suite
- [ ] Makefile targets

---

## 7. Milestone 2: Tree-sitter Grammar

**Goal**: Create a Tree-sitter grammar for Lonala.

**Note**: File extension is `.lona`, grammar name is `lona`.

### 7.1 Tasks

#### Task 2.1: Initialize Tree-sitter Project

**Directory**: `tree-sitter-lona/`

**package.json**:
```json
{
  "name": "tree-sitter-lona",
  "version": "0.1.0",
  "description": "Tree-sitter grammar for Lonala (Lona programming language)",
  "main": "bindings/node",
  "license": "MIT",
  "tree-sitter": [
    {
      "scope": "source.lona",
      "file-types": ["lona"],
      "injection-regex": "lona"
    }
  ]
}
```

---

#### Task 2.2: Implement Core Grammar

**File**: `tree-sitter-lona/grammar.js`

(Same as before, but with `name: 'lona'`)

---

#### Task 2.3-2.6: Create Queries and Tests

(Same as before, but using `.lona` extension and `lona` name)

---

### 7.2 Milestone 2 Deliverables

- [ ] Complete Tree-sitter grammar for Lonala
- [ ] Highlight queries
- [ ] Test corpus
- [ ] Makefile targets

---

## 8-16: Remaining Milestones

*The remaining milestones (3-11) follow the same structure as before, with these key differences:*

1. **File extension**: Use `.lona` instead of `.lonala`
2. **Grammar name**: Use `lona` instead of `lonala`
3. **LSP capabilities**: Call `lonala-human` functions for all text generation
4. **Hover content**: Use `lonala_human::docs::special_form_doc()` and `builtin_doc()`
5. **Error formatting**: Use `lonala_human::error::format_parse_error()` etc.

### Key Changes in Later Milestones

#### Milestone 6 (Diagnostics)

```rust
// In lonala-lsp diagnostic handling
use lonala_human::error::format_parse_error;

fn create_diagnostic(error: &ParseError, doc: &Document) -> Diagnostic {
    // Use lonala-human for message text
    let message = format_parse_error(error, &self.registry());

    Diagnostic {
        range: span_to_range(error.location.span, &doc.line_index),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("lonala".to_string()),
        message,  // From lonala-human
        ..Default::default()
    }
}
```

#### Milestone 7 (Hover)

```rust
// In lonala-lsp hover handling
use lonala_human::docs::{special_form_doc, builtin_doc};

fn hover_for_symbol(name: &str) -> Option<String> {
    special_form_doc(name)
        .or_else(|| builtin_doc(name))
        .map(|s| s.to_string())
}
```

---

## 17. Build Infrastructure

### 17.1 Makefile Targets Summary

```makefile
# LSP targets (local cargo, not Docker)
lsp              # Test and build LSP
lsp-test         # Run lonala-human and LSP tests
lsp-build        # Build release binary
lsp-install      # Install to ~/.cargo/bin

# Tree-sitter targets
tree-sitter      # Generate and test grammar
tree-sitter-generate
tree-sitter-test

# Editor targets
editors          # Build all editors
editors-zed      # Build Zed extension
editors-vscode   # Build VS Code extension
```

---

## 18. Testing Strategy

### 18.1 Unit Tests

- `lona-core`: Source registry, span operations
- `lonala-human`: Line index, error formatting, documentation
- `lonala-lsp`: Type conversion, semantic tokens

### 18.2 Integration Tests

- Parser with source tracking
- Compiler with source tracking
- Full LSP protocol tests

### 18.3 Shared Test Cases

Since `lonala-human` is shared between REPL and LSP, tests should verify identical output:

```rust
#[test]
fn error_message_consistency() {
    let error = /* ... */;
    let registry = /* ... */;

    let message = format_parse_error(&error, &registry);

    // Same message should appear in REPL and LSP
    assert!(message.contains("error:"));
    assert!(message.contains("-->"));
}
```

---

## Appendix A: LSP Capability Matrix

| Capability | Milestone | Uses lonala-human |
|------------|-----------|-------------------|
| Semantic Tokens | M1 | docs::is_special_form |
| Diagnostics | M6 | error::format_* |
| Hover | M7 | docs::*_doc |
| Completion | M10 | completion::* |
| Signature Help | M11 | signature::* |

---

## Appendix B: File Extension Summary

| Context | Extension | Example |
|---------|-----------|---------|
| Source files | `.lona` | `main.lona` |
| Namespace path | → | `lona/core.lona` → `lona.core` |
| Tree-sitter | `lona` | `tree-sitter-lona` |
| Language ID | `lona` | VS Code, Zed, etc. |

---

## Appendix C: References

- [LSP Specification 3.17](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/)
- [tower-lsp Documentation](https://docs.rs/tower-lsp)
- [Tree-sitter Documentation](https://tree-sitter.github.io/tree-sitter/)
- [Zed Extension API](https://zed.dev/docs/extensions)
- [VS Code Language Extensions](https://code.visualstudio.com/api/language-extensions/overview)
- [Neovim LSP Documentation](https://neovim.io/doc/user/lsp.html)
