# Lona Implementation Plan

This document provides a comprehensive overview of all components required to implement the Lona runtime, their dependencies, and a phased implementation strategy.

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
| **VM** | Compiler, Allocator | Dispatch Table |
| **Dispatch Table** | Value Types | - |
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
| 1.3 Value Representation | Tagged union: Integer, Symbol, Nil, Bool |

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
| 3.2 More Value Types | String, List, Vector, Map |
| 3.3 Special Forms | `def`, `let`, `if`, `do`, `fn`, `quote` |
| 3.4 Collection Primitives | `cons`, `first`, `rest`, `vector`, `hash-map` |

**Deliverable**:
```clojure
lona> (def x 42)
lona> (+ x 8)
50
```

---

### Phase 4: Functions and Closures

**Goal**: Define and call functions, lexical scope

| Task | Description |
|------|-------------|
| 4.1 Named Functions | `defn` compiles to `def` + `fn`, multi-arity |
| 4.2 Closures | Capture lexical environment, upvalue handling |
| 4.3 Tail Call Optimization | Detect tail position, reuse frame, `recur` |
| 4.4 Dispatch Table | Symbol to function mapping, late binding |

**Deliverable**:
```clojure
lona> (defn factorial [n]
        (if (<= n 1) 1 (* n (factorial (- n 1)))))
lona> (factorial 10)
3628800
```

---

### Phase 5: Embedded Standard Library

**Goal**: Load Lonala code at boot, self-hosting REPL

| Task | Description |
|------|-------------|
| 5.1 Build System Integration | `build.rs` embeds `stdlib/*.lona`, compile at boot |
| 5.2 `stdlib/core.lona` | `map`, `filter`, `reduce`, `comp`, `partial`, `str` |
| 5.3 Native Primitives | `native/read-string`, `native/eval`, `native/uart-*` |
| 5.4 `stdlib/repl.lona` | `read-line`, `print-result`, `repl-loop` |
| 5.5 Boot Sequence | Load core, load repl, call `(repl/main)` |

**Deliverable**: REPL is Lonala code: `(source repl/main)` works

---

### Phase 6: Basic Introspection

**Goal**: Inspect and modify the running system

| Task | Description |
|------|-------------|
| 6.1 Source Storage | Store source per-definition, track provenance |
| 6.2 Introspection Primitives | `source`, `doc`, `ns-publics` |
| 6.3 Hot Patching | Redefine updates dispatch table immediately |

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

### Phase 7: Multiple Processes

**Goal**: Concurrent execution within single domain

| Task | Description |
|------|-------------|
| 7.1 Process Data Structure | PID, status, heap, stack, mailbox |
| 7.2 Per-Process Heap | Each process gets own allocator |
| 7.3 Cooperative Scheduler | Run queue, yield points, context switching |
| 7.4 Process Primitives | `spawn`, `self`, `exit` |

**Deliverable**:
```clojure
lona> (spawn (fn [] (println "Hello from process!")))
#<pid:2>
Hello from process!
```

---

### Phase 8: Message Passing

**Goal**: Processes communicate via messages

| Task | Description |
|------|-------------|
| 8.1 Mailbox | FIFO message queue per process |
| 8.2 send Primitive | Copy message to target's mailbox |
| 8.3 receive Special Form | Pattern matching, selective receive, blocking |
| 8.4 Timeouts | `(after ms expr)` clause, timer management |
| 8.5 `stdlib/process.lona` | `call` (sync), `cast` (async) |

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

### Phase 9: Garbage Collection

**Goal**: Automatic memory management

| Task | Description |
|------|-------------|
| 9.1 Root Discovery | Stack, dispatch table, mailbox roots |
| 9.2 Mark-Sweep Collector | Per-process, triggered on allocation pressure |
| 9.3 GC Primitives | `gc`, `gc-stats` |

**Deliverable**: Long-running processes without OOM

---

### Phase 10: Fault Tolerance

**Goal**: Supervision trees, let it crash

| Task | Description |
|------|-------------|
| 10.1 Process Linking | `link`, `unlink`, `spawn-link` |
| 10.2 Process Monitoring | `monitor`, `demonitor`, `:DOWN` messages |
| 10.3 Exit Signals | Normal/abnormal exits, propagation, `trap-exit` |
| 10.4 Preemptive Scheduling | Reduction counting, fair preemption |
| 10.5 `stdlib/supervisor.lona` | Supervisor behavior, restart strategies |

**Deliverable**:
```clojure
lona> (def-supervisor my-sup
        :strategy :one-for-one
        :children [{:id :worker :start #(spawn worker [])}])
```

---

### Phase 11: Advanced Debugging

**Goal**: LISP-machine-style debugging

