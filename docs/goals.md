# Lona

## Vision

Lona is a general-purpose operating system that brings together three powerful paradigms into a cohesive whole:

1. **The seL4 microkernel** — providing a formally verified, capability-based foundation with unparalleled security isolation
2. **The LISP machine philosophy** — enabling complete runtime introspection, hot-patching, and system modification through a unified high-level language
3. **The BEAM/Erlang/OTP concurrency model** — delivering massive concurrency through lightweight isolated processes, fault tolerance through supervision trees, and resilience through the "let it crash" philosophy

The result is an operating system where users have complete visibility and control over every aspect of the running system, where failures are contained and recoverable, and where the full power of modern concurrent programming is available at every level of the stack.

---

## Core Philosophy

### "The Inspectable Machine"

Traditional operating systems are opaque. Source code is separate from the running system. Debugging requires special tools, core dumps, and restarts. Modifying system behavior requires recompilation and redeployment.

Lona rejects this model. Like the LISP machines of the 1980s, Lona treats the running system as a living, malleable environment:

- **Every function** in the system can be inspected, disassembled, and understood
- **Every value** can be examined at runtime
- **Every process** can be queried for its state
- **Every piece of code** can be modified without stopping the system

This philosophy extends from user applications down to device drivers. If a network driver has a bug, connect via UART, find the problematic function, fix it, and continue — no reboot required.

### "Let It Crash"

Borrowed from Erlang/OTP, Lona embraces failure as a normal part of system operation:

- Processes are cheap and disposable
- Each process has its own heap — a crash in managed Lonala code doesn't corrupt other processes
- Low-level code (inline assembly, direct hardware access) can break this isolation — that's the tradeoff for systems programming power
- Supervisor hierarchies automatically restart failed processes
- The system continues operating even as individual components fail and recover

This is not about ignoring errors. It's about building systems where errors are contained, logged, and handled systematically rather than catastrophically.

### "Capabilities, Not Permissions"

Through seL4's capability-based security model, Lona ensures that:

- Every resource access requires an unforgeable capability token
- Capabilities can be delegated with reduced rights
- No process can access resources it wasn't explicitly granted
- Security is enforced at the kernel level, not by convention

---

## Key Concepts

Lona introduces two primary abstractions that unify seL4's kernel primitives with BEAM-style concurrency:

### Process

A **Process** is the fundamental unit of execution in Lona, directly inspired by Erlang/BEAM processes:

- Extremely lightweight (hundreds of bytes overhead)
- Millions can run concurrently
- Communicates exclusively via message passing
- Has its own heap and is garbage collected independently
- Can be supervised, linked, and monitored

Every piece of running code in Lona executes within a Process. From the user's perspective, all Processes behave identically — the same API for spawning, messaging, supervision, and debugging applies universally.

### Domain

A **Domain** is a security and memory isolation boundary, implemented using seL4's VSpace (address space) and CSpace (capability space):

- Provides hardware-enforced memory isolation
- Holds a set of capabilities that determine what resources are accessible
- Contains one or more Processes
- Can spawn child Domains with reduced capabilities
- **Is not a Process itself** — a Domain is a container, not an execution unit

A Domain requires at least one Process to do any work. When you spawn a Process into a new Domain, that Process becomes the Domain's first inhabitant. A Domain with zero Processes is dormant — it exists (holding its capabilities and memory mappings) but executes nothing.

Domains form a hierarchy rooted at system initialization. A Domain can only grant capabilities it possesses to its children — privilege can never be escalated.

### How They Relate

The following example shows a realistic system hierarchy with multiple levels of nesting. Note how each telnet user session gets its own isolated Domain — a compromised or misbehaving user cannot affect other users:

```
Domain: root
├── Process: init (the first process, spawns the system)
│
├── Domain: drivers
│   ├── Process: driver-supervisor
│   │
│   ├── Domain: uart-driver (caps: uart-device, uart-irq)
│   │   └── Process: uart-main
│   │
│   ├── Domain: net-driver (caps: nic-device, nic-irq, packet-buffer:write)
│   │   ├── Process: rx-handler
│   │   ├── Process: tx-handler
│   │   └── Process: stats-collector
│   │
│   └── Domain: blk-driver (caps: virtio-blk, blk-irq)
│       └── Process: blk-main
│
├── Domain: services
│   ├── Process: service-supervisor
│   │
│   ├── Domain: tcp-stack (caps: net-driver-ipc, packet-buffer:read)
│   │   ├── Process: ip-handler
│   │   ├── Process: tcp-handler
│   │   └── Process: connection-manager
│   │
│   └── Domain: telnet (caps: tcp-stack-ipc)
│       ├── Process: listener
│       ├── Process: session-supervisor
│       │
│       │   ┌── Per-user isolation ──────────────────────────┐
│       │   │                                                │
│       ├── Domain: "user:tobias" (caps: user-ipc, fs:home/tobias)
│       │   ├── Process: repl
│       │   ├── Process: user-task-1
│       │   └── Process: user-task-2
│       │
│       ├── Domain: "user:alice" (caps: user-ipc, fs:home/alice)
│       │   └── Process: repl
│       │
│       └── Domain: "user:guest" (caps: user-ipc, fs:none)
│           └── Process: repl  (very restricted)
│
└── Domain: apps
    ├── Process: app-supervisor
    │
    └── Domain: "app:downloaded-xyz" (caps: none, sandboxed)
        └── Process: main (untrusted code, fully isolated)
```

Key observations:
- **Domains nest arbitrarily deep** — `root → services → telnet → user:tobias`
- **Each user gets their own Domain** — tobias cannot read alice's memory or files
- **Capabilities narrow at each level** — users only get what they need
- **A Domain always has at least one Process** — the Domain itself does nothing
- **Supervisors span Domain boundaries** — `session-supervisor` manages user Domains

### Mapping to seL4 Primitives

| Lona Concept | seL4 Primitive | Purpose |
|--------------|----------------|---------|
| Domain | VSpace + CSpace | Memory isolation + capability access control |
| Process | Multiplexed on TCBs | Lightweight concurrency |
| (hidden) TCB | Thread Control Block | Kernel scheduling entity — not exposed to users |

