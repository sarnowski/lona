# Metadata Operations

Operations for reading and attaching metadata to values.

## Functions

| Function | Syntax | Description | Status |
|----------|--------|-------------|--------|
| `meta` | `(meta obj)` | Get metadata map (or nil) | Implemented |
| `with-meta` | `(with-meta obj map)` | Return copy with new metadata | Implemented |
| `vary-meta` | `(vary-meta obj f & args)` | Transform metadata with function | **Deferred** (requires `apply`) |

## Examples

```clojure
;; Attach metadata
(def v (with-meta [1 2 3] {:source "test"}))
(meta v)              ; => {:source "test"}

;; Metadata does not affect equality
(= v [1 2 3])         ; => true

;; Metadata preserved through operations
(meta (conj v 4))     ; => {:source "test"}

;; Clear metadata with nil
(meta (with-meta v nil))  ; => nil

;; Works on all supporting types
(meta (with-meta '(1 2) {:type :list}))   ; => {:type :list}
(meta (with-meta {} {:type :map}))        ; => {:type :map}
(meta (with-meta #{} {:type :set}))       ; => {:type :set}
(meta (with-meta 'foo {:type :symbol}))   ; => {:type :symbol}

;; Non-supporting types return nil
(meta 42)             ; => nil
(meta "hello")        ; => nil
(meta :keyword)       ; => nil
```

## What Supports Metadata

- Symbols
- Lists
- Vectors
- Maps
- Sets

**Keywords, nil, booleans, numbers, strings, binaries, and functions do NOT support metadata.**

## Deferred: `vary-meta`

The `vary-meta` function requires `apply` for its variadic `(vary-meta obj f & args)` signature.
It will be implemented after Task 1.8.20 (`apply`).

---

