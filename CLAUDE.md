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
│   ├── lonala-parser/ (planned)  # Lexer and parser (100% host-testable)
│   ├── lonala-compiler/ (planned)# Bytecode compiler (100% host-testable)
│   ├── lona-kernel/ (planned)    # Process/scheduler abstractions (host-testable with mocks)
│   └── lona-test/ (planned)      # Test harness for QEMU tests
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
