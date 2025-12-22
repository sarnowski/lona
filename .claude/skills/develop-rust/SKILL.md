---
name: develop-rust
description: Mandatory workflow for writing any Rust code in the runtime. Use this skill BEFORE implementing any Rust code, including new features, bug fixes, refactoring, or adding tests.
---

# Develop Rust

This skill enforces the mandatory development workflow for all Rust code in this project.

## Step 1: Read the Guidelines

Read these files completely before writing any code:

- `docs/development/rust-coding-guidelines.md`
- `docs/development/testing-strategy.md`

Understanding these guidelines prevents lengthy iteration cycles fixing issues after the fact.

## Step 2: Implement Code and Tests

With the guidance from the above documents:

- Write testable, well-structured code following the coding guidelines
- Write comprehensive tests following the testing strategy
- Aim for high test coverage

> **Bug Fixes: Test-First Approach**
>
> When fixing a bug (whether reported by the user or discovered during development), you MUST write a failing test that demonstrates the bug BEFORE attempting any fix. This test should:
>
> 1. Clearly demonstrate the buggy behavior
> 2. Be named descriptively (e.g., `test_parser_handles_empty_input` or `test_issue_123_stack_overflow`)
> 3. Be placed in the appropriate test location per the testing strategy
> 4. FAIL when run against the current (buggy) code
>
> Run `make test` to confirm the test fails, then implement the minimal fix required to make it pass. This approach prevents regressions and verifies the fix works.

## Step 3: Verify with Make Targets

Run the following command and iterate until there are ZERO issues:

```bash
make test
```

This runs the full verification suite: formatting, compilation, clippy, unit tests, and integration tests. Fix all warnings, errors, and test failures before proceeding.

## Step 4: Finish Work

Call the `finishing-work` skill to complete the workflow.
