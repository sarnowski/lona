# Error Handling Functions
> **Status**: *(Planned)*

Functions for working with result tuples. These are **implemented in pure Lonala** using basic map operations and predicates — no native primitives required.

See [Data Types: Error Tuples](../data-types.md#317-error-tuples) for the error handling philosophy.

## Implementation Note

All functions in this module are pure Lonala, built on `get`, `contains?`, `map?`, and `panic!`:

```clojure
(defn ok? [x] (and (map? x) (contains? x :ok)))
(defn error? [x] (and (map? x) (contains? x :error)))
(defn unwrap! [result]
  (if (ok? result)
    (get result :ok)
    (panic! "unwrap! called on error" {:result result})))
```

## Result Predicates

| Function | Syntax | Description |
|----------|--------|-------------|
| `ok?` | `(ok? x)` | Is x an `{:ok _}` tuple? |
| `error?` | `(error? x)` | Is x an `{:error _}` tuple? |

```clojure
(ok? {:ok 42})        ; => true
(ok? {:error :fail})  ; => false
(error? {:error :x})  ; => true
```

## Result Accessors

| Function | Syntax | Description |
|----------|--------|-------------|
| `unwrap!` | `(unwrap! result)` | Extract value or panic |
| `unwrap-or` | `(unwrap-or result default)` | Extract value or return default |
| `unwrap-error` | `(unwrap-error result)` | Extract error reason or panic |

```clojure
(unwrap! {:ok 42})              ; => 42
(unwrap! {:error :fail})        ; panics!

(unwrap-or {:ok 42} 0)          ; => 42
(unwrap-or {:error :fail} 0)    ; => 0

(unwrap-error {:error :fail})   ; => :fail
(unwrap-error {:ok 42})         ; panics!
```

## Result Transformers

| Function | Syntax | Description |
|----------|--------|-------------|
| `map-ok` | `(map-ok result f)` | Transform success value |
| `map-error` | `(map-error result f)` | Transform error reason |
| `and-then` | `(and-then result f)` | Chain fallible operations |
| `or-else` | `(or-else result f)` | Recover from error |

### Examples

```clojure
;; Transform success
(map-ok {:ok 5} inc)            ; => {:ok 6}
(map-ok {:error :x} inc)        ; => {:error :x}

;; Transform error
(map-error {:error :x} name)    ; => {:error "x"}

;; Chain operations (f returns result tuple)
(and-then {:ok 5} (fn [x] {:ok (* x 2)}))   ; => {:ok 10}
(and-then {:error :x} (fn [x] {:ok 0}))    ; => {:error :x}

;; Recover from error
(or-else {:error :x} (fn [e] {:ok :default}))  ; => {:ok :default}
(or-else {:ok 5} (fn [e] {:ok 0}))             ; => {:ok 5}
```

## See Also

- [Data Types: Error Tuples](../data-types.md#317-error-tuples) — The result convention
- [Macros: Error Handling Macros](../macros.md#114-error-handling-macros) — `with`, `if-ok`, `when-ok`, `ok->`
- [Appendix F: Error Handling Idioms](../appendices/error-idioms.md) — Practical patterns

---

