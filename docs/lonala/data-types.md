# Data Types

All Lonala values belong to one of these types.

---

## Nil

The absence of a value.

```clojure
nil  ; => nil
```

Predicate: `nil?`

```clojure
(nil? nil)    ; => true
(nil? false)  ; => false
(nil? 0)      ; => false
(nil? "")     ; => false
(nil? '())    ; => false  @todo
```

---

## Booleans

```clojure
true   ; => true
false  ; => false
```

Predicates: `boolean?`, `true?`, `false?`

```clojure
;; @todo
(boolean? true)   ; => true
(boolean? false)  ; => true
(boolean? nil)    ; => false
(boolean? 0)      ; => false
(boolean? 1)      ; => false

(true? true)    ; => true
(true? false)   ; => false
(true? nil)     ; => false
(true? 1)       ; => false

(false? false)  ; => true
(false? true)   ; => false
(false? nil)    ; => false
```

Falsiness: Only `nil` and `false` are falsy. Everything else is truthy.

```clojure
;; nil and false are falsy
(not nil)    ; => true
(not false)  ; => true

;; Everything else is truthy (including 0, empty collections, empty string)
(not true)   ; => false
(not 0)      ; => false
(not "")     ; => false
(not '())    ; => false  @todo
(not [])     ; => false
(not {})     ; => false
(not %{})    ; => false
(not #{})    ; => false  @todo
```

---

## Numbers

### Integer

Arbitrary precision by default.

```clojure
42                      ; => 42
-17                     ; => -17
99999999999999999999N   ; => 99999999999999999999N  @todo
```

Predicate: `integer?`

```clojure
(integer? 42)      ; => true
(integer? -17)     ; => true
(integer? 3.14)    ; => false  @todo
(integer? 22/7)    ; => false  @todo
(integer? "42")    ; => false
```

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

```clojure
;; @todo
(u8 255)       ; => 255u8
(u8 256)       ; => 0u8      ; wraps
(u8 -1)        ; => 255u8    ; wraps
(i8 127)       ; => 127i8
(i8 128)       ; => -128i8   ; wraps
(i8 -129)      ; => 127i8    ; wraps
```

Coercion: `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`

```clojure
;; @todo
(u8 42)   ; => 42u8
(u16 42)  ; => 42u16
(u32 42)  ; => 42u32
(u64 42)  ; => 42u64
(i8 42)   ; => 42i8
(i16 42)  ; => 42i16
(i32 42)  ; => 42i32
(i64 42)  ; => 42i64
```

Additional overflow/underflow cases:

```clojure
;; @todo
;; u16 overflow
(u16 65536)   ; => 0u16
(u16 65537)   ; => 1u16
(u16 -1)      ; => 65535u16

;; u32 overflow
(u32 4294967296)  ; => 0u32
(u32 -1)          ; => 4294967295u32

;; i16 overflow
(i16 32768)   ; => -32768i16
(i16 -32769)  ; => 32767i16

;; i32 overflow
(i32 2147483648)   ; => -2147483648i32
(i32 -2147483649)  ; => 2147483647i32
```

### Float

IEEE 754 floating point.

```clojure
3.14        ; => 3.14      @todo
3.14f32     ; => 3.14f32   @todo
6.022e23    ; => 6.022e23  @todo
```

Predicate: `float?`

```clojure
;; @todo
(float? 3.14)    ; => true
(float? 3.14f32) ; => true
(float? 42)      ; => false
(float? 22/7)    ; => false
```

### Ratio

Exact rational numbers.

```clojure
22/7  ; => 22/7  @todo
1/3   ; => 1/3   @todo
2/4   ; => 1/2   @todo  ; auto-reduces
```

Predicate: `ratio?`

```clojure
;; @todo
(ratio? 22/7)  ; => true
(ratio? 1/3)   ; => true
(ratio? 1)     ; => false
(ratio? 0.5)   ; => false
```

---

## Characters

Single Unicode code points.

```clojure
;; @todo
\a        ; => \a
\newline  ; => \newline
\u0041    ; => \A
```

Predicate: `char?`

```clojure
;; @todo
(char? \a)       ; => true
(char? \newline) ; => true
(char? "a")      ; => false
(char? 65)       ; => false
```

Coercion: `char`

```clojure
;; @todo
(char 65)   ; => \A
(char 97)   ; => \a
(char 10)   ; => \newline
```

---

## Strings

Immutable UTF-8 sequences.

```clojure
"Hello, World!"  ; => "Hello, World!"
""               ; => ""
"Line 1\nLine 2" ; => "Line 1\nLine 2"
```

Predicate: `string?`

```clojure
(string? "hello")  ; => true
(string? "")       ; => true
(string? \a)       ; => false  @todo
(string? 42)       ; => false
(string? :hello)   ; => false
```

