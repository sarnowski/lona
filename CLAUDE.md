# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Lona is a general-purpose operating system combining:
- **seL4 microkernel** - formally verified, capability-based security
- **LISP machine philosophy** - runtime introspection, hot-patching
- **Erlang/OTP concurrency model** - lightweight processes, supervision trees, "let it crash"

The runtime is written in Rust (`no_std`) and runs as seL4's root task. Userspace will be programmed in Lonala, a custom Clojure/Erlang-inspired language.

## Required Reading

**Read when planning new features or tasks:**
- `docs/goals.md` - The complete vision and design philosophy. Consult when making architectural decisions.
- `docs/roadmap/index.md` - The phased roadmap, component dependencies, and current status.
- `docs/lonala/index.md` - The Lonala language specification.
- `docs/minimal-rust.md` - The Lonala-first principle. Consult before adding any native function.

## Required Principle

Always aim for the correct solution. Never take a shortcut, workaround, hack or defer a solution. We always favor the correct solution, even if it is more effort.

## Lonala-First Principle

**Everything achievable in Lonala MUST be implemented in Lonala, not Rust.**

The Rust runtime exists solely to provide the minimal foundation that Lonala cannot provide for itself. This follows the LISP tradition where the entire language is built from a handful of primitives.

### Review Checklist

Before adding ANY native function, ask:

1. Can this be implemented using existing primitives? → **Implement in Lonala**
2. Does this require hardware access? → Native is acceptable
3. Does this require inspecting runtime type tags? → Native is acceptable
4. Is this purely for efficiency? → **Implement in Lonala first**, optimize later only if profiling proves necessary

**When in doubt: Lonala.**

## Directory Structure

```
lona/
├── Cargo.toml                    # Workspace root
├── Makefile                      # Build orchestration (docker, check, build, run, test)
├── CLAUDE.md                     # AI assistant instructions (this file)
│
├── crates/
│   ├── lona-core/                # Foundational types (100% host-testable)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── allocator.rs      # Bump allocator traits
│   │       ├── chunk/            # Bytecode chunk format
│   │       ├── hamt/             # Hash Array Mapped Trie (persistent maps)
│   │       ├── integer/          # Arbitrary-precision integers
│   │       ├── list.rs           # Cons cell lists
│   │       ├── map/              # Persistent hash maps
│   │       ├── opcode/           # VM instruction encoding
│   │       ├── pvec/             # Persistent vectors
│   │       ├── ratio/            # Arbitrary-precision ratios
│   │       ├── source.rs         # Source tracking
│   │       ├── string.rs         # Immutable string type
│   │       ├── symbol.rs         # Interned symbols
│   │       ├── value/            # Core value types
│   │       └── vector.rs         # Vector utilities
│   │
│   ├── lona-kernel/              # VM and runtime abstractions (host-testable)
│   │   └── src/
│   │       ├── lib.rs
│   │       └── vm/               # Bytecode virtual machine
│   │           ├── mod.rs
│   │           ├── collections/  # Collection operations
│   │           ├── error.rs      # Runtime errors
│   │           ├── frame.rs      # Call frames
│   │           ├── globals.rs    # Global variable storage
│   │           ├── helpers.rs    # Interpreter helper functions
│   │           ├── interpreter/  # Bytecode execution
│   │           ├── introspection.rs # Runtime introspection
│   │           ├── macro_expander.rs # Macro expansion
│   │           ├── natives.rs    # Native function registry
│   │           ├── numeric.rs    # Numeric operations
│   │           ├── primitives.rs # Built-in functions
│   │           └── tests/        # VM tests
│   │
│   ├── lona-runtime/             # seL4 root task (QEMU-tested only)
│   │   └── src/
│   │       ├── main.rs           # Entry point, receives bootinfo
│   │       ├── repl.rs           # Interactive REPL (bootstrap)
│   │       ├── integration_tests.rs # QEMU integration tests
│   │       ├── memory/           # seL4 memory management
│   │       └── platform/         # Hardware abstraction (UART, FDT)
│   │
│   ├── lonala-parser/            # Lexer and parser (100% host-testable)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ast.rs            # Abstract syntax tree types
│   │       ├── lexer/            # Tokenizer
│   │       └── parser/           # S-expression parser
│   │
│   ├── lonala-compiler/          # Bytecode compiler (100% host-testable)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs          # Compiler errors
│   │       └── compiler/         # AST to bytecode compiler
│   │
│   ├── lonala-human/             # Human-readable output formatting
│   │   └── src/
│   │
│   ├── lona-spec-tests/          # Language specification tests
│   │   └── src/
│   │       ├── builtins/         # Built-in function tests
│   │       ├── data_types/       # Data type tests
│   │       ├── evaluation.rs     # Evaluation tests
│   │       ├── functions.rs      # Function tests
│   │       ├── literals.rs       # Literal tests
│   │       ├── macros.rs         # Macro tests
│   │       ├── operators.rs      # Operator tests
│   │       ├── reader_macros.rs  # Reader macro tests
│   │       └── special_forms.rs  # Special form tests
│   │
│   └── lona-test/                # Test harness for QEMU tests
│       └── src/
│
├── docs/
│   ├── goals.md                  # Project vision and design philosophy
│   ├── installation.md           # Installation guide for physical hardware
│   ├── license.md                # License information (GPL-3.0)
│   ├── minimal-rust.md           # Lonala-first principle
│   │
│   ├── development/              # Development guidelines
│   │   ├── rust-coding-guidelines.md
│   │   ├── lonala-coding-guidelines.md
│   │   └── testing-strategy.md
│   │
│   ├── lonala/                   # Lonala language specification
│   │   ├── index.md              # Specification table of contents
│   │   ├── introduction.md       # Language overview
│   │   ├── lexical-structure.md  # Tokens, whitespace, comments
│   │   ├── data-types.md         # Type hierarchy
│   │   ├── literals.md           # Literal syntax
│   │   ├── evaluation.md         # Evaluation rules
│   │   ├── special-forms.md      # def, let, if, fn, etc.
│   │   ├── operators.md          # Arithmetic, comparison, bitwise
│   │   ├── functions.md          # Function definition and calling
│   │   ├── reader-macros.md      # Quote, syntax-quote, unquote
│   │   ├── macros.md             # defmacro
│   │   ├── namespaces.md         # Module system (planned)
│   │   ├── concurrency.md        # Process model (planned)
│   │   ├── builtins/             # Built-in function reference
│   │   └── appendices/           # Grammar, bytecode, Clojure differences
│   │
│   └── roadmap/                  # Implementation roadmap
│       ├── index.md              # Roadmap overview and task status
│       └── milestone-*.md        # Individual milestone details
│
├── docker/
│   └── Dockerfile                # Development environment with seL4 SDK
│
├── support/
│   └── targets/
│       └── aarch64-sel4.json     # Custom Rust target for seL4
│
├── lona/                         # Lonala standard library
│   └── core.lona                 # Core macros (defn, when, etc.)
│
└── .claude/                      # Claude Code configuration
    ├── commands/                 # Custom slash commands (git-commit)
    ├── agents/                   # Custom agents (lona-code-reviewer)
    └── skills/                   # Workflow skills (develop-runtime, develop-lona, finishing-work)
```

