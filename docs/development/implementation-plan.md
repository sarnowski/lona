# Lona Implementation Plan

This document provides a comprehensive overview of all components required to implement the Lona runtime, their dependencies, and a phased implementation strategy.

---

## Key Principle: Lonala-First

**Everything achievable in Lonala MUST be implemented in Lonala, not Rust.**

See [Minimal Rust Runtime](minimal-rust.md) for the complete primitive specification. The Rust runtime provides only:
- Core data structure operations (cons, first, rest, nth, assoc, etc.)
- Type predicates (nil?, list?, etc.)
- Arithmetic and bitwise operations
- Hardware access (MMIO, DMA, IRQ)
- Process primitives (spawn, send)
- seL4 domain operations

**Everything else** — including the UART driver, REPL, TCP/IP stack, and supervision trees — is Lonala code.

### Critical Execution Dependencies

```
Phase 5.5 (Binary, Type Predicates, Bitwise)
    │
    ├──► Phase 7 (lona.core with map, filter, reduce)
    │
Phase 9 (Process Model)
    │
    └──► Phase 9.5 (MMIO, DMA, IRQ primitives)
              │
              └──► Phase 9.5.5 (Lonala UART Driver)
                        │
                        └──► Phase 7.4 (Lonala REPL using UART driver)
```

**Note**: The Lonala REPL (Phase 7.4) requires the Lonala UART driver (Phase 9.5.5), which requires MMIO/IRQ primitives (Phase 9.5), which requires the process model (Phase 9). This means Phase 7.4 cannot complete until Phase 9.5 is done.

---

## Component Overview

The Lona runtime must provide a complete execution environment for Lonala code on top of seL4. The following subsystems are required:

### 1. Memory Management Subsystem

| Component | Description |
|-----------|-------------|
| **Heap Allocator** | Per-process heaps with independent allocation |
| **Garbage Collector** | Per-process, incremental GC to avoid global pauses |
| **Shared Region Manager** | Zero-copy memory regions with capability-controlled access |
| **seL4 Memory Integration** | Untyped memory to frames, page table management via VSpace |

### 2. Process Scheduler

| Component | Description |
|-----------|-------------|
| **Green Thread Scheduler** | Cooperative/preemptive scheduling of Processes onto seL4 TCBs |
| **Reduction Counter** | Bytecode instruction counting for fair preemption |
| **Run Queue Manager** | Priority queues, process states (running/waiting/suspended) |
| **TCB Pool** | Manages seL4 TCBs (typically one per core per Domain) |

### 3. Compiler Pipeline

| Component | Description |
|-----------|-------------|
| **Lexer** | Tokenize Lonala S-expression source |
| **Reader/Parser** | Parse into AST, handle reader macros, data literals |
| **Macro Expander** | Compile-time macro expansion |
| **Analyzer** | Semantic analysis, namespace resolution |
| **Bytecode Compiler** | Generate bytecode from analyzed AST |
| **Bytecode Cache** | Cache compiled bytecode, invalidate on source change |
| **JIT Compiler** | Hot path optimization to native code (future) |

### 4. Virtual Machine / Interpreter

| Component | Description |
|-----------|-------------|
| **Bytecode Interpreter** | Execute bytecode instructions |
| **Dispatch Table** | Per-Domain symbol-to-bytecode mapping for late binding |
| **Call Stack Manager** | Stack frames with locals, supports introspection |
| **Pattern Matcher** | Efficient pattern matching for `receive`, destructuring |
| **Tail Call Optimizer** | Required TCO for idiomatic Lonala recursion |

### 5. Message Passing / IPC

| Component | Description |
|-----------|-------------|
| **Mailbox** | Per-process message queue |
| **Intra-Domain IPC** | Fast memory copy within same Domain |
| **Inter-Domain IPC** | seL4 IPC wrapper with serialization/capability transfer |
| **Selective Receive** | Pattern-matched message extraction with timeout |

### 6. Process Management

| Component | Description |
|-----------|-------------|
| **Process Spawner** | Create new Processes (same or new Domain) |
| **Process Registry** | Name-to-PID mapping, process lookup |
| **Link Manager** | Bidirectional process linking for crash propagation |
| **Monitor Manager** | Unidirectional process monitoring |
| **Supervisor Framework** | one-for-one, one-for-all, rest-for-one strategies |

### 7. Domain Management

| Component | Description |
|-----------|-------------|
| **Domain Creator** | seL4 VSpace + CSpace setup |
| **Capability Manager** | Delegation, attenuation, revocation |
| **Code Sharing** | Map bytecode/source pages read-only to children |
| **Dispatch Table Cloner** | Copy parent's dispatch table on spawn |
| **Domain Registry** | Hierarchical naming, metadata storage |

### 8. Source & Definition Store

| Component | Description |
|-----------|-------------|
| **Definition Database** | Per-definition storage: source, bytecode, provenance |
| **Provenance Tracker** | Origin tracking (file, REPL, network) with history |
| **Namespace Manager** | Namespace-to-definitions mapping |
| **Hot Patcher** | Atomic function redefinition, dispatch table update |
| **Export/Import** | Serialize namespace state to/from files |

### 9. Debugging & Introspection