---

## Symbols

Names that refer to values.

```clojure
'foo        ; => foo
'my.ns/bar  ; => my.ns/bar
```

Predicate: `symbol?`

```clojure
(symbol? 'foo)       ; => true
(symbol? 'my.ns/bar) ; => true
(symbol? :foo)       ; => false
(symbol? "foo")      ; => false
```

Constructor: `symbol`

```clojure
;; @todo
(symbol "foo")         ; => foo
(symbol "my.ns" "bar") ; => my.ns/bar
```

Accessors: `name`, `namespace`

```clojure
(name 'foo)            ; => "foo"
(name 'my.ns/bar)      ; => "bar"
(namespace 'foo)       ; => nil
(namespace 'my.ns/bar) ; => "my.ns"
```

---

## Keywords

Self-evaluating names, often used as map keys.

```clojure
:foo        ; => :foo
:my.ns/bar  ; => :my.ns/bar
```

Predicate: `keyword?`

```clojure
(keyword? :foo)       ; => true
(keyword? :my.ns/bar) ; => true
(keyword? 'foo)       ; => false
(keyword? "foo")      ; => false
```

Constructor: `keyword`

```clojure
(keyword "foo")         ; => :foo
(keyword "my.ns" "bar") ; => :my.ns/bar  @todo
```

Accessors: `name`, `namespace`

```clojure
(name :foo)            ; => "foo"
(name :my.ns/bar)      ; => "bar"
(namespace :foo)       ; => nil
(namespace :my.ns/bar) ; => "my.ns"
```

---

## Lists

Singly-linked lists. O(1) prepend, O(n) access.

```clojure
'(1 2 3)    ; => (1 2 3)
'(a b c)    ; => (a b c)
'()         ; => ()  @todo
```

Predicate: `list?`

```clojure
;; @todo
(list? '(1 2 3))  ; => true
(list? '())       ; => true
(list? [1 2 3])   ; => false
(list? {1 2 3})   ; => false
(list? nil)       ; => false
```

---

## Tuples

Fixed-size indexed sequences. O(1) access, O(n) modification (creates new tuple).

```clojure
[1 2 3]     ; => [1 2 3]
[:ok 42]    ; => [:ok 42]
[]          ; => []
```

Predicate: `tuple?`

```clojure
(tuple? [1 2 3])   ; => true
(tuple? [:ok 42])  ; => true
(tuple? [])        ; => true
(tuple? '(1 2 3))  ; => false
(tuple? {1 2 3})   ; => false
```

**When to use tuples:**
- Function parameters and return values: `[:ok result]`, `[:error reason]`
- Pattern matching: `[head & tail]`
- Fixed-structure data: `[x y z]` coordinates
- Message payloads: `[:request id data]`

---

## Vectors

Persistent indexed sequences with structural sharing. O(log₃₂n) update, O(1) append.

```clojure
{1 2 3}  ; => {1 2 3}
{}       ; => {}
```

Predicate: `vector?`

```clojure
(vector? {1 2 3})  ; => true
(vector? {})       ; => true
(vector? [1 2 3])  ; => false
(vector? '(1 2 3)) ; => false
```

**When to use vectors:**
- Accumulating data: `(append vec item)`
- Dynamic collections that grow/shrink
- When you need efficient updates without copying everything
- Building results incrementally

---

## Maps

Persistent key-value associations.

```clojure
%{:name "Alice" :age 30}  ; => %{:name "Alice" :age 30}
%{}                       ; => %{}
```

Predicate: `map?`

```clojure
(map? %{:a 1})     ; => true
(map? %{})         ; => true
(map? [1 2 3])     ; => false
(map? #{1 2 3})    ; => false  @todo
```

---

## Sets

Persistent unique element collections.

```clojure
;; @todo
#{1 2 3}  ; => #{1 2 3}
#{}       ; => #{}
#{1 1 2}  ; => #{1 2}   ; duplicates removed
```

Predicate: `set?`

