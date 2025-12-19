---
name: develop-lona
description: Mandatory workflow for writing any Lonala code. Use this skill BEFORE implementing any Lonala code, including new macros, functions, standard library additions, or bug fixes.
---

# Develop Lona

This skill enforces the mandatory development workflow for all Lonala code in this project.

## Determine Workflow Type

First, determine what type of work you are doing:

- **Bug Fix**: A bug was reported in Lonala code OR you discovered a bug → Follow the **Bug Fix Workflow** below
- **New Feature / Enhancement**: Any other Lonala code changes → Follow the **Standard Development Workflow** below

---

## Bug Fix Workflow

**CRITICAL: You MUST follow this test-first approach for ALL bug fixes.**

### Step B1: Read the Guidelines

Read these files completely before proceeding:
- `docs/development/lonala-coding-guidelines.md`
- `docs/development/testing-strategy.md`

### Step B2: Understand the Bug

Before writing any code:
1. Reproduce the bug and understand its root cause
2. Document what the expected behavior should be
3. Identify which component(s) are affected

### Step B3: Write a Failing Test FIRST

**You MUST write a test that demonstrates the bug BEFORE attempting any fix.**

For Lonala code, this typically means:
- Adding a test case in the appropriate spec test file
- Or adding a test in the relevant Rust test module that exercises the Lonala code

This test should:
- Clearly demonstrate the buggy behavior
- Be named descriptively
- FAIL when run against the current (buggy) code

Run the test to confirm it fails:
```bash
make test
```

The test MUST fail. If it passes, your test doesn't capture the bug correctly. Revise the test until it demonstrates the failure.

### Step B4: Fix the Bug

Now implement the minimal fix required to make the test pass:
- Keep the fix focused - don't refactor or add unrelated improvements
- Follow the Lonala coding guidelines
- Ensure the fix addresses the root cause, not just symptoms

### Step B5: Verify the Fix

Run the full test suite:
```bash
make test
```

- Your new test MUST now pass
- All existing tests MUST still pass

### Step B6: Finish Work

Call the `finishing-work` skill to complete the workflow.

---

## Standard Development Workflow

For new features, enhancements, and standard library additions, follow these steps:

### Step 1: Read the Lonala Coding Guidelines

Read the complete file: `docs/development/lonala-coding-guidelines.md`

Do not skip any sections. Understand all conventions before writing code. Key points to remember:

| Aspect | Requirement |
|--------|-------------|
| File Header | SPDX license + copyright |
| Module Docs | Use `;;;` for top-level descriptions |
| Section Headers | Use `;;;;` for major sections |
| Code Comments | Use `;;` for code block explanations |
| Naming | kebab-case for all identifiers |
| Predicates | End with `?` (e.g., `empty?`) |
| Mutating Ops | End with `!` (e.g., `reset!`) |
| Indentation | 2 spaces, never tabs |
| Documentation | Every public macro/function needs docs |

### Step 2: Read the Testing Strategy

Read the complete file: `docs/development/testing-strategy.md`

Understand how Lonala code is tested within the project's testing pyramid.

### Step 3: Understand File Organization

Standard Lonala file structure:

```clojure
;; SPDX-License-Identifier: GPL-3.0-or-later
;; Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

;;; Module Name
;;;
;;; Module description explaining the purpose of this file
;;; and its role in the system.

;;;; Section Heading

;; function-name - Brief description.
;;
;; Detailed explanation of what the function does,
;; including parameter documentation with `backticks`.
;;
;; Usage: (function-name arg1 arg2)
;;
;; Example:
;;   (function-name "foo" 42) ; => result
(defn function-name [arg1 arg2]
  ...)
```

### Step 4: Implement Code

With the guidance from the above documents:

- Write well-documented Lonala code following the coding guidelines
- Use proper comment levels (`;;;`, `;;;;`, `;;`, `;`)
- Follow kebab-case naming conventions
- Document all public functions and macros
- Include Usage and Example sections in documentation

### Step 5: Verify

Run the full verification suite:

```bash
make test
```

This runs formatting, compilation, clippy, unit tests, and integration tests. Fix all failures before proceeding.

### Step 6: Finish Work

Call the `finishing-work` skill to complete the workflow.

---

## Quick Reference: Comment Levels

| Semicolons | Usage | Example |
|------------|-------|---------|
| `;;;;` | Section headings | `;;;; Control Flow` |
| `;;;` | Module-level docs | `;;; This module provides...` |
| `;;` | Code block comments | `;; Validate input first` |
| `;` | Inline comments | `(+ x y) ; sum values` |

## Quick Reference: Naming Conventions

| Pattern | Usage | Example |
|---------|-------|---------|
| `kebab-case` | All identifiers | `process-message` |
| `name?` | Predicates | `empty?`, `valid?` |
| `name!` | Mutating/unsafe | `swap!`, `reset!` |
| `-name` | Private | `-internal-helper` |
| `_` | Unused binding | `(let [_ (side-effect)] ...)` |