| Task | Description |
|------|-------------|
| 11.1 Stack Introspection | `current-stack-frames`, `frame-locals`, `frame-source` |
| 11.2 Breakpoints | `break-on-entry`, `break-on-exit`, conditional |
| 11.3 Tracing | `trace-calls`, `trace-messages` |
| 11.4 Condition/Restart System | `signal`, `restart-case`, `handler-bind` |
| 11.5 `stdlib/debug.lona` | Debugger UI, inspector |

**Deliverable**: Fix bugs in running system without restart

---

### Phase 12: Domain Isolation

**Goal**: Security boundaries via seL4

| Task | Description |
|------|-------------|
| 12.1 VSpace Manager | Create address spaces, map pages |
| 12.2 CSpace Manager | Capability space creation, slots, delegation |
| 12.3 Domain Creation | `spawn` with `:domain`, capability specification |
| 12.4 Domain Registry | Hierarchical naming, metadata, `find-domains` |

**Deliverable**:
```clojure
lona> (spawn sandboxed-fn []
        {:domain "sandbox:untrusted"
         :capabilities []})
```

---

### Phase 13: Inter-Domain Communication

**Goal**: Secure message passing across domains

| Task | Description |
|------|-------------|
| 13.1 seL4 IPC Integration | Endpoints, seL4 Call/Send/Recv |
| 13.2 Serialization | Values to bytes, capability transfer |
| 13.3 Transparent Routing | `send` works across domains automatically |
| 13.4 Cross-Domain Supervision | Link/monitor work cross-domain |

**Deliverable**: Supervision trees span domain boundaries

---

### Phase 14: Code Sharing & Zero-Copy

**Goal**: Efficient resource sharing

| Task | Description |
|------|-------------|
| 14.1 Read-Only Code Mapping | Share bytecode/source pages across domains |
| 14.2 Dispatch Table Cloning | Child gets copy of parent's bindings |
| 14.3 Shared Memory Regions | `create-shared-region`, `grant-capability` |
| 14.4 Code Propagation | `push-code`, `pull-code`, `on-code-push` |

**Deliverable**: Zero-copy data pipelines across domains

---

### Phase 15: I/O & Drivers

**Goal**: Real hardware interaction

| Task | Description |
|------|-------------|
| 15.1 IRQ Handling | seL4 IRQ notifications, IRQ to process message |
| 15.2 MMIO Abstraction | Memory-mapped device access |
| 15.3 Driver Framework | Driver behaviors in Lonala |
| 15.4 VirtIO Drivers | virtio-net, virtio-blk |
| 15.5 TCP/IP Stack | IP, TCP, UDP in Lonala |
| 15.6 Telnet Server | Network REPL, per-user domains |

**Deliverable**: Connect via network, interactive REPL

---

## Milestone Summary

