# Lona

**LISP Machines Never Died. They Evolved.**

Lona is a capability-secure operating system where everything — from device drivers to applications — is written in one language, fully inspectable, and modifiable at runtime.

## The Vision

The original LISP machines offered something we lost: a unified system where you could inspect any running code, modify it live, and understand the entire stack in one language. But they lacked modern security and couldn't survive malicious code.

Lona brings back that vision with modern foundations:

- **seL4** — Capability-based security, formally verified microkernel design
- **BEAM/Erlang** — Millions of lightweight processes, fault isolation, "let it crash"
- **Clojure** — Live code updates via vars, atomic namespace transactions
- **LISP** — One language top to bottom, homoiconicity, REPL-driven development

## Why Lona?

### The Problem

Modern systems force false choices:

- **Security vs. Dynamism** — Sandboxes and containers provide isolation but kill the ability to inspect and modify running systems
- **Safety vs. Expressiveness** — Type systems catch bugs but add ceremony; dynamic languages are expressive but unsafe
- **Isolation vs. Efficiency** — VMs are secure but heavy; threads are fast but share too much

And everywhere: artificial boundaries between "system" and "application", multiple languages, restart-to-update.

### The Solution

Lona provides **one language for the entire system** with **two levels of isolation**:

| Level | Unit | Cost | Enforced By | Purpose |
|-------|------|------|-------------|---------|
| **Realm** | Protection domain | ~1ms to create | seL4 kernel | Security boundaries |
| **Process** | Execution unit | ~1-10µs to create | Userspace scheduler | Concurrency |

- **Realms** have their own address space, capabilities, and CPU budgets — a compromised realm cannot affect others
- **Processes** are lightweight (512 bytes minimum), communicate via message passing, and have independent garbage collection

## Key Features

### Hierarchical Resource Control

```
Root Realm (100% resources)
├── Drivers (30% CPU, 2GB)
│   ├── Network (shares parent budget)
│   └── Storage (shares parent budget)
└── Applications (70% CPU, 60GB)
    └── WebServer (shares parent budget)
        ├── Worker 1
        ├── Worker 2
        └── Worker 3
```

Resources flow down the hierarchy. Children share their parent's budget — creating 1000 child realms doesn't give you more CPU than you started with.

### Kernel-Enforced Limits

CPU and memory limits are enforced by seL4's capability system, not userspace cooperation:

- **CPU**: MCS scheduling contexts with configurable budgets
- **Memory**: Realms can only allocate from granted Untyped capabilities
- **Capabilities**: Access rights can only be delegated downward, never escalated

### One Language, Fully Inspectable

Everything is Lonala — drivers, protocols, applications, the scheduler. No context switching between C, Python, and shell scripts:

```clojure
;; Inspect a running process
(process-info worker-pid)
; → %{:status :running :heap-size 8192 :mailbox-len 3 ...}

;; Read the source of any function
(source handle-request)

;; Redefine it live — all callers see the new version immediately
(def handle-request (fn [req] (process-v2 req)))

;; Hot-patch a driver without rebooting
(ns-transaction 'drivers.uart
  (fn [tx]
    (tx-def tx 'uart-init improved-uart-init)))
```

Atomic namespace transactions ensure you never see half-updated code.

### Live Code Updates

Clojure-style vars enable code updates that propagate through the realm hierarchy:

```clojure
;; Parent realm defines a function
(def handle-request (fn [req] (process req)))

;; Child realms inherit the binding (read-only, shared memory)
;; When parent updates the var, children see it immediately — no restart
(def handle-request (fn [req] (process-v2 req)))
```

The var table lives in shared memory. Parent writes, children see it on next deref. No IPC, no coordination, no downtime.

### Zero-Copy Data Sharing

Large datasets can be shared between realms without copying:

```clojure
(def corpus (make-shared-region (* 1024 1024 1024) 'dataset))  ; 1 GB
(share-region corpus worker-realm :read-only)
;; Workers read same physical memory, different virtual mappings
```

### The Lonala Language

A LISP dialect with just **5 special forms** (`def`, `fn*`, `match`, `do`, `quote`) — everything else is macros:

```clojure
;; Pattern matching with guards
(defn factorial
  ([0] 1)
  ([n] when (> n 0) (* n (factorial (- n 1)))))

;; Binary protocol parsing
(match packet
  #bits[version:4 ihl:4 ttl:8 protocol:8 & rest]
    (when (= version 4))
      (handle-ipv4 ttl protocol rest))

;; Erlang-style error handling
(match (divide a b)
  [:ok result] result
  [:error :div-by-zero] (panic! "Cannot divide by zero"))
```

Collection literals: `[]` tuples, `{}` vectors, `%{}` maps, `#{}` sets, `#bits[...]` binary patterns.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         HARDWARE                            │
└─────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────│───────────────────────────────┐
│                      seL4 MICROKERNEL                       │
│   Capabilities │ MCS Scheduling │ Memory │ IPC              │
└─────────────────────────────│───────────────────────────────┘
                              │
┌─────────────────────────────│───────────────────────────────┐
│                        ROOT REALM                           │
│   Memory Pool │ Scheduler Config │ Capability Manager       │
│                              │                              │
│   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐    │
│   │   Drivers    │   │     Apps     │   │   Services   │    │
│   │   (Realm)    │   │   (Realm)    │   │   (Realm)    │    │
│   │ ┌──────────┐ │   │ ┌──────────┐ │   │              │    │
│   │ │ Process  │ │   │ │ Process  │ │   │              │    │
│   │ │ Process  │ │   │ │ Process  │ │   │              │    │
│   │ └──────────┘ │   │ └──────────┘ │   │              │    │
│   └──────────────┘   └──────────────┘   └──────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Target Platforms

- x86_64
- aarch64

## Documentation

| Document | Description |
|----------|-------------|
| [concept.md](concept.md) | Full system design and rationale |
| [lonala.md](lonala.md) | Language specification |
| [lonala-process.md](lonala-process.md) | Process and realm APIs |
| [lonala-kernel.md](lonala-kernel.md) | seL4 kernel primitives |
| [lonala-io.md](lonala-io.md) | Device driver primitives |

## Status

This is a **design document**. All code examples are pseudocode illustrating concepts, not working implementations.

## Acknowledgments

Lona draws inspiration from:

- [seL4](https://sel4.systems/) — The formally verified microkernel
- [BEAM](https://www.erlang.org/) — Erlang's virtual machine
- [Clojure](https://clojure.org/) — Rich Hickey's LISP dialect

## License

Copyright 2026 Tobias Sarnowski

This project is licensed under the GNU General Public License v3.0 or later — see [license.md](license.md) for details.
