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

### Step 3: Implement Code and Tests

With the guidance from the above documents:

- Write testable, well-structured code following the coding guidelines
- Write comprehensive tests following the testing strategy
- Aim for high test coverage

### Step 4: Verify with Make Targets

Run the following commands and iterate until there are ZERO issues:

```bash
make check
make test
```

Fix all warnings, errors, and test failures before proceeding.

### Step 5: Finish Work

Call the `finishing-work` skill to complete the workflow.
