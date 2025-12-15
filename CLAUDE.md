# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Lona is a general-purpose operating system combining:
- **seL4 microkernel** - formally verified, capability-based security
- **LISP machine philosophy** - runtime introspection, hot-patching
- **Erlang/OTP concurrency model** - lightweight processes, supervision trees, "let it crash"

The runtime is written in Rust (`no_std`) and runs as seL4's root task. Userspace will be programmed in Lonala, a custom Clojure/Erlang-inspired language.

### Mandatory Reading

**BEFORE ANY TASK** (planning, discussing features, or writing code), you MUST read:

1. `docs/goals.md` - The complete vision and design philosophy. This is essential for understanding all architectural decisions.
2. `docs/development/implementation-plan.md` - The phased roadmap, component dependencies, and current status.

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

All Rust commands run inside Docker. The `make check` target runs:
1. `cargo fmt --check`
2. `cargo build` (with seL4 target)
3. `cargo clippy -- -D warnings`
4. `cargo test --workspace --exclude lona-runtime` (host-testable crates only)

## Code Architecture

### Crate Structure

```
crates/
└── lona-runtime/     # seL4 root task, QEMU-tested only
    src/main.rs       # Entry point, receives bootinfo from seL4
```

Future crates will follow a layered architecture to maximize host-testability:

| Layer | Crate | Purpose |
|-------|-------|---------|
| Top | `lona-runtime` | seL4-specific, entry point, hardware interaction |
| Middle | `lona-kernel` | Abstractions with trait-based mocking |
| Language | `lonala-compiler`, `lonala-parser` | Pure logic, 100% host-testable |
| Foundation | `lona-core` | Value types, traits, errors |

Only `lona-runtime` depends on `sel4` and `sel4-root-task`.

## Workflows

- **Before writing any Rust code**: Load the `develop-runtime` skill and follow its instructions
- **When finishing all work**: Load the `finishing-work` skill and follow its instructions

## Target Platform

- **Architecture**: ARM64 (aarch64-sel4 custom target)
- **Machine**: QEMU virt with Cortex-A57 CPU
- **Memory**: 1GB default
- **seL4 Prefix**: `/opt/seL4` (inside Docker)
