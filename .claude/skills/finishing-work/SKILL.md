---
name: finishing-work
description: Mandatory workflow to complete any implementation work. Use this skill BEFORE claiming success, being done, or finishing any coding task. Ensures all changes pass code review.
---

# Finishing Work

You MUST NOT claim success or completion without following this workflow.

**Core principle: We always leave code better than we found it.** During review, ALL issues found must be resolved—including pre-existing issues unrelated to your current task. Finding an issue late is lucky—better late than never. "Out of scope" is never an acceptable reason to ignore a problem.

---

## Step 1: Manual Verification in seL4

Verify changes work in the real seL4 environment before code review.

1. **Restart QEMU**: Use `mcp__lona-dev-repl__restart` to rebuild with latest changes
2. **Identify new features**: Review changed files with `git status`
3. **Test in REPL**: Use `mcp__lona-dev-repl__eval` to verify:
   - Happy path for each new feature
   - Edge cases and error conditions
   - Error message accuracy
4. **Fix issues** before proceeding

**Report results to user in this format:**

```markdown
## Manual REPL Verification

| # | Expression | Expected | Actual | Status |
|---|------------|----------|--------|--------|
| 1 | `(+ 1 2)` | `3` | `3` | ✓ |
| 2 | `(/ 1 0)` | Error | `Error: Division by zero` | ✓ |

### Expressions
​```clojure
(+ 1 2)
(/ 1 0)
​```
```

Report ALL expressions with actual REPL output. Use ✓ for pass, ✗ for fail.

---

## Step 2: Code Review Loop

Run three independent code reviews in parallel, then resolve all findings.

### 2.1: Gather Context

Run these commands and capture output:

```bash
git status --porcelain          # Changed files list
make test                       # Verification results
```

Classify changed files: Rust (`*.rs`), Lonala (`*.lona`), Documentation (`*.md`), Other.

### 2.2: Launch Parallel Reviews

Send a **single message** with **three background Bash calls** to run reviews in parallel.

All three CLIs accept the prompt as a positional argument:

```bash
timeout 900 claude --model opus -p "PROMPT"
timeout 900 gemini -m gemini-3-pro-preview -s "PROMPT"
timeout 900 codex exec -m gpt-5.2-codex -c model_reasoning_effort=low -c hide_agent_reasoning=true -s read-only "PROMPT" 2>/dev/null
```

All three receive the **identical prompt** (substitute actual values):

```
You are a code reviewer for the Lona project.

## Restrictions
- You may ONLY read files and output analysis
- You CANNOT modify files or execute commands (except file reads)
- Output a structured review report only

## Project Context
Lona is an OS combining seL4 microkernel (capability-based security), LISP machine philosophy (runtime introspection, hot-patching), and Erlang/OTP concurrency (lightweight processes, supervision).

## Required Reading
Read these files completely before reviewing:

**Always read:**
- docs/goals/index.md
- docs/development/principles.md
- docs/development/testing-strategy.md
- docs/roadmap/index.md

**If Rust files changed, also read:**
- docs/development/rust-coding-guidelines.md

**If Lonala files changed, also read:**
- docs/development/lonala-coding-guidelines.md

## Changed Files to Review
{INSERT GIT STATUS OUTPUT}

## Verification Results
{INSERT MAKE TEST OUTPUT}

## Your Task
1. Read all required documents above
2. Read all changed files listed above
3. Read related files (imports, callers, tests) to understand full context
4. Evaluate against ALL review dimensions below

**IMPORTANT: Report ALL issues you find, including pre-existing issues in the reviewed files.** If you spot a problem that existed before these changes, report it anyway. Finding issues late is valuable—better late than never. Do not filter issues based on whether they were introduced by these changes.

## Review Dimensions

1. **Conceptual Alignment**: Does it align with Lona's goals and vision?

2. **Lonala-First Principle** (CRITICAL): Is there Rust code that should be Lonala?
   - Allowed in Rust: cons/first/rest, type predicates, equality, peek/poke, basic arithmetic, symbol interning
   - Must be Lonala: macros, collection constructors, sequence ops, higher-order functions, string ops, REPL, ALL drivers, eval
   - Flag any new native function that can be built from existing primitives

3. **OS/Kernel Design**: Are microkernel principles respected? Is TCB minimized?

4. **seL4 Alignment**: Correct capability patterns? Proper isolation?

5. **Rust Quality** (if applicable): Follows guidelines? Unsafe code justified?

6. **Lonala Quality** (if applicable): Proper indentation, kebab-case, comment levels?

7. **BEAM/OTP Patterns**: Correct supervision and fault-tolerance?

8. **Security**: Capability leaks? Privilege escalation? Vulnerabilities?

9. **Testing**: Adequate coverage per testing strategy? Edge cases?

10. **Regression Tests**: Bug fixes MUST have regression tests

11. **Roadmap Status**: Is the task documented? Status accurate?

12. **Spec Tests**: Check crates/lona-spec-tests/ - are relevant tests un-ignored? Edge cases covered?

13. **Documentation**: Accurate and up-to-date?

## Output Format

```
## Files Reviewed
- path/to/file.rs (read)
- path/to/related.rs (read for context)

