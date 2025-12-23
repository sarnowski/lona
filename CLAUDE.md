# Agent Instructions

This file provides guidance to AI coding agents such as Claude Code, Gemini and Codex when working with code in this repository.

## Project Overview

Lona is an operating system for developers who want full transparency and control over their computing stack. It unifies the strong security of the **seL4 microkernel** with the introspective power of a **LISP machine** and the fault-tolerant concurrency of **Erlang/OTP**. Programmed entirely in **Lonala**, a Clojure-inspired language, Lona lets you inspect, debug, and live-patch every layer of the system—from drivers to applications—without reboots, without opaque binaries, and without sacrificing security.

The runtime is written in Rust (`no_std`) and runs as seL4's root task.

## Mandatory Reading

**You MUST read these files before doing ANY work in this codebase:**

1. @docs/goals/index.md - The complete vision and design philosophy
2. @docs/development/principles.md - The governing development principles
3. @docs/roadmap/index.md - The phased roadmap and current status
4. @docs/lonala/index.md - The Lonala language specification

These four documents provide essential context for understanding the project. Other files linked from these should be read on demand as needed.

## Directory Structure

```
lona/
├── Cargo.toml                    # Workspace root
├── Makefile                      # Build orchestration (docker, check, build, run, test)
├── CLAUDE.md                     # AI agent instructions (this file)
├── AGENTS.md                     # Symlink to CLAUDE.md (for other agents)
├── README.md                     # Project README
├── PLAN.md                       # Current development plan
├── mkdocs.yml                    # Documentation site configuration
├── docker-compose.yml            # Docker Compose configuration
├── rust-toolchain.toml           # Rust toolchain specification
├── requirements.txt              # Python dependencies (for tools)
│
├── crates/
│   ├── lona-core/                # Foundational types (100% host-testable)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── allocator.rs      # Bump allocator traits
│   │       ├── allocator_tests.rs
│   │       ├── binary.rs         # Binary data handling
│   │       ├── chunk/            # Bytecode chunk format
│   │       ├── error_context.rs  # Error context tracking
│   │       ├── fnv.rs            # FNV hash algorithm
│   │       ├── hamt/             # Hash Array Mapped Trie (persistent maps)
│   │       ├── integer/          # Arbitrary-precision integers
│   │       ├── list.rs           # Cons cell lists
│   │       ├── list_tests.rs
│   │       ├── map/              # Persistent hash maps
│   │       ├── meta.rs           # Metadata handling
│   │       ├── opcode/           # VM instruction encoding
│   │       ├── pvec/             # Persistent vectors
│   │       ├── ratio/            # Arbitrary-precision ratios
│   │       ├── set/              # Persistent sets
│   │       ├── source.rs         # Source tracking
│   │       ├── span.rs           # Source spans
│   │       ├── string.rs         # Immutable string type
│   │       ├── symbol.rs         # Interned symbols
│   │       ├── value/            # Core value types
│   │       ├── vector.rs         # Vector utilities
│   │       └── vector_tests.rs
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
│   ├── index.md                  # Documentation homepage
│   ├── installation.md           # Installation guide for physical hardware
│   ├── license.md                # License information (GPL-3.0)
│   │
│   ├── assets/                   # Static assets
│   │   ├── fonts/                # Web fonts
│   │   ├── favicon.svg           # Site favicon
│   │   └── logo.svg              # Lona logo
│   │
│   ├── includes/                 # MkDocs includes
│   │   └── glossary.md           # Shared glossary
│   │
│   ├── overrides/                # MkDocs theme overrides
│   │   └── stylesheets/          # Custom CSS
│   │
│   ├── goals/                    # Project vision and design philosophy
│   │   ├── index.md              # Vision + 4 pillars overview
│   │   ├── pillar-sel4.md        # seL4 security foundation
│   │   ├── pillar-beam.md        # BEAM/OTP resilience
│   │   ├── pillar-lisp-machine.md # LISP machine introspection
│   │   ├── pillar-clojure.md     # Clojure data philosophy
│   │   ├── core-concepts.md      # Unified abstractions
│   │   ├── system-design.md      # Implementation mechanics
│   │   └── non-goals.md          # What we don't build
│   │
│   ├── development/              # Development guidelines
│   │   ├── principles.md         # Governing development principles
│   │   ├── defnative.md          # Native function guidelines
│   │   ├── editor-plan.md        # Editor integration plan
│   │   ├── lisp-machine.md       # LISP machine philosophy
│   │   ├── lonala-coding-guidelines.md
│   │   ├── minimal-rust.md       # Lonala-first principle
│   │   ├── rust-coding-guidelines.md
│   │   ├── tco.md                # Tail call optimization design
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
│   ├── Dockerfile.base           # Base image with seL4 SDK
│   ├── Dockerfile.aarch64        # ARM64 build environment
│   ├── Dockerfile.x86_64         # x86_64 build environment
│   └── Makefile                  # Docker build helpers
│
├── scripts/
│   └── run-integration-tests.sh  # Integration test runner
│
├── support/
│   ├── boot/                     # Boot configuration files
│   │   ├── grub-x86_64.cfg       # GRUB config for x86_64
│   │   ├── rpi4b-boot.txt        # Raspberry Pi 4 boot script
│   │   └── rpi4b-config.txt      # Raspberry Pi 4 config
│   │
│   └── targets/                  # Rust target specifications
│       ├── README.md             # Target documentation
│       ├── aarch64-sel4.json     # ARM64 Rust target for seL4
│       └── x86_64-sel4.json      # x86_64 Rust target for seL4
│
├── tools/
│   ├── lona_dev_repl/            # MCP server for REPL access
│   │   ├── __init__.py
│   │   ├── __main__.py
│   │   ├── repl_manager.py       # QEMU/REPL management
│   │   └── server.py             # MCP server implementation
│   │
│   └── pygments-lonala/          # Syntax highlighter for Lonala
│       └── pyproject.toml
│
├── lona/                         # Lonala standard library
│   └── core.lona                 # Core macros (defn, when, etc.)
│
└── .claude/                      # Claude Code configuration
    ├── commands/                 # Custom slash commands (git-commit, plan-next-task)
    └── skills/                   # Workflow skills (develop-rust, develop-lonala, finishing-work)
```

