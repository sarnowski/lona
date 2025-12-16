---
name: develop-runtime
description: Mandatory workflow for writing any Rust code in the runtime. Use this skill BEFORE implementing any Rust code, including new features, bug fixes, refactoring, or adding tests.
---

# Develop Runtime

This skill enforces the mandatory development workflow for all Rust code in this project.

## Mandatory Workflow

You MUST follow these steps in order:

### Step 1: Read the Rust Coding Guidelines

Read the complete file: `docs/development/rust-coding-guidelines.md`

Do not skip any sections. Understand all conventions before writing code.

### Step 2: Read the Testing Strategy

Read the complete file: `docs/development/testing-strategy.md`

Do not skip any sections. Understand the testing requirements and patterns.

### Step 3: Understand Clippy Configuration

Read the workspace `Cargo.toml` file and examine the `[workspace.lints.clippy]` section to understand which clippy lints are enabled and at what level (warn, deny, forbid).

Write code that conforms to these lint rules from the start. This avoids lengthy iteration cycles fixing clippy issues after the fact.

### Step 4: Implement Code and Tests

With the guidance from the above documents:

- Write testable, well-structured code following the coding guidelines
- Write comprehensive tests following the testing strategy
- Aim for high test coverage

### Step 5: Verify with Make Targets

Run the following command and iterate until there are ZERO issues:

```bash
make test
```

This runs the full verification suite: formatting, compilation, clippy, unit tests, and integration tests. Fix all warnings, errors, and test failures before proceeding.

#### Clippy Policy

**CRITICAL: You MUST NOT disable any clippy check at any level.**

A pre-tool hook automatically blocks any attempt to add suppression directives without proper approval.

When encountering clippy warnings:

1. **Always attempt to fix the issue correctly first**
2. **If you cannot correctly resolve a clippy issue**:
   - STOP and explain the issue to the user
   - Describe what the warning means and why it's occurring
   - Provide your recommendation for how to handle it (fix approach or why an exception might be warranted)
   - Wait for the user's EXPLICIT approval before taking any action
3. **NEVER use `#[allow(...)]`, `#[expect(...)]`, or any other mechanism to suppress clippy warnings without explicit user approval**
4. **NEVER add clippy exceptions to Cargo.toml, clippy.toml, or any source file without explicit user approval**

Do not assume approval. The user MUST explicitly approve ANY clippy exception.

**Approved Directive Format**: When the user explicitly approves, include `[approved]` in the reason:

```rust
#[expect(clippy::lint_name, reason = "[approved] explanation")]
```

### Step 6: Finish Work

Call the `finishing-work` skill to complete the workflow.
