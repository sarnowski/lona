Select and plan the next open task from the roadmap.

## Step 1: Read Roadmap

Read `docs/roadmap/index.md` completely to understand:
- Current project state (what's done vs open)
- Task dependencies
- The next logical task to implement

## Step 2: Select Next Task

Identify the next open task by:
1. Finding the first `open` task in the roadmap
2. Verifying its dependencies are `done`
3. Reading the relevant milestone document for full task details

If dependencies are incomplete, flag this and select the appropriate dependency instead.

## Step 3: Confirm Selection

Present the selected task to the user:

```
Next task: {Task ID} - {Task Name}

From: {Milestone Name}

Description: {Brief description from milestone doc}

Dependencies: {List any, or "None"}
```

Ask if the user wants to proceed with planning this task.

## Step 4: Invoke Plan Feature Skill

Once confirmed, invoke the `plan-feature` skill with the selected task as the feature to plan.

The skill will:
- Read all required project documents
- Research existing code
- Get independent plans from Gemini and Codex
- Synthesize insights
- Write a phased plan to PLAN.md

## Done

Selection is complete when the user confirms the task and `plan-feature` is invoked.
