# Data Types
Lonala is dynamically typed. Every value has a runtime type. This section describes the semantics of each type.

## 3.1 Type Hierarchy

```
Value
├── Nil
├── Bool
├── Number
│   ├── Integer (arbitrary precision)
│   ├── Float (64-bit IEEE 754)
│   └── Ratio (exact fractions)
├── Symbol
├── Keyword
├── String
├── Binary (raw byte buffer)
├── Collection
│   ├── List (linked, immutable)
│   ├── Vector (indexed, immutable)
│   ├── Map (associative, immutable)
│   └── Set (unordered, unique elements, immutable)
└── Function
```

## 3.2 Nil

`nil` represents the absence of a value. It is the only value of its type.

- **Truthiness**: `nil` is falsy
- **Equality**: `nil` equals only itself
- **Use cases**: Missing values, empty results, uninitialized state

```clojure
nil           ; the nil value
(= nil nil)   ; => true
(= nil false) ; => false
```

## 3.3 Bool

Booleans represent logical truth values. There are exactly two boolean values: `true` and `false`.

- **Truthiness**: `false` is falsy; `true` is truthy
- **Equality**: Booleans equal only themselves

```clojure
true          ; logical true
false         ; logical false
(= true true) ; => true
(= true 1)    ; => false (no type coercion)
```

## 3.4 Numbers

Lonala supports three numeric types with automatic promotion where appropriate.

### 3.4.1 Integer

Arbitrary-precision integers with no fixed size limit. Small integers (fitting in 63 bits) are stored inline; larger integers use heap allocation.

```clojure
42                          ; small integer
-17                         ; negative integer
9999999999999999999999999   ; big integer (arbitrary precision)
```

**Operations**: Integers support all arithmetic operations. Division of integers that doesn't produce a whole number yields a Ratio.

### 3.4.2 Float

64-bit IEEE 754 double-precision floating-point numbers.

```clojure
3.14          ; decimal float
-0.5          ; negative float
1e10          ; scientific notation
1.5e-3        ; 0.0015
##Inf         ; positive infinity
##-Inf        ; negative infinity
##NaN         ; not a number
```

**Special values**:
- `##Inf` — positive infinity
- `##-Inf` — negative infinity
- `##NaN` — not a number (note: `(= ##NaN ##NaN)` is `false` per IEEE 754)

### 3.4.3 Ratio

Exact rational numbers represented as a numerator and denominator. Ratios are automatically normalized (reduced to lowest terms) and the denominator is always positive.

```clojure
1/3           ; one third
-2/4          ; normalized to -1/2
22/7          ; approximation of pi
```

**Operations**: Arithmetic on ratios produces exact results. Mixing ratios with floats promotes to float.

## 3.5 Symbol

Symbols are identifiers used to name things. They are interned (deduplicated) for fast equality comparison.

```clojure
foo           ; simple symbol
+             ; operator symbol
empty?        ; predicate symbol (conventionally ends with ?)
set!          ; mutating operation (conventionally ends with !)
my-var        ; hyphenated symbol
ns/name       ; qualified symbol (namespace/name)
```

**Symbol naming rules**:
- Must not begin with a digit
- May contain: alphanumerics, `*`, `+`, `!`, `-`, `_`, `'`, `?`, `<`, `>`, `=`, `/`
- The `/` character separates namespace from name in qualified symbols

## 3.6 Keyword

Keywords begin with a colon and evaluate to themselves. They are typically used as map keys and for enumeration values.

```clojure
:foo              ; simple keyword
:bar-baz          ; with hyphen
:ns/qualified     ; qualified keyword
```

**Characteristics**:
- **Self-evaluating**: Keywords evaluate to themselves
- **Interned**: Fast equality comparison via identity
- **Common use**: Map keys, option flags, enumeration values

## 3.7 String

Immutable sequences of UTF-8 encoded characters.

```clojure
"hello"                  ; simple string
"line1\nline2"           ; with escape sequence
"say \"hello\""          ; with escaped quotes
""                       ; empty string
```

**Escape sequences**:

| Sequence | Meaning |
|----------|---------|
| `\\` | Backslash |
| `\"` | Double quote |
| `\n` | Newline |
| `\r` | Carriage return |
| `\t` | Tab |

