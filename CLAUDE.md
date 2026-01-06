# Lona Project Guide

Lona is a capability-secure operating system built on the seL4 microkernel, combining BEAM-style lightweight processes with a Clojure-inspired LISP dialect (Lonala).

## Skills: When to Use Them

**Skills are mandatory workflows. Use them - don't skip them.**

| Skill | When to Use | Invocation |
|-------|-------------|------------|
| **develop-rust** | **BEFORE** reading, writing, reviewing, or thinking about any Rust code. Load FIRST whenever Rust is involved. | `/develop-rust` |
| **finishing-work** | **AFTER** completing ANY work (concepts, plans, features, bugfixes, docs). MANDATORY before claiming work is done. | `/finishing-work` |
| **git-commit** | When creating git commits. | `/git-commit` |

### Skill Workflow Summary

```
┌─────────────────────────────────────────────────────────────────┐
│  START: Any task requiring implementation                       │
│         ↓                                                       │
│  /develop-rust  ← Load principles (if Rust involved)            │
│         ↓                                                       │
│  Create PLAN.md  ← Save the implementation plan                 │
│         ↓                                                       │
│  [Implement COMPLETELY - no TODOs, no stubs, no placeholders]   │
│         ↓                                                       │
│  Use REPL tools for debugging/manual verification               │
│         ↓                                                       │
│  /finishing-work  ← Validates plan, runs agent reviews          │
│         ↓                                                       │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │  MANDATORY REVIEW LOOP (cannot exit with issues)          │  │
│  │  Review → Fix Issues → make verify → Re-review → Repeat   │  │
│  │  Exit ONLY when agents report ZERO issues                 │  │
│  └───────────────────────────────────────────────────────────┘  │
│         ↓                                                       │
│  Delete PLAN.md  ← Plan is fulfilled                            │
│         ↓                                                       │
│  DONE: Only now is the work complete                            │
└─────────────────────────────────────────────────────────────────┘
```

**The `finishing-work` skill is non-negotiable.** It validates the plan was followed, triggers three AI agents to review in parallel, and loops until ALL issues are resolved. Work is NOT complete until this skill passes with zero issues.

---

## CRITICAL: Complete Implementations Only

**THIS IS AN ABSOLUTE RULE. VIOLATIONS ARE UNACCEPTABLE.**

### What You MUST NEVER Do

When implementing any functionality, you are **ABSOLUTELY FORBIDDEN** from:

| Violation | Examples | Why It's Unacceptable |
|-----------|----------|----------------------|
| **Placeholders** | `// placeholder`, `pass`, `unimplemented!()`, `todo!()` | Defers work that should be done now |
| **TODO/FIXME comments** | `// TODO: implement this`, `// FIXME: handle edge case` | Admits incompleteness |
| **Stub functions** | `fn foo() {}`, `fn foo() { Ok(()) }` with no logic | Pretends to implement |
| **Hardcoded values** | Magic numbers, test data in production code | Avoids real implementation |
| **Partial implementations** | Handling 2 of 5 cases, happy path only | Leaves work unfinished |
| **Deferred error handling** | `unwrap()` where errors should be handled | Ignores real requirements |
| **"Will add later" comments** | Any comment suggesting future work | Admits plan not followed |
| **Dummy data/mock data** | Fake responses, simulated behavior | Not real functionality |
| **No-op implementations** | Functions that do nothing but return | Dishonest about capabilities |
| **Workarounds** | Temporary hacks presented as solutions | Technical debt disguised |

### The Implementation Standard

Every piece of code you write MUST be:

1. **Complete** - All planned functionality is implemented, all cases handled
2. **Correct** - Logic is sound, edge cases considered, errors handled properly
3. **Production-ready** - No temporary code, no test fixtures in production paths
4. **Self-contained** - No external work required to make it functional

### Plan Fulfillment Is Mandatory

If you create a plan (via EnterPlanMode or TodoWrite), you MUST:

1. **Implement EVERY item** in the plan completely
2. **Save the plan** to `PLAN.md` before starting implementation
3. **Check each item off** only when truly complete
4. **Never skip items** without explicit user approval
5. **Never partially implement** an item and call it done

The finishing-work skill will verify your implementation against `PLAN.md`. Reviewers will flag any deviation.

### Enforcement

The review agents are explicitly instructed to:
- Check every line for incomplete implementation patterns
- Compare implementation against the saved plan
- Flag ANY placeholder, TODO, stub, or partial implementation
- Reject the review until all issues are resolved

**You will loop through review cycles until zero completeness issues remain.**

### Why This Matters

Incomplete implementations:
- Waste the user's time (they think work is done when it isn't)
- Create hidden technical debt
- Break trust with the user
- Violate the project's "correctness over speed" principle

**If you cannot implement something completely, STOP and discuss with the user. Never pretend completion.**

---

## Development REPL

Two MCP tools are available for interactive Lonala development in QEMU:

| Tool | Purpose |
|------|---------|
| `mcp__lona-dev-repl__eval` | Evaluate Lonala expressions. QEMU starts automatically on first use. |
| `mcp__lona-dev-repl__restart` | Restart QEMU to pick up code changes after rebuilding. |

Both tools support `arch` parameter: `aarch64` (default) or `x86_64`. Each architecture runs an independent QEMU instance with a 60-second idle timeout.

**Workflow:**
1. Use `eval` to test Lonala expressions interactively
2. After modifying Rust code, use `restart` to rebuild and test with updated code
3. Run both architectures in parallel if needed

---

## IMPORTANT: Lonala Is Its Own Language

**Lonala is NOT Clojure. Lonala is NOT Erlang/Elixir. It is its own language.**

While heavily inspired by Clojure (syntax, persistent data structures, vars) and Erlang/Elixir (processes, message passing, supervisors), Lonala has its own design decisions and deviations.

**Rules for working with this codebase:**

1. **NEVER assume** any function, macro, or behavior exists unless documented in the specification
2. **ALWAYS verify** in the Lonala specification before using any function or feature
3. **What is not in the specification does not exist** - do not invent functions based on Clojure/Erlang knowledge
4. **Check the docs** - if you need a function, look it up in [docs/lonala/](docs/lonala/index.md) first

**Key differences from Clojure:**
- No `recur` (automatic TCO instead)
- No `try`/`catch`/`finally` (tuple returns + "let it crash")
- Different collection syntax: `[]` = tuple, `{}` = vector, `%{}` = map
- Only 5 special forms: `def`, `fn*`, `match`, `do`, `quote`

**Key differences from Erlang/Elixir:**
- LISP syntax, not Erlang/Elixir syntax
- Clojure-style vars and namespaces
- Different standard library

**When in doubt, read the specification. Do not guess.**

## IMPORTANT: Quality Assuance Has One Command

There is one canonical command to verify if changes work:

    make verify
  
You MUST use that command to verify if changes actually happen. No other command counts.

## Security Model: Zero Trust Between Realms