seL4's Thread Control Blocks (TCBs) are an implementation detail. The Lona runtime creates TCBs as needed (typically one per CPU core per Domain) and multiplexes Processes onto them using cooperative/preemptive green thread scheduling. Users never interact with TCBs directly.

### Spawning: Same Domain vs New Domain

When spawning a Process, you choose whether it shares the current Domain or gets a new one:

```clojure
;; Spawn in current Domain (lightweight, fast intra-domain messaging)
(spawn worker-fn args)

;; Spawn in a NEW Domain with a user-defined name
;; Creates the domain if it doesn't exist; reuses if it does
(spawn repl/main []
       {:domain "user:tobias"
        :capabilities [user-ipc-cap (fs-cap "/home/tobias")]
        :memory-limit (megabytes 64)})

;; Spawn with metadata for debugging and filtering
(spawn session/handler [socket]
       {:domain (str "session:" (unique-id))
        :capabilities [session-caps]
        :meta {:type :user-session
               :user "tobias"
               :connected-at (now)
               :remote-addr "192.168.1.5"}})

;; Spawn into an existing Domain (by name or reference)
(spawn worker-fn args {:domain "user:tobias"})
(spawn worker-fn args {:domain existing-domain-ref})
```

**Domain naming:**
- Named domains use strings: `"user:tobias"`, `"driver:uart"`, `"app:xyz"`
- Names are hierarchical within parent: telnet's `"user:tobias"` is distinct from another domain's
- Spawning into a named domain that exists adds a Process to it
- Spawning into a named domain that doesn't exist creates it first
- Use `(unique-id)` for system-generated unique identifiers when names aren't meaningful

**Domain metadata:**
- Attach arbitrary key-value data via `:meta`
- Query domains by metadata: `(find-domains {:type :user-session})`
- Useful for debugging, monitoring, and administration
- Metadata is informational — it doesn't affect security or isolation

From spawn onward, **all Processes use the identical API**:

```clojure
;; These work exactly the same regardless of Domain boundaries
(send pid {:request :data})
(receive
  {:response resp} (handle resp)
  (after 5000 (timeout)))

;; Supervision works the same
(link pid)
(monitor pid)

;; Query domain information
(domain-of pid)           ; => "user:tobias"
(domain-meta pid)         ; => {:type :user-session, :user "tobias", ...}
(same-domain? pid1 pid2)  ; => true/false
```

The runtime transparently handles the difference:
- **Same Domain**: Message is a memory copy (fast)
- **Different Domain**: Message goes via seL4 IPC (still fast, kernel-mediated)

### Domain Hierarchy and Capability Delegation

Domains form a tree structure. Each Domain can create child Domains, delegating a subset of its capabilities:

```clojure
;; Process in driver-supervisor domain creates isolated driver
(defn start-net-driver []
  (spawn net-driver/main []
         {:domain "driver:net"
          :capabilities [(attenuate nic-cap :read-write)
                         (attenuate irq-cap :nic-only)]
          :memory-limit (megabytes 16)
          :meta {:type :driver, :device :network}}))

;; Telnet session-supervisor creates per-user domains
(defn start-user-session [username socket]
  (let [user-caps (get-user-capabilities username)]
    (spawn repl/main [socket]
           {:domain (str "user:" username)
            :capabilities user-caps
            :memory-limit (megabytes 64)
            :meta {:type :user-session
                   :user username
                   :started (now)}})))

;; If user "tobias" connects twice, both REPLs run in the same domain
;; They share capabilities and can communicate efficiently
(start-user-session "tobias" socket1)  ; creates "user:tobias" domain
(start-user-session "tobias" socket2)  ; reuses "user:tobias" domain
```

Rules:
1. **A child can only receive capabilities the parent has**
2. **Capabilities can be attenuated** (reduced rights) when delegated
3. **Revocation cascades** — revoking a parent's capability affects all descendants
4. **Names are scoped** — `"user:tobias"` under telnet is different from `"user:tobias"` under ssh

This creates natural alignment between the supervision tree and the security hierarchy.

---

## The Lonala Language

**Lonala** is the system programming language for Lona. It is the sole language for the entire userland — there is no foreign function interface and no support for third-party C libraries. Everything from device drivers to applications is written in Lonala.

### From Clojure

- **S-expression syntax** — code as data, enabling powerful metaprogramming
- **Immutable data structures by default** — persistent data structures with structural sharing
- **Rich set of data literals** — vectors `[]`, maps `{}`, sets `#{}`
- **Sequence abstraction** — uniform interface over all collections
- **Namespaces** — for code organization and avoiding conflicts

### From Erlang/BEAM

- **Processes** as the fundamental unit of concurrency
- **Message passing** as the only means of inter-process communication
- **Pattern matching** for elegant control flow and data destructuring
- **Process linking and monitoring** for building supervision trees
- **Hot code loading** for updating running systems

### For Systems Programming

- **Direct hardware access** — memory-mapped I/O, interrupt handling
- **Inline assembly** — when you need precise control
- **Low-level memory operations** — for implementing drivers
- **Capability manipulation** — first-class support for seL4 capabilities
- **Shared memory regions** — for zero-copy data exchange between Domains
- **Tail Call Optimization (TCO)** — required; recursion is the primary iteration pattern in LISP

### Example Syntax

```clojure
;; Define a stateful process using receive loop
(defn counter [state]
  (receive
    :increment       (recur (inc state))
    :decrement       (recur (dec state))
    [:get pid]       (do (send pid state)
                         (recur state))
    :stop            :ok))

;; Spawn and interact
(def c (spawn counter 0))
(send c :increment)
(send c :increment)
(send c [:get (self)])  ; => receive 2

;; Supervisor definition
(def-supervisor app-supervisor
  :strategy :one-for-one
  :children
  [{:id :worker-1 :start #(spawn worker-fn [])}
   {:id :worker-2 :start #(spawn worker-fn [])}
   {:id :sandbox  :start #(spawn untrusted-fn []
                                  {:domain "sandbox:untrusted"
                                   :capabilities []
                                   :meta {:type :sandbox}})}])
```

**Value proposition:** A single language for writing everything from device drivers to high-level applications, with the expressiveness of LISP and the concurrency model of Erlang.

