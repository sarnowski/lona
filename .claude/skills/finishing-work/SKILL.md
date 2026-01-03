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

### 1.3 Run `make docs` (Always Required)

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

### 1.4 If Verification Fails

If `make verify` or `make docs` fails:
1. Fix ALL reported issues
2. Run the failing command again
3. Repeat until both pass with zero issues/output
4. Only then proceed to manual REPL testing

**Do NOT proceed to agent review with a failing build or documentation errors.**

### 1.5 Manual REPL Testing (MANDATORY for Code Changes)

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

**You MUST report the exact REPL commands and their outputs to the user before proceeding to Phase 2.**

This manual verification catches issues that automated tests may miss and validates the implementation works end-to-end in QEMU.

### 1.6 Skip Conditions

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

## Phase 4: Issue Resolution

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

### 4.2 Implement All Fixes

For each confirmed issue:
1. Implement the fix
2. Verify the fix is complete and correct
3. Do NOT introduce new issues while fixing

### 4.3 Loop Back If Code Changed

**If ANY source code was changed during issue resolution:**

1. Return to **Phase 1** - run `make verify` again
2. Return to **Phase 2** - launch all three agents again for re-review
3. Continue until a cycle completes with zero new issues

**Documentation-only changes do NOT require a new review cycle** (during the loop in Phase 4.3). However, documentation-only work still requires the initial full workflow (Phases 1-5) - the exemption only applies to the re-review loop when fixing issues.

---

## Phase 5: Completion

### 5.1 Completion Checklist

Work is complete ONLY when ALL of the following are true:

- [ ] `make verify` passes with zero issues (if code was changed)
- [ ] `make docs` passes with zero output (always required)
- [ ] All three agents have reviewed the changes
- [ ] All agent-reported issues have been verified
- [ ] All confirmed issues have been resolved
- [ ] No code changes remain unreviewed
- [ ] Documentation is up-to-date and all links are valid

### 5.2 Reporting Completion

**Only when the checklist is fully satisfied may you report to the user that the work is done.**

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

```
Review the following changes for the Lona project.

CHANGED FILES:
- path/to/file1.rs
- path/to/file2.md
- ...

INSTRUCTIONS:
1. Read ALL changed files completely
2. Read ALL relevant documentation: CLAUDE.md, docs/concept.md, docs/lonala.md, docs/rust.md
3. Read any other code files necessary to understand the context

REVIEW CRITERIA:
Assess each changed file for:

A. CORRECTNESS: Logic errors, edge cases, integration issues
B. KISS: Unnecessary complexity, over-abstraction
C. YAGNI: Unneeded features, speculative code, dead paths
D. CLEAN CODE: Naming, function focus, self-documentation
E. COMPLETENESS: Check EVERY LINE for:
   - Hacks, workarounds, dummy code, no-ops
   - Deferred functionality (TODO, FIXME, implement later)
   - Partial implementations, stubbed functions
   - Mock/hardcoded data where real data should be
F. DOCUMENTATION: Is all documentation still accurate?

Our core principle: CORRECTNESS OVER SPEED. We never sacrifice completeness.

Report ALL issues found, even if minor. Categorize by criteria above.
Include file path and line numbers for each issue.
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

**REMEMBER:** This skill exists because correctness matters more than speed. Follow it completely, every time.
