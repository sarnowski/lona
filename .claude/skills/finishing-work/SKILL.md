---
name: finishing-work
description: MANDATORY skill to invoke before claiming ANY work is complete. Must be used after finishing a new concept, plan, feature implementation, bugfix, or any other deliverable. Ensures verification passes, triggers parallel AI agent reviews, and guarantees all issues are resolved. No exceptions - work is NOT done until this skill completes successfully.
---

# Finishing Work

**THIS SKILL IS MANDATORY. You MUST invoke this skill before claiming ANY work is complete.**

Work includes but is not limited to:
- New concepts or designs
- Implementation plans
- Feature implementations
- Bug fixes
- Refactoring
- Documentation updates
- Any other deliverable

**You are NOT done until this skill completes successfully with zero unresolved issues.**

---

## Phase 0: Plan Verification

### 0.1 Check for Existing Plan

Before proceeding, check if `PLAN.md` exists in the repository root:

```bash
cat PLAN.md 2>/dev/null || echo "NO PLAN FILE"
```

### 0.2 If PLAN.md Exists

**You MUST verify the plan was followed before proceeding:**

1. Read `PLAN.md` completely
2. For EACH item in the plan, self-assess:
   - Did I implement this item?
   - Is my implementation COMPLETE (not partial)?
   - Does my implementation match what was planned?
3. If ANY item is incomplete or missing:
   - **STOP** - do not proceed to Phase 1
   - Go back and complete the missing work
   - Return to this skill when truly done

**Do NOT proceed with verification or review if the plan is not fully implemented.**

### 0.3 Plan Contents for Review

If `PLAN.md` exists, you will include its contents in the review prompt for agents to validate. Save the contents for Phase 2.

---

## Phase 1: Verification Build

### 1.1 Determine If Verification Is Needed

`make verify` is required **only if code files were changed**. Check your changed files:

- **Code changes** (`.rs`, `.toml`, `.c`, `.h`, build scripts, etc.): Run `make verify`
- **Documentation-only changes** (`.md` files, comments-only changes): Skip to Phase 2

### 1.2 Run `make verify` (If Required)

If code files were changed, execute:

```bash
make verify
```

**This MUST pass with ZERO issues.** No warnings treated as acceptable, no "minor" failures ignored.

### 1.3 File Length Check (MANDATORY)

**After `make verify` passes, check the length of ALL changed source files.**

Run the following to get line counts for all changed source files:
```bash
git diff --name-only origin/main...HEAD | xargs -I{} sh -c 'echo "$(wc -l < "{}" 2>/dev/null || echo 0) {}"' | sort -rn
```

For uncommitted changes, also check:
```bash
git diff --name-only HEAD | xargs -I{} sh -c 'echo "$(wc -l < "{}" 2>/dev/null || echo 0) {}"' | sort -rn
```

#### File Length Thresholds

| Lines | Severity | Action Required |
|-------|----------|-----------------|
| **≤ 600** | OK | No action needed |
| **601-800** | WARNING | Should be refactored and split into smaller files |
| **> 800** | CRITICAL | **MUST** be split into smaller files |

#### Exception for 601-800 Line Files

Files between 601-800 lines MAY be acceptable **ONLY IF** the logic is so tightly coupled that splitting would significantly harm readability and understanding. You must:

1. Evaluate whether the file has a single, cohesive responsibility
2. Check if natural split points exist (separate concerns, distinct functionality)
3. Consider if splitting would require excessive cross-file dependencies

**If natural split points exist, the file MUST be split.** The exception is narrow: only for files where every function genuinely depends on understanding the whole.

#### Reporting File Length Issues

For each file exceeding thresholds, report to the user:

```
FILE LENGTH ISSUE [WARNING/CRITICAL]:
- File: <path>
- Lines: <count>
- Threshold exceeded: <600 for warning, 800 for critical>
- Natural split points: <identified concerns that could be separate files>
- Recommendation: <specific suggestion for how to split>
```

For files in the 601-800 range that you deem acceptable:
```
FILE LENGTH EXCEPTION [ACCEPTED]:
- File: <path>
- Lines: <count>
- Justification: <why splitting would harm readability>
```