## Workflows

- **Before writing any Rust code**: Load the `develop-rust` skill and follow its instructions
- **Before writing any Lonala code**: Load the `develop-lonala` skill and follow its instructions
- **When finishing all work**: Load the `finishing-work` skill and follow its instructions

## Build Commands

Requires Docker and GNU Make 4.0+ (on macOS: `brew install make`, use `gmake`).

```bash
make docker          # Build Docker development image (first time only)
make debug-aarch64   # Create bootable aarch64 QEMU image
make run-aarch64     # Run in QEMU (includes build)
make test            # Full verification: fmt, clippy, unit tests, build, integration tests
make clean           # Remove build artifacts
make shell-aarch64   # Interactive Docker shell for debugging
```

**IMPORTANT: To check code quality or whether tests run successful, ALWAYS run `make test`!**

## Command Usage Policy

**Use ONLY the commands documented in this file.** Do not invent, guess, or assume that other commands or make targets exist. If a command is not explicitly listed in this document, do not use it.

If you believe a faster or alternative command might be useful, **ask the user first** before attempting to use it.

## MCP Tools

The following MCP tools are available for interactive testing in the real seL4 environment:

| Tool | Description |
|------|-------------|
| `mcp__lona-dev-repl__eval` | Evaluate Lonala expressions in the REPL running on seL4 in QEMU. Use this to test language features in the actual runtime environment. The REPL maintains state between calls, so defined functions and variables persist across evaluations. |
| `mcp__lona-dev-repl__restart` | Rebuild Lona and restart the QEMU instance with a fresh state. Use this after making changes to Rust code (runtime, kernel, or compiler crates) to ensure the newest version is running. Also use before verification to ensure a clean state. |

## External AI CLI Tools

For code review and analysis, these CLI tools are available:

```bash
# Gemini
timeout 900 gemini -m gemini-3-pro-preview -s "PROMPT"

# Codex (add "-c model_reasoning_effort=LEVEL" where LEVEL is minimal/low/medium/high/xhigh - defaults to medium)
timeout 900 codex exec -m gpt-5.2 -c hide_agent_reasoning=true "PROMPT" 2>/dev/null
```

Both accept prompts as positional arguments. For multiline prompts, use proper shell quoting. Always run with the `timeout 900` to give them sufficient time!

**Important:** When crafting prompts, include references to relevant files (e.g., `docs/goals/index.md`, `docs/development/principles.md`) so the agent knows which documents to read for context.