**CRITICAL: Realms are the ONLY security boundary in Lona.**

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          SECURITY BOUNDARY                              │
│                                                                         │
│   ┌─────────────────┐      ┌─────────────────┐      ┌───────────────┐   │
│   │    REALM A      │      │    REALM B      │      │   REALM C     │   │
│   │  (potentially   │      │  (potentially   │      │  (potentially │   │
│   │  compromised)   │      │  compromised)   │      │  compromised) │   │
│   │                 │      │                 │      │               │   │
│   │  Own VSpace     │      │  Own VSpace     │      │  Own VSpace   │   │
│   │  Own CSpace     │      │  Own CSpace     │      │  Own CSpace   │   │
│   │  Own Memory     │      │  Own Memory     │      │  Own Memory   │   │
│   └────────┬────────┘      └────────┬────────┘      └───────┬───────┘   │
│            │                        │                       │           │
│            └────────────────────────┼───────────────────────┘           │
│                                     │                                   │
│                          seL4 KERNEL ENFORCES                           │
│                          COMPLETE ISOLATION                             │
└─────────────────────────────────────────────────────────────────────────┘
```

### What This Means

- **Always assume any realm is compromised** - design accordingly
- **Zero trust between realms** - realms cannot access each other's memory or capabilities
- **Kernel-enforced isolation** - VSpace (address space), CSpace (capabilities), and CPU budgets are enforced by seL4, not userspace
- **Communication only via IPC endpoints** - no shared mutable state between realms

### What Is NOT a Security Boundary

**Process isolation within a realm is NOT a security boundary.** It exists for:
- **Reliability** - crashes don't propagate, per-process GC
- **Bug prevention** - no shared mutable state reduces race conditions
- **Fault tolerance** - supervisors can restart failed processes

Since all processes in a realm share the same VSpace (address space), any code running in the realm can theoretically access and modify any memory within that realm through low-level operations. Language constructs like isolated heaps and message passing prevent accidental bugs, not malicious actors.

## Core Terminology

| Term | Definition |
|------|------------|
| **seL4** | Formally verified microkernel providing capabilities, VSpaces, CSpaces, and MCS scheduling. Foundation of all security guarantees. |
| **Realm** | Protection domain = own VSpace + CSpace + SchedContext. THE security boundary. Hardware-enforced isolation. |
| **Process** | Lightweight execution unit within a realm. Own heap, mailbox. Pure userspace construct (no kernel objects). NOT a security boundary. |
| **Capability** | Token granting specific rights to a kernel object. All access control in seL4 is capability-based. |
| **VSpace** | Virtual address space. Each realm has its own, enforced by hardware MMU. |
| **CSpace** | Capability space. Each realm has its own, cannot access others' capabilities. |
| **BEAM** | Erlang's virtual machine. Lona adopts its process model (lightweight processes, per-process GC, message passing) but is NOT BEAM-compatible. |
| **Lonala** | The LISP dialect for Lona. Clojure-inspired syntax with BEAM-style concurrency. |
| **Root Realm** | The singular privileged realm (trusted computing base). Coordinates the system, manages resources. Only realm that is trusted. |

## Memory Model

### Per-Realm (Kernel-Enforced)

| Resource | Description | Enforcement |
|----------|-------------|-------------|
| VSpace | Virtual address space | seL4 kernel, hardware MMU |
| CSpace | Capability space | seL4 kernel |
| Untyped Memory | Physical memory budget | seL4 capability system |
| SchedContext | CPU time budget | seL4 MCS scheduler |
| Endpoint | IPC channel for cross-realm communication | seL4 kernel |

### Per-Process (Userspace, NOT Security)

| Resource | Description | Purpose |
|----------|-------------|---------|
| Heap | Process-local allocation area | Reliability (independent GC) |
| Stack | Call frames, locals | Normal execution |
| Mailbox | FIFO message queue | Communication without shared state |
| Reductions | Scheduling counter | Fairness within realm |

**Remember:** All processes in a realm share the same VSpace. Process isolation is a programming model for reliability, not security.

## Architecture Layers

```
┌─────────────────────────────────────────────────────────────────┐
│  APPLICATION CODE (Lonala)                                      │
│  - Pattern matching, message passing, supervisors               │
│  - Uses lona.process for process/realm management               │
└───────────────────────────────────────┬─────────────────────────┘
                                        │
┌───────────────────────────────────────▼─────────────────────────┐
│  LONA VM RUNTIME                                                │
│  - Scheduler (userspace, per-realm)                             │
│  - Garbage collector (per-process)                              │
│  - Uses lona.kernel for seL4 syscalls                           │
└───────────────────────────────────────┬─────────────────────────┘
                                        │
