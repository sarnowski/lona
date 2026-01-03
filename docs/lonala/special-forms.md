# Special Forms

Lonala has exactly **5 special forms**. These are the only constructs with special evaluation rules. Everything else is either an intrinsic function or derived from these forms.

---

## `def`

Creates or updates a root var binding.

```clojure
(def name value)
(def ^metadata name value)
```

Semantics:
- Creates var in current namespace if it doesn't exist
- Updates root binding if var exists (upsert)
- Returns the var

```clojure
(def pi 3.14159)
(def ^{:macro true} my-macro (fn* [form] ...))
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

```clojure
(fn* [x] (+ x 1))
(fn* [a b] (+ a b))
(fn* [& args] (apply + args))
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
- No match exits process with `[:error :badmatch %{:value v}]`
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

### Guards

```clojure
(match x
  n when (> n 0) "positive"
  n when (< n 0) "negative"
  _ "zero")
```

Guards must be pure expressions.

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

```clojure
(do
  (send pid [:start])
  (log "started")
  :ok)
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

```clojure
(quote (+ 1 2))   ; → (+ 1 2) as list
'foo              ; → symbol foo
```

---

## Macro Definition

Macros are not a special form. A macro is a var with `{:macro true}` metadata:

```clojure
(def ^{:macro true} when
  (fn* [test & body]
    `(match ~test
       false nil
       nil nil
       _ (do ~@body))))
```

The compiler invokes the function at compile-time, passing unevaluated forms.