## 3.8 Binary

Raw byte buffers for efficient binary data handling. Binary is the **only mutable type** in Lonala, designed for high-performance device drivers, network stacks, and DMA operations.

### 3.8.1 Ownership Model

Binary implements an ownership system to enable safe concurrent access without locking overhead:

```
┌─────────────────────────────────────────────────────────────────┐
│ BinaryBuffer (shared, reference-counted)                        │
│ ├── data: bytes           # The actual byte buffer              │
│ └── phys_addr: address    # Physical address (for DMA)          │
└─────────────────────────────────────────────────────────────────┘
         ▲                    ▲
         │ Owned              │ View (read-only)
         │                    │
┌────────┴────────┐   ┌───────┴────────┐
│ Binary (owner)  │   │ Binary (view)  │
│ - can read      │   │ - can read     │
│ - can write     │   │ - cannot write │
│ - can transfer  │   │ - cannot xfer  │
└─────────────────┘   └────────────────┘
```

**Access Modes**:

| Mode | Read | Write | Create View | Transfer |
|------|:----:|:-----:|:-----------:|:--------:|
| **Owned** | ✓ | ✓ | ✓ | ✓* |
| **View** | ✓ | ✗ | ✓ | ✗ |

*Transfer only succeeds if no other references exist.

### 3.8.2 Creating Binaries

```clojure
;; Create a new binary - caller becomes owner
(def buf (make-binary 1024))  ; 1024 zeroed bytes, Owned

;; Check ownership
(binary-owner? buf)           ; => true
```

Only `make-binary` and receiving a `binary-transfer!` create Owned binaries.

### 3.8.3 Reading and Writing

```clojure
;; Read a byte (works for Owned or View)
(binary-get buf 0)            ; => 0 (byte at index 0)

;; Write a byte (Owned only)
(binary-set buf 0 0xFF)       ; sets byte at index 0 to 255

;; Get length
(binary-len buf)              ; => 1024

;; Write to a View - ERROR
(def view (binary-view buf))
(binary-set view 0 0xFF)      ; => {:error :read-only}
```

### 3.8.4 Slicing and Views

Slicing creates zero-copy views into the same underlying buffer:

```clojure
;; Create a slice - inherits access mode
(def slice (binary-slice buf 10 100))  ; bytes 10-109, Owned
(binary-set slice 0 0xAB)              ; OK - modifies buf[10]

;; Create explicit read-only view
(def view (binary-view buf))           ; View
(binary-set view 0 1)                  ; ERROR: :read-only

;; View of a slice
(def view-slice (binary-slice view 0 50))  ; View (inherits from view)
```

**Slicing semantics**:
- `binary-slice` of Owned → Owned (can write to slice, affects parent)
- `binary-slice` of View → View
- `binary-view` always creates View regardless of source

### 3.8.5 Cloning Behavior

When a Binary is cloned (copied within a process), **the clone is always a View**:

```clojure
(def buf (make-binary 100))   ; Owned
(let [local buf]              ; 'local' is a View of same buffer
  (binary-set local 0 1))     ; ERROR: :read-only
```

This prevents accidental dual ownership within a process.

### 3.8.6 Ownership Transfer

To transfer ownership to another process, use explicit transfer:

```clojure
;; Transfer ownership - buf becomes invalid
(binary-transfer! other-pid buf)

;; buf is now in "zombie" state - any operation errors
(binary-len buf)              ; => {:error :transferred}

;; other-pid receives message: {:binary-transfer <owned-binary>}
```

**Transfer requirements**:
- Binary must be Owned
- No other references can exist (no views, no clones)
- After transfer, the original binary enters "zombie" state

**Transfer failure cases**:
```clojure
;; Trying to transfer a View
(def view (binary-view buf))
(binary-transfer! pid view)   ; => {:error :not-owner}

;; Trying to transfer with outstanding references
(def buf (make-binary 100))
(def view (binary-view buf))  ; Creates another reference
(binary-transfer! pid buf)    ; => {:error :references-exist}
```

### 3.8.7 Message Passing Semantics

When a Binary is sent in a message (not transferred), it becomes a View:

```clojure
;; Sending in a message - automatic View conversion
(send other-pid {:data buf})
;; other-pid receives {:data <view-of-buf>}
;; buf remains Owned in sender
```

For explicit ownership transfer, use `binary-transfer!`.

### 3.8.8 Copying Bytes

```clojure
;; Copy bytes between binaries
(binary-copy! dst dst-offset src src-offset length)

;; dst must be Owned, src can be Owned or View
(def dst (make-binary 100))
(def src (make-binary 50))
(binary-copy! dst 10 src 0 50)  ; Copy all of src to dst[10..60]
```

### 3.8.9 Concurrent Access

Binary provides **no locking or synchronization**. This is intentional for maximum performance in device drivers.

**Programmer responsibility**:
- Don't write to a buffer while another process is reading
- Use message passing to coordinate access
- For shared buffers, establish clear ownership protocols

**Safe patterns**:
```clojure
;; Pattern 1: Sequential handoff
(defn producer []
  (let [buf (make-binary 1024)]
    (fill-buffer buf)
    (binary-transfer! consumer-pid buf)))  ; Consumer now owns it

;; Pattern 2: Read-only sharing
(defn share-data []
  (let [buf (make-binary 1024)]
    (fill-buffer buf)
    (send reader1 {:data buf})    ; Gets View
    (send reader2 {:data buf})    ; Gets View
    ;; Don't write to buf while readers are using it!
    ))

;; Pattern 3: Explicit coordination
(defn coordinated-access []
  (let [buf (make-binary 1024)]
    (fill-buffer buf)
    (send worker {:data buf :reply-to (self)})
    (receive
      {:done _} (continue-writing buf))))
```

### 3.8.10 DMA Buffers

For device drivers requiring DMA-capable memory:

```clojure
;; Allocate DMA-capable buffer (physical address tracked)
(def dma-buf (dma-alloc 4096))

;; Get physical address for hardware
(phys-addr dma-buf)           ; => physical memory address

;; Use memory barrier for DMA coherency
(memory-barrier)
```

DMA buffers are fixed-size and cannot be resized (reallocation would invalidate the physical address).

### 3.8.11 Characteristics Summary

