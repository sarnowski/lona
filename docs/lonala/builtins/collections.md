# Collection Primitives

Core operations on collections. Higher-level functions like `map`, `filter`, `reduce` are implemented in Lonala using these primitives.

## Native Primitives (Rust)

### List Operations

| Function | Syntax | Description |
|----------|--------|-------------|
| `cons` | `(cons x coll)` | Prepend x to collection |
| `first` | `(first coll)` | Get first element (nil if empty) |
| `rest` | `(rest coll)` | Get all but first element (empty list if empty) |

```clojure
(cons 1 '(2 3))   ; => (1 2 3)
(first '(1 2 3))  ; => 1
(rest '(1 2 3))   ; => (2 3)
(first nil)       ; => nil
(rest nil)        ; => ()
```

### Vector/Collection Operations

| Function | Syntax | Description | Status |
|----------|--------|-------------|--------|
| `nth` | `(nth coll index)` | Get element at index | *(Planned)* |
| `conj` | `(conj coll x)` | Add element to collection | *(Planned)* |
| `count` | `(count coll)` | Get collection size | *(Planned)* |

### Map Operations

| Function | Syntax | Description | Status |
|----------|--------|-------------|--------|
| `get` | `(get m key)` | Get value for key (nil if missing) | *(Planned)* |
| `assoc` | `(assoc m key val)` | Associate key with value | *(Planned)* |
| `dissoc` | `(dissoc m key)` | Remove key | *(Planned)* |
| `keys` | `(keys m)` | Get sequence of keys | *(Planned)* |
| `vals` | `(vals m)` | Get sequence of values | *(Planned)* |

### Set Operations (Native)

| Function | Syntax | Description | Status |
|----------|--------|-------------|--------|
| `conj` | `(conj s x)` | Add element to set (polymorphic) | *(Planned)* |
| `disj` | `(disj s x)` | Remove element from set | *(Planned)* |
| `contains?` | `(contains? s x)` | Check if set contains element | *(Planned)* |

---

## Lonala Functions (Built on Primitives)

Collection constructors and higher-level operations implemented in pure Lonala.

### Collection Constructors

| Function | Syntax | Description |
|----------|--------|-------------|
| `list` | `(list & args)` | Create list from arguments |
| `vector` | `(vector & args)` | Create vector from arguments |
| `vec` | `(vec coll)` | Convert collection to vector |
| `hash-map` | `(hash-map k1 v1 ...)` | Create map from key-value pairs |
| `hash-set` | `(hash-set & keys)` | Create set from arguments |
| `set` | `(set coll)` | Create set from collection |

```clojure
(list 1 2 3)              ; => (1 2 3)
(vector 1 2 3)            ; => [1 2 3]
(vec '(1 2 3))            ; => [1 2 3]
(hash-map :a 1 :b 2)      ; => {:a 1 :b 2}
(hash-set 1 2 3)          ; => #{1 2 3}
(set [1 2 2 3])           ; => #{1 2 3}
```

### Set Operations

| Function | Syntax | Description |
|----------|--------|-------------|
| `union` | `(union s1 s2)` | Set union |
| `intersection` | `(intersection s1 s2)` | Set intersection |
| `difference` | `(difference s1 s2)` | Set difference (elements in s1 but not s2) |
| `subset?` | `(subset? s1 s2)` | Is s1 a subset of s2? |
| `superset?` | `(superset? s1 s2)` | Is s1 a superset of s2? |

```clojure
(union #{1 2} #{2 3})        ; => #{1 2 3}
(intersection #{1 2} #{2 3}) ; => #{2}
(difference #{1 2 3} #{2})   ; => #{1 3}
(subset? #{1 2} #{1 2 3})    ; => true
```

---

