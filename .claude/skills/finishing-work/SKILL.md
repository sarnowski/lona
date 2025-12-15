---
name: finishing-work
description: MANDATORY skill for completing any task. ALWAYS use this skill when you have finished implementing a feature, fixing a bug, making code changes, writing documentation, or completing any user-requested task. This skill MUST be invoked at the end of EVERY task before reporting completion to the user. It runs code review, ensures documentation consistency, and maintains project quality. Use when: task is done, implementation complete, changes finished, work ready for review, about to tell user "done".
---

# Finishing Work

This skill ensures all completed work meets project quality standards. **It MUST be invoked at the end of EVERY task** before reporting completion to the user.

## CRITICAL: When to Use This Skill

**ALWAYS invoke this skill when:**

- You have finished implementing any feature or change
- You have completed a bug fix
- You have made ANY code modifications
- You have written or modified documentation
- You are about to tell the user the task is "done" or "complete"
- You have finished any user-requested work

**There are NO exceptions.** Every completed task goes through this checklist.

## Workflow

### Step 1: Code Review (If Code Was Changed)

If ANY code files were created or modified (`.rs`, `.S`, `.toml`, `Makefile`, etc.):

1. **Invoke the lona-code-reviewer subagent:**

   Use the Task tool with `subagent_type: lona-code-reviewer` to perform a comprehensive code review of all changes. The agent will:
   - Run `make build` (fmt, clippy, compile)
   - Read all coding guidelines and ADRs
   - Analyze changes against project standards
   - Check for correctness, security, and performance issues
   - Verify test coverage for pure logic
   - Verify documentation consistency
   - Produce a structured review report

2. **Report findings to the user:**

   Present the code review report to the user. Include:
   - Summary of what was reviewed
   - All findings (Critical, Major, Minor, Suggestions)
   - Positive observations

3. **Handle issues - WORK IS NOT FINISHED UNTIL ALL ISSUES ARE RESOLVED:**

   **CRITICAL RULE: ALL findings (Critical, Major, AND Minor) MUST be fixed. There are NO exceptions. Do not skip ANY issue.**

   **Critical Issues:**
   - MUST be fixed immediately before work can be considered complete
   - Fix the issues yourself, then re-run the code reviewer

   **Major Issues:**
   - MUST be fixed before work can be considered complete
   - If there are multiple valid resolution approaches, **ASK THE USER** which approach they prefer
   - If the fix is non-trivial or you're unsure of the best approach, **ASK THE USER** for advice
   - Fix the issues, then re-run the code reviewer

   **Minor Issues:**
   - MUST be fixed before work can be considered complete
   - Do NOT skip minor issues or merely "note" them—fix them
   - If there are multiple valid resolution approaches, **ASK THE USER** which approach they prefer
   - Fix the issues yourself, then re-run the code reviewer

   **Suggestions:**
   - Present ALL suggestions to user for consideration
   - Ask: "The code reviewer suggested [X]. Would you like me to implement this improvement?"
   - Wait for user response before proceeding—do not assume "no"

4. **Iterate until clean:**

   Re-run the code reviewer after fixes. Repeat until no Critical, Major, or Minor issues remain.

5. **Do NOT skip this step.** Even "trivial" changes need code review.

### Step 2: Documentation Impact Assessment

Evaluate whether your changes affect documentation:

#### 2a. Identify Affected Documentation Areas

Consider which documentation might need updates:

- **Project goals** (`docs/goals.md`) - If you changed core concepts or design philosophy
- **Architecture docs** (`docs/architecture/`) - If you changed system design, behavior, or interfaces (when this directory exists)
- **Reference docs** (`docs/reference/`) - If you changed kernel objects, syscalls, or memory layout (when this directory exists)
- **ADRs** (`docs/development/adr/`) - If implementation deviates from or fulfills ADR decisions
- **Coding guidelines** (`docs/development/`) - If new patterns were established

**Note**: This project is in early development. Some documentation directories may not exist yet. Focus on updating documentation that does exist.

#### 2b. Check for Documentation Inconsistencies

Read relevant documentation files and verify they still accurately describe the system:

1. If your changes altered behavior documented elsewhere, the docs are now **inconsistent**
2. If your implementation differs from what documentation describes, there's a **mismatch**
3. If error messages, APIs, or interfaces changed, docs may need **updates**

Note: The code reviewer also checks documentation consistency, so some issues may already be identified.

### Step 3: Documentation Updates

#### If Documentation is Inconsistent:

**ASK THE USER** before making changes:

> "I noticed that [specific documentation] describes [old behavior], but the implementation now [new behavior]. Should I update the documentation to reflect this change?"

Provide specific details:
- Which file(s) are affected
- What the current documentation says
- What the new reality is
- Proposed changes

**Wait for user approval before modifying documentation.**

#### If Significant Documentation is Missing:

**ASK THE USER** for guidance:

> "The [feature/component] I implemented doesn't appear to be documented. Should I add documentation for it? If so, where should it go?"

Consider suggesting:
- Which existing doc file could be extended
- Whether a new doc file is needed
- What level of detail is appropriate

**Wait for user guidance before adding new documentation.**

### Step 4: Final Verification

Before reporting task completion:

- [ ] Build verification passes (`make build`)
- [ ] Code review passes with **ZERO** Critical, Major, or Minor issues (all must be fixed)
- [ ] All code reviewer suggestions have been presented to user and addressed per their response
- [ ] New pure logic has unit tests
- [ ] No documentation inconsistencies remain (or user has been consulted)
- [ ] No significant undocumented features (or user has been consulted)
- [ ] All changes are coherent and complete

**BLOCKING REQUIREMENT:** If any issues remain unresolved, you MUST NOT report the task as complete. Continue working until all issues are fixed.

## Quick Reference: Documentation Locations

**Note**: This project is in early development. Many of these documentation files may not exist yet. Create them as needed or update `docs/goals.md` for high-level design changes.

| Change Type | Likely Documentation Impact |
|-------------|----------------------------|
| Core concepts (Processes, Domains) | `docs/goals.md` |
| Project vision or philosophy | `docs/goals.md`, `docs/index.md` |
| Technical decisions | `docs/development/adr/` (create new ADR) |
| Rust patterns | `docs/development/rust-coding-guidelines.md` |
| Kernel behavior | `docs/architecture/kernel.md` (when created) |
| Boot process | `docs/architecture/boot.md` (when created) |
| Memory management | `docs/architecture/memory.md` (when created) |
| IPC changes | `docs/architecture/ipc.md` (when created) |
| Capability changes | `docs/architecture/capabilities.md` (when created) |

## Common Scenarios

### Scenario: Simple Bug Fix

Even for bug fixes:
1. Run build verification (`make build`)
2. Run the lona-code-reviewer to verify the fix is correct
3. Fix ALL issues identified in the review (Critical, Major, AND Minor)
   - If multiple resolution approaches exist, ask the user which they prefer
4. Re-run reviewer to verify zero issues remain
5. Consider adding a test for the bug if one doesn't exist
6. Check if the bug was caused by behavior that matches documentation
7. If docs described buggy behavior as correct, ask user about updating

### Scenario: New Feature Implementation

1. Ensure new pure logic has unit tests
2. Run the lona-code-reviewer for comprehensive analysis
3. Fix ALL issues—Critical, Major, AND Minor (including missing tests)
   - If multiple resolution approaches exist, ask the user which they prefer
4. Present ALL suggestions to user; wait for response on each
5. Re-run reviewer to verify zero issues remain
6. Identify which existing docs should mention this feature
7. Ask user: "Should I document the new [feature] in [location]?"

### Scenario: Refactoring

1. Run build verification to ensure existing tests still pass
2. Run the lona-code-reviewer to ensure refactoring is correct
3. Fix ALL issues identified (Critical, Major, AND Minor)
   - If multiple resolution approaches exist, ask the user which they prefer
4. Re-run reviewer to verify zero issues remain
5. Verify behavior hasn't changed (tests should still pass, docs should still be accurate)
6. If behavior DID change, treat as new feature scenario

### Scenario: Documentation-Only Changes

1. Skip code review (no code changes)
2. Verify documentation is internally consistent
3. Check cross-references still work

## Important Reminders

1. **Never skip code review** - Every code change needs the lona-code-reviewer
2. **All tests must pass** - Build verification must succeed before completion
3. **New logic needs tests** - Pure logic requires unit tests; missing tests are Major Issues
4. **ALL issues block completion** - Work is NOT done until ALL issues (Critical, Major, AND Minor) are resolved
5. **No skipping Minor issues** - Minor issues MUST be fixed, not just noted or acknowledged
6. **Ask when multiple options exist** - If there are multiple valid resolution approaches, ask the user which they prefer
7. **Ask for help on unclear issues** - Don't guess; ask the user for guidance
8. **Always consult the user on docs** - Don't update docs without asking
9. **Be specific** - When asking about docs, quote the inconsistency
10. **Iterate until clean** - Re-run reviewer after fixes; repeat until zero issues remain
11. **Complete the checklist** - Every step, every time
12. **No unauthorized `#[allow(...)]` directives** - See policy below

## Clippy Allow Directive Policy

**CRITICAL: `#[allow(clippy::...)]` and `#[allow(dead_code)]` directives are FORBIDDEN without explicit user approval.**

### During Code Review

The code reviewer will flag any `#[allow(...)]` directives. These are **Critical Issues** unless:
1. The directive has a `// LINT-EXCEPTION:` comment block documenting approval
2. The directive existed before your changes (but should still be flagged as tech debt)

### If You Need to Suppress a Lint

**DO NOT add `#[allow(...)]` yourself.** Instead:

1. **Stop and explain** - Tell the user which lint is triggered and why
2. **Propose alternatives** - Suggest how to fix the underlying issue:
   - `arithmetic_side_effects` → Use checked/saturating/wrapping arithmetic
   - `indexing_slicing` → Use `.get()` with error handling
   - `cast_possible_truncation` → Use `TryFrom` with error handling
   - `dead_code` → Remove unused code or use feature flags
3. **Ask for approval** - Only if fix is truly impossible, request explicit approval
4. **Document thoroughly** - If approved, use this format:

```rust
// LINT-EXCEPTION: clippy::lint_name
// Reason: <why this cannot satisfy the lint>
// Safety: <what invariants ensure correctness>
#[allow(clippy::lint_name)]
```

### Pre-existing Allow Directives

When reviewing code, note any pre-existing `#[allow(...)]` directives that lack proper documentation. Report these as tech debt to the user.