## Findings

### [Category] file:line - Description
Violates: [which principle/guideline]

### [Category] file:line - Description
Violates: [which principle/guideline]

## Summary
Total issues: N
```

Be specific with file paths and line numbers. Every finding is an issue that MUST be fixed - no "minor" or "optional" categorization.
```

### 2.3: Collect and Combine Results

Use `TaskOutput` to wait for all three reviews. Then:

1. **Parse each report** extracting findings with file:line
2. **Deduplicate**: Match identical issues (same file, line, description)
3. **Attribute**: Tag each finding with who found it:
   - `[claude]`, `[gemini]`, `[codex]`
   - `[claude, gemini]` if both found same issue
   - `[claude, gemini, codex]` if all three found it

### 2.4: Present Unified Report

Present ALL findings to user:

```markdown
## Code Review Results

### Findings

| # | Attribution | File:Line | Category | Issue |
|---|-------------|-----------|----------|-------|
| 1 | [claude, gemini] | src/vm/mod.rs:42 | Security | Missing bounds check |
| 2 | [codex] | src/compiler/mod.rs:18 | Testing | No edge case test |

### Summary
- Claude: N issues
- Gemini: M issues
- Codex: K issues
- Unique issues after dedup: X
```

### 2.5: Resolve All Issues

**Every finding must be resolved. No exceptions.**

This includes:
- Issues in code you changed
- Issues in code you touched but didn't intentionally change
- **Pre-existing issues** found during review (even if they existed before your work began)
- Issues in related files the reviewers examined for context

**We always leave code better than we found it.** If a reviewer flags an issue, it gets fixed—regardless of when it was introduced. Pre-existing issues are opportunities, not excuses—we're lucky to catch them now rather than never.

Resolution approach:
- **Obvious fix**: Fix immediately
- **Unclear solution**: Present 2-3 options to user with recommendation, wait for feedback
- **High-effort fix**: Ask user how to proceed (but do NOT skip or ignore)

Rules:
- ALL findings must be addressed—no exceptions
- "Out of scope" is NOT a valid reason to skip issues
- "Pre-existing" is NOT a valid reason to skip issues
- Do NOT defer to "future improvements"
- Do NOT skip "minor" or "low priority" issues
- When in doubt, ask user—but never ignore

### 2.6: Re-run if Code Changed

After fixing:
- **Documentation-only fixes**: Proceed to Step 3
- **Code changes**: Return to Step 2.2 and re-run all three reviews

Repeat until all reviews report **zero issues**.

---

## Step 3: Documentation Build

Verify documentation builds cleanly:

```bash
make docs
```

Fix any warnings (broken links, missing files, invalid markdown) and re-run until zero warnings.

---

## Step 4: Update Progress

Only after code review AND doc build pass with zero issues:

### 4.1: Update Roadmap

In `docs/roadmap/index.md`, change completed task status from `open` to `done`:

```markdown
# Before
| 1.1.6 | Metadata System - Reader Syntax | open |

# After
| 1.1.6 | Metadata System - Reader Syntax | done |
```

Only mark `done` if ALL acceptance criteria are met.

### 4.2: Clean Up Plan

If `PLAN.md` exists at repo root:
- Work complete → Delete the file
- Partial completion → Update to mark completed sections

---

## Step 5: File Size Check

Verify no source code or documentation files exceed the size limit:

```bash
# Find files over 600 lines (hard limit)
find . -type f \( -name "*.rs" -o -name "*.lona" -o -name "*.md" \) \
  -exec sh -c 'lines=$(wc -l < "$1"); [ "$lines" -gt 600 ] && echo "$1: $lines lines"' _ {} \;
```

**Hard limit: 600 lines.** Any file exceeding this must be split before completion.

**Splitting strategies:**
- **Rust/Lonala**: Externalize tests to `tests/` subdirectory, or split into submodules
- **Markdown**: Split into multiple linked documents (e.g., `index.md` linking to `section-*.md`)

See [Rust Coding Guidelines - File Size Limits](../../docs/development/rust-coding-guidelines.md#file-size-limits) for detailed guidance.

---

## Done

Work is complete when:
1. Manual REPL verification passed
2. All three code reviews report zero issues
3. **All issues were resolved**—including pre-existing issues found during review
4. Documentation build has zero warnings
5. Roadmap updated
6. **All source and markdown files are under 600 lines**

**If you skipped any issue for any reason, work is NOT complete.** This includes issues dismissed as "out of scope", "pre-existing", "minor", or "to be fixed later". Pre-existing issues discovered during review are lucky finds—fix them now.