## Workflows

- **Before writing any Rust code**: Load the `develop-runtime` skill and follow its instructions
- **Before writing any Lonala code**: Load the `develop-lona` skill and follow its instructions
- **When finishing all work**: Load the `finishing-work` skill and follow its instructions

## Test-First Bug Fixing (MANDATORY)

**CRITICAL: ALL bug fixes MUST follow a test-first approach. No exceptions.**

When you encounter a bug—whether reported by the user OR discovered during development—you MUST:

1. **Write a failing test FIRST** that demonstrates the bug
2. **Verify the test fails** against the current (buggy) code
3. **Then fix the bug** to make the test pass
4. **Keep the test** as a permanent regression test

### Why This Matters

- Tests prove the bug exists and is reproducible
- Tests document the expected behavior
- Tests prevent the same bug from recurring
- Tests serve as living documentation of edge cases

### Workflow

The `develop-runtime` skill contains the detailed Bug Fix Workflow (Steps B1-B6). Load it before starting any bug fix work.

### Examples of When This Applies

- User reports: "The parser crashes on empty input" → Write test first
- You notice: "This function doesn't handle negative numbers" → Write test first
- CI reveals: "Test flakes under high load" → Write test first
- Code review finds: "Edge case not handled" → Write test first

### What Counts as a Bug

- Incorrect behavior (code does the wrong thing)
- Missing behavior (code doesn't handle a valid case)
- Crashes or panics on valid input
- Performance regressions (if measurable via tests)
- Security issues (if demonstrable via tests)

**Remember: If you're about to fix something, write a test for it first.**

## Clippy Policy

**CRITICAL: You MUST NOT disable any clippy check at any level (file, module, crate, or workspace).**

A pre-tool hook (`.claude/hooks/check-clippy-directives.py`) automatically blocks any attempt to add `#[allow(...)]` or `#[expect(...)]` directives without proper approval.

When encountering clippy warnings or errors:
1. **Always attempt to fix the issue correctly first**
2. **If the issue cannot be correctly resolved**:
   - Explain the issue to the user in detail
   - Provide your recommendation for how to handle it
   - Wait for the user's EXPLICIT approval before taking any action
3. **Never use `#[allow(...)]`, `#[expect(...)]`, `#[cfg_attr(..., allow(...))]`, `#[cfg_attr(..., expect(...))]`, or clippy.toml to suppress warnings without explicit user approval**
4. **Never add `#![allow(clippy::...)]` or `#![expect(clippy::...)]` to any file**

The user MUST give explicit approval for ANY exception to clippy rules. Do not assume approval.

### Approved Directive Format

When the user explicitly approves a suppression, include the `[approved]` marker in the directive's reason:

```rust
#[expect(clippy::lint_name, reason = "[approved] explanation of why this is needed")]
```

The hook checks for `[approved]` (case-insensitive) in the reason string. Without this marker, the hook will block the operation.

## Build Commands

Requires Docker and GNU Make 4.0+ (on macOS: `brew install make`, use `gmake`).

```bash
make docker          # Build Docker development image (first time only)
make build           # Create bootable QEMU image
make run             # Run in QEMU (includes build)
make test            # Full verification: fmt, clippy, unit tests, build, integration tests
make clean           # Remove build artifacts
make shell           # Interactive Docker shell for debugging
```

## MCP Tools

The following MCP tools are available for interactive testing in the real seL4 environment:

| Tool | Description |
|------|-------------|
| `mcp__lona-dev-repl__eval` | Evaluate Lonala expressions in the REPL running on seL4 in QEMU. Use this to test language features in the actual runtime environment. The REPL maintains state between calls, so defined functions and variables persist across evaluations. |
| `mcp__lona-dev-repl__restart` | Rebuild Lona and restart the QEMU instance with a fresh state. Use this after making changes to Rust code (runtime, kernel, or compiler crates) to ensure the newest version is running. Also use before verification to ensure a clean state. |
