# System Design

This document describes the implementation mechanics that realize Lona's goals. These are cross-cutting concerns that derive from multiple pillars working together.

---

## Memory Model for High-Throughput Data

**Pillars**: seL4 + Clojure + BEAM

Lona adopts **pure BEAM semantics** for message passing: all immutable values are deep-copied on send. This ensures per-process heap independence and instant memory reclaim on process death.

However, for high-throughput scenarios like networking, copying every byte is unacceptable. The **Binary** type is the explicit escape hatch—a reference-counted byte buffer that can be shared without copying.

### The Challenge

```
net-driver → tcp-stack → application
          copy       copy
```

Without optimization, every boundary crossing copies data. For a network packet:
- Copy from DMA buffer to driver
- Copy from driver to TCP stack
- Copy from TCP stack to application

### The Solution: Binary + Shared Memory Regions

For large data, Lona uses capability-controlled shared memory via the **Binary** type:

```clojure
;; Create a shared memory region
(def packet-buffer (create-shared-region (megabytes 16)))

;; Grant capabilities to specific Domains
(cap-grant net-driver-domain packet-buffer :read-write)
(cap-grant tcp-stack-domain packet-buffer :read-only)
```

Multiple Domains map the same physical memory. Access is controlled by capabilities:
- **Write capability**: Can modify the region
- **Read-only capability**: Can only read

### Zero-Copy Networking

```
┌─────────────────────────────────────────────────────────────────┐
│                    Shared Packet Buffer (16 MB)                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ [pkt1][pkt2][pkt3][...][pktN]                              │ │
│  └────────────────────────────────────────────────────────────┘ │
│       ▲ write              ▲ read-only                          │
│       │                    │                                    │
│  Domain: net-driver   Domain: tcp-stack                         │
└─────────────────────────────────────────────────────────────────┘
```

```clojure
;; Net driver: DMA writes packet, send tiny reference
(let [pkt-ref (region-ref packet-buffer offset length)]
  (send tcp-process {:packet pkt-ref}))  ; ~24 bytes

;; TCP stack: zero-copy read
(receive
  {:packet pkt-ref}
  (let [header (read-ref pkt-ref 0 40)]  ; direct from shared memory
    (process-packet header pkt-ref)))
```

### Why Binary is the Escape Hatch

Regular immutable values (maps, vectors, etc.) are always **deep-copied** on send—this is pure BEAM semantics. This ensures:
- Process heaps are truly independent
- Dead process memory can be instantly reclaimed
- Per-process GC with no cross-process references

The **Binary** type is the explicit exception for large data. For Binaries:
- Large binaries (> 64B intra-domain, > 4KB cross-domain) are shared by reference
- Receiver gets a read-only **View**
- Owner domain maintains the refcount
- Views are read-only—writer must be explicit owner

### Data Transfer Costs

| Scenario | Mechanism | Copy Cost |
|----------|-----------|-----------|
| Immutable values | Deep copy | 1 copy (always) |
| Binary (intra, > 64B) | Share reference | Zero copy |
| Binary (cross, > 4KB) | Shared region + capability | Zero copy |
| Binary (cross, ≤ 4KB) | Inline in seL4 IPC | 1 copy |
| Stream data (network, disk) | Shared ring buffer | Zero copy |

---

## Code Sharing Across Domains

**Pillars**: LISP Machine + seL4

When spawning a child Domain, we want to:
1. Share compiled code (efficiency)
2. Allow independent hot-patching (isolation)

### The Three Components

| Component | Mutability | Sharing Strategy |
|-----------|------------|------------------|
| **Bytecode** | Immutable once compiled | Shared read-only via page mapping |
| **Source text** | Immutable per-definition | Shared read-only via page mapping |
| **Dispatch table** | Mutable (late binding) | Private copy per Domain |

### At Domain Spawn

