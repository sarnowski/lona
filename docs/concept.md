# Lona: A LISP-Powered Operating System on seL4

> **⚠️ IMPORTANT NOTICE ⚠️**
>
> **All code examples in this document are PSEUDOCODE for illustrative purposes only.**
>
> The code snippets are intended to demonstrate concepts, mechanisms, and possible logic flows.
> They do NOT represent:
> - Actual implementation code
> - A precise API specification
> - Code that currently exists
> - Code that will necessarily exist in this exact form
>
> The pseudocode uses Clojure-like syntax for readability, but the actual implementation
> language and APIs may differ significantly. Treat all code as conceptual illustrations
> of the underlying ideas, not as a programming reference.

---

Lona is a capability-secure operating system built on the seL4 microkernel, combining:

- **seL4's** capability-based security and minimal trusted computing base
- **BEAM/Erlang's** lightweight process model, per-process garbage collection, message-passing concurrency, and immutable data structures
- **Clojure's** rich literal syntax (vectors, sets, maps), namespaces, and var-based late binding
- **LISP's** homoiconicity and runtime code evolution

> **Note on seL4 Formal Verification**: seL4 is formally verified only in specific configurations
> (e.g., single-processor, specific platforms). Multi-processor configurations, which Lona targets,
> are NOT formally verified. We choose seL4 for its strong security foundations and the code quality
> that formal verification methodology brings, but formal verification does not apply to our
> configuration and is not a goal for this system.

> **Target Platforms**: Lona aims to support x86_64 and aarch64 architectures.

---

## Table of Contents

