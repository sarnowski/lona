---
name: develop-lonala
description: Mandatory workflow for writing any Lonala code. Use this skill BEFORE implementing any Lonala code, including new macros, functions, standard library additions, or bug fixes.
---

# Develop Lonala

This skill enforces the mandatory development workflow for all Lonala code in this project.

## Step 1: Read the Guidelines

Read these files completely before writing any code:

- `docs/lonala/index.md`
- `docs/development/lonala-coding-guidelines.md`
- `docs/development/testing-strategy.md`

Understanding these guidelines prevents lengthy iteration cycles fixing issues after the fact.

## Step 2: Implement Code and Tests

With the guidance from the above documents:

- Write well-documented Lonala code following the coding guidelines
- Write comprehensive tests following the testing strategy
- Tests are placed in `crates/lona-spec-tests/src/` or as Rust test modules

> **Bug Fixes: Test-First Approach**
>
> When fixing a bug (whether reported by the user or discovered during development), you MUST write a failing test that demonstrates the bug BEFORE attempting any fix. This test should:
>
> 1. Clearly demonstrate the buggy behavior
> 2. Be named descriptively (e.g., `test_map_handles_nil_key`)
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
