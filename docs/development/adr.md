# Architecture Decision Records

Architecture Decision Records (ADRs) capture important architectural decisions made during the development of this project, along with their context and consequences.

## What is an ADR?

An ADR is a short document that describes a significant decision affecting the architecture of this project. ADRs are immutable once accepted—if a decision is changed, a new ADR supersedes the old one.

## When to Write an ADR

Write an ADR when you make a decision that:

- **Affects the structure** of the codebase (e.g., choosing a framework, defining module boundaries)
- **Has long-term consequences** that would be costly to reverse
- **Involves trade-offs** between competing concerns (performance vs. maintainability, flexibility vs. simplicity)
- **Might be questioned later** by team members who weren't present for the discussion
- **Deviates from conventions** or common practices in the industry or team

### Examples of ADR-worthy decisions

- Choosing a database technology
- Selecting an authentication approach
- Defining API design patterns
- Establishing testing strategies
- Adopting a new library or framework
- Changing deployment architecture
- Defining data models or schemas

### Not ADR-worthy

- Bug fixes
- Minor refactoring
- Routine implementation details
- Decisions that can be easily reversed

## ADR Overview

| ADR | Title | Date | Status |
|-----|-------|------|--------|
| [0001](adr/0001-use-rust-for-runtime.md) | Use Rust for Runtime | 2025-12-15 | Accepted |

## How to Create an ADR

1. **Copy the template**
   ```bash
   cp docs/development/adr-template.md docs/development/adr/NNNN-short-title.md
   ```

2. **Number your ADR**
   - Use sequential four-digit numbers: `0001`, `0002`, etc.
   - Check existing ADRs in `docs/development/adr/` to find the next number

3. **Choose a descriptive title**
   - Use lowercase with hyphens: `0001-use-postgresql-for-persistence.md`
   - Keep it short but meaningful

4. **Fill out the template**
   - **Status**: Start with "Proposed"
   - **Date**: Use ISO format (YYYY-MM-DD)
   - **Context**: Explain why this decision is needed
   - **Decision**: State clearly what was decided
   - **Consequences**: Be honest about trade-offs
   - **Alternatives**: Document what else was considered

5. **Add to the overview table**
   - Add a new row to the "ADR Overview" table in this document
   - Include the ADR number (as a link), title, date, and status
   - Keep the table sorted by ADR number

6. **Submit for review**
   - Create a pull request with the ADR
   - Discuss with the team
   - Update status to "Accepted" once approved (in both the ADR file and the overview table)

## ADR Statuses

| Status | Meaning |
|--------|---------|
| **Proposed** | Under discussion, not yet accepted |
| **Accepted** | Decision has been agreed upon and is in effect |
| **Deprecated** | No longer relevant but kept for historical context |
| **Superseded** | Replaced by a newer ADR (link to the new one) |

## ADR Lifecycle

```
Proposed → Accepted → [Deprecated | Superseded by new ADR]
```

ADRs are never deleted or modified after acceptance (except to update status). If a decision changes:

1. Create a new ADR explaining the new decision
2. Update the old ADR's status to "Superseded by ADR-NNNN"
3. Update the status in the ADR Overview table to match

This preserves the historical context of why decisions were made.

## Directory Structure

```
docs/
└── development/
    ├── adr.md              # This guide
    ├── adr-template.md     # Template for new ADRs
    └── adr/
        ├── 0001-example-decision.md
        ├── 0002-another-decision.md
        └── ...
```

## Tips for Writing Good ADRs

- **Be concise**: ADRs should be quick to read (1-2 pages max)
- **Be specific**: Avoid vague language; state exactly what was decided
- **Be honest**: Document real trade-offs, not just benefits
- **Write for the future**: Assume readers have no context about current discussions
- **Focus on "why"**: The reasoning is more valuable than the decision itself

## Further Reading

- [Michael Nygard's original ADR article](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions)
- [ADR GitHub organization](https://adr.github.io/)
