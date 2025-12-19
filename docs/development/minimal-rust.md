# Minimal Rust Runtime

This document defines the principle that the Rust runtime must be minimal, with almost everything implemented in Lonala itself.

## Philosophy

> "By understanding eval you're understanding what will probably be the main model of computation well into the future." — Paul Graham

John McCarthy's 1960 paper demonstrated that an entire programming language can be built from just seven primitives. This wasn't an implementation shortcut—it was a discovery about the nature of computation itself. Lona embraces this philosophy: the Rust runtime exists only to provide what Lonala cannot provide for itself.

### Historical Foundation

**McCarthy's Original LISP (1960)** required only:
- `car`, `cdr`, `cons` — list manipulation
- `eq`, `atom` — predicates
- `quote`, `cond`, `lambda` — special forms

From these primitives, McCarthy defined `eval`—a function that interprets LISP in LISP. This self-evaluating property is what makes LISP fundamentally different from other languages.

**Paul Graham's "Roots of Lisp"** showed that with just seven operators (`quote`, `atom`, `eq`, `car`, `cdr`, `cons`, `cond`), you can define a complete evaluator. The entire language emerges from this minimal foundation.

**Femtolisp** (by Jeff Bezanson, co-creator of Julia) proves this works in practice: only 12 special forms and 33 functions in C, with "many primitives (e.g. `filter` and `for-each`) written in the language instead of C." Despite this minimalism, it ranks among the fastest non-native-compiled Scheme implementations.

### Why This Matters for Lona

1. **Security**: Every line of Rust is trusted code in the TCB. Lonala code runs with capability-based isolation. Less Rust = smaller attack surface.

2. **Flexibility**: Lonala code can be hot-patched, introspected, and modified at runtime. Rust code is frozen at compile time.

3. **Self-hosting**: The ultimate goal is a Lonala that can compile itself, modify its own compiler, and evolve without recompilation.

4. **Simplicity**: A minimal runtime is easier to verify, audit, and understand.

## What Must Be Native (Rust)

These primitives require Rust implementation because they cannot be expressed in Lonala. They fall into categories based on WHY they must be native.

---

### Category 1: Core Data Structure Operations

Operations on the internal representation of data structures.

#### List Operations

| Primitive | Purpose |
|-----------|---------|
| `cons` | Construct a pair |
| `first` | Get the first element (car) |
| `rest` | Get the remaining elements (cdr) |

#### Vector Operations

| Primitive | Purpose |
|-----------|---------|
| `nth` | Get element at index |
| `conj` | Add element to collection |
| `count` | Get collection size |

#### Map Operations

| Primitive | Purpose |
|-----------|---------|
| `get` | Get value for key |
| `assoc` | Associate key with value |
| `dissoc` | Remove key |
| `keys` | Get sequence of keys |
| `vals` | Get sequence of values |

**Note**: The *data structures themselves* (persistent vectors, HAMT maps) are Rust implementations for efficiency. The *constructor functions* (`list`, `vector`, `hash-map`) are Lonala wrappers around these primitives.

#### Binary Operations

| Primitive | Purpose |
|-----------|---------|
| `make-binary` | Allocate zeroed byte buffer |
| `binary-len` | Get buffer length |
| `binary-get` | Get byte at index |
| `binary-set` | Set byte at index |
| `binary-slice` | Zero-copy view |
| `binary-copy!` | Copy bytes between buffers |

Binary buffers are essential for network packets, device I/O, and DMA.

---

### Category 2: Type Predicates

Inspect runtime type tags that are opaque to Lonala.

| Primitive | Purpose |
|-----------|---------|
| `nil?` | Test for nil |
| `boolean?` | Test for boolean |
| `integer?` | Test for integer |
| `float?` | Test for float |
| `ratio?` | Test for ratio |
| `symbol?` | Test for symbol |
| `keyword?` | Test for keyword |
| `string?` | Test for string |
| `binary?` | Test for binary buffer |
| `list?` | Test for list/cons |
| `vector?` | Test for vector |
| `map?` | Test for hash map |
| `fn?` | Test for function |

---

### Category 3: Arithmetic & Comparison

Efficient machine-level operations.

#### Arithmetic

| Primitive | Purpose |
|-----------|---------|
| `+`, `-`, `*`, `/` | Basic arithmetic |
| `mod` | Modulo operation |

