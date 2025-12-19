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
- `docs/development/implementation-plan.md` - The phased roadmap, component dependencies, and current status.
- `docs/development/minimal-rust.md` - The Lonala-first principle. Consult before adding any native function.

## Required Principle

Always aim for the correct solution. Never take a shortcut, workaround, hack or defer a solution. We always favor the correct solution, even if it is more effort.

## Lonala-First Principle

**Everything achievable in Lonala MUST be implemented in Lonala, not Rust.**

The Rust runtime exists solely to provide the minimal foundation that Lonala cannot provide for itself. This follows the LISP tradition where the entire language is built from a handful of primitives.

### What MUST Be Native (Rust)

Only these categories require Rust implementation:

| Category | Primitives | Why Native |
|----------|-----------|------------|
| **Cons cell operations** | `cons`, `first`, `rest` | Fundamental data structure |
| **Type predicates** | `nil?`, `symbol?`, `list?`, `fn?`, etc. | Inspect runtime type tags |
| **Equality** | `eq?`, `=` | Pointer/value comparison |
| **Memory access** | `peek`, `poke`, `address-of` | Direct hardware access for drivers |
| **Arithmetic** | `+`, `-`, `*`, `/`, `mod` on integers | Efficiency, bootstrap |
| **Comparison** | `<`, `>`, `<=`, `>=` | Efficiency |
| **Symbol interning** | `symbol`, `gensym` | Interner access |

Note: The Rust runtime has its own UART access for panic handlers and early boot diagnostics, but this is NOT exposed to Lonala. Lonala implements device drivers (including UART) using `peek`/`poke` on memory-mapped I/O registers.

Special forms (`quote`, `if`, `fn`, `def`, `do`, `defmacro`) are handled by the compiler, not native functions.

### What MUST Be Lonala

Everything else, including:

- **All macros**: `defn`, `when`, `unless`, `let`, `cond`, `and`, `or`
- **Collection constructors**: `list`, `vector`, `hash-map` (build from `cons`)
- **Sequence operations**: `map`, `filter`, `reduce`, `concat`, `nth`, `count`
- **Higher-order functions**: `apply`, `comp`, `partial`, `identity`
- **The REPL itself**: Read-eval-print loop in Lonala
- **String operations**: All string manipulation beyond raw bytes
- **ALL device drivers**: Including UART, via `peek`/`poke` on memory-mapped I/O
- **Process management**: Supervision trees, spawn, message passing
- **The evaluator**: `eval` written in Lonala (self-hosting)

### Current Interim Code

The current Rust implementations of REPL, collections, and introspection are **interim scaffolding** that will be replaced by Lonala implementations. When extending these, remember they are temporary.

### Review Checklist

Before adding ANY native function, ask:

1. Can this be implemented using existing primitives? → **Implement in Lonala**
2. Does this require hardware access? → Native is acceptable
3. Does this require inspecting runtime type tags? → Native is acceptable
4. Is this purely for efficiency? → **Implement in Lonala first**, optimize later only if profiling proves necessary

**When in doubt: Lonala.**

## Directory Structure

> **Note**: Directories marked with `(planned)` are defined in the implementation plan but do not exist yet.

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
│   │       ├── integer.rs        # Arbitrary-precision integers
│   │       ├── ratio.rs          # Arbitrary-precision ratios
│   │       ├── string.rs         # Immutable string type
│   │       ├── symbol.rs         # Interned symbols
│   │       └── value.rs          # Core value types
│   │
│   ├── lona-runtime/             # seL4 root task (QEMU-tested only)
│   │   └── src/
│   │       ├── main.rs           # Entry point, receives bootinfo
│   │       ├── repl.rs           # Interactive REPL
│   │       ├── memory/           # seL4 memory management
│   │       │   ├── mod.rs
│   │       │   ├── provider.rs   # Memory provider implementation
│   │       │   ├── untyped.rs    # Untyped memory handling
│   │       │   └── slots.rs      # Capability slot management
│   │       └── platform/         # Hardware abstraction
│   │           ├── mod.rs
│   │           ├── uart.rs       # UART driver
│   │           └── fdt.rs        # Device tree parsing
│   │
│   ├── lonala-parser/            # Lexer and parser (100% host-testable)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ast.rs            # Abstract syntax tree types
│   │       ├── error.rs          # Error types and spans
│   │       ├── token.rs          # Token types
│   │       ├── lexer/            # Tokenizer
│   │       │   ├── mod.rs
│   │       │   └── tests.rs
│   │       └── parser/           # S-expression parser
│   │           ├── mod.rs
│   │           └── tests.rs
│   │
│   ├── lonala-compiler/          # Bytecode compiler (100% host-testable)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── chunk.rs          # Bytecode chunk format
│   │       ├── error.rs          # Compiler errors
│   │       ├── opcode.rs         # VM instruction encoding
│   │       └── compiler/         # AST to bytecode compiler
│   │           ├── mod.rs
│   │           └── tests.rs
│   │
│   ├── lona-kernel/              # Process/scheduler abstractions (host-testable with mocks)
│   │   └── src/
│   │       ├── lib.rs
│   │       └── vm/               # Bytecode virtual machine
│   │           ├── mod.rs
│   │           ├── error.rs      # Runtime errors
│   │           ├── frame.rs      # Call frames
│   │           ├── globals.rs    # Global variable storage
│   │           ├── helpers.rs    # Interpreter helper functions
│   │           ├── interpreter.rs# Bytecode execution
│   │           ├── natives.rs    # Native function registry
│   │           ├── numeric.rs    # Numeric operations
│   │           ├── output.rs     # Output abstraction
│   │           ├── primitives.rs # Built-in functions (print)
│   │           └── tests.rs      # VM tests
│   │
│   └── lona-test/                # Test harness for QEMU tests
│       └── src/
│           └── lib.rs            # Test utilities and markers
│
├── docs/
│   ├── index.md                  # Documentation index
│   ├── goals.md                  # Project vision and design philosophy
│   ├── license.md                # License information
│   ├── architecture/
│   │   └── register-based-vm.md  # VM architecture documentation
│   └── development/
│       ├── implementation-plan.md      # Phased roadmap and task checklist
│       ├── minimal-rust.md             # Lonala-first principle (minimal Rust runtime)
│       ├── testing-strategy.md         # Three-tier testing pyramid
│       ├── rust-coding-guidelines.md   # Rust coding standards
│       └── lonala-coding-guidelines.md # Lonala coding standards
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
├── tests/ (planned)              # Integration tests (Tier 3)
│   └── integration/
│
└── .claude/                      # Claude Code configuration
    ├── commands/                 # Custom slash commands
    ├── agents/                   # Custom agent definitions
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