1. [Design Principles](#1-design-principles)
2. [Architecture Overview](#2-architecture-overview)
3. [seL4 Foundation](#3-sel4-foundation)
4. [Realms: Hierarchical Protection Domains](#4-realms-hierarchical-protection-domains)
5. [Processes: Lightweight Execution Units](#5-processes-lightweight-execution-units)
6. [Multi-Core Scheduling](#6-multi-core-scheduling)
7. [Resource Management](#7-resource-management)
8. [Memory Management](#8-memory-management)
9. [Vars and Namespaces](#9-vars-and-namespaces)
10. [Message Passing](#10-message-passing)
11. [Cross-Realm Services](#11-cross-realm-services)
12. [Zero-Copy Data Sharing](#12-zero-copy-data-sharing)
13. [Device Drivers and I/O](#13-device-drivers-and-io)
14. [Virtual Address Space Layout](#14-virtual-address-space-layout)
15. [Security Model](#15-security-model)
16. [API Overview](#16-api-overview-illustrative)

---

## 1. Design Principles

### Core Philosophy

1. **Untrusted by Default**: All realms are treated as potentially compromised. Resource limits are kernel-enforced, requiring no cooperation from untrusted code.

2. **Isolation for Security, Not Performance**: Realms provide security boundaries. Process isolation within realms provides fault tolerance. Neither requires sacrificing performance.

3. **Cheap Processes, Expensive Realms**: Process creation is microseconds (pure userspace). Realm creation is milliseconds (kernel objects). Use realms for security boundaries, processes for concurrency.

4. **Zero-Copy Where Possible**: Large data shared via capability-granted memory mappings. Messages copied only at security boundaries.

5. **Late Binding for Live Updates**: Clojure-style vars enable code updates that propagate automatically to child realms without restart.

6. **Policy Compiled to Kernel Mechanisms**: Resource policies are "compiled" into seL4 scheduling contexts and capabilities. No userspace in the hot path.

### What We Take From Each System

| Source | What We Adopt |
|--------|---------------|
| **seL4** | Capabilities, VSpace/CSpace, MCS scheduling, security-focused microkernel design |
| **BEAM** | Lightweight processes, per-process heaps, reduction-based scheduling, mailboxes, immutable/persistent data structures |
| **Clojure** | Vars, namespaces, atomic updates, rich literal syntax (tuples `[]`, vectors `{}`, sets `#{}`, maps `%{}`), using data literals for function parameters |
| **LISP** | Homoiconicity, REPL-driven development, runtime code loading |

---

## 2. Architecture Overview

### Conceptual Model

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        HARDWARE (16 cores, 64 GB RAM)                       │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
┌─────────────────────────────────────│───────────────────────────────────────┐
│                              seL4 MICROKERNEL                               │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌───────────────────────┐  │
│  │ Scheduling  │ │ Capability  │ │   Memory    │ │         IPC           │  │
│  │    (MCS)    │ │   System    │ │ Management  │ │   (Endpoints, NBs)    │  │
│  └─────────────┘ └─────────────┘ └─────────────┘ └───────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
                                      │
┌─────────────────────────────────────│───────────────────────────────────────┐
│                              ROOT REALM                                     │
│                       (Trusted Computing Base)                              │
│  ┌─────────────────────────────────────────────────────────────────────┐    │
│  │  Core LISP │ Memory Pool │ CPU Scheduler │ Capability Manager       │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│         │                                                                   │
│         │ Resources + Code (read-only)                                      │
│         ▼                                                                   │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐       │
│  │  DRIVERS REALM   │    │   APPS REALM     │    │  SERVICES REALM  │       │
│  │  ┌────────────┐  │    │  ┌────────────┐  │    │  ┌────────────┐  │       │
│  │  │ Network    │  │    │  │ WebServer  │  │    │  │ Database   │  │       │
│  │  │  ├─ TX     │  │    │  │  ├─ Worker │  │    │  │            │  │       │
│  │  │  └─ RX     │  │    │  │  ├─ Worker │  │    │  └────────────┘  │       │
│  │  │ Storage    │  │    │  │  └─ Worker │  │    │                  │       │
│  │  │ UART       │  │    │  └────────────┘  │    │                  │       │
│  │  └────────────┘  │    └──────────────────┘    └──────────────────┘       │
│  └──────────────────┘                                                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Key Terminology

| Term | Definition |
|------|------------|
| **Root Realm** | The singular, privileged realm that coordinates the entire system. Acts as init system, manages memory pool, configures kernel scheduling. Part of the trusted computing base. |
| **Realm** | Protection domain with own VSpace, CSpace, and scheduler TCB(s). Unit of security isolation. All realms except root are replaceable. |
| **Process** | Lightweight execution unit within a realm. Own heap, stack, mailbox. Pure userspace construct. |
| **Resource Policy** | Optional CPU/memory min/max constraints. Realms can have explicit policies or inherit their parent's budget. |
| **Var** | Mutable reference to immutable value. Enables late binding. |
| **Namespace** | Immutable collection of var bindings. Updated atomically. |
| **Mailbox** | FIFO message queue for a process. |

---

## 3. seL4 Foundation

Lona builds directly on seL4's primitives. Understanding these is essential.

### VSpace: Virtual Address Spaces

Each realm has its own VSpace (virtual address space):

```
VSpace = seL4 page table hierarchy
  - x86_64: PML4 → PDPT → PageDirectory → PageTable → Frame
  - ARM64: PGD → PUD → PMD → PTE → Frame

Key operations:
  - Map frame at virtual address (with permissions: R, RW, RX)
  - Share physical frames between VSpaces via capability copying
  - Kernel reserves high addresses for its own use
```

### CSpace: Capability Spaces

Each realm has its own CSpace (capability space):

```
CSpace = Tree of CNodes, each containing capability slots

Capability types:
  - Untyped: Raw memory, can be retyped into other objects
  - Frame: Physical memory page, can be mapped into VSpace
  - TCB: Thread control block
  - Endpoint: IPC channel
  - CNode: Container for other capabilities
  - VSpace: Virtual address space root
  - SchedContext: CPU time budget (MCS)

Key operations:
  - Copy: Duplicate capability (possibly with reduced rights)
  - Revoke: Delete capability and all its derivatives
  - Retype: Convert Untyped into typed objects
```

### Untyped Memory

All physical memory starts as Untyped capabilities given to root task:

```
Boot:
  Kernel → Root: [Untyped 1GB, Untyped 512MB, Untyped 256MB, ...]

Root retypes as needed:
  Untyped 1GB → [Frame, Frame, Frame, ..., TCB, CNode, ...]

Watermark tracking:
  - Each Untyped tracks allocation via watermark
  - Memory before watermark: allocated
  - Memory after watermark: free
  - Revoke resets watermark, reclaims memory
```

### MCS: Mixed Criticality Scheduling

seL4's MCS configuration provides CPU time budgets:

```
SchedContext = {
  budget: u64,     // CPU time allowed per period (nanoseconds)
  period: u64,     // Replenishment period
  consumed: u64,   // Used this period
}

TCB binds to SchedContext:
  - Each TCB binds to exactly one SchedContext (1:1 binding)
  - When budget exhausted, the bound TCB is descheduled
  - Budget replenishes at start of each period
  - Kernel enforces this - no userspace bypass possible

Multi-core realms:
  - One SchedContext per scheduler TCB (one per core)
  - Each SchedContext gets a fraction of the realm's total CPU budget
  - Root realm tracks aggregate budget across all SchedContexts
  - Example: realm with 30% CPU on 4 cores → 4 SchedContexts at 7.5% each
```

### IPC: Inter-Process Communication

seL4 provides synchronous IPC via Endpoints:

```
Operations:
  seL4_Send(endpoint, msg)       // Block until receiver ready
  seL4_Recv(endpoint)            // Block until sender ready
  seL4_Call(endpoint, msg)       // Send + wait for reply
  seL4_Reply(msg)                // Reply to caller
  seL4_ReplyRecv(endpoint, msg)  // Reply + wait for next (fastpath)

Message structure:
  - Small payload in registers (fast)
  - Larger payload in IPC buffer (memory)
  - Can transfer capabilities alongside data
```

---

## 4. Realms: Hierarchical Protection Domains

### Realm Structure

A realm combines seL4 objects into a protection domain:

```clojure
%{;; seL4 kernel objects
  :vspace         <seL4-VSpace>         ; Virtual address space
  :cspace         <seL4-CNode>          ; Capability space root
  :sched-contexts {<seL4-SchedContext>} ; CPU budgets (one per scheduler TCB)
  :endpoint       <seL4-Endpoint>       ; IPC endpoint for incoming messages
  :tcbs           {<seL4-TCB>}          ; Scheduler threads (one per active core)
  :untyped        {<seL4-Untyped>}      ; Memory budget

  ;; Parent/child relationships (nil parent = root realm)
  :parent         <realm-id or nil>
  :children       #{<realm-id>}

  ;; Resource policy (optional - if nil, shares parent's budget)
  :policy         %{:cpu    %{:min <fraction> :max <fraction>}
                    :memory %{:min <bytes> :max <bytes>}}

  ;; Userspace structures (in VSpace)
  :process-table  <ConcurrentHashMap>
  :schedulers     {<SchedulerState>}
  :allocator      <MemoryAllocator>
  :namespaces     <NamespaceRegistry>}
```

### Realm Hierarchy

Realms form a tree with the root realm at the top. The root realm is special: it coordinates the entire system like an init process, managing memory distribution and kernel scheduling configuration. All other realms are subordinate and replaceable.

```
Root Realm (coordinator, 100% resources, trusted)
│
├── Drivers Realm (policy: 30% CPU, 2GB)
│   ├── Network Realm (shares Drivers' budget)
│   │   ├── TX Realm (shares Network's budget)
│   │   └── RX Realm (shares Network's budget)
│   ├── Storage Realm (shares Drivers' budget)
│   └── UART Realm (shares Drivers' budget)
│
└── Applications Realm (policy: 70% CPU, 60GB)
    ├── WebServer Realm (shares Applications' budget)
    │   ├── Worker 1 Realm (shares WebServer's budget)
    │   ├── Worker 2 Realm (shares WebServer's budget)
    │   └── Worker 3 Realm (shares WebServer's budget)
    └── Database Realm (shares Applications' budget)
```

**Key rules**:
- Root realm is singular and privileged (trusted computing base)
- All other realms are replaceable (drivers, applications, services)
- Realms can have explicit resource policies or inherit from parent
- Creating children cannot increase total resources (anti-Sybil)
- Parent can revoke child's capabilities at any time

### Realm Creation with Resource Policy

```clojure
(let [realm (realm-create
              %{:name     'webserver
                :policy   %{:cpu    %{:min 0.20 :max 0.50}   ; 20-50% CPU
                            :memory %{:min (* 4 1024 1024 1024)   ; 4 GB guaranteed
                                      :max (* 16 1024 1024 1024)}} ; 16 GB maximum
                :schedulers :auto})]                         ; One per available core
  (spawn-in realm (fn [] (webserver-main))))
```

### Child Realm Creation (Security Isolation)

```clojure
;; Inside webserver realm, create isolated worker
;; Shares webserver's resource budget, no separate policy
(let [realm (realm-create
              %{:name            'worker-1
                :parent          (self-realm)             ; Inherits budget
                :internal-weight 1})]                        ; Fair share among siblings
  (spawn-in realm (fn [] (worker-main))))
```

### Realm Lifecycle

```
Creation:
  1. Root validates resource request against policy
  2. Root allocates seL4 objects (TCBs, VSpace, CSpace, SchedContext)
  3. Root grants Untyped for memory budget
  4. Root maps inherited code pages (read-only)
  5. Root copies endpoint capability for IPC
  6. Realm is now created in dormant state (no processes running)

Execution:
  - Processes are spawned into the realm via `spawn-in`
  - Realm remains dormant until first `spawn-in` call
  - Realms and processes are independent: a realm can exist with zero processes

Termination:
  1. Root revokes all capabilities granted to realm
  2. All derived objects (frames, child realms) destroyed
  3. Memory returns to root's pool
  4. seL4 objects reclaimed
```

---

## 5. Processes: Lightweight Execution Units

Processes in Lona are modeled after BEAM/Erlang processes: lightweight, isolated, with hybrid scheduling:
- **Cooperative within realm:** Processes yield after reduction count
- **Preemptive between realms:** seL4 MCS preempts realm scheduler TCBs

### Process Characteristics

| Property | Value |
|----------|-------|
| Minimum size | ~512 bytes |
| Initial heap | 4 KB (configurable) |
| Kernel objects | None (pure userspace) |
| Creation time | ~1-10 µs (typical) |
| Max per realm | Millions |
| GC | Per-process, non-blocking |
| Communication | Message passing only |

### Process Structure

```clojure
%{:pid           <u64>              ; Unique within realm
  :status        :running           ; :running :ready :waiting :dead

  ;; Memory (heap grows down, stack grows up, meet in middle)
  :heap-start    <ptr>              ; Top of heap region
  :heap-ptr      <ptr>              ; Current heap allocation point
  :stack-start   <ptr>              ; Bottom of stack region
  :stack-ptr     <ptr>              ; Current stack pointer

  ;; Execution state
  :ip            <ptr>              ; Instruction pointer
  :env           <ptr>              ; Lexical environment chain
  :reductions    <u32>              ; Reduction counter (for preemption)

  ;; Mailbox
  :mailbox       %{:head <ptr>      ; First message
                   :tail <ptr>      ; Last message
                   :len  <u32>}     ; Message count

  ;; Scheduling
  :priority      <u8>               ; 0-255
  :last-scheduler <u8>              ; Affinity hint

  ;; GC state
  :gc-generation <u8>
  :gc-threshold  <u32>

  ;; Linking (for crash propagation)
  :links         #{<pid>}
  :monitors      #{<pid>}}
```

### Process Memory Layout

```
┌─────────────────────────────────────────┐ ← stack-start (high address)
│              STACK                      │
│         (grows downward ↓)              │
│                                         │
│   Call frames, arguments, locals,       │
│   return addresses, saved registers...  │
│                                         │
├─────────────────────────────────────────┤ ← stack-ptr (moves down)
│                                         │
│           (free space)                  │
│                                         │
│   When stack-ptr meets heap-ptr:        │
│   → Trigger garbage collection          │
│   → If still insufficient: grow region  │
│                                         │
├─────────────────────────────────────────┤ ← heap-ptr (moves up)
│              HEAP                       │
│         (grows upward ↑)                │
│                                         │
│   Lists, vectors, closures,             │
│   local bindings, message data...       │
│                                         │
└─────────────────────────────────────────┘ ← heap-start (low address)
```

This follows the standard x86_64/ARM64 ABI convention, ensuring compatibility with
debuggers, profilers, and any FFI code. The BEAM-style "heap and stack grow toward
each other" model works identically with conventional directions.

### Process Creation

```clojure
;; Basic spawn
(spawn (fn [] (worker-loop)))
;; → Returns pid immediately, process starts asynchronously

;; Spawn with options
(spawn (fn [] (worker-loop))
  %{:min-heap-size (* 64 1024)   ; 64 KB initial heap
    :priority      100})          ; Higher priority

;; Spawn-link: bidirectional crash notification
(spawn-link (fn [] (critical-worker)))
;; → If either process crashes, the other receives [:EXIT pid reason]

;; Spawn-monitor: unidirectional crash notification
(spawn-monitor (fn [] (monitored-worker)))
;; → Returns [pid monitor-ref]
;; → If spawned process crashes, caller receives [:DOWN monitor-ref pid reason]
```

### Process Scheduling (Within Realm)

Each realm runs its own cooperative/preemptive scheduler:

```clojure
(defn scheduler-loop [scheduler-id]
  (match (deque-pop-bottom (local-run-queue))
    proc when proc
      ;; Have local work
      (do
        (set-current-process! proc)
        (match (execute-until-yield proc +reduction-budget+)
          :reductions-exhausted
            (deque-push-bottom (local-run-queue) proc)

          :waiting-on-receive
            nil  ; Stays off queue until message arrives

          :terminated
            (cleanup-process proc)

          :yielded
            (deque-push-bottom (local-run-queue) proc))
        (scheduler-loop scheduler-id))  ; TCO tail call

    _
      ;; No local work - try stealing
      (do
        (match (steal-from-sibling)
          stolen when stolen
            (scheduler-loop scheduler-id)  ; TCO tail call

          _
            ;; Truly idle - let other realms run
            (do
              (process-inter-realm-messages)
              (seL4-Yield)
              (scheduler-loop scheduler-id))))))
```

**Reduction counting**: Each operation (function call, arithmetic, allocation) decrements the reduction counter. When it hits zero, the process yields. This ensures fairness without timer interrupts at the process level.

### Garbage Collection

Per-process GC using generational copying collection:

```
Young Generation (allocation area):
  - New allocations go here
  - Collected frequently (minor GC)
  - Survivors promoted to old generation

Old Generation:
  - Long-lived objects
  - Collected infrequently (major GC)
  - Triggered after N minor GCs or when old gen full

GC trigger:
  - heap-ptr meets stack-ptr
  - Explicit (gc) call
  - Memory pressure signal from root

GC does NOT stop other processes!
  - Each process has independent heap
  - GC pauses only affect that one process
  - Soft real-time guarantees
```

---

## 6. Multi-Core Scheduling

### The Challenge

With 16 cores and multiple realms, we need efficient parallelism without sacrificing isolation.

### Solution: Multiple TCBs per Realm

Each realm can have multiple scheduler TCBs, typically one per core:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              REALM                                          │
│                                                                             │
│   Shared: VSpace, CSpace, SchedContext (CPU budget)                         │
│                                                                             │
│   ┌─────────────┐ ┌─────────────┐ ┌─────────────┐     ┌─────────────┐       │
│   │ Scheduler 0 │ │ Scheduler 1 │ │ Scheduler 2 │ ... │ Scheduler N │       │
│   │   TCB 0     │ │   TCB 1     │ │   TCB 2     │     │   TCB N     │       │
│   │   Core 0    │ │   Core 1    │ │   Core 2    │     │   Core N    │       │
│   └──────┬──────┘ └──────┬──────┘ └──────┬──────┘     └──────┬──────┘       │
│          │               │               │                   │              │
│          ▼               ▼               ▼                   ▼              │
│   ┌───────────┐   ┌───────────┐   ┌───────────┐       ┌───────────┐         │
│   │ Run Queue │   │ Run Queue │   │ Run Queue │       │ Run Queue │         │
│   │ (local)   │   │ (local)   │   │ (local)   │       │ (local)   │         │
│   └───────────┘   └───────────┘   └───────────┘       └───────────┘         │
│          │               │               │                   │              │
│          └───────────────┴───────────────┴───────────────────┘              │
│                              Work Stealing                                  │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Key properties**:
- All TCBs share the same VSpace (address space)
- All TCBs share the same CSpace (capabilities)
- Each TCB has its own SchedContext (1:1 binding required by seL4 MCS)
- All SchedContexts collectively represent the realm's CPU budget
- Each TCB is bound to a specific core (affinity)
- Each TCB has its own run queue (lock-free)

### Demand-Based TCB Activation

TCBs are only active when there's work:

```clojure
(defn maybe-activate-scheduler [realm]
  (when (and (has-ready-processes? realm)
             (< (active-scheduler-count realm) (max-schedulers realm)))
    (match (find-suspended-scheduler realm)
      sched
        (do
          (atomic-store! (:state sched) :active)
          (seL4-TCB-Resume (:tcb sched))))))

(defn maybe-suspend-scheduler [sched]
  (when (and (empty? (:run-queue sched))
             (nil? (steal-work sched)))
    (do
      (atomic-store! (:state sched) :suspended)
      (seL4-TCB-Suspend (:tcb sched)))))
```

**Benefit**: Idle realms use zero CPU. A realm with 2 ready processes only has 2 active TCBs, not 16.

### Work Stealing

When a scheduler's local queue is empty, it steals from siblings:

```clojure
(defn steal-work [thief-sched]
  (let [num-scheds (count (:schedulers realm))
        start      (rand-int num-scheds)]  ; Random start avoids thundering herd
    (letfn [(try-steal [i]
              (when (< i num-scheds)
                (let [victim-id (mod (+ start i) num-scheds)]
                  (if (= victim-id (:id thief-sched))
                    (try-steal (inc i))  ; TCO tail call
                    (or (deque-steal-top (:run-queue (get-scheduler victim-id)))
                        (try-steal (inc i)))))))]  ; TCO tail call
      (try-steal 0))))
```

Uses Chase-Lev deque: owner pushes/pops from bottom (LIFO), thieves steal from top (FIFO).

### Scheduling Layers

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ Layer 3: INTRA-REALM PROCESS SCHEDULING                                     │
│                                                                             │
│   Who:    Realm's userspace scheduler                                       │
│   What:   Process ↔ Process within same realm                               │
│   Cost:   ~50-100 ns (register save/restore, no kernel)                     │
│   Freq:   Very high (millions/sec possible)                                 │
├─────────────────────────────────────────────────────────────────────────────┤
│ Layer 2: INTER-REALM TCB SCHEDULING                                         │
│                                                                             │
│   Who:    seL4 kernel (MCS scheduler)                                       │
│   What:   Realm A TCB ↔ Realm B TCB                                         │
│   Cost:   ~100-500 ns (seL4 context switch)                                 │
│   Freq:   Medium (thousands/sec typical)                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│ Layer 1: POLICY CONFIGURATION                                               │
│                                                                             │
│   Who:    Root task                                                         │
│   What:   Create realms, configure SchedContexts, adjust priorities         │
│   Cost:   ~1-10 µs (but happens rarely)                                     │
│   Freq:   Very low (realm creation, policy changes)                         │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Critical insight**: Root task sets up scheduling policy once. Kernel enforces it at runtime. No userspace in the hot path.

---

## 7. Resource Management

### Resource Policy Model

Realms can have explicit resource policies with min/max for CPU and memory:

```clojure
%{:cpu    %{:min <fraction-or-nil>   ; Guaranteed minimum (reservation)
            :max <fraction-or-nil>}  ; Hard ceiling (cap)

  :memory %{:min <bytes-or-nil>      ; Guaranteed minimum (reservation)
            :max <bytes-or-nil>}}    ; Hard ceiling (cap)
```

### Policy Interpretation

| min | max | Meaning |
|-----|-----|---------|
| nil | nil | Best-effort: no guarantees, uses what's available |
| 0.10 | nil | Reserved 10%, can burst higher if available |
| nil | 0.30 | No guarantee, but capped at 30% |
| 0.10 | 0.30 | Reserved 10%, can burst to 30%, never more |
| 0.20 | 0.20 | Exactly 20% (fixed allocation) |

### Hierarchical CPU Budgets

Children share parent's budget - they don't expand it:

```
Root Realm: 100% (coordinator)
├── Drivers: 30% (has own policy)
│   ├── Network: shares Drivers' 30%
│   │   ├── TX: shares Network's budget
│   │   └── RX: shares Network's budget
│   ├── UART: shares Drivers' 30%
│   └── Storage: shares Drivers' 30%
└── Applications: 70% (has own policy)
    └── WebServer: shares Applications' 70%
        ├── Worker 1: shares WebServer's budget
        ├── Worker 2: shares WebServer's budget
        └── Worker 3: shares WebServer's budget

Key invariant:
  Network + UART + Storage processes collectively ≤ 30%
  WebServer + Worker1 + Worker2 + Worker3 collectively ≤ 70%
```

### Anti-Sybil Protection

Creating child realms cannot increase your CPU allocation:

```clojure
;; Malicious realm has 10% CPU budget
;; Tries to create 1000 children to get more CPU

(dotimes [i 1000]
  (realm-create %{:name (str "child-" i) ...}))

;; Result: All 1001 realms (parent + children) share the same 10%
;; Attack fails - no extra CPU gained
```

### Mapping Policy to seL4 Mechanisms

**CPU max → MCS SchedContext budget** (kernel enforced):
```clojure
;; max 30% CPU with 1 second period = 300ms budget
(seL4-SchedContext-Configure sc
  %{:budget (* 0.30 1000000000)   ; 300ms in nanoseconds
    :period 1000000000})           ; 1 second
```

**CPU min → Priority** (kernel enforced, approximated):
```clojure
;; Higher min = higher priority = runs first when ready
(defn policy->priority [policy]
  (let [min-cpu (get-in policy [:cpu :min] 0)]
    (match min-cpu
      n when (>= n 0.25) 200
      n when (>= n 0.10) 150
      n when (>  n 0)    100
      _                  50)))
```

**Memory max → Untyped capability grants** (kernel enforced):
```clojure
;; Realm can only allocate from Untyped caps it possesses
;; No caps = no allocation, enforced by kernel capability system
```

### Example Configuration

```clojure
;; Network driver: guaranteed resources, hard cap
(realm-create
  %{:name   'network-driver
    :policy %{:cpu    %{:min 0.05 :max 0.15}
              :memory %{:min (* 64 1024 1024) :max (* 256 1024 1024)}}})

;; Database: high guaranteed resources, can burst
(realm-create
  %{:name   'database
    :policy %{:cpu    %{:min 0.30 :max 0.70}
              :memory %{:min (* 8 1024 1024 1024) :max (* 24 1024 1024 1024)}}})

;; Background tasks: no guarantees, hard cap
(realm-create
  %{:name   'background
    :policy %{:cpu    %{:min nil :max 0.10}
              :memory %{:min nil :max (* 512 1024 1024)}}})

;; Development sandbox: best-effort, no limits
(realm-create
  %{:name   'dev-sandbox
    :policy %{:cpu    %{:min nil :max nil}
              :memory %{:min nil :max nil}}})
```

---

## 8. Memory Management

### Dynamic Allocation from Root Pool

Memory is NOT statically partitioned at boot. Root maintains a pool:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         SYSTEM MEMORY (64 GB)                               │
│                                                                             │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │  RESERVED (sum of memory-min values)                                   │ │
│  │                                                                        │ │
│  │  Network: 64 MB    Database: 8 GB    WebApp: 4 GB    = 12 GB           │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │  DYNAMIC POOL (64 GB - 12 GB = 52 GB)                                  │ │
│  │                                                                        │ │
│  │  Available for realms to request on demand, up to their max            │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Memory Request Protocol

```clojure
;; Realm's allocator runs out of local Untyped
(defn request-memory-from-root [bytes]
  (ipc-call root-memory-endpoint
    %{:type  :memory-request
      :realm (realm-id)
      :bytes bytes}))

;; Root handles request
(defn handle-memory-request [realm-id bytes]
  (let [realm       (get-realm realm-id)
        current     (get-memory-usage realm-id)
        max-allowed (get-in (:policy realm) [:memory :max] ##Inf)
        new-total   (+ current bytes)]

    (match [new-total max-allowed (pool-has-available? bytes)]
      ;; Over limit
      [_ _ _] when (> new-total max-allowed)
        %{:status :denied :reason :over-max}

      ;; Pool has space
      [_ _ true]
        (let [caps (allocate-from-pool bytes)]
          (grant-caps-to-realm realm-id caps)
          %{:status :granted :caps caps})

      ;; Pool exhausted
      _
        %{:status :denied :reason :pool-exhausted})))
```

### Memory Lifecycle

```
1. BOOT
   Kernel → Root: All Untyped capabilities
   Root: Organizes into pool by size (1GB, 64MB, 4MB, 256KB chunks)

2. REALM CREATION
   Root: Grants Untyped for memory-min immediately (guaranteed)
   Realm: Has guaranteed minimum available

3. RUNTIME GROWTH
   Realm: "I need more memory" (IPC to root)
   Root: Checks (current + request) ≤ max
   Root: Grants from pool if available
   Realm: Receives new Untyped caps, adds to local pool

4. MEMORY PRESSURE
   Root: "Pool running low" (broadcast to realms)
   Realms: Voluntarily return unused Untyped caps
   Root: Returns caps to pool

5. REALM TERMINATION
   Root: seL4_CNode_Revoke on all granted Untypeds
   All derived objects destroyed, memory returns to pool
```

### Memory Enforcement

Memory limits are enforced by the capability system:

```
Realm can only allocate from Untyped caps it possesses.
No Untyped cap = no allocation.
Kernel enforces this - no userspace bypass possible.

Root tracks:
  - How much Untyped granted to each realm
  - Each realm's policy max
  - Pool availability

Allocation request:
  1. Realm asks root for more Untyped
  2. Root checks against policy max
  3. Root checks pool availability
  4. Root grants (or denies) Untyped caps
  5. Realm can only use what it has
```

---

## 9. Vars and Namespaces

Lona uses Clojure-style vars for late binding, enabling live code updates that propagate to child realms automatically.

### Vars: Mutable References to Immutable Values

```clojure
(def foo (fn [x] (+ x 1)))   ; foo is a var pointing to a function

(foo 5)                       ; → 6

(def foo (fn [x] (* x 2)))   ; Redefine foo

(foo 5)                       ; → 10 (new definition used immediately)
```

**Key**: Function calls go through var indirection, not direct addresses.

### Var Structure

```clojure
%{:name       'foo
  :namespace  'myapp
  :root       <AtomicPtr>      ; Pointer to current value (atomically updatable)
  :meta       <AtomicPtr>      ; Metadata (doc, type hints)
  :version    <AtomicU64>}     ; Incremented on each update
```

### Shared Var Table

The var table is in shared memory, enabling automatic update propagation:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      SHARED VAR TABLE (Physical Pages)                      │
│                                                                             │
│   Parent Realm: RW mapping            Child Realm: RO mapping               │
│                                                                             │
│   ┌──────────────────────────────────────────────────────────────────────┐  │
│   │  Var[0] foo:  root ──→ 0x3000_1000 (fn)                              │  │
│   │  Var[1] bar:  root ──→ 0x3000_2000 (fn)                              │  │
│   │  Var[2] baz:  root ──→ 0x3000_3000 (fn)                              │  │
│   └──────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
│   Same physical pages! Parent writes, child sees immediately.               │
│                                                                             │
│   When parent updates:                                                      │
│     (atomic-store! (var-root foo) new-fn-ptr)                               │
│                                                                             │
│   Child sees on next var deref:                                             │
│     (atomic-load (var-root foo)) → new-fn-ptr                               │
│                                                                             │
│   No notification needed - shared memory!                                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Hierarchical Trust Model:**

The var sharing mechanism implies that **parent realms are trusted by their children**.
A parent can update vars that children execute, which is by design: parents control
the child's execution environment.

- Parent realms CAN affect children via var updates
- This is intentional: parents create, configure, and manage children
- **Untrusted code should run in sibling realms, not child realms**
- Only the root realm's vars are trusted by all realms

**Memory Safety Guarantees:**

- Var pointers always reference memory mapped RO in the child's VSpace
- Parent cannot update a var to point outside shared code pages
- Memory referenced by vars is never freed while children exist (refcounted)
- Atomic updates ensure children never see torn pointers

### Hierarchical Var Lookup

```
1. Local realm's namespace
      ↓ not found
2. Parent realm's namespace (shared, RO)
      ↓ not found
3. Grandparent's namespace
      ↓ ...
4. Root realm's core namespace
      ↓ not found
5. Error: unbound var
```

### Shadowing

Child realms can override parent definitions locally:

```clojure
;; Parent defines:
(def add (fn [a b] (+ a b)))

;; Child shadows with logging version:
(def add (fn [a b]
  (log "adding" a b)
  (+ a b)))

;; Child's add logs, parent's add doesn't
;; Parent's definition unchanged
```

### Namespaces: Atomic Multi-Var Updates

A namespace is an immutable snapshot of var bindings:

```clojure
%{:name    'math
  :version 47
  :vars    %{'add <Var> 'sub <Var> 'mul <Var> 'div <Var>}
  :parent  <ptr-or-nil>}
```

Each `def` is atomic — the var binding update is a single pointer swap. Child realms reading the var see either the old value or the new value, never a partial state.

---

## 10. Message Passing

Processes communicate exclusively via message passing - no shared mutable state.

### Intra-Realm Messages (Fast Path)

Same VSpace, pure userspace, no kernel involvement:

```clojure
(send pid %{:type :work :data [1 2 3]})
```

**Implementation**:
1. Deep-copy message from sender's heap to receiver's heap
2. Link message to receiver's mailbox tail (lock-free MPSC queue)
3. If receiver waiting: mark ready, add to run queue

**Cost**: ~100-500 ns

### Inter-Realm Messages (Kernel Path)

Different VSpaces, requires seL4 IPC. The `send` function handles this transparently based on the target PID's realm:

```clojure
(send remote-pid %{:type :request :data ...})
```

**Implementation**:
1. Serialize message to IPC buffer
2. `seL4_Call` to target realm's endpoint
3. Target realm's scheduler receives, deserializes
4. Delivers to target process mailbox
5. Reply confirms delivery

**Cost**: ~1-10 µs

### Receive with Pattern Matching

```clojure
(receive
  %{:type :request :id id :payload p}
    (handle-request id p)

  %{:type :shutdown}
    (cleanup-and-exit)

  [:EXIT pid reason]
    (handle-linked-crash pid reason)

  :after 5000
    (handle-timeout))
```

**Selective receive**: Non-matching messages stay in mailbox.

### Message Structure

```clojure
%{:next   <ptr>          ; Linked list (mailbox)
  :sender pid            ; For replies
  :tag    <u64>          ; Type tag (for fast filtering)
  :data   <Value>}       ; The actual LISP value
```

### Large Message Optimization

Messages > 256 bytes use reference-counted shared binaries:

```clojure
(def big-data (binary large-byte-sequence))  ; Reference-counted binary

;; Sending doesn't copy the bytes
(send pid %{:type :data :payload big-data})

;; Receiver gets reference to same physical memory
;; Binary freed when refcount reaches 0
```

### Mailbox Implementation

Lock-free MPSC (Multiple Producer, Single Consumer) queue:

```clojure
;; Enqueue (multiple senders can call concurrently)
(defn mailbox-enqueue! [mailbox msg]
  (let [tail (atomic-load (:tail mailbox))]
    (if (atomic-cas! (:next tail) nil msg)
      (atomic-cas! (:tail mailbox) tail msg)
      (mailbox-enqueue! mailbox msg))))  ; TCO tail call - retry

;; Dequeue (only owner process calls)
(defn mailbox-dequeue! [mailbox]
  (let [head (atomic-load (:head mailbox))]
    (match (:next head)
      next when next
        (do
          (atomic-store! (:head mailbox) next)
          next)
      _ nil)))
```

---

## 11. Cross-Realm Services

### Process Registry

Processes can register names for discovery:

```clojure
(register 'db-server (self))
(whereis 'db-server)  ; → pid
```

### Hierarchical Lookup

`whereis` searches up the realm hierarchy:

```
1. Local realm's registry
      ↓ not found
2. Parent realm's registry
      ↓ not found
3. Continue up to root realm
      ↓ not found
4. Return nil
```

This allows parent realms to provide services discoverable by children.

### Cross-Realm Send by Name

```clojure
(send-named 'logger %{:level :error :msg "Something broke"})
;; Equivalent to: whereis + send
```

### Registry Server

Each realm runs a registry server process:

```clojure
(defn registry-server []
  (letfn [(server-loop [registry]
            (receive
              %{:type :register :name n :pid p :from f}
                (do (send f :ok)
                    (server-loop (assoc registry n p)))

              %{:type :whereis :name n :from f}
                (do (send f (get registry n :not-found))
                    (server-loop registry))

              [:DOWN pid _]
                (server-loop (dissoc-by-value registry pid))))]
    (server-loop %{})))  ; Start with empty registry
```

---

## 12. Zero-Copy Data Sharing

### Shared Regions

Large data can be shared between realms without copying:

```clojure
;; Owner creates shared region
(def dataset (make-shared-region (* 1024 1024 1024) 'corpus))  ; 1 GB

;; Fill with data
(load-into-region! dataset "/data/corpus.bin")

;; Share read-only with another realm
(share-region dataset target-realm :read-only)
```

### Implementation: Frame Capability Transfer

```
Physical Memory:
┌─────────────────────────────────────────────────────────────────┐
│                     1 GB Dataset (pages)                        │
└─────────────────────────────────────────────────────────────────┘
        ↑                                    ↑
   Frame caps (RW)                      Frame caps (RO)
   Owner's CSpace                       Reader's CSpace
        ↓                                    ↓
┌──────────────────┐               ┌──────────────────┐
│ Owner's VSpace   │               │ Reader's VSpace  │
│ (mapped RW)      │               │ (mapped RO)      │
└──────────────────┘               └──────────────────┘

Same physical pages, different permissions, zero copying!
```

### Sharing Protocol

1. Owner creates region, maps into own VSpace
2. Owner copies frame caps (with restricted rights) to target's CSpace
3. Target maps frames into own VSpace
4. Both realms access same physical memory
5. Owner can revoke via `seL4_CNode_Revoke`

### Memory-Mapped Data Pattern

```clojure
(def corpus (mmap-shared-file "/data/corpus.bin"))

(dotimes [i num-workers]
  (let [realm (realm-create
                %{:name   (symbol (str "worker-" i))
                  :shared #{%{:region corpus :access :read-only}}})]
    (spawn-in realm
      (fn []
        (let [data (get-shared-region 'corpus)
              chunk (/ (region-size data) num-workers)]
          (process-range data (* i chunk) chunk))))))
```

---

## 13. Device Drivers and I/O

In seL4, device drivers run in userspace. Lona supports zero-copy I/O.

### Network Stack Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        APPLICATION REALMS                                   │
│   [Web Server]  [DNS Client]  [Custom App]                                  │
└───────────────────────────────────┬─────────────────────────────────────────┘
                                    │ Socket API (IPC)
┌───────────────────────────────────▼─────────────────────────────────────────┐
│                           IP STACK REALM                                    │
│   TCP/UDP  │  IP Routing  │  ICMP  │  ARP                                   │
│                                                                             │
│   [RX Ring Buffer - mapped RO]        [TX Ring Buffer - mapped RW]          │
└────────────────────┬────────────────────────────────┬───────────────────────┘
                     │ Zero-copy                      │
┌────────────────────▼────────────────────────────────▼───────────────────────┐
│                       NETWORK DRIVER REALM                                  │
│   IRQ Handler  │  Ring Management  │  NIC Register Access                   │
│                                                                             │
│   [DMA RX Ring]                           [DMA TX Ring]                     │
└────────────────────┬────────────────────────────────┬───────────────────────┘
                     │ DMA                            │ MMIO
                     ▼                                ▼
              ┌─────────────────────────────────────────────┐
              │              NETWORK CARD                   │
              └─────────────────────────────────────────────┘
```

### Zero-Copy Receive Path

```
1. Hardware receives packet
   NIC DMA writes to buffer[tail]
   NIC sets descriptor status, triggers interrupt

2. Driver handles interrupt
   Reads descriptors, sees new packets
   Sends notification (not data!) to IP stack:
     %{:type :packets-ready :start 42 :count 5}

3. IP stack processes packets (zero copy!)
   Reads directly from shared RX buffer
   Parses headers in-place

4. Buffer recycling
   IP stack: %{:type :buffers-done :indices [42 43 44 45 46]}
   Driver marks buffers available
```

### DMA Buffer Sharing

```clojure
(def dma-region
  (allocate-dma-buffers
    %{:rx-ring-size 256
      :tx-ring-size 256
      :buffer-size  2048}))

(share-dma-region dma-region ip-stack-realm
  %{:rx :read-only
    :tx :read-write})
```

### Cache Coherency

DMA bypasses CPU cache. Use appropriate mappings:

```clojure
;; RX buffers: uncached (hardware writes frequently)
(map-frame frame vaddr %{:cache :uncached})

;; TX buffers: write-combine (CPU writes, hardware reads)
(map-frame frame vaddr %{:cache :write-combine})

;; Descriptor rings: uncached
(map-frame frame vaddr %{:cache :uncached})
```

---

## 14. Virtual Address Space Layout

### Complete Layout

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                       REALM VIRTUAL ADDRESS SPACE                           │
│                                                                             │
│  0x0000_0000_0000 ────────────────────────────────────────────────────────  │
│                   │  (unmapped - null pointer trap)                       │ │
│                                                                             │
│  0x0000_0010_0000 ────────────────────────────────────────────────────────  │
│                   │  GLOBAL CONTROL                                       │ │
│                   │  - Namespace epoch                                    │ │
│                   │  - Sequence lock                                      │ │
│                   │  - Global configuration                               │ │
│                                                                             │
│  0x0000_0020_0000 ────────────────────────────────────────────────────────  │
│                   │  SCHEDULER STATE (per-scheduler)                      │ │
│                   │  - Run queues                                         │ │
│                   │  - Current process pointers                           │ │
│                   │  - Scheduler stacks                                   │ │
│                                                                             │
│  0x0000_0100_0000 ────────────────────────────────────────────────────────  │
│                   │  NAMESPACE REGISTRY (ancestors, RO)                   │ │
│                   │  - Root's namespaces                                  │ │
│                   │  - Parent's namespaces                                │ │
│                                                                             │
│  0x0000_0200_0000 ────────────────────────────────────────────────────────  │
│                   │  LOCAL NAMESPACE REGISTRY (this realm, RW)            │ │
│                   │  - Realm-local namespaces                             │ │
│                   │  - Shadow namespaces                                  │ │
│                                                                             │
│  0x0000_1000_0000 ────────────────────────────────────────────────────────  │
│                   │  NAMESPACE OBJECTS (immutable snapshots)              │ │
│                   │  - Namespace structs                                  │ │
│                   │  - Var entries                                        │ │
│                                                                             │
│  0x0000_2000_0000 ────────────────────────────────────────────────────────  │
│                   │  ANCESTOR CODE PAGES (RO, shared physical)            │ │
│                   │  - Core LISP primitives                               │ │
│                   │  - Parent's compiled code                             │ │
│                   │  - Values pointed to by inherited vars                │ │
│                                                                             │
│  0x0000_3000_0000 ────────────────────────────────────────────────────────  │
│                   │  LOCAL CODE PAGES (RW, realm-specific)                │ │
│                   │  - Realm-local definitions                            │ │
│                   │  - Shadowed functions                                 │ │
│                   │  - JIT-compiled code                                  │ │
│                                                                             │
│  0x0000_4000_0000 ────────────────────────────────────────────────────────  │
│                   │  PROCESS HEAPS                                        │ │
│                   │  ┌─────────┐ ┌─────────┐ ┌─────────┐                  │ │
│                   │  │ Proc 0  │ │ Proc 1  │ │ Proc 2  │ ...              │ │
│                   │  │heap/stk │ │heap/stk │ │heap/stk │                  │ │
│                   │  └─────────┘ └─────────┘ └─────────┘                  │ │
│                                                                             │
│  0x0000_8000_0000 ────────────────────────────────────────────────────────  │
│                   │  SHARED BINARY HEAP (reference counted)               │ │
│                   │  - Large binaries (> 256 bytes)                       │ │
│                   │  - Shared across processes in realm                   │ │
│                                                                             │
│  0x0001_0000_0000 ────────────────────────────────────────────────────────  │
│                   │  CROSS-REALM SHARED REGIONS                           │ │
│                   │  - Memory-mapped files                                │ │
│                   │  - Inter-realm shared data                            │ │
│                                                                             │
│  0x00F0_0000_0000 ────────────────────────────────────────────────────────  │
│                   │  DEVICE MAPPINGS (driver realms only)                 │ │
│                   │  - MMIO registers                                     │ │
│                   │  - DMA buffers (uncached/write-combine)               │ │
│                                                                             │
│  0xFFFF_8000_0000 ────────────────────────────────────────────────────────  │
│                   │  (kernel reserved)                                    │ │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 15. Security Model

### Threat Model

All realms except root are untrusted and potentially compromised. The system must:

1. Prevent resource exhaustion attacks (CPU, memory)
2. Prevent capability leakage
3. Isolate faults to single realms
4. Enforce access control on shared resources

### Defense Mechanisms

| Threat | Defense |
|--------|---------|
| CPU exhaustion | MCS SchedContext budgets (kernel enforced) |
| Memory exhaustion | Untyped capability limits (kernel enforced) |
| Capability theft | seL4 capability system (kernel enforced) |
| Sybil attack (many children) | Hierarchical budgets (children share parent's allocation) |
| Code injection | W^X enforcement, RO code mappings |
| IPC flooding | Userspace rate limiting via mailbox (ring buffer) and notifications |
| Crash propagation | Realm isolation (different VSpaces) |

### Capability Flow

```
Root Realm (has all capabilities, trusted)
    │
    │ Grants subset to child
    ▼
Child Realm (has delegated capabilities)
    │
    │ Grants subset to its children (from own budget)
    ▼
Grandchild Realm (further restricted)
    │
    │ Cannot grant more than it has
    ▼
...and so on

Key properties:
  - Capabilities can only flow DOWN the tree
  - Child cannot have more than parent granted
  - Parent can revoke at any time
  - Kernel enforces all access checks
  - All realms except root are replaceable
```

### Isolation Guarantees

```
Process isolation (within realm):
  - Separate heaps (no shared mutable state)
  - Communication only via messages
  - Crash of one process doesn't affect others
  - Enforced by realm's scheduler (userspace)

Realm isolation (between realms):
  - Separate VSpaces (no memory access)
  - Separate CSpaces (no capability access)
  - Communication only via IPC endpoints
  - Crash of one realm doesn't affect others
  - Enforced by kernel
```

---

## 16. API Overview (Illustrative)

> **Note:** This section provides an illustrative overview. For normative API
> specifications, see `docs/lonala/`.

### Process Operations

```clojure
;; Creation
(spawn f)                           ; → pid
(spawn f opts)                      ; → pid, with %{:min-heap-size :priority}
(spawn-link f)                      ; → pid, linked (mutual crash notification)
(spawn-monitor f)                   ; → [pid ref], monitored (one-way notification)

;; Identity
(self)                              ; → own pid
(self-realm)                        ; → own realm-id

;; Messaging
(send pid msg)                      ; → :ok (async)
(receive pattern body ...)          ; → result of matching body
(receive ... :after ms body)        ; → with timeout

;; Process info
(alive? pid)                        ; → bool
(process-info pid)                  ; → map or nil
```

### Realm Operations

```clojure
;; Creation with explicit resource policy
(let [realm (realm-create
              %{:name   'my-realm
                :policy %{:cpu %{:min 0.10 :max 0.30}
                          :memory %{:min (* 1024 1024 1024) :max (* 4 1024 1024 1024)}}})]
  (spawn-in realm (fn [] ...)))

;; Creation without policy (shares parent's budget)
(let [realm (realm-create %{:name 'child :parent (self-realm)})]
  (spawn-in realm (fn [] ...)))

;; Shared memory
(make-shared-region size name)      ; → region
(share-region region realm access)  ; → :ok, access is :read-only or :read-write
(get-shared-region name)            ; → local mapping
```

### Namespace Operations

```clojure
;; Define var (atomic)
(def name value)

;; Var lookup
(var 'name)                         ; → var
(var-get v)                         ; → current value
```

### Registry Operations

```clojure
(register name pid)                 ; → :ok
(unregister name)                   ; → :ok
(whereis name)                      ; → pid or nil (hierarchical lookup)
(send-named name msg)               ; → :ok (whereis + send)
```

### Memory Operations

```clojure
;; Binaries (immutable, reference-counted)
(binary bytes)                      ; → binary from byte sequence
(binary-size bin)                   ; → u64
(binary-ref bin offset)             ; → byte

;; Bytebufs (mutable, for I/O)
(bytebuf-alloc size)                ; → bytebuf
(bytebuf-read8 buf offset)          ; → u8
(bytebuf-write8! buf offset val)    ; → bytebuf

;; Shared regions (illustrative - see lona.process)
(region-create size name)
(region-size region)
```

---

## Design Rationale

### Why seL4?

- **Capability security**: No ambient authority, all access must be explicitly granted
- **Minimal TCB**: Small kernel, most code in userspace, reduced attack surface
- **Security-focused design**: Developed with formal verification methodology, resulting in high code quality
- **Performance**: Fast IPC and context switching

> Note: While seL4's formal verification does not apply to multi-processor configurations (which we use),
> the rigorous design methodology still results in a more secure and reliable kernel than alternatives.

### Why BEAM-style Processes?

- **Isolation**: Per-process heaps eliminate shared-state bugs
- **Scalability**: Millions of lightweight processes
- **Fault tolerance**: Process crashes don't affect others
- **Soft real-time**: Per-process GC, no stop-the-world

### Why Clojure-style Vars and Syntax?

- **Live updates**: Var indirection enables code changes without restart
- **Consistency**: Atomic namespace updates prevent partial states
- **Hierarchy**: Natural code sharing from parent to child realms
- **Rich literals**: Tuples `[]`, vectors `{}`, sets `#{}`, maps `%{}` provide expressive data notation
- **Data as configuration**: Using literal syntax for function parameters and options

### Why Hierarchical Realms?

- **Least authority**: Each realm has minimal capabilities
- **Defense in depth**: Multiple isolation boundaries
- **Resource accounting**: Hierarchical budgets
- **Composability**: Build systems from isolated components

---

## References

- [seL4 Reference Manual](https://sel4.systems/Info/Docs/seL4-manual-latest.pdf)
- [seL4 MCS Tutorial](https://docs.sel4.systems/Tutorials/mcs.html)
- [The BEAM Book](https://blog.stenmans.org/theBeamBook/)
- [Clojure Reference: Vars](https://clojure.org/reference/vars)
- [Erlang Efficiency Guide: Processes](https://www.erlang.org/doc/efficiency_guide/processes.html)
- [Inside the Erlang VM (YouTube)](https://www.youtube.com/watch?v=fEqvtNxWXI4)