#### If Critical File Length Issues Exist

**Files over 800 lines are BLOCKING.** You MUST:

1. **STOP** - do not proceed to Phase 2 (agent review)
2. Refactor and split the oversized file(s)
3. Run `make verify` again after splitting
4. Re-check file lengths
5. Only proceed when no files exceed 800 lines

**Do NOT proceed to agent review with any file over 800 lines.**

### 1.4 Run `make docs` (Always Required)

Documentation must build successfully. Execute:

```bash
make docs
```

**This MUST pass with NO validation errors.** The command runs with `--strict` mode, so any warnings are treated as errors.

Normal output includes:
- `INFO - Cleaning site directory` (OK)
- `INFO - Building documentation to directory: ...` (OK)
- `INFO - Documentation built in X.XX seconds` (OK)

**Failure indicators** (must be fixed):
- `INFO - Doc file '...' contains a link '...', but there is no such anchor` - broken internal link
- `WARNING - ...` - any warning message
- `ERROR - ...` - any error message

Common issues to fix:
- **Incorrect internal links** - fix anchor names or paths in markdown files
- **Missing pages** - add referenced files or remove dead links
- **Invalid syntax** - fix markdown formatting

If `make docs` reports any validation problems, fix all issues before proceeding.

### 1.5 If Verification Fails

If `make verify` or `make docs` fails:
1. Fix ALL reported issues
2. Run the failing command again
3. Repeat until both pass with zero issues/output
4. Only then proceed to manual REPL testing

**Do NOT proceed to agent review with a failing build or documentation errors.**

### 1.6 Manual REPL Testing (MANDATORY for Code Changes)

After `make verify` passes, you MUST perform manual testing using the development REPL:

1. **Restart QEMU** to pick up code changes:
   ```
   mcp__lona-dev-repl__restart(arch="aarch64")
   ```

2. **Perform at least 10 manual tests** using the REPL:
   ```
   mcp__lona-dev-repl__eval(code="<expression>", arch="aarch64")
   ```

3. **Test coverage requirements:**
   - Test the specific functionality you implemented or changed
   - Test edge cases and boundary conditions
   - Test error cases where applicable
   - Test integration with existing functionality

4. **Report to user:** Present the exact commands and results:
   ```
   REPL Test Results:
   1. eval: (+ 1 2) → 3
   2. eval: (def x 42) → #'user/x
   ... (at least 10 tests)
   ```

**Important:** the exactly same commands must be run on `aarch64` as well as on `x86_64` architecture (see `arch` parameter).

**You MUST report the exact REPL commands and their outputs to the user before proceeding to Phase 2.**

This manual verification catches issues that automated tests may miss and validates the implementation works end-to-end in QEMU.

### 1.7 Skip Conditions

You MAY skip `make verify` if:
- Your changes are documentation-only (`.md` files)
- You literally just ran it in the immediately preceding step, it passed, and no code changes occurred since

You MAY skip `make docs` if:
- You literally just ran it in the immediately preceding step, it passed, and no `.md` file changes occurred since

**`make docs` is required for ALL changes** (code or documentation) because code changes may affect documentation links or require documentation updates.

If ANY doubt exists, run the commands. Running unnecessarily is acceptable; skipping incorrectly is not.

---

## Phase 2: Parallel Agent Review

### 2.1 Gather Changed Files

Identify ALL files changed as part of your current task. The scope depends on whether changes are committed:

**For uncommitted changes:**
```bash
git diff --name-only HEAD
git diff --name-only --cached
git status --porcelain
```

**For committed changes (compare to main branch):**
```bash
git diff --name-only origin/main...HEAD
```

**Combined approach (recommended):**
```bash
git diff --name-only origin/main...HEAD
git diff --name-only HEAD
git status --porcelain
```

Collect the complete list of modified, added, and deleted files. The scope is all changes related to the task being finished, whether committed or not.

### 2.2 Launch All Three Agents IN PARALLEL

**CRITICAL: You MUST launch all three agents in a SINGLE message with multiple tool calls. Do NOT run them sequentially.**