#### Comparison

| Primitive | Purpose |
|-----------|---------|
| `=` | Value equality |
| `<`, `>`, `<=`, `>=` | Numeric comparison |

#### Bitwise Operations

| Primitive | Purpose |
|-----------|---------|
| `bit-and` | Bitwise AND |
| `bit-or` | Bitwise OR |
| `bit-xor` | Bitwise XOR |
| `bit-not` | Bitwise NOT |
| `bit-shift-left` | Left shift |
| `bit-shift-right` | Right shift (arithmetic) |

Bitwise operations are essential for protocol parsing, checksums, and hardware register manipulation.

---

### Category 4: Symbol Operations

Require access to the symbol interner.

| Primitive | Purpose |
|-----------|---------|
| `symbol` | Create/intern a symbol |
| `gensym` | Generate unique symbol |

---

### Category 5: Metadata Operations

Metadata is a map attached to values that describes the value without affecting equality.

| Primitive | Purpose |
|-----------|---------|
| `meta` | Get metadata map (or nil) |
| `with-meta` | Return copy with new metadata |
| `vary-meta` | Transform metadata with function |

**Types that support metadata**: Symbol, List, Vector, Map, Var

**Types that do NOT support metadata**: nil, bool, numbers, strings, binaries

Metadata enables:
- Documentation (`:doc` key)
- Macro detection (`:macro true`)
- Source location tracking (`:file`, `:line`, `:column`)
- Private vars (`:private true`)

```clojure
(def v (with-meta [1 2 3] {:source "test"}))
(meta v)              ; => {:source "test"}
(meta #'my-fn)        ; => {:doc "...", :arglists ([x y]), :line 42, ...}
```

---

### Category 6: Hardware Access (MMIO, DMA, IRQ)

Direct hardware interaction for device drivers.

#### MMIO (Memory-Mapped I/O)

| Primitive | Purpose |
|-----------|---------|
| `peek-u8` | Read unsigned 8-bit value |
| `peek-u16` | Read unsigned 16-bit value |
| `peek-u32` | Read unsigned 32-bit value |
| `peek-u64` | Read unsigned 64-bit value |
| `poke-u8` | Write unsigned 8-bit value |
| `poke-u16` | Write unsigned 16-bit value |
| `poke-u32` | Write unsigned 32-bit value |
| `poke-u64` | Write unsigned 64-bit value |

#### DMA (Direct Memory Access)

| Primitive | Purpose |
|-----------|---------|
| `dma-alloc` | Allocate DMA-capable buffer (returns virt + phys addr) |
| `phys-addr` | Get physical address of binary buffer |
| `memory-barrier` | Ensure memory ordering for DMA coherency |

#### IRQ (Interrupt Handling)

| Primitive | Purpose |
|-----------|---------|
| `irq-wait` | Block process until interrupt fires |

**Example: UART Driver in Lonala**

```clojure
;; UART is just memory-mapped I/O registers
(def uart-base 0x09000000)
(def uart-data uart-base)
(def uart-flag (+ uart-base 0x18))

(defn uart-write-byte [b]
  (poke-u8 uart-data b))

(defn uart-read-byte []
  (peek-u8 uart-data))

;; Driver main loop
(defn uart-driver-loop []
  (loop []
    (irq-wait uart-irq-cap)
    (handle-uart-data)
    (recur)))
```

---

### Category 7: Time

| Primitive | Purpose |
|-----------|---------|
| `now-ms` | Current time in milliseconds |
| `send-after` | Send message to process after delay |

---

### Category 8: Process & Scheduler

Require deep integration with the runtime scheduler.

| Primitive | Purpose |
|-----------|---------|
| `spawn` | Create new process (allocates PCB, heap, registers with scheduler) |
| `self` | Get current process ID |
| `exit` | Exit current process |
| `send` | Send message to process mailbox |

`receive` is a **special form** (not a function) because it involves pattern matching and blocking semantics handled by the compiler.

**Note**: Higher-level process patterns (supervision trees, GenServer, call/cast) are implemented in Lonala using these primitives.

---

### Category 9: seL4 / Domain Operations

Require seL4 syscalls that cannot be made from Lonala.

