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

## 3.6 Keyword *(Planned)*

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

> **Note**: Keywords are currently parsed but not yet represented as values. Full keyword semantics are planned for a future phase.

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

## 3.8 Binary *(Planned)*

Raw byte buffers for efficient binary data handling. Used for network packets, file I/O, and DMA buffers.

```clojure
(make-binary 1024)        ; allocate 1024-byte buffer (zeroed)
(binary-get buf 0)        ; get byte at index 0
(binary-set buf 0 0xFF)   ; set byte at index 0
(binary-slice buf 10 20)  ; zero-copy view of bytes 10-19
(binary-len buf)          ; => 1024
```

**Characteristics**:
- **Mutable**: Unlike other Lonala types, binaries can be modified in place for efficiency
- **Raw bytes**: Each element is an unsigned 8-bit integer (0-255)
- **Zero-copy slicing**: Slices share underlying memory
- **DMA-capable**: Can be allocated for hardware DMA with physical address access

**Use cases**:
- Network packet parsing and construction
- Device driver buffers
- Binary file I/O
- Memory-mapped I/O data

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

## 3.12 Set *(Planned)*

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

Lonala uses structural equality for most types:

```clojure
(= 1 1)                 ; => true
(= "abc" "abc")         ; => true
(= [1 2 3] [1 2 3])     ; => true
(= {:a 1} {:a 1})       ; => true
```

**Special cases**:
- Numbers of different types can be equal if they represent the same value: `(= 1 1.0)` is `true`
- `##NaN` is not equal to anything, including itself
- Functions are compared by identity (same object), not structure

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

For truly unrecoverable errors, see [Process Termination](special-forms.md#68-process-termination).

---

