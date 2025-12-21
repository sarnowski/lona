# Core Concepts

This document defines Lona's unified abstractions—the concepts that emerge from combining the four pillars. Each concept shows its **pillar lineage** (which pillars contribute to it) and provides canonical definitions and examples.

---

## Domain

**Pillar Lineage**: seL4 + BEAM

A **Domain** is the fundamental unit of isolation in Lona. It represents a security boundary that contains one or more Processes.

### Definition

| Property | Description |
|----------|-------------|
| **Memory isolation** | Processes in different Domains cannot access each other's memory |
| **Capability scope** | A Domain holds a set of capabilities determining resource access |
| **Contains Processes** | A Domain is a container, not an execution unit |
| **Forms hierarchy** | Domains are organized in a tree; children receive capabilities from parents |

### Mapping to seL4

| Lona Concept | seL4 Primitive |
|--------------|----------------|
| Domain | VSpace + CSpace |
| Memory isolation | VSpace (virtual address space) |
| Capability scope | CSpace (capability space) |

### The Only Security Boundary

**Within a Domain**: No security boundary. Processes share memory and capabilities. This is a single trust zone—if you don't trust code, don't run it in your Domain.

**Between Domains**: Complete isolation. seL4 enforces:
- Memory separation (hardware MMU)
- Capability separation (kernel CSpace)
- IPC mediation (kernel message passing)

### Domain Hierarchy

Domains form a tree rooted at system initialization:

```
Domain: root
├── Process: init
│
├── Domain: drivers
│   ├── Process: driver-supervisor
│   │
│   ├── Domain: uart-driver
│   │   └── Process: uart-main
│   │
│   └── Domain: net-driver
│       ├── Process: rx-handler
│       └── Process: tx-handler
│
├── Domain: services
│   ├── Process: service-supervisor
│   │
│   └── Domain: tcp-stack
│       ├── Process: ip-handler
│       └── Process: tcp-handler
│
└── Domain: users
    ├── Domain: user:tobias
    │   └── Process: repl
    │
    └── Domain: user:alice
        └── Process: repl
```

**Key observations**:
- Domains nest arbitrarily deep
- Each user gets their own Domain—tobias cannot access alice's memory
- Capabilities narrow at each level
- A Domain requires at least one Process to execute code

### Spawning: Same Domain vs New Domain

```clojure
;; Spawn in CURRENT Domain (fast, shared capabilities)
(spawn worker-fn args)

;; Spawn in NEW Domain (isolated, restricted capabilities)
(spawn repl/main []
       {:domain "user:tobias"
        :capabilities [user-ipc-cap (fs-cap "/home/tobias")]
        :memory-limit (megabytes 64)})

;; Spawn in EXISTING Domain (by name)
(spawn worker-fn args {:domain "user:tobias"})
```

**Same Domain**: Direct memory access, fast message passing (memory copy within same address space).

**Different Domain**: seL4 IPC, capability-controlled communication.

---

## Process

**Pillar Lineage**: BEAM + Clojure

A **Process** is the fundamental unit of execution in Lona. Every piece of running code executes within a Process.

### Definition

| Property | Description |
|----------|-------------|
| **Lightweight** | Hundreds of bytes overhead |
| **Scalable** | Millions can run concurrently |
| **Isolated heap** | Each Process has its own memory, GC'd independently |
| **Mailbox** | Queue of incoming messages |
| **Preemptive** | Scheduled via reduction counting |

### Process vs OS Thread

| Aspect | Lona Process | OS Thread |
|--------|--------------|-----------|
| Memory | Hundreds of bytes | Megabytes |
| Creation | Microseconds | Milliseconds |
| Scheduling | Reduction counting | Kernel scheduler |
| Communication | Message passing | Shared memory |
| Failure | Isolated crash | May corrupt others |

### Process Lifecycle

