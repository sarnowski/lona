# Lonala Language Specification

Lonala is a LISP dialect designed for the **Lonala VM**, a custom virtual machine running on the **seL4 microkernel**. The language is heavily inspired by Clojure's elegance and Elixir/Erlang's concurrency model. The VM adopts many concepts from BEAM (Erlang VM) but is not BEAM-compatible — it's a new implementation leveraging seL4's formally verified foundation for security and reliability.

---

## Table of Contents

1. [Design Philosophy](#design-philosophy)
2. [Special Forms](#special-forms)
3. [Reader Literals](#reader-literals)
4. [Binary Types](#binary-types)
5. [Address Types](#address-types)
6. [System Types](#system-types)
7. [Reader Macros](#reader-macros)
8. [Bitwise Operations](#bitwise-operations)
9. [Pattern Matching](#pattern-matching)
10. [Function Definitions](#function-definitions)
11. [Error Handling](#error-handling)
12. [Derived Macros](#derived-macros)
13. [Summary](#summary)

### Related Specifications

| Document | Namespace | Description |
|----------|-----------|-------------|
| [lonala-process.md](lonala-process.md) | `lona.process` | BEAM-style lightweight processes, message passing, supervisors |
| [lonala-kernel.md](lonala-kernel.md) | `lona.kernel` | Low-level seL4 kernel operations (capabilities, IPC, scheduling) |
| [lonala-io.md](lonala-io.md) | `lona.io` | Device driver primitives (MMIO, DMA, IRQ, Port I/O) |

---

## Design Philosophy

### Core Principles

- **seL4 foundation**: Runs on a battle-tested microkernel for security and reliability
- **BEAM-inspired concurrency**: Lightweight processes, message passing, supervisors (custom VM, not BEAM)
- **Minimal primitives**: Only 5 special forms; everything else is macros
- **Pattern matching central**: Replaces conditionals, destructuring, and dispatch
- **"Let it crash"**: Elixir-style error handling with supervisor restarts
- **Automatic TCO**: Guaranteed tail-call optimization (Scheme-style)
- **Homoiconicity**: Code is data, fully manipulable

### What We Took from Clojure

- Persistent data structures (vectors, maps, sets)
- Namespace system and vars
- Macro system with syntax-quote
- Metadata support
- Keyword and symbol distinction
- Reader macro system

### What We Changed from Clojure

| Clojure | Lonala | Reason |
|---------|--------|--------|
| `recur` | Removed | Automatic TCO makes it unnecessary |
| `try`/`catch`/`finally` | Removed | Tuple returns + "let it crash" |
| `loop` | Derived macro | With TCO, just use recursive functions |
| Thread-local bindings | Removed (for now) | Process isolation instead |
| `deftype*`/`reify*`/`import*` | Removed | JVM class loading mechanics |
| Java interop (`.`, `new`) | Removed | Not on JVM |
| `monitor-enter`/`monitor-exit` | Removed | Message passing, not locks |
| Regex literals | Removed | Deferred to library |
| Tagged literals | Removed | Not needed initially |

### What We Took from Elixir/Erlang

- Tuple-based returns (`[:ok value]`, `[:error reason]`)
- Pattern matching in function heads
- Process-based concurrency
- "Let it crash" philosophy
- Supervisor trees for fault tolerance
- Guards in pattern matching
- First-class `pid` type with realm/local structure (like BEAM's node/pid)
- Immutable `binary` type with reference counting for large data
- Bit syntax (`#bits[...]`) for protocol parsing and construction
- Monitor references and process linking semantics
- `receive` with selective pattern matching and timeouts

---

## Special Forms

Lonala has exactly **5 special forms**. Everything else is built as macros.

### `def`

Creates or updates a root var binding (upsert semantics).

```clojure
(def name value)
```

**Examples:**
```clojure
(def pi 3.14159)
(def greeting "Hello, World!")

;; Redefining updates the root binding
(def pi 3.14159265359)
```

**Notes:**
- No separate `set!` needed — `def` handles both creation and update
- No thread/process-local bindings initially (root vars only)

---

### `fn*`

Creates a function with a single parameter list and body.

```clojure
(fn* [params] body)
```

**Examples:**
```clojure
(fn* [x] (+ x 1))
(fn* [a b] (+ a b))
(fn* [& args] (apply + args))  ; variadic
```

**Notes:**
- Single arity only — multi-arity is handled by `match` in the `fn` macro
- No pattern matching — patterns live in `match`
- Body has implicit `do` (multiple expressions allowed)
- This is the low-level primitive; users typically use `fn` or `defn` macros

---

### `match`

Pattern matching expression with optional guards.

```clojure
(match expr
  pattern1 body1
  pattern2 when guard2 body2
  pattern3 body3
  ...)
```

**Examples:**
```clojure
;; Simple value matching
(match x
  1 "one"
  2 "two"
  _ "other")

;; Destructuring tuples
(match result
  [:ok value]    value
  [:error reason] (str "Error: " reason))

;; With guards
(match n
  x when (> x 0) "positive"
  x when (< x 0) "negative"
  _ "zero")

;; Nested patterns
(match data
  [:user %{:name n :age a}] (format "~a is ~a years old" n a)
  _ "unknown")
```

**Pattern Types:**

| Pattern | Matches | Binds |
|---------|---------|-------|
| `x` (symbol) | Anything | Yes, binds `x` |
| `_` | Anything | No (wildcard) |
| `42` (literal) | Exactly `42` | No |
| `:ok` (keyword) | Exactly `:ok` | No |
| `"hello"` | Exactly `"hello"` | No |
| `[a b]` | 2-element tuple | Yes, binds `a`, `b` |
| `[h & t]` | Non-empty tuple | Yes, head + tail |
| `{a b c}` | 3-element vector | Yes, binds `a`, `b`, `c` |
| `%{:key v}` | Map with `:key` | Yes, binds `v` |
| `#{a b}` | 2-element set | Yes, binds `a`, `b` |

**Guard Syntax:**
- `when` is a reserved symbol within `match` context
- Everything after `when` until the body is the guard expression
- Guards must be pure expressions (no side effects)

**Failed Match:**
- If no pattern matches, raises `MatchError`
- Process crashes (let it crash philosophy)
- Supervisors handle restart

**Body:**
- Each body is a **single expression**
- Use explicit `do` for multiple expressions:
  ```clojure
  (match x
    [:ok v] (do
              (log "success")
              v)
    [:error e] (panic! e))
  ```

---

### `do`

Evaluates expressions in sequence, returns the last.

```clojure
(do expr1 expr2 ... exprN)
```

**Examples:**
```clojure
(do
  (println "Starting...")
  (process-data)
  (println "Done!")
  :ok)
```

**Notes:**
- Essential for sequencing side effects
- Returns value of last expression
- Used explicitly in `match` bodies when multiple expressions needed

---

### `quote`

Prevents evaluation, returns the form as data.

```clojure
(quote form)
'form  ; reader shorthand
```

**Examples:**
```clojure
(quote (+ 1 2))    ; → (+ 1 2) as a list
'(+ 1 2)           ; same
'foo               ; → symbol foo
```

---

## Reader Literals

### Numeric Types

Lonala provides arbitrary-precision numbers by default, plus fixed-width integers for systems programming.

#### Arbitrary-Precision Numbers (Default)

| Literal | Type | Example |
|---------|------|---------|
| `42` | Integer | `42`, `-17` |
| `42N` | BigInt (explicit arbitrary precision) | `99999999999999999999N` |
| `3.14` | Float (f64) | `3.14`, `-0.5`, `6.022e23` |
| `3.14M` | BigDecimal (arbitrary precision) | `3.14159265358979M` |
| `22/7` | Ratio | `1/3`, `355/113` |

**Base prefixes** (work with any integer type):

| Prefix | Base | Example |
|--------|------|---------|
| `0x` | Hexadecimal | `0xFF` → 255 |
| `0o` | Octal | `0o10` → 8 |
| `0b` | Binary | `0b1010` → 10 |
| `Nr` | Arbitrary radix | `2r1111` → 15, `16rFF` → 255 |

**Special float values:**

| Literal | Value |
|---------|-------|
| `##Inf` | Positive infinity |
| `##-Inf` | Negative infinity |
| `##NaN` | Not a number |

#### Fixed-Width Integer Types

For device drivers, protocol parsing, and low-level operations, Lonala provides fixed-width integers with explicit size and signedness.

**Unsigned integers:**

| Type | Size | Range | Literal Suffix |
|------|------|-------|----------------|
| `u8` | 8-bit | 0 to 255 | `u8` |
| `u16` | 16-bit | 0 to 65,535 | `u16` |
| `u32` | 32-bit | 0 to 4,294,967,295 | `u32` |
| `u64` | 64-bit | 0 to 2⁶⁴-1 | `u64` |

**Signed integers:**

| Type | Size | Range | Literal Suffix |
|------|------|-------|----------------|
| `i8` | 8-bit | -128 to 127 | `i8` |
| `i16` | 16-bit | -32,768 to 32,767 | `i16` |
| `i32` | 32-bit | -2³¹ to 2³¹-1 | `i32` |
| `i64` | 64-bit | -2⁶³ to 2⁶³-1 | `i64` |

**Literal syntax:**

```clojure
;; Decimal with suffix
42u8              ; u8 value 42
1000u16           ; u16 value 1000
0u32              ; u32 value 0
-50i16            ; i16 value -50

;; Hex with suffix (common for device registers)
0xFFu8            ; u8 value 255
0x1000_0000u32    ; u32 with underscores for readability
0xDEAD_BEEFu32    ; u32 value

;; Binary with suffix (common for bit flags)
0b1010_0101u8     ; u8 value 165
0b1111_1111u8     ; u8 value 255
```

**Underscore separators:** All numeric literals support `_` for readability:

```clojure
1_000_000         ; One million
0xFF_FF_FF_FFu32  ; Max u32
0b1111_0000u8     ; Binary with groups
```

**Overflow behavior:**

Fixed-width integers wrap on overflow (two's complement semantics):

```clojure
(+ 255u8 1u8)     ; → 0u8 (wraps)
(- 0u8 1u8)       ; → 255u8 (wraps)
```

For checked arithmetic, use explicit functions:

```clojure
(checked-add 255u8 1u8)   ; → [:error :overflow]
(saturating-add 255u8 1u8) ; → 255u8 (clamps to max)
```

#### Floating-Point Types

| Type | Size | Literal | Precision |
|------|------|---------|-----------|
| `f32` | 32-bit | `3.14f32` | ~7 decimal digits |
| `f64` | 64-bit | `3.14` or `3.14f64` | ~15 decimal digits |

```clojure
3.14              ; f64 (default)
3.14f64           ; f64 (explicit)
3.14f32           ; f32 (single precision)
6.022e23f32       ; f32 with exponent
```

---

### Atomic Types

| Literal | Type | Example |
|---------|------|---------|
| `nil` | Nil/null | `nil` |
| `true` | Boolean | `true` |
| `false` | Boolean | `false` |
| `"hello"` | String (UTF-8) | `"Hello, World!"` |
| `\a` | Character | `\a`, `\newline`, `\space` |

**Character Escapes:**
- `\newline`, `\space`, `\tab`, `\return`, `\formfeed`, `\backspace`
- `\uNNNN` — Unicode code point
- `\oNNN` — Octal

---

### Symbolic Types

| Literal | Type | Example |
|---------|------|---------|
| `foo` | Symbol | `foo`, `my-var`, `+` |
| `my.ns/bar` | Qualified symbol | `clojure.core/map` |
| `:foo` | Keyword | `:name`, `:ok`, `:error` |
| `:my.ns/bar` | Qualified keyword | `:user/name` |
| `::foo` | Auto-resolved keyword | Resolves to current namespace |

---

### Collection Types

| Syntax | Type | Characteristics |
|--------|------|-----------------|
| `(1 2 3)` | **List** | Linked, O(1) prepend, code-as-data |
| `[1 2 3]` | **Tuple** | Fixed-size, VM native, O(1) access, O(n) copy on modify |
| `{1 2 3}` | **Vector** | Persistent, structural sharing, O(1) append, O(log₃₂n) update |
| `%{:a 1 :b 2}` | **Map** | Key-value pairs, persistent |
| `#{1 2 3}` | **Set** | Unique elements, persistent |

**Design Rationale:**

- **Tuples `[]`** get the cleanest syntax because they're the most common structure:
  - Function parameters: `[a b c]`
  - Return values: `[:ok value]`, `[:error reason]`
  - Pattern matching: `[:user name age]`

- **Vectors `{}`** are for dynamic accumulation:
  - Building up results: `(conj {1 2} 3)` → `{1 2 3}`
  - When you need persistent data structure semantics

- **Visual metaphor:**
  - `[]` — rigid, closed → fixed-size tuples
  - `{}` — curly, expandable → dynamic vectors

---

## Binary Types

Lonala provides binary types for efficient handling of raw byte data, essential for device drivers, network protocols, and file formats.

### `binary` — Immutable Byte Sequence

An immutable, reference-counted sequence of bytes. Efficient for message passing (large binaries are shared, not copied).

```clojure
;; Literal syntax
#bytes[0x48 0x65 0x6C 0x6C 0x6F]     ; Explicit bytes
#bytes"Hello"                         ; UTF-8 encoded string
#bytes/ascii"Hello"                   ; ASCII only (error if non-ASCII)

;; Construction
(binary [0x48u8 0x65u8 0x6C 0x6C 0x6F])  ; From byte sequence
(string->binary "Hello")                  ; From string (UTF-8)
(string->binary "Hello" :ascii)           ; From string (ASCII)
```

**Operations:**

```clojure
(binary-size bin)                 ; → u64 (byte count)
(binary-ref bin offset)           ; → u8 (bounds checked)
(binary-slice bin start len)      ; → binary (zero-copy when possible)
(binary-concat bin1 bin2)         ; → binary
(binary->string bin)              ; → string (UTF-8 decode)
(binary->string bin :latin1)      ; → string (Latin-1 decode)
(binary= bin1 bin2)               ; → boolean
```

**Pattern matching:**

```clojure
(match data
  #bytes[0x89 0x50 0x4E 0x47 & rest]  ; PNG magic number
    [:png rest]
  #bytes[0xFF 0xD8 0xFF & rest]       ; JPEG magic number
    [:jpeg rest]
  _
    [:unknown data])
```

### `bytebuf` — Mutable Byte Buffer

A mutable byte buffer for I/O operations, DMA, and building binary data. **Not shareable across processes** — use `binary` for message passing.

```clojure
;; Allocation
(bytebuf-alloc size)              ; → bytebuf (zeroed)
(bytebuf-alloc-unsafe size)       ; → bytebuf (uninitialized, faster)
```

**Read operations (with endianness):**

```clojure
;; Native endianness (matches CPU)
(bytebuf-read8 buf offset)        ; → u8
(bytebuf-read16 buf offset)       ; → u16 (native endian)
(bytebuf-read32 buf offset)       ; → u32 (native endian)
(bytebuf-read64 buf offset)       ; → u64 (native endian)

;; Explicit little-endian
(bytebuf-read16-le buf offset)    ; → u16
(bytebuf-read32-le buf offset)    ; → u32
(bytebuf-read64-le buf offset)    ; → u64

;; Explicit big-endian (network byte order)
(bytebuf-read16-be buf offset)    ; → u16
(bytebuf-read32-be buf offset)    ; → u32
(bytebuf-read64-be buf offset)    ; → u64

;; Signed variants
(bytebuf-read-i8 buf offset)      ; → i8
(bytebuf-read-i16-le buf offset)  ; → i16 (little-endian)
(bytebuf-read-i32-be buf offset)  ; → i32 (big-endian)
```

**Write operations:**

```clojure
;; Native endianness
(bytebuf-write8! buf offset val)      ; val: u8
(bytebuf-write16! buf offset val)     ; val: u16 (native endian)
(bytebuf-write32! buf offset val)     ; val: u32 (native endian)
(bytebuf-write64! buf offset val)     ; val: u64 (native endian)

;; Explicit endianness
(bytebuf-write16-le! buf offset val)  ; Little-endian
(bytebuf-write16-be! buf offset val)  ; Big-endian
(bytebuf-write32-le! buf offset val)
(bytebuf-write32-be! buf offset val)
(bytebuf-write64-le! buf offset val)
(bytebuf-write64-be! buf offset val)
```

**Bulk operations:**

```clojure
(bytebuf-copy! dst dst-off src src-off len)  ; Copy between buffers
(bytebuf-fill! buf offset len val)            ; Fill with byte value
(bytebuf-size buf)                            ; → u64
(bytebuf->binary buf)                         ; → binary (immutable snapshot)
(bytebuf->binary buf offset len)              ; → binary (slice)
```

**Example: Building a network packet**

```clojure
(defn build-udp-header [src-port dst-port payload-len]
  (let [buf (bytebuf-alloc 8u64)]
    (bytebuf-write16-be! buf 0u64 src-port)
    (bytebuf-write16-be! buf 2u64 dst-port)
    (bytebuf-write16-be! buf 4u64 (+ 8u16 payload-len))  ; Length
    (bytebuf-write16-be! buf 6u64 0u16)                  ; Checksum placeholder
    (bytebuf->binary buf)))
```

### Endianness Utilities

```clojure
;; Byte swapping
(swap16 val)              ; Swap bytes in u16
(swap32 val)              ; Swap bytes in u32
(swap64 val)              ; Swap bytes in u64

;; Conditional swapping (based on platform)
(native->be16 val)        ; Convert native to big-endian
(native->le32 val)        ; Convert native to little-endian
(be->native64 val)        ; Convert big-endian to native
(le->native16 val)        ; Convert little-endian to native

;; Platform detection
(native-endian)           ; → :little or :big
```

---

## Address Types

Lonala provides distinct types for physical and virtual addresses to prevent mixing them accidentally — a common source of driver bugs.

### `paddr` — Physical Address

A physical memory address (as seen by hardware/DMA).

```clojure
;; Construction
(paddr 0x1000_0000u64)            ; From u64

;; Operations
(paddr+ addr offset)              ; Add offset → paddr
(paddr- addr1 addr2)              ; Difference → u64
(paddr->u64 addr)                 ; Extract raw value
(paddr-align addr alignment)      ; Round up to alignment
(paddr-align-down addr alignment) ; Round down to alignment
(paddr-aligned? addr alignment)   ; Check alignment

;; Comparison
(paddr= addr1 addr2)              ; Equality
(paddr< addr1 addr2)              ; Less than
(paddr<= addr1 addr2)             ; Less than or equal
```

### `vaddr` — Virtual Address

A virtual memory address (as seen by the CPU/process).

```clojure
;; Construction
(vaddr 0x4000_0000u64)            ; From u64

;; Operations (same as paddr)
(vaddr+ addr offset)              ; Add offset → vaddr
(vaddr- addr1 addr2)              ; Difference → u64
(vaddr->u64 addr)                 ; Extract raw value
(vaddr-align addr alignment)      ; Round up to alignment
(vaddr-align-down addr alignment) ; Round down to alignment
(vaddr-aligned? addr alignment)   ; Check alignment

;; Comparison
(vaddr= addr1 addr2)              ; Equality
(vaddr< addr1 addr2)              ; Less than
```

### Address Conversions

Conversions between address types require explicit operations (provided by `lona.io`):

```clojure
;; In driver context (requires appropriate capabilities)
(vaddr->paddr vaddr)              ; → paddr (page table walk)
(paddr->vaddr paddr)              ; → vaddr or nil (if mapped)
```

**Design rationale:** Making addresses first-class types with explicit conversions prevents bugs like:
- Passing a virtual address to DMA hardware (which needs physical)
- Dereferencing a physical address directly
- Accidentally mixing address spaces

### Address Arithmetic

```clojure
;; Adding offsets (returns same type)
(paddr+ base-paddr 0x1000u64)     ; → paddr
(vaddr+ base-vaddr 0x100u64)      ; → vaddr

;; Cannot add two addresses (nonsensical)
;; (paddr+ addr1 addr2) → compile error

;; Subtracting addresses (returns offset)
(paddr- end-paddr start-paddr)    ; → u64 (byte difference)

;; Cannot mix address types
;; (paddr- paddr vaddr) → compile error
```

---

## System Types

Types for interacting with processes, the seL4 kernel, and system resources.

### `pid` — Process Identifier

A process identifier containing both realm and local process identity. First-class type (not a plain tuple) for type safety.

```clojure
;; Construction
(pid realm-id local-id)           ; Create PID

;; Accessors
(pid-realm pid)                   ; → realm-id (integer)
(pid-local pid)                   ; → local-id (integer)

;; Predicates
(pid? x)                          ; → boolean
(pid= pid1 pid2)                  ; → boolean

;; Current process
(self)                            ; → pid of current process
(self-realm)                      ; → realm-id of current process
```

**Pattern matching:** PIDs can be destructured in patterns:

```clojure
(match some-pid
  (pid r p) when (= r (self-realm))
    (handle-local-process p)
  (pid r p)
    (handle-remote-process r p))

;; In receive
(receive
  [:message from-pid data] when (pid? from-pid)
    (handle-message from-pid data))
```

**Usage in messaging:**

```clojure
(send target-pid [:hello "world"])
(send (pid 5 42) [:request data])
```

### Capability Types

seL4 capabilities are first-class types in Lonala. Each capability type grants specific rights to kernel objects.

**Capability type hierarchy:**

```
cap (abstract base)
├── tcb-cap           ; Thread Control Block
├── endpoint-cap      ; IPC endpoint
├── notification-cap  ; Async notification
├── cnode-cap         ; Capability container
├── untyped-cap       ; Raw memory
├── frame-cap         ; Physical memory page
├── vspace-cap        ; Virtual address space
├── sched-context-cap ; CPU time budget (MCS)
├── irq-handler-cap   ; Interrupt handler
└── port-cap          ; I/O port (x86 only)
```

**Type predicates:**

```clojure
(cap? x)                  ; Any capability
(tcb-cap? x)              ; Thread Control Block cap
(endpoint-cap? x)         ; IPC endpoint cap
(notification-cap? x)     ; Notification cap
(cnode-cap? x)            ; CNode cap
(untyped-cap? x)          ; Untyped memory cap
(frame-cap? x)            ; Frame cap
(vspace-cap? x)           ; VSpace cap
(sched-context-cap? x)    ; Scheduling context cap
(irq-handler-cap? x)      ; IRQ handler cap
(port-cap? x)             ; I/O port cap (x86)
```

**Capability inspection:**

```clojure
(cap-type cap)            ; → :tcb :endpoint :frame etc.
(cap-rights cap)          ; → #{:read :write :grant ...}
(cap-has-right? cap :write) ; → boolean
```

**Rights representation:**

```clojure
;; Rights are sets of keywords
#{:read}                  ; Read only
#{:read :write}           ; Read-write
#{:read :write :grant}    ; Full access with delegation
#{:read :write :grant :grant-reply}  ; All rights

;; Common right combinations
:rights/read-only         ; Alias for #{:read}
:rights/read-write        ; Alias for #{:read :write}
:rights/full              ; Alias for #{:read :write :grant :grant-reply}
```

**Example: Capability operations**

```clojure
(require '[lona.kernel :as k])

;; Copy capability with reduced rights
(k/cap-copy! dest-cnode dest-slot src-cnode src-slot #{:read})

;; Mint badged capability (for IPC identification)
(k/cap-mint! dest-cnode dest-slot src-cnode src-slot #{:read :write} badge)

;; Revoke capability and all derivatives
(k/cap-revoke! cnode slot)
```

### Reference Types (Opaque Handles)

Several system operations return opaque reference types:

| Type | Constructor | Predicate | Description |
|------|-------------|-----------|-------------|
| `monitor-ref` | `(monitor pid)` | `monitor-ref?` | Process monitor reference |
| `region` | `(make-shared-region ...)` | `region?` | Shared memory region |
| `dma-buffer` | `(dma-alloc ...)` | `dma-buffer?` | DMA-capable buffer |
| `ring` | `(ring-create ...)` | `ring?` | Lock-free ring buffer |

**Monitor reference:**

```clojure
;; Create monitor
(def mref (monitor target-pid))

;; Pattern match on DOWN message
(receive
  [:DOWN mref pid reason]
    (handle-process-exit pid reason))

;; Cancel monitor
(demonitor mref)
```

**Shared memory region:**

```clojure
;; Create region
(def data-region (make-shared-region (* 1024 1024) 'my-data))

;; Access region
(region-size data-region)         ; → u64
(region-name data-region)         ; → 'my-data

;; Share with child realm
(share-region data-region child-realm-id :read-only)
```

**DMA buffer:**

```clojure
;; Allocate DMA-capable memory
(def dma-buf (dma-alloc 4096u64 :dma/coherent))

;; Access addresses
(dma-vaddr dma-buf)               ; → vaddr (for CPU access)
(dma-paddr dma-buf)               ; → paddr (for device programming)
(dma-size dma-buf)                ; → u64
```

### Message Info (`msg-info`)

Message metadata for seL4 IPC operations (used in `lona.kernel`):

```clojure
;; Construction
(msg-info label length caps)

;; Fields
(msg-info-label mi)               ; → u64 (message type tag)
(msg-info-length mi)              ; → u8 (message register count)
(msg-info-caps mi)                ; → u8 (capability transfer count)

;; Usage in kernel IPC
(k/send! endpoint (msg-info 0u64 4u8 0u8))
(k/call! endpoint (msg-info 1u64 2u8 1u8))
```

---

## Reader Macros

| Syntax | Expansion | Purpose |
|--------|-----------|---------|
| `'x` | `(quote x)` | Quote |
| `#'x` | `(var x)` | Var reference |
| `` `x `` | Syntax-quote | Template with namespace resolution |
| `~x` | Unquote | Insert value in syntax-quote |
| `~@x` | Unquote-splicing | Splice sequence in syntax-quote |
| `^{:k v}` | Metadata | Attach metadata to next form |
| `^:keyword` | Metadata shorthand | `^{:keyword true}` |
| `#(...)` | Anonymous function | `#(+ % 1)` → `(fn* [x] (+ x 1))` |
| `#_form` | Ignore | Comment out next form |
| `#bytes[...]` | Binary literal | `#bytes[0x48 0x65]` → binary |
| `#bytes"..."` | Binary from string | `#bytes"Hi"` → UTF-8 binary |
| `#bits[...]` | Bit syntax | Binary pattern/construction |

**Anonymous Function Shorthand:**
```clojure
#(+ % 1)           ; single arg
#(+ %1 %2)         ; multiple args
#(apply + %&)      ; rest args
```

### Binary Literals

```clojure
#bytes[0x48 0x65 0x6C 0x6C 0x6F]   ; Explicit byte values
#bytes"Hello"                       ; UTF-8 encoded string
#bytes/ascii"Hello"                 ; ASCII only
#bytes/latin1"Héllo"                ; Latin-1 encoded
```

### Bit Syntax

The `#bits[...]` reader macro provides Erlang-style binary pattern matching and construction. This is essential for parsing and building network protocols, file formats, and device registers.

#### Segment Syntax

Each segment in a bit pattern has the form:

```
value:size/modifiers
```

| Component | Description | Examples |
|-----------|-------------|----------|
| `value` | Literal, binding, or `_` | `0x45`, `port`, `_` |
| `:size` | Size in bits (default) or bytes | `:16`, `:32` |
| `/modifiers` | Endianness, signedness, unit | `/be`, `/le`, `/signed`, `/bytes` |

#### Modifiers

| Modifier | Description |
|----------|-------------|
| `/be` | Big-endian (network byte order) |
| `/le` | Little-endian |
| `/native` | Platform native endianness |
| `/signed` | Signed integer |
| `/unsigned` | Unsigned integer (default) |
| `/bytes` | Size is in bytes, not bits |

#### Binary Pattern Matching

```clojure
;; Parse IP header
(match packet
  #bits[version:4 ihl:4 tos:8 total-len:16/be
        id:16/be flags:3 frag-off:13/be
        ttl:8 protocol:8 checksum:16/be
        src-ip:32/be dst-ip:32/be & options]
    %{:version version
      :ihl ihl
      :total-length total-len
      :ttl ttl
      :protocol protocol
      :src src-ip
      :dst dst-ip
      :options options})

;; Parse Ethernet frame
(match frame
  #bits[dst:6/bytes src:6/bytes ethertype:16/be & payload]
    (case ethertype
      0x0800 (parse-ipv4 payload)
      0x0806 (parse-arp payload)
      _      [:unknown ethertype]))

;; Match literal prefix
(match data
  #bits[0x89 0x50 0x4E 0x47 & rest]    ; PNG magic
    [:png rest]
  #bits[0xFF 0xD8 0xFF & rest]         ; JPEG magic
    [:jpeg rest]
  _
    [:unknown data])

;; With guards
(match packet
  #bits[version:4 & _] when (= version 4)
    (parse-ipv4 packet)
  #bits[version:4 & _] when (= version 6)
    (parse-ipv6 packet))
```

#### Binary Construction

```clojure
;; Build UDP header
(def udp-header
  #bits[src-port:16/be
        dst-port:16/be
        length:16/be
        checksum:16/be])

;; Build with literal values
(def syn-flags #bits[0:6 1:1 0:1 0:8])  ; SYN flag set

;; Build DNS query header
(defn make-dns-header [id qcount]
  #bits[id:16/be
        0:1            ; QR (query)
        0:4            ; Opcode
        0:1            ; AA
        0:1            ; TC
        1:1            ; RD (recursion desired)
        0:1            ; RA
        0:3            ; Z (reserved)
        0:4            ; RCODE
        qcount:16/be   ; QDCOUNT
        0:16           ; ANCOUNT
        0:16           ; NSCOUNT
        0:16])         ; ARCOUNT
```

#### Rest Pattern

The `& rest` pattern captures remaining bytes:

```clojure
(match data
  #bits[header:4/bytes & payload]
    (process header payload))
```

#### Skip Pattern

Use `_` to match without binding:

```clojure
(match packet
  #bits[_:4 ihl:4 _:8 len:16/be & _]
    ;; Only extract ihl and len
    [ihl len])
```

---

## Bitwise Operations

Lonala provides a complete set of bitwise operations for low-level programming. These operations work with all fixed-width integer types (`u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`).

### Basic Bitwise Operations

```clojure
(bit-and a b)         ; Bitwise AND: a & b
(bit-or a b)          ; Bitwise OR: a | b
(bit-xor a b)         ; Bitwise XOR: a ^ b
(bit-not a)           ; Bitwise NOT: ~a (one's complement)

;; Variadic versions
(bit-and a b c d)     ; a & b & c & d
(bit-or a b c d)      ; a | b | c | d
(bit-xor a b c d)     ; a ^ b ^ c ^ d
```

### Shift Operations

```clojure
(bit-shl a n)         ; Shift left: a << n
(bit-shr a n)         ; Logical shift right: a >> n (zero-fill)
(bit-sar a n)         ; Arithmetic shift right (sign-extend)

;; Examples
(bit-shl 1u8 4)       ; → 16u8 (0b0001_0000)
(bit-shr 0x80u8 4)    ; → 8u8  (0b0000_1000)
(bit-sar -128i8 4)    ; → -8i8 (sign preserved)
```

### Rotate Operations

```clojure
(bit-rol a n width)   ; Rotate left within width bits
(bit-ror a n width)   ; Rotate right within width bits

;; Examples
(bit-rol 0b1100_0011u8 2 8)  ; → 0b0000_1111u8
(bit-ror 0b1100_0011u8 2 8)  ; → 0b1111_0000u8
```

### Single-Bit Operations

```clojure
(bit-test a n)        ; Test if bit n is set: (a & (1 << n)) != 0
(bit-set a n)         ; Set bit n: a | (1 << n)
(bit-clear a n)       ; Clear bit n: a & ~(1 << n)
(bit-flip a n)        ; Toggle bit n: a ^ (1 << n)

;; Examples
(bit-test 0b1010u8 1) ; → true (bit 1 is set)
(bit-test 0b1010u8 2) ; → false (bit 2 is clear)
(bit-set 0b1010u8 2)  ; → 0b1110u8
(bit-clear 0b1010u8 1); → 0b1000u8
(bit-flip 0b1010u8 0) ; → 0b1011u8
```

### Bit Field Operations

```clojure
(bit-field a start len)           ; Extract bits [start, start+len)
(bit-field-set a start len val)   ; Insert val into bit field

;; Examples
(bit-field 0xABCDu16 4 8)         ; → 0xBCu16 (bits 4-11)
(bit-field-set 0x0000u16 4 8 0xFFu8)  ; → 0x0FF0u16
```

### Bit Counting

These operations are typically hardware-accelerated:

```clojure
(bit-count a)         ; Population count (number of 1 bits)
(leading-zeros a)     ; Count leading zeros (CLZ)
(trailing-zeros a)    ; Count trailing zeros (CTZ)
(leading-ones a)      ; Count leading ones
(trailing-ones a)     ; Count trailing ones

;; Examples
(bit-count 0b1010_1010u8)    ; → 4
(leading-zeros 0b0000_1000u8) ; → 4
(trailing-zeros 0b0100_0000u8) ; → 6
```

### Byte Order Operations

```clojure
(byte-reverse16 a)    ; Reverse bytes in u16
(byte-reverse32 a)    ; Reverse bytes in u32
(byte-reverse64 a)    ; Reverse bytes in u64

;; Examples
(byte-reverse16 0x1234u16)    ; → 0x3412u16
(byte-reverse32 0x12345678u32) ; → 0x78563412u32
```

### Masking Utilities

```clojure
(mask-bits n)         ; Create mask with n low bits set
(mask-range start end) ; Create mask for bits [start, end)

;; Examples
(mask-bits 4)         ; → 0b1111 (0xF)
(mask-bits 8)         ; → 0xFF
(mask-range 4 8)      ; → 0b1111_0000 (0xF0)
```

### Example: Device Register Manipulation

```clojure
;; UART register bit definitions
(def +LCR-DLAB+  7)    ; Divisor Latch Access Bit
(def +LCR-BREAK+ 6)    ; Break Control
(def +LCR-PARITY-START+ 3)
(def +LCR-PARITY-LEN+ 3)
(def +LCR-STOP+  2)    ; Stop bits
(def +LCR-WLEN-START+ 0)
(def +LCR-WLEN-LEN+ 2)

(defn uart-configure-8n1 [base-addr]
  "Configure UART for 8 data bits, no parity, 1 stop bit"
  (let [lcr (mmio-read8 (vaddr+ base-addr 0x0Cu64))]
    (-> lcr
        ;; Set word length to 8 bits (0b11)
        (bit-field-set +LCR-WLEN-START+ +LCR-WLEN-LEN+ 0b11u8)
        ;; Clear stop bit (1 stop bit)
        (bit-clear +LCR-STOP+)
        ;; Clear parity (no parity)
        (bit-field-set +LCR-PARITY-START+ +LCR-PARITY-LEN+ 0u8)
        ;; Write back
        (->> (mmio-write8! (vaddr+ base-addr 0x0Cu64))))))

(defn uart-enable-dlab [base-addr]
  "Enable Divisor Latch Access Bit"
  (let [lcr (mmio-read8 (vaddr+ base-addr 0x0Cu64))]
    (mmio-write8! (vaddr+ base-addr 0x0Cu64)
                  (bit-set lcr +LCR-DLAB+))))
```

### Example: Flag Processing

```clojure
;; TCP flags
(def +TCP-FIN+ 0)
(def +TCP-SYN+ 1)
(def +TCP-RST+ 2)
(def +TCP-PSH+ 3)
(def +TCP-ACK+ 4)
(def +TCP-URG+ 5)

(defn tcp-flags->set [flags]
  "Convert TCP flags byte to set of keywords"
  (cond-> #{}
    (bit-test flags +TCP-FIN+) (conj :fin)
    (bit-test flags +TCP-SYN+) (conj :syn)
    (bit-test flags +TCP-RST+) (conj :rst)
    (bit-test flags +TCP-PSH+) (conj :psh)
    (bit-test flags +TCP-ACK+) (conj :ack)
    (bit-test flags +TCP-URG+) (conj :urg)))

(defn set->tcp-flags [flag-set]
  "Convert set of keywords to TCP flags byte"
  (cond-> 0u8
    (contains? flag-set :fin) (bit-set +TCP-FIN+)
    (contains? flag-set :syn) (bit-set +TCP-SYN+)
    (contains? flag-set :rst) (bit-set +TCP-RST+)
    (contains? flag-set :psh) (bit-set +TCP-PSH+)
    (contains? flag-set :ack) (bit-set +TCP-ACK+)
    (contains? flag-set :urg) (bit-set +TCP-URG+)))
```

---

## Pattern Matching

Pattern matching is the core mechanism in Lonala, replacing traditional conditionals and destructuring.

### In `match` Expressions

```clojure
(match value
  pattern1 body1
  pattern2 when guard body2
  _ default-body)
```

### In Function Definitions

```clojure
;; Single clause
(defn greet [name]
  (str "Hello, " name))

;; Multiple clauses with patterns
(defn handle
  ([[:ok x]] x)
  ([[:error e]] (panic! e)))

;; With guards
(defn classify
  ([n] when (> n 0) "positive")
  ([n] when (< n 0) "negative")
  ([_] "zero"))

;; Multi-arity
(defn foo
  ([a] a)
  ([a b] (+ a b))
  ([a b c] (+ a b c)))
```

### Pattern Syntax Summary

```clojure
;; Literals (match exactly)
42, "hello", :keyword, true, nil
42u8, 0xFFu32         ; Fixed-width integer literals

;; Binding (match anything, bind to name)
x, my-var, _ignored

;; Wildcard (match anything, no binding)
_

;; Tuple destructuring
[a b c]           ; exactly 3 elements
[head & tail]     ; 1+ elements, rest as tuple
[:ok value]       ; literal + binding

;; Vector destructuring
{a b c}           ; exactly 3 elements
{first & rest}    ; 1+ elements

;; Map destructuring
%{:name n}                    ; extract :name
%{:name n :age a}             ; extract multiple
%{:config %{:debug d}}        ; nested

;; PID destructuring
(pid realm local)             ; extract realm-id and local-id

;; Binary pattern matching
#bytes[0x89 0x50 & rest]      ; Match byte prefix
#bits[ver:4 ihl:4 & _]        ; Match bit fields

;; Guards
pattern when (guard-expr)
```

### Type-Specific Patterns

Lonala supports pattern matching on system types:

```clojure
;; PID patterns
(match pid
  (pid r l) when (= r (self-realm))
    [:local l]
  (pid r l)
    [:remote r l])

;; Binary patterns (see Reader Macros → Bit Syntax for details)
(match data
  #bytes[0x89 0x50 0x4E 0x47 & rest]
    [:png rest]
  #bits[version:4 ihl:4 & _] when (= version 4)
    [:ipv4]
  _
    [:unknown])

;; Capability type checking (via guards)
(match cap
  c when (endpoint-cap? c)
    (handle-endpoint c)
  c when (frame-cap? c)
    (handle-frame c)
  _
    (handle-other cap))
```

### Guard Expressions

Guards are boolean expressions that further constrain a pattern:

```clojure
(match x
  n when (> n 0) "positive"
  n when (< n 0) "negative"
  n when (= n 0) "zero")

(defn factorial
  ([0] 1)
  ([n] when (> n 0) (* n (factorial (- n 1)))))
```

**Guard Restrictions:**
- Must be pure (no side effects)
- Limited to safe functions (comparison, arithmetic, type checks)
- `when` is a reserved symbol in pattern context

---

## Function Definitions

### Primitive: `fn*`

Low-level function creation. Single parameter list, single body.

```clojure
(fn* [x] (+ x 1))
(fn* [a b] (+ a b))
```

### Macro: `fn`

User-facing function with pattern matching support.

```clojure
;; Single clause
(fn [x] (+ x 1))

;; Multiple clauses
(fn
  ([a] a)
  ([a b] (+ a b)))

;; With patterns
(fn
  ([[:ok x]] x)
  ([[:error e]] (panic! e)))
```

**Expansion:**
```clojure
;; This:
(fn
  ([[:ok x]] x)
  ([[:error e]] (panic! e)))

;; Expands to:
(fn* [& args]
  (match args
    [[:ok x]] x
    [[:error e]] (panic! e)))
```

### Macro: `defn`

Defines a named function.

```clojure
;; Single clause (vector after name)
(defn add [a b]
  (+ a b))

;; Multiple clauses (lists after name)
(defn factorial
  ([0] 1)
  ([n] (* n (factorial (- n 1)))))

;; With guards
(defn abs
  ([n] when (>= n 0) n)
  ([n] (- n)))

;; Multi-expression body
(defn process [data]
  (validate data)
  (transform data)
  (save data))

;; Multi-clause with multi-expression body
(defn handle
  ([[:ok x]]
    (log "success")
    (do
      (process x)
      x))
  ([[:error e]]
    (do
      (log "failure")
      (panic! e))))
```

**Syntax Rules:**
- If first form after name is a **vector** `[...]`: single clause, rest is body
- If first form after name is a **list** `(...)`: multiple clauses, each list is `(pattern body)`

---

## Error Handling

Lonala follows an Elixir-inspired error model.

### Convention: Tuple Returns

```clojure
;; Success
[:ok value]

;; Failure
[:error reason]

;; Example
(defn divide [a b]
  (if (= b 0)
    [:error :division-by-zero]
    [:ok (/ a b)]))

;; Handling
(match (divide 10 2)
  [:ok result]   (println "Result:" result)
  [:error :division-by-zero] (println "Cannot divide by zero")
  [:error other] (println "Error:" other))
```

### Crashes: Let It Crash

Unrecoverable errors crash the process:

| Error | Cause |
|-------|-------|
| `MatchError` | No pattern matched in `match` |
| `FunctionClauseError` | No function clause matched |
| `ArityError` | Wrong number of arguments |

```clojure
;; This crashes if result is neither :ok nor :error tuple
(match result
  [:ok x]    x
  [:error e] (panic! e))
;; No wildcard → MatchError if unexpected shape
```

### Philosophy

- **Don't catch match errors** — let the process crash
- **Supervisors restart** from clean state
- **Defensive coding** is discouraged
- **Good error messages** help debugging

---

## Derived Macros

These are implemented as macros over the 5 special forms.

### `if`

```clojure
(if test then else)

;; Expands to:
(match test
  false else
  nil   else
  _     then)
```

### `let`

```clojure
(let [x 1
      y 2]
  (+ x y))

;; Expands to nested match:
(match 1 x
  (match 2 y
    (+ x y)))
```

**With Destructuring:**
```clojure
(let [[a b] point
      %{:name n} user]
  ...)
```

### `cond`

```clojure
(cond
  (< n 0) "negative"
  (> n 0) "positive"
  :else   "zero")

;; Expands to nested if
```

### `case`

Alias for `match`:

```clojure
(case x
  1 "one"
  2 "two"
  _ "other")
```

### `when`

If without else (returns `nil` on false):

```clojure
(when condition
  (do-something)
  result)
```

### `and` / `or`

Short-circuit boolean operations:

```clojure
(and a b c)  ; returns first falsy or last value
(or a b c)   ; returns first truthy or last value
```

### `letfn`

Mutual recursion:

```clojure
(letfn [(even? [n] (if (= n 0) true (odd? (- n 1))))
        (odd?  [n] (if (= n 0) false (even? (- n 1))))]
  (even? 10))
```

### `->` and `->>`

Threading macros:

```clojure
(-> x
    (foo a)
    (bar b))
;; Expands to: (bar (foo x a) b)

(->> x
     (map inc)
     (filter even?))
;; Expands to: (filter even? (map inc x))
```

---

## Summary

### Special Forms (5 total)

| Form | Purpose |
|------|---------|
| `def` | Create/update root var |
| `fn*` | Create function (single arity) |
| `match` | Pattern matching |
| `do` | Sequence expressions |
| `quote` | Prevent evaluation |

### Type System Overview

```
Lonala Types
├── Scalar Types
│   ├── nil
│   ├── Boolean (true, false)
│   ├── Numbers
│   │   ├── Integer (arbitrary precision, default)
│   │   ├── BigInt (N suffix)
│   │   ├── Fixed-Width: u8, u16, u32, u64, i8, i16, i32, i64
│   │   ├── Float: f32, f64 (default)
│   │   ├── BigDecimal (M suffix)
│   │   └── Ratio (22/7)
│   ├── Character (\a, \newline)
│   └── String ("hello")
│
├── Symbolic Types
│   ├── Symbol (foo, ns/bar)
│   └── Keyword (:foo, :ns/bar, ::auto)
│
├── Collection Types
│   ├── List (1 2 3)
│   ├── Tuple [1 2 3]
│   ├── Vector {1 2 3}
│   ├── Map %{:a 1}
│   └── Set #{1 2 3}
│
├── Binary Types
│   ├── binary (immutable bytes)
│   └── bytebuf (mutable buffer)
│
├── Address Types
│   ├── paddr (physical address)
│   └── vaddr (virtual address)
│
├── System Types
│   ├── pid (process identifier)
│   ├── Capabilities
│   │   ├── tcb-cap, endpoint-cap, notification-cap
│   │   ├── cnode-cap, untyped-cap, frame-cap
│   │   ├── vspace-cap, sched-context-cap
│   │   ├── irq-handler-cap, port-cap
│   │   └── (rights: #{:read :write :grant :grant-reply})
│   └── References
│       ├── monitor-ref, region
│       ├── dma-buffer, ring
│       └── msg-info
│
└── Function Types
    └── fn (first-class functions)
```

### Literal Syntax Quick Reference

| Type | Literal | Example |
|------|---------|---------|
| Integer | decimal | `42`, `-17` |
| BigInt | `N` suffix | `999N` |
| Fixed unsigned | `u` suffix | `42u8`, `0xFFu32` |
| Fixed signed | `i` suffix | `-10i16` |
| Float (f64) | decimal | `3.14` |
| Float (f32) | `f32` suffix | `3.14f32` |
| Ratio | `/` | `22/7` |
| Binary | `#bytes` | `#bytes[0x48 0x65]` |
| Bit pattern | `#bits` | `#bits[ver:4 ihl:4]` |
| Tuple | `[]` | `[1 2 3]` |
| Vector | `{}` | `{1 2 3}` |
| Map | `%{}` | `%{:a 1}` |
| Set | `#{}` | `#{1 2 3}` |
| List | `()` | `(1 2 3)` |

### Collection Literals

| Syntax | Type | Use Case |
|--------|------|----------|
| `()` | List | Code as data, sequences |
| `[]` | Tuple | Fixed data, params, returns |
| `{}` | Vector | Accumulating, dynamic |
| `%{}` | Map | Key-value data |
| `#{}` | Set | Unique elements |
| `#bytes[]` | Binary | Byte data, protocols |

### Key Characteristics

- **5 special forms** — minimal core
- **Pattern matching** — central to the language, including binary patterns
- **Automatic TCO** — no `recur` needed
- **seL4 + custom VM** — tuples, processes, message passing
- **Let it crash** — supervisor-based fault tolerance
- **Homoiconic** — code is data, powerful macros
- **Systems programming** — fixed-width integers, addresses, capabilities
- **Driver development** — MMIO, DMA, bit manipulation, endianness control

### Standard Library Namespaces

| Namespace | Document | Purpose |
|-----------|----------|---------|
| `lona.core` | This document | Language primitives and core macros |
| `lona.process` | [lonala-process.md](lonala-process.md) | Process management, messaging, supervisors |
| `lona.kernel` | [lonala-kernel.md](lonala-kernel.md) | seL4 syscalls and capability operations |
| `lona.io` | [lonala-io.md](lonala-io.md) | Device I/O, DMA, MMIO, interrupts |

---

## Appendix: Complete Syntax Examples

```clojure
;; Function definition with pattern matching
(defn fibonacci
  ([0] 0)
  ([1] 1)
  ([n] (+ (fibonacci (- n 1))
          (fibonacci (- n 2)))))

;; Error handling
(defn safe-divide [a b]
  (if (= b 0)
    [:error :division-by-zero]
    [:ok (/ a b)]))

(defn calc [a b]
  (match (safe-divide a b)
    [:ok result]   (println "Result:" result)
    [:error :division-by-zero]
      (do
        (println "Error: division by zero")
        nil)))

;; Working with collections
(defn sum-positives [numbers]
  (reduce
    (fn [acc n]
      (if (> n 0)
        (+ acc n)
        acc))
    0
    numbers))

;; Pattern matching with guards
(defn describe-number
  ([0] "zero")
  ([n] when (> n 0) (if (even? n) "positive even" "positive odd"))
  ([n] (if (even? n) "negative even" "negative odd")))

;; Nested destructuring
(defn get-city [person]
  (match person
    %{:address %{:city c}} c
    _ "unknown"))

;; Accumulating with vectors
(defn collect-valid [items]
  (reduce
    (fn [acc item]
      (match item
        [:valid x] (conj acc x)
        [:invalid _] acc))
    {}  ; empty vector
    items))
```

### Systems Programming Examples

```clojure
;; Fixed-width integers for device registers
(def +UART-BASE+ (vaddr 0x1000_0000u64))
(def +UART-DR+   0x00u64)  ; Data Register
(def +UART-FR+   0x18u64)  ; Flag Register
(def +FR-TXFF+   0x20u8)   ; TX FIFO Full
(def +FR-RXFE+   0x10u8)   ; RX FIFO Empty

(defn uart-putc [ch]
  ;; Wait until TX FIFO not full
  (let [flags (mmio-read8 (vaddr+ +UART-BASE+ +UART-FR+))]
    (if (bit-test flags 5)  ; TXFF bit
      (uart-putc ch)        ; TCO: spin until ready
      (mmio-write8! (vaddr+ +UART-BASE+ +UART-DR+) (u8 ch)))))

;; Binary protocol parsing with bit syntax
(defn parse-ipv4-header [packet]
  (match packet
    #bits[version:4 ihl:4 dscp:6 ecn:2 total-len:16/be
          identification:16/be flags:3 frag-offset:13/be
          ttl:8 protocol:8 checksum:16/be
          src:32/be dst:32/be & options-and-data]
      (when (= version 4)
        %{:version     version
          :header-len  (* ihl 4)
          :total-len   total-len
          :ttl         ttl
          :protocol    (case protocol
                         1  :icmp
                         6  :tcp
                         17 :udp
                         _  protocol)
          :src-ip      (format-ip src)
          :dst-ip      (format-ip dst)
          :payload     (binary-slice options-and-data
                                     (* (- ihl 5) 4)
                                     (- total-len (* ihl 4)))})
    _
      [:error :not-ipv4]))

(defn format-ip [ip]
  (str (bit-shr ip 24)
       "." (bit-and (bit-shr ip 16) 0xFF)
       "." (bit-and (bit-shr ip 8) 0xFF)
       "." (bit-and ip 0xFF)))

;; Building network packets
(defn build-icmp-echo [id seq payload]
  (let [header #bits[8:8           ; Type: Echo Request
                     0:8           ; Code
                     0:16          ; Checksum placeholder
                     id:16/be      ; Identifier
                     seq:16/be]    ; Sequence
        data   (binary-concat header payload)
        csum   (internet-checksum data)]
    ;; Insert checksum at offset 2
    (bytebuf-write16-be! (binary->bytebuf data) 2u64 csum)
    (bytebuf->binary data)))

;; DMA buffer management
(defn setup-rx-descriptors [ring-size]
  (let [desc-buf (dma-alloc (* ring-size 16u64) :dma/uncached)]
    (dotimes [i ring-size]
      (let [data-buf (dma-alloc 2048u64 :dma/cached)
            offset   (* i 16u64)]
        ;; Write descriptor: [paddr:64 | len:16 | flags:16 | status:32]
        (bytebuf-write64-le! desc-buf offset (paddr->u64 (dma-paddr data-buf)))
        (bytebuf-write16-le! desc-buf (+ offset 8u64) 2048u16)
        (bytebuf-write16-le! desc-buf (+ offset 10u64) 0u16)
        (bytebuf-write32-le! desc-buf (+ offset 12u64) 0u32)))
    desc-buf))

;; Process communication with PIDs
(defn rpc-call [server-pid request timeout-ms]
  (let [ref (make-ref)]
    (send server-pid [:request (self) ref request])
    (receive
      [:response ref result]
        [:ok result]
      :after timeout-ms
        [:error :timeout])))

;; Capability-based resource sharing
(defn share-buffer-with-driver [driver-realm buf access]
  (let [frames (region-frames buf)]
    (doseq [frame frames]
      (cap-copy! (realm-cspace driver-realm)
                 (next-slot driver-realm)
                 (self-cspace)
                 (frame-slot frame)
                 (case access
                   :read-only  #{:read}
                   :read-write #{:read :write})))))
```
