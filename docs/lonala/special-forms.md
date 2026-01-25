# Special Forms

Lonala has exactly **5 special forms**. These are the only constructs with special evaluation rules. Everything else is either an intrinsic function or derived from these forms.

---

## `def`

Creates or updates a root var binding.

```clojure
(def name value)
(def ^%{...} name value)
```

Semantics:
- Creates var in current namespace if it doesn't exist
- Updates root binding if var exists (upsert)
- Returns the var

```clojure
;; @todo
(def x 42)
x            ; => 42
(var? #'x)   ; => true
```

Def returns the var:

```clojure
;; @todo
(var? (def y 10))  ; => true
```

Updating existing binding:

```clojure
;; @todo
(def z 1)
z          ; => 1
(def z 2)
z          ; => 2
```

With metadata:

```clojure
;; @todo
(def ^%{:doc "A constant"} pi 3.14159)
(:doc (meta #'pi))  ; => "A constant"
```

def errors:

```clojure
(def 42 :value)  ; => ERROR :syntax-error  @todo
(def :kw :value) ; => ERROR :syntax-error  @todo
(def)            ; => ERROR :syntax-error
```

---

## `fn*`

Creates a function with a single parameter list.

```clojure
(fn* [params] body)
(fn* name [params] body)
```

Semantics:
- Single arity only (multi-arity via `match` in derived `fn`)
- No pattern matching in parameters (use `match` in body)
- Body has implicit `do` (multiple expressions allowed)
- Variadic: `[a b & rest]`

Basic functions:

```clojure
((fn* [x] (+ x 1)) 5)       ; => 6
((fn* [a b] (+ a b)) 3 4)   ; => 7  @todo
((fn* [] 42))               ; => 42
```

Variadic functions:

```clojure
;; @todo
((fn* [& args] (first args)) 1 2 3)  ; => 1
((fn* [x & rest] rest) 1 2 3)        ; => (2 3)
((fn* [x & rest] x) 1 2 3)           ; => 1
```

fn? predicate:

```clojure
(fn? (fn* [x] x))  ; => true
(fn? 42)           ; => false
```

Implicit do in body:

```clojure
;; @todo
(def counter 0)
((fn* []
   (def counter 1)
   (def counter 2)
   counter))       ; => 2
```

Named function for recursion:

```clojure
;; @todo
((fn* factorial [n]
   (match n
     0 1
     _ (* n (factorial (- n 1))))) 5)  ; => 120
```

Closures capture lexical scope:

```clojure
;; @todo
(def make-adder (fn* [x] (fn* [y] (+ x y))))
((make-adder 10) 5)   ; => 15
```

**fn* is single-arity only.** Use `match` in body for multi-arity:

```clojure
;; @todo
;; fn* takes a single parameter vector
;; Multi-arity uses match inside the body, not multiple arities
(def multi (fn* [& args]
  (match (count args)
    1 :one
    2 :two
    _ :many)))
(multi 1)      ; => :one
(multi 1 2)    ; => :two
(multi 1 2 3)  ; => :many
```

**No pattern matching in parameter list.** Patterns belong in `match`:

```clojure
;; This is correct - parameters are simple bindings
((fn* [[a b]] (+ a b)) [1 2])  ; => ERROR :syntax-error

;; Use match for destructuring
((fn* [pair]
   (match pair
     [a b] (+ a b))) [1 2])  ; => 3  @todo
```

---

## `match`

Pattern matching expression with optional guards.

```clojure
(match expr
  pattern1 body1
  pattern2 when guard2 body2
  ...)
```

Semantics:
- Patterns tried in order
- First match executes corresponding body
- `when` introduces a guard (boolean expression)
- No match exits process with reason `[:error :badmatch %{:value v}]`
- Each body is a single expression (use `do` for multiple)

### Pattern Syntax

| Pattern | Matches | Binds |
|---------|---------|-------|
| `x` | Anything | Yes |
| `_` | Anything | No (wildcard) |
| `42` | Literal 42 | No |
| `:ok` | Keyword :ok | No |
| `"hi"` | String "hi" | No |
| `[a b]` | 2-element tuple | `a`, `b` |
| `[h & t]` | Non-empty tuple | `h` (head), `t` (tail) |
| `{a b c}` | 3-element vector | `a`, `b`, `c` |
| `%{:k v}` | Map with key `:k` | `v` |
| `#{a b}` | 2-element set | `a`, `b` |
| `(pid r l)` | PID | `r` (realm), `l` (local) |
| `#bytes[0x89 & r]` | Binary prefix | `r` (rest) |
| `#bits[v:4 & _]` | Bit fields | `v` |

Variable binding:

```clojure
(match 42 x x)        ; => 42
(match [1 2] x x)     ; => [1 2]
```

Wildcard (no binding):

```clojure
(match 42 _ :matched)        ; => :matched  @todo
(match "anything" _ :ok)     ; => :ok
```

Literal matching:

```clojure
;; @todo
(match 42
  42 :forty-two
  _ :other)           ; => :forty-two

(match :ok
  :ok :success
  :error :failure)    ; => :success

(match "hello"
  "hello" :greeting
  _ :other)           ; => :greeting
```

Tuple patterns:

```clojure
(match [1 2]
  [a b] (+ a b))      ; => 3

(match [:ok 42]
  [:ok val] val
  [:error _] nil)     ; => 42  @todo

(match [:error :not-found]
  [:ok val] val
  [:error reason] reason)  ; => :not-found  @todo
```

