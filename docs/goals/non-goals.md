# Non-Goals

This document explicitly states what Lona does **not** aim to achieve. Clear non-goals prevent scope creep and clarify the project's focus.

---

## POSIX Compatibility

**Status**: Never

Lona is not a UNIX clone. We do not aim to run existing UNIX applications or provide a POSIX-compatible API.

**Why not?**
- POSIX assumes shared mutable state (files, global namespaces)
- POSIX security model (users, permissions) conflicts with capabilities
- POSIX process model differs fundamentally from BEAM-style processes
- Supporting POSIX would compromise our core abstractions

**What we provide instead**: A native Lonala API designed around Lona's abstractions (Domains, Processes, Capabilities).

---

## Foreign Language Support

**Status**: Never

Lonala is the sole programming language. There is no C FFI, no support for third-party C libraries, and no polyglot runtime.

**Why not?**
- Foreign code cannot be introspected (violates LISP machine philosophy)
- Foreign code bypasses capability checks (violates security model)
- Foreign binaries are opaque (violates source-only distribution)
- Memory-unsafe languages could corrupt the runtime

**What we provide instead**: Everything from device drivers to applications is written in Lonala. Systems programming primitives (MMIO, DMA, interrupts) are native to the language.

---

## Hard Real-Time Guarantees

**Status**: Never (in current form)

Lona uses garbage collection for memory management. While we aim for low-latency GC (per-process collection, incremental algorithms), we do not guarantee deterministic response times.

**Why not?**
- GC pauses are not bounded deterministically
- Scheduling is fair, not deadline-aware
- seL4 can provide real-time, but our runtime adds latency

**What we provide instead**: Soft real-time behavior with low average latencies. For hard real-time requirements, use dedicated real-time systems or bare-metal seL4.

---

## Formal Verification of Userland

**Status**: Not planned

While seL4's kernel is formally verified, Lona's userland runtime and applications are not.

**Why not?**
- Formal verification of a dynamic language runtime is a research problem
- The effort would delay practical usability by years
- Verification of all applications is impractical

**What we provide instead**:
- seL4's verified isolation guarantees (bugs can't cross Domain boundaries)
- BEAM-style fault tolerance (bugs cause crashes, supervisors restart)
- Runtime monitoring and debugging (catch bugs quickly)

---

## GUI / Desktop Environment

**Status**: Not now (maybe later)

The initial Lona release targets server-side, headless use cases. There is no GUI toolkit, window manager, or desktop environment.

**Why not now?**
- GUI adds enormous complexity (input handling, rendering, window management)
- Our primary use case is networked servers and embedded systems
- The core abstractions must stabilize first

**What we might provide later**: A LISP-machine-style graphical environment, if the project succeeds and community interest exists.

---

## Distributed Clustering

**Status**: Not now (planned for future)

Lona processes currently run on a single machine. There is no built-in distributed messaging, remote process spawning, or cluster management.

**Why not now?**
- Single-machine concurrency must work correctly first
- Distribution adds significant complexity (network partitions, consistency)
- The process model is designed to extend to distribution later

**What we might provide later**: Erlang-style distribution where Processes can transparently communicate across machines.

---

## Package Repository / Ecosystem

**Status**: Not now

There is no central package repository, dependency resolution system, or ecosystem of third-party libraries.

**Why not now?**
- The language and runtime are still evolving
- API stability is required before encouraging third-party code
- We need to establish conventions for source-only distribution

**What we might provide later**: A package system for Lonala source bundles with dependency resolution and version management.

---

## Backwards Compatibility Guarantees

**Status**: Not until 1.0

During development, we may make breaking changes to:
- Language syntax and semantics
- Runtime APIs
- System call interfaces
- Storage formats

**Why not?**
- Premature compatibility constraints prevent necessary improvements
- We need freedom to fix design mistakes
- The project is explicitly pre-1.0

**When this changes**: After 1.0 release, we will follow semantic versioning and provide migration paths for breaking changes.

---

## Summary Table

| Non-Goal | Status | Reason |
|----------|--------|--------|
| POSIX compatibility | Never | Conflicts with core model |
| Foreign language support | Never | Violates introspection, security |
| Hard real-time | Never | GC incompatible |
| Verified userland | Not planned | Research problem |
| GUI / Desktop | Not now | Scope, complexity |
| Distributed clustering | Not now | Complexity, stability first |
| Package ecosystem | Not now | API stability required |
| Backwards compatibility | Not now | Pre-1.0 flexibility needed |

---

## What This Means for Users

If you need:
- **Run existing UNIX software** → Use Linux, BSD, or another UNIX
- **Write in C/C++/Rust** → Use a traditional OS with those toolchains
- **Hard real-time** → Use an RTOS or bare-metal seL4
- **GUI applications** → Use a desktop OS (for now)
- **Formal safety proofs** → Use a verified language on seL4

Lona is for users who want:
- Full system introspection and live modification
- BEAM-style fault tolerance
- Capability-based security
- Source-only, transparent systems
- Clojure-style data-centric design

---

## Further Reading

- [Index](index.md) — What Lona IS
- [Core Concepts](core-concepts.md) — The abstractions we DO provide
