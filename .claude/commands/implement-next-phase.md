Implement the next open phase from PLAN.md.

## Step 1: Read the Plan

Read `PLAN.md` at the repository root completely.

If PLAN.md does not exist, inform the user and suggest running `/plan-next-task` first.

## Step 2: Find Next Open Phase

Scan for phase headers with `[OPEN]` status:

```
## Phase N: {Title} [OPEN]
```

Select the **first** phase marked `[OPEN]`. This is the next phase to implement.

If no phases are `[OPEN]`:
- Check if all phases are `[DONE]` → Plan is complete, inform user
- Check if remaining phases are blocked → Inform user of the blocker

## Step 3: Present Phase to User

Present the selected phase:

```
## Next Phase: Phase {N} - {Title}

**Scope:** {One-line summary from the phase}

**Files to modify:**
- {List from phase}

**Entry conditions:**
- {List from phase}

Ready to implement this phase?
```

Wait for user confirmation before proceeding.

## Step 4: Verify Entry Conditions

Before implementing, verify all entry conditions are met:

1. Check that prerequisite phases are marked `[DONE]`
2. Verify required files/state exist
3. Run `make test` to confirm baseline is green

If entry conditions are not met, inform the user and stop.

## Step 5: Implement the Phase

Follow the implementation steps from the phase description.

**Invoke the appropriate development skill:**
- If phase involves Rust code → Use `develop-rust` skill
- If phase involves Lonala code → Use `develop-lonala` skill
- If phase involves both → Use `develop-rust` first, then `develop-lonala`

Work through each implementation step in order. Use the TodoWrite tool to track progress through the steps.

## Step 6: Verify Exit Conditions

After implementation, verify ALL exit conditions from the phase:

1. Check each exit condition checkbox
2. Run `make test` and confirm it passes
3. Verify any phase-specific tests pass

If any exit condition fails, fix the issue before proceeding.

## Step 7: Mark Phase Complete

Once all exit conditions are verified:

1. Edit `PLAN.md` to change the phase status from `[OPEN]` to `[DONE]`:

   ```
   ## Phase N: {Title} [DONE]
   ```

2. Report completion to user:

   ```
   ## Phase {N} Complete

   **Completed:** {Title}

   **Exit conditions verified:**
   - [x] {condition 1}
   - [x] {condition 2}
   - [x] `make test` passes

   **Next phase:** Phase {N+1} - {Title} [OPEN]
   ```

## Step 8: Finish Work

Invoke the `finishing-work` skill to complete the workflow with code review.

## Done

Phase implementation is complete when:
1. All implementation steps are done
2. All exit conditions are verified
3. Phase is marked `[DONE]` in PLAN.md
4. `finishing-work` skill has completed successfully
