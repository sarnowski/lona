# Development Principles

This document defines the governing principles for Lona development. These principles guide day-to-day decisions and ensure consistency across the codebase.

For coding style details, see:
- [Rust Coding Guidelines](rust-coding-guidelines.md)
- [Lonala Coding Guidelines](lonala-coding-guidelines.md)
- [Testing Strategy](testing-strategy.md)
- [Minimal Rust Runtime](minimal-rust.md)

---

## I. Foundational Philosophy

### Correct, Simple, Auditable

Prefer simple, verifiable correctness over clever optimizations. Code should be:

- **Correct first**: Make it work, make it right, make it fast—in that order
- **Simple**: Readable, maintainable code beats clever tricks
- **Auditable**: Every line should be understandable during review

Optimize only when profiling proves necessity. Premature optimization conflicts with our priorities of introspection, debuggability, and correctness.

### Pre-1.0 Freedom

We are in the creative phase. No backwards compatibility until 1.0:

- Refactor fearlessly when it improves the design
- Change APIs, syntax, and semantics as needed
- Delete code that no longer serves the vision

**After 1.0**: Semantic versioning with migration paths for breaking changes.

### Long-Term Thinking

Lona is meant to run for decades. Every decision considers:

- Future maintainability over short-term convenience
- Extensibility of the design
- Impact on the entire system, not just the immediate task

---

## II. Explicit Non-Goals (Design Constraints)

These are not merely deprioritized—they are explicitly rejected. Reference them when design discussions drift toward these areas.

| Constraint | Rationale |
|------------|-----------|
| **No POSIX** | POSIX assumes shared mutable state and identity-based security. Conflicts with capabilities. |
| **No FFI** | Foreign code cannot be inspected and bypasses capability checks. Everything must be Lonala. |
| **No Hard Real-Time** | GC pauses exist. We aim for low latency, not deterministic timing. |
| **No Persistent Images** | Source files + reproducible builds, not saved memory images. |

See [Non-Goals](../goals/non-goals.md) for the complete list.

---

## III. Lonala-First Development

> "By understanding eval you're understanding what will probably be the main model of computation well into the future." — Paul Graham

### The Default Answer is Lonala

If functionality CAN be implemented in Lonala using existing primitives, it MUST be implemented in Lonala. The Rust runtime exists only to provide what Lonala cannot provide for itself.

Before adding any native primitive, consult the [Minimal Rust Runtime](minimal-rust.md) checklist and ask:

1. Can this be derived from existing primitives? → **Implement in Lonala**
2. Does this require hardware access (MMIO, IRQ)? → Native is acceptable
3. Does this require inspecting runtime type tags? → Native is acceptable
4. Is this purely for efficiency? → **Implement in Lonala first**, optimize only if profiling proves necessary

### Primitive Budget

Every addition to the native runtime requires written justification. The [Minimal Rust Runtime](minimal-rust.md) document defines the allowed categories:

- Core data structure operations
- Type predicates
- Arithmetic and comparison
- Symbol operations
- Metadata operations
- Hardware access (MMIO, DMA, IRQ)
- Process and scheduling primitives
- Domain and capability operations
- Atoms (process-local)
- Introspection and hot-patching

Anything not in these categories requires explicit approval.

### Self-Hosting Goal

Design toward a Lonala that can compile itself. The compiler, macro expander, and language tooling should eventually be written in Lonala, with only the minimal bootstrap in Rust.

### Source is Canonical

There is no ahead-of-time compilation. There are no pre-compiled binaries distributed.

- **Source-only distribution**: The only way to get code into Lona is to load source files
- **Bytecode caching allowed**: The runtime may cache compiled bytecode for performance
- **Bytecode-only distribution forbidden**: Source must always be available for inspection

The compiler is part of the runtime, not a separate tool.

---

## IV. Security (seL4-Driven)

> "Capabilities, Not Permissions"

### No Ambient Authority

Functions receive their dependencies as arguments. There is no:

- Global capability access
- Implicit privilege based on identity
- Hidden authority in the environment

