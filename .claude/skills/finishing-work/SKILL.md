---
name: finishing-work
description: MANDATORY skill to invoke before claiming ANY work is complete. Ensures verification passes, triggers parallel AI agent reviews, and guarantees all issues are resolved. No exceptions - work is NOT done until this skill completes successfully.
---

# Finishing Work

**THIS SKILL IS MANDATORY. You MUST invoke this skill before claiming ANY work is complete.**

Work includes: concepts, plans, implementations, bug fixes, refactoring, documentation updates, or any other deliverable.

```
┌─────────────────────────────────────────────────────────────────┐
│                         WORKFLOW                                │
│                                                                 │
│  Phase 0: Plan Verification                                     │
│      ↓                                                          │
│  Phase 1: Build & Test (make verify, make docs, REPL)           │
│      ↓                                                          │
│  Phase 2: Parallel Agent Review (3 agents)                      │
│      ↓                                                          │
│  Phase 3: Issue Resolution ←──────────────────────┐             │
│      ↓                                            │             │
│  Issues remain? ──── YES ─── Fix all ─── Re-run ──┘             │
│      │                                                          │
│      NO                                                         │
│      ↓                                                          │
│  Phase 4: Completion                                            │
└─────────────────────────────────────────────────────────────────┘
```

---

## Phase 0: Plan Verification

### 0.1 Check for Plan

```bash
cat PLAN.md 2>/dev/null || echo "NO PLAN FILE"
```

### 0.2 If PLAN.md Exists

**You MUST verify the plan was followed before proceeding:**

1. Read `PLAN.md` completely
2. For EACH item, self-assess:
   - Did I implement this item?
   - Is my implementation COMPLETE (not partial)?
   - Does my implementation match what was planned?
3. If ANY item is incomplete or missing:
   - **STOP** - do not proceed
   - Go back and complete the missing work
   - Return when truly done

Save the plan contents for Phase 2 (agents will validate against it).

---

## Phase 1: Build & Test

### 1.1 Determine Required Verification

Check which directories have changes:

```bash
.claude/skills/finishing-work/changed-directories.sh
```

| Changed Directory | Required Command | Must Result In |
|-------------------|------------------|----------------|
| `src/` or `lib/`  | `make verify`    | ZERO issues    |
| `docs/`           | `make docs`      | ZERO issues    |
| Both              | Both commands    | ZERO issues each |
| Neither           | Skip to 1.4      | N/A            |

### 1.2 Run `make verify` (If src/ or lib/ Changed)

```bash
make verify
```

**MUST pass with ZERO issues.** No warnings acceptable, no failures ignored.

If it fails: fix ALL issues, re-run until it passes.

### 1.3 Run `make docs` (If docs/ Changed)

```bash
make docs
```

**MUST pass with ZERO validation errors.** The command runs with `--strict` mode.

If it fails: fix ALL issues (broken links, invalid syntax, missing pages), re-run until it passes.

### 1.4 File Length Check

**Check the length of ALL changed source files.**

```bash
.claude/skills/finishing-work/file-line-counts.sh
```

| Lines | Action |
|-------|--------|
| ≤ 600 | OK |
| 601-800 | Should split unless logic is tightly coupled |
| > 800 | **BLOCKING** - must split before proceeding |

### 1.5 Manual REPL Testing (If Code Changed)

After `make verify` passes:

1. **Restart QEMU** to pick up code changes:
   ```
   mcp__lona-dev-repl__restart(arch="aarch64")
   mcp__lona-dev-repl__restart(arch="x86_64")
   ```

2. **Perform at least 10 manual tests** on EACH architecture:
   ```
   mcp__lona-dev-repl__eval(code="<expression>", arch="aarch64")
   mcp__lona-dev-repl__eval(code="<expression>", arch="x86_64")
   ```

3. **Test coverage:**
   - The specific functionality you implemented
   - Edge cases and boundary conditions
   - Error cases where applicable
   - Integration with existing functionality

4. **Report to user:**
   ```
   REPL Test Results (aarch64):
   1. (+ 1 2) → 3
   2. (def x 42) → #'user/x
   ... (at least 10 tests)

   REPL Test Results (x86_64):
   1. (+ 1 2) → 3
   ... (same tests)
   ```

### 1.6 Test Coverage Verification (MANDATORY)

**Verify appropriate tests exist for all changes.**

#### Identify Changed Source Files

```bash
.claude/skills/finishing-work/changed-rust-source-files.sh
```

#### For Each Changed Source File

1. **Check for corresponding test file:**
   ```bash
   # For src/foo/bar.rs, check for src/foo/bar_test.rs
   ls src/foo/bar_test.rs 2>/dev/null || echo "NO TEST FILE"
   ```