---

## Source-Only Distribution

Lona takes a radical approach to code distribution: **source code is the only distributable format**.

### No Ahead-of-Time Compilation

There is no external compiler. There are no pre-compiled binaries. The only way to get code into Lona is to load source files.

*Note: The Lonala runtime itself (compiler, scheduler, garbage collector) is native code, shipped as part of the kernel image. This is analogous to a JVM or BEAM VM — the runtime is a binary, but all user-facing code is source. Users never compile anything; they write Lonala source and load it.*

```
Traditional systems:                 Lona:

Developer machine                    Developer machine
  └── compiler ────┐                   └── source files only
                   │                            │
                   ▼                            ▼
         binary package                  source package (.lona-p)
                   │                            │
                   ▼                            ▼
         Target machine                Target machine (Lona)
           └── runs binary               └── Lona runtime reads source
                                              └── compiles to bytecode
                                              └── JIT compiles hot paths
                                              └── caches for fast restart
```

### How Loading Works

```clojure
;; Loading a module reads source, compiles, and caches
(load "tcp-stack/core")

;; First load: parse → compile → cache bytecode (slower)
;; Subsequent loads: load cached bytecode (fast)
;; Cache invalidated automatically if source changes
```

The bytecode cache is purely an internal optimization — an implementation detail not exposed to the rest of the system. Source code is always canonical.

### Why Source-Only?

| Benefit | Explanation |
|---------|-------------|
| **Total transparency** | You can always see exactly what code is running |
| **Full debugging everywhere** | Source is always available — no "missing symbols" |
| **No binary compatibility issues** | Same package works on ARM, x86, any platform |
| **Security auditing** | Inspect any package before loading |
| **Hot patching** | Modify source, reload — natural workflow |
| **No toolchain required** | Developers don't need compiler installations |
| **Reproducibility** | Same source produces same behavior (aim for deterministic compilation) |

### File Extensions

| Extension | Description |
|-----------|-------------|
| `.lona-s` | Lonala source code file |
| `.lona-p` | Lonala source package (bundled source files + metadata) |

### What This Means

- **The compiler is part of Lona** — not a separate tool developers install
- **All compilation happens at load time** — just like LISP machines
- **No obfuscation** — defeats the purpose; if you don't trust users with your source, don't distribute to them
- **Developers write source, upload source, debug source** — the full cycle

---

## Source Code Storage and Hot-Patching

Hot-patching is a core capability of Lona. To support it properly, we need a clear model for how source code is stored and how changes are tracked.

### Per-Definition Storage

Source code is stored **per-definition**, not per-file. Each function, macro, or value definition is an independent unit with its own:

| Component | Description |
|-----------|-------------|
| **Source text** | The raw source code as text (preserves formatting, comments) |
| **Provenance** | Where this definition came from (file, REPL, network) |
| **Compiled form** | Bytecode compiled from the source |

```clojure
;; Internal representation of a definition
{:name        'hello-world/greet
 :type        :function
 :source      ";; Greets a user by name\n(defn greet [name]\n  (str \"Hello, \" name))"
 :provenance  {:origin :file
               :file "hello-world.lona-s"
               :line 4
               :timestamp #inst "2025-12-15T10:00:00"}
 :compiled    <bytecode>}
```

### Comments and Definitions

When parsing a source file, comments immediately preceding a definition are attached to that definition. This means:

- Comments **within** a definition are preserved (part of the source text)
- Comments **before** a definition are attached to it
- Comments between definitions that don't precede anything are dropped on hot-patch

```clojure
;; Original file: hello-world.lona-s
(ns hello-world)

;; Greets a user by name        ← attached to 'greet' definition
(defn greet [name]
  ;; Build greeting string      ← part of 'greet' source text
  (str "Hello, " name))

;; Says farewell                ← attached to 'farewell' definition
(defn farewell [name]
  (str "Goodbye, " name))
```

### Hot-Patching Behavior

When you redefine a function at the REPL:

```clojure
;; Load original file
lona> (load "hello-world")

;; Redefine greet at REPL
lona> (defn hello-world/greet [name]
       (str "Hi there, " name "!"))
```

The definition is replaced:

```clojure
;; New internal state for 'greet'
{:name        'hello-world/greet
 :source      "(defn greet [name]\n  (str \"Hi there, \" name \"!\"))"
 :provenance  {:origin :repl
               :session "session-42"
               :timestamp #inst "2025-12-15T14:30:00"
               :previous {:origin :file
                          :file "hello-world.lona-s"
                          :line 4}}
 :compiled    <new-bytecode>}
```

### Inspecting Source and Provenance

```clojure
;; View current source (shows the REPL version)
lona> (source hello-world/greet)
;; Source: REPL session-42 at 2025-12-15T14:30:00
;; Previously: hello-world.lona-s:4
(defn greet [name]
  (str "Hi there, " name "!"))

;; View source of unmodified function (shows file version)
lona> (source hello-world/farewell)
;; Source: hello-world.lona-s:8
;; Says farewell
(defn farewell [name]
  (str "Goodbye, " name))

;; Get provenance metadata
lona> (provenance hello-world/greet)
{:origin :repl
 :session "session-42"
 :timestamp #inst "2025-12-15T14:30:00"
 :previous {:origin :file :file "hello-world.lona-s" :line 4}}

;; List all definitions in namespace with origins
lona> (ns-definitions 'hello-world)
{greet    {:origin :repl :modified true}
 farewell {:origin :file :file "hello-world.lona-s"}}
```

### Exporting Modified Namespaces

Since source is stored per-definition, you can export the current state of a namespace:

```clojure
;; Export namespace to a new file (includes all current definitions)
lona> (export-ns 'hello-world "hello-world-v2.lona-s")
;; Writes file with current source for all definitions
```

### Late Binding

Hot-patching works because Lonala uses **late binding**: function calls are resolved through a dispatch table at runtime, not compiled to direct jumps.

```
Function call: (greet "Alice")
       │
       ▼
Dispatch table lookup: greet → <bytecode-pointer>
       │
       ▼
Execute bytecode
```

When you redefine `greet`, the dispatch table is updated to point to the new bytecode. All future calls use the new implementation — no recompilation of callers needed.