Mutable state must be explicit and capability-contained. The dispatch table within a Domain is intentionally mutable (for hot-patching), but access to it is Domain-scoped.

### Sandboxing by Default

New Domains receive no capabilities unless explicitly granted:

```clojure
(spawn untrusted-code/main [data]
       {:domain "sandbox"
        :capabilities []              ; NONE - pure computation only
        :memory-limit (megabytes 32)})
```

### Hierarchical Delegation

Capabilities flow downward in a tree:

- A Domain can only delegate capabilities it possesses
- Revocation cascades to all descendants
- Privilege never escalates

### Domain is the ONLY Security Boundary

| Within a Domain | Between Domains |
|-----------------|-----------------|
| Single trust zone | Complete isolation |
| Shared memory and capabilities | seL4-enforced separation |
| Processes can inspect each other | Inspection requires capability |

If you don't trust code, put it in a separate Domain.

### Minimal Trusted Computing Base

Less Rust = smaller attack surface:

- The Rust runtime is trusted code (part of the TCB)
- Every line of Rust increases audit burden
- Prefer Lonala implementations where possible

---

## V. Resilience (BEAM-Driven)

> "Let It Crash"

### Don't Handle Errors You Can't Fix

Write the happy path. When something unexpected happens:

1. Process crashes immediately
2. Supervisor detects and restarts
3. Fresh state from known-good initial conditions

Avoid defensive coding that tries to recover from unknown states.

### Supervision is Mandatory

Every persistent activity must have:

- An owning supervisor
- A defined restart strategy
- Bounded restart intensity (max restarts per time window)

Orphan processes are bugs.

### Per-Process Heaps and GC

Each process has its own heap, garbage collected independently:

- GC pause in one process doesn't affect others
- Dead process's memory is immediately released
- Small heaps = fast GC = low latency

### No Shared Mutable State Across Processes

Processes communicate via messages and immutable data only.

**Controlled mutability**:
- `Binary` is the only *shareable* mutable type (with explicit ownership transfer via `binary-transfer!`)
- `Atoms` exist but are process-local (not a cross-process coordination primitive)

### Backpressure and Timeouts

Every boundary has:

- Bounded queues (no unbounded growth)
- Timeouts on blocking operations
- Clear overload behavior

---

## VI. Introspection (LISP Machine-Driven)

> "The Inspectable Machine"

### Everything Observable Within Domain

Every value, function, process, and capability can be examined at runtime:

```clojure
(source some-function)      ; View source code
(disassemble some-function) ; View bytecode
(provenance some-function)  ; Where did this come from?
(process-info pid)          ; Process state
```

**Cross-domain inspection requires capability**:

```clojure
(debug-attach other-domain-pid)  ; requires :debug capability
(trace-calls 'other/function)    ; requires :trace capability
```

### Late Binding as Foundation

Function calls are resolved through dispatch tables at runtime, not compiled to direct jumps:

```
Function call: (process-packet pkt)
       ↓
Dispatch table: process-packet → bytecode-ptr
       ↓
Execute bytecode
```

This enables hot-patching without recompiling callers.

### Two-Mode Architecture

| Mode | Trigger | Error Behavior | Use Case |
|------|---------|----------------|----------|
| **Production** | Default | Crash, supervisor restarts | Servers |
| **Debug** | Debugger attached | Pause, user inspects | Troubleshooting |

Production systems self-heal. When a debugger is attached, errors pause for inspection instead of crashing.

### Separate Error Detection from Recovery

The condition/restart system (inspired by Common Lisp) preserves context:

- Error is signaled, stack is NOT unwound
- Handler inspects the situation
- Handler chooses a restart
- Execution continues from the restart point

This differs from exceptions where context is lost on throw.

### Per-Domain Hot-Patching

Patches are domain-local by default:

- Parent hot-patches → Parent sees new version
- Child continues with old version
- **Explicit propagation** required: `push-code`, `pull-code`
- Cross-domain propagation is capability-guarded
- Provenance enables rollback to previous versions