| Component | Description |
|-----------|-------------|
| **REPL** | Read-Eval-Print loop with history |
| **Condition System** | Signal/restart mechanism (Common Lisp style) |
| **Debugger** | Breakpoints, stepping, frame inspection |
| **Tracer** | Function call and message tracing |
| **Source Inspector** | Retrieve current source for any function |
| **Process Inspector** | State, mailbox, stack inspection |

### 10. Standard Library (Core)

| Component | Description |
|-----------|-------------|
| **Primitive Types** | Integers, floats, atoms, binaries |
| **Persistent Collections** | Vectors, maps, sets with structural sharing |
| **Sequence Abstraction** | Lazy sequences, transducers |
| **String/Binary** | UTF-8 strings, binary manipulation |
| **Arithmetic** | Arbitrary precision integers, checked ops |
| **I/O Primitives** | Low-level read/write for drivers |

### 11. Hardware Abstraction Layer

| Component | Description |
|-----------|-------------|
| **Capability Wrappers** | Lonala-level cap manipulation |
| **IRQ Handler** | seL4 IRQ notification to Process message |
| **MMIO Abstraction** | Safe memory-mapped I/O |
| **DMA Manager** | Device memory for zero-copy drivers |
| **Inline Assembly** | Escape hatch for precise hardware control |

### 12. Bootstrap & Init

| Component | Description |
|-----------|-------------|
| **Root Task** | Initial seL4 task, starts runtime |
| **Capability Bootstrapper** | Parse bootinfo, organize initial caps |
| **Init Process** | First Lonala process, spawns system hierarchy |
| **Module Loader** | Load `.lona` files from storage |

---

## Dependency Analysis

### Dependency Layers

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        DEPENDENCY LAYERS                                │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Layer 7: Applications                                                  │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐                        │
│  │ Telnet REPL │ │   Drivers   │ │  User Apps  │                        │
│  └──────┬──────┘ └──────┬──────┘ └──────┬──────┘                        │
│         │               │               │                               │
│  Layer 6: Domain Isolation                                              │
│  ┌──────┴───────────────┴───────────────┴──────┐                        │
│  │  Domains · Code Sharing · Inter-Domain IPC  │                        │
│  └──────────────────────┬──────────────────────┘                        │
│                         │                                               │
│  Layer 5: Fault Tolerance                                               │
│  ┌──────────────────────┴──────────────────────┐                        │
│  │  Supervision · Linking · Monitors · Restart │                        │
│  └──────────────────────┬──────────────────────┘                        │
│                         │                                               │
│  Layer 4: Concurrency                                                   │
│  ┌──────────────────────┴──────────────────────┐                        │
│  │  Processes · Scheduler · Messages · GC      │                        │
│  └──────────────────────┬──────────────────────┘                        │
│                         │                                               │
│  Layer 3: Introspection                                                 │
│  ┌──────────────────────┴──────────────────────┐                        │
│  │  Source Storage · Hot Patching · Debugging  │                        │
│  └──────────────────────┬──────────────────────┘                        │
│                         │                                               │
│  Layer 2: Language Runtime                                              │
│  ┌──────────────────────┴──────────────────────┐                        │
│  │  Parser · Compiler · VM · Dispatch Table    │                        │
│  └──────────────────────┬──────────────────────┘                        │
│                         │                                               │
│  Layer 1: Foundation                                                    │
│  ┌──────────────────────┴──────────────────────┐                        │
│  │  Allocator · Values · UART · seL4 Bindings  │                        │
│  └─────────────────────────────────────────────┘                        │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Detailed Component Dependencies

| Component | Hard Dependencies | Soft Dependencies |
|-----------|-------------------|-------------------|
| **Allocator** | seL4 memory | - |
| **Value Types** | Allocator | - |
| **Parser** | Value Types | - |
| **Compiler** | Parser, Value Types | - |
| **Macro Expander** | Compiler, fn, quote | Collection Primitives |
| **VM** | Compiler, Allocator | Dispatch Table |
| **Dispatch Table** | Value Types | - |
| **Namespace Manager** | Dispatch Table, Compiler | Module Loader |
| **UART Driver** | seL4 bindings | - |
| **REPL (Rust)** | VM, UART | - |
| **Embedded Loader** | Compiler | - |
| **REPL (Lonala)** | Embedded Loader, Primitives | - |
| **Closures** | VM, Allocator | - |
| **TCO** | VM | - |
| **Process Struct** | Allocator, Value Types | - |
| **Scheduler** | Process Struct | - |
| **Per-Process Heap** | Allocator | - |
| **GC** | Per-Process Heap, VM (roots) | - |
| **Mailbox** | Process Struct, Value Types | - |
| **send/receive** | Mailbox, Scheduler | Pattern Matching |
| **Linking** | Process Struct, Scheduler | - |
| **Monitors** | Process Struct, Mailbox | - |
| **Supervisors** | Linking, Monitors, spawn | - |
| **Preemption** | Scheduler, VM (reduction count) | - |
| **Source Storage** | Value Types | - |
| **Hot Patching** | Dispatch Table, Compiler | Source Storage |
| **Stack Introspection** | VM | - |
| **Condition System** | Stack Introspection | - |
| **VSpace Manager** | seL4 bindings | - |
| **CSpace Manager** | seL4 bindings | - |
| **Domain Creation** | VSpace, CSpace | - |
| **Inter-Domain IPC** | Domain, seL4 IPC | Serialization |
| **Shared Regions** | seL4 memory, Capabilities | - |
| **Code Sharing** | Domain, Dispatch Table | - |

