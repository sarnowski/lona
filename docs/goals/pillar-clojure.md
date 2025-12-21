# Pillar IV: Clojure — The Data

> *"Data is Ultimate"*

## Why Clojure?

Clojure is often described as "a LISP with great syntax." This undersells its contribution. Clojure's real innovation is **data-centric design**:

1. **Immutable by default**: Data structures cannot be modified after creation
2. **Persistent data structures**: "Updates" create new versions sharing structure with old
3. **Data as interface**: Systems communicate through plain data (maps, vectors), not opaque objects
4. **Rich literals**: Complex data can be written and read without serialization code

For an operating system, these properties are transformative:

- **Immutability enables safe sharing**: Pass a map across a domain boundary; the receiver knows it won't change
- **Data as interface enables inspection**: Every message, every configuration, every state is readable data
- **Persistent structures enable efficiency**: Structural sharing means "copying" is cheap

Lona adopts Clojure's philosophy completely. Lonala is not just "LISP syntax for systems programming"—it's Clojure's data philosophy applied to operating systems.

---

## Philosophy: Data is Ultimate

Traditional systems programming uses opaque structures:

```c
struct packet {
    uint32_t flags;
    uint16_t length;
    char data[MAX_PACKET];
};
```

The structure is:
- Mutable (anyone with a pointer can change it)
- Opaque (fields require documentation to understand)
- Unsafe to share (data races, use-after-free)

Clojure philosophy inverts this:

```clojure
{:type :packet
 :flags #{:syn :ack}
 :length 1420
 :data <binary>}
```

The data is:
- **Immutable**: Cannot be changed after creation
- **Self-describing**: Keywords name the fields
- **Safe to share**: Immutability guarantees consistency
- **Printable/readable**: Can be logged, transmitted, stored

---

## What Clojure Philosophy Forces in Lona

### 1. Immutable Persistent Data Structures

All core data types in Lonala are immutable:

| Type | Description | Access |
|------|-------------|--------|
| **Vector** | Indexed collection | O(log32 n) |
| **Map** | Key-value associations | O(log32 n) |
| **Set** | Unique elements | O(log32 n) |
| **List** | Linked list | O(1) first, O(n) nth |

"Updating" a data structure returns a new structure:

```clojure
(def v1 [1 2 3])
(def v2 (conj v1 4))

v1  ; => [1 2 3]  (unchanged)
v2  ; => [1 2 3 4]
```

The new structure **shares structure** with the old. This is not copying—it's structural sharing via tree nodes. "Copying" a million-element vector and changing one element creates only ~6 new nodes (log32 of a million).

### 2. Data as the Interface

Processes communicate through plain data:

```clojure
;; Request
(send server {:type :query
              :table :users
              :where {:active true}})

;; Response
(receive
  {:type :result :rows rows}
    (process-users rows)
  {:type :error :reason reason}
    (handle-error reason))
```

This is not serialization—the data **is** the message. Benefits:

| Benefit | Explanation |
|---------|-------------|
| **Inspectable** | Any message can be logged, traced, debugged |
| **Versionable** | Add fields without breaking receivers |
| **Testable** | Construct test messages as literal data |
| **Universal** | Same format for IPC, config, storage |

### 3. Homoiconicity

Code is data. Data is code. This enables:

**Macros**: Transform code at compile time

```clojure
;; defn is a macro that transforms to fn + def
(defn square [x] (* x x))

;; expands to:
(def square (fn [x] (* x x)))
```

**Introspection**: Examine code structure programmatically

```clojure
(source square)
;; => (defn square [x] (* x x))

;; The source is a data structure you can manipulate
(first (source square))  ; => defn
```

**REPL power**: The REPL reads data, evaluates it as code, prints the result

### 4. Rich Literal Syntax

Complex data can be written directly:

```clojure
;; Configuration as data
{:server {:host "0.0.0.0"
          :port 8080
          :ssl {:enabled true
                :cert-path "/etc/certs/server.pem"}}
 :database {:driver :postgres
            :pool-size 10}
 :features #{:auth :logging :metrics}}

;; No parsing code needed—this IS the configuration
```

---

## Immutability Enables Safe Sharing

This is the critical insight for operating systems.

### The Problem

In traditional systems, sharing data across boundaries is dangerous:

```
Process A                    Process B
    │                            │
    └──── pointer to data ───────┘
              │
              ▼
         ┌─────────┐
         │ mutable │ ← Data race! Who owns this?
         │  data   │
         └─────────┘
```

Solutions (all have costs):
- **Copy everything**: Expensive
- **Locks**: Deadlocks, priority inversion
- **Ownership types**: Complexity

### The Clojure Solution

With immutable data, sharing is safe:

```
Process A                    Process B
    │                            │
    └──── reference to data ─────┘
              │
              ▼
         ┌───────────┐
         │ immutable │ ← Safe! Neither can change it
         │   data    │
         └───────────┘
```

When Process A "sends" an immutable map to Process B:
- No copy needed (within same Domain)
- No lock needed (data can't change)
- No race possible (immutability guarantee)

### Cross-Domain Sharing

Even across Domain boundaries (which have separate memory), immutability helps:

```
Domain A                     Domain B
    │                            │
    └──── shared memory ─────────┘
              │
              ▼
         ┌───────────┐
         │ immutable │ ← Safe read-only sharing
         │   data    │
         └───────────┘
```

Domain A can grant read-only access to a memory region containing immutable data. Domain B can read it safely—there's no way for A to mutate it underneath B.

This enables **zero-copy networking**:

```clojure
;; Network driver writes packet to shared buffer
;; TCP stack reads directly from buffer (no copy)
;; Safe because the packet data is immutable
```

---

## The Lonala Language

Lonala is Clojure for systems programming. It inherits:

### From Clojure

| Feature | Purpose |
|---------|---------|
| S-expression syntax | Code as data, macros |
| Immutable collections | Safe sharing, concurrency |
| Rich literals | Vectors, maps, sets, keywords |
| Sequence abstraction | Uniform collection operations |
| Destructuring | Elegant pattern matching |

### Added for Systems Programming

| Feature | Purpose |
|---------|---------|
| Binary type | Mutable byte buffers for DMA |
| Hardware primitives | MMIO, interrupts |
| Process primitives | spawn, send, receive |
| Capability operations | Domain management |

### Notably Different from Clojure

| Feature | Clojure | Lonala |
|---------|---------|--------|
| Runtime | JVM | seL4 + custom |
| Concurrency | STM + atoms | Processes + messages |
| Error handling | Exceptions | Result tuples + conditions |
| Interop | Java | None (pure Lonala) |

---

## Data-Centric System Design

Clojure philosophy influences how Lona systems are structured:

### Messages are Data

```clojure
;; Not: object.method(args)
;; But: (send process {:operation :method :args args})

(send db-process {:type :query
                  :sql "SELECT * FROM users"
                  :params []})
```

### Configuration is Data

```clojure
;; Not: config.setPort(8080)
;; But: data structure read at startup

(def config
  {:server {:port 8080}
   :workers 4})

(spawn-workers (:workers config))
```

### State is Data

```clojure
;; Process state is a single immutable value
(defn server-loop [state]
  (receive
    {:type :get :from pid}
      (do (send pid {:value (:value state)})
          (recur state))
    {:type :set :value v}
      (recur (assoc state :value v))))
```

### Protocols are Data Schemas

```clojure
;; The "protocol" between client and server is just:
;; - what keywords appear in messages
;; - what types the values have
;; No interface definitions, no IDL, no code generation
```

---

## The Binary Escape Hatch

Pure immutability doesn't work for device drivers. DMA buffers must be mutable. Network packets are written in place.

Lonala provides exactly one mutable type: **Binary**.

```clojure
(def buf (make-binary 1024))  ; Mutable byte buffer
(binary-set buf 0 0xFF)       ; Write byte
(binary-get buf 0)            ; Read byte
```

Binary has an ownership model:
- **Owned**: Can read and write
- **View**: Can only read

This is the intentional escape hatch for systems programming. Everything else is immutable.

---

## Implications for Lona Design

Clojure philosophy shapes these Lona design decisions:

| Decision | Driven By |
|----------|-----------|
| Immutable collections | Safe sharing, zero-copy |
| Data as messages | Inspectable, versionable IPC |
| Rich literals | Configuration without parsing |
| Homoiconicity | REPL, macros, introspection |
| Single mutable type (Binary) | Escape hatch for hardware |

---

## Summary

Clojure philosophy provides Lona with:

| Guarantee | Mechanism |
|-----------|-----------|
| **Safe sharing** | Immutable data structures |
| **Zero-copy IPC** | Immutability + capability-controlled regions |
| **Inspectable messages** | Data is the interface |
| **Powerful metaprogramming** | Homoiconicity |
| **Concurrency safety** | No shared mutable state |

**The Bottom Line**: In Lona, data is not hidden inside objects. Data is visible, immutable, shareable, and inspectable. Messages between processes are plain data. Configuration is plain data. State is plain data. This transparency—enabled by immutability—makes systems understandable.

---

## Further Reading

- [Core Concepts: Message](core-concepts.md#message)
- [System Design: Zero-Copy Memory](system-design.md#zero-copy-memory-model)
- [Language Specification: Data Types](/docs/lonala/data-types.md)
- [Clojure Rationale](https://clojure.org/about/rationale)
- [Simple Made Easy](https://www.infoq.com/presentations/Simple-Made-Easy/) — Rich Hickey