| Primitive | Purpose |
|-----------|---------|
| `domain-create` | Create new domain (VSpace + CSpace) |
| `cap-grant` | Grant capability to domain |
| `cap-revoke` | Revoke capability from domain |

---

### Rust-Internal UART (Not a Lonala Primitive)

The Rust runtime has its own UART access for:
- Panic handlers
- Early boot diagnostics
- Runtime error messages before Lonala is initialized

This is **internal to Rust** and is **NOT exposed to Lonala**. Lonala implements its own UART driver using MMIO primitives.

**Boot sequence:**
1. Rust runtime initializes, uses its internal UART for early diagnostics
2. Rust hands control to Lonala init system
3. Lonala init loads the UART driver (written in Lonala)
4. Lonala init spawns the REPL (written in Lonala)
5. From this point, all I/O goes through Lonala drivers

---

### Special Forms (Compiler, not Runtime)

These are handled by the compiler, not as runtime functions:

| Form | Purpose |
|------|---------|
| `quote` | Prevent evaluation |
| `if` | Conditional |
| `fn` | Create function |
| `def` | Define global |
| `do` | Sequence expressions |
| `defmacro` | Define macro |
| `let` | Local bindings |
| `receive` | Pattern-matched message receive |

## What Must Be Lonala

Everything else, without exception:

### Macros

All control flow macros:
- `defn` — define function (expands to `def` + `fn`)
- `when`, `unless` — conditional execution
- `cond` — multi-way conditional
- `and`, `or` — short-circuit boolean
- `->`, `->>` — threading macros
- `let` bindings beyond the primitive form

### Collection Constructors

- `list` — create list (uses `cons` internally)
- `vector` — create vector
- `hash-map` — create map

These can be implemented as functions that call `cons` or allocate via primitives.

### Sequence Operations

- `map`, `filter`, `reduce` — higher-order sequence functions
- `concat`, `append` — join sequences
- `nth`, `count`, `empty?` — access and query
- `take`, `drop`, `partition` — subsequences

All implementable with `first`, `rest`, `cons`, and recursion.

### Higher-Order Functions

- `apply` — apply function to argument list
- `comp` — compose functions
- `partial` — partial application
- `identity`, `constantly` — utility functions

### The REPL

The read-eval-print loop itself:
- `read` — parse input (uses `uart-read-byte`)
- `eval` — evaluate expression
- `print` — output result (uses `uart-write-byte`)
- The loop that ties them together

### String Operations

All string manipulation beyond raw bytes:
- `str` — concatenate to string
- `subs` — substring
- `split`, `join` — string splitting/joining
- `format` — formatted output

### Device Drivers (ALL of them)

Using `peek`, `poke`, and interrupt handling:
- **UART driver** — serial console I/O
- Keyboard driver
- Display driver
- Storage driver
- Network driver

Every device driver is implemented in Lonala. The Rust runtime has internal UART access for panics and early boot, but this is not exposed as a primitive—it's purely for runtime debugging before Lonala takes over.

### Process Patterns (Built on Native Primitives)

Higher-level process abstractions built on `spawn`, `send`, `receive`:
- Supervision trees (`one-for-one`, `one-for-all`, `rest-for-one` strategies)
- GenServer pattern (`call`, `cast`, `handle_call`, `handle_cast`)
- Process linking and monitoring helpers
- Named process registry

### Evaluation

The `eval` function can be implemented in Lonala:
```clojure
(defn eval [form]
  (vm/load (compiler/compile form)))
```

This enables runtime code evaluation, REPLs, and dynamic code loading—all in Lonala.

## Current Interim Code

The following Rust implementations are **temporary scaffolding**:

| Module | Location | Future |
|--------|----------|--------|
| REPL | `lona-runtime/src/repl.rs` | Replace with Lonala REPL |
| Collections | `lona-kernel/src/vm/collections/` | Replace with Lonala implementations |
| Introspection | `lona-kernel/src/vm/introspection.rs` | Replace with Lonala implementations |

When working on these modules, remember they will be deleted once Lonala is capable enough to replace them.

## Complete Native Primitive Summary

