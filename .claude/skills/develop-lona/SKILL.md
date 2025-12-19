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

**CRITICAL: You MUST follow a test-first approach for ALL bug fixes.**

The bug fix workflow for Lonala follows the same pattern as Rust development. See **CLAUDE.md Test-First Bug Fixing section** for the philosophy and the `develop-runtime` skill for detailed steps (B1-B6).

Key differences for Lonala:

1. **Read these guidelines** (Step B1):
   - `docs/lonala/index.md`
   - `docs/development/lonala-coding-guidelines.md`
   - `docs/development/testing-strategy.md`

2. **Write tests in spec test files** (Step B3):
   - Add test cases in `crates/lona-spec-tests/src/` in the appropriate module
   - Or add tests in Rust test modules that exercise the Lonala code

3. **Follow Lonala coding guidelines** when fixing (Step B4)

4. **Run verification**: `make test`

5. **Finish**: Call the `finishing-work` skill

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
