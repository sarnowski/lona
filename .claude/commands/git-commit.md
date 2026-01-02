---
name: git-commit
description: "Analyze repository changes and create well-structured commits using Conventional Commits format.\nDetects independent changes and offers to split them. Focuses on WHY and HOW, not WHAT."
arguments:
  - name: options
    description: "Flags: --staged (only staged), --no-split (skip split detection), --type <type>, --scope <scope>, --breaking"
---
 
# Smart Git Commit Command
 
Create well-structured git commits by analyzing changes, detecting independent modifications, and generating meaningful commit messages that focus on the purpose and approach rather than listing file changes.
 
## Execution Steps
 
### Step 1: Parse Arguments
 
Parse the provided arguments to determine behavior:
 
| Argument | Effect |
|----------|--------|
| `--staged` or `-s` | Only analyze already staged changes (skip unstaged) |
| `--no-split` | Skip split detection, treat all changes as one commit |
| `--type <type>` | Pre-specify commit type (feat, fix, refactor, docs, test, chore, build, ci, perf, style) |
| `--scope <scope>` | Pre-specify scope (e.g., auth, api, ui) |
| `--breaking` | Mark commit as a breaking change |
 
Default behavior (no arguments): Analyze all changes (staged + unstaged), detect splits.
 
### Step 2: Analyze Repository State
 
Run the following git commands to understand the current state:
 
```bash
# Get status overview
git status --porcelain
 
# Get detailed diff of all changes (or staged only if --staged)
git diff          # Unstaged changes
git diff --staged # Staged changes
 
# Get recent commit history for style reference
git log --oneline -10
```
 
Collect and categorize:
- **Modified files**: Files with changes
- **Added files**: New untracked or staged new files
- **Deleted files**: Removed files
- **Renamed files**: Moved or renamed files
 
If there are no changes, inform the user and exit.
 
### Step 3: Detect Independent Changes (unless --no-split)
 
Analyze the changes to identify logically independent modifications that can be safely committed separately.
 
**CRITICAL: Atomicity Requirement**
 
Each proposed commit MUST be atomic - meaning:
1. **Self-contained**: The codebase compiles/works after the commit
2. **No broken dependencies**: If change A uses code from change B, they must be in the same commit OR B committed first
3. **Correct order**: When splitting, commits must be ordered so dependencies are satisfied
 
**Dependency Analysis (do this FIRST):**
 
Before grouping by scope/type, analyze code dependencies:
- Does file A import/use something added in file B? → Same commit or B first
- Does a feature require a utility/helper added elsewhere? → Commit utility first
- Does new code depend on updated dependencies in package.json? → Commit deps first
- Are there interface changes that consumers depend on? → Same commit
 
**Anti-patterns to AVOID:**
- Splitting "feature X" and "utility for feature X" into separate commits where feature X is committed first (broken state)
- Committing a component that imports a not-yet-committed module
- Committing code that uses a function signature that only exists in uncommitted changes
- Separating by directory when directories have cross-dependencies
 
**Safe splitting heuristics (only after confirming no dependencies):**
 