┌───────────────────────────────────────▼─────────────────────────┐
│  seL4 MICROKERNEL                                               │
│  - Capabilities, VSpace, CSpace                                 │
│  - MCS scheduling (CPU budgets)                                 │
│  - IPC (endpoints, notifications)                               │
│  - THE SECURITY ENFORCEMENT LAYER                               │
└─────────────────────────────────────────────────────────────────┘
```

## Documentation

**CRITICAL: Always read the relevant documentation BEFORE discussing or implementing any topic.**

The authoritative documentation index is **[mkdocs.yaml](mkdocs.yaml)**. Consult it to discover all available documentation pages and their structure.

### Documentation Overview

| Document | Description |
|----------|-------------|
| [docs/index.md](docs/index.md) | Project homepage: vision, key features, architecture overview |
| [docs/supported-hardware.md](docs/supported-hardware.md) | Supported hardware platforms, IOMMU requirements, security implications |

### Architecture Specification

| Document | Description |
|----------|-------------|
| [docs/architecture/index.md](docs/architecture/index.md) | Architecture overview: design philosophy, security model, terminology |
| [docs/architecture/memory-fundamentals.md](docs/architecture/memory-fundamentals.md) | Physical memory, MMU, address translation, seL4's memory model |
| [docs/architecture/system-architecture.md](docs/architecture/system-architecture.md) | Lona Memory Manager, realms, threads, bootstrapping, IPC, resource management |
| [docs/architecture/realm-memory-layout.md](docs/architecture/realm-memory-layout.md) | VSpace layout, inherited regions, vars, code compilation |
| [docs/architecture/process-model.md](docs/architecture/process-model.md) | Processes, scheduling, message passing, garbage collection |
| [docs/architecture/device-drivers.md](docs/architecture/device-drivers.md) | Driver isolation, zero-copy I/O, DMA, interrupt handling |
| [docs/architecture/services.md](docs/architecture/services.md) | Inter-realm communication: service registry, connections, access control |
| [docs/architecture/virtual-machine.md](docs/architecture/virtual-machine.md) | Bytecode VM: register architecture, instruction format, execution model |

### Lonala Language Specification

| Document | Description |
|----------|-------------|
| [docs/lonala/index.md](docs/lonala/index.md) | Language overview: design philosophy, what Lonala is NOT, type system |
| [docs/lonala/reader.md](docs/lonala/reader.md) | Lexical syntax: symbols, keywords, numeric literals, collections, reader macros |
| [docs/lonala/special-forms.md](docs/lonala/special-forms.md) | The 5 special forms: `def`, `fn*`, `match`, `do`, `quote` |
| [docs/lonala/data-types.md](docs/lonala/data-types.md) | All value types: nil, booleans, numbers, strings, collections, addresses, capabilities |
| [docs/lonala/metadata.md](docs/lonala/metadata.md) | Var metadata: native intrinsics, macros, documentation, process-local bindings |
| [docs/lonala/lona.core.md](docs/lonala/lona.core.md) | Core intrinsics: namespaces, vars, arithmetic, collections, predicates |
| [docs/lonala/lona.process.md](docs/lonala/lona.process.md) | Process intrinsics: spawn, message passing, linking, monitoring, realms |
| [docs/lonala/lona.kernel.md](docs/lonala/lona.kernel.md) | seL4 intrinsics: IPC, capabilities, memory mapping (for VM/system code) |
| [docs/lonala/lona.io.md](docs/lonala/lona.io.md) | I/O intrinsics: MMIO, DMA, interrupt handling (for driver development) |
| [docs/lonala/lona.time.md](docs/lonala/lona.time.md) | Time intrinsics: monotonic time, sleep, system time |

### Standard Library

| File | Description |
|------|-------------|
| [lib/lona/core.lona](lib/lona/core.lona) | Core intrinsics and derived macros (bootstrap file) |
| [lib/lona/init.lona](lib/lona/init.lona) | System init process (first user process) |

### Development

| Document | Description |
|----------|-------------|
| [docs/development/structure.md](docs/development/structure.md) | Project structure: crate organization, directory layout, build artifacts |
| [docs/development/rust-coding-guidelines.md](docs/development/rust-coding-guidelines.md) | Rust implementation guide: project structure, coding guidelines, testing strategy |
| [docs/development/lonala-coding-guidelines.md](docs/development/lonala-coding-guidelines.md) | Coding style conventions for Lonala source files |
| [docs/development/library-loading.md](docs/development/library-loading.md) | Library loading: tar archive format, embedding, namespace resolution |

### Before You Work

1. **Identify the relevant documentation** for your task using [mkdocs.yaml](mkdocs.yaml)
2. **Read those documents** before starting implementation
3. **Do not assume** - if something isn't documented, it doesn't exist
4. **Verify your understanding** matches the specification

## Key Design Decisions

### Why Realms for Security, Processes for Concurrency

- **Realm creation**: ~milliseconds (kernel objects, page tables)
- **Process creation**: ~microseconds (pure userspace)
- **Use realms** when you need security isolation (untrusted code, drivers, user applications)
- **Use processes** when you need concurrency (workers, servers, handlers)

### Hierarchical Resources

```
Root Realm (100% resources, trusted)
├── Drivers Realm (30% CPU, 2GB) ← policy enforced by kernel
│   └── children share parent's budget
└── Apps Realm (70% CPU, 60GB) ← policy enforced by kernel
    └── children share parent's budget
