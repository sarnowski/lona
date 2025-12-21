---
name: finishing-work
description: Mandatory workflow to complete any implementation work. Use this skill BEFORE claiming success, being done, or finishing any coding task. Ensures all changes pass code review.
---

# Finishing Work

This skill enforces the mandatory review workflow before any work can be considered complete.

You MUST NOT claim success or completion without following this workflow.

## Mandatory Workflow

### Step 1: Manual Verification in seL4

Before code review, verify the changes work correctly in the real seL4 environment:

1. **Use `mcp__lona-dev-repl__restart`** to rebuild Lona and start a fresh QEMU instance with the latest code changes

2. **Identify new features** by reviewing the changed files (use `git diff` or `git status`)

3. **Perform manual tests** using `mcp__lona-dev-repl__eval` to verify the new functionality:
   - Test the happy path for each new feature
   - Test edge cases and error conditions
   - Verify that error messages are helpful and accurate

4. **Document any issues found** and fix them before proceeding to code review

5. **Report ALL tests to the user** (see format below)

This step ensures the implementation actually works in the target environment, not just in host-based unit tests.

#### REPL Test Reporting (MANDATORY)

After completing manual tests, you MUST present a summary to the user in exactly this format:

```markdown
## Manual REPL Verification

### Test Results

| # | Expression | Expected | Actual | Status |
|---|------------|----------|--------|--------|
| 1 | `(+ 1 2)` | `3` | `3` | ✓ |
| 2 | `(/ 1 0)` | Error | `Error: Division by zero` | ✓ |
| ... | ... | ... | ... | ... |

### Expressions for Copy/Paste

​```clojure
;; Test 1: Basic addition
(+ 1 2)

;; Test 2: Division by zero error handling
(/ 1 0)

;; ... additional tests
​```
```

**Requirements:**
- Report EVERY expression you evaluated, not just failures
- Include the actual REPL output verbatim (do not paraphrase)
- Use ✓ for passing tests, ✗ for failures
- Provide a copy-paste block with all expressions so the user can reproduce your tests
- If any test fails unexpectedly, fix the issue and re-run ALL tests before proceeding

### Step 2: Update Progress Tracking

Before code review, update the roadmap and plan to reflect completed work:

#### 2.1: Identify Completed Tasks

1. **Review what was implemented** by checking `git diff` against the last commit
2. **Cross-reference with the roadmap** (`docs/roadmap/index.md`) to identify which task(s) were completed
3. **Verify completion criteria** - ensure all aspects of the task are actually done, not just started

#### 2.2: Update the Roadmap

For each completed task in `docs/roadmap/index.md`:

1. **Find the task row** in the appropriate milestone section
2. **Change status from `open` to `done`**

Example change:
```markdown
# Before
| 1.1.6 | Metadata System - Reader Syntax | open |

# After
| 1.1.6 | Metadata System - Reader Syntax | done |
```

**Rules:**
- Only mark a task as `done` if ALL acceptance criteria are met
- If a task is partially complete, leave it as `open` and document what remains
- If multiple tasks were completed, update all of them

#### 2.3: Update or Archive the Plan

If a `PLAN.md` file exists at the repository root:

1. **If the plan corresponds to completed work**: Delete the file (the plan has served its purpose)
2. **If the plan covers multiple tasks and some remain**: Update the plan to mark completed sections

**Rationale:** Plans are temporary working documents. Once work is complete, the roadmap serves as the authoritative record.

### Step 3: Invoke Code Review

Use the Task tool to invoke the `lona-code-reviewer` subagent:

```
subagent_type: lona-code-reviewer
prompt: Perform a full review of all changes in the current repository.
```

### Step 4: Present Findings

Present ALL findings from the reviewer to the user, regardless of severity.

### Step 5: Resolve All Issues

**CRITICAL: You MUST resolve EVERY issue before proceeding. No exceptions.**

For each finding from the review:

- **If the solution is obvious**: Fix the issue immediately
- **If the solution is not obvious or requires significant effort**:
  1. Explain the issue to the user
  2. Provide 2-3 options for how to solve it
  3. Include your recommendation
  4. Wait for user feedback before proceeding

**Rules:**
- ALL findings must be addressed. There are no "optional" or "minor" issues.
- Do NOT skip issues because they seem small or cosmetic.
- Do NOT defer issues to "future improvements" or "later".
- Do NOT claim completion if ANY issues remain unresolved.
- If unsure how to fix an issue, ASK the user for guidance.

**Clippy Policy:**

See **CLAUDE.md Clippy Policy section** for the complete policy. Key points:
- Never disable clippy checks without explicit user approval
- A pre-tool hook blocks unauthorized suppression directives
- Approved directives must include `[approved]` in the reason string

### Step 6: Repeat Until Clean

After resolving all issues, determine whether to re-run the code review:

#### Documentation-Only Issues

If ALL findings from the review were **documentation issues** (e.g., outdated docs, incorrect examples, missing doc updates), then:
- Fix the documentation issues
- **Skip re-running the code reviewer** - proceed directly to Step 7 (Documentation Verification)

Documentation issues include:
- Outdated or incorrect documentation
- Missing documentation for new features
- Inconsistent examples or descriptions
- Typos or formatting issues in docs

#### Code Issues

If ANY findings involved **code issues** (bugs, style violations, missing tests, architectural concerns, etc.), then:
- Fix all issues
- **Return to Step 3** and run another code review
- Continue this loop until the reviewer reports **exactly ZERO issues**

**Rationale:** Documentation fixes are straightforward and don't introduce new bugs. Code changes require verification to ensure the fix didn't introduce new problems.

### Step 7: Documentation Verification

After the code review passes with zero issues, verify documentation builds cleanly:

1. **Run `make docs`** to build the documentation
2. **Check for warnings** - the build must complete with **zero warnings**
3. **Fix any warnings** before proceeding:
   - Broken links
   - Missing files
   - Invalid markdown syntax
   - Any other documentation issues

If warnings are found, fix them and re-run `make docs` until clean.

Only when both the code review AND documentation build pass with zero issues is the work considered complete.