---

## Implementation Phases

### Phase 1: Foundation

**Goal**: Rust infrastructure to build upon

| Task | Description |
|------|-------------|
| 1.1 Memory Allocator | Bump allocator, seL4 untyped memory integration |
| 1.2 UART Driver | Read byte, write byte, blocking I/O |
| 1.3 Value Representation | Tagged union: Integer, Float, Symbol, Nil, Bool |

**Deliverable**: `println!("Hello from allocator + UART")`

---

### Phase 2: Minimal Interpreter

**Goal**: Execute simple Lonala expressions

| Task | Description |
|------|-------------|
| 2.1 Lexer | Tokenize S-expressions, handle `() [] {} "" ;` |
| 2.2 Parser | Tokens to AST, reader macros `' \` ~ ~@` |
| 2.3 Bytecode Format | Define instruction set, constant pool |
| 2.4 Compiler | AST to bytecode for literals, symbols, calls |
| 2.5 VM Core | Bytecode interpreter, operand stack, call frames |
| 2.6 Primitives | Arithmetic `+ - * / mod`, comparison `= < >`, output `print` |

**Deliverable**: `(print (+ 1 2))` prints `3`

---

### Phase 3: Interactive REPL (Rust)

**Goal**: Interactive development environment

| Task | Description |
|------|-------------|
| 3.1 REPL Loop | Read line, parse, compile, execute, print, error recovery |
| 3.2 More Value Types | String, List, Vector, Map, arbitrary precision Integer, Ratio |
| 3.3 Special Forms | `def`, `let`, `if`, `do`, `fn`, `quote` |
| 3.4 Collection Primitives | `cons`, `first`, `rest`, `vector`, `hash-map` |

**Deliverable**:
```clojure
lona> (def x 42)
x
lona> (+ x 8)
50
```

---

### Phase 4: Macro System

**Goal**: Compile-time code transformation