---

## Code Sharing Across Domains

When a child Domain is spawned, how does it get access to code? Recompiling everything from source would be slow. Sharing everything mutably would break isolation. Lona uses a carefully designed model.

### The Three Components

| Component | Mutability | Sharing Strategy |
|-----------|------------|------------------|
| **Compiled bytecode** | Immutable once compiled | Shared read-only via seL4 page mapping |
| **Source text** | Immutable per-definition | Shared read-only via seL4 page mapping |
| **Dispatch table** | Mutable (late binding) | Private copy per Domain |

### At Domain Spawn Time

```
Parent Domain                          Child Domain (new)
┌─────────────────────────────┐       ┌─────────────────────────────┐
│                             │       │                             │
│  Dispatch Table (mutable)   │       │  Dispatch Table (copy)      │
│  ┌───────────────────────┐  │       │  ┌───────────────────────┐  │
│  │ foo → bytecode-A      │  │ copy  │  │ foo → bytecode-A      │  │
│  │ bar → bytecode-B      │──┼──────►│  │ bar → bytecode-B      │  │
│  └───────────────────────┘  │       │  └───────────────────────┘  │
│            │                │       │            │                │
│            ▼                │       │            ▼                │
│  ┌───────────────────────┐  │       │  ┌───────────────────────┐  │
│  │ Bytecode (read-only)  │◄─┼───────┼──│ Shared mapping (RO)   │  │
│  │ bytecode-A            │  │ share │  │                       │  │
│  │ bytecode-B            │  │       │  │                       │  │
│  └───────────────────────┘  │       │  └───────────────────────┘  │
│                             │       │                             │
│  ┌───────────────────────┐  │       │  ┌───────────────────────┐  │
│  │ Source text (RO)      │◄─┼───────┼──│ Shared mapping (RO)   │  │
│  └───────────────────────┘  │       │  └───────────────────────┘  │
└─────────────────────────────┘       └─────────────────────────────┘
```

**What happens:**
1. Child receives **read-only mapping** of parent's compiled bytecode (same physical pages)
2. Child receives **read-only mapping** of parent's source text (for introspection)
3. Child receives **copy** of parent's dispatch table (symbol → bytecode mappings)

**Why this works:**
- Bytecode is immutable — safe to share read-only
- Source text is immutable per-definition — safe to share read-only
- Dispatch table is where late binding happens — must be private for isolation

### After Parent Hot-Patches

```clojure
;; Parent domain hot-patches foo
(defn foo [] (println "new implementation"))
```

```
Parent Domain                          Child Domain
┌─────────────────────────────┐       ┌─────────────────────────────┐
│                             │       │                             │
│  Dispatch Table             │       │  Dispatch Table             │
│  ┌───────────────────────┐  │       │  ┌───────────────────────┐  │
│  │ foo → bytecode-A' ◄───┼──┼─ NEW  │  │ foo → bytecode-A ◄────┼──┼─ OLD
│  │ bar → bytecode-B      │  │       │  │ bar → bytecode-B      │  │
│  └───────────────────────┘  │       │  └───────────────────────┘  │
│                             │       │                             │
│  Bytecode:                  │       │  Still references:          │
│  - bytecode-A  (old, kept)  │       │  - bytecode-A (old)         │
│  - bytecode-A' (new)        │       │                             │
│  - bytecode-B               │       │                             │
└─────────────────────────────┘       └─────────────────────────────┘

Parent sees: new foo
Child sees:  old foo (isolation preserved)
Old bytecode-A kept alive because child still references it
```

### Key Behaviors

| Event | Parent Domain | Child Domain |
|-------|---------------|--------------|
| Parent hot-patches `foo` | Sees new `foo` | Still sees old `foo` |
| Child hot-patches `bar` | Sees old `bar` | Sees new `bar` |
| New grandchild spawned from child | — | Grandchild gets child's current state |

### Explicit Code Propagation

Updates don't propagate automatically. This is intentional — isolation by default:

```clojure
;; Parent can push code updates to child (if it has capability)
(push-code child-domain 'foo)

;; Child can pull updates from parent (if it has capability)
(pull-code parent-domain 'foo)

;; Child can choose to accept or reject
(on-code-push [fn-name new-source]
  (if (validate-update fn-name new-source)
    (accept-update fn-name new-source)
    (reject-update fn-name)))
```

### Startup Efficiency

This model enables fast domain spawning:

```
First boot of Lona:
  └── Root domain parses & compiles SDK (slow, ~10-30 seconds)
  └── Bytecode stored in memory (read-only pages)

Spawning child domain:
  └── Map parent's bytecode pages read-only (instant, ~microseconds)
  └── Copy dispatch table (~microseconds)
  └── Child is ready (no reparse, no recompile)
```

### Closures Across Domain Boundaries

Closures capture values from their environment. When a closure crosses a domain boundary:

```clojure
;; In parent domain
(def config {:timeout 5000})

(defn make-handler []
  (fn [request]
    ;; Closure captures 'config'
    (process-with-timeout request (:timeout config))))
```

**Rules for closures:**
- **Immutable values**: Shared read-only (safe, since Lonala values are immutable)
- **Mutable state** (atoms, refs): Must be explicitly passed or not allowed to cross domain boundaries

```clojure
;; Safe: immutable data is shared
(def config {:timeout 5000})  ; immutable, can share across domains

;; Not allowed implicitly: mutable state
(def counter (atom 0))  ; cannot be captured in cross-domain closure
;; Must explicitly pass as capability or copy initial value
```

### Summary

| Question | Answer |
|----------|--------|
| Does child reparse source? | No — shares parent's compiled bytecode read-only |
| Does parent's patch affect child? | No — each domain has private dispatch table |
| How to propagate updates? | Explicit `push-code` / `pull-code` with capability |
| What about closures? | Immutable values shared; mutable state requires explicit handling |
| Startup cost for new domain? | Minimal — map pages + copy dispatch table |

**Principle: Isolation by default, explicit sharing when needed.**

---

## Concurrency Model

### Processes

Lona Processes are not OS threads. They are:

- **Extremely lightweight** — target: hundreds of bytes initial overhead
- **Massively scalable** — support millions of concurrent Processes
- **Preemptively scheduled** — using reduction counting, no Process can monopolize the CPU
- **Garbage collected independently** — one Process's GC pause doesn't affect others

### Isolation Levels

| Boundary | Enforcement | What It Provides |
|----------|-------------|------------------|
| Between Domains | seL4 kernel (hardware) | Memory isolation, capability separation, CPU time |
| Between Processes (same Domain) | Lonala runtime | Separate heaps, failure containment, message queues |

Processes in the same Domain have separate heaps and communicate via messages, providing logical isolation. However, they share the Domain's capabilities and memory space — a Domain is a single trust zone. For true security isolation, use separate Domains.

**Important**: Fault isolation within a Domain assumes well-behaved managed Lonala code. Code that uses inline assembly, direct memory operations, or hardware access can bypass runtime protections and potentially corrupt other processes in the same Domain. This is an intentional tradeoff — Lona provides the power of systems programming, but with that power comes responsibility. For code you don't fully trust, use a separate Domain.

### Inter-Process Communication

All Processes communicate via message passing:

```clojure
;; Send a message
(send pid {:request :read-block :block-id 42})

;; Receive with pattern matching
(receive
  {:response :ok :data data}    (process-data data)
  {:response :error :reason r}  (handle-error r)
  (after 5000                   (timeout-handler)))

;; Synchronous call (sends and waits for reply)
(call pid {:request :status})  ; => returns response
```

The API is identical whether the target Process is in the same Domain or a different one. The runtime handles routing:

- **Same Domain**: Direct memory copy or reference passing for immutable data
- **Different Domain**: seL4 IPC with automatic capability transfer

### Supervision Trees

Following OTP conventions, Processes are organized into supervision hierarchies:

```
                    [System Supervisor]
                           /          \
              [Driver Supervisor]    [App Supervisor]
                /        \              /          \
    [uart-driver]  [net-driver]  [tcp-stack]  [telnet-server]
      (Domain)       (Domain)      (Domain)       (Domain)
```

Supervisors can manage Processes across Domain boundaries. When a child fails:

- **Restart** the failed Process (and its Domain if isolated)
- **Restart** all sibling Processes
- **Escalate** the failure to the parent supervisor

**Supervision across Domain boundaries**: From a supervisor's perspective, child processes are managed uniformly regardless of which Domain they inhabit. When a child process in a sub-domain crashes, the supervisor receives the same notification as for same-domain children and applies the same restart strategy. The Domain boundary affects *security isolation*, not *supervision semantics*.

**Value proposition:** Build systems that self-heal. Individual component failures are automatically handled without human intervention.

---

## Memory Model and Zero-Copy

### The Challenge

With strong isolation between Domains, naive message passing would require copying all data across boundaries. For high-throughput scenarios like networking, this is unacceptable:

```
net-driver → tcp-stack → telnet-server
           copy       copy
```

### Shared Memory Regions

Lona solves this with **capability-controlled shared memory**:

```clojure
;; Create a shared memory region
(def packet-buffer (create-shared-region (megabytes 16)))

;; Grant capabilities to specific Domains
(grant-capability net-driver-domain packet-buffer :read-write)
(grant-capability tcp-stack-domain packet-buffer :read-only)
```

Multiple Domains can map the same physical memory. Access is controlled by capabilities:
- **Write capability**: Can modify the region
- **Read-only capability**: Can only read

### Zero-Copy Networking Example

```
┌─────────────────────────────────────────────────────────────┐
│                Raw Packet Buffer (16 MB)                    │
│  ┌────────────────────────────────────────────────────────┐ │
│  │ [pkt1][pkt2][pkt3][...][pktN]                          │ │
│  └────────────────────────────────────────────────────────┘ │
│       ▲ write              ▲ read-only                      │
│       │                    │                                │
│  Domain: net-driver   Domain: tcp-stack                     │
└─────────────────────────────────────────────────────────────┘
```

```clojure
;; In net-driver: DMA writes packet, send reference
(let [pkt-ref (region-ref packet-buffer offset length)]
  (send tcp-process {:packet pkt-ref}))  ; tiny message (~24 bytes)

;; In tcp-stack: zero-copy read
(receive
  {:packet pkt-ref}
  (let [header (read-ref pkt-ref 0 40)]  ; reads directly from shared memory
    (process-packet header pkt-ref)))
```

### Security Isolation with Per-Connection Buffers

For application-level isolation (e.g., telnet vs SSH), the TCP stack acts as a trusted demultiplexer:

```
Raw Buffer (net-driver + tcp-stack only)
        │
        ▼
   TCP Stack demultiplexes
        │
   ┌────┴────┬────────────┐
   ▼         ▼            ▼
Buffer A   Buffer B    Buffer C
(telnet)   (ssh)       (http)
```

Each application only has capability to its own connection buffers:

```clojure
;; TCP stack creates per-connection buffer on accept
(defn accept-connection [listening-socket]
  (let [conn-buffer (create-shared-region (kilobytes 64))
        app-domain (lookup-app-for-port (conn :local-port))]
    ;; Only this specific app gets access
    (grant-capability app-domain conn-buffer :read-only)
    conn-buffer))
```

Telnet cannot read SSH packets — it doesn't have the capability.

### Summary of Data Transfer Costs

| Scenario | Mechanism | Copy Cost |
|----------|-----------|-----------|
| Small messages (< 1KB) | seL4 IPC inline | 1 copy (acceptable) |
| Large immutable values | Shared region + read-only cap | Zero copy |
| Stream data (network, disk) | Shared ring buffer | Zero copy |
| Security demux boundary | Per-connection buffers | 1 copy (necessary) |

---

## Security Model

### The Domain as the Only Security Boundary

Lona has a simple, principled security model: **Domains are the only security boundary**. We do not attempt to enforce restrictions that cannot be reliably enforced.

**Within a Domain:**
- All Processes share memory and capabilities
- Full LISP-style access to everything
- No artificial restrictions — it's one trust zone
- If you don't trust code, don't run it in your Domain

**Between Domains:**
- Complete isolation enforced by seL4 (hardware-level)
- Communication only via message passing (seL4 IPC)
- Resource access only via explicit capabilities
- No bypass possible — kernel-enforced