| Property | Value |
|----------|-------|
| **Mutability** | Mutable (only mutable type in Lonala) |
| **Elements** | Unsigned 8-bit integers (0-255) |
| **Size** | Fixed at creation (no resize) |
| **Slicing** | Zero-copy, shares underlying buffer |
| **Ownership** | Owned (read/write) or View (read-only) |
| **Cloning** | Always produces View |
| **Equality** | Content-based (if not zombie) |
| **Hashing** | Not supported (mutable types shouldn't be map keys) |
| **Concurrency** | No locking (programmer responsibility) |

### 3.8.12 Operations Summary

| Operation | Description | Owned | View |
|-----------|-------------|:-----:|:----:|
| `(make-binary size)` | Create zeroed buffer | returns Owned | - |
| `(binary-len buf)` | Get length in bytes | ✓ | ✓ |
| `(binary-get buf idx)` | Get byte at index | ✓ | ✓ |
| `(binary-set buf idx val)` | Set byte at index | ✓ | error |
| `(binary-slice buf start len)` | Zero-copy slice | → Owned | → View |
| `(binary-view buf)` | Create read-only view | → View | → View |
| `(binary-copy! dst do src so len)` | Copy bytes | ✓ (dst) | error (dst) |
| `(binary-owner? buf)` | Check if Owned | ✓ | ✓ |
| `(binary-transfer! pid buf)` | Transfer ownership | ✓* | error |

*Only if no other references exist.

## 3.9 List

Immutable singly-linked lists. Lists are the fundamental data structure for code representation.

```clojure
()            ; empty list
(1 2 3)       ; list of integers
(+ 1 2)       ; when evaluated, a function call
'(a b c)      ; quoted list (data, not code)
```

**Characteristics**:
- **Immutable**: Operations return new lists
- **Structural sharing**: Efficient memory use through shared tails
- **Access**: O(1) first element, O(n) for nth element

## 3.10 Vector

Immutable indexed collections with efficient random access.

```clojure
[]            ; empty vector
[1 2 3]       ; vector of integers
["a" "b" "c"] ; vector of strings
[[1 2] [3 4]] ; nested vectors
```

**Characteristics**:
- **Immutable**: Operations return new vectors
- **Indexed**: O(log32 n) access to any element
- **Structural sharing**: Efficient updates through tree structure

## 3.11 Map

Immutable associative collections mapping keys to values.

```clojure
{}                    ; empty map
{:a 1 :b 2}           ; keyword keys (common)
{"name" "Alice"}      ; string keys
{1 "one" 2 "two"}     ; integer keys
```

**Characteristics**:
- **Immutable**: Operations return new maps
- **Any key type**: Keys can be any value that supports equality
- **O(log32 n)**: Lookup, insertion, and update

## 3.12 Set

Immutable unordered collections of unique elements.

```clojure
#{}                   ; empty set
#{1 2 3}              ; set of integers
#{:a :b :c}           ; set of keywords
#{1 "two" :three}     ; mixed types
```

**Characteristics**:
- **Immutable**: Operations return new sets
- **Unique elements**: Duplicates are automatically removed
- **Unordered**: No guaranteed iteration order (use `sorted-set` for ordering)
- **O(log32 n)**: Membership test, insertion, and removal
- **Any element type**: Elements can be any value that supports equality

```clojure
#{1 2 2 3}            ; => #{1 2 3} (duplicate removed)
(conj #{1 2} 3)       ; => #{1 2 3}
(disj #{1 2 3} 2)     ; => #{1 3}
(contains? #{1 2} 1)  ; => true
```

## 3.13 Function

First-class callable values created with `fn`.

```clojure
(fn [x] (* x x))           ; anonymous function
(fn square [x] (* x x))    ; named function (for recursion/debugging)
```

**Characteristics**:
- **First-class**: Can be passed as arguments, returned from functions, stored in collections
- **Multi-arity**: Functions support multiple arities with dispatch based on argument count
- **Variadic**: Rest parameters via `& rest` syntax
- **Identity**: Functions are compared by identity, not structure
- **No closures yet**: Functions cannot capture lexical environment *(closures planned)*

## 3.14 Truthiness

Lonala uses a simple truthiness model:

| Value | Truthiness |
|-------|------------|
| `nil` | Falsy |
| `false` | Falsy |
| Everything else | Truthy |

This includes: `true`, all numbers (including `0` and `0.0`), all strings (including `""`), all collections (including empty ones), and all functions.

```clojure
(if nil "yes" "no")     ; => "no"
(if false "yes" "no")   ; => "no"
(if 0 "yes" "no")       ; => "yes" (0 is truthy!)
(if "" "yes" "no")      ; => "yes" (empty string is truthy!)
(if [] "yes" "no")      ; => "yes" (empty vector is truthy!)
```

## 3.15 Equality

Lonala uses deep structural equality with Clojure-style semantics:

```clojure
(= 1 1)                 ; => true
(= "abc" "abc")         ; => true
(= [1 2 3] [1 2 3])     ; => true
(= {:a 1} {:a 1})       ; => true
```

### 3.15.1 Numeric Equality

Numbers of different types are equal if they represent the same mathematical value:

```clojure
(= 1 1.0)               ; => true (integer equals float)
(= 1 1/1)               ; => true (integer equals ratio)
(= 2.0 4/2)             ; => true (float equals ratio)
```

This applies recursively within collections:

```clojure
(= [1] [1.0])           ; => true
(= {:a 1} {:a 1.0})     ; => true
```

### 3.15.2 Sequential Equality

Lists and vectors belong to the same "sequential" partition. Two sequences are equal if they have the same elements in the same order, regardless of concrete type:

```clojure
(= [1 2 3] '(1 2 3))    ; => true (vector equals list)
(= '(1 2) [1 2])        ; => true (list equals vector)
```

This follows Clojure semantics where sequential collections are compared by their contents, not their type.

### 3.15.3 Special Cases

- **NaN**: `##NaN` is not equal to anything, including itself: `(= ##NaN ##NaN)` is `false`
- **NaN in collections**: `(= [##NaN] [##NaN])` is `false` (elements compared with `=`)
- **Functions**: Compared by identity (same object), not structure
- **Symbols**: Compared by identity (interned)

## 3.16 Metadata *(Planned)*

Metadata is a map of data *about* a value, attached to the value without affecting its identity or equality. Two values that differ only in metadata are still equal.

**What Supports Metadata**:
- Symbols
- Lists
- Vectors
- Maps
- Vars (the binding between a symbol and its value)

**Primitives and scalars (nil, booleans, numbers, strings, binaries) do NOT support metadata.**

### 3.16.1 Reading Metadata

```clojure
(meta obj)              ; => metadata map or nil
```

### 3.16.2 Attaching Metadata

```clojure
;; Create new value with metadata (immutable - original unchanged)
(with-meta [1 2 3] {:source "user"})

;; Transform existing metadata
(vary-meta obj assoc :new-key value)
```

### 3.16.3 Reader Syntax

```clojure
;; Full metadata map
^{:doc "A vector" :private true} [1 2 3]

;; Shorthand for ^{:keyword true}
^:private my-var

;; Multiple metadata items
^:private ^:dynamic my-var
;; equivalent to: ^{:private true :dynamic true} my-var
```

### 3.16.4 Var Metadata

Vars (created by `def`) carry metadata separate from their value:

```clojure
(def my-var 42)
(meta #'my-var)         ; => {:name my-var, :ns user, :line 1, ...}

;; Docstrings become :doc metadata
(def my-var "Documentation here" 42)
(meta #'my-var)         ; => {:doc "Documentation here", ...}
```

### 3.16.5 Standard Metadata Keys

| Key | Set By | Purpose |
|-----|--------|---------|
| `:doc` | User/defn | Documentation string |
| `:arglists` | defn/defmacro | List of argument vectors |
| `:macro` | defmacro | `true` if this var names a macro |
| `:private` | User | `true` for namespace-private vars |
| `:file` | Compiler | Source file path |
| `:line` | Compiler | Source line number |
| `:column` | Compiler | Source column number |
| `:name` | Compiler | Simple symbol name |
| `:ns` | Compiler | Namespace symbol |

### 3.16.6 Metadata and Equality

Metadata does NOT affect equality or hash codes:

```clojure
(= [1 2 3] (with-meta [1 2 3] {:foo :bar}))  ; => true

(= (hash [1 2 3])
   (hash (with-meta [1 2 3] {:foo :bar})))   ; => true
```

This allows metadata to annotate values without changing program semantics.

## 3.17 Error Tuples

Lonala uses **tagged tuples** for error handling rather than exceptions. This approach makes error handling explicit, composable, and aligns with the Erlang/BEAM philosophy.

### 3.17.1 The Result Convention

Functions that can fail return one of two tagged tuples:

| Tuple | Meaning |
|-------|---------|
| `{:ok value}` | Success with result `value` |
| `{:error reason}` | Failure with `reason` (typically a keyword or map) |

```clojure
;; Function that can fail
(defn read-file [path]
  (if (file-exists? path)
    {:ok (slurp-bytes path)}
    {:error :not-found}))

;; Caller handles both cases explicitly
(case (read-file "/etc/config")
  {:ok contents}   (parse-config contents)
  {:error :not-found} (use-default-config)
  {:error reason}  (log-and-fail reason))
```

### 3.17.2 Error Reasons

Error reasons should be descriptive and structured:

```clojure
;; Simple keyword for common cases
{:error :not-found}
{:error :timeout}
{:error :permission-denied}

;; Map for rich context
{:error {:type :validation
         :field :email
         :message "Invalid format"}}

;; Nested for error chains
{:error {:type :io-error
         :cause {:type :timeout :ms 5000}
         :operation :read}}
```

### 3.17.3 Why Not Exceptions?

Lonala deliberately omits exception-based error handling (`try`/`catch`/`throw`) for these reasons:

1. **Explicit over implicit**: Tagged tuples make failure modes visible in the code
2. **Composable**: Errors can be transformed, wrapped, and chained using standard functions
3. **No hidden control flow**: Every function call either returns or the process terminates
4. **BEAM alignment**: Matches Erlang/Elixir idioms and "let it crash" philosophy
5. **Performance**: No stack unwinding machinery in the VM

For truly unrecoverable errors, see [Process Termination](special-forms.md#69-process-termination-planned).

---
