# Lona Project Guide

Lona is a capability-secure operating system built on the seL4 microkernel, combining BEAM-style lightweight processes with a Clojure-inspired LISP dialect (Lonala).

## Mandatory Workflow

**Skills are mandatory. Use them - don't skip them.**

| Skill | When | Invocation |
|-------|------|------------|
| **develop-rust** | BEFORE any Rust work (reading, writing, reviewing) | `/develop-rust` |
| **finishing-work** | AFTER completing ANY work - validates and reviews | `/finishing-work` |

```
START → /develop-rust (if Rust) → Create PLAN.md → Implement COMPLETELY
      → Use REPL for verification → /finishing-work → Fix all issues → DONE
```

### Implementation Rules

**Every implementation MUST be complete. No exceptions.**

You are FORBIDDEN from: placeholders, TODO/FIXME comments, stub functions, partial implementations, deferred error handling, or any code that admits incompleteness.

If you create a plan, save it to `PLAN.md` and implement EVERY item. The finishing-work skill verifies against this plan. **If you cannot implement something completely, STOP and discuss with the user.**

---

## Tools

### Verification

```
make verify
```

This is the ONE command to verify changes work. No other command counts.

### Development REPL

| Tool | Purpose |
|------|---------|
| `mcp__lona-dev-repl__eval` | Evaluate Lonala expressions (QEMU starts automatically) |
| `mcp__lona-dev-repl__restart` | Restart QEMU after code changes |

Both support `arch` parameter: `aarch64` (default) or `x86_64`.

### AI Agents

| Agent | Command |
|-------|---------|
| **Claude** | `Task(subagent_type="reviewer", run_in_background=true, prompt="...")` |
| **Gemini** | `Bash(run_in_background=true, timeout=600000, command='gemini -m gemini-3-pro-preview "..."')` |
| **Codex** | `Bash(run_in_background=true, timeout=600000, command='codex exec -m gpt-5.2 -c model_reasoning_effort=medium -c hide_agent_reasoning=true -s read-only "..."')` |

**Run all three IN PARALLEL** (single message, multiple tool calls). For Codex code reviews, use `-m gpt-5.2-codex`. Don't use any other models than shown here.

---

## Lonala Language

**Lonala is NOT Clojure. Lonala is NOT Erlang/Elixir.**

- NEVER assume any function exists unless documented in [docs/lonala/](docs/lonala/index.md)
- What is not in the specification does not exist

**Key syntax differences from Clojure:**
- `[]` = tuple, `{}` = vector, `%{}` = map
- No `recur` (automatic TCO), no `try`/`catch` (tuple returns + "let it crash")
- Only 5 special forms: `def`, `fn*`, `match`, `do`, `quote`

---

## Architecture

### Security Model

**Realms are the ONLY security boundary.** Process isolation exists for reliability and bug prevention, NOT security.

```
┌─────────────────────────────────────────────────────────────────┐
│  APPLICATION CODE (Lonala)                                      │
│  - Pattern matching, message passing, supervisors               │
└───────────────────────────────────────┬─────────────────────────┘
                                        │
┌───────────────────────────────────────▼─────────────────────────┐
│  LONA VM RUNTIME                                                │
│  - Scheduler (userspace, per-realm)                             │
│  - Garbage collector (per-process)                              │
└───────────────────────────────────────┬─────────────────────────┘
                                        │
┌───────────────────────────────────────▼─────────────────────────┐
│  seL4 MICROKERNEL                                               │
│  - Capabilities, VSpace, CSpace, MCS scheduling                 │
│  - THE SECURITY ENFORCEMENT LAYER                               │
└─────────────────────────────────────────────────────────────────┘
```

### Core Terminology

| Term | Definition |
|------|------------|
| **seL4** | Formally verified microkernel. Foundation of all security guarantees. |
| **Realm** | Protection domain (VSpace + CSpace + SchedContext). THE security boundary. |
| **Process** | Lightweight execution unit within a realm. Own heap, mailbox. NOT a security boundary. |
| **Capability** | Token granting specific rights to a kernel object. |
| **VSpace** | Virtual address space. Each realm has its own. |
| **CSpace** | Capability space. Each realm has its own. |
| **Lonala** | The LISP dialect for Lona. |

---

## Documentation

**Authoritative index: [mkdocs.yaml](mkdocs.yaml)**

Read relevant documentation BEFORE implementing. If something isn't documented, it doesn't exist.

### Essential Documents

| Document | Description |
|----------|-------------|
| [docs/architecture/index.md](docs/architecture/index.md) | Architecture overview, security model |
| [docs/architecture/process-model.md](docs/architecture/process-model.md) | Processes, scheduling, message passing, GC |
| [docs/lonala/index.md](docs/lonala/index.md) | Language overview, type system |
| [docs/lonala/lona.core.md](docs/lonala/lona.core.md) | Core intrinsics |
| [docs/lonala/lona.process.md](docs/lonala/lona.process.md) | Process intrinsics |
| [docs/development/rust-coding-guidelines.md](docs/development/rust-coding-guidelines.md) | Rust implementation guide |