| Phase | Milestone | Key Deliverable |
|-------|-----------|-----------------|
| 1-3 | **"Hello REPL"** | Interactive Lonala over UART |
| 4-5 | **"Self-Hosting"** | REPL is Lonala code you can modify |
| 6 | **"Inspectable"** | View source, hot-patch functions |
| 7-8 | **"Concurrent"** | Spawn processes, send messages |
| 9 | **"Sustainable"** | Long-running without memory exhaustion |
| 10 | **"Resilient"** | Supervision trees, automatic restart |
| 11 | **"Debuggable"** | Fix production bugs without restart |
| 12-14 | **"Isolated"** | Untrusted code in sandboxes |
| 15 | **"Connected"** | Network access, telnet REPL |

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
│       ├── build.rs              # Embeds stdlib/*.lona
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
│       ├── tests/                # On-target tests (Tier 2)
│       │   └── basic.rs
│       └── stdlib/               # Lonala standard library
│           ├── core.lona       # Core functions
│           ├── collections.lona
│           ├── process.lona    # Process utilities
│           ├── supervisor.lona # Supervision trees
│           ├── repl.lona       # REPL implementation
│           ├── debug.lona      # Debugging tools
│           └── io.lona         # I/O abstractions
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
| 1 | 1.1 | Memory Allocator | Bump allocator with seL4 untyped memory integration | open |
| 2 | 1.2 | UART Driver | Read/write byte operations, blocking I/O | open |
| 3 | 1.3 | Value Representation | Tagged union: Integer, Symbol, Nil, Bool | open |
| 4 | 2.1 | Lexer | Tokenize S-expressions | open |
| 5 | 2.2 | Parser | Tokens to AST, reader macros | open |
| 6 | 2.3 | Bytecode Format | Define instruction set and constant pool | open |
| 7 | 2.4 | Compiler | AST to bytecode compilation | open |
| 8 | 2.5 | VM Core | Bytecode interpreter, stack, frames | open |
| 9 | 2.6 | Primitives | Arithmetic, comparison, output functions | open |
| 10 | 3.1 | REPL Loop (Rust) | Read, parse, compile, execute, print cycle | open |
| 11 | 3.2 | Extended Value Types | String, List, Vector, Map | open |
| 12 | 3.3 | Special Forms | def, let, if, do, fn, quote | open |
| 13 | 3.4 | Collection Primitives | cons, first, rest, vector, hash-map | open |
| 14 | 4.1 | Named Functions | defn with multi-arity support | open |
| 15 | 4.2 | Closures | Lexical capture, upvalue handling | open |
| 16 | 4.3 | Tail Call Optimization | Tail position detection, frame reuse, recur | open |
| 17 | 4.4 | Dispatch Table | Symbol-to-function mapping, late binding | open |
| 18 | 5.1 | Build Integration | build.rs embeds stdlib/*.lona files | open |
| 19 | 5.2 | core.lona | map, filter, reduce, comp, partial, str | open |
| 20 | 5.3 | Native Primitives | read-string, eval, uart-read, uart-write | open |
| 21 | 5.4 | repl.lona | read-line, print-result, repl-loop | open |
| 22 | 5.5 | Boot Sequence | Load core, load repl, start REPL process | open |
| 23 | 6.1 | Source Storage | Per-definition source, provenance tracking | open |
| 24 | 6.2 | Introspection Primitives | source, doc, ns-publics | open |
| 25 | 6.3 | Hot Patching | Redefine updates dispatch table | open |
| 26 | 7.1 | Process Data Structure | PID, status, heap, stack, mailbox | open |
| 27 | 7.2 | Per-Process Heap | Independent allocator per process | open |
| 28 | 7.3 | Cooperative Scheduler | Run queue, yield points, context switch | open |
| 29 | 7.4 | Process Primitives | spawn, self, exit | open |
| 30 | 8.1 | Mailbox | FIFO message queue per process | open |
| 31 | 8.2 | send Primitive | Copy message to target mailbox | open |
| 32 | 8.3 | receive Special Form | Pattern matching, selective receive | open |
| 33 | 8.4 | Timeouts | after clause, timer management | open |
| 34 | 8.5 | process.lona | call (sync), cast (async) helpers | open |
| 35 | 9.1 | Root Discovery | Stack, dispatch table, mailbox roots | open |
| 36 | 9.2 | Mark-Sweep Collector | Per-process GC on allocation pressure | open |
| 37 | 9.3 | GC Primitives | gc, gc-stats functions | open |
| 38 | 10.1 | Process Linking | link, unlink, spawn-link | open |
| 39 | 10.2 | Process Monitoring | monitor, demonitor, DOWN messages | open |
| 40 | 10.3 | Exit Signals | Normal/abnormal exits, propagation | open |
| 41 | 10.4 | Preemptive Scheduling | Reduction counting, fair preemption | open |
| 42 | 10.5 | supervisor.lona | Supervisor behavior, restart strategies | open |
| 43 | 11.1 | Stack Introspection | current-stack-frames, frame-locals | open |
| 44 | 11.2 | Breakpoints | break-on-entry, break-on-exit | open |
| 45 | 11.3 | Tracing | trace-calls, trace-messages | open |
| 46 | 11.4 | Condition/Restart System | signal, restart-case, handler-bind | open |
| 47 | 11.5 | debug.lona | Debugger UI, inspector | open |
| 48 | 12.1 | VSpace Manager | Address space creation, page mapping | open |
| 49 | 12.2 | CSpace Manager | Capability space, slots, delegation | open |
| 50 | 12.3 | Domain Creation | spawn with :domain, capabilities | open |
| 51 | 12.4 | Domain Registry | Hierarchical naming, metadata | open |
| 52 | 13.1 | seL4 IPC Integration | Endpoints, Call/Send/Recv | open |
| 53 | 13.2 | Serialization | Values to bytes, capability transfer | open |
| 54 | 13.3 | Transparent Routing | send works across domains | open |
| 55 | 13.4 | Cross-Domain Supervision | Link/monitor across domains | open |
| 56 | 14.1 | Read-Only Code Mapping | Share bytecode/source pages | open |
| 57 | 14.2 | Dispatch Table Cloning | Child inherits parent bindings | open |
| 58 | 14.3 | Shared Memory Regions | create-shared-region, grant-capability | open |
| 59 | 14.4 | Code Propagation | push-code, pull-code | open |
| 60 | 15.1 | IRQ Handling | seL4 IRQ to process message | open |
| 61 | 15.2 | MMIO Abstraction | Memory-mapped device access | open |
| 62 | 15.3 | Driver Framework | Driver behaviors in Lonala | open |
| 63 | 15.4 | VirtIO Drivers | virtio-net, virtio-blk | open |
| 64 | 15.5 | TCP/IP Stack | IP, TCP, UDP in Lonala | open |
| 65 | 15.6 | Telnet Server | Network REPL, per-user domains | open |
