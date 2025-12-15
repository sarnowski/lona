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

For each finding from the review:

- **If the solution is obvious**: Fix the issue immediately
- **If the solution is not obvious**:
  1. Explain the issue to the user
  2. Provide 2-3 options for how to solve it
  3. Include your recommendation
  4. Wait for user feedback before proceeding

ALL findings must be addressed, including minor ones. Do not skip any issues.

### Step 4: Repeat Until Clean

After resolving all issues, return to Step 1 and run another review.

Continue this loop until the reviewer finds ZERO issues.

Only then is the work considered complete.