```
              ┌──────────────────────────────────┐
              │                                  │
              ▼                                  │
┌─────┐    ┌─────────┐    ┌─────────┐    ┌──────┴─┐
│spawn│───▶│ running │◀──▶│ waiting │───▶│  exit  │
└─────┘    └────┬────┘    └─────────┘    └────────┘
                │              ▲
                ▼              │
           ┌─────────┐         │
           │suspended│─────────┘
           └─────────┘
```

| State | Description |
|-------|-------------|
| **running** | Executing bytecode |
| **waiting** | Blocked in `receive` |
| **suspended** | Preempted by scheduler |
| **exit** | Terminated (normal or crash) |

### Message Passing

Processes communicate exclusively through messages:

```clojure
;; Send
(send pid {:type :request :data data})

;; Receive with pattern matching
(receive
  {:type :request :data data}
    (handle-request data)
  {:type :shutdown}
    (cleanup-and-exit)
  (after 5000
    (handle-timeout)))
```

Messages are immutable data (Clojure pillar). Within the same Domain, this means safe reference sharing. Across Domains, messages go through seL4 IPC.

### Isolation and Failure

Each Process has its own heap. When a Process crashes:
- Its heap is released
- Its mailbox is discarded
- Linked Processes are notified
- Other Processes continue unaffected

This is BEAM's fault isolation combined with Clojure's immutable data—crashes cannot corrupt shared state because there is no shared mutable state.

---

## Capability

**Pillar Lineage**: seL4 + Clojure

A **Capability** is an unforgeable token that grants access to a specific resource with specific rights.

### Definition

| Property | Description |
|----------|-------------|
| **Unforgeable** | Only the kernel can create capabilities |
| **Specific** | Each capability grants access to one resource |
| **Rights** | Capabilities encode what operations are permitted |
| **Delegable** | Can be passed to other Domains |
| **Attenuable** | Rights can be reduced when delegating |
| **Revocable** | Grantor can invalidate delegated capabilities |

### Capability as Data

In Lonala, capabilities are first-class values that can be stored, passed in messages, and inspected (with appropriate rights):

```clojure
;; Receive a capability
(def my-cap (receive-capability))

;; Check what rights it grants
(cap-rights my-cap)
;; => #{:read :write}

;; Attenuate before delegating
(def read-only-cap (cap-attenuate my-cap #{:read}))

;; Grant to child domain
(cap-grant child-domain read-only-cap)
```

### Common Capability Types

| Capability | Grants Access To |
|------------|------------------|
| Device capability | Hardware registers |
| IRQ capability | Interrupt handling |
| Memory capability | Memory region |
| IPC capability | Communication endpoint |
| Domain capability | Domain management |

### Capability Flow

```
Root Domain (all capabilities at boot)
    │
    ├─► grants {nic-device, nic-irq, packet-buffer:write}
    │       └──► net-driver Domain
    │
    ├─► grants {packet-buffer:read, ipc-endpoint}
    │       └──► tcp-stack Domain
    │
    └─► grants {ipc-endpoint}
            └──► user Domain
                    │
                    └─► CANNOT grant packet-buffer (doesn't have it)
```

---

## Message

**Pillar Lineage**: BEAM + Clojure

A **Message** is the unit of inter-process communication. Messages are always immutable data.

### Definition

| Property | Description |
|----------|-------------|
| **Immutable** | Messages cannot be modified after creation |
| **Data** | Messages are plain Clojure data (maps, vectors, etc.) |
| **Asynchronous** | `send` is non-blocking |
| **Pattern-matched** | `receive` selects messages by pattern |

### Message Semantics

```clojure
;; Sending is asynchronous and always succeeds
;; (unless the target process doesn't exist)
(send pid {:type :ping})

;; Messages queue in the receiver's mailbox
;; receive scans for matching patterns
(receive
  {:type :ping}
    (send sender {:type :pong})
  {:type :data :payload p}
    (process-payload p))
```

### Data as Interface

