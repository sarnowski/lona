# Sorted Collections
> **Status**: *(Planned)*

Sorted collections maintain elements in sorted order.

## Functions

| Function | Syntax | Description |
|----------|--------|-------------|
| `sorted-map` | `(sorted-map & kvs)` | Create sorted map (keys in natural order) |
| `sorted-set` | `(sorted-set & keys)` | Create sorted set (elements in natural order) |
| `sorted-map-by` | `(sorted-map-by cmp & kvs)` | Create sorted map with custom comparator |
| `sorted-set-by` | `(sorted-set-by cmp & keys)` | Create sorted set with custom comparator |
| `subseq` | `(subseq sc test key)` | Get subsequence matching condition |
| `rsubseq` | `(rsubseq sc test key)` | Get reverse subsequence matching condition |

## Examples

```clojure
;; Sorted map - keys in ascending order
(sorted-map :c 3 :a 1 :b 2)
; => {:a 1 :b 2 :c 3}

;; Sorted set - elements in ascending order
(sorted-set 3 1 4 1 5 9)
; => #{1 3 4 5 9}

;; Custom comparator (descending order)
(sorted-set-by > 3 1 4 1 5 9)
; => #{9 5 4 3 1}

;; Subsequence operations
(def s (sorted-set 1 2 3 4 5 6 7))
(subseq s > 3)        ; => (4 5 6 7)
(subseq s >= 3 <= 5)  ; => (3 4 5)
(rsubseq s < 5)       ; => (4 3 2 1)
```

---

