# Lonala Language Specification

**Version**: 0.4.3 (Phase 4 - Macros Complete)

Lonala is the programming language for the Lona operating system. It combines Clojure's elegant syntax and immutable data structures with Erlang's actor-based concurrency model, designed to run on seL4's capability-based microkernel.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Lexical Structure](#2-lexical-structure)
3. [Data Types](#3-data-types)
4. [Literals](#4-literals)
5. [Symbols and Evaluation](#5-symbols-and-evaluation)
6. [Special Forms](#6-special-forms)
7. [Operators](#7-operators)
8. [Functions](#8-functions)
9. [Built-in Functions](#9-built-in-functions)
10. [Reader Macros](#10-reader-macros)
11. [Macros](#11-macros) *(Partial)*
12. [Namespaces](#12-namespaces) *(Planned)*
13. [Concurrency](#13-concurrency) *(Planned)*
14. [Appendices](#appendices)

---

## 1. Introduction

### 1.1 Overview

Lonala (/loˈnaːla/) is a dynamically-typed, functional programming language designed for systems programming. It serves as the sole programming language for the Lona operating system—everything from device drivers to user applications is written in Lonala.

### 1.2 Design Influences

Lonala draws from three major traditions:

| Influence | What Lonala Takes |
|-----------|-------------------|
| **Clojure** | S-expression syntax, immutable persistent data structures, sequence abstraction |
| **Erlang/BEAM** | Lightweight processes, message passing, supervision trees, "let it crash" philosophy |
| **Common Lisp** | Condition/restart system, runtime introspection, hot-patching |

### 1.3 Key Characteristics

- **Homoiconic**: Code is represented as data structures (lists), enabling powerful metaprogramming
- **Dynamically typed**: Types are checked at runtime, not compile time
- **Immutable by default**: Data structures cannot be modified after creation
- **Functional**: Functions are first-class values; recursion is the primary iteration mechanism
- **Concurrent**: Lightweight processes communicate via message passing (planned)

### 1.4 Quick Start

```clojure
;; Define a variable
(def greeting "Hello, Lona!")

;; Define a function
(fn square [x] (* x x))

;; Use let for local bindings
(let [x 10
      y 20]
  (+ x y))  ; => 30

;; Conditionals
(if (> x 5)
  "big"
  "small")

;; Sequences of expressions
(do
  (print "Step 1")
  (print "Step 2")
  "done")
```

### 1.5 How to Read This Document

- **Syntax notation**: `(form arg1 arg2)` shows the structure of expressions
- **Optional elements**: `arg?` means the argument is optional
- **Repeated elements**: `args*` means zero or more; `args+` means one or more
- **Alternatives**: `a | b` means either `a` or `b`
- **Planned features**: Sections marked *(Planned)* describe future functionality

---

## 2. Lexical Structure

### 2.1 Character Set

Lonala source code is UTF-8 encoded. All Unicode characters are valid in strings and comments. Identifiers (symbols) are restricted to a subset of characters.

### 2.2 Whitespace

The following characters are treated as whitespace and serve to separate tokens:

| Character | Name | Code Point |
|-----------|------|------------|
| ` ` | Space | U+0020 |
| `\t` | Tab | U+0009 |
| `\n` | Newline | U+000A |
| `\r` | Carriage Return | U+000D |
| `,` | Comma | U+002C |

Commas are whitespace in Lonala, allowing their optional use for readability:

```clojure
[1, 2, 3]      ; equivalent to [1 2 3]
{:a 1, :b 2}   ; equivalent to {:a 1 :b 2}
```

### 2.3 Comments

Comments begin with a semicolon (`;`) and extend to the end of the line:

```clojure
; This is a comment
(def x 42)  ; inline comment
```

### 2.4 Token Categories

Lonala recognizes the following token types:

| Category | Examples |
|----------|----------|
| Delimiters | `(` `)` `[` `]` `{` `}` |
| Numbers | `42` `-17` `3.14` `1/3` `0xFF` |
| Strings | `"hello"` `"line\nbreak"` |
| Symbols | `foo` `+` `empty?` `ns/name` |
| Keywords | `:foo` `:ns/name` |
| Booleans | `true` `false` |
| Nil | `nil` |
| Reader Macros | `'` `` ` `` `~` `~@` |

---

## 3. Data Types

Lonala is dynamically typed. Every value has a runtime type. This section describes the semantics of each type.

### 3.1 Type Hierarchy

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
│   └── Map (associative, immutable)
└── Function
```

### 3.2 Nil

`nil` represents the absence of a value. It is the only value of its type.

- **Truthiness**: `nil` is falsy
- **Equality**: `nil` equals only itself
- **Use cases**: Missing values, empty results, uninitialized state

```clojure
nil           ; the nil value
(= nil nil)   ; => true
(= nil false) ; => false
```

### 3.3 Bool

Booleans represent logical truth values. There are exactly two boolean values: `true` and `false`.

- **Truthiness**: `false` is falsy; `true` is truthy
- **Equality**: Booleans equal only themselves

```clojure
true          ; logical true
false         ; logical false
(= true true) ; => true
(= true 1)    ; => false (no type coercion)
```

### 3.4 Numbers

Lonala supports three numeric types with automatic promotion where appropriate.

#### 3.4.1 Integer

Arbitrary-precision integers with no fixed size limit. Small integers (fitting in 63 bits) are stored inline; larger integers use heap allocation.

```clojure
42                          ; small integer
-17                         ; negative integer
9999999999999999999999999   ; big integer (arbitrary precision)
```

**Operations**: Integers support all arithmetic operations. Division of integers that doesn't produce a whole number yields a Ratio.

#### 3.4.2 Float

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

#### 3.4.3 Ratio

Exact rational numbers represented as a numerator and denominator. Ratios are automatically normalized (reduced to lowest terms) and the denominator is always positive.

```clojure
1/3           ; one third
-2/4          ; normalized to -1/2
22/7          ; approximation of pi
```

**Operations**: Arithmetic on ratios produces exact results. Mixing ratios with floats promotes to float.

### 3.5 Symbol

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

### 3.6 String

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

### 3.7 Binary

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

### 3.8 List

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

### 3.9 Vector

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

### 3.10 Map

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

### 3.11 Function

First-class callable values created with `fn`.

```clojure
(fn [x] (* x x))           ; anonymous function
(fn square [x] (* x x))    ; named function (for recursion/debugging)
```

**Characteristics**:
- **First-class**: Can be passed as arguments, returned from functions, stored in collections
- **Arity**: Fixed number of parameters (variadic functions planned)
- **Identity**: Functions are compared by identity, not structure

### 3.12 Truthiness

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

### 3.13 Equality

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

### 3.14 Metadata

Metadata is a map of data *about* a value, attached to the value without affecting its identity or equality. Two values that differ only in metadata are still equal.

**What Supports Metadata**:
- Symbols
- Lists
- Vectors
- Maps
- Vars (the binding between a symbol and its value)

**Primitives and scalars (nil, booleans, numbers, strings, binaries) do NOT support metadata.**

#### 3.14.1 Reading Metadata

```clojure
(meta obj)              ; => metadata map or nil
```

#### 3.14.2 Attaching Metadata

```clojure
;; Create new value with metadata (immutable - original unchanged)
(with-meta [1 2 3] {:source "user"})

;; Transform existing metadata
(vary-meta obj assoc :new-key value)
```

#### 3.14.3 Reader Syntax

```clojure
;; Full metadata map
^{:doc "A vector" :private true} [1 2 3]

;; Shorthand for ^{:keyword true}
^:private my-var

;; Multiple metadata items
^:private ^:dynamic my-var
;; equivalent to: ^{:private true :dynamic true} my-var
```

#### 3.14.4 Var Metadata

Vars (created by `def`) carry metadata separate from their value:

```clojure
(def my-var 42)
(meta #'my-var)         ; => {:name my-var, :ns user, :line 1, ...}

;; Docstrings become :doc metadata
(def my-var "Documentation here" 42)
(meta #'my-var)         ; => {:doc "Documentation here", ...}
```

#### 3.14.5 Standard Metadata Keys

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

#### 3.14.6 Metadata and Equality

Metadata does NOT affect equality or hash codes:

```clojure
(= [1 2 3] (with-meta [1 2 3] {:foo :bar}))  ; => true

(= (hash [1 2 3])
   (hash (with-meta [1 2 3] {:foo :bar})))   ; => true
```

This allows metadata to annotate values without changing program semantics.

---

## 4. Literals

This section describes the syntax for writing literal values in source code.

### 4.1 Numeric Literals

#### 4.1.1 Integer Literals

```clojure
42              ; decimal
-17             ; negative decimal
0               ; zero

; Alternate bases
0xFF            ; hexadecimal (255)
0xff            ; hexadecimal (case insensitive)
0b1010          ; binary (10)
0o755           ; octal (493)
```

#### 4.1.2 Float Literals

```clojure
3.14            ; decimal point required
-0.5            ; negative
.5              ; ERROR: leading decimal point not allowed
0.5             ; correct form

; Scientific notation
1e10            ; 10000000000.0
1E10            ; case insensitive
1.5e-3          ; 0.0015
-2.5e+4         ; -25000.0

; Special values
##Inf           ; positive infinity
##-Inf          ; negative infinity
##NaN           ; not a number
```

#### 4.1.3 Ratio Literals

```clojure
1/3             ; one third
22/7            ; ratio (not evaluated as division)
-1/2            ; negative ratio
4/2             ; normalized to 2 (integer)
```

### 4.2 String Literals

Strings are delimited by double quotes:

```clojure
"hello world"
"line 1\nline 2"
"tab\there"
"quote: \"hi\""
"backslash: \\"
""              ; empty string
```

### 4.3 Boolean Literals

```clojure
true
false
```

### 4.4 Nil Literal

```clojure
nil
```

### 4.5 Collection Literals

#### 4.5.1 List Literals

Lists are written with parentheses. When evaluated, lists are treated as function calls unless quoted:

```clojure
'()             ; empty list (quoted)
'(1 2 3)        ; list of 1, 2, 3 (quoted)
(list 1 2 3)    ; using list function (planned)
```

#### 4.5.2 Vector Literals

```clojure
[]              ; empty vector
[1 2 3]         ; vector of integers
[1, 2, 3]       ; commas optional
[:a :b :c]      ; vector of keywords
```

#### 4.5.3 Map Literals

Maps are written with curly braces containing alternating keys and values:

```clojure
{}              ; empty map
{:a 1 :b 2}     ; two key-value pairs
{:a 1, :b 2}    ; commas optional
{"key" "value"} ; string keys
```

### 4.6 Symbol Literals

Symbols are written directly without delimiters:

```clojure
foo
bar-baz
*special*
+
->
empty?
update!
ns/qualified
```

### 4.7 Keyword Literals

Keywords begin with a colon:

```clojure
:foo
:bar-baz
:ns/qualified
```

> **Note**: Keywords are parsed but full keyword semantics are planned for future implementation.

---

## 5. Symbols and Evaluation

### 5.1 Evaluation Rules

When Lonala evaluates an expression, it follows these rules:

1. **Self-evaluating values**: Numbers, strings, booleans, `nil`, keywords, vectors, and maps evaluate to themselves
2. **Symbols**: Look up the symbol's value in the current environment
3. **Lists**: Treat the first element as a function/special form and apply it to the remaining elements

```clojure
42              ; => 42 (self-evaluating)
"hello"         ; => "hello" (self-evaluating)
[1 2 3]         ; => [1 2 3] (self-evaluating)

x               ; => looks up x in environment
(+ 1 2)         ; => evaluates +, then calls it with 1 and 2
```

### 5.2 Symbol Resolution

Symbols are resolved by searching:

1. **Local bindings**: Parameters and `let`-bound variables
2. **Global definitions**: Values bound with `def`

```clojure
(def x 10)              ; global binding

(let [y 20]             ; local binding
  (+ x y))              ; x from global, y from local
```

### 5.3 Qualified Symbols

Qualified symbols contain a namespace prefix separated by `/`:

```clojure
user/foo                ; symbol foo in namespace user
clojure.core/map        ; symbol map in namespace clojure.core
```

> **Note**: Full namespace support is planned for Phase 6.

### 5.4 Preventing Evaluation

Use `quote` to prevent evaluation:

```clojure
(quote foo)     ; => the symbol foo (not its value)
'foo            ; => same, using reader macro

(quote (+ 1 2)) ; => the list (+ 1 2) (not 3)
'(+ 1 2)        ; => same
```

---

## 6. Special Forms

Special forms are fundamental language constructs with evaluation rules that differ from normal function calls. They cannot be implemented as functions.

### 6.1 `def`

Binds a value to a global variable.

**Syntax**: `(def name value)`

**Parameters**:
- `name` — A symbol naming the variable
- `value` — An expression to evaluate and bind

**Returns**: The symbol `name`

**Semantics**: Evaluates `value` and binds the result to `name` in the global environment. If `name` is already defined, it is rebound to the new value.

```clojure
(def x 42)          ; => x
x                   ; => 42

(def greeting "Hello, World!")
greeting            ; => "Hello, World!"

(def square (fn [n] (* n n)))
(square 5)          ; => 25
```

### 6.2 `let`

Creates local bindings for a body of expressions.

**Syntax**: `(let [bindings*] body*)`

**Parameters**:
- `bindings` — A vector of alternating symbols and values: `[name1 val1 name2 val2 ...]`
- `body` — Zero or more expressions to evaluate

**Returns**: The value of the last body expression, or `nil` if body is empty

**Semantics**:
1. Bindings are evaluated left-to-right
2. Each binding can refer to previously bound names
3. Body expressions are evaluated with all bindings in scope
4. Bindings are local to the `let` form

```clojure
(let [x 10]
  x)                ; => 10

(let [x 10
      y 20]
  (+ x y))          ; => 30

(let [x 10
      y (* x 2)]    ; y can use x
  y)                ; => 20

(let [x 1]
  (let [x 2]        ; inner x shadows outer
    x))             ; => 2
```

### 6.3 `if`

Conditional branching.

**Syntax**: `(if test then else?)`

**Parameters**:
- `test` — Expression to evaluate for truthiness
- `then` — Expression to evaluate if test is truthy
- `else` — Expression to evaluate if test is falsy (optional, defaults to `nil`)

**Returns**: The value of `then` or `else` branch

**Semantics**: Evaluates `test`. If the result is truthy (not `nil` or `false`), evaluates and returns `then`. Otherwise, evaluates and returns `else` (or `nil` if `else` is omitted).

```clojure
(if true "yes" "no")      ; => "yes"
(if false "yes" "no")     ; => "no"
(if nil "yes" "no")       ; => "no"
(if 0 "yes" "no")         ; => "yes" (0 is truthy)

(if (> 5 3)
  "five is greater"
  "three is greater")     ; => "five is greater"

(if false "yes")          ; => nil (no else branch)
```

### 6.4 `do`

Sequential execution of multiple expressions.

**Syntax**: `(do exprs*)`

**Parameters**:
- `exprs` — Zero or more expressions

**Returns**: The value of the last expression, or `nil` if empty

**Semantics**: Evaluates each expression in order, returning the value of the final expression. Earlier expressions are evaluated for their side effects.

```clojure
(do)                      ; => nil

(do 1 2 3)                ; => 3

(do
  (print "first")
  (print "second")
  "done")                 ; prints, then => "done"
```

### 6.5 `fn`

Creates a function.

**Syntax**: `(fn name? [params*] body*)`

**Parameters**:
- `name` — Optional symbol for recursion and debugging
- `params` — Vector of parameter symbols
- `body` — Zero or more expressions forming the function body

**Returns**: A function value

**Semantics**: Creates a new function that, when called:
1. Binds arguments to parameter names
2. Evaluates body expressions in order
3. Returns the value of the last expression

```clojure
; Anonymous function
(fn [x] (* x x))

; Named function (for recursion)
(fn factorial [n]
  (if (<= n 1)
    1
    (* n (factorial (- n 1)))))

; Multiple body expressions
(fn [x]
  (print "computing...")
  (* x x))

; No parameters
(fn [] 42)

; Multiple parameters
(fn [a b c] (+ a b c))
```

**Calling functions**:

```clojure
((fn [x] (* x x)) 5)      ; => 25

(def square (fn [x] (* x x)))
(square 5)                 ; => 25
```

### 6.6 `quote`

Returns its argument unevaluated.

**Syntax**: `(quote form)`

**Parameters**:
- `form` — Any expression

**Returns**: The form itself, as data

**Semantics**: Prevents evaluation of the form. Lists become list data, symbols become symbol data.

```clojure
(quote foo)               ; => foo (the symbol)
(quote (+ 1 2))           ; => (+ 1 2) (the list)
(quote [1 2 3])           ; => [1 2 3]

; Shorthand with reader macro
'foo                      ; => foo
'(+ 1 2)                  ; => (+ 1 2)
```

### 6.7 `syntax-quote`

Template quoting with unquote support.

**Syntax**: `` `form `` or `(syntax-quote form)`

**Parameters**:
- `form` — A template expression

**Returns**: The form with unquoted parts evaluated

**Semantics**: Like `quote`, but allows selective evaluation within the template using `~` (unquote) and `~@` (unquote-splicing).

```clojure
`(1 2 3)                  ; => (1 2 3)

(let [x 10]
  `(1 ~x 3))              ; => (1 10 3)

(let [nums [2 3 4]]
  `(1 ~@nums 5))          ; => (1 2 3 4 5)
```

See [Section 10: Reader Macros](#10-reader-macros) for details on unquote operators.

---

## 7. Operators

Operators in Lonala are implemented as functions but have special syntax support for common operations.

### 7.1 Arithmetic Operators

#### 7.1.1 Addition: `+`

**Syntax**: `(+ args*)`

Adds numbers together. With no arguments, returns 0.

```clojure
(+)             ; => 0
(+ 1)           ; => 1
(+ 1 2)         ; => 3
(+ 1 2 3 4)     ; => 10
(+ 1.5 2.5)     ; => 4.0
(+ 1 1/2)       ; => 3/2
```

#### 7.1.2 Subtraction: `-`

**Syntax**: `(- x)` or `(- x ys*)`

With one argument, returns its negation. With multiple arguments, subtracts subsequent values from the first.

```clojure
(- 5)           ; => -5 (negation)
(- 10 3)        ; => 7
(- 10 3 2)      ; => 5
(- 1.5)         ; => -1.5
```

#### 7.1.3 Multiplication: `*`

**Syntax**: `(* args*)`

Multiplies numbers together. With no arguments, returns 1.

```clojure
(*)             ; => 1
(* 5)           ; => 5
(* 2 3)         ; => 6
(* 2 3 4)       ; => 24
(* 1/2 1/3)     ; => 1/6
```

#### 7.1.4 Division: `/`

**Syntax**: `(/ x)` or `(/ x ys*)`

With one argument, returns its reciprocal. With multiple arguments, divides the first by the product of the rest.

```clojure
(/ 2)           ; => 1/2 (reciprocal)
(/ 10 2)        ; => 5
(/ 10 2 5)      ; => 1
(/ 1 3)         ; => 1/3 (exact ratio)
(/ 1.0 3)       ; => 0.333... (float)
```

#### 7.1.5 Modulo: `mod`

**Syntax**: `(mod x y)`

Returns the remainder of dividing x by y.

```clojure
(mod 10 3)      ; => 1
(mod 10 5)      ; => 0
(mod -10 3)     ; => -1
```

### 7.2 Comparison Operators

All comparison operators return boolean values.

#### 7.2.1 Equality: `=`

**Syntax**: `(= x y)`

Returns `true` if x and y are equal.

```clojure
(= 1 1)         ; => true
(= 1 2)         ; => false
(= "a" "a")     ; => true
(= [1 2] [1 2]) ; => true
(= 1 1.0)       ; => true (numeric equality)
```

#### 7.2.2 Less Than: `<`

**Syntax**: `(< x y)`

Returns `true` if x is less than y.

```clojure
(< 1 2)         ; => true
(< 2 1)         ; => false
(< 1 1)         ; => false
```

#### 7.2.3 Greater Than: `>`

**Syntax**: `(> x y)`

Returns `true` if x is greater than y.

```clojure
(> 2 1)         ; => true
(> 1 2)         ; => false
(> 1 1)         ; => false
```

#### 7.2.4 Less Than or Equal: `<=`

**Syntax**: `(<= x y)`

Returns `true` if x is less than or equal to y.

```clojure
(<= 1 2)        ; => true
(<= 1 1)        ; => true
(<= 2 1)        ; => false
```

#### 7.2.5 Greater Than or Equal: `>=`

**Syntax**: `(>= x y)`

Returns `true` if x is greater than or equal to y.

```clojure
(>= 2 1)        ; => true
(>= 1 1)        ; => true
(>= 1 2)        ; => false
```

### 7.3 Bitwise Operators

Bitwise operations work on integers.

#### 7.3.1 Bitwise AND: `bit-and`

**Syntax**: `(bit-and x y)`

Returns the bitwise AND of x and y.

```clojure
(bit-and 0xFF 0x0F)     ; => 15 (0x0F)
(bit-and 0b1100 0b1010) ; => 8 (0b1000)
```

#### 7.3.2 Bitwise OR: `bit-or`

**Syntax**: `(bit-or x y)`

Returns the bitwise OR of x and y.

```clojure
(bit-or 0b1100 0b0011)  ; => 15 (0b1111)
```

#### 7.3.3 Bitwise XOR: `bit-xor`

**Syntax**: `(bit-xor x y)`

Returns the bitwise XOR of x and y.

```clojure
(bit-xor 0b1100 0b1010) ; => 6 (0b0110)
```

#### 7.3.4 Bitwise NOT: `bit-not`

**Syntax**: `(bit-not x)`

Returns the bitwise complement of x.

```clojure
(bit-not 0)             ; => -1
(bit-and (bit-not 0xFF) 0xFFFF) ; => 0xFF00
```

#### 7.3.5 Shift Left: `bit-shift-left`

**Syntax**: `(bit-shift-left x n)`

Shifts x left by n bits.

```clojure
(bit-shift-left 1 4)    ; => 16
(bit-shift-left 0xFF 8) ; => 0xFF00
```

#### 7.3.6 Shift Right: `bit-shift-right`

**Syntax**: `(bit-shift-right x n)`

Shifts x right by n bits (arithmetic shift, preserves sign).

```clojure
(bit-shift-right 16 2)  ; => 4
(bit-shift-right 0xFF00 8) ; => 0xFF
```

### 7.4 Logical Operators

#### 7.4.1 Logical Not: `not`

**Syntax**: `(not x)`

Returns `true` if x is falsy, `false` otherwise.

```clojure
(not false)     ; => true
(not nil)       ; => true
(not true)      ; => false
(not 0)         ; => false (0 is truthy)
(not "")        ; => false (empty string is truthy)
```

### 7.5 Numeric Type Coercion

When operations mix numeric types:

| Operation | Result Type |
|-----------|-------------|
| Integer + Integer | Integer |
| Integer + Float | Float |
| Integer + Ratio | Ratio |
| Float + Ratio | Float |
| Integer / Integer (exact) | Integer |
| Integer / Integer (inexact) | Ratio |

```clojure
(+ 1 2)         ; => 3 (Integer)
(+ 1 2.0)       ; => 3.0 (Float)
(+ 1 1/2)       ; => 3/2 (Ratio)
(+ 1.0 1/2)     ; => 1.5 (Float)
(/ 6 2)         ; => 3 (Integer, exact)
(/ 5 2)         ; => 5/2 (Ratio, inexact)
```

---

## 8. Functions

### 8.1 Defining Functions

Functions are created using the `fn` special form:

```clojure
; Anonymous function
(fn [x] (* x x))

; Named function (useful for recursion and debugging)
(fn square [x] (* x x))
```

To give a function a global name, combine `def` and `fn`:

```clojure
(def square (fn [x] (* x x)))
(square 5)  ; => 25

; Or with a name for recursion
(def factorial
  (fn factorial [n]
    (if (<= n 1)
      1
      (* n (factorial (- n 1))))))
```

### 8.2 Calling Functions

Function calls use list syntax with the function in the first position:

```clojure
(function-name arg1 arg2 ...)
```

Arguments are evaluated left-to-right before being passed to the function:

```clojure
(+ 1 2)              ; call + with arguments 1 and 2
(square 5)           ; call square with argument 5
(+ (square 2) 1)     ; nested: (+ 4 1) => 5
```

### 8.3 Function Arity

Each function has a fixed arity (number of parameters). Calling a function with the wrong number of arguments is a runtime error:

```clojure
(def greet (fn [name] (print name)))
(greet "Alice")      ; OK
(greet)              ; ERROR: wrong arity
(greet "A" "B")      ; ERROR: wrong arity
```

### 8.4 Function Bodies

Function bodies can contain multiple expressions. The value of the last expression is returned:

```clojure
(def process
  (fn [x]
    (print "Processing...")
    (print x)
    (* x 2)))  ; this value is returned

(process 5)  ; prints messages, returns 10
```

### 8.5 Higher-Order Functions

Functions can accept functions as arguments and return functions:

```clojure
; Function that takes a function
(def apply-twice
  (fn [f x]
    (f (f x))))

(apply-twice (fn [x] (+ x 1)) 5)  ; => 7

; Function that returns a function
(def make-adder
  (fn [n]
    (fn [x] (+ x n))))

(def add-5 (make-adder 5))
(add-5 10)  ; => 15
```

### 8.6 Recursion

Named functions can call themselves recursively:

```clojure
(def sum-to
  (fn sum-to [n]
    (if (<= n 0)
      0
      (+ n (sum-to (- n 1))))))

(sum-to 5)  ; => 15 (5+4+3+2+1)
```

### 8.7 Planned Features

The following function features are planned for future implementation:

- **Closures** (Phase 5.2): Capture lexical environment
- **Variadic functions**: Accept variable number of arguments with `& rest`
- **Multi-arity functions**: Different implementations for different arities
- **Destructuring parameters**: Pattern match in parameter lists
- **Tail call optimization** (Phase 5.3): Efficient recursive loops with `recur`

---

## 9. Built-in Functions

Built-in functions (also called primitives or natives) are implemented in Rust and provide core functionality that cannot be implemented in Lonala itself.

### 9.1 Design Principle: Minimal Native Functions

Lonala follows the Lisp tradition of building the entire language from minimal primitives. Native functions are only used when:

1. **Hardware access is required** (MMIO, DMA, IRQ)
2. **Runtime type inspection is required** (type predicates)
3. **Core data structure operations** (cons, first, rest on internal representations)
4. **Scheduler/process integration** (spawn, send, irq-wait)
5. **seL4 kernel operations** (domain creation, capabilities)

Everything else — including collection constructors (`list`, `vector`, `hash-map`), sequence operations (`map`, `filter`, `reduce`), and even the REPL — is implemented in Lonala itself.

### 9.2 Type Predicates

Type predicates inspect runtime type tags and return boolean values.

| Function | Description |
|----------|-------------|
| `nil?` | Is the value nil? |
| `boolean?` | Is the value a boolean? |
| `integer?` | Is the value an integer? |
| `float?` | Is the value a float? |
| `ratio?` | Is the value a ratio? |
| `string?` | Is the value a string? |
| `symbol?` | Is the value a symbol? |
| `keyword?` | Is the value a keyword? |
| `binary?` | Is the value a binary buffer? |
| `list?` | Is the value a list? |
| `vector?` | Is the value a vector? |
| `map?` | Is the value a map? |
| `fn?` | Is the value a function? |

```clojure
(nil? nil)        ; => true
(list? '(1 2 3))  ; => true
(vector? [1 2])   ; => true
(fn? +)           ; => true
```

### 9.3 Collection Primitives

Core operations on collections. Higher-level functions like `map`, `filter`, `reduce` are implemented in Lonala using these primitives.

#### 9.3.1 List Operations

| Function | Syntax | Description |
|----------|--------|-------------|
| `cons` | `(cons x coll)` | Prepend x to collection |
| `first` | `(first coll)` | Get first element (nil if empty) |
| `rest` | `(rest coll)` | Get all but first element (empty list if empty) |

```clojure
(cons 1 '(2 3))   ; => (1 2 3)
(first '(1 2 3))  ; => 1
(rest '(1 2 3))   ; => (2 3)
(first nil)       ; => nil
(rest nil)        ; => ()
```

#### 9.3.2 Vector Operations

| Function | Syntax | Description |
|----------|--------|-------------|
| `nth` | `(nth coll index)` | Get element at index |
| `conj` | `(conj coll x)` | Add element to collection |
| `count` | `(count coll)` | Get collection size |

```clojure
(nth [1 2 3] 1)   ; => 2
(conj [1 2] 3)    ; => [1 2 3]
(count [1 2 3])   ; => 3
```

#### 9.3.3 Map Operations

| Function | Syntax | Description |
|----------|--------|-------------|
| `get` | `(get m key)` | Get value for key (nil if missing) |
| `assoc` | `(assoc m key val)` | Associate key with value |
| `dissoc` | `(dissoc m key)` | Remove key |
| `keys` | `(keys m)` | Get sequence of keys |
| `vals` | `(vals m)` | Get sequence of values |

```clojure
(get {:a 1} :a)       ; => 1
(assoc {:a 1} :b 2)   ; => {:a 1 :b 2}
(dissoc {:a 1 :b 2} :a) ; => {:b 2}
```

### 9.4 Binary Operations

Operations on raw byte buffers for systems programming.

| Function | Syntax | Description |
|----------|--------|-------------|
| `make-binary` | `(make-binary size)` | Allocate zeroed byte buffer |
| `binary-len` | `(binary-len buf)` | Get buffer length |
| `binary-get` | `(binary-get buf index)` | Get byte at index (0-255) |
| `binary-set` | `(binary-set buf index byte)` | Set byte at index |
| `binary-slice` | `(binary-slice buf start end)` | Zero-copy view |
| `binary-copy!` | `(binary-copy! dst dst-off src src-off len)` | Copy bytes |

```clojure
(def buf (make-binary 4))
(binary-set buf 0 0xFF)
(binary-get buf 0)        ; => 255
(binary-len buf)          ; => 4
```

### 9.5 Symbol Operations

| Function | Syntax | Description |
|----------|--------|-------------|
| `symbol` | `(symbol name)` | Create/intern a symbol |
| `gensym` | `(gensym)` or `(gensym prefix)` | Generate unique symbol |

```clojure
(symbol "foo")    ; => foo
(gensym)          ; => G__123
(gensym "temp")   ; => temp__124
```

### 9.6 Metadata Operations

Operations for reading and attaching metadata to values.

| Function | Syntax | Description |
|----------|--------|-------------|
| `meta` | `(meta obj)` | Get metadata map (or nil) |
| `with-meta` | `(with-meta obj map)` | Return copy with new metadata |
| `vary-meta` | `(vary-meta obj f & args)` | Transform metadata with function |

```clojure
;; Attach metadata
(def v (with-meta [1 2 3] {:source "test"}))
(meta v)              ; => {:source "test"}

;; Transform metadata
(def v2 (vary-meta v assoc :modified true))
(meta v2)             ; => {:source "test" :modified true}

;; Var metadata
(defn add "Adds two numbers" [x y] (+ x y))
(meta #'add)          ; => {:doc "Adds two numbers"
                      ;     :arglists ([x y])
                      ;     :name add
                      ;     :file "user.lona"
                      ;     :line 1 ...}
```

See [Section 3.14 Metadata](#314-metadata) for full documentation.

### 9.8 MMIO (Memory-Mapped I/O)

Direct hardware register access for device drivers. These operate on physical memory addresses.

| Function | Syntax | Description |
|----------|--------|-------------|
| `peek-u8` | `(peek-u8 addr)` | Read unsigned 8-bit value |
| `peek-u16` | `(peek-u16 addr)` | Read unsigned 16-bit value |
| `peek-u32` | `(peek-u32 addr)` | Read unsigned 32-bit value |
| `peek-u64` | `(peek-u64 addr)` | Read unsigned 64-bit value |
| `poke-u8` | `(poke-u8 addr val)` | Write unsigned 8-bit value |
| `poke-u16` | `(poke-u16 addr val)` | Write unsigned 16-bit value |
| `poke-u32` | `(poke-u32 addr val)` | Write unsigned 32-bit value |
| `poke-u64` | `(poke-u64 addr val)` | Write unsigned 64-bit value |

```clojure
;; Example: UART driver
(def uart-base 0x09000000)
(poke-u8 uart-base 0x41)      ; Write 'A' to UART data register
(peek-u8 uart-base)           ; Read from UART data register
```

### 9.9 DMA (Direct Memory Access)

Primitives for zero-copy hardware I/O with physically contiguous memory.

| Function | Syntax | Description |
|----------|--------|-------------|
| `dma-alloc` | `(dma-alloc size)` | Allocate DMA-capable buffer |
| `phys-addr` | `(phys-addr binary)` | Get physical address of buffer |
| `memory-barrier` | `(memory-barrier)` | Ensure memory ordering |

```clojure
;; Allocate DMA buffer for network card
(def dma-buf (dma-alloc 4096))
;; Returns {:virt <addr> :phys <addr> :buffer <binary>}

;; Get physical address for device descriptor
(phys-addr (:buffer dma-buf))

;; Ensure writes are visible to device
(memory-barrier)
```

### 9.10 IRQ (Interrupt Handling)

Interrupt handling for device drivers.

| Function | Syntax | Description |
|----------|--------|-------------|
| `irq-wait` | `(irq-wait irq-cap)` | Block until interrupt fires |

```clojure
;; Driver main loop
(loop []
  (irq-wait uart-irq-cap)
  (handle-uart-interrupt)
  (recur))
```

### 9.11 Time

Time-related primitives.

| Function | Syntax | Description |
|----------|--------|-------------|
| `now-ms` | `(now-ms)` | Current time in milliseconds |
| `send-after` | `(send-after pid delay msg)` | Send message after delay |

```clojure
(now-ms)                  ; => 1234567890
(send-after (self) 1000 :timeout)  ; Send :timeout to self after 1 second
```

### 9.12 I/O

Basic output primitives. Note: The REPL and high-level I/O are implemented in Lonala using MMIO primitives.

| Function | Syntax | Description |
|----------|--------|-------------|
| `print` | `(print args*)` | Print values followed by newline |

```clojure
(print "Hello")           ; prints: Hello
(print 1 2 3)             ; prints: 1 2 3
```

### 9.13 Process Primitives

See [Section 13: Concurrency](#13-concurrency) for process-related functions.

| Function | Syntax | Description |
|----------|--------|-------------|
| `spawn` | `(spawn fn)` | Create new process |
| `self` | `(self)` | Get current process ID |
| `exit` | `(exit reason)` | Exit current process |
| `send` | `(send pid msg)` | Send message to process |

### 9.14 seL4 / Domain Primitives

Low-level seL4 operations for domain isolation.

| Function | Syntax | Description |
|----------|--------|-------------|
| `domain-create` | `(domain-create opts)` | Create isolated domain |
| `cap-grant` | `(cap-grant domain cap)` | Grant capability to domain |
| `cap-revoke` | `(cap-revoke domain cap)` | Revoke capability |

### 9.15 Standard Library Functions (Lonala)

The following functions are implemented in Lonala (in `lona/core.lona`), not as native primitives:

**Collection Constructors:**
- `list` — `(defn list [& args] args)`
- `vector` — `(defn vector [& args] (into [] args))`
- `hash-map` — `(defn hash-map [& kvs] (apply assoc {} kvs))`

**Sequence Operations:**
- `map`, `filter`, `reduce`, `concat`, `take`, `drop`, `partition`

**Higher-Order Functions:**
- `apply`, `comp`, `partial`, `identity`, `constantly`

**Predicates:**
- `empty?`, `seq?`, `coll?`

**Numeric:**
- `inc`, `dec`, `abs`, `min`, `max`

**String:**
- `str`, `subs`, `join`, `split`

---

## 10. Reader Macros

Reader macros transform syntax during the read phase, before evaluation. They provide concise notation for common patterns.

### 10.1 Quote: `'`

**Syntax**: `'form`

**Expands to**: `(quote form)`

Prevents evaluation of the following form.

```clojure
'foo              ; => foo (symbol)
'(1 2 3)          ; => (1 2 3) (list)
'[a b c]          ; => [a b c] (vector)
```

### 10.2 Syntax-Quote: `` ` ``

**Syntax**: `` `form ``

**Expands to**: `(syntax-quote form)`

Template quoting that allows selective unquoting.

```clojure
`foo              ; => foo
`(1 2 3)          ; => (1 2 3)

(let [x 10]
  `(a ~x c))      ; => (a 10 c)
```

### 10.3 Unquote: `~`

**Syntax**: `~form`

**Valid in**: Inside syntax-quote only

Evaluates `form` and inserts the result into the surrounding template.

```clojure
(let [x 1
      y 2]
  `(~x ~y ~(+ x y)))  ; => (1 2 3)

(let [op '+]
  `(~op 1 2))         ; => (+ 1 2)
```

### 10.4 Unquote-Splicing: `~@`

**Syntax**: `~@form`

**Valid in**: Inside syntax-quote only, within a list or vector

Evaluates `form` (which must be a sequence) and splices its elements into the surrounding collection.

```clojure
(let [nums [2 3 4]]
  `(1 ~@nums 5))      ; => (1 2 3 4 5)

(let [args [1 2 3]]
  `(+ ~@args))        ; => (+ 1 2 3)
```

**Difference from unquote**:

```clojure
(let [xs [1 2 3]]
  `(a ~xs b))         ; => (a [1 2 3] b)  -- xs inserted as vector

(let [xs [1 2 3]]
  `(a ~@xs b))        ; => (a 1 2 3 b)    -- xs elements spliced
```

### 10.5 Nested Syntax-Quote

Syntax-quote can be nested. Each level of nesting requires an additional unquote to escape:

```clojure
`(a `(b ~x))          ; outer ~x not evaluated
`(a `(b ~~x))         ; x evaluated at outer level

(let [x 1]
  `(a `(b ~~x)))      ; => (a (syntax-quote (b (unquote 1))))
```

---

## 11. Macros

> **Status**: `defmacro` implemented (Phase 4.2). Macro expansion (Phase 4.3) and introspection (Phase 4.4) are planned.

Macros enable compile-time code transformation. They receive unevaluated code as data and return new code to be compiled.

### 11.1 Defining Macros

**Syntax**: `(defmacro name [params*] body+)`

**Parameters**:
- `name` — A symbol naming the macro
- `params` — A vector of parameter symbols
- `body` — One or more expressions forming the macro body

**Returns**: The symbol `name`

**Semantics**: Defines a macro that, when called during compilation, receives its arguments as unevaluated AST. The macro body should return transformed code (typically using quasiquote).

```clojure
(defmacro unless [test body]
  `(if (not ~test) ~body nil))
; => unless

(defmacro when [test body]
  `(if ~test ~body nil))
; => when
```

> **Note**: Macros are fully functional. They are defined with `defmacro`, stored in a persistent registry, and expanded at compile time using the VM-based macro expander. Introspection primitives (`macro?`, `macroexpand-1`, `macroexpand`) are also available.

### 11.2 Macro Introspection

```clojure
;; Check if a symbol names a macro
(macro? 'unless)  ; => true
(macro? 'if)      ; => false (special form)
(macro? 'foo)     ; => false (undefined)

;; macroexpand-1: expand exactly once
(macroexpand-1 '(unless false (print "hi")))
; => (when (not false) (print "hi"))   ; if unless expands to when

;; macroexpand: keep expanding while top-level is a macro
(macroexpand '(unless false (print "hi")))
; => (if (not false) (print "hi") nil) ; unless -> when -> if
```

**Key difference**: `macroexpand-1` performs a single expansion step. `macroexpand` iterates until the top-level form is no longer a macro call.

### 11.3 Common Macro Patterns (Planned)

> **Note**: These patterns require rest arguments (`& args`), which is planned for Phase 5.

```clojure
; when - one-armed if
(defmacro when [test & body]
  `(if ~test (do ~@body) nil))

; defn - define named function
(defmacro defn [name params & body]
  `(def ~name (fn ~name ~params ~@body)))

; -> threading macro
(defmacro -> [x & forms]
  ...)
```

---

## 12. Namespaces

> **Status**: *Planned for Phase 6*

Namespaces organize code and prevent name collisions.

### 12.1 Namespace Declaration (Planned)

```clojure
(ns my.app
  (:require [lona.core :as c]
            [lona.string :refer [join]]))
```

### 12.2 Qualified References (Planned)

```clojure
lona.core/map        ; fully qualified
c/map                ; using alias
join                 ; referred directly
```

### 12.3 Creating and Switching (Planned)

```clojure
(in-ns 'my.namespace)  ; switch to namespace
(ns-name *ns*)         ; get current namespace name
```

---

## 13. Concurrency

> **Status**: *Planned for Phases 9-12*

Lonala provides Erlang-style lightweight processes and message passing.

### 13.1 Processes (Planned)

```clojure
; Spawn a new process
(spawn (fn [] (print "Hello from process!")))

; Get current process ID
(self)

; Exit current process
(exit :normal)
```

### 13.2 Message Passing (Planned)

```clojure
; Send message to process
(send pid {:type :greeting :text "Hello"})

; Receive messages with pattern matching
(receive
  {:type :greeting :text text}
    (print "Got greeting:" text)
  {:type :shutdown}
    (exit :normal)
  (after 5000
    (print "Timeout!")))
```

### 13.3 Supervision (Planned)

```clojure
(def-supervisor my-supervisor
  :strategy :one-for-one
  :children
  [{:id :worker-1 :start #(spawn worker-fn [])}
   {:id :worker-2 :start #(spawn worker-fn [])}])
```

### 13.4 Linking and Monitoring (Planned)

```clojure
(link pid)           ; bidirectional link
(unlink pid)
(spawn-link fn args) ; spawn and link atomically

(monitor pid)        ; unidirectional monitor
(demonitor ref)
```

---

## Appendices

### Appendix A: Grammar

This appendix provides a formal grammar for Lonala in EBNF notation.

```ebnf
(* Top-level *)
program     = form* ;
form        = literal | symbol | list | vector | map | reader-macro ;

(* Literals *)
literal     = nil | boolean | number | string | keyword ;
nil         = "nil" ;
boolean     = "true" | "false" ;
number      = integer | float | ratio ;
integer     = decimal-int | hex-int | binary-int | octal-int ;
decimal-int = ["-"] digit+ ;
hex-int     = "0" ("x" | "X") hex-digit+ ;
binary-int  = "0" ("b" | "B") ("0" | "1")+ ;
octal-int   = "0" ("o" | "O") octal-digit+ ;
float       = ["-"] digit+ "." digit+ [exponent]
            | ["-"] digit+ exponent
            | "##Inf" | "##-Inf" | "##NaN" ;
exponent    = ("e" | "E") ["+" | "-"] digit+ ;
ratio       = ["-"] digit+ "/" digit+ ;
string      = '"' string-char* '"' ;
string-char = escape-seq | (any char except '"' and '\') ;
escape-seq  = "\\" | '\"' | "\n" | "\r" | "\t" ;
keyword     = ":" symbol-name ;

(* Symbols *)
symbol      = symbol-name | qualified-symbol ;
symbol-name = symbol-start symbol-char* ;
qualified-symbol = symbol-name "/" symbol-name ;
symbol-start = letter | special-char ;
symbol-char = letter | digit | special-char ;
special-char = "*" | "+" | "!" | "-" | "_" | "'" | "?" | "<" | ">" | "=" ;
letter      = "a".."z" | "A".."Z" ;
digit       = "0".."9" ;
hex-digit   = digit | "a".."f" | "A".."F" ;
octal-digit = "0".."7" ;

(* Collections *)
list        = "(" form* ")" ;
vector      = "[" form* "]" ;
map         = "{" (form form)* "}" ;

(* Reader macros *)
reader-macro = quote | syntax-quote | unquote | unquote-splice ;
quote         = "'" form ;
syntax-quote  = "`" form ;
unquote       = "~" form ;
unquote-splice = "~@" form ;

(* Whitespace and comments *)
whitespace  = " " | "\t" | "\n" | "\r" | "," ;
comment     = ";" (any char except newline)* ;
```

### Appendix B: Bytecode Reference

This appendix documents the virtual machine instruction set. This is an implementation detail for contributors.

#### Instruction Encoding

Instructions are 32-bit values with the following formats:

| Format | Layout | Description |
|--------|--------|-------------|
| iABC | `[op:8][A:8][B:8][C:8]` | Three register operands |
| iABx | `[op:8][A:8][Bx:16]` | Register + 16-bit index |
| iAsBx | `[op:8][A:8][sBx:16]` | Register + signed offset |

#### Instruction Set

| Opcode | Name | Format | Description |
|--------|------|--------|-------------|
| 0 | Move | iABC | `R[A] = R[B]` |
| 1 | LoadK | iABx | `R[A] = K[Bx]` |
| 2 | LoadNil | iABC | `R[A]..R[A+B] = nil` |
| 3 | LoadTrue | iABC | `R[A] = true` |
| 4 | LoadFalse | iABC | `R[A] = false` |
| 5 | GetGlobal | iABx | `R[A] = globals[K[Bx]]` |
| 6 | SetGlobal | iABx | `globals[K[Bx]] = R[A]` |
| 7 | Add | iABC | `R[A] = RK[B] + RK[C]` |
| 8 | Sub | iABC | `R[A] = RK[B] - RK[C]` |
| 9 | Mul | iABC | `R[A] = RK[B] * RK[C]` |
| 10 | Div | iABC | `R[A] = RK[B] / RK[C]` |
| 11 | Mod | iABC | `R[A] = RK[B] % RK[C]` |
| 12 | Neg | iABC | `R[A] = -R[B]` |
| 13 | Eq | iABC | `R[A] = RK[B] == RK[C]` |
| 14 | Lt | iABC | `R[A] = RK[B] < RK[C]` |
| 15 | Le | iABC | `R[A] = RK[B] <= RK[C]` |
| 16 | Gt | iABC | `R[A] = RK[B] > RK[C]` |
| 17 | Ge | iABC | `R[A] = RK[B] >= RK[C]` |
| 18 | Not | iABC | `R[A] = not R[B]` |
| 19 | Jump | iAsBx | `PC += sBx` |
| 20 | JumpIf | iAsBx | `if R[A] then PC += sBx` |
| 21 | JumpIfNot | iAsBx | `if not R[A] then PC += sBx` |
| 22 | Call | iABC | `R[A..A+C-1] = R[A](R[A+1..A+B])` |
| 23 | TailCall | iABC | `return R[A](R[A+1..A+B])` |
| 24 | Return | iABC | `return R[A..A+B-1]` |

#### RK Encoding

Operands B and C in arithmetic/comparison instructions use RK encoding:
- If bit 7 is clear (0-127): Refers to register R[n]
- If bit 7 is set (128-255): Refers to constant K[n-128]

### Appendix C: Differences from Clojure

Lonala is inspired by Clojure but differs in several ways:

| Feature | Clojure | Lonala |
|---------|---------|--------|
| **Runtime** | JVM | seL4 / custom VM |
| **Concurrency** | STM + atoms + agents | Erlang-style processes |
| **Interop** | Java interop | No FFI; systems programming primitives |
| **Lazy sequences** | Default | Planned, explicit |
| **Namespaces** | First-class | Planned (Phase 6) |
| **Metadata** | Pervasive | Planned |
| **Protocols** | Supported | Planned |
| **Transducers** | Supported | Planned |
| **Keywords** | Full support | Partial (parsing only) |
| **Regular expressions** | `#"pattern"` | Planned |
| **Sets** | `#{1 2 3}` | Planned |

### Appendix D: Reserved Words and Symbols

The following symbols have special meaning in Lonala:

#### Special Forms
- `def`
- `defmacro`
- `let`
- `if`
- `do`
- `fn`
- `quote`
- `syntax-quote`
- `unquote`
- `unquote-splicing`

#### Reserved for Future Use
- `loop`
- `recur`
- `try`
- `catch`
- `finally`
- `throw`
- `ns`
- `require`
- `use`
- `import`
- `spawn`
- `send`
- `receive`

#### Boolean and Nil
- `true`
- `false`
- `nil`

### Appendix E: Version History

| Version | Phase | Features |
|---------|-------|----------|
| 0.1.0 | 1.x | Foundation: allocator, UART, basic values |
| 0.2.0 | 2.x | Minimal interpreter: lexer, parser, bytecode, VM |
| 0.3.0 | 3.x | REPL: def, let, if, do, fn, quote |
| 0.4.0 | 4.1 | Quasiquote: syntax-quote, unquote, unquote-splicing |
| 0.4.2 | 4.2 | Macro definition: defmacro |
| 0.5.0 | 4.3-4.4 | Macro expansion, introspection *(planned)* |
| 0.6.0 | 5.x | Closures, TCO *(planned)* |
| 0.7.0 | 6.x | Namespaces *(planned)* |
| 0.8.0 | 7.x | Standard library *(planned)* |
| 0.9.0 | 8.x | Introspection *(planned)* |
| 1.0.0 | 9-12 | Processes, messages, supervision *(planned)* |

---

## References

- [Lona Project Goals](goals.md) — Vision and design philosophy
- [Implementation Plan](development/implementation-plan.md) — Development roadmap
- [Clojure Reference](https://clojure.org/reference) — Clojure documentation
- [Erlang Reference Manual](https://www.erlang.org/doc/system/reference_manual.html) — Erlang documentation
- [seL4 Documentation](https://docs.sel4.systems/) — seL4 microkernel