This clarity is intentional. We don't pretend to offer security guarantees we can't deliver. A Domain is either trusted (runs in your Domain) or untrusted (runs in its own isolated Domain).

### Capability-Based Security

Every resource in Lona is protected by capabilities:

```clojure
;; A driver receives only the capabilities it needs
(defn start-uart-driver [uart-cap irq-cap]
  ;; uart-cap: capability to access UART hardware
  ;; irq-cap: capability to handle UART interrupts
  ;; This Domain CANNOT access network, disk, or other hardware
  ...)
```

Capabilities are:
- **Unforgeable** — created only by the kernel
- **Delegable** — can be passed to child Domains with equal or reduced rights
- **Revocable** — can be invalidated by the granting authority
- **Attenuable** — can be weakened (e.g., read-write → read-only)

### Principle of Least Privilege

The Lona architecture enforces minimal capability grants:

1. The root Domain receives all capabilities at boot
2. It creates child Domains with subsets of capabilities
3. Each child can further subdivide to its children
4. Leaf Domains have exactly the capabilities they need — no more

```clojure
;; Example: downloading and running untrusted code
(spawn untrusted-code/main [downloaded-data]
       {:domain (str "sandbox:" (unique-id))
        :capabilities []                    ; no hardware access
        :memory-limit (megabytes 32)
        :can-spawn-domains false
        :meta {:type :sandbox
               :source "https://example.com/app.lona-p"
               :downloaded-at (now)}})

;; The untrusted code:
;; - Cannot access any hardware
;; - Cannot access other Domains' memory
;; - Cannot create child Domains
;; - Has limited memory
;; - Can be killed at any time by supervisor
;; - Can be found via: (find-domains {:type :sandbox})
```

### Security Boundaries

```
┌─────────────────────────────────────────────────────────────────┐
│                        seL4 Kernel                              │
│                    (Verified, Trusted)                          │
├───────────────────┬──────────────────┬──────────────────────────┤
│    Domain A       │    Domain B      │    Domain C              │
│  ┌─────────────┐  │  ┌────────────┐  │  ┌────────────────────┐  │
│  │ Process 1   │  │  │ Process 3  │  │  │ Process 5          │  │
│  │ Process 2   │  │  │ Process 4  │  │  │ Process 6          │  │
│  └─────────────┘  │  └────────────┘  │  └────────────────────┘  │
│   One trust zone  │   One trust zone │    One trust zone        │
│  caps: uart, irq  │  caps: nic, irq  │  caps: (ipc endpoints)   │
└───────────────────┴──────────────────┴──────────────────────────┘
         ▲                   ▲                    ▲
         │                   │                    │
         └───────────────────┴────────────────────┘
              seL4 IPC (the ONLY boundary)
```

- **Between Domains**: The real security boundary — kernel-enforced via VSpace/CSpace
- **Within a Domain**: One trust zone — Processes can access everything in their Domain

**Value proposition:** Clear security model. Domains are isolated by verified kernel code. Within a Domain, full LISP-style power. No false promises about restrictions we can't enforce.

---

## Runtime Introspection and Debugging

Lona provides LISP-machine-style debugging capabilities: the ability to inspect, modify, and control any aspect of the running system. This is made possible by source-only distribution (source is always available) and the dynamic nature of Lonala.

### Debugging Model: Domains as Trust Zones

**Within a Domain**: Full, unrestricted access. A Domain is a single trust zone — all Processes within it share the same memory space and capabilities. In the LISP tradition, everything is inspectable and modifiable. There are no artificial restrictions on what code within a Domain can do to itself.

**Across Domains**: Requires capabilities. To debug a different Domain (inspect its memory, set breakpoints, read its source), you need a capability granting that access. This is enforced by seL4 at the kernel level.

```
┌─────────────────────────────────────────────────────────────────┐
│ Domain: "user:tobias"                                           │
│                                                                 │
│  Full access within:                                            │
│  - Inspect any value                                            │
│  - Set breakpoints anywhere                                     │
│  - Modify any function                                          │
│  - Read all source code                                         │
│  - Trace any process                                            │
│                                                                 │
│  This IS the trust boundary.                                    │
│  LISP philosophy: everything goes within your domain.           │
└─────────────────────────────────────────────────────────────────┘
         │
         │ To debug another domain, need capability
         │ (enforced by seL4, not by convention)
         ▼
┌─────────────────────────────────────────────────────────────────┐
│ Domain: "driver:net"                                            │
│                                                                 │
│  Cannot access without :debug-domain capability                 │
└─────────────────────────────────────────────────────────────────┘
```

### The REPL

The Read-Eval-Print Loop is the primary interface to a Lona system:

```clojure
lona> (list-processes)
#{{:pid 1 :name init :domain "root" :status :waiting}
  {:pid 2 :name uart-main :domain "driver:uart" :status :waiting}
  {:pid 3 :name repl :domain "user:tobias" :status :running}
  {:pid 4 :name rx-handler :domain "driver:net" :status :waiting}
  ...}

lona> (list-domains)
#{{:name "root" :parent nil :processes 1 :children 3}
  {:name "drivers" :parent "root" :processes 1 :children 3}
  {:name "driver:uart" :parent "drivers" :processes 1 :children 0}
  {:name "driver:net" :parent "drivers" :processes 3 :children 0}
  {:name "services" :parent "root" :processes 1 :children 2}
  {:name "user:tobias" :parent "telnet" :processes 2 :children 0}
  ...}

lona> (domain-info "user:tobias")
{:name "user:tobias"
 :parent "telnet"
 :capabilities #{:user-ipc :fs-home-tobias}
 :memory-used 2458624
 :memory-limit 67108864
 :processes [{:pid 3 :name repl} {:pid 47 :name user-task}]
 :meta {:type :user-session :user "tobias" :started #inst "2025-..."}}

lona> (find-domains {:type :user-session})
["user:tobias" "user:alice" "user:guest"]

lona> (process-info 4)
{:pid 4
 :name rx-handler
 :domain "driver:net"
 :function net-driver/rx-loop
 :heap-size 8192
 :message-queue-len 3}

lona> (source uart/transmit)
(defn transmit [port byte]
  (mem-write (+ (port :base-addr) TX-REG) byte)
  (wait-for-irq (port :tx-irq)))
```