---

## VII. Data Philosophy (Clojure-Driven)

> "Data is Ultimate"

### Data is the Interface

Messages, configuration, state, protocols—all plain data:

```clojure
;; Not opaque objects, but inspectable data
{:type :request
 :method :get
 :path "/users/42"}
```

No interface definitions, no IDL, no code generation. Protocols are implicit in the shape of the data.

### Immutability by Default

All core types are immutable:

| Type | Mutability |
|------|------------|
| Vector, Map, Set, List | Immutable |
| Keyword, Symbol, String | Immutable |
| Numbers | Immutable |
| Binary | **Mutable** (escape hatch) |
| Atom | **Process-local mutable** |

"Updates" return new values; old values unchanged.

### BEAM-Style Message Passing

Lona adopts **pure BEAM semantics** for message passing:

- **Within Domain**: Immutable data is **deep-copied** on send (ensures independent heaps)
- **Binary (large data)**: Shared by reference—the explicit escape hatch
- **Across Domains**: Serialization + seL4 IPC for small data, shared memory for Binary

This ensures per-process heap independence and instant memory reclaim on process death.

```clojure
;; Regular values: deep copied on send
(send other-process {:data large-map})  ; map is copied

;; Binary: shared by reference (explicit escape hatch)
(send other-process {:packet packet-binary})  ; binary shared, not copied
```

### Zero Magic

Explicit is better than implicit:

- No implicit control flow
- No implicit state mutation
- No hidden behavior behind simple syntax
- Error handling is visible and structured

### Homoiconicity

Code is data. This enables:

- Macros that transform code at compile time
- REPL that reads data and evaluates it as code
- Introspection that examines code as data structures

---

## VIII. Testing & Verification

### Test-First Bug Fixes

Every bug fix starts with a failing test:

1. Write test that reproduces the bug
2. Verify test fails against current code
3. Fix the bug
4. Test passes and stays forever as regression test

No exceptions.

### Host-Testable Architecture

Structure code so logic runs on the development machine:

- Only `lona-runtime` depends on seL4
- All other crates (`lona-core`, `lona-kernel`, `lonala-compiler`, etc.) are host-testable
- Use traits and mocks for hardware abstraction

### Layered Testing Pyramid

```
                ┌─────────────────┐
                │   QEMU Tests    │  ← Slowest (seconds)
                │  (Integration)  │     System-level
                └────────┬────────┘
                         │
             ┌───────────┴───────────┐
             │   On-Target Tests     │  ← Medium (QEMU overhead)
             │  (seL4 interaction)   │     Hardware-specific
             └───────────┬───────────┘
                         │
    ┌────────────────────┴────────────────────┐
    │           Host Unit Tests               │  ← Fastest (milliseconds)
    │  (Pure logic, data structures, parser)  │     90%+ of tests
    └─────────────────────────────────────────┘
```

### Property Tests for Core Types

Data structures, numeric types, and serialization get generative/property-based tests:

- Persistent data structure invariants
- Numeric operation properties
- Round-trip serialization
- Fuzzing for parsers

### Reproducible Tests

Tests must not depend on:

- Timing or execution speed
- Global state
- Execution order
- Random seeds (unless explicitly seeded)

Flaky tests are release blockers.

---

## IX. Code Quality

### Strict Lints with Justification

Use `#[expect]` (not `#[allow]`) for lint suppressions, and always provide a reason:

```rust
#[expect(clippy::too_many_arguments, reason = "[approved] seL4 syscall requires 7 args")]
```

Warnings are errors. Fix root causes instead of silencing tools.

### Determinism by Default

Prefer deterministic behavior:

- Use `BTreeMap`/`BTreeSet` over `HashMap`/`HashSet` (no random seeds needed)
- Avoid time-dependent behavior unless explicitly required
- Reproducible builds and test runs

### Small, Reviewable Changes

- Atomic commits with clear rationale
- One logical change per commit
- Tests preserve behavior through refactors

### Single Source of Truth