| Task | Description |
|------|-------------|
| 4.1 Quasiquote Expansion | Expand `` ` `` `~` `~@` reader forms into list construction code |
| 4.2 Macro Definition | `defmacro` special form, macro storage registry |
| 4.3 Macro Expansion Pass | Recursive expansion before compilation |
| 4.4 Macro Introspection | `macroexpand`, `macroexpand-1` primitives |

**Deliverable**:
```clojure
lona> (defmacro unless [test body]
        `(if (not ~test) ~body nil))
lona> (unless false (print "runs"))
runs
lona> (macroexpand '(unless false (print "runs")))
(if (not false) (print "runs") nil)
```

---

### Phase 5: Functions and Closures

**Goal**: Define and call functions, lexical scope

| Task | Description | Status |
|------|-------------|--------|
| 5.1a Rest Arguments | `& rest` syntax for variadic functions and macros | **Done** |
| 5.1b Multi-Arity | Multiple arities via `(fn ([x] ...) ([x y] ...))` | Pending |
| 5.2 Closures | Capture lexical environment, upvalue handling | Pending |
| 5.3 Tail Call Optimization | Detect tail position, reuse frame, `recur` | Pending |
| 5.4 Dispatch Table | Symbol to function mapping, late binding | Pending |

**5.1a Details**: Rest arguments enable:
- Variadic functions: `(fn [a b & rest] ...)`
- Core macros: `defn`, `when` defined in `lona/core.lona`
- Core library loaded at REPL boot

**Deliverable**:
```clojure
lona> (defn factorial [n]
        (if (<= n 1) 1 (* n (factorial (- n 1)))))
lona> (factorial 10)
3628800
```

---

### Phase 5.5: Core Data Extensions

**Goal**: Low-level types and operations required for systems programming and drivers

| Task | Description | Status |
|------|-------------|--------|
| 5.5.1 Binary Type | Add `Value::Binary` (raw byte buffer) to lona-core | Pending |
| 5.5.2 Type Predicates | `nil?`, `symbol?`, `list?`, `vector?`, `map?`, `fn?`, `integer?`, `string?`, `keyword?`, `binary?` | Pending |
| 5.5.3 Bitwise Operations | `bit-and`, `bit-or`, `bit-xor`, `bit-not`, `bit-shift-left`, `bit-shift-right` | Pending |
| 5.5.4 Binary Constructors | `make-binary`, `binary-len` | Pending |
| 5.5.5 Binary Mutators | `binary-get`, `binary-set`, `binary-slice`, `binary-copy!` | Pending |

**Why This Phase Exists**: These primitives are prerequisites for:
- Lonala UART driver (needs bitwise ops for register manipulation)
- Network drivers (needs binary buffers for packets)
- Any protocol parsing (needs bitwise ops)

**Deliverable**:
```clojure
lona> (def buf (make-binary 4))
lona> (binary-set buf 0 0xFF)
lona> (binary-get buf 0)
255
lona> (bit-and 0xFF 0x0F)
15
```

---

### Phase 5.6: Metadata System

**Goal**: Attach metadata to values and vars for documentation, introspection, and macro support

| Task | Description | Status |
|------|-------------|--------|
| 5.6.1 | Value metadata storage | Add optional metadata map to List, Vector, Map, Symbol | Pending |
| 5.6.2 | Var metadata | Vars carry metadata separate from their value | Pending |
| 5.6.3 | Native primitives | `meta`, `with-meta`, `vary-meta` | Pending |
| 5.6.4 | Reader syntax | Parser support for `^{...}` and `^:keyword` | Pending |
| 5.6.5 | Compiler source tracking | Auto-attach `:file`, `:line`, `:column` to defs | Pending |
| 5.6.6 | Update `def` | Handle docstrings → `:doc`, merge symbol metadata | Pending |
| 5.6.7 | Update `defmacro` | Set `:macro true` on var metadata | Pending |
| 5.6.8 | Update `defn` macro | Generate `:doc` and `:arglists` metadata | Pending |
| 5.6.9 | Refactor `macro?` | Use metadata instead of separate MacroRegistry | Pending |

**Why This Phase Exists**: Metadata is the foundation for:
- Documentation system (`doc`, `:doc` metadata)
- Macro detection via `:macro` metadata (unifies macros and functions)
- Source-level debugging (`:file`, `:line`, `:column`)
- Introspection (Phase 8 depends on this)
- Private vars (`:private` metadata for namespace access control)
- Hot-patching provenance tracking

**Key Design Decisions**:
- Metadata does NOT affect equality or hash codes
- `defmacro` remains a special form but contributes `:macro true` to metadata
- Types that support metadata: Symbol, List, Vector, Map, Var
- Types that do NOT support metadata: nil, bool, numbers, strings, binaries

**Deliverable**:
```clojure
lona> (defn greet "Greets a person" [name]
        (str "Hello, " name))
lona> (meta #'greet)
{:doc "Greets a person"
 :arglists ([name])
 :name greet
 :file "user.lona"
 :line 1}

lona> (meta #'when)
{:macro true
 :arglists ([test & body])
 :doc "Evaluates body if test is truthy"}

lona> (def v ^:private (with-meta [1 2 3] {:source "test"}))
lona> (meta v)
{:source "test"}
```

---

### Phase 6: Namespace System

**Goal**: Organize code into namespaces, avoid name collisions

| Task | Description |
|------|-------------|
| 6.1 Qualified Symbols | Parse `ns/name` syntax, extend Symbol representation |
| 6.2 Namespace Declaration | `ns` special form, namespace registry, current namespace tracking |
| 6.3 Namespace-Aware Dispatch | Extend dispatch table for qualified symbol resolution |
| 6.4 Require/Use/Refer | Load namespaces, create aliases, selectively import symbols |

**Deliverable**:
```clojure
lona> (ns my.app
        (:require [lona.core :as c]
                  [lona.string :refer [join]]))
lona> (c/map inc [1 2 3])
(2 3 4)
lona> (join ", " ["a" "b" "c"])
"a, b, c"
```

---

### Phase 7: Embedded Standard Library

**Goal**: Load Lonala code at boot, Lonala UART driver, Lonala REPL

| Task | Description |
|------|-------------|
| 7.1 Build System Integration | `build.rs` embeds `lona/*.lona`, compile at boot |
| 7.2 `lona/core.lona` | `map`, `filter`, `reduce`, `comp`, `partial`, `str`, `list`, `vector`, `hash-map` constructors |
| 7.3 Native Primitives | `read-string` (parser access) — **Note**: No uart-* or eval; UART is Lonala, eval is Lonala |
| 7.4 `lona/repl.lona` | `read-line`, `print-result`, `repl-loop` (uses UART driver) |
| 7.5 Boot Sequence | Load core, load repl, call `(lona.repl/main)` |

**Important**: The UART driver and REPL are implemented in Lonala, not Rust. This requires Phase 9.5 (MMIO/IRQ primitives) to be complete first. See dependency note below.

**Deliverable**: REPL is Lonala code: `(source lona.repl/main)` works

---

### Phase 8: Basic Introspection

**Goal**: Inspect and modify the running system

| Task | Description |
|------|-------------|
| 8.1 Source Storage | Store source per-definition, track provenance |
| 8.2 Introspection Primitives | `source`, `doc`, `ns-publics`, `ns-map` |
| 8.3 Hot Patching | Redefine updates dispatch table immediately |

**Deliverable**:
```clojure
lona> (defn greet [n] (str "Hi " n))
lona> (greet "Alice")
"Hi Alice"
lona> (defn greet [n] (str "Hello, " n "!"))
lona> (greet "Alice")
"Hello, Alice!"
```

---

### Phase 9: Multiple Processes

**Goal**: Concurrent execution within single domain

| Task | Description |
|------|-------------|
| 9.1 Process Data Structure | PID, status, heap, stack, mailbox |
| 9.2 Per-Process Heap | Each process gets own allocator |
| 9.3 Cooperative Scheduler | Run queue, yield points, context switching |
| 9.4 Process Primitives | `spawn`, `self`, `exit` |

**Deliverable**:
```clojure
lona> (spawn (fn [] (println "Hello from process!")))
#<pid:2>
Hello from process!
```

---

### Phase 9.5: Hardware Primitives

**Goal**: Enable device drivers in Lonala with MMIO, DMA, and IRQ support

| Task | Description | Status |
|------|-------------|--------|
| 9.5.1 MMIO Primitives | `peek-u8/16/32/64`, `poke-u8/16/32/64` | Pending |
| 9.5.2 DMA Primitives | `dma-alloc`, `phys-addr`, `memory-barrier` | Pending |
| 9.5.3 IRQ Primitives | `irq-wait` (blocks process until interrupt) | Pending |
| 9.5.4 Time Primitives | `now-ms`, `send-after` | Pending |
| 9.5.5 Lonala UART Driver | `lona/driver/uart.lona` using MMIO primitives | Pending |

**Dependencies**: Requires Phase 9 (process model for irq-wait blocking)

**Why This Phase Exists**: These primitives unblock:
- Lonala UART driver (Phase 7 REPL depends on this)
- Network card drivers
- Any device driver written in Lonala

**Deliverable**:
```clojure
;; UART driver in Lonala
(ns lona.driver.uart)

(def uart-base 0x09000000)

(defn write-byte [b]
  (poke-u8 uart-base b))

(defn read-byte []
  (peek-u8 uart-base))

(defn driver-loop []
  (loop []
    (irq-wait uart-irq-cap)
    (handle-data)
    (recur)))
```

---

### Phase 10: Message Passing

**Goal**: Processes communicate via messages

| Task | Description |
|------|-------------|
| 10.1 Mailbox | FIFO message queue per process |
| 10.2 send Primitive | Copy message to target's mailbox |
| 10.3 receive Special Form | Pattern matching, selective receive, blocking |
| 10.4 Timeouts | `(after ms expr)` clause, timer management |
| 10.5 `lona/process.lona` | `call` (sync), `cast` (async) |

**Deliverable**:
```clojure
lona> (def counter
        (spawn (fn []
          (loop [n 0]
            (receive
              :inc (recur (inc n))
              [:get pid] (do (send pid n) (recur n)))))))
lona> (send counter :inc)
lona> (send counter [:get (self)])
lona> (receive n n)
1
```

---

### Phase 11: Garbage Collection

**Goal**: Automatic memory management

| Task | Description |
|------|-------------|
| 11.1 Root Discovery | Stack, dispatch table, mailbox roots |
| 11.2 Mark-Sweep Collector | Per-process, triggered on allocation pressure |
| 11.3 GC Primitives | `gc`, `gc-stats` |

**Deliverable**: Long-running processes without OOM

---

### Phase 12: Fault Tolerance

**Goal**: Supervision trees, let it crash

| Task | Description |
|------|-------------|
| 12.1 Process Linking | `link`, `unlink`, `spawn-link` |
| 12.2 Process Monitoring | `monitor`, `demonitor`, `:DOWN` messages |
| 12.3 Exit Signals | Normal/abnormal exits, propagation, `trap-exit` |
| 12.4 Preemptive Scheduling | Reduction counting, fair preemption |
| 12.5 `lona/supervisor.lona` | Supervisor behavior, restart strategies |

**Deliverable**:
```clojure
lona> (def-supervisor my-sup
        :strategy :one-for-one
        :children [{:id :worker :start #(spawn worker [])}])
```

---

### Phase 13: Advanced Debugging

**Goal**: LISP-machine-style debugging

| Task | Description |
|------|-------------|
| 13.1 Stack Introspection | `current-stack-frames`, `frame-locals`, `frame-source` |
| 13.2 Breakpoints | `break-on-entry`, `break-on-exit`, conditional |
| 13.3 Tracing | `trace-calls`, `trace-messages` |
| 13.4 Condition/Restart System | `signal`, `restart-case`, `handler-bind` |
| 13.5 `lona/debug.lona` | Debugger UI, inspector |

**Deliverable**: Fix bugs in running system without restart

---

### Phase 14: Domain Isolation

**Goal**: Security boundaries via seL4

| Task | Description |
|------|-------------|
| 14.1 VSpace Manager | Create address spaces, map pages |
| 14.2 CSpace Manager | Capability space creation, slots, delegation |
| 14.3 Domain Creation | `spawn` with `:domain`, capability specification |
| 14.4 Domain Registry | Hierarchical naming, metadata, `find-domains` |

**Deliverable**:
```clojure
lona> (spawn sandboxed-fn []
        {:domain "sandbox:untrusted"
         :capabilities []})
```

---

### Phase 15: Inter-Domain Communication

**Goal**: Secure message passing across domains

| Task | Description |
|------|-------------|
| 15.1 seL4 IPC Integration | Endpoints, seL4 Call/Send/Recv |
| 15.2 Serialization | Values to bytes, capability transfer |
| 15.3 Transparent Routing | `send` works across domains automatically |
| 15.4 Cross-Domain Supervision | Link/monitor work cross-domain |

**Deliverable**: Supervision trees span domain boundaries

---

### Phase 16: Code Sharing & Zero-Copy

**Goal**: Efficient resource sharing

| Task | Description |
|------|-------------|
| 16.1 Read-Only Code Mapping | Share bytecode/source pages across domains |
| 16.2 Dispatch Table Cloning | Child gets copy of parent's bindings |
| 16.3 Shared Memory Regions | `create-shared-region`, `grant-capability` |
| 16.4 Code Propagation | `push-code`, `pull-code`, `on-code-push` |

**Deliverable**: Zero-copy data pipelines across domains

---

### Phase 17: I/O & Drivers

**Goal**: Real hardware interaction

| Task | Description |
|------|-------------|
| 17.1 IRQ Handling | seL4 IRQ notifications, IRQ to process message |
| 17.2 MMIO Abstraction | Memory-mapped device access |
| 17.3 Driver Framework | Driver behaviors in Lonala |
| 17.4 VirtIO Drivers | virtio-net, virtio-blk |
| 17.5 TCP/IP Stack | IP, TCP, UDP in Lonala |
| 17.6 Telnet Server | Network REPL, per-user domains |

**Deliverable**: Connect via network, interactive REPL

---

## Milestone Summary

| Phase | Milestone | Key Deliverable |
|-------|-----------|-----------------|
| 1-3 | **"Hello REPL"** | Interactive Lonala over UART |
| 4-7 | **"Self-Hosting"** | Macros, functions, namespaces, REPL is Lonala code |
| 8 | **"Inspectable"** | View source, hot-patch functions |
| 9-10 | **"Concurrent"** | Spawn processes, send messages |
| 11 | **"Sustainable"** | Long-running without memory exhaustion |
| 12 | **"Resilient"** | Supervision trees, automatic restart |
| 13 | **"Debuggable"** | Fix production bugs without restart |
| 14-16 | **"Isolated"** | Untrusted code in sandboxes |
| 17 | **"Connected"** | Network access, telnet REPL |

---

## Workspace Structure

The project uses a multi-crate workspace to maximize host-testability. Only `lona-runtime` depends on seL4; all other crates are testable on the development machine using standard `cargo test`.

### Crate Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         CRATE DEPENDENCIES                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  lona-runtime (seL4-specific, QEMU-tested only)                         │
│  ├── Entry point, receives bootinfo from seL4                           │
│  ├── Hardware interaction (UART, IRQ, MMIO)                             │
│  └── seL4 syscalls and capability operations                            │
│       │                                                                 │
│       ├── lona-kernel (abstractions, mostly host-testable)              │
│       │   ├── Traits for hardware abstraction                           │
│       │   ├── Domain/Process logic with mock implementations            │
│       │   ├── Scheduler, mailbox, garbage collector                     │
│       │   └── depends on: lona-core                                     │
│       │                                                                 │
│       ├── lonala-compiler (pure logic, 100% host-testable)              │
│       │   ├── AST to bytecode compilation                               │
│       │   ├── Bytecode format and instruction set                       │
│       │   └── depends on: lonala-parser, lona-core                      │
│       │                                                                 │
│       └── sel4, sel4-root-task (external dependencies)                  │
│                                                                         │
│  lonala-parser (pure logic, 100% host-testable)                         │
│  ├── Lexer: tokenize S-expressions                                      │
│  ├── Parser: tokens to AST, reader macros                               │
│  └── depends on: lona-core                                              │
│                                                                         │
│  lona-core (foundational types, 100% host-testable)                     │
│  ├── Value types (Integer, Symbol, List, Map, etc.)                     │
│  ├── Common traits and error types                                      │
│  └── no dependencies (leaf crate)                                       │
│                                                                         │
│  lona-test (test harness for QEMU tests)                                │
│  ├── Custom test framework for bare-metal                               │
│  ├── QEMU exit codes and serial output                                  │
│  └── depends on: lona-runtime (dev-dependency)                          │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Directory Layout

```
lona/
├── Cargo.toml                    # Workspace root
├── Makefile                      # Build orchestration
├── CLAUDE.md                     # AI assistant instructions
│
├── crates/
│   ├── lona-core/                # Foundational types (Tier 1 tests)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # Crate root
│   │       ├── value.rs          # Value representation
│   │       ├── symbol.rs         # Interned symbols
│   │       └── error.rs          # Common error types
│   │
│   ├── lonala-parser/            # Lexer and parser (Tier 1 tests)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # Crate root
│   │       ├── lexer.rs          # Tokenizer
│   │       ├── token.rs          # Token types
│   │       ├── parser.rs         # S-expr parser
│   │       └── ast.rs            # AST node types
│   │
│   ├── lonala-compiler/          # Bytecode compiler (Tier 1 tests)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # Crate root
│   │       ├── bytecode.rs       # Instruction definitions
│   │       ├── compiler.rs       # AST → Bytecode
│   │       └── constant_pool.rs  # Constant pool management
│   │
│   ├── lona-kernel/              # Kernel abstractions (Tier 1 + mocks)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # Crate root
│   │       ├── process.rs        # Process data structure
│   │       ├── scheduler.rs      # Green thread scheduler
│   │       ├── mailbox.rs        # Message queues
│   │       ├── gc.rs             # Garbage collector
│   │       ├── dispatch.rs       # Dispatch table
│   │       ├── vm.rs             # Bytecode interpreter
│   │       └── memory/
│   │           ├── mod.rs
│   │           ├── allocator.rs  # Allocator traits
│   │           └── heap.rs       # Per-process heap
│   │
│   ├── lona-test/                # Test harness for QEMU tests
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs            # Test runner, QEMU exit
│   │
│   └── lona-runtime/             # seL4 root task (Tier 2/3 tests)
│       ├── Cargo.toml
│       ├── build.rs              # Embeds lona/*.lona
│       ├── src/
│       │   ├── main.rs           # Entry point, receives bootinfo
│       │   ├── platform/
│       │   │   ├── mod.rs
│       │   │   ├── sel4.rs       # seL4 capability wrappers
│       │   │   ├── uart.rs       # UART driver
│       │   │   └── irq.rs        # IRQ handling
│       │   └── domain/           # Phase 12+
│       │       ├── mod.rs
│       │       ├── vspace.rs     # Address space management
│       │       ├── cspace.rs     # Capability space management
│       │       └── ipc.rs        # Inter-domain IPC
│       └── tests/                # On-target tests (Tier 2)
│           └── basic.rs
│
├── lona/                         # Lonala standard library
│   ├── core.lona                 # lona.core namespace
│   ├── collections.lona          # lona.collections namespace
│   ├── process.lona              # lona.process namespace
│   ├── supervisor.lona           # lona.supervisor namespace
│   ├── repl.lona                 # lona.repl namespace
│   ├── debug.lona                # lona.debug namespace
│   └── io.lona                   # lona.io namespace
│
├── tests/                        # Integration tests (Tier 3)
│   └── integration/
│       ├── boot_test.rs          # Boot sequence validation
│       └── repl_test.rs          # REPL smoke tests
│
└── docs/
    ├── goals.md                  # Project vision
    └── development/
        ├── implementation-plan.md
        ├── testing-strategy.md
        └── rust-coding-guidelines.md
```

### Crate Testability

| Crate | Test Tier | Dependencies | Host-Testable |
|-------|-----------|--------------|---------------|
| `lona-core` | Tier 1 | None | ✓ 100% |
| `lonala-parser` | Tier 1 | `lona-core` | ✓ 100% |
| `lonala-compiler` | Tier 1 | `lonala-parser`, `lona-core` | ✓ 100% |
| `lona-kernel` | Tier 1 | `lona-core` | ✓ With mocks |
| `lona-runtime` | Tier 2/3 | All + `sel4` | ✗ QEMU only |

See [Testing Strategy](testing-strategy.md) for details on the three-tier testing pyramid.

---

## Task Checklist

All implementation tasks with status tracking.

| # | Phase | Task | Description | Status |
|---|-------|------|-------------|--------|
| 1 | 1.1 | Memory Allocator | Bump allocator with seL4 untyped memory integration | done |
| 2 | 1.2 | UART Driver | Read/write byte operations, blocking I/O | done |
| 3 | 1.3 | Value Representation | Tagged union: Integer, Float, Symbol, Nil, Bool | done |
| 4 | 2.1 | Lexer | Tokenize S-expressions | done |
| 5 | 2.2 | Parser | Tokens to AST, reader macros | done |
| 6 | 2.3 | Bytecode Format | Define instruction set and constant pool | done |
| 7 | 2.4 | Compiler | AST to bytecode compilation | done |
| 8 | 2.5 | VM Core | Bytecode interpreter, stack, frames | done |
| 9 | 2.6 | Primitives | Arithmetic, comparison, output functions | done |
| 10 | 3.1 | REPL Loop (Rust) | Read, parse, compile, execute, print cycle | done |
| 11 | 3.2 | Extended Value Types | String, List, Vector, Map, arbitrary precision Integer, Ratio | done |
| 12 | 3.3 | Special Forms | def, let, if, do, fn, quote | done |
| 13 | 3.4 | Collection Primitives | cons, first, rest, vector, hash-map | done |
| 14 | 4.1 | Quasiquote Expansion | Expand `` ` `` `~` `~@` into list construction code | done |
| 15 | 4.2 | Macro Definition | defmacro special form, macro storage registry | done |
| 16 | 4.3 | Macro Expansion Pass | Recursive expansion before compilation | done |
| 17 | 4.4 | Macro Introspection | macroexpand, macroexpand-1 primitives | done |
| 18 | 5.1 | Named Functions | defn macro expands to def + fn, multi-arity | open |
| 19 | 5.2 | Closures | Lexical capture, upvalue handling | open |
| 20 | 5.3 | Tail Call Optimization | Tail position detection, frame reuse, recur | open |
| 21 | 5.4 | Dispatch Table | Symbol-to-function mapping, late binding | open |
| 22 | 5.5.1 | Binary Type | Add Value::Binary (raw byte buffer) to lona-core | open |
| 23 | 5.5.2 | Type Predicates | nil?, symbol?, list?, vector?, map?, fn?, integer?, string?, keyword?, binary? | open |
| 24 | 5.5.3 | Bitwise Operations | bit-and, bit-or, bit-xor, bit-not, bit-shift-left, bit-shift-right | open |
| 25 | 5.5.4 | Binary Constructors | make-binary, binary-len | open |
| 26 | 5.5.5 | Binary Mutators | binary-get, binary-set, binary-slice, binary-copy! | open |
| 27 | 5.6.1 | Value Metadata Storage | Add optional metadata map to List, Vector, Map, Symbol | open |
| 28 | 5.6.2 | Var Metadata | Vars carry metadata separate from their value | open |
| 29 | 5.6.3 | Metadata Primitives | meta, with-meta, vary-meta | open |
| 30 | 5.6.4 | Metadata Reader Syntax | Parser support for ^{...} and ^:keyword | open |
| 31 | 5.6.5 | Compiler Source Tracking | Auto-attach :file, :line, :column to defs | open |
| 32 | 5.6.6 | Update def | Handle docstrings → :doc, merge symbol metadata | open |
| 33 | 5.6.7 | Update defmacro | Set :macro true on var metadata | open |
| 34 | 5.6.8 | Update defn Macro | Generate :doc and :arglists metadata | open |
| 35 | 5.6.9 | Refactor macro? | Use metadata instead of MacroRegistry | open |
| 36 | 6.1 | Qualified Symbols | Parse ns/name syntax, extend Symbol representation | open |
| 37 | 6.2 | Namespace Declaration | ns special form, namespace registry, current namespace | open |
| 38 | 6.3 | Namespace-Aware Dispatch | Extend dispatch table for qualified symbol resolution | open |
| 39 | 6.4 | Require/Use/Refer | Load namespaces, create aliases, selectively import | open |
| 40 | 7.1 | Build Integration | build.rs embeds lona/*.lona files | open |
| 41 | 7.2 | lona.core | map, filter, reduce, comp, partial, str, list, vector, hash-map | open |
| 42 | 7.3 | Native Primitives | read-string (parser access only) | open |
| 43 | 7.4 | lona.repl | read-line, print-result, repl-loop (uses UART driver) | open |
| 44 | 7.5 | Boot Sequence | Load core, load repl, call (lona.repl/main) | open |
| 45 | 8.1 | Source Storage | Per-definition source via :source metadata | open |
| 46 | 8.2 | Introspection Primitives | source, doc, ns-publics, ns-map (use metadata) | open |
| 47 | 8.3 | Hot Patching | Redefine updates dispatch table | open |
| 48 | 9.1 | Process Data Structure | PID, status, heap, stack, mailbox | open |
| 49 | 9.2 | Per-Process Heap | Independent allocator per process | open |
| 50 | 9.3 | Cooperative Scheduler | Run queue, yield points, context switch | open |
| 51 | 9.4 | Process Primitives | spawn, self, exit | open |
| 52 | 9.5.1 | MMIO Primitives | peek-u8/16/32/64, poke-u8/16/32/64 | open |
| 53 | 9.5.2 | DMA Primitives | dma-alloc, phys-addr, memory-barrier | open |
| 54 | 9.5.3 | IRQ Primitives | irq-wait (blocks process until interrupt) | open |
| 55 | 9.5.4 | Time Primitives | now-ms, send-after | open |
| 56 | 9.5.5 | Lonala UART Driver | lona/driver/uart.lona using MMIO primitives | open |
| 57 | 10.1 | Mailbox | FIFO message queue per process | open |
| 58 | 10.2 | send Primitive | Copy message to target mailbox | open |
| 59 | 10.3 | receive Special Form | Pattern matching, selective receive | open |
| 60 | 10.4 | Timeouts | after clause, timer management | open |
| 61 | 10.5 | lona.process | call (sync), cast (async) helpers | open |
| 62 | 11.1 | Root Discovery | Stack, dispatch table, mailbox roots | open |
| 63 | 11.2 | Mark-Sweep Collector | Per-process GC on allocation pressure | open |
| 64 | 11.3 | GC Primitives | gc, gc-stats functions | open |
| 65 | 12.1 | Process Linking | link, unlink, spawn-link | open |
| 66 | 12.2 | Process Monitoring | monitor, demonitor, DOWN messages | open |
| 67 | 12.3 | Exit Signals | Normal/abnormal exits, propagation | open |
| 68 | 12.4 | Preemptive Scheduling | Reduction counting, fair preemption | open |
| 69 | 12.5 | lona.supervisor | Supervisor behavior, restart strategies | open |
| 70 | 13.1 | Stack Introspection | current-stack-frames, frame-locals | open |
| 71 | 13.2 | Breakpoints | break-on-entry, break-on-exit | open |
| 72 | 13.3 | Tracing | trace-calls, trace-messages | open |
| 73 | 13.4 | Condition/Restart System | signal, restart-case, handler-bind | open |
| 74 | 13.5 | lona.debug | Debugger UI, inspector | open |
| 75 | 14.1 | VSpace Manager | Address space creation, page mapping | open |
| 76 | 14.2 | CSpace Manager | Capability space, slots, delegation | open |
| 77 | 14.3 | Domain Creation | spawn with :domain, capabilities | open |
| 78 | 14.4 | Domain Registry | Hierarchical naming, metadata | open |
| 79 | 15.1 | seL4 IPC Integration | Endpoints, Call/Send/Recv | open |
| 80 | 15.2 | Serialization | Values to bytes, capability transfer | open |
| 81 | 15.3 | Transparent Routing | send works across domains | open |
| 82 | 15.4 | Cross-Domain Supervision | Link/monitor across domains | open |
| 83 | 16.1 | Read-Only Code Mapping | Share bytecode/source pages | open |
| 84 | 16.2 | Dispatch Table Cloning | Child inherits parent bindings | open |
| 85 | 16.3 | Shared Memory Regions | create-shared-region, grant-capability | open |
| 86 | 16.4 | Code Propagation | push-code, pull-code | open |
| 87 | 17.1 | IRQ Handling | seL4 IRQ to process message (see also 9.5.3) | open |
| 88 | 17.2 | MMIO Abstraction | Memory-mapped device access (see also 9.5.1) | open |
| 89 | 17.3 | Driver Framework | Driver behaviors in Lonala | open |
| 90 | 17.4 | VirtIO Drivers | virtio-net, virtio-blk | open |
| 91 | 17.5 | TCP/IP Stack | IP, TCP, UDP in Lonala | open |
| 92 | 17.6 | Telnet Server | Network REPL, per-user domains | open |