Messages are self-describing data:

```clojure
;; Request
{:type :query
 :table :users
 :where {:active true}
 :limit 100}

;; Response
{:type :result
 :rows [{:id 1 :name "Alice"} {:id 2 :name "Bob"}]
 :count 2}

;; Error
{:type :error
 :reason :table-not-found
 :table :users}
```

Benefits:
- **Inspectable**: Log, trace, or debug any message
- **Extensible**: Add fields without breaking receivers
- **Universal**: Same format for IPC, config, storage

### Cross-Domain Messages

When a message crosses a Domain boundary:
- seL4 IPC mediates the transfer
- Immutable data can be shared read-only (zero-copy)
- Mutable data (Binary) requires explicit transfer or copy

---

## Dispatch Table

**Pillar Lineage**: LISP Machine + seL4

A **Dispatch Table** is a per-Domain mapping from symbols to bytecode. It enables late binding and hot-patching while maintaining Domain isolation.

### Definition

| Property | Description |
|----------|-------------|
| **Per-Domain** | Each Domain has its own dispatch table |
| **Mutable** | Updated when functions are redefined |
| **Late binding** | Function calls resolved at runtime through table |

### How It Works

```
Function call: (process-packet pkt)
       │
       ▼
Dispatch table lookup: process-packet → bytecode-ptr
       │
       ▼
Execute bytecode
```

### Enabling Hot-Patching

When you redefine a function:

```clojure
(defn process-packet [pkt]
  ;; new implementation
  ...)
```

1. New bytecode is compiled
2. Dispatch table updated: `process-packet → new-bytecode-ptr`
3. All future calls use new implementation
4. No caller recompilation needed

### Domain Isolation

Each Domain has a private dispatch table:

```
Parent Domain                    Child Domain
┌─────────────────────┐         ┌─────────────────────┐
│ Dispatch Table      │         │ Dispatch Table      │
│ foo → bytecode-A    │  copy   │ foo → bytecode-A    │
│ bar → bytecode-B    │ ──────► │ bar → bytecode-B    │
└─────────────────────┘         └─────────────────────┘
        │                               │
        ▼                               ▼
   (shared bytecode)             (shared bytecode)
```

At spawn:
- Child receives **copy** of parent's dispatch table
- Child receives **read-only mapping** to parent's bytecode

After parent hot-patches:
- Parent's table updated
- Child's table unchanged
- **Isolation preserved**

---

## Supervisor

**Pillar Lineage**: BEAM + seL4

A **Supervisor** is a Process that manages other Processes (children), restarting them according to a strategy when they fail.

### Definition

| Property | Description |
|----------|-------------|
| **Process** | A Supervisor is itself a Process |
| **Children** | Manages a set of child Processes |
| **Strategy** | Defines restart behavior |
| **Intensity** | Limits restart frequency |

### Supervision Strategies

| Strategy | Behavior |
|----------|----------|
| `:one-for-one` | Restart only the failed child |
| `:one-for-all` | Restart all children if one fails |
| `:rest-for-one` | Restart failed child and all started after it |

### Defining a Supervisor

```clojure
(def-supervisor my-supervisor
  :strategy :one-for-one
  :max-restarts 5
  :max-seconds 60
  :children
  [{:id :worker-1
    :start #(spawn worker/start [config])
    :restart :permanent}
   {:id :worker-2
    :start #(spawn worker/start [config])
    :restart :permanent}
   {:id :metrics
    :start #(spawn metrics/collector [])
    :restart :transient}])
```

### Restart Types

| Type | Behavior |
|------|----------|
| `:permanent` | Always restart |
| `:transient` | Restart on abnormal exit only |
| `:temporary` | Never restart |

### Cross-Domain Supervision

Supervisors can manage Processes across Domain boundaries:

```
Domain: services
├── Process: connection-supervisor
│   │
│   ├── supervises ──► Domain: conn-handler-1
│   │                  └── Process: handler
│   │
│   └── supervises ──► Domain: conn-handler-2
│                      └── Process: handler
```

