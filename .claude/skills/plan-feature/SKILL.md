---
name: plan-feature
description: Rigorous feature planning with multi-AI challenge. Use this skill when planning any significant feature, whether from the roadmap or ad-hoc requests. Ensures plans are validated against project principles and challenged by external perspectives.
---

# Plan Feature

This skill ensures rigorous, principle-aligned planning for any significant feature.

---

## When to Use

- Planning any feature that touches multiple files or concepts
- Planning roadmap tasks (invoked by `plan-next-task` command)
- Planning ad-hoc features requested by the user
- Any work where getting the design right matters

---

## Step 1: Understand Project Foundations

Read these documents completely before any planning:

**Required reading:**
- `docs/goals/index.md` - The four-pillar vision
- `docs/development/principles.md` - Governing development principles
- `docs/lonala/index.md` - Language specification

**Read based on scope:**
- `docs/development/minimal-rust.md` - If feature might require Rust code
- `docs/development/rust-coding-guidelines.md` - If writing Rust
- `docs/development/lonala-coding-guidelines.md` - If writing Lonala
- `docs/development/testing-strategy.md` - For test planning
- Relevant milestone documents in `docs/roadmap/`

---

## Step 2: Research Existing Code

Before designing anything:

1. **Identify related code**: Search for existing implementations, patterns, and abstractions
2. **Understand dependencies**: What does this feature depend on? What depends on it?
3. **Find precedents**: How have similar features been implemented?
4. **Check the roadmap**: Are there dependencies or related tasks?

Document your findings. Note any inconsistencies with project principles.

---

## Step 3: Draft Your Plan

Create an initial plan covering:

1. **Problem Statement**: What are we solving?
2. **Design Approach**: How will we solve it?
3. **Key Decisions**: What architectural choices are we making?
4. **Lonala-First Check**: Can any part be implemented in Lonala instead of Rust?
5. **Files to Change**: Which files will be created/modified?
6. **Testing Strategy**: How will we verify correctness?
7. **Phasing**: How do we break it into context-window-sized chunks?

Do NOT output this plan yet. Hold it for comparison.

---

## Step 4: Multi-AI Challenge

Launch Gemini and Codex in parallel to create independent plans. Send a **single message** with **two background Bash calls**:

```bash
timeout 900 gemini -m gemini-3-pro-preview "PROMPT"
timeout 900 codex exec -m gpt-5.2 -c model_reasoning_effort=high "PROMPT"
```

Both receive the **identical prompt** (substitute actual values):

```
You are a systems architect planning a feature for the Lona project.

## Project Context
Lona is an OS combining:
- seL4 microkernel (capability-based security, formal verification)
- LISP machine philosophy (runtime introspection, hot-patching, source as distribution)
- Erlang/OTP concurrency (lightweight processes, supervision trees, "let it crash")
- Clojure data philosophy (immutable persistent data structures)

All userland code is written in Lonala (a Clojure dialect). The Rust runtime exists ONLY to provide what Lonala cannot provide for itself.

## Required Reading
Read these files completely:
- docs/goals/index.md (vision and four pillars)
- docs/development/principles.md (governing principles)
- docs/lonala/index.md (language specification)
- docs/development/minimal-rust.md (what belongs in Rust vs Lonala)
- docs/roadmap/index.md (current state and dependencies)

{ADDITIONAL_FILES_TO_READ}

## Feature to Plan
{FEATURE_DESCRIPTION}

## Your Task
1. Read all required documents to understand context
2. Research existing code for patterns and precedents
3. Do web research for best practices (especially seL4, Erlang/OTP, Clojure patterns)
4. Create a comprehensive implementation plan

## Plan Requirements
1. **Problem Statement**: What problem does this solve?
2. **Design Approach**: Detailed technical approach
3. **Lonala-First Analysis**: Justify any Rust code; prefer Lonala
4. **Key Design Decisions**: List decisions with rationale and alternatives considered
5. **Files to Create/Modify**: Specific paths and purpose
6. **Testing Strategy**: How to verify correctness (unit, integration, REPL)
7. **Risks and Mitigations**: What could go wrong?
8. **Phasing**: Break into phases where EACH phase is completable in ONE agent context window

## Phasing Constraint (CRITICAL)
The plan MUST be broken into phases where each phase:
- Is self-contained and completable in a single agent session
- Has clear entry conditions (what must exist before starting)
- Has clear exit conditions (how to verify phase is complete)
- Includes all necessary context so a fresh agent can execute it

## Constraints
- NO backwards compatibility concerns (pre-1.0)
- NO shortcuts or deferred solutions
- CORRECT solutions only
- Lonala-first: if it CAN be Lonala, it MUST be Lonala

Output your complete plan.
```

