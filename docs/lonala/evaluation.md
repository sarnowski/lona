# Symbols and Evaluation
## 5.1 Evaluation Rules

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

## 5.2 Symbol Resolution

Symbols are resolved by searching:

1. **Local bindings**: Parameters and `let`-bound variables
2. **Global definitions**: Values bound with `def`

```clojure
(def x 10)              ; global binding

(let [y 20]             ; local binding
  (+ x y))              ; x from global, y from local
```

## 5.3 Qualified Symbols

Qualified symbols contain a namespace prefix separated by `/`:

```clojure
user/foo                ; symbol foo in namespace user
clojure.core/map        ; symbol map in namespace clojure.core
```

> **Note**: Full namespace support is planned for Phase 6.

## 5.4 Preventing Evaluation

Use `quote` to prevent evaluation:

```clojure
(quote foo)     ; => the symbol foo (not its value)
'foo            ; => same, using reader macro

(quote (+ 1 2)) ; => the list (+ 1 2) (not 3)
'(+ 1 2)        ; => same
```

---

