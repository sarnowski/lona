# Architecture

This is the architecture reference for Lona, a capability-secure operating system built on the seL4 microkernel. It covers the design philosophy, security model, memory architecture, and system structure.

---

## Terminology

Before diving into the architecture, here are the core concepts:

| Term | Definition |
|------|------------|
| **Lona** | The operating system. Combines seL4's security with BEAM-style concurrency. |
| **Lonala** | Lona's LISP dialect. Clojure-inspired syntax with BEAM-style concurrency semantics. |
| **Realm** | The primary security isolation boundary. Each realm is a completely isolated execution environment that can safely run untrusted code without compromising the rest of the system. Realms form a tree hierarchy, and each realm can host millions of lightweight processes. Typical examples: a driver runs in its own realm, an application runs in its own realm, a plugin runs in its own realm. |
| **Process** | A lightweight execution unit within a realm, similar to Erlang/BEAM processes. Processes have their own heap and mailbox. Millions can exist per realm. NOT a security boundary - processes share trust with their realm. |
| **Lona Memory Manager** | The minimal privileged component that manages system resources (memory, CPU budgets, realm lifecycle). Contains no Lonala code - just resource management and fault handling. |
| **Lona VM** | The virtual machine that runs Lonala code, mapped into every realm. Interprets bytecode, schedules processes, handles garbage collection. |
| **seL4** | The microkernel underlying Lona. Provides the low-level mechanisms (capabilities, address spaces, scheduling) that make realm isolation possible. |

**Key insight**: Realms provide security isolation (hardware-enforced, cannot be bypassed). Processes provide concurrency and fault tolerance within a realm. Running untrusted code requires a dedicated realm, not just a separate process.

---

## Design Philosophy

### Core Design Principles

#### 1. Untrusted by Default

All realms are treated as potentially compromised. Resource limits are kernel-enforced, requiring no cooperation from untrusted code. The system must:

- Prevent resource exhaustion attacks (CPU, memory)
- Prevent capability leakage
- Isolate faults to single realms
- Enforce access control on shared resources

#### 2. Cheap Processes, Expensive Realms

Process creation is microseconds (pure userspace). Realm creation is milliseconds (kernel objects, page tables). Use realms for security boundaries, processes for concurrency.

| Operation | Cost | Use Case |
|-----------|------|----------|
| Process spawn | ~1-10 µs | Workers, handlers, concurrent tasks |
| Realm creation | ~1-10 ms | Security isolation, untrusted code, drivers |

#### 3. BEAM-Style Message Passing

Messages between processes are always deep-copied (BEAM semantics). This enables independent per-process garbage collection and prevents shared-state bugs. Large binaries (≥64 bytes) use reference counting via a realm-wide binary pool to avoid copying bulk data. Between realms, messages are serialized for IPC.

#### 4. Late Binding for Live Updates

Clojure-style vars enable code updates that propagate automatically to child realms without restart. Parent updates a var binding, children see the new value immediately through shared memory mappings.

#### 5. Policy Compiled to Kernel Mechanisms

Resource policies are "compiled" into seL4 scheduling contexts and capabilities. No userspace code in the enforcement hot path - the kernel enforces CPU budgets and memory limits directly.

### What We Take From Each System

| Source | What We Adopt |
|--------|---------------|
| **seL4** | Capabilities, address space isolation, MCS scheduling, security-focused microkernel design |
| **BEAM** | Lightweight processes, per-process heaps, reduction-based scheduling, mailboxes, immutable/persistent data structures |
| **Clojure** | Vars, namespaces, atomic updates, rich literal syntax (tuples `[]`, vectors `{}`, sets `#{}`, maps `%{}`), using data literals for function parameters |
| **LISP** | Homoiconicity, REPL-driven development, runtime code loading |

### Design Rationale

**Why seL4?**

- **Capability security**: No ambient authority - all access must be explicitly granted
- **Minimal TCB**: Small kernel, most code in userspace, reduced attack surface
- **Security-focused design**: Developed with formal verification methodology, resulting in high code quality
- **Performance**: Fast IPC and context switching

**Why BEAM-style Processes?**

- **Isolation**: Per-process heaps eliminate shared-state bugs
- **Scalability**: Millions of lightweight processes
- **Fault tolerance**: Process crashes don't affect others
- **Soft real-time**: Per-process GC, no stop-the-world pauses

**Why Clojure-style Vars?**

- **Live updates**: Var indirection enables code changes without restart
- **Consistency**: Atomic namespace updates prevent partial states
- **Hierarchy**: Natural code sharing from parent to child realms
- **Rich literals**: Expressive data notation for configuration and messages

**Why Hierarchical Realms?**