### The Condition/Restart System

Inspired by Common Lisp, Lonala provides a condition system that separates error detection from error handling. Unlike traditional exceptions that immediately unwind the stack, conditions allow inspection and recovery:

```clojure
;; Low-level code signals conditions and provides restarts
(defn read-config [path]
  (restart-case
    (if (file-exists? path)
      (parse-config (slurp path))
      (signal :file-not-found {:path path}))

    ;; Restarts: ways to recover from this situation
    (:retry []
      "Try reading the file again"
      (read-config path))

    (:use-default []
      "Use default configuration"
      default-config)

    (:use-value [config]
      "Provide a configuration value"
      config)))

;; High-level code decides how to handle conditions
(handler-bind
  [:file-not-found
   (fn [condition]
     (if (= (:path condition) "/etc/critical.conf")
       (invoke-restart :use-default)
       (invoke-restart :retry)))]

  (start-application))
```

**Key insight**: When an error occurs, the stack is **not unwound** until you decide how to handle it. From the debugger, you can:
- Inspect the full call stack
- Examine all variables at each frame
- Choose from available restarts
- Provide values interactively
- Then continue execution

```clojure
;; Interactive debugging session
lona> (start-server)

;; ERROR: :file-not-found at read-config
;;
;; Condition: {:type :file-not-found :path "/etc/server.conf"}
;;
;; Available restarts:
;;   0: [:retry] Try reading the file again
;;   1: [:use-default] Use default configuration
;;   2: [:use-value] Provide a configuration value
;;   3: [:abort] Abort to REPL
;;
;; Stack:
;;   0: (read-config "/etc/server.conf")
;;   1: (load-settings)
;;   2: (start-server)

lona:debug> (frame 0)
;; Frame 0: read-config
;; Locals: {path "/etc/server.conf"}

lona:debug> (restart 1)  ; use-default
;; Server started with default configuration
```

### Stack Frame Introspection

Full access to the call stack, including local variables:

```clojure
;; Get current stack
(current-stack-frames)
; => [{:function foo :args [1 2] :locals {:x 10 :y 20} :line 42}
;     {:function bar :args [:init] :locals {:state {...}} :line 15}
;     ...]

;; Inspect a specific frame
(frame-locals 0)     ; => {:x 10, :y 20}
(frame-source 0)     ; => shows source code at that point

;; Modify and continue (within your domain)
(set-frame-local! 0 :x 42)
(continue)
```

### Process State Inspection

BEAM-style process introspection:

```clojure
;; Get the internal state of a process
(process-state pid)

;; Get the message queue
(process-messages pid)
; => [{:from pid-23 :msg {:request :status}}
;     {:from pid-47 :msg {:data [1 2 3]}}]

;; Get full process info
(process-info pid)
; => {:pid 42
;     :name worker-3
;     :domain "services"
;     :status :waiting
;     :current-function worker/loop
;     :heap-size 8192
;     :reductions 145023
;     :message-queue-len 2}
```

### Tracing