The supervision semantics are identical whether children are in the same Domain or different Domains. The Domain boundary affects **security isolation**, not **supervision behavior**.

### Supervision Tree Example

```
               [Application]
                    │
       ┌────────────┼────────────┐
       │            │            │
   [Database]   [Web Server]  [Cache]
       │            │            │
   ┌───┴───┐    ┌───┴───┐    ┌───┴───┐
   │       │    │       │    │       │
 [Pool] [Query] [Accept][Workers] [GC][Store]
```

When `[Query]` crashes:
1. `[Database]` supervisor detects crash
2. Strategy is `:one-for-one` → restart only `[Query]`
3. New `[Query]` Process starts with fresh state
4. System continues

---

## Condition/Restart

**Pillar Lineage**: LISP Machine + BEAM

The **Condition/Restart** system separates error detection from error handling, preserving context for interactive recovery.

### Definition

| Component | Purpose |
|-----------|---------|
| **Condition** | A signaled error or exceptional situation |
| **Restart** | A recovery option provided by error-detection code |
| **Handler** | High-level code that chooses which restart to invoke |

### How It Differs from Exceptions

**Exceptions**: Error → Stack unwinds → Context lost → Handler runs

**Conditions**: Error → Condition signaled → Stack preserved → Handler inspects → Restart chosen → Execution continues

### Basic Pattern

```clojure
;; Low-level code: detect error, provide restarts
(defn read-config [path]
  (restart-case
    (if (file-exists? path)
      (parse-config (slurp path))
      (signal :file-not-found {:path path}))

    (:retry []
      "Try reading again"
      (read-config path))

    (:use-default []
      "Use default configuration"
      default-config)

    (:use-value [config]
      "Provide configuration directly"
      config)))

;; High-level code: decide how to recover
(handler-bind
  [:file-not-found
   (fn [c]
     (if (critical-file? (:path c))
       (invoke-restart :abort)
       (invoke-restart :use-default)))]

  (start-application))
```

### Integration with Two-Mode Architecture

| Mode | Unhandled Condition |
|------|---------------------|
| **Production** | Process crashes, supervisor restarts |
| **Debug** | Debugger activates, user chooses restart |

```
╭─ PROCESS BREAK ────────────────────────────────────────────────╮
│ Condition: :file-not-found                                      │
│ Path: /etc/app.conf                                            │
╰─────────────────────────────────────────────────────────────────╯

Restarts:
  [1] :retry       - Try reading again
  [2] :use-default - Use default configuration
  [3] :use-value   - Provide configuration directly
  [4] :abort       - Crash process

proc-debug[0]> 2
;; Uses default configuration, continues execution
```

---

## Summary

| Concept | Pillar Lineage | One-Line Definition |
|---------|----------------|---------------------|
| **Domain** | seL4 + BEAM | Security/memory isolation boundary containing Processes |
| **Process** | BEAM + Clojure | Lightweight execution unit with isolated heap and mailbox |
| **Capability** | seL4 + Clojure | Unforgeable token granting specific resource access |
| **Message** | BEAM + Clojure | Immutable data sent between Processes |
| **Dispatch Table** | LISP + seL4 | Per-Domain symbol→bytecode mapping enabling hot-patching |
| **Supervisor** | BEAM + seL4 | Process that monitors and restarts children |
| **Condition/Restart** | LISP + BEAM | Error handling preserving context for recovery |

---

## Further Reading

- [System Design](system-design.md) — Implementation mechanics
- [Pillar: seL4](pillar-sel4.md) — Security foundation
- [Pillar: BEAM](pillar-beam.md) — Concurrency and resilience
- [Pillar: LISP Machine](pillar-lisp-machine.md) — Introspection and debugging
- [Pillar: Clojure](pillar-clojure.md) — Data philosophy