- **Least authority**: Each realm has minimal capabilities
- **Defense in depth**: Multiple isolation boundaries
- **Resource accounting**: Hierarchical budgets (children share parent's allocation)
- **Composability**: Build systems from isolated components

### Note on seL4 Formal Verification

seL4 is formally verified only in specific configurations (e.g., single-processor, specific platforms). Multi-processor configurations with MCS scheduling, which Lona targets, are NOT formally verified.

We choose seL4 for its strong security foundations and the code quality that formal verification methodology brings, but formal verification does not apply to our configuration and is not a goal for this system.

### Hardware Requirements

Lona requires specific hardware capabilities for operation and security. See [Supported Hardware](../supported-hardware.md) for the full platform support matrix.

| Requirement | x86_64 | aarch64 | Mandatory | Purpose |
|-------------|--------|---------|-----------|---------|
| **64-bit CPU** | Yes | Yes | Yes | Address space layout |
| **MMU** | Yes | Yes | Yes | VSpace isolation |
| **IOMMU** | Intel VT-d | ARM SMMU | No* | DMA isolation |
| **Timer** | APIC | GIC | Yes | MCS scheduling |

#### IOMMU and Driver Security

**IOMMU is required for full security isolation of device drivers.**

Without IOMMU, a device can DMA to arbitrary physical memory, bypassing all VSpace/CSpace isolation. This means:

- **With IOMMU**: Driver realms are fully isolated. A compromised driver cannot access memory outside its allocated DMA regions. Drivers can be untrusted.
- **Without IOMMU**: Driver realms must be trusted. They become part of the Trusted Computing Base (TCB). A compromised driver can read or write any physical memory.

At boot, Lona detects IOMMU availability:

- **If present**: IOMMU is configured to restrict each device to its allocated DMA regions. Log: `IOMMU enabled, DMA isolation active`
- **If absent**: Warning logged: `WARNING: No IOMMU detected. Driver realms are TRUSTED. DMA isolation disabled.`

See [DMA Isolation](device-drivers.md#dma-isolation-iommu) for technical details.

---

## Security Model

### Threat Model

We assume any realm may be compromised. A compromised realm may:

- Attempt to exhaust CPU or memory
- Try to access memory it shouldn't have
- Send malformed or excessive IPC messages
- Attempt to forge capabilities or identities
- Crash repeatedly to disrupt the system

The system must remain stable and protect other realms even when one realm is actively malicious.

### Defense Mechanisms

| Threat | Defense | Enforcement |
|--------|---------|-------------|
| **CPU exhaustion** | MCS scheduling with per-realm budgets | seL4 kernel |
| **Memory exhaustion** | Memory quotas per realm | Lona Memory Manager* |
| **Unauthorized memory access** | Separate address spaces per realm | seL4 kernel + MMU |
| **Capability theft/forgery** | Capability system, per-realm endpoints | seL4 kernel |
| **IPC flooding** | Fault rate limiting | Lona Memory Manager |
| **Crash loops** | Supervisor policies, restart limits | Lona VM |
| **Code injection** | W^X memory mappings (no RWX) | seL4 kernel |
| **DMA attacks** | IOMMU restricts device memory access | Hardware IOMMU** |

*Memory quotas are enforced by the Memory Manager through capability partitioning, not directly by the kernel.

**IOMMU required. Without IOMMU, driver realms must be trusted. See [Hardware Requirements](#hardware-requirements).

### Trust and Authority Direction

Realms form a tree hierarchy. Authority flows downward from parent to child:

```
Lona Memory Manager (trusted, manages all realms)
    │
    └── Init Realm (first realm, high trust)
            │
            ├── App Realm A (trusted)
            │       │
            │       └── Plugin Realm (untrusted) ← OK: untrusted child
            │
            └── App Realm B (trusted)
```

**Parent authority over children:**

- Parents can update inherited code (children see changes via live sharing)
- Parents can revoke child capabilities
- Parents control child resource budgets

**Trust implications:**

- **Untrusted child under trusted parent**: SAFE. Parent controls child; child cannot harm parent.
- **Trusted child under untrusted parent**: UNSAFE. Untrusted parent can compromise trusted child through inheritance.

**Rule**: Encapsulate untrusted code in dedicated child realms. Never run trusted code as a child of untrusted code.

### Process Isolation vs Realm Isolation

| Aspect | Process Isolation | Realm Isolation |
|--------|-------------------|-----------------|
| **Enforced by** | Lona VM (userspace) | seL4 kernel (hardware) |
| **Address space** | Shared within realm | Separate per realm |
| **Purpose** | Concurrency, fault tolerance | Security |
| **Trust level** | Same trust as realm | Independent trust |
| **Bypass possible?** | Yes (bugs, malicious code) | No (kernel + MMU enforced) |

Processes within a realm share an address space. A bug or exploit in one process could theoretically access another process's memory. Realm isolation is hardware-enforced and cannot be bypassed by userspace code.

---

## Architecture Summary

```
┌─────────────────────────────────────────────────────────────────────┐
│                           seL4 Kernel                               │
│                    (capabilities, scheduling, IPC)                  │
└─────────────────────────────────┬───────────────────────────────────┘
                                  │
                    ┌─────────────┴─────────────┐
                    ▼                           │
       ┌──────────────────────┐                 │
       │ Lona Memory Manager  │                 │
       │                      │                 │
       │   Resource mgmt      │                 │ schedules
       │   Fault handling     │                 │
       │   Realm lifecycle    │                 │
       └──────────┬───────────┘                 │
                  │                             │
                  │ creates realms              │
                  ▼                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                            REALMS                                   │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐      │
│  │   Init Realm    │  │   App Realm     │  │  Driver Realm   │      │
│  │                 │  │                 │  │                 │      │
│  │  Lona VM        │  │  Lona VM        │  │  Lona VM        │      │
│  │       │         │  │       │         │  │       │         │      │
│  │       ▼         │  │       ▼         │  │       ▼         │      │
│  │  ┌─┬─┬─┐        │  │  ┌─┬─┬─┐        │  │  ┌─┬─┬─┐        │      │
│  │  │P│P│P│        │  │  │P│P│P│        │  │  │P│P│P│        │      │
│  │  └─┴─┴─┘        │  │  └─┴─┴─┘        │  │  └─┴─┴─┘        │      │
│  │  Processes      │  │  Processes      │  │  Processes      │      │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Document Overview

| Document | Description |
|----------|-------------|
| [Memory Fundamentals](memory-fundamentals.md) | Physical memory, MMU, address translation, and seL4's memory model |
| [System Architecture](system-architecture.md) | Lona Memory Manager, realms, threads, bootstrapping, IPC, and resource management |
| [Realm Memory Layout](realm-memory-layout.md) | Address space structure, inherited regions, vars, and code compilation |
| [Process Model](process-model.md) | Processes, scheduling, message passing, and garbage collection |
| [Device Drivers](device-drivers.md) | Driver isolation, zero-copy I/O, DMA, and interrupt handling |

---

## Key Concepts

Detailed definitions for reference:

| Term | Definition |
|------|------------|
| **Lona Memory Manager** | Minimal privileged component managing resources. No Lonala code. |
| **Lona VM** | Virtual machine running Lonala code, mapped into all realms |
| **Realm** | Security boundary with its own address space, capability space, and CPU budget |
| **Worker** | Kernel thread (TCB) running the Lona VM within a realm |
| **Process** | Lightweight Lonala execution unit, multiplexed by the VM |
| **Inherited Region** | Parent realm's code/data mapped read-only into child at fixed address |
| **Realm Endpoint** | Per-realm IPC endpoint for communication with Lona Memory Manager (unforgeable identity) |

seL4-specific terms:

| Term | Definition |
|------|------------|
| **VSpace** | seL4's virtual address space object - each realm has its own |
| **CSpace** | seL4's capability space - stores capabilities a realm holds |
| **Untyped** | Raw physical memory that can be retyped into kernel objects |
| **MCS** | Mixed Criticality Scheduling - seL4's scheduling model with CPU budgets |
| **TCB** | Thread Control Block - seL4's kernel object representing a thread |

---

## Scope and Precision

This documentation captures architectural decisions and design discussions. Some aspects are precisely defined, others are deliberately left open for future refinement:

**Precisely Defined:**

- Separation between Lona Memory Manager and Lona VM binaries
- Two-level memory management (realms via seL4, processes via VM)
- Inherited code regions with fixed virtual addresses
- Process memory: single contiguous block per process (stack + heap growing toward each other, BEAM-style)
- Per-worker allocator instances for lock-free allocation
- Heap growth via reallocation (Fibonacci then 20% increments)
- Per-process garbage collection (no global pauses)
- Per-realm endpoints for IPC identity (not badges)
- Fixed virtual addresses for shared code (same address in all realms)
- IPC buffer location within worker stacks region
- Live sharing semantics for inherited regions (not snapshots)
- Var shadowing: local definitions override inherited
- Per-realm fault rate limiting for DoS protection
- Region table with strict permission enforcement
- Lazy frame mapping for inherited regions

**Deliberately Open:**

- Exact virtual address assignments (specific numbers are illustrative)
- Specific segment sizes and limits
- Multi-worker (multi-TCB) scheduling within realms
- Detailed IPC protocol message formats
- Fault rate limit thresholds (configurable)

**Known Limitations (Future Work):**

- Code region GC: append-only model causes unbounded growth, GC deferred to later
- Single-threaded fault handler: no thread pool, rate limiting only
