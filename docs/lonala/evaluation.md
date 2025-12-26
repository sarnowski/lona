# Symbols and Evaluation
## 5.1 Evaluation Rules

When Lonala evaluates an expression, it follows these rules:

1. **Self-evaluating atoms**: Numbers, strings, booleans, `nil`, and keywords evaluate to themselves
2. **Symbols**: Look up the symbol's value in the current environment
3. **Collection literals**: Vectors, maps, and sets evaluate their elements, then construct the collection
4. **Lists**: Treat the first element as a function/special form and apply it to the remaining elements

```clojure
42              ; => 42 (self-evaluating)
"hello"         ; => "hello" (self-evaluating)
:keyword        ; => :keyword (self-evaluating)

[1 (+ 2 3) 4]   ; => [1 5 4] (elements evaluated, then vector constructed)
{:a (+ 1 2)}    ; => {:a 3} (values evaluated, then map constructed)

x               ; => looks up x in environment
(+ 1 2)         ; => evaluates +, then calls it with 1 and 2
```

## 5.2 Symbol Resolution

Symbols are resolved by searching in order:

1. **Local bindings**: Parameters and `let`-bound variables (innermost first)
2. **Upvalues**: Captured variables from enclosing closures
3. **Current namespace defs**: Vars defined with `def` in the current namespace
4. **Referred vars**: Symbols imported via `:require :refer` or `:use`
5. **Implicit `lona.core`**: All namespaces implicitly refer `lona.core` (like Clojure's `clojure.core`)

Current namespace defs **shadow** referred vars. If you define `(def first ...)` in your namespace, it shadows `lona.core/first`.

```clojure
(def x 10)              ; global binding in current ns

(let [y 20]             ; local binding
  (+ x y))              ; y from local, x from current ns, + from lona.core

(def first 42)          ; shadows lona.core/first
first                   ; => 42 (not lona.core/first)
```

## 5.3 Qualified Symbols

Qualified symbols contain a namespace prefix separated by `/`:

```clojure
user/foo                ; symbol foo in namespace user
clojure.core/map        ; symbol map in namespace clojure.core
```

> **Note**: Namespace system implemented in Phase 1.3. See [Namespaces](namespaces.md) for details.

## 5.4 Preventing Evaluation

Use `quote` to prevent evaluation:

```clojure
(quote foo)     ; => the symbol foo (not its value)
'foo            ; => same, using reader macro

(quote (+ 1 2)) ; => the list (+ 1 2) (not 3)
'(+ 1 2)        ; => same
```

---