```clojure
;; @todo
(set? #{1 2 3})  ; => true
(set? #{})       ; => true
(set? [1 2 3])   ; => false
(set? %{:a 1})   ; => false
```

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
;; first on different collection types
(first '(1 2 3))      ; => 1
(first [1 2 3])       ; => 1
(first {1 2 3})       ; => 1
(first '())           ; => nil
(first [])            ; => nil
(first nil)           ; => nil

;; Strings are seqable too
(first "abc")         ; => \a  @todo
(first "")            ; => nil @todo
```

```clojure
;; rest always returns a list
(rest '(1 2 3))  ; => (2 3)
(rest [1 2 3])   ; => (2 3)
(rest {1 2 3})   ; => (2 3)
(rest '(1))      ; => ()  @todo
(rest '())       ; => ()  @todo
(rest nil)       ; => ()  @todo
```

**No lazy sequences (yet):** All sequence operations are eager. Laziness may be added
in a future version.

---

## Binary

Immutable byte sequences. Reference-counted for efficient sharing.

```clojure
;; @todo
#bytes[0x48 0x65 0x6C]  ; => #bytes[0x48 0x65 0x6C]
#bytes"Hello"           ; => #bytes[0x48 0x65 0x6C 0x6C 0x6F]
#bytes[]                ; => #bytes[]
```

Predicate: `binary?`

```clojure
;; @todo
(binary? #bytes[1 2 3])  ; => true
(binary? #bytes"hi")     ; => true
(binary? "hello")        ; => false
(binary? [1 2 3])        ; => false
```

Constructor: `binary`

```clojure
;; @todo
(binary '(72 101 108 108 111))  ; => #bytes[0x48 0x65 0x6C 0x6C 0x6F]
```

---

## Bytebuf

Mutable byte buffers for I/O operations. Not shareable across processes.

Predicate: `bytebuf?`

```clojure
;; @todo
(def buf (bytebuf-alloc 8))
(bytebuf? buf)   ; => true
(bytebuf? #bytes[1 2 3])  ; => false
```

Constructor: `bytebuf-alloc`, `bytebuf-alloc-unsafe`

```clojure
;; @todo
(def buf1 (bytebuf-alloc 16))        ; zeroed buffer
(def buf2 (bytebuf-alloc-unsafe 16)) ; uninitialized buffer
(bytebuf-size buf1)  ; => 16
```

**Enforcement:** Attempting to send a bytebuf in a message raises an error.
Convert to immutable `binary` first if sharing is needed.

```clojure
;; @todo
;; Bytebufs cannot be sent in messages
(def buf (bytebuf-alloc 8))
(send (self) buf)  ; => ERROR

;; Convert to binary for sharing
(binary? (bytebuf->binary buf))  ; => true
```

---

## Physical Address (paddr)

Physical memory address as seen by hardware/DMA.

```clojure
;; @todo
(paddr 0x1000_0000u64)  ; => #paddr<0x10000000>
```

Predicate: `paddr?`

```clojure
;; @todo
(paddr? (paddr 0x1000u64))  ; => true
(paddr? 0x1000u64)          ; => false
(paddr? (vaddr 0x1000u64))  ; => false
```

---

## Virtual Address (vaddr)

Virtual memory address as seen by CPU/process.

```clojure
;; @todo
(vaddr 0x4000_0000u64)  ; => #vaddr<0x40000000>
```

Predicate: `vaddr?`

```clojure
;; @todo
(vaddr? (vaddr 0x1000u64))  ; => true
(vaddr? 0x1000u64)          ; => false
(vaddr? (paddr 0x1000u64))  ; => false
```

---

## Realm ID (realm-id)

Opaque identifier for a realm. Returned by `realm-create`, extracted via `pid-realm`.

Predicate: `realm-id?`

```clojure
;; @todo
(realm-id? (self-realm))  ; => true
(realm-id? 42)            ; => false
```

Accessors: `self-realm` returns current process's realm-id.

```clojure
;; @todo
(self-realm)  ; => #realm-id<...>
```

Used in: `spawn-in`, `realm-terminate`, `share-region`, etc.

---

## Process ID (pid)

Identifies a process with realm and local components.

```clojure
;; @todo
(def my-pid (self))
(pid? my-pid)  ; => true
```

Predicate: `pid?`

```clojure
;; @todo
(pid? (self))  ; => true
```

```clojure
(pid? 42)      ; => false
(pid? nil)     ; => false
```

Accessors: `pid-realm` (returns realm-id), `pid-local`

```clojure
;; @todo
(def my-pid (self))
(realm-id? (pid-realm my-pid))  ; => true
(integer? (pid-local my-pid))   ; => true
```

Pattern matching: `(pid r l)` destructures realm and local.

```clojure
;; @todo
(match (self)
  (pid realm local) [realm local])  ; => [#realm-id<...> N]
```

---

## Reference (ref)

Unique reference values for request/response correlation.

```clojure
;; @todo
(def r (make-ref))
(ref? r)  ; => true
```

Predicate: `ref?`

```clojure
;; @todo
(ref? (make-ref))  ; => true
(ref? 42)          ; => false
(ref? :ref)        ; => false
```

Uniqueness: each call returns a distinct ref.

```clojure
;; @todo
(= (make-ref) (make-ref))  ; => false
```

---

## Notification

Asynchronous signaling object (wraps seL4 notification).

```clojure
;; @todo
(def n (make-notification))
(notification? n)  ; => true
```

Predicate: `notification?`

```clojure
;; @todo
(notification? (make-notification))  ; => true
(notification? 42)                   ; => false
```

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

```clojure
;; @todo
;; cap? returns true for any capability type
(cap? tcb-cap)  ; => true
(cap? 42)       ; => false
```

Inspectors: `cap-type`, `cap-rights`, `cap-has-right?`

```clojure
;; @todo
;; Generic cap? predicate
(def frame (get-test-frame-cap))  ; setup - get a frame capability
(cap? frame)          ; => true
(cap? 42)             ; => false
(cap? nil)            ; => false
(cap? :frame)         ; => false

;; cap-type returns the type keyword
(keyword? (cap-type frame))  ; => true

;; cap-rights returns set of rights
(set? (cap-rights frame))  ; => true

;; cap-has-right? checks for specific right
(boolean? (cap-has-right? frame :read))  ; => true
```

Rights: `:read`, `:write`, `:grant`, `:grant-reply`

---

## Message Info (msg-info)

Metadata for seL4 IPC operations.

```clojure
;; @todo
(def mi (msg-info 100 4 2))
(msg-info? mi)  ; => true
```

Predicate: `msg-info?`

```clojure
;; @todo
(msg-info? (msg-info 0 0 0))  ; => true
(msg-info? 42)                ; => false
```

Accessors: `msg-info-label`, `msg-info-length`, `msg-info-caps`

```clojure
;; @todo
(def mi (msg-info 100 4 2))
(msg-info-label mi)   ; => 100
(msg-info-length mi)  ; => 4
(msg-info-caps mi)    ; => 2
```

---

## Region

Shared memory region handle.

Predicate: `region?`

```clojure
;; @todo
(def r (make-shared-region 4096 'my-region))
(region? r)   ; => true
(region? 42)  ; => false
```

Accessors: `region-size`, `region-name`

```clojure
;; @todo
(def r (make-shared-region 4096 'my-region))
(region-size r)  ; => 4096
(region-name r)  ; => my-region
```

---

## DMA Buffer

DMA-capable memory buffer.

Predicate: `dma-buffer?`

```clojure
;; @todo
(def buf (dma-alloc 4096 :dma/coherent))
(dma-buffer? buf)  ; => true
(dma-buffer? 42)   ; => false
```

Accessors: `dma-vaddr`, `dma-paddr`, `dma-size`

```clojure
;; @todo
(def buf (dma-alloc 4096 :dma/coherent))
(vaddr? (dma-vaddr buf))  ; => true
(paddr? (dma-paddr buf))  ; => true
(dma-size buf)            ; => 4096
```

---

## Ring Buffer

Lock-free ring buffer for driver communication.

Predicate: `ring?`

```clojure
;; @todo
(def r (ring-create 64 256))  ; 64 entries of 256 bytes each
(ring? r)   ; => true
(ring? 42)  ; => false
```

---

## Functions

First-class functions.

Predicate: `fn?`

```clojure
(fn? (fn* [x] x))  ; => true
(fn? +)            ; => true
(fn? 42)           ; => false
(fn? :foo)         ; => false
```

---

## Namespace

A named container for var bindings.

```clojure
(def ns (create-ns 'my.namespace))
(namespace? ns)              ; => true
(find-ns 'my.namespace)      ; => ns  @todo
(find-ns 'nonexistent)       ; => nil
```

Predicate: `namespace?`

```clojure
(namespace? (find-ns 'lona.core))  ; => true
(namespace? 42)                    ; => false
(namespace? 'lona.core)            ; => false
```

Accessors: `ns-name` (returns symbol), `ns-map` (returns var bindings map)

```clojure
(def ns (find-ns 'lona.core))
(symbol? (ns-name ns))  ; => true
(map? (ns-map ns))      ; => true  @todo
```

---

## Vars

References to namespace bindings.

Predicate: `var?`

```clojure
(var? #'+)   ; => true
(var? +)     ; => false
(var? 42)    ; => false
```

Accessor: `var-get`

```clojure
(def x 42)
(var-get #'x)  ; => 42
```

### Process-Bound Vars

Vars with `^:process-bound` metadata have per-process bindings. A single Var exists
in the realm, but each process can shadow its root value via a per-process binding table.

```clojure
(def ^:process-bound *ns* (find-ns 'lona.core))
```

- `def` on a process-bound var updates only the current process's binding (not the realm root)
- Spawned processes inherit the parent's binding values at spawn time

The current namespace `*ns*` is the primary use case for process-bound vars.

Process-bound var inheritance:

```clojure
;; @todo
;; Child processes inherit parent's binding at spawn time
(def ^:process-bound *my-var* :parent-value)
(def child-result (make-ref))
(def p (spawn (fn* []
  (send (self) *my-var*))))
(receive val val :after 1000 :timeout)  ; => :parent-value
```
