---
name: finishing-work
description: Mandatory workflow to complete any implementation work. Use this skill BEFORE claiming success, being done, or finishing any coding task. Ensures all changes pass code review.
---

# Finishing Work

This skill enforces the mandatory review workflow before any work can be considered complete.

You MUST NOT claim success or completion without following this workflow.

## Mandatory Workflow

### Step 1: Invoke Code Review

Use the Task tool to invoke the `lona-code-reviewer` subagent:

```
subagent_type: lona-code-reviewer
prompt: Perform a full review of all changes in the current repository.
```

### Step 2: Present Findings

Present ALL findings from the reviewer to the user, regardless of severity.

### Step 3: Resolve All Issues

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
- You MUST NOT disable any clippy check at any level (file, module, crate, or workspace).
- A pre-tool hook automatically blocks any attempt to add suppression directives without proper approval.
- If a clippy issue cannot be correctly resolved:
  1. Explain the issue to the user in detail
  2. Describe why a standard fix is not possible
  3. Provide your recommendation for how to handle it
  4. Wait for the user's EXPLICIT approval before taking any action
- NEVER use `#[allow(...)]`, `#[expect(...)]`, `#[cfg_attr(..., allow(...))]`, `#[cfg_attr(..., expect(...))]`, or modify clippy.toml to suppress warnings without explicit user approval.
- Do not assume approval. The user MUST explicitly approve ANY clippy exception.
- **Approved Directive Format**: When the user explicitly approves, include `[approved]` in the reason:
  ```rust
  #[expect(clippy::lint_name, reason = "[approved] explanation")]
  ```

### Step 4: Repeat Until Clean

After resolving all issues, return to Step 1 and run another review.

Continue this loop until the reviewer reports **exactly ZERO issues**.

Only when the issue count is zero is the work considered complete.
