---
name: develop-rust
description: MUST be loaded before reading, writing, reviewing, or even thinking about Rust code in Lona. Load this skill FIRST whenever Rust is involved - before exploring files, before planning, before any Rust-related reasoning. Contains development principles (YAGNI, KISS, clean code), project-specific coding guidelines (rust.md), and the test-first workflow for implementation.
---

# Develop Rust

**STOP. Before you read any Rust file, write any Rust code, or even think about Rust architecture - complete Part 1.**

---

## Part 1: Principles and Context (ALWAYS MANDATORY)

This part is required for ALL Rust-related work: reviews, conceptual discussions, planning, or implementation.

### 1.1 Read the Rust Guide

**Read [docs/rust.md](../../../docs/rust.md) NOW.**

This file contains project-specific conventions you must know:
- Project structure and build infrastructure
- Memory layout conventions (Paddr/Vaddr newtypes, struct layout)
- Testing strategy (host-testable vs seL4-only components)
- Platform abstraction patterns (MemorySpace trait, MockVSpace)

**Do not skip this. Do not assume you know the conventions.**

### 1.2 Architecture Principles

All Rust work in this project follows these principles strictly:

**YAGNI (You Aren't Gonna Need It)**
- Only implement what is explicitly required
- No "future-proofing" abstractions
- No configurability unless requested
- Delete unused code, don't comment it out

**KISS (Keep It Simple, Stupid)**
- Straightforward solutions over clever ones
- Minimize indirection and abstraction layers
- Three similar lines > premature abstraction
- No over-engineering

**Clean Code**
- Functions do one thing
- Names reveal intent
- Small, focused functions
- No dead code

---

## Part 2: Writing Code (Only When Implementing)

Skip this part if you're only reviewing or discussing concepts. Follow this part when writing or modifying Rust code.

### 2.1 Write Tests First

Before writing any business logic:
- Write tests that define the expected behavior
- Use the testability matrix from rust.md to determine what can be host-tested
- Unit tests: `#[cfg(test)] mod tests` in the same file
- Integration tests: `tests/` directory
- Use `MockVSpace` for memory operations

### 2.2 Write the Code

Implement the minimal code to make tests pass. Apply YAGNI/KISS/Clean Code from Part 1.

### 2.3 Verify

Run:

```bash
make verify
```

This runs all checks: format, lint, test, build.

**The task is not complete until `make verify` passes.**

---

## Part 3: Finishing Work (MANDATORY)

After `make verify` passes and your implementation is complete, you **MUST** invoke the `finishing-work` skill before claiming the work is done.

The `finishing-work` skill triggers parallel AI agent reviews and ensures all issues are resolved. **No work is complete without it.**
