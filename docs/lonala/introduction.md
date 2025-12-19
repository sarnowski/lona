# Introduction
## 1.1 Overview

Lonala (/loˈnaːla/) is a dynamically-typed, functional programming language designed for systems programming. It serves as the sole programming language for the Lona operating system—everything from device drivers to user applications is written in Lonala.

## 1.2 Design Influences

Lonala draws from three major traditions:

| Influence | What Lonala Takes |
|-----------|-------------------|
| **Clojure** | S-expression syntax, immutable persistent data structures, sequence abstraction |
| **Erlang/BEAM** | Lightweight processes, message passing, supervision trees, "let it crash" philosophy |
| **Common Lisp** | Condition/restart system, runtime introspection, hot-patching |

## 1.3 Key Characteristics

- **Homoiconic**: Code is represented as data structures (lists), enabling powerful metaprogramming
- **Dynamically typed**: Types are checked at runtime, not compile time
- **Immutable by default**: Data structures cannot be modified after creation
- **Functional**: Functions are first-class values; recursion is the primary iteration mechanism
- **Concurrent**: Lightweight processes communicate via message passing (planned)

## 1.4 Quick Start

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

## 1.5 How to Read This Document

- **Syntax notation**: `(form arg1 arg2)` shows the structure of expressions
- **Optional elements**: `arg?` means the argument is optional
- **Repeated elements**: `args*` means zero or more; `args+` means one or more
- **Alternatives**: `a | b` means either `a` or `b`
- **Planned features**: Sections marked *(Planned)* describe future functionality

---

