# Lona Project Guide

Lona is a capability-secure operating system built on the seL4 microkernel, combining BEAM-style lightweight processes with a Clojure-inspired LISP dialect (Lonala).

## MANDATORY: Documentation-First Development

**This project has authoritative specifications. You MUST consult them.**

### The Rule

**BEFORE** discussing, planning, implementing, or reviewing ANY aspect of Lona:

1. **Identify** which documents cover that topic (see complete map below)
2. **Read** those documents - not skim, READ
3. **Base all decisions** on what the specification says
4. **Never assume** - if it's not documented, it doesn't exist

### Enforcement

The finishing-work skill verifies your work matches the specification. Deviations from documented behavior are rejected. If you propose something that contradicts the specification, reviewers will catch it.

### Complete Documentation Map

**Use this table to identify which documents to read for any given task.**

#### Architecture Specification

| Document | What It Contains | Read When |
|----------|------------------|-----------|
| [architecture/index.md](docs/architecture/index.md) | Design philosophy, security model (realms vs processes), zero-trust principles, capability-based security overview, core terminology definitions | Starting any work on Lona; need to understand security boundaries; confused about realms vs processes |
| [architecture/memory-fundamentals.md](docs/architecture/memory-fundamentals.md) | Physical memory management, MMU and address translation, page tables, seL4's memory model (Untyped, frames, page tables), how capabilities control memory | Working on memory allocation, page mapping, understanding how physical memory becomes usable |
| [architecture/system-architecture.md](docs/architecture/system-architecture.md) | Lona Memory Manager design, realm lifecycle (creation, resource budgets, termination), thread management, system bootstrapping sequence, IPC mechanisms, hierarchical resource management | Creating/managing realms, understanding system startup, resource quotas, parent-child realm relationships |
| [architecture/realm-memory-layout.md](docs/architecture/realm-memory-layout.md) | Virtual address space layout within a realm, memory regions (code, heap, stack, vars), inherited read-only regions from parent realms, how vars are shared, code compilation and loading | Understanding where things live in memory, how child realms inherit from parents, var sharing mechanics |
| [architecture/process-model.md](docs/architecture/process-model.md) | Lightweight process design, process state machine, scheduling (reductions, preemption), message passing semantics, mailbox implementation, per-process garbage collection, process linking and monitoring | Implementing process-related features, understanding scheduling, message passing, GC, supervision trees |
| [architecture/device-drivers.md](docs/architecture/device-drivers.md) | Driver isolation in separate realms, MMIO mapping, DMA buffer management, interrupt handling (notifications), zero-copy I/O patterns, driver capability requirements | Writing or understanding device drivers, working with hardware, DMA, interrupts |
| [architecture/services.md](docs/architecture/services.md) | Inter-realm communication via named services, service registry, connection establishment, access control for services, request-response patterns | Implementing cross-realm communication, service discovery, understanding how realms talk to each other |
| [architecture/virtual-machine.md](docs/architecture/virtual-machine.md) | Bytecode VM design, register-based architecture, instruction format and encoding, opcode reference, execution model, stack frames, function calls at VM level | Working on the VM/compiler, understanding bytecode, debugging execution, adding new instructions |

#### Lonala Language Specification

