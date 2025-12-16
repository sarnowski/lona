# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Lona is a general-purpose operating system combining:
- **seL4 microkernel** - formally verified, capability-based security
- **LISP machine philosophy** - runtime introspection, hot-patching
- **Erlang/OTP concurrency model** - lightweight processes, supervision trees, "let it crash"

The runtime is written in Rust (`no_std`) and runs as seL4's root task. Userspace will be programmed in Lonala, a custom Clojure/Erlang-inspired language.

### Required Reading

**Read when planning new features or tasks:**
- `docs/goals.md` - The complete vision and design philosophy. Consult when making architectural decisions.
- `docs/development/implementation-plan.md` - The phased roadmap, component dependencies, and current status.

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
│   │       └── allocator.rs      # Bump allocator traits
│   │
│   ├── lona-runtime/             # seL4 root task (QEMU-tested only)
│   │   └── src/
│   │       ├── main.rs           # Entry point, receives bootinfo
│   │       ├── memory/           # seL4 memory management
│   │       │   ├── provider.rs   # Memory provider implementation
│   │       │   ├── untyped.rs    # Untyped memory handling
│   │       │   └── slots.rs      # Capability slot management
│   │       └── platform/         # Hardware abstraction
│   │           ├── uart.rs       # UART driver
│   │           └── fdt.rs        # Device tree parsing
│   │
│   ├── lonala-parser/            # Lexer and parser (100% host-testable)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ast.rs            # Abstract syntax tree types
│   │       ├── error.rs          # Error types and spans
│   │       ├── lexer.rs          # Tokenizer
│   │       ├── parser.rs         # S-expression parser
│   │       └── token.rs          # Token types
│   │
│   ├── lonala-compiler/          # Bytecode compiler (100% host-testable)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── chunk.rs          # Bytecode chunk format
│   │       ├── compiler.rs       # AST to bytecode compiler
│   │       ├── error.rs          # Compiler errors
│   │       └── opcode.rs         # VM instruction encoding
│   │
│   ├── lona-kernel/              # Process/scheduler abstractions (host-testable with mocks)
│   │   └── src/
│   │       ├── lib.rs
│   │       └── vm/               # Bytecode virtual machine
│   │           ├── mod.rs
│   │           ├── error.rs      # Runtime errors
│   │           ├── frame.rs      # Call frames
│   │           ├── globals.rs    # Global variable storage
│   │           ├── interpreter.rs# Bytecode execution
│   │           ├── natives.rs    # Native function registry
│   │           ├── output.rs     # Output abstraction
│   │           └── primitives.rs # Built-in functions (print)
│   │
│   └── lona-test/                # Test harness for QEMU tests
│       └── src/
│           └── lib.rs            # Test utilities and markers
│
├── docs/
│   ├── goals.md                  # Project vision and design philosophy
│   └── development/
│       ├── implementation-plan.md    # Phased roadmap and task checklist
│       ├── testing-strategy.md       # Three-tier testing pyramid
│       └── rust-coding-guidelines.md # Coding standards
│
├── docker/
│   └── Dockerfile                # Development environment with seL4 SDK
│
├── support/
│   └── targets/
│       └── aarch64-sel4.json     # Custom Rust target for seL4
│
├── tests/ (planned)              # Integration tests (Tier 3)
│   └── integration/
│
└── .claude/                      # Claude Code configuration
    ├── commands/                 # Custom slash commands
    ├── agents/                   # Custom agent definitions
    └── skills/                   # Workflow skills (develop-runtime, finishing-work)
```

## Workflows

- **Before writing any Rust code**: Load the `develop-runtime` skill and follow its instructions
- **When finishing all work**: Load the `finishing-work` skill and follow its instructions

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
make check           # Fast verification: fmt, compile, clippy, unit tests
make build           # Create bootable QEMU image (includes check)
make run             # Run in QEMU (includes build)
make test            # Run integration tests in QEMU
make clean           # Remove build artifacts
make shell           # Interactive Docker shell for debugging
```
