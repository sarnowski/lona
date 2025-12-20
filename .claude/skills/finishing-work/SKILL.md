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

This step ensures the implementation actually works in the target environment, not just in host-based unit tests.

### Step 2: Invoke Code Review

Use the Task tool to invoke the `lona-code-reviewer` subagent:

```
subagent_type: lona-code-reviewer
prompt: Perform a full review of all changes in the current repository.
```

### Step 3: Present Findings

Present ALL findings from the reviewer to the user, regardless of severity.

### Step 4: Resolve All Issues

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

### Step 5: Repeat Until Clean

After resolving all issues, return to Step 2 and run another code review.

Continue this loop until the reviewer reports **exactly ZERO issues**.

Only when the issue count is zero is the work considered complete.
