# Data Types

All Lonala values belong to one of these types.

---

## Nil

The absence of a value.

```clojure
nil
```

Predicate: `nil?`

---

## Booleans

```clojure
true
false
```

Predicates: `boolean?`, `true?`, `false?`

Falsiness: Only `nil` and `false` are falsy. Everything else is truthy.

---

## Numbers

### Integer

Arbitrary precision by default.

```clojure
42
-17
99999999999999999999N   ; Explicit BigInt
```

Predicate: `integer?`

### Fixed-Width Integers

| Type | Size | Signed |
|------|------|--------|
| `u8` | 8-bit | No |
| `u16` | 16-bit | No |
| `u32` | 32-bit | No |
| `u64` | 64-bit | No |
| `i8` | 8-bit | Yes |
| `i16` | 16-bit | Yes |
| `i32` | 32-bit | Yes |
| `i64` | 64-bit | Yes |

Overflow wraps (two's complement).

Coercion: `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`

### Float

IEEE 754 floating point.

```clojure
3.14        ; f64 (default)
3.14f32     ; f32
6.022e23    ; Scientific notation
```

Predicate: `float?`

### Ratio

Exact rational numbers.

```clojure
22/7
1/3
```

Predicate: `ratio?`

---

## Characters

Single Unicode code points.

```clojure
\a
\newline
\u0041
```

Predicate: `char?`

Coercion: `char`

---

## Strings

Immutable UTF-8 sequences.

```clojure
"Hello, World!"
```

Predicate: `string?`

---

## Symbols

Names that refer to values.

```clojure
foo
my.ns/bar
```

Predicate: `symbol?`

Constructor: `symbol`

Accessors: `name`, `namespace`

---

## Keywords

Self-evaluating names, often used as map keys.

```clojure
:foo
:my.ns/bar
```

Predicate: `keyword?`

Constructor: `keyword`

Accessors: `name`, `namespace`

---

## Lists

Singly-linked lists. O(1) prepend, O(n) access.

```clojure
(1 2 3)
'(a b c)
```

Predicate: `list?`

---

## Tuples

Fixed-size indexed sequences. O(1) access, O(n) modification (creates new tuple).

```clojure
[1 2 3]
[:ok value]
```

Predicate: `tuple?`

**When to use tuples:**
- Function parameters and return values: `[:ok result]`, `[:error reason]`
- Pattern matching: `[head & tail]`
- Fixed-structure data: `[x y z]` coordinates
- Message payloads: `[:request id data]`

---

## Vectors

Persistent indexed sequences with structural sharing. O(log₃₂n) update, O(1) append.

```clojure
{1 2 3}
```

Predicate: `vector?`

**When to use vectors:**
- Accumulating data: `(append vec item)`
- Dynamic collections that grow/shrink
- When you need efficient updates without copying everything
- Building results incrementally

---

## Maps

Persistent key-value associations.

```clojure
%{:name "Alice" :age 30}
```

Predicate: `map?`

---

## Sets

Persistent unique element collections.

```clojure
#{1 2 3}
```

Predicate: `set?`

---

## Sequences (Abstraction)

A sequence is an ordered traversal of a collection via `first` and `rest`.

**All collections are seqable:**

| Collection | Traversal Order |
|------------|-----------------|
| List | Front to back |
| Tuple | Front to back |
| Vector | Front to back |
| Map | Key-value tuples (order unspecified) |
| Set | Elements (order unspecified) |

**Key properties:**

- `first` and `rest` are polymorphic intrinsics that work on any collection
- `rest` always returns a **list**, regardless of input collection type
- This enables generic functions (`map`, `filter`, `reduce`) to work on all collections

```clojure
;; Same function works on any collection
(defn my-map [f coll]
  (if (empty? coll)
    '()
    (cons (f (first coll)) (my-map f (rest coll)))))

(my-map inc '(1 2 3))  ; → (2 3 4)
(my-map inc [1 2 3])   ; → (2 3 4)
(my-map inc {1 2 3})   ; → (2 3 4)
```

**No lazy sequences (yet):** All sequence operations are eager. Laziness may be added
in a future version.

---

## Binary

Immutable byte sequences. Reference-counted for efficient sharing.

```clojure
#bytes[0x48 0x65 0x6C]
#bytes"Hello"
```

Predicate: `binary?`

Constructor: `binary`

---

## Bytebuf

Mutable byte buffers for I/O operations. Not shareable across processes.

Predicate: `bytebuf?`

Constructor: `bytebuf-alloc`, `bytebuf-alloc-unsafe`

**Enforcement:** Attempting to send a bytebuf in a message raises an error.
Convert to immutable `binary` first if sharing is needed.

---

## Physical Address (paddr)

Physical memory address as seen by hardware/DMA.

```clojure
(paddr 0x1000_0000u64)
```

Predicate: `paddr?`

---

## Virtual Address (vaddr)

Virtual memory address as seen by CPU/process.

```clojure
(vaddr 0x4000_0000u64)
```

Predicate: `vaddr?`

---

## Realm ID (realm-id)

Opaque identifier for a realm. Returned by `realm-create`, extracted via `pid-realm`.

Predicate: `realm-id?`

Accessors: `self-realm` returns current process's realm-id.

Used in: `spawn-in`, `realm-terminate`, `share-region`, etc.

---

## Process ID (pid)

Identifies a process with realm and local components.

```clojure
(pid realm-id local-id)
```

Predicate: `pid?`

Accessors: `pid-realm` (returns realm-id), `pid-local`

Pattern matching: `(pid r l)` destructures realm and local.

---

## Reference (ref)

Unique reference values for request/response correlation.

```clojure
(make-ref)
```

Predicate: `ref?`

---

## Notification

Asynchronous signaling object (wraps seL4 notification).

```clojure
(make-notification)
```

Predicate: `notification?`

---

## Capabilities

Tokens granting rights to seL4 kernel objects. Used by `lona.kernel` intrinsics.

**Note:** High-level types like `notification` (from `make-notification`) wrap underlying capabilities. The `*-cap?` predicates test for raw capabilities; `lona.kernel` functions accept these directly. Higher-level `lona.process` and `lona.io` functions accept wrapper types.

**Object capabilities:**

| Type | Predicate | Description |
|------|-----------|-------------|
| TCB | `tcb-cap?` | Thread Control Block |
| Endpoint | `endpoint-cap?` | IPC endpoint |
| Notification | `notification-cap?` | Async notification |
| CNode | `cnode-cap?` | Capability container |
| Untyped | `untyped-cap?` | Raw memory |
| Frame | `frame-cap?` | Physical page (4K, 2M, 1G) |
| SchedContext | `sched-context-cap?` | CPU budget (MCS) |
| IRQHandler | `irq-handler-cap?` | Interrupt handler |

**Page table capabilities (architecture-specific):**

| x86_64 | ARM64 | Predicate |
|--------|-------|-----------|
| PML4 | PGD | `pml4-cap?` / `pgd-cap?` |
| PDPT | PUD | `pdpt-cap?` / `pud-cap?` |
| PageDirectory | PMD | `page-directory-cap?` / `pmd-cap?` |
| PageTable | PTE | `page-table-cap?` / `pte-cap?` |

**Control capabilities (singleton, held by root):**

| Type | Predicate | Description |
|------|-----------|-------------|
| SchedControl | `sched-control-cap?` | Scheduling domain control |
| IRQControl | `irq-control-cap?` | IRQ handler creation authority |
| ASIDControl | `asid-control-cap?` | ASID pool creation authority |
| ASIDPool | `asid-pool-cap?` | ASID allocation for VSpaces |
| FDT | `fdt-cap?` | Device tree access |

**Architecture-specific capabilities:**

| Type | Predicate | Arch | Description |
|------|-----------|------|-------------|
| Port | `port-cap?` | x86 | I/O port access |
| IOSpace | `iospace-cap?` | x86 | IOMMU domain (VT-d) |
| VCPU | `vcpu-cap?` | both | Virtual CPU (virtualization) |

Generic predicate: `cap?`

Inspectors: `cap-type`, `cap-rights`, `cap-has-right?`

Rights: `:read`, `:write`, `:grant`, `:grant-reply`

---

## Message Info (msg-info)

Metadata for seL4 IPC operations.

```clojure
(msg-info label length caps)
```

Predicate: `msg-info?`

Accessors: `msg-info-label`, `msg-info-length`, `msg-info-caps`

---

## Region

Shared memory region handle.

Predicate: `region?`

Accessors: `region-size`, `region-name`

---

## DMA Buffer

DMA-capable memory buffer.

Predicate: `dma-buffer?`

Accessors: `dma-vaddr`, `dma-paddr`, `dma-size`

---

## Ring Buffer

Lock-free ring buffer for driver communication.

Predicate: `ring?`

---

## Functions

First-class functions.

Predicate: `fn?`

---

## Namespace

A named container for var bindings.

```clojure
(create-ns 'my.namespace)  ; → namespace
(find-ns 'my.namespace)    ; → namespace or nil
```

Predicate: `namespace?`

Accessors: `ns-name` (returns symbol), `ns-map` (returns var bindings map)

---

## Vars

References to namespace bindings.

Predicate: `var?`

Accessor: `var-get`

### Process-Bound Vars

Vars with `^:process-bound` metadata have per-process bindings. A single Var exists
in the realm, but each process can shadow its root value via a per-process binding table.

```clojure
(def ^:process-bound *ns* (find-ns 'lona.core))
```

- `def` on a process-bound var updates only the current process's binding (not the realm root)
- Spawned processes inherit the parent's binding values at spawn time

The current namespace `*ns*` is the primary use case for process-bound vars.