Non-blocking observation of system behavior (inspired by Erlang's tracing):

```clojure
;; Trace function calls
(trace-calls 'tcp/handle-packet
             {:args true :return true :timestamps true})
; >> [12:34:56.789] tcp/handle-packet called with: [{:src ...}]
; << [12:34:56.791] tcp/handle-packet returned: :ok

;; Trace message passing
(trace-messages pid {:send true :receive true})
; >> [12:34:57.001] pid-42 <- {:request :data} from pid-10
; << [12:34:57.003] pid-42 -> {:response :ok} to pid-10

;; Trace across domain boundaries (requires capability)
(trace-domain-ipc "driver:net" "tcp-stack")
```

### Hot Code Loading

Modify the system without stopping it:

```clojure
;; Current implementation has a bug
lona> (source net/checksum)
(defn checksum [data]
  (reduce + data))  ; Bug: doesn't handle overflow

;; Fix it live
lona> (defn net/checksum [data]
        (reduce #(bit-and (+ %1 %2) 0xFFFF) 0 data))
#'net/checksum

;; All future calls use the new implementation
;; Existing connections continue without interruption
```

Because Lonala uses late binding (function calls resolved at runtime), redefining a function immediately affects all callers — no recompilation of dependents needed.

### Cross-Domain Debugging

To debug a domain other than your own, you need the appropriate capability:

```clojure
;; Normal user cannot debug drivers
lona[user:tobias]> (debug-attach "driver:net")
;; Error: No capability to debug domain "driver:net"

;; Admin user with :debug-all capability
lona[admin]> (debug-attach "driver:net")
;; Attached to domain "driver:net"

lona[admin]> (trace-calls 'net-driver/rx-loop)
;; Now tracing in driver:net domain...

lona[admin]> (list-processes "driver:net")
#{{:pid 47 :name rx-handler :status :waiting}
  {:pid 48 :name tx-handler :status :running}}
```

### Summary of Debugging Capabilities

| Capability | Within Domain | Cross Domain |
|------------|---------------|--------------|
| Inspect values | Always | Requires capability |
| Set breakpoints | Always | Requires capability |
| Modify functions | Always | Requires capability |
| Read source | Always | Requires capability |
| Trace calls | Always | Requires capability |
| Inspect processes | Always | Requires capability |
| Modify process state | Always | Requires capability |

**Value proposition:** Debug production systems in place. Fix bugs without downtime. Understand exactly what your system is doing at any moment. Full LISP-machine-style power within your trust zone, with capability-controlled access across trust boundaries.

---

## Target Platforms

Lona aims to run on diverse hardware:

| Platform | Architecture | Use Case |
|----------|--------------|----------|
| **QEMU** | ARM64, x86_64 | Development and testing |
| **Raspberry Pi 4** | ARM64 | Embedded systems, education, hobbyist |
| **AWS Graviton** | ARM64 | Cloud server deployment |
| **x86_64 servers** | x86_64 | Traditional server infrastructure |
| **x86_64 desktop** | x86_64 | Future desktop/workstation use |

### Platform Abstraction

The Lona runtime provides platform abstraction:

```clojure
;; Platform-independent driver interface
(defprotocol BlockDevice
  (read-block [dev block-id])
  (write-block [dev block-id data])
  (block-count [dev]))

;; Platform-specific implementations
(def virtio-blk (make-virtio-block-device virtio-cap))
(def sd-card (make-sd-device sd-cap))

;; Application code works with either
(read-block device 0)
```

**Value proposition:** Write once, run on embedded devices, cloud servers, and everything in between.

---

## Initial System Components

The first release targets a minimal but functional networked system:

### Phase 1: Core System

1. **Lonala Runtime** — process scheduler, garbage collector, memory manager
2. **UART Driver** — serial console for initial interaction (in own Domain)
3. **REPL** — interactive Lonala environment over UART
4. **Boot Supervisor** — initial process and Domain hierarchy

### Phase 2: Storage & Networking

5. **VirtIO Block Driver** — virtualized storage access (in own Domain)
6. **VirtIO Network Driver** — virtualized network access (in own Domain)
7. **TCP/IP Stack** — basic networking: IP, TCP, UDP (in own Domain)
8. **Telnet Server** — networked REPL access (in own Domain)

### Phase 3: Dynamic Loading

9. **Module Loader** — load Lonala code from storage
10. **Network Loader** — download and run Lonala applications
11. **Package System** — dependency management

### Bootstrap Sequence

```
1. seL4 kernel boots, starts initial task (root Domain)
2. Root task starts Lonala runtime
3. Runtime spawns UART driver in isolated Domain
4. Runtime starts REPL Process (connected to UART)
5. User loads network driver via REPL → new Domain
6. User starts TCP/IP stack → new Domain
7. User starts Telnet server → new Domain
8. User connects via Telnet for remote access
9. User loads additional applications as needed
```

**Value proposition:** A working system with minimal components, extensible at runtime through the REPL.

---

## Non-Goals

Lona explicitly does not aim for:

### Formal Verification of Userland

While seL4's kernel is formally verified, Lona's userland is not. We benefit from seL4's verified isolation guarantees, but application code relies on:
- Testing
- The fault-tolerance of supervision trees
- Runtime monitoring and debugging
- Domain isolation limiting blast radius

### Hard Real-Time Guarantees

Lona uses garbage collection for memory management. While we aim for low-latency GC (per-process collection, incremental algorithms), we do not guarantee deterministic response times. Applications requiring hard real-time should use specialized systems.

### POSIX Compatibility

Lona is not a UNIX clone. We do not aim to run existing UNIX applications. The system is designed for native Lonala applications that embrace its concurrency and capability models.

### Foreign Language Support

Lonala is the sole programming language. There is no C FFI, no support for third-party C libraries, and no polyglot runtime. This constraint ensures:
- Complete introspection of all running code
- Uniform debugging and hot-patching
- No escape hatches that bypass the capability model

---

## Who Is Lona For?

### Primary Audience

- **Systems developers** who want to understand and modify every layer of their stack
- **Distributed systems engineers** who need Erlang-style fault tolerance with lower-level control
- **Security-conscious developers** who want capability-based isolation without giving up productivity
- **Educators and researchers** exploring operating system design

### Use Cases

- **Network appliances** — routers, firewalls, load balancers
- **Embedded systems** — IoT devices, industrial controllers
- **Cloud infrastructure** — specialized server workloads
- **Development platforms** — environments for learning OS concepts

---

## Summary

Lona combines:

| Component | Inspiration | Contribution |
|-----------|-------------|--------------|
| **seL4** | L4 microkernel family | Verified security, capability model |
| **LISP machines** | Symbolics, MIT CADR | Runtime introspection, hot-patching |
| **BEAM/OTP** | Erlang, Elixir | Lightweight processes, fault tolerance |
| **Clojure** | Rich Hickey's work | Immutability, syntax, data-centric design |

### Core Abstractions

| Abstraction | Description |
|-------------|-------------|
| **Process** | Lightweight unit of execution; the unified concurrency primitive |
| **Domain** | Security/memory isolation boundary; the single trust zone |
| **Capability** | Unforgeable token granting access to resources |
| **Dispatch Table** | Per-domain symbol→bytecode mapping; enables late binding and hot-patching |
| **Shared Region** | Zero-copy memory sharing with capability-controlled access |
| **Condition/Restart** | Error handling without stack unwinding; interactive recovery |

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Source-only distribution** | Total transparency; source always available for debugging |
| **No ahead-of-time compilation** | Compiler is part of Lona; simplifies toolchain and deployment |
| **Per-definition source storage** | Enables hot-patching with proper provenance tracking |
| **Late binding via dispatch tables** | Function redefinition affects all callers immediately |
| **Isolation by default for code** | Parent's patches don't affect children; explicit propagation when needed |
| **Domain = only security boundary** | Clear model; no false promises about restrictions we can't enforce |
| **Full introspection within Domain** | LISP philosophy; everything goes within your trust zone |
| **Capability-controlled cross-domain access** | seL4-enforced; the real security mechanism |

### The Result

An operating system where:

- **Security** is enforced by hardware and capabilities, not conventions
- **Concurrency** is natural and scalable to millions of Processes
- **Isolation** is hierarchical — Domains contain Processes, parent Domains control children
- **Failures** are contained and automatically recovered
- **Development** happens in a live, inspectable environment
- **Source** is always available — no "missing symbols", no opaque binaries
- **Debugging** is a first-class capability with full LISP-machine-style power
- **Understanding** is never blocked — you can always look deeper

Lona is the operating system for developers who refuse to accept "you can't see that" or "you can't change that" as answers.

---

## References and Inspirations

- **seL4**: [seL4 Foundation](https://sel4.systems/)
- **L4 Microkernel Family**: [Wikipedia](https://en.wikipedia.org/wiki/L4_microkernel_family)
- **LISP Machines**: Historical systems from MIT, Symbolics, and Lisp Machines Inc.
- **Erlang/OTP**: [Erlang.org](https://www.erlang.org/)
- **Clojure**: [Clojure.org](https://clojure.org/)
- **Microkernel Design**: [OSDev Wiki](http://wiki.osdev.org/Microkernel)