```

- Children cannot exceed parent's allocation
- Creating child realms doesn't increase total resources (anti-Sybil)
- Parent can revoke child's capabilities at any time

### Message Passing

- **Intra-realm**: Deep copy to receiver's heap, ~100-500 ns
- **Inter-realm**: seL4 IPC, serialization, ~1-10 µs
- **No shared mutable state** - messages are the only communication

### Vars and Late Binding

Clojure-style vars enable live code updates:
- Parent realm updates var → child realms see new value immediately (shared RO mapping)
- No restart required for code updates
- Atomic namespace updates prevent inconsistent states

## Quick Reference

### Realm Operations
```clojure
(realm-create %{:name 'worker :policy %{:cpu %{:max 0.3} :memory %{:max (* 1 +GB+)}}})
(realm-terminate realm-id)
```

### Process Operations
```clojure
(spawn (fn [] (worker-loop)))           ; New process in current realm
(spawn-in realm-id (fn [] (work)))      ; New process in child realm
(send pid [:message data])              ; Async message (local or remote)
(receive [:ok result] result)           ; Pattern-matched receive
```

### Linking and Monitoring
```clojure
(spawn-link f)                          ; Bidirectional crash notification
(spawn-monitor f)                       ; Unidirectional monitoring
```

## Consulting AI Agents

Three AI agents are available for reviews, second opinions, and parallel consultation.

### Agent Commands

| Agent | Method |
|-------|--------|
| **Claude** | `Task(subagent_type="reviewer", run_in_background=true, prompt="<PROMPT>")` |
| **Gemini** | `Bash(run_in_background=true, timeout=600000, command='gemini -m gemini-3-pro-preview "<PROMPT>"')` |
| **Codex** | `Bash(run_in_background=true, timeout=600000, command='codex exec -m gpt-5.2 -c model_reasoning_effort=medium -c hide_agent_reasoning=true -s read-only "<PROMPT>"')` |

The `reviewer` subagent is defined in `.claude/agents/reviewer.md`. Symlinks `AGENTS.md` and `GEMINI.md` point to this file so all agents use consistent instructions.

For **Codex**: MUST use "-m gpt-5.2" model for conceptual reviews such as designs or plans. MUST use "-m gpt-5.2-codex" model for code reviews. DO NOT use o3, o1, or other models - they are not supported with Codex.

### Running Agents in Parallel

**CRITICAL:** Run all three agents IN PARALLEL using a single message with multiple tool calls. Do NOT run sequentially.

Collect results with `TaskOutput` when complete.

### Handling Agent Timeouts

**NOTE:** Agents, especially Codex, can take longer than the `TaskOutput` timeout (30 seconds default). If `TaskOutput` times out before an agent completes:
1. Call `TaskOutput` again with the same `task_id`
2. Repeat until the agent completes
3. Typically takes 2-3 `TaskOutput` calls for Codex to finish
