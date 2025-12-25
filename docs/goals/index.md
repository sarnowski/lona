# Lona: The Four-Pillar Operating System

## Vision

Lona is a general-purpose operating system that fuses four powerful paradigms into a coherent whole:

1. **seL4 Microkernel** — Formally verified, capability-based security with hardware-enforced isolation
2. **BEAM/OTP Runtime** — Massive concurrency through lightweight processes, fault tolerance through supervision
3. **LISP Machine Philosophy** — Complete runtime introspection, hot-patching, and live system modification
4. **Clojure Data Philosophy** — Immutable persistent data structures, data-centric design, modern LISP ergonomics

The result is an operating system where:

- **Security** is enforced by hardware and capabilities, not conventions
- **Failures** are contained, logged, and automatically recovered
- **Everything** is inspectable and modifiable at runtime
- **Data** is the universal interface between all components

---

## The Four Pillars

Lona's design emerges from the intersection of four foundational technologies. Each pillar contributes essential properties that the others cannot provide alone.

### Pillar I: seL4 — The Fortress

> *"Capabilities, Not Permissions"*

seL4 provides the foundation upon which everything else is built. Its formally verified microkernel guarantees that security properties hold absolutely—not by convention or careful programming, but by mathematical proof.

**What seL4 Contributes:**
- Hardware-enforced memory isolation between Domains
- Capability-based access control (unforgeable tokens for all resources)
- Minimal trusted computing base (~10,000 lines of verified C)
- The only security boundary in the system

**The Core Invariant:** A Domain can only access resources for which it holds an explicit capability. No exceptions, no bypasses—the kernel enforces this.

→ [Deep Dive: seL4 Foundation](pillar-sel4.md)

### Pillar II: BEAM/OTP — The Engine

> *"Let It Crash"*

The BEAM virtual machine and OTP framework, proven over decades in telecommunications, provide the concurrency and fault-tolerance model. Lona adopts this philosophy completely.

**What BEAM/OTP Contributes:**
- Lightweight processes (millions concurrent, hundreds of bytes each)
- Message passing as the only inter-process communication
- Supervision trees for automatic failure recovery
- Process isolation with independent garbage collection

**The Core Invariant:** A process crash is a normal event. Supervisors detect failures and restart processes according to defined strategies. The system self-heals.

→ [Deep Dive: BEAM/OTP Runtime](pillar-beam.md)

### Pillar III: LISP Machine — The Living System

> *"The Inspectable Machine"*

LISP machines of the 1980s treated the running system as a living, malleable environment. Lona revives this philosophy for modern systems programming.

**What LISP Machine Philosophy Contributes:**
- Source code as the canonical distribution format
- Hot-patching: modify running code without restarts
- Full runtime introspection (every value, every function, every process)
- REPL as the primary system interface
- Condition/restart system for interactive error recovery

**The Core Invariant:** There is no "compiled binary you can't inspect." Source is always available. The running system can always be examined and modified.

→ [Deep Dive: LISP Machine Philosophy](pillar-lisp-machine.md)

### Pillar IV: Clojure — The Data

> *"Data is Ultimate"*

Clojure's contribution goes far beyond syntax. Its philosophy of immutable, persistent data structures and data-centric design enables safe sharing and clear interfaces.

**What Clojure Philosophy Contributes:**
- Immutable persistent data structures (vectors, maps, sets)
- Data as the universal interface (not opaque objects)
- Homoiconicity: code is data, enabling powerful metaprogramming
- Rich literal syntax for complex data
- Sequence abstraction over all collections

**The Core Invariant:** Data doesn't change. When you share a map across a domain boundary, the receiver knows it cannot be mutated underneath them. This enables zero-copy sharing.

→ [Deep Dive: Clojure Data Philosophy](pillar-clojure.md)

---

## What Lona Promises

These four pillars combine to deliver specific guarantees:

| Promise | How It's Achieved |
|---------|-------------------|
| **Security by default** | seL4 capabilities + Domain isolation |
| **Resilience without effort** | OTP supervision + "let it crash" |
| **Total transparency** | Source-only distribution + runtime introspection |
| **Safe concurrency** | Immutable data + message passing |
| **Live modification** | Hot-patching + late binding |
| **Zero-copy performance** | Immutability + capability-controlled shared memory |

---

## Core Abstractions

Lona introduces unified abstractions that emerge from combining the four pillars:

| Abstraction | Pillar Lineage | Purpose |
|-------------|----------------|---------|
| **Domain** | seL4 + BEAM | Security and memory isolation boundary |
| **Process** | BEAM + Clojure | Lightweight execution unit with immutable messaging |
| **Capability** | seL4 + Clojure | Unforgeable resource access token (represented as data) |
| **Message** | BEAM + Clojure | Inter-process communication via immutable data |
| **Dispatch Table** | LISP + seL4 | Per-domain symbol→code mapping enabling hot-patching |
| **Supervisor** | BEAM + seL4 | Fault recovery across domain boundaries |

→ [Full Details: Core Concepts](core-concepts.md)

---

## The Lonala Language

**Lonala** is the programming language for Lona—a dialect of Clojure designed for systems programming. It is the sole language for the entire userland: device drivers, network stacks, applications, and system utilities are all written in Lonala.

Lonala synthesizes all four pillars:
- **Clojure syntax and semantics** for expressiveness
- **BEAM-style processes and messaging** for concurrency
- **LISP machine introspection** for debuggability
- **seL4 capability primitives** for security

There is no C FFI. No third-party binary libraries. Everything is source, everything is inspectable, everything follows the same rules.

→ [Language Specification](../lonala/index.md)

---

## Target Platforms

Lona runs on:

| Platform | Architecture | Use Case |
|----------|--------------|----------|
| **QEMU** | aarch64, x86_64 | Development and testing |
| **Raspberry Pi 4** | aarch64 | Embedded, education, hobbyist |
| **AWS Graviton** | aarch64 | Cloud server deployment |
| **x86_64 servers** | x86_64 | Traditional infrastructure |

---

## Initial System Components

The first release targets a minimal but functional networked system:

**Phase 1: Core**
- Lonala runtime (scheduler, GC, memory manager)
- UART driver (serial console)
- REPL (interactive environment)

**Phase 2: Networking**
- VirtIO block and network drivers
- TCP/IP stack
- Telnet server (remote REPL)

**Phase 3: Dynamic Loading**
- Module loader from storage
- Network code loading
- Package system

→ [Implementation Roadmap](../roadmap/index.md)

---

## Reading Guide

### "I'm new here"
1. Read this page completely
2. Explore the pillar that interests you most
3. Read [Core Concepts](core-concepts.md) for the unified picture

### "I want to implement something"
1. [Core Concepts](core-concepts.md) — understand the abstractions
2. [System Design](system-design.md) — understand the mechanics
3. [Language Specification](../lonala/index.md) — understand the syntax

### "I want the quick version"
This page is the quick version. The four pillars, the core promises, and the key abstractions are all here.

---

## Further Reading

- [Core Concepts](core-concepts.md) — Unified abstractions with full details
- [System Design](system-design.md) — Implementation mechanics
- [Non-Goals](non-goals.md) — What we explicitly don't build
- [Pillar: seL4](pillar-sel4.md) — Security foundation deep dive
- [Pillar: BEAM/OTP](pillar-beam.md) — Resilience runtime deep dive
- [Pillar: LISP Machine](pillar-lisp-machine.md) — Introspection philosophy deep dive
- [Pillar: Clojure](pillar-clojure.md) — Data philosophy deep dive

---

## References

- [seL4 Foundation](https://sel4.systems/)
- [Erlang/OTP](https://www.erlang.org/)
- [Clojure](https://clojure.org/)
- [LISP Machines](https://en.wikipedia.org/wiki/Lisp_machine) — Symbolics, MIT CADR