```
Parent Domain                          Child Domain
┌─────────────────────────────┐       ┌─────────────────────────────┐
│                             │       │                             │
│  Dispatch Table (mutable)   │       │  Dispatch Table (COPY)      │
│  ┌───────────────────────┐  │       │  ┌───────────────────────┐  │
│  │ foo → bytecode-A      │──┼─copy──│  │ foo → bytecode-A      │  │
│  │ bar → bytecode-B      │  │       │  │ bar → bytecode-B      │  │
│  └───────────────────────┘  │       │  └───────────────────────┘  │
│            │                │       │            │                │
│            ▼                │       │            ▼                │
│  ┌───────────────────────┐  │       │  ┌───────────────────────┐  │
│  │ Bytecode (read-only)  │◄─┼─share─┼──│ Shared mapping (RO)   │  │
│  │ bytecode-A            │  │       │                             │
│  │ bytecode-B            │  │       │                             │
│  └───────────────────────┘  │       └─────────────────────────────┘
└─────────────────────────────┘
```

**What happens**:
1. Child receives **read-only mapping** of parent's bytecode (same physical pages)
2. Child receives **read-only mapping** of parent's source text
3. Child receives **copy** of parent's dispatch table

### After Parent Hot-Patches

```
Parent redefines foo:

Parent Domain                          Child Domain
┌─────────────────────────────┐       ┌─────────────────────────────┐
│                             │       │                             │
│  Dispatch Table             │       │  Dispatch Table             │
│  ┌───────────────────────┐  │       │  ┌───────────────────────┐  │
│  │ foo → bytecode-A' ◄───┼──┼─NEW   │  │ foo → bytecode-A ◄────┼──┼─OLD
│  │ bar → bytecode-B      │  │       │  │ bar → bytecode-B      │  │
│  └───────────────────────┘  │       │  └───────────────────────┘  │
└─────────────────────────────┘       └─────────────────────────────┘

Parent sees: new foo
Child sees:  old foo
```

Old bytecode is kept alive (reference counted) until no Domain references it.

### Explicit Code Propagation

Updates don't propagate automatically. This is intentional—isolation by default:

```clojure
;; Parent pushes update to child (requires capability)
(push-code child-domain 'foo)

;; Child pulls update from parent (requires capability)
(pull-code parent-domain 'foo)

;; Child can accept or reject
(on-code-push [fn-name new-source]
  (if (validate-update fn-name new-source)
    (accept-update fn-name new-source)
    (reject-update fn-name)))
```

### Startup Efficiency

```
First boot:
  └── Root domain parses & compiles stdlib (~10-30 seconds)
  └── Bytecode stored in memory (read-only pages)

Spawning child domain:
  └── Map parent's bytecode pages read-only (instant)
  └── Copy dispatch table (~microseconds)
  └── Child ready (no reparse, no recompile)
```

---

## Security Mechanics

**Pillars**: seL4 + LISP Machine

### Capability Plumbing

Every capability is a kernel object. The kernel:
1. Creates capabilities at boot (or on resource creation)
2. Tracks which CSpace holds each capability
3. Validates every capability use
4. Enforces rights (read, write, grant, etc.)

```
┌─────────────────────────────────────────────────────────────────┐
│                        seL4 Kernel                              │
│                                                                 │
│  Capability Tables                                              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │ CSpace: root    │  │ CSpace: driver  │  │ CSpace: user    │ │
│  │ cap-0: uart     │  │ cap-0: nic      │  │ cap-0: ipc      │ │
│  │ cap-1: nic      │  │ cap-1: irq      │  │                 │ │
│  │ cap-2: ...      │  │ cap-2: ipc      │  │                 │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### Authority Flow

```
Boot:
  Root Domain receives: [uart, nic, disk, irq-controller, memory, ...]

Root grants to driver domain:
  [nic, nic-irq, packet-buffer:write]

