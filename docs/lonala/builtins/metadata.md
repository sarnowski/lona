# Metadata Operations
> **Status**: *(Planned)* — Requires the Metadata system to be implemented first.

Operations for reading and attaching metadata to values.

## Functions

| Function | Syntax | Description |
|----------|--------|-------------|
| `meta` | `(meta obj)` | Get metadata map (or nil) |
| `with-meta` | `(with-meta obj map)` | Return copy with new metadata |
| `vary-meta` | `(vary-meta obj f & args)` | Transform metadata with function |

## Examples

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

## What Supports Metadata

- Symbols
- Lists
- Vectors
- Maps
- Vars (the binding between a symbol and its value)

**Primitives and scalars (nil, booleans, numbers, strings, binaries) do NOT support metadata.**

See [Data Types: Metadata](../data-types.md#316-metadata-planned) for full documentation.

---