| Document | What It Contains | Read When |
|----------|------------------|-----------|
| [lonala/index.md](docs/lonala/index.md) | Language design philosophy, what Lonala is NOT (not Clojure, not Erlang), type system overview, evaluation model, key differences from inspirations | Starting any Lonala work; need to understand the language's unique design; tempted to assume Clojure behavior |
| [lonala/reader.md](docs/lonala/reader.md) | Lexical syntax, symbol and keyword rules, numeric literal formats, string escaping, collection syntax (`[]` tuple, `{}` vector, `%{}` map), reader macros (`'`, `` ` ``, `~`, `~@`, `^`, `@`) | Parsing Lonala code, understanding syntax, working on the reader, confused about collection literals |
| [lonala/special-forms.md](docs/lonala/special-forms.md) | The 5 special forms: `def` (var binding), `fn*` (function creation), `match` (pattern matching), `do` (sequencing), `quote` (preventing evaluation). Complete semantics for each | Implementing or using special forms, understanding evaluation rules, pattern matching semantics |
| [lonala/data-types.md](docs/lonala/data-types.md) | All value types: nil, booleans, integers (arbitrary precision), floats, strings, symbols, keywords, tuples, vectors, maps, addresses (PIDs, realm IDs), capabilities. Memory representation | Understanding what values exist, type checking, working on value representation in the VM |
| [lonala/metadata.md](docs/lonala/metadata.md) | Var metadata system: `:native` (intrinsic functions), `:macro` (compile-time expansion), `:doc` (documentation), `:private`, `:dynamic` (process-local bindings). How metadata affects behavior | Defining vars with special behavior, understanding intrinsics, macros, dynamic bindings |
| [lonala/lona.core.md](docs/lonala/lona.core.md) | Core namespace intrinsics: namespace management, var operations, arithmetic, comparison, string operations, collection functions (conj, get, assoc, etc.), type predicates, equality | Using or implementing any core function; need to know what functions exist; checking function signatures |
| [lonala/lona.process.md](docs/lonala/lona.process.md) | Process namespace: `spawn`, `spawn-link`, `spawn-monitor`, `send`, `receive`, `self`, `exit`, `link`, `unlink`, `monitor`, `demonitor`, process flags, realm operations | Working with processes, message passing, supervision, process lifecycle, spawning into realms |
| [lonala/lona.kernel.md](docs/lonala/lona.kernel.md) | Low-level seL4 intrinsics: IPC primitives (call, send, recv), capability operations, memory mapping, thread control. For VM and system code only | Working on VM internals, direct seL4 interaction, capability manipulation, memory mapping |
| [lonala/lona.io.md](docs/lonala/lona.io.md) | I/O intrinsics: MMIO read/write, DMA buffer allocation and management, interrupt waiting and handling, port I/O. For driver development | Writing device drivers, hardware interaction, DMA, interrupt handlers |
| [lonala/lona.time.md](docs/lonala/lona.time.md) | Time intrinsics: monotonic time, wall clock time, sleep/delay, timeouts, timer creation | Working with time, implementing timeouts, scheduling delayed actions |

#### Standard Library

| File | What It Contains | Read When |
|------|------------------|-----------|
| [lib/lona/core.lona](lib/lona/core.lona) | Bootstrap file defining core macros and derived functions built on intrinsics: `defn`, `defmacro`, `let`, `if`, `cond`, `and`, `or`, `->`, `->>`, etc. | Understanding how macros expand, what derived forms exist, how core is bootstrapped |
| [lib/lona/init.lona](lib/lona/init.lona) | System init process - first userspace process that runs, initializes the system | Understanding system startup sequence, what happens after boot |

#### Development Guides

| Document | What It Contains | Read When |
|----------|------------------|-----------|
| [development/structure.md](docs/development/structure.md) | Project directory layout, crate organization and dependencies, build artifacts, what each crate does | Navigating the codebase, understanding project organization, finding where code lives |
| [development/rust-coding-guidelines.md](docs/development/rust-coding-guidelines.md) | Rust coding standards for this project: error handling patterns, naming conventions, module organization, testing strategy, documentation requirements, no_std constraints | Writing any Rust code, reviewing Rust code, setting up new modules |
| [development/lonala-coding-guidelines.md](docs/development/lonala-coding-guidelines.md) | Lonala coding style: naming conventions, formatting, documentation comments, idiomatic patterns | Writing Lonala library code, reviewing Lonala code, style questions |
| [development/library-loading.md](docs/development/library-loading.md) | How libraries are packaged (tar format), embedded in binary, loaded at runtime, namespace resolution order | Working on library loading, understanding how code gets into a realm, namespace resolution |

---

## Mandatory Workflow

**Skills are mandatory. Use them - don't skip them.**

| Skill | When | Invocation |
|-------|------|------------|
| **develop-rust** | BEFORE any Rust work (reading, writing, reviewing) | `/develop-rust` |
| **finishing-work** | AFTER completing ANY work - validates and reviews | `/finishing-work` |

```
START → Read relevant docs → /develop-rust (if Rust) → Create PLAN.md
      → Implement COMPLETELY → Use REPL for verification
      → /finishing-work → Fix all issues → DONE
```

### Implementation Rules

**Every implementation MUST be complete. No exceptions.**

You are FORBIDDEN from: placeholders, TODO/FIXME comments, stub functions, partial implementations, deferred error handling, or any code that admits incompleteness.

If you create a plan, save it to `PLAN.md` and implement EVERY item. The finishing-work skill verifies against this plan. **If you cannot implement something completely, STOP and discuss with the user.**

---

## Test-First Development (MANDATORY)

**Tests are not optional. They are part of the implementation.**

### The Rule

1. **Planning**: Every feature/task in `PLAN.md` MUST include a "Tests" subsection defining what tests will verify it
2. **Implementation**: Write tests FIRST, watch them fail, then implement code to make them pass
3. **Bug Fixes**: Write a failing regression test FIRST that demonstrates the bug, then fix it

### Test Coverage Requirements

| Change Type | Required Tests |
|-------------|----------------|
| New function/method | Unit tests for all code paths |
| New feature | Unit tests + integration test demonstrating the feature |
| Bug fix | Regression test that fails before fix, passes after |
| Refactoring | Existing tests must pass; add tests if coverage gaps found |

### Planning with Tests

Every plan item must specify what tests will verify it:

```markdown
## Task: Implement vector `nth` function

### Implementation
- Add `nth` intrinsic to sequence module
- Handle out-of-bounds with nil return

### Tests
- Unit: `nth` returns correct element at valid index
- Unit: `nth` returns nil for negative index
- Unit: `nth` returns nil for index >= length
- Integration: REPL test `(nth {1 2 3} 1)` returns `2`
```

### Bug Fix Workflow (STRICT)

Bug fixes MUST follow this exact sequence:

```
1. Write regression test that reproduces the bug
2. Run test → MUST FAIL (proves test catches the bug)
3. Implement the fix
4. Run test → MUST PASS (proves fix works)
5. Regression test stays in codebase permanently
```

The regression test name should identify the bug: `regression_issue_NNN_description` or `regression_description_of_bug`.

### Enforcement

- The `develop-rust` skill enforces test-first workflow during implementation
- The `finishing-work` skill verifies tests were written for all changes
- The reviewer agent validates test coverage is adequate

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

| Boundary | Enforced By | Purpose |
|----------|-------------|---------|
| Realm | seL4 kernel (VSpace, CSpace, SchedContext) | Security isolation - assume any realm may be compromised |
| Process | Userspace runtime | Reliability - crash isolation, independent GC, no shared mutable state |

### Core Terminology

| Term | Definition |
|------|------------|
| **seL4** | Formally verified microkernel. Foundation of all security guarantees. |
| **Realm** | Protection domain (VSpace + CSpace + SchedContext). THE security boundary. |
| **Process** | Lightweight execution unit within a realm. Own heap, mailbox. NOT a security boundary. |
| **Capability** | Unforgeable token granting specific rights to a kernel object. |
| **VSpace** | Virtual address space. Each realm has its own, hardware-enforced. |
| **CSpace** | Capability space. Each realm has its own, kernel-enforced. |

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
