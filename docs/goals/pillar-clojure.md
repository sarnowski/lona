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

## Immutability and the BEAM Memory Model

### The Design Choice

Lona adopts **pure BEAM semantics** for message passing: all immutable values are **deep-copied** on send. This might seem to contradict the "safe sharing" promise of immutability, but it's the right choice:

| Property | Benefit |
|----------|---------|
| **Per-process heap isolation** | Each process GCs independently |
| **Instant memory reclaim** | Dead process heap freed immediately |
| **No cross-process references** | Simpler, more predictable GC |
| **Crash isolation** | Process crash can't corrupt other heaps |

### Why Not Share References?

While immutable data is technically safe to share, doing so creates problems:

```
Process A                    Process B
    │                            │
    └──── reference to data ─────┘
              │
              ▼
         ┌───────────┐
         │ immutable │ ← Safe from mutation...
         │   data    │   but heaps are now coupled!
         └───────────┘
```

If processes share references:
- GC must trace across process boundaries
- Process A's death doesn't free memory B still references
- "Instant heap reclaim" becomes impossible
- GC complexity increases dramatically

### The Binary Escape Hatch

For **large data** (network packets, file contents), copying is too expensive. The **Binary** type is Clojure's (and BEAM's) solution:

```clojure
;; Binary is explicitly shared, not copied
(def packet-data (binary-create 1500))

;; Send to another process - shares reference, not bytes
(send tcp-handler {:packet packet-data})

;; Receiver gets read-only View
;; Underlying bytes shared via reference counting
```

This gives us the best of both worlds:
- Regular data: deep copy (simple GC, crash isolation)
- Large data: explicit sharing via Binary (efficiency)

### Cross-Domain Sharing

Across Domain boundaries (separate address spaces), Binary sharing uses capability-controlled shared memory:

```
Domain A                     Domain B
    │                            │
    └──── shared memory ─────────┘
              │
              ▼
         ┌───────────┐
         │  Binary   │ ← Capability-controlled access
         │  bytes    │   A: read-write, B: read-only
         └───────────┘
```

This enables **zero-copy networking** for large payloads:

```clojure
;; Network driver writes packet to shared Binary region
;; TCP stack reads directly from shared region (no copy)
;; Only possible via explicit Binary type
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
| **Immutable values** | Core data structures can't be modified |
| **Per-process isolation** | Deep copy on send (BEAM semantics) |
| **Zero-copy for large data** | Binary type as explicit escape hatch |
| **Inspectable messages** | Data is the interface |
| **Powerful metaprogramming** | Homoiconicity |
| **Concurrency safety** | No shared mutable state |

**The Bottom Line**: In Lona, data is not hidden inside objects. Data is visible, immutable, and inspectable. Messages are plain data that gets deep-copied on send—ensuring process isolation. For large data, Binary provides an explicit escape hatch with reference sharing. This combination of immutability + BEAM copy semantics makes systems both understandable and robust.

---

## Further Reading

- [Core Concepts: Message](core-concepts.md#message)
- [System Design: Memory Model](system-design.md#memory-model-for-high-throughput-data)
- [Language Specification: Data Types](../lonala/data-types.md)
- [Clojure Rationale](https://clojure.org/about/rationale)
- [Simple Made Easy](https://www.infoq.com/presentations/Simple-Made-Easy/) — Rich Hickey