Use these exact invocations:

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
- Use `-m gpt-5.2-codex` for **code reviews** (feature implementations, bug fixes, refactoring)
- Use `-m gpt-5.2` for **conceptual reviews** (designs, plans, architecture documents)

### 2.3 The Review Prompt

Each agent must receive a prompt containing:

1. **The list of ALL changed files**
2. **Instructions to read ALL changed files completely**
3. **Instructions to read ALL relevant documentation and code** necessary to assess correctness
4. **The review criteria below**

#### Review Criteria for Agents

The agents MUST assess:

**A. Correctness**
- Does the code do what it claims to do?
- Are there logic errors, off-by-one errors, edge cases not handled?
- Does it integrate correctly with existing code?

**B. KISS (Keep It Simple, Stupid)**
- Is the solution straightforward or unnecessarily complex?
- Are there simpler alternatives?
- Is there over-abstraction?

**C. YAGNI (You Aren't Gonna Need It)**
- Is everything implemented actually needed?
- Are there speculative features or "future-proofing"?
- Is there unused code or dead paths?

**D. Clean Code**
- Do names reveal intent?
- Are functions focused and small?
- Is the code self-documenting?

**E. Completeness - CRITICAL**
The agents MUST check EVERY LINE for:
- **NO hacks** - temporary solutions presented as permanent
- **NO workarounds** - avoiding proper solutions
- **NO dummy code** - placeholder implementations
- **NO no-ops** - functions that do nothing
- **NO deferred functionality** - TODOs, FIXMEs, implement later comments
- **NO partial implementations** - half-finished features
- **NO stubbed functions** - empty or trivially returning functions
- **NO mock data where real data should be** - hardcoded test values in production code

**We only accept REAL and CORRECT code. Completeness and correctness are NEVER sacrificed.**

**F. Documentation Currency**
- Is all affected documentation still accurate?
- Do code changes require documentation updates?
- Are there inconsistencies between docs and implementation?
- Check: CLAUDE.md, README.md, docs/*.md

---

## Phase 3: Issue Synthesis and Verification

### 3.1 Collect All Reports

Use `TaskOutput` to collect results from all three agents once they complete:

```
TaskOutput(task_id="<agent_id>", block=true, timeout=300000)
```

Where `<agent_id>` is returned when you launch each agent. You can collect all three in parallel:
```
TaskOutput(task_id="<claude_id>", block=true, timeout=300000)
TaskOutput(task_id="<gemini_id>", block=true, timeout=300000)
TaskOutput(task_id="<codex_id>", block=true, timeout=300000)
```

**NOTE:** Agents, especially Codex, can take longer than the `TaskOutput` timeout. If `TaskOutput` times out before an agent completes, call it again with the same `task_id` and repeat until the agent finishes. This typically takes 2-3 `TaskOutput` calls for Codex.

### 3.1.1 Handling Agent Failures

If an agent times out or returns an error:
1. Note the failure in the synthesis report
2. Consider re-launching the failed agent once
3. If still failing, proceed with results from the agents that succeeded
4. Document which agent(s) failed in your final report to the user

### 3.2 Synthesize Reports

Combine all findings into a single consolidated report:
- List every issue raised by any agent
- Note which agent(s) raised each issue
- Categorize by: Correctness, KISS, YAGNI, Clean Code, Completeness, Documentation

### 3.3 Verify Each Claimed Issue

**You MUST verify every single issue.** Agents can be wrong. Before acting:
- Read the relevant code yourself
- Confirm the issue actually exists
- Determine if it's a true problem or a misunderstanding

Mark each issue as:
- **CONFIRMED** - issue is real and must be fixed
- **FALSE POSITIVE** - agent misunderstood, explain why

### 3.4 Report to User

Present the verified issues to the user with:
- Issue description
- Location (file:line)
- Which agent(s) identified it
- Your verification result
- Proposed fix (if confirmed)

**After reporting, categorize each confirmed issue:**

- **Routine fixes** (typos, formatting, minor clarifications, small corrections): Proceed immediately to Phase 4 without waiting for approval.
- **Significant changes** (architectural changes, conceptual approach changes, major rewrites, changes that alter the design or behavior): **STOP and ask the user for approval before implementing.** Present the proposed change clearly and wait for confirmation.

The user sees the report for transparency. For routine fixes, proceed automatically. For significant changes, respect that the user needs to approve the direction.

---

## Phase 4: Issue Resolution (MANDATORY LOOP)

### 4.1 ALL Issues MUST Be Resolved

**There are NO exceptions. Every confirmed issue MUST be fixed.**

Ignore these agent qualifiers - they do NOT excuse skipping:
- "optional"
- "nice-to-have"
- "minor"
- "suggestion"
- "could consider"
- "might want to"
- "low priority"
- "non-blocking"

**If an agent raised it and verification confirmed it, FIX IT.**

### 4.2 Completeness Issues Are BLOCKING

**SPECIAL RULE FOR COMPLETENESS ISSUES:**

Completeness issues (placeholders, TODOs, stubs, partial implementations, etc.) are **ALWAYS BLOCKING**. You CANNOT proceed to completion while any completeness issue remains.

This includes:
- Plan items not fully implemented
- `TODO`, `FIXME`, `XXX`, `HACK` comments in changed code
- `unimplemented!()`, `todo!()` macros
- Stub functions with no real logic
- Partial implementations (only some cases handled)
- Hardcoded test data in production code
- No-op functions
- "Will add later" or "temporary" comments

**You must fix ALL of these before the review can pass.**

### 4.3 Implement All Fixes

For each confirmed issue:
1. Implement the fix **COMPLETELY** - no partial fixes
2. Verify the fix is complete and correct
3. Do NOT introduce new issues while fixing
4. Do NOT leave any aspect of the fix for "later"

### 4.4 Issue Resolution Accounting (MANDATORY)

**Before proceeding to re-review, you MUST explicitly account for EVERY confirmed issue.**

#### Severity Is Irrelevant

Once an issue is confirmed as real, its severity does not matter:
- "Minor" issues **MUST** be fixed
- "Small" issues **MUST** be fixed
- "Trivial" issues **MUST** be fixed
- "Style" issues **MUST** be fixed

**There is no category of real issue that can be skipped.**

#### Required Accounting Format

For EVERY confirmed issue from Phase 3, you must provide one of:

**Option A - Fixed:**
```
ISSUE #N: <description>
STATUS: FIXED
FIX: <what you changed, with file:line reference>
```

**Option B - User Approved Skip:**
```
ISSUE #N: <description>
STATUS: SKIPPED WITH USER APPROVAL
USER SAID: <quote the user's explicit approval to skip this specific issue>
```

**There is no Option C.** You cannot skip an issue without explicit user approval.

#### Checklist Before Re-Review

Before proceeding to Phase 4.5 (review loop), verify:

- [ ] Every confirmed issue has an accounting entry above
- [ ] Every entry is either FIXED (with proof) or SKIPPED WITH USER APPROVAL (with quote)
- [ ] Zero issues were silently ignored
- [ ] Zero issues were skipped because they seemed "minor"

**If you cannot check all boxes, STOP and fix the remaining issues.**

### 4.5 MANDATORY Review Loop

**THIS LOOP IS NOT OPTIONAL. You MUST repeat until zero issues remain.**

```
┌─────────────────────────────────────────────────────────────────────┐
│                    MANDATORY FIX-REVIEW LOOP                        │
│                                                                     │
│  ┌─────────────────┐                                                │
│  │ Agent Review    │◄─────────────────────────────────────┐        │
│  │ (Phase 2)       │                                      │        │
│  └────────┬────────┘                                      │        │
│           │                                               │        │
│           ▼                                               │        │
│  ┌─────────────────┐                                      │        │
│  │ Issues Found?   │──── NO ────► EXIT LOOP (Phase 5)    │        │
│  └────────┬────────┘                                      │        │
│           │                                               │        │
│           │ YES                                           │        │
│           ▼                                               │        │
│  ┌─────────────────┐                                      │        │
│  │ Fix ALL Issues  │                                      │        │
│  │ (Phase 4.3)     │                                      │        │
│  └────────┬────────┘                                      │        │
│           │                                               │        │
│           ▼                                               │        │
│  ┌─────────────────┐                                      │        │
│  │ make verify     │                                      │        │
│  │ (Phase 1)       │                                      │        │
│  └────────┬────────┘                                      │        │
│           │                                               │        │
│           └───────────────────────────────────────────────┘        │
│                                                                     │
│  YOU CANNOT EXIT THIS LOOP UNTIL AGENTS REPORT ZERO ISSUES         │
└─────────────────────────────────────────────────────────────────────┘
```

**If ANY source code was changed during issue resolution:**

1. Run `make verify` again - must pass
2. Run `make docs` again - must pass
3. Launch all three agents again for re-review
4. Process their findings
5. If new issues found, fix them and REPEAT
6. Continue until a cycle completes with **ZERO** new issues

**You CANNOT claim completion until agents return zero issues.**

### 4.6 Loop Counter and Escalation

Track how many review cycles you've done:
- Cycle 1: Initial review
- Cycle 2: First fix review
- Cycle 3+: Subsequent fix reviews

**If you reach cycle 5 or more:**
- Stop and inform the user
- Something is fundamentally wrong with the implementation approach
- Discuss whether to continue fixing or reconsider the approach

### 4.7 Documentation-Only Changes

**Documentation-only changes do NOT require a new review cycle** (during the loop in Phase 4.5). However, documentation-only work still requires the initial full workflow (Phases 0-5) - the exemption only applies to the re-review loop when fixing issues.

---

## Phase 5: Completion

### 5.1 Completion Checklist

Work is complete ONLY when ALL of the following are true:

- [ ] **Plan fulfilled** - If `PLAN.md` existed, every item was fully implemented
- [ ] **Zero completeness issues** - No placeholders, TODOs, stubs, or partial implementations
- [ ] `make verify` passes with zero issues (if code was changed)
- [ ] `make docs` passes with zero validation errors (always required)
- [ ] All three agents have reviewed the changes
- [ ] All agent-reported issues have been verified
- [ ] All confirmed issues have been resolved
- [ ] No code changes remain unreviewed
- [ ] Documentation is up-to-date and all links are valid
- [ ] **Final agent review passed with ZERO issues**

### 5.2 Reporting Completion

**Only when the checklist is fully satisfied AND the final agent review returned zero issues may you report to the user that the work is done.**

Your completion report must include:
- Summary of what was implemented
- Number of review cycles completed
- Confirmation that all plan items were fulfilled (if plan existed)
- Confirmation that zero completeness issues remain

---

## Quick Reference: Agent Commands

From CLAUDE.md:

| Agent | Command |
|-------|---------|
| Claude | `Task(subagent_type="reviewer", run_in_background=true, prompt="...")` |
| Gemini | `Bash(run_in_background=true, timeout=600000, command='gemini -m gemini-3-pro-preview "..."')` |
| Codex (code) | `Bash(run_in_background=true, timeout=600000, command='codex exec -m gpt-5.2-codex -c model_reasoning_effort=medium -c hide_agent_reasoning=true -s read-only "..."')` |
| Codex (conceptual) | `Bash(run_in_background=true, timeout=600000, command='codex exec -m gpt-5.2 -c model_reasoning_effort=medium -c hide_agent_reasoning=true -s read-only "..."')` |

**Remember: Launch ALL THREE in a SINGLE message with parallel tool calls.**

---

## Example Review Prompt Template

**Important:** When passing this prompt to shell commands (Gemini, Codex), you must escape inner double quotes or use single quotes within the prompt text. The template below uses escaped quotes where needed.

**CRITICAL:** If `PLAN.md` exists, you MUST include its contents in the review prompt. This is mandatory.

```
Review the following changes for the Lona project.

==== THE PLAN (from PLAN.md) ====
<INSERT FULL CONTENTS OF PLAN.md HERE, or "No plan file exists" if none>
==== END PLAN ====

CHANGED FILES:
- path/to/file1.rs
- path/to/file2.md
- ...

INSTRUCTIONS:
1. Read PLAN.md contents above FIRST (if present)
2. Read ALL changed files completely
3. Read ALL relevant documentation: CLAUDE.md, docs/lonala/index.md, docs/development/rust-coding-guidelines.md
4. Read any other code files necessary to understand the context

REVIEW CRITERIA:

A. PLAN FULFILLMENT (CRITICAL - if plan exists):
   - Was EVERY item in the plan implemented?
   - Is each implementation COMPLETE (not partial)?
   - Does implementation match what was planned?
   - Report: "[PASS/FAIL] Plan Item X: <status>"

B. COMPLETENESS (CRITICAL - check EVERY LINE):
   - Placeholders: unimplemented!(), todo!(), // placeholder
   - TODO/FIXME: Any TODO, FIXME, XXX, HACK, TEMP comments
   - Stub functions: Empty bodies, functions returning defaults without logic
   - Hardcoded values: Magic numbers, test data in production code
   - Partial implementations: Missing cases, happy path only
   - Deferred handling: unwrap() where errors should be handled
   - Future work comments: "will add later", "temporary", "for now"
   - Dummy/mock data: Fake responses in non-test code
   - No-op functions: Functions that do nothing meaningful
   - Workarounds: "workaround for", "hack to fix"

   Report format for EACH issue:
   COMPLETENESS ISSUE [CRITICAL]:
   - File: <path>
   - Line: <number>
   - Pattern: <type>
   - Code: <the code>

C. CORRECTNESS: Logic errors, edge cases, integration issues
D. KISS: Unnecessary complexity, over-abstraction
E. YAGNI: Unneeded features, speculative code, dead paths
F. CLEAN CODE: Naming, function focus, self-documentation
G. DOCUMENTATION: Is all documentation still accurate?

Our core principle: CORRECTNESS OVER SPEED. We NEVER sacrifice completeness.

THE PRIMARY AGENT HAS A HISTORY OF LEAVING INCOMPLETE IMPLEMENTATIONS.
Be EXTREMELY thorough. Be skeptical. Check EVERY line.
Completeness issues are ALWAYS critical - never mark them as "minor".

OUTPUT FORMAT:
## Plan Validation
- [PASS/FAIL] Item 1: <status>
...

## Completeness Issues (CRITICAL)
<list or "None found">

## Other Issues
<categorized list>

## Summary
- Plan items: X/Y fulfilled
- Completeness issues: N (MUST be 0)
- Recommendation: PASS/FAIL
```

---

## Failure Modes to Avoid

**DO NOT:**
- Skip this skill because "the change is small"
- Skip agents because "make verify passed"
- Skip issues because they seem "minor"
- Skip re-review because "I only fixed one thing"
- Claim completion before all phases finish
- Run agents sequentially instead of in parallel
- Accept partial fixes
- Leave TODOs for "later"
- Exit the review loop while issues remain
- Delete `PLAN.md` before work is complete
- Omit the plan from review prompts
- Downgrade completeness issues to "minor"

### SPECIFICALLY PROHIBITED BEHAVIORS

The following are **ABSOLUTE VIOLATIONS** that have occurred in the past and **MUST NEVER happen again**:

| Violation | Why It's Unacceptable |
|-----------|----------------------|
| Leaving `TODO` comments | Admits work is incomplete |
| Using `unimplemented!()` | Defers required functionality |
| Writing stub functions | Pretends to implement without doing the work |
| Partial implementations | Claims completion when cases are missing |
| "Will add later" comments | Acknowledges plan not followed |
| Skipping plan items | Violates explicit agreement with user |
| Claiming done with issues remaining | Breaks trust, wastes user time |
| Exiting review loop early | Allows defects to slip through |
| Skipping "minor" issues | Severity is irrelevant; all confirmed issues must be fixed |

**If you find yourself doing ANY of these, STOP. Go back and do the work properly.**

### The Prime Directive

**CORRECTNESS OVER SPEED. COMPLETENESS IS NON-NEGOTIABLE.**

Incomplete work that appears done is worse than no work at all. It:
- Wastes the user's time (they discover problems later)
- Creates hidden technical debt
- Breaks trust with the user
- Violates the project's core principles

**If you cannot complete something fully, STOP and discuss with the user. Never fake completion.**

**REMEMBER:** This skill exists because correctness matters more than speed. Follow it completely, every time.