2. **Check if tests were added/modified:**
   ```bash
   git diff --name-only HEAD | grep '_test\.rs$'
   ```

#### Test Coverage Requirements

| Change Type | Required Tests | How to Verify |
|-------------|----------------|---------------|
| New function | Unit tests exist | `grep -l "fn test_" in corresponding `_test.rs` |
| New feature | Unit + integration | Check both `_test.rs` and `tests/` |
| Bug fix | Regression test | `grep -r "regression_" *_test.rs` |
| Refactoring | Tests unchanged | `git diff *_test.rs` shows no changes needed |

#### Bug Fix Verification (STRICT)

If this change is a bug fix:

1. **A regression test MUST exist**
2. **The test name should identify the bug** (e.g., `regression_off_by_one_nth`)
3. **Verify the test actually tests the fix:**
   - Read the regression test
   - Confirm it would have failed before the fix
   - Confirm it passes after the fix

```bash
# Search for regression tests in changed test files
git diff --name-only HEAD | xargs grep -l 'fn regression_' 2>/dev/null
```

If no regression test exists for a bug fix → **BLOCKING**. Do not proceed.

#### Report Test Gaps

For each gap found:

```
TEST COVERAGE ISSUE [BLOCKING]:
- File: src/feature/impl.rs
- Function: new_function()
- Issue: No unit tests for this new function
- Required: Add tests to src/feature/impl_test.rs
```

**All test coverage issues are BLOCKING.** Fix them before proceeding to Phase 2.

---

## Phase 2: Parallel Agent Review

### 2.1 Gather Changed Files

```bash
.claude/skills/finishing-work/changed-files.sh
```

### 2.2 Launch All Three Agents IN PARALLEL

**CRITICAL: Launch all three in a SINGLE message with multiple tool calls.**

**Claude Reviewer:**
```
Task(subagent_type="reviewer", run_in_background=true, prompt="<REVIEW_PROMPT>")
```

**Gemini:**
```
Bash(run_in_background=true, timeout=600000, command='gemini -m gemini-3-pro-preview "<REVIEW_PROMPT>"')
```

**Codex:**
```
Bash(run_in_background=true, timeout=600000, command='codex exec -m <MODEL> -c model_reasoning_effort=medium -c hide_agent_reasoning=true -s read-only "<REVIEW_PROMPT>"')
```

**Codex Model Selection:**
- `-m gpt-5.2-codex` for code reviews
- `-m gpt-5.2` for conceptual reviews (designs, plans, docs)

### 2.3 The Review Prompt

The reviewer agent has full review criteria in its instructions. Your prompt should contain:

```
Review the following changes for the Lona project.

==== PLAN (from PLAN.md) ====
<INSERT FULL CONTENTS OF PLAN.md HERE, or "No plan file exists">
==== END PLAN ====

CHANGED FILES:
- path/to/file1.rs
- path/to/file2.md
...

VERIFICATION STATUS:
- make verify: [PASSED/SKIPPED - no src/ or lib/ changes]
- make docs: [PASSED/SKIPPED - no docs/ changes]