Scope-based (when truly independent):
- Different top-level directories with no cross-imports
- Separate modules that don't interact
- Configuration vs source code (when config doesn't break source)
 
Type-based (when truly independent):
- Feature additions that don't depend on each other
- Bug fix in unrelated area + separate feature
- Documentation-only changes
- Test additions for already-committed code
- Dependency updates (commit BEFORE code using new deps)
 
**For each detected group, identify:**
- The files involved
- The likely commit type
- A brief description of the change purpose
- **Dependencies on other groups** (if any → cannot split or must order)
 
**When dependencies exist between groups:**
- If circular dependencies → must be single commit
- If linear dependencies → propose ordered commits with clear sequence
- When uncertain → default to single commit (safer)
 
### Step 4: Handle Multiple Changes
 
Based on the dependency analysis from Step 3, present one of these scenarios:
 
**Scenario A: Truly independent changes (no dependencies)**
 
If changes are genuinely independent with no cross-dependencies:
 
**Use the AskUserQuestion tool:**
```
Question: "I detected {N} independent changes with no dependencies between them. How would you like to commit them?"
Header: "Commit strategy"
Options:
  1. "Split into {N} commits (recommended)" - "Create separate atomic commits for better history"
  2. "Single commit" - "Combine all changes into one commit"
```
 
**Scenario B: Changes with linear dependencies**
 
If changes depend on each other but can be ordered (A → B → C):
 
Present the dependency chain clearly:
```
I detected {N} related changes with dependencies:
 
  1. [First] chore(deps): update axios to v2.0
     └─ Required by subsequent changes
 
  2. [Depends on #1] feat(api): add retry logic to HTTP client
     └─ Uses new axios interceptor API
 
  3. [Depends on #2] feat(auth): add token refresh on 401
     └─ Uses the new retry logic
```
 
**Use the AskUserQuestion tool:**
```
Question: "These changes have dependencies. How would you like to proceed?"
Header: "Commit strategy"
Options:
  1. "Split into {N} ordered commits" - "Commit in dependency order (1 → 2 → 3)"
  2. "Single commit" - "Combine all into one commit"
```
 
**Scenario C: Changes with circular or complex dependencies**
 
If changes cannot be cleanly separated:
 
```
These changes are interdependent and cannot be safely split:
- src/utils/helper.ts uses types from src/models/user.ts
- src/models/user.ts imports helper from src/utils/helper.ts
 
Recommending: Single commit
```
 
**Use the AskUserQuestion tool:**
```
Question: "These changes have circular dependencies. Commit as single unit?"
Header: "Confirm"
Options:
  1. "Yes, single commit (recommended)" - "Safest option for interdependent changes"
  2. "Let me review the changes" - "I'll manually decide what to stage"
```
 
The "Other" option in all scenarios allows the user to specify custom grouping.
 
If the user chooses to split, process each group as a separate commit in the correct order (Steps 5-7 for each).
 
### Step 5: Generate Commit Message
 
For each commit (or the single combined commit), generate a message following Conventional Commits format:
 
**Format:**
```
<type>(<scope>): <subject>
 
<body>
 
[optional footer]
```
 
**Rules for the subject line:**
- Use imperative mood ("add", "fix", "update", not "added", "fixes", "updated")
- Keep under 50 characters
- Lowercase after the type
- No period at the end
 
**Rules for the body (CRITICAL):**
- Focus on **WHY** the change was made (motivation, problem being solved)
- Explain **HOW** the approach works (strategy, key decisions)
- Do **NOT** list individual file changes (the diff shows that)
- Do **NOT** describe what changed superficially
- Wrap at 72 characters
 
**Commit types:**
| Type | Use when |
|------|----------|
| `feat` | Adding new functionality |
| `fix` | Correcting a bug |
| `refactor` | Restructuring code without changing behavior |
| `docs` | Documentation only changes |
| `test` | Adding or modifying tests |
| `chore` | Maintenance tasks, tooling |
| `build` | Build system or dependencies |
| `ci` | CI/CD configuration |
| `perf` | Performance improvements |
| `style` | Code style (formatting, semicolons) |
 
**If `--breaking` is specified or breaking changes detected:**
- Add an exclamation mark after the type/scope, for example: `feat(api)!: change response format`
- Include `BREAKING CHANGE:` footer explaining the impact
 
**Message quality examples:**
 
BAD (describes WHAT):
```
feat(auth): update auth.ts and add auth.test.ts
 
Modified the authentication service file and added
a new test file for authentication.
```
 
GOOD (explains WHY and HOW):
```
feat(auth): add token refresh to prevent session expiry
 
Implement automatic token refresh 5 minutes before
expiry to ensure uninterrupted user sessions during
long-running tasks. Uses refresh token rotation
with sliding window for enhanced security.
```
 
### Step 6: Present Message for User Approval
 
**ALWAYS** present the generated commit message for user review before committing.
 
**Use the AskUserQuestion tool** for approval:
 
First, display the proposed commit message in a clear format:
 
```
Proposed commit message:
────────────────────────────────────────
<type>(<scope>): <subject>
 
<body>
────────────────────────────────────────
 
Files to be committed:
- path/to/file1.ts (modified)
- path/to/file2.ts (added)
```
 
Then ask for approval:
 
```
Question: "How would you like to proceed with this commit?"
Header: "Commit"
Options:
  1. "Commit" - "Create the commit with this message"
  2. "Edit message" - "I'll provide changes to the message"
```
 
The "Other" option allows the user to provide specific edits or additional context.
 
**If user chooses "Edit message" or provides feedback via "Other":**
- Incorporate their feedback
- Generate a revised message
- Present again for approval (repeat Step 6)
 
### Step 7: Execute the Commit
 
Once approved:
 
1. **Stage the appropriate files:**
   - If committing a subset (split commits), stage only those files
   - Use `git add <files>` for specific files
   - For partial file changes, note that the user may need to use `git add -p`
 
2. **Create the commit:**
   ```bash
   git commit -m "$(cat <<'EOF'
   <type>(<scope>): <subject>
 
   <body>
 
   [footer]
   EOF
   )"
   ```
 
3. **Report success:**
   - Show the commit hash
   - Show a summary of what was committed
 
4. **If there are more commits to process (split scenario):**
   - Continue to the next group
   - Repeat Steps 5-7
 
### Step 8: Final Summary
 
After all commits are complete, provide a summary:
 
```
Commits created:
  abc1234 feat(auth): add token refresh mechanism
  def5678 fix(api): correct rate limit calculation
 
Remaining uncommitted changes: none (or list if any)
```
 
## Edge Cases
 
### No changes detected
Inform the user there's nothing to commit.
 
### Only untracked files
Ask if the user wants to add and commit them.
 
### Merge conflicts present
Warn the user and do not proceed until conflicts are resolved.
 
### User cancels
Gracefully exit without making any changes.
 
### Very large changesets
If there are more than 20 files changed, suggest reviewing the changes first or splitting into smaller commits.
 
## Interaction Style
 
- Be concise in explanations
- Use the AskUserQuestion tool for all user decisions
- Never commit without explicit user approval
- If uncertain about change categorization, ask the user
- Respect user preferences from arguments (--type, --scope, etc.)