- Avoid duplicated logic across Rust/Lonala/docs
- When duplication is unavoidable, add tests that assert consistency
- Specs are authoritative; implementations must match

---

## X. Rust Implementation

These principles apply specifically to Rust code in the runtime.

### Panic is Abort

No unwinding. `panic = "abort"` in release builds. Code must be exception-safe by design, not by cleanup.

### Fallible Allocation

No panics on OOM:

```rust
// Good: fallible
let vec = Vec::try_with_capacity(size)?;

// Bad: panics on OOM
let vec = Vec::with_capacity(size);
```

Use `try_reserve`, `try_new`, and similar fallible APIs.

### no_std Only

`core` and `alloc` only. No `std` dependencies. This ensures code can run on bare metal.

### Strict Unsafe Hygiene

Every `unsafe` block must have a preceding `// SAFETY:` comment:

```rust
// SAFETY: `ptr` is valid because:
// 1. It was obtained from Box::into_raw() in new()
// 2. No other code has access to this pointer
// 3. The pointer is properly aligned for T
unsafe {
    Box::from_raw(ptr)
}
```

One unsafe operation per block. Minimize scope.

### Structured Errors Only

User-facing errors (compiler, VM) use typed structures:

```rust
pub enum CompileError {
    UnboundSymbol { name: String, span: Span },
    ArityMismatch { expected: usize, got: usize, span: Span },
}
```

**Do not implement `Display`** on error types. Formatting belongs in `lonala-human` crate, separated from error definition.

### Runtime Layering

Only `lona-runtime` may depend on seL4 crates. All other crates must be host-testable:

```
lona-runtime (seL4-specific)
    ↓ depends on
lona-kernel (host-testable)
    ↓ depends on
lona-core (host-testable)
```

---

## XI. Documentation

### Spec-Driven Behavior

Language, VM, and runtime behavior is defined in written specifications:

- [Lonala Language Specification](../lonala/index.md)
- Implementation must match spec
- Tests enforce compliance

When spec and implementation disagree, the spec is authoritative (fix the implementation).

### Document Why, Not What

Code explains what it does. Documentation explains:

- **Why** this design was chosen
- **How** it fits into the larger system
- **What guarantees** it provides
- **What would change** the decision

### Explicit Tradeoffs

When making a design choice, record:

- The alternatives considered
- Why this option was chosen
- What would cause us to reconsider

---

## Summary

| Category | Core Principle |
|----------|----------------|
| Philosophy | Correct, simple, auditable code. Pre-1.0 freedom to change. |
| Constraints | No POSIX, No FFI, No hard real-time, No persistent images. |
| Lonala-First | Default answer is Lonala. Primitive budget for native code. |
| Security | No ambient authority. Sandboxing by default. Domain is only boundary. |
| Resilience | Let it crash. Supervision mandatory. No shared mutable state. |
| Introspection | Everything observable (within domain). Late binding. Two-mode architecture. |
| Data | Data is the interface. Immutable by default. Deep copy on send. Binary escape hatch. |
| Testing | Test-first bug fixes. Host-testable architecture. Reproducible tests. |
| Code Quality | Strict lints. Determinism. Single source of truth. |
| Rust | Panic=abort. Fallible allocation. Strict unsafe hygiene. |
| Documentation | Spec-driven. Document why. Explicit tradeoffs. |

---

## Further Reading

- [Goals Overview](../goals/index.md) — The four-pillar vision
- [Non-Goals](../goals/non-goals.md) — What we explicitly don't build
- [Core Concepts](../goals/core-concepts.md) — Unified abstractions
- [System Design](../goals/system-design.md) — Implementation mechanics
- [Minimal Rust Runtime](minimal-rust.md) — Native primitive checklist
- [Rust Coding Guidelines](rust-coding-guidelines.md) — Detailed Rust patterns
- [Lonala Coding Guidelines](lonala-coding-guidelines.md) — Detailed Lonala patterns
- [Testing Strategy](testing-strategy.md) — Testing pyramid and practices