INSTRUCTIONS:
1. Read the plan above (if present) - validate implementation against it
2. Read ALL changed files completely
3. Read relevant documentation and related code for context
4. Perform your full review per your instructions
5. Output in your standard format
```

**Note:** The reviewer agent knows the review criteria, completeness patterns, and output format. Do not duplicate them in the prompt.

---

## Phase 3: Issue Resolution

### 3.1 Collect Reports

Use `TaskOutput` to collect results:

```
TaskOutput(task_id="<agent_id>", block=true, timeout=300000)
```

**If timeout:** Call `TaskOutput` again with the same `task_id`. Repeat until complete (typically 2-3 calls for Codex).

**If agent fails:** Note the failure, consider re-launching once. If still failing, proceed with successful agents and document which failed.

### 3.2 Synthesize and Verify

1. Combine all findings into a consolidated report
2. Note which agent(s) raised each issue
3. **Verify each issue yourself** - agents can be wrong:
   - Read the relevant code
   - Confirm the issue exists
   - Mark as **CONFIRMED** or **FALSE POSITIVE** (with explanation)

### 3.3 Documentation Conflicts Require User Decision

**Documentation conflicts are SPECIAL.** When a reviewer reports `DOCUMENTATION CONFLICT [BLOCKING]`:

1. **STOP** - do not fix it yourself
2. Present to user:
   ```
   DOCUMENTATION CONFLICT - USER DECISION REQUIRED:

   Documentation says: <X>
   Implementation does: <Y>

   Which is correct?
   A) Documentation → I will fix the implementation
   B) Implementation → I will fix the documentation
   C) Neither → Please clarify intended behavior
   ```
3. Wait for user response
4. Implement user's decision
5. Continue with review loop

**Never assume implementation is correct. Never assume documentation is correct. Ask.**

### 3.4 Report to User

Present verified issues:
- Issue description
- Location (file:line)
- Which agent(s) identified it
- Your verification result
- Proposed fix (if confirmed)

**Categorize confirmed issues:**
- **Routine fixes** (typos, formatting, small corrections): Proceed immediately
- **Significant changes** (architectural, behavioral): Ask user approval first

### 3.5 Fix All Confirmed Issues

**ALL confirmed issues MUST be fixed. No exceptions.**

Ignore these agent qualifiers - they do NOT excuse skipping:
- "optional", "nice-to-have", "minor", "suggestion"
- "could consider", "might want to", "low priority"

**If it's confirmed, fix it.**

For each issue:
1. Implement the fix COMPLETELY
2. Verify fix is correct
3. Do NOT introduce new issues

### 3.6 Issue Accounting (MANDATORY)

Before re-review, account for EVERY confirmed issue:

**Fixed:**
```
ISSUE #N: <description>
STATUS: FIXED
FIX: <what you changed, file:line>
```

**Skipped (requires explicit user approval):**
```
ISSUE #N: <description>
STATUS: SKIPPED WITH USER APPROVAL
USER SAID: <quote user's approval>
```

There is no other option. You cannot skip without user approval.

### 3.7 Review Loop

**If ANY source code changed during issue resolution:**

1. Run `make verify` (if src/ or lib/ changed) - must pass
2. Run `make docs` (if docs/ changed) - must pass
3. Launch all three agents again
4. Process findings
5. If new issues found, fix and REPEAT
6. Continue until **ZERO new issues**

```
┌─────────────────────────────────────────────────────────────────┐
│                    MANDATORY REVIEW LOOP                        │
│                                                                 │
│  Agent Review ──► Issues? ──NO──► EXIT (Phase 4)               │
│       ▲              │                                          │
│       │             YES                                         │
│       │              │                                          │
│       │              ▼                                          │
│       └─── verify ◄── Fix ALL                                   │
│                                                                 │
│  EXIT ONLY WHEN AGENTS REPORT ZERO ISSUES                       │
└─────────────────────────────────────────────────────────────────┘
```

**Loop escalation:** If you reach cycle 5+, stop and discuss with user. Something is fundamentally wrong.

---

## Phase 4: Completion

### 4.1 Completion Checklist

Work is complete ONLY when ALL are true:

- [ ] Plan fulfilled (if PLAN.md existed, every item implemented)
- [ ] Zero completeness issues (no placeholders, TODOs, stubs)
- [ ] `make verify` passed (if src/ or lib/ changed)
- [ ] `make docs` passed (if docs/ changed)
- [ ] **Test coverage verified** (all new code has tests, bug fixes have regression tests)
- [ ] All three agents reviewed
- [ ] All confirmed issues resolved
- [ ] Documentation is accurate and current
- [ ] **Final agent review: ZERO issues**

### 4.2 Report Completion

Only when checklist is satisfied AND final review returned zero issues:

```
WORK COMPLETE:
- Summary: <what was implemented>
- Review cycles: <N>
- Plan items fulfilled: <X/Y or N/A>
- Final review: PASSED (zero issues)
```

Delete `PLAN.md` after completion (plan is fulfilled).

---

## Prohibited Behaviors

| Behavior | Why Prohibited |
|----------|----------------|
| Skip this skill ("change is small") | All work requires verification |
| Skip agents ("make verify passed") | Agents catch what automation misses |
| Skip issues ("seems minor") | Severity is irrelevant; confirmed = fix |
| Skip re-review ("only fixed one thing") | Any change can introduce issues |
| Claim done before Phase 4 | Premature completion breaks trust |
| Run agents sequentially | Must be parallel for efficiency |
| Leave TODOs "for later" | Incomplete work is unacceptable |
| Exit loop with issues remaining | Zero issues is the only exit |
| Assume docs are wrong | User decides doc vs implementation |
| Assume implementation is wrong | User decides doc vs implementation |
| Skip test verification | Code without tests is incomplete |
| Ship bug fix without regression test | Bug will likely return |
| Accept coverage theater | Tests must actually test new code |

### The Prime Directive

**CORRECTNESS OVER SPEED. COMPLETENESS IS NON-NEGOTIABLE.**

If you cannot complete something fully, STOP and discuss with the user. Never fake completion.