---

## Step 5: Synthesize Insights

Use `TaskOutput` to collect both responses. Then:

1. **Compare approaches**: What did Gemini and Codex do differently?
2. **Identify gaps**: What did they catch that you missed?
3. **Challenge assumptions**: Where do they disagree with your plan?
4. **Extract best ideas**: What insights improve your design?

Create a synthesis noting:
- Areas of consensus (high confidence)
- Areas of disagreement (need resolution)
- Novel ideas from external plans
- Improvements to incorporate

---

## Step 6: Finalize Plan

Revise your plan incorporating insights from all sources. The final plan must:

1. **Address all disagreements**: Explain why you chose one approach over alternatives
2. **Justify Rust code**: Any native primitives must pass the minimal-rust checklist
3. **Be properly phased**: Each phase MUST fit in one context window
4. **Be self-contained per phase**: A fresh agent with only PLAN.md can execute any phase

---

## Step 7: Write to PLAN.md

Write the finalized plan to `PLAN.md` at repo root.

### Phasing Requirements (CRITICAL)

Each phase in the plan must be structured so that:

1. **A fresh agent can execute it**: The phase description contains ALL context needed
2. **It fits in one context window**: Scope is limited enough to complete without summarization
3. **Entry conditions are explicit**: What files/state must exist before starting
4. **Exit conditions are testable**: How to verify the phase is complete
5. **It references specific files**: Not vague descriptions, but exact paths

### PLAN.md Structure

```markdown
# Plan: {Feature Name}

## Summary
{1-2 sentence overview}

## Problem Statement
{What we're solving}

## Design Approach
{Technical approach with rationale}

## Key Decisions
| Decision | Choice | Rationale | Alternatives Considered |
|----------|--------|-----------|------------------------|
| ... | ... | ... | ... |

## Lonala-First Analysis
{Justification for any Rust code, or confirmation it's all Lonala}

---

## Phase 1: {Descriptive Name}

**Entry Conditions:**
- {What must exist before starting this phase}

**Scope:**
{Detailed description of what this phase accomplishes}

**Files to Create/Modify:**
- `path/to/file.rs` - {what changes and why}
- `path/to/file.lona` - {what changes and why}

**Implementation Steps:**
1. {Specific step with enough detail for a fresh agent}
2. {Next step}
3. ...

**Testing:**
- {How to verify this phase works}

**Exit Conditions:**
- [ ] {Checkable condition}
- [ ] {Another condition}
- [ ] `make test` passes

---

## Phase 2: {Descriptive Name}

**Entry Conditions:**
- Phase 1 exit conditions met
- {Additional requirements}

{Same structure as Phase 1}

---

## Synthesis Notes

### Multi-AI Consensus
{Where all plans agreed - high confidence areas}

### Resolved Disagreements
| Topic | Options Considered | Decision | Rationale |
|-------|-------------------|----------|-----------|
| ... | ... | ... | ... |

### Incorporated Insights
- From Gemini: {specific insight incorporated}
- From Codex: {specific insight incorporated}
```

---

## Step 8: Present to User

Present the plan summary to the user:

1. Overview of the approach
2. Number of phases and estimated scope of each
3. Key design decisions that might need user input
4. Request approval before implementation

If the user has questions or suggests changes, revise the plan accordingly.

---

## Done

Planning is complete when:
1. All required documents have been read
2. Existing code has been researched
3. Gemini and Codex have provided independent plans
4. Insights have been synthesized
5. Final plan is written to PLAN.md with properly scoped phases
6. User has approved the plan
