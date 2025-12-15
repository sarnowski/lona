---
name: record-decision
description: Record architectural decisions as ADRs (Architecture Decision Records). Use this skill when the user makes or discusses a significant technical decision such as choosing a database, framework, library, authentication method, API design pattern, deployment strategy, or any decision that affects codebase structure, has long-term consequences, involves trade-offs, or deviates from conventions. Also use when the user says "let's document this decision", "we should record this", "ADR", or "architecture decision".
---

# Record Architecture Decision

This skill helps create well-structured Architecture Decision Records (ADRs) that capture important technical decisions with their full context and reasoning.

## Before You Start

1. Read the project's ADR guide at `docs/development/adr.md` to understand the format and conventions
2. Read the template at `docs/development/adr-template.md` - use this exact structure
3. Check `docs/development/adr/` to find the next available ADR number (use `0001` if empty)

## Information Gathering

Before writing the ADR, ensure you have gathered the following information. If any is missing, ask the user:

### Required Information

1. **Decision Title**: What is being decided? (e.g., "Use PostgreSQL for persistence")

2. **Context**: Ask the user:
   - "What problem or situation prompted this decision?"
   - "What constraints or requirements are we working with?"
   - "What technical, business, or organizational forces are at play?"

3. **The Decision**: Ask the user:
   - "What exactly was decided? Please be specific."
   - "What is explicitly NOT part of this decision?"

4. **Alternatives Considered**: This is critical. Ask:
   - "What other options did you consider?"
   - "For each alternative, why was it not chosen?"
   - If the user says "none" or provides insufficient alternatives, probe deeper:
     - "What would be the obvious alternative approaches?"
     - "Did you consider [suggest relevant alternatives based on context]?"
     - "What would someone unfamiliar with this project expect you to choose?"

5. **Consequences**: Ask the user to think through:
   - "What are the benefits of this decision?"
   - "What are the drawbacks or trade-offs?"
   - "What technical debt might this introduce?"
   - "How does this affect the team or development workflow?"

### Quality Checks

Before writing the ADR, verify:

- [ ] Context explains WHY the decision was needed (not just WHAT)
- [ ] At least 2 alternatives were genuinely considered and documented
- [ ] Both positive AND negative consequences are listed
- [ ] The decision statement uses active voice ("We will...")
- [ ] Future readers with no context can understand the reasoning

## Writing the ADR

1. Determine the next ADR number by listing `docs/development/adr/` (start at `0001` if empty)
2. Create the file as `docs/development/adr/NNNN-short-kebab-title.md`
3. Use the exact structure from `docs/development/adr-template.md`
4. Set status to "Proposed"
5. Set date to today's date in ISO format (YYYY-MM-DD)
6. Fill in all sections with the gathered information
7. Add entry to the ADR Overview table in `docs/development/adr.md`:
   - Add a new row with: ADR number (as link), title, date, and status
   - Keep the table sorted by ADR number
8. Show the user the created ADR for review

## Updating ADR Status

When an ADR status changes (e.g., Proposed → Accepted, or Accepted → Superseded):

1. Update the status in the ADR file itself
2. Update the status in the ADR Overview table in `docs/development/adr.md` to match
3. Both locations must always be in sync

## Conversation Flow

When a user mentions a significant decision, guide them through this process:

1. Acknowledge the decision and confirm it warrants an ADR
2. Ask about context: "What led to this decision?"
3. Confirm the decision: "So the decision is to [X], correct?"
4. Probe for alternatives: "What other approaches did you consider?"
5. Explore consequences: "What are the trade-offs - both benefits and drawbacks?"
6. Summarize what you've gathered and confirm before writing
7. Create the ADR file using the template structure
8. Present the created ADR for the user to review

## When NOT to Create an ADR

Do not suggest an ADR for:

- Bug fixes
- Minor refactoring
- Routine implementation details
- Decisions that can be easily reversed
- Style or formatting choices
