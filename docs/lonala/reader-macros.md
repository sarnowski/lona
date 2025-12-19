# Reader Macros
Reader macros transform syntax during the read phase, before evaluation. They provide concise notation for common patterns.

## 10.1 Quote: `'`

**Syntax**: `'form`

**Expands to**: `(quote form)`

Prevents evaluation of the following form.

```clojure
'foo              ; => foo (symbol)
'(1 2 3)          ; => (1 2 3) (list)
'[a b c]          ; => [a b c] (vector)
```

## 10.2 Syntax-Quote: `` ` ``

**Syntax**: `` `form ``

**Expands to**: `(syntax-quote form)`

Template quoting that allows selective unquoting.

```clojure
`foo              ; => foo
`(1 2 3)          ; => (1 2 3)

(let [x 10]
  `(a ~x c))      ; => (a 10 c)
```

## 10.3 Unquote: `~`

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

## 10.4 Unquote-Splicing: `~@`

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

## 10.5 Nested Syntax-Quote

Syntax-quote can be nested. Each level of nesting requires an additional unquote to escape:

```clojure
`(a `(b ~x))          ; outer ~x not evaluated
`(a `(b ~~x))         ; x evaluated at outer level

(let [x 1]
  `(a `(b ~~x)))      ; => (a (syntax-quote (b (unquote 1))))
```

## 10.6 Anonymous Function: `#()` *(Planned)*

**Syntax**: `#(body)`

**Expands to**: `(fn [args...] body)`

Creates an anonymous function. Arguments are referenced using `%`, `%1`, `%2`, etc., and `%&` for rest arguments.

```clojure
#(+ % 1)              ; => (fn [p1] (+ p1 1))
#(+ %1 %2)            ; => (fn [p1 p2] (+ p1 p2))
#(apply + %&)         ; => (fn [& rest] (apply + rest))

;; Usage
(map #(* % %) [1 2 3 4])
; => (1 4 9 16)

(filter #(> % 2) [1 2 3 4])
; => (3 4)
```

**Argument placeholders**:
- `%` or `%1` — first argument
- `%2`, `%3`, ... — second, third, etc. arguments
- `%&` — rest arguments (as a sequence)

## 10.7 Var Quote: `#'` *(Planned)*

**Syntax**: `#'symbol`

**Expands to**: `(var symbol)`

Returns the var object itself, rather than its value. Useful for introspection and passing functions by reference.

```clojure
#'map                 ; => #'lona.core/map
(meta #'map)          ; => {:doc "..." :arglists ...}

;; Getting var vs value
map                   ; => <function>
#'map                 ; => <var>
```

## 10.8 Discard: `#_` *(Planned)*

**Syntax**: `#_form`

**Effect**: The following form is read but completely discarded

Useful for temporarily commenting out code without using line comments.

```clojure
[1 #_2 3]             ; => [1 3]
[1 #_(this is ignored) 2]  ; => [1 2]

;; Comment out multiple forms
[1 #_#_2 3 4]         ; => [1 4] (both 2 and 3 discarded)
```

## 10.9 Regex Literal: `#""` *(Planned)*

**Syntax**: `#"pattern"`

**Expands to**: `(re-pattern "pattern")`

Creates a compiled regular expression pattern.

```clojure
#"\d+"                ; pattern matching one or more digits
#"[a-z]+"             ; pattern matching lowercase letters
#"(?i)hello"          ; case-insensitive pattern

(re-find #"\d+" "abc123")  ; => "123"
```

See [Regular Expressions](builtins/regex.md) for regex functions.

## 10.10 Metadata: `^` *(Planned)*

**Syntax**: `^metadata form` or `^:keyword form`

Attaches metadata to the following form.

```clojure
;; Full metadata map
^{:doc "A vector"} [1 2 3]

;; Shorthand for {:keyword true}
^:private my-var

;; Multiple metadata items
^:private ^:dynamic *my-var*
```

See [Data Types: Metadata](data-types.md#316-metadata) for details.

---