Driver domain CANNOT:
  - Access uart (doesn't have capability)
  - Access disk (doesn't have capability)
  - Grant capabilities it doesn't have
```

### Revocation

Capabilities can be revoked, and revocation cascades:

```clojure
;; Parent grants cap to child
(cap-grant child-domain net-cap)

;; Child grants to grandchild
(cap-grant grandchild-domain net-cap)  ; if allowed

;; Parent revokes from child
(cap-revoke child-domain net-cap)
;; => Child loses net-cap
;; => Grandchild loses net-cap (cascade)
```

### Introspection Safety

The LISP machine pillar demands introspection. But introspection across Domains requires capability:

```clojure
;; Can always inspect within your Domain
(source my-local-function)       ; OK
(process-info (self))            ; OK

;; Cross-domain inspection requires capability
(debug-attach other-domain-pid)  ; requires :debug capability
(trace-calls 'other/function)    ; requires :trace capability
```

---

## Hot-Patching Mechanics

**Pillars**: LISP Machine + seL4

### The Pipeline

1. **Parse**: Source text → AST
2. **Compile**: AST → Bytecode
3. **Store**: Bytecode in code region (read-only after write)
4. **Update**: Dispatch table entry points to new bytecode
5. **Retain**: Old bytecode kept until unreferenced

```clojure
;; User redefines function
(defn net/checksum [data]
  (reduce #(bit-and (+ %1 %2) 0xFFFF) 0 data))
```

```
1. Parse source
        │
        ▼
2. Compile to bytecode
        │
        ▼
3. Store bytecode in code region
        │
        ▼
4. Update dispatch table
   ┌─────────────────────────┐
   │ net/checksum → old-bc   │ ──► │ net/checksum → new-bc │
   └─────────────────────────┘     └───────────────────────┘
        │
        ▼
5. Future calls use new bytecode
```

### Atomicity

Dispatch table updates are atomic. A function call either uses:
- The old implementation (before update)
- The new implementation (after update)

Never a partial or inconsistent state.

### Cross-Domain Boundaries

Hot-patching in one Domain does not affect other Domains (see Code Sharing above). This is intentional:
- Production code continues running
- Test Domain can try patches
- Explicit propagation when ready

### Rollback

Source provenance enables rollback:

```clojure
(provenance net/checksum)
;; => {:origin :repl
;;     :previous {:origin :file :file "net.lona" :line 42}}

;; Restore previous version
(restore-previous 'net/checksum)
```

---

## Two-Mode Architecture

**Pillars**: LISP Machine + BEAM

### Implementation

Each Process has a debug flag:

```
Process {
  pid: ProcessId,
  state: ProcessState,
  debug_mode: bool,          // true when debugger attached
  debug_channel: Option<Channel>,
  ...
}
```

### Error Handling Flow

```
Error occurs in Process
        │
        ▼
   debug_mode?
   ┌────┴────┐
  YES        NO
   │          │
   ▼          ▼
Pause      Crash
   │          │
   ▼          ▼
Debugger  Supervisor
presents  restarts
restarts  process
```

### Debugger Attachment

```clojure
(debug-attach pid)
;; => Process enters debug mode
;; => Future errors pause instead of crash

(debug-detach pid)
;; => Process returns to production mode
```

### Supervisor Interaction

Supervisors recognize the `:debugging` state:

```clojure
;; Supervisor sees child in :debugging state
(process-status child-pid)
;; => :debugging

;; Supervisor does NOT restart
;; Supervisor waits (configurable timeout)
```

### Breakpoint Implementation

Breakpoints modify the dispatch table temporarily:

```
Normal:     foo → bytecode-A
With break: foo → trampoline → bytecode-A
```

The trampoline:
1. Checks if breakpoint condition matches
2. If match: suspends process, notifies debugger
3. If no match: jumps to original bytecode

---

## Concurrency Model

**Pillars**: BEAM + seL4

### Scheduling

Lona uses preemptive scheduling via reduction counting:

1. Each Process gets a reduction budget (~2000 function calls)
2. Every function call decrements the counter
3. When budget exhausted, Process yields
4. Scheduler picks next runnable Process

This ensures no Process can monopolize the CPU, even without explicit yields.

### Per-Process GC

Each Process has its own heap:

```
┌─────────────────────────────────────────────────────────┐
│                        Domain                            │
│                                                         │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐           │
│  │ Process 1 │  │ Process 2 │  │ Process 3 │           │
│  │ ┌───────┐ │  │ ┌───────┐ │  │ ┌───────┐ │           │
│  │ │ Heap  │ │  │ │ Heap  │ │  │ │ Heap  │ │           │
│  │ └───────┘ │  │ └───────┘ │  │ └───────┘ │           │
│  │   (GC)    │  │   (GC)    │  │   (GC)    │           │
│  └───────────┘  └───────────┘  └───────────┘           │
└─────────────────────────────────────────────────────────┘
```

Benefits:
- GC pause in Process 1 doesn't affect Process 2
- Dead Process's heap immediately released (no GC needed)
- Small heaps = fast GC

### Message Passing Internals

Lona adopts **pure BEAM semantics**: all messages are deep-copied except for large Binaries.

**Same Domain**:
- Immutable data (maps, vectors, etc.): **deep copy** to receiver's heap
- Binary (> 64 bytes): share reference, receiver gets read-only view
- Binary (≤ 64 bytes): deep copy (avoids refcount overhead)
- This ensures process heaps are independent—dead process heap is instantly reclaimable

**Different Domain**:
- Immutable data: deep copy via serialization
- Binary (> 4 KB): shared memory region + capability grant
- Binary (≤ 4 KB): inline copy in seL4 IPC
- Capability transfer for resource delegation

---

## Condition/Restart Runtime

**Pillars**: LISP Machine + BEAM

### Implementation

Restart points are stored on a special stack:

```
Restart Stack (per Process):
┌─────────────────────────────────────────┐
│ restart-case: read-config               │
│   restarts: [:retry :use-default ...]   │
│   continuation: <saved-state>           │
├─────────────────────────────────────────┤
│ restart-case: parse-section             │
│   restarts: [:skip :use-empty]          │
│   continuation: <saved-state>           │
└─────────────────────────────────────────┘
```

When a condition is signaled:
1. Search for matching handler in handler stack
2. If found, call handler with condition
3. Handler can invoke any restart on the restart stack
4. Invoking restart restores continuation and runs restart code

### Integration with Debug Mode

In debug mode, if no handler matches:
1. Pause Process
2. Present all restarts from restart stack to user
3. User selects restart
4. Continue execution

In production mode, if no handler matches:
1. Process exits with condition as reason
2. Supervisor handles restart

---

## Summary

| Mechanism | Purpose | Key Insight |
|-----------|---------|-------------|
| **Deep copy messages** | Process isolation | BEAM semantics enable instant heap reclaim |
| **Binary sharing** | High-throughput data | Explicit escape hatch for large data |
| **Code sharing** | Fast domain spawn | Bytecode shared, dispatch tables private |
| **Capability plumbing** | Security enforcement | Kernel validates every access |
| **Hot-patching pipeline** | Live modification | Late binding via dispatch tables |
| **Two-mode architecture** | Resilience + debugging | Per-process debug flag |
| **Reduction scheduling** | Fair concurrency | No process monopolizes CPU |
| **Per-process GC** | Low latency | Isolated heaps, independent collection |
| **Restart stack** | Interactive recovery | Context preserved until recovery chosen |

---

## Further Reading

- [Core Concepts](core-concepts.md) — What these mechanisms implement
- [Pillar: seL4](pillar-sel4.md) — Security foundation
- [Pillar: BEAM](pillar-beam.md) — Concurrency model inspiration
- [Pillar: LISP Machine](pillar-lisp-machine.md) — Introspection philosophy
- [Pillar: Clojure](pillar-clojure.md) — Data philosophy
