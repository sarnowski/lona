# Lonala Language Specification

**Version**: 0.4.2 (Phase 4.2 - Macro Definition)

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
├── String
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

### 3.7 List

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

### 3.8 Vector

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

### 3.9 Map

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

### 3.10 Function

First-class callable values created with `fn`.

```clojure
(fn [x] (* x x))           ; anonymous function
(fn square [x] (* x x))    ; named function (for recursion/debugging)
```

**Characteristics**:
- **First-class**: Can be passed as arguments, returned from functions, stored in collections
- **Arity**: Fixed number of parameters (variadic functions planned)
- **Identity**: Functions are compared by identity, not structure

### 3.11 Truthiness

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

### 3.12 Equality

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

### 7.3 Logical Operators

#### 7.3.1 Logical Not: `not`

**Syntax**: `(not x)`

Returns `true` if x is falsy, `false` otherwise.

```clojure
(not false)     ; => true
(not nil)       ; => true
(not true)      ; => false
(not 0)         ; => false (0 is truthy)
(not "")        ; => false (empty string is truthy)
```

### 7.4 Numeric Type Coercion

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

Built-in functions (also called primitives or natives) are implemented in Rust and provide core functionality.

### 9.1 I/O Functions

#### 9.1.1 `print`

**Syntax**: `(print args*)`

**Parameters**: Zero or more values to print

**Returns**: `nil`

**Semantics**: Prints each argument separated by spaces, followed by a newline.

```clojure
(print "Hello")           ; prints: Hello
(print 1 2 3)             ; prints: 1 2 3
(print "x =" 42)          ; prints: x = 42
(print)                   ; prints newline only
```

### 9.2 Planned Built-in Functions

The following categories of built-in functions are planned:

#### Type Predicates (Planned)
- `nil?` — Is the value nil?
- `boolean?` — Is the value a boolean?
- `number?` — Is the value a number?
- `integer?` — Is the value an integer?
- `float?` — Is the value a float?
- `ratio?` — Is the value a ratio?
- `string?` — Is the value a string?
- `symbol?` — Is the value a symbol?
- `keyword?` — Is the value a keyword?
- `list?` — Is the value a list?
- `vector?` — Is the value a vector?
- `map?` — Is the value a map?
- `fn?` — Is the value a function?

#### Collection Functions (Planned)
- `cons` — Prepend element to list
- `first` — Get first element
- `rest` — Get all but first element
- `list` — Create a list
- `vector` — Create a vector
- `hash-map` — Create a map
- `get` — Get value by key/index
- `assoc` — Associate key with value
- `dissoc` — Remove key
- `count` — Get collection size
- `empty?` — Is collection empty?
- `conj` — Add element to collection

#### String Functions (Planned)
- `str` — Convert to string / concatenate strings
- `subs` — Substring
- `string/join` — Join strings with separator
- `string/split` — Split string

#### Numeric Functions (Planned)
- `inc` — Increment by 1
- `dec` — Decrement by 1
- `abs` — Absolute value
- `min` — Minimum of arguments
- `max` — Maximum of arguments

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

> **Note**: Macros are currently defined and stored but not yet expanded during compilation. Macro expansion is Phase 4.3.

### 11.2 Macro Expansion (Planned)

```clojure
(macroexpand '(unless false (print "hi")))
; => (if (not false) (print "hi") nil)

(macroexpand-1 '(unless false (print "hi")))
; => (if (not false) (print "hi") nil)
```

### 11.3 Common Macro Patterns (Planned)

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
