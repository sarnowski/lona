# Standard Library Functions

The following functions are implemented in Lonala (in `lona/core.lona`), not as native primitives. This follows the Lonala-first principle where everything achievable in Lonala is implemented in Lonala.

## Currently Implemented

| Function | Description |
|----------|-------------|
| `defn` | Define named function (macro) |
| `when` | One-armed conditional (macro) |

## Collection Constructors *(Planned)*

```clojure
(defn list [& args] args)
(defn vector [& args] (into [] args))
(defn hash-map [& kvs] (apply assoc {} kvs))
(defn hash-set [& vals] (into #{} vals))
(defn vec [coll] (if (vector? coll) coll (into [] coll)))
(defn set [coll] (into #{} coll))
```

## Sequence Operations *(Planned)*

Built on native `first`, `rest`, `cons`:
- `next` — rest that returns nil for empty
- `map` — Apply function to each element
- `filter` — Keep elements matching predicate
- `reduce` — Fold collection with function
- `take` — Take first n elements
- `drop` — Drop first n elements
- `partition` — Partition into groups

> **Note**: `seq` and `concat` are native primitives, not Lonala functions.

## Higher-Order Functions *(Planned)*

- `comp` — Compose functions
- `partial` — Partial function application
- `identity` — Return argument unchanged
- `constantly` — Return function that always returns value
- `juxt` — Apply multiple functions to same args

> **Note**: `apply` is a native primitive. A Lonala wrapper provides multi-argument convenience forms.

## Set Operations *(Planned)*

Built on native `conj`, `disj`, `contains?`:

```clojure
(defn union [s1 s2] (into s1 s2))
(defn intersection [s1 s2] (set (filter #(contains? s2 %) s1)))
(defn difference [s1 s2] (set (remove #(contains? s2 %) s1)))
(defn subset? [s1 s2] (every? #(contains? s2 %) s1))
(defn superset? [s1 s2] (subset? s2 s1))
```

## Atom Operations *(Planned)*

Built on native `atom`, `deref`, `reset!`, `compare-and-set!`:

```clojure
(defn swap! [a f & args]
  (loop []
    (let [old @a
          new (apply f old args)]
      (if (compare-and-set! a old new)
        new
        (recur)))))
```

- `add-watch` — Register watcher function
- `remove-watch` — Unregister watcher
- `set-validator!` — Set validation function

## Predicates *(Planned)*

- `empty?` — Is collection empty?
- `not-empty` — nil if empty, else collection
- `every?` — All elements satisfy predicate?
- `some` — Any element satisfies predicate?
- `not-every?`, `not-any?` — Negations

## Numeric *(Planned)*

- `inc`, `dec` — Increment/decrement by 1
- `abs` — Absolute value
- `min`, `max` — Extremes
- `even?`, `odd?` — Parity tests
- `pos?`, `neg?`, `zero?` — Sign tests

## String *(Planned)*

- `str` — Concatenate to string
- `join` — Join with separator
- `split` — Split by pattern
- `trim` — Remove whitespace
- `upper-case`, `lower-case` — Case conversion

> **Note**: `subs`, `string-length`, `codepoint-at` are native primitives.

## Control Flow Macros *(Planned)*

- `when-not` — Inverted conditional
- `cond` — Multi-way conditional
- `case` — Constant dispatch
- `and` — Short-circuit and
- `or` — Short-circuit or
- `->`, `->>` — Threading macros
- `as->` — Named threading
- `if-let`, `when-let` — Conditional binding

---