| Category | Primitives | Count |
|----------|------------|-------|
| **List Ops** | `cons`, `first`, `rest` | 3 |
| **Vector Ops** | `nth`, `conj`, `count` | 3 |
| **Map Ops** | `get`, `assoc`, `dissoc`, `keys`, `vals` | 5 |
| **Binary Ops** | `make-binary`, `binary-len`, `binary-get`, `binary-set`, `binary-slice`, `binary-copy!` | 6 |
| **Type Predicates** | `nil?`, `boolean?`, `integer?`, `float?`, `ratio?`, `symbol?`, `keyword?`, `string?`, `binary?`, `list?`, `vector?`, `map?`, `fn?` | 13 |
| **Arithmetic** | `+`, `-`, `*`, `/`, `mod` | 5 |
| **Comparison** | `=`, `<`, `>`, `<=`, `>=` | 5 |
| **Bitwise** | `bit-and`, `bit-or`, `bit-xor`, `bit-not`, `bit-shift-left`, `bit-shift-right` | 6 |
| **Symbol** | `symbol`, `gensym` | 2 |
| **Metadata** | `meta`, `with-meta`, `vary-meta` | 3 |
| **MMIO** | `peek-u8`, `peek-u16`, `peek-u32`, `peek-u64`, `poke-u8`, `poke-u16`, `poke-u32`, `poke-u64` | 8 |
| **DMA** | `dma-alloc`, `phys-addr`, `memory-barrier` | 3 |
| **IRQ** | `irq-wait` | 1 |
| **Time** | `now-ms`, `send-after` | 2 |
| **Process** | `spawn`, `self`, `exit`, `send` | 4 |
| **Domain** | `domain-create`, `cap-grant`, `cap-revoke` | 3 |
| **I/O** | `print` (temporary) | 1 |
| **TOTAL** | | **73** |

**Special Forms** (compiler, not runtime): `quote`, `if`, `fn`, `def`, `do`, `defmacro`, `let`, `receive`

---

## Decision Checklist

Before adding ANY native function, answer these questions:

1. **Can this be implemented using existing primitives?**
   - If yes → **Implement in Lonala**
   - This includes anything buildable from cons/first/rest, arithmetic, or memory access

2. **Does this require inspecting runtime type tags?**
   - If yes → Native is acceptable (type predicates)
   - If no → **Implement in Lonala**

3. **Does this require direct hardware access?**
   - If yes → Native is acceptable (memory, raw I/O)
   - If no → **Implement in Lonala**

4. **Is this purely for performance?**
   - **Implement in Lonala first**
   - Only move to native after profiling proves it's a bottleneck
   - Femtolisp proves this approach works

5. **Does this require access to the symbol interner?**
   - If yes → Native is acceptable
   - If no → **Implement in Lonala**

**The default answer is always: Lonala.**

## Examples

### Good: Native Primitive

```rust
// peek - reads from arbitrary memory address
// This REQUIRES native implementation because Lonala
// cannot directly access memory addresses
fn native_peek(addr: Value) -> Result<Value, Error> {
    let ptr = addr.as_integer()? as *const u8;
    Ok(Value::Integer(unsafe { *ptr } as i64))
}
```

### Bad: Native Implementation of Derivable Function

```rust
// DON'T DO THIS
// `map` can be implemented in Lonala using first, rest, cons
fn native_map(f: Value, coll: Value) -> Result<Value, Error> {
    // ... Rust implementation ...
}
```

```clojure
;; DO THIS INSTEAD
(defn map [f coll]
  (if (nil? coll)
    nil
    (cons (f (first coll))
          (map f (rest coll)))))
```

### Bad: Native Collection Constructor

```rust
// DON'T DO THIS
// `list` can be implemented using cons
fn native_list(args: &[Value]) -> Value {
    // ... build list in Rust ...
}
```

```clojure
;; DO THIS INSTEAD
;; (list) is just syntax sugar the compiler can handle,
;; or a simple function:
(defn list [& args] args)  ; rest args already produce a list
```

## References

- McCarthy, John. "Recursive Functions of Symbolic Expressions and Their Computation by Machine, Part I." CACM, April 1960.
- Graham, Paul. "The Roots of Lisp." 2002. https://paulgraham.com/rootsoflisp.html
- Bezanson, Jeff. Femtolisp. https://github.com/JeffBezanson/femtolisp
- Dybvig, R. Kent. "Three Implementation Models for Scheme." PhD thesis, 1987.