Rest patterns in tuples:

```clojure
;; @todo
(match [1 2 3 4]
  [h & t] [h t])      ; => [1 (2 3 4)]

(match [1]
  [h & t] [h t])      ; => [1 ()]
```

Vector patterns:

```clojure
(match {1 2 3}
  {a b c} (+ a b c))  ; => 6  @todo

(match {10 20}
  {x y} (* x y))      ; => 200

(match {}
  {} :empty
  _ :not-empty)       ; => :empty  @todo
```

Map patterns:

```clojure
(match %{:name "Alice" :age 30}
  %{:name n} n)       ; => "Alice"

(match %{:a 1 :b 2}
  %{:a x :b y} (+ x y))  ; => 3

(match %{:a 1}
  %{:b x} :has-b
  _ :no-b)            ; => :no-b  @todo
```

Set patterns:

```clojure
;; @todo
(match #{1 2}
  #{a b} (+ a b))     ; => 3

(match #{}
  #{} :empty
  _ :not-empty)       ; => :empty
```

PID patterns:

```clojure
;; @todo
(def my-pid (self))
(match my-pid
  (pid realm local) [realm local])  ; => [<realm-id> <local-id>]
```

Binary patterns:

```clojure
;; @todo
(match #bytes[0x89 0x50 0x4E 0x47]
  #bytes[0x89 & rest] rest)   ; => #bytes[0x50 0x4E 0x47]

(match #bytes[1 2 3]
  #bytes[a b c] (+ a b c))    ; => 6
```

Bit field patterns:

```clojure
;; @todo
;; Extract fields from a byte using bit patterns
(match #bytes[0x45]
  #bits[version:4 ihl:4] [version ihl])  ; => [4 5]

;; Endianness in bit patterns
(match #bytes[0x00 0x50]
  #bits[port:16/be] port)  ; => 80

(match #bytes[0x50 0x00]
  #bits[port:16/le] port)  ; => 80
```

Multiple clauses (first match wins):

```clojure
;; @todo
(match 5
  1 :one
  2 :two
  _ :other)           ; => :other
```

### Guards

```clojure
;; @todo
(match 5
  n when (> n 0) :positive
  n when (< n 0) :negative
  _ :zero)            ; => :positive

(match -3
  n when (> n 0) :positive
  n when (< n 0) :negative
  _ :zero)            ; => :negative

(match 0
  n when (> n 0) :positive
  n when (< n 0) :negative
  _ :zero)            ; => :zero
```

Guards must be pure expressions.

No match exits process:

```clojure
;; @todo
;; When no pattern matches, process exits with badmatch
(match 5
  1 :one
  2 :two)  ; => ERROR :badmatch
```

---

## `do`

Evaluates expressions in sequence, returns last.

```clojure
(do expr1 expr2 ... exprN)
```

Semantics:
- Evaluates each expression left-to-right
- Returns value of last expression
- Used for side effects

Returns last expression:

```clojure
(do 1 2 3)          ; => 3
(do :a :b :c)       ; => :c
(do (+ 1 2) (+ 3 4)) ; => 7
```

Single expression:

```clojure
(do 42)             ; => 42
```

Empty do:

```clojure
(do)                ; => nil
```

Side effects execute in order:

```clojure
;; @todo
(def result {})
(do
  (def result (append result 1))
  (def result (append result 2))
  (def result (append result 3))
  result)           ; => {1 2 3}
```

---

## `quote`

Prevents evaluation, returns form as data.

```clojure
(quote form)
'form
```

Semantics:
- Returns the form unevaluated
- Symbols remain symbols, lists remain lists

Quoting symbols:

```clojure
(quote foo)       ; => foo
'foo              ; => foo
(symbol? 'foo)    ; => true
```

Quoting lists:

```clojure
(quote (+ 1 2))   ; => (+ 1 2)
'(+ 1 2)          ; => (+ 1 2)
(list? '(+ 1 2))  ; => true  @todo
```

Quoting prevents evaluation:

```clojure
(= (+ 1 2) 3)       ; => true
(= '(+ 1 2) 3)      ; => false
(= '(+ 1 2) '(+ 1 2)) ; => true
```

Quoting different types:

```clojure
'42          ; => 42
':keyword    ; => :keyword  @todo
'"string"    ; => "string"
'[1 2 3]     ; => [1 2 3]
```

Nested quotes:

```clojure
''foo        ; => (quote foo)
(first ''foo) ; => quote
```

Quoting special forms:

```clojure
;; Special forms can be quoted like any symbol
'def    ; => def
'fn*    ; => fn*
'match  ; => match
'do     ; => do
'quote  ; => quote
(symbol? 'def)  ; => true
```

---

## Macro Definition

Macros are not a special form. A macro is a var with `%{:macro true}` metadata:

```clojure
(def ^%{:macro true} when
  (fn* [test & body]
    `(match ~test
       false nil
       nil nil
       _ (do ~@body))))
```

The compiler invokes the function at compile-time, passing unevaluated forms.

Macro expansion:

```clojure
;; @todo
(def ^%{:macro true} unless
  (fn* [test & body]
    `(match ~test
       false (do ~@body)
       nil (do ~@body)
       _ nil)))

(unless false :executed)  ; => :executed
(unless true :skipped)    ; => nil
```
