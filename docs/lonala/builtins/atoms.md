# Atoms
> **Status**: *(Planned)*

Atoms provide synchronous, process-local state management. Unlike Clojure's atoms which are thread-safe across threads, Lonala atoms are process-local. Cross-process coordination uses message passing.

## Native Primitives (Rust)

These require native implementation for mutable state management:

| Function | Syntax | Description |
|----------|--------|-------------|
| `atom` | `(atom val)` | Create atom with initial value |
| `deref` / `@` | `(deref a)` or `@a` | Get current value |
| `reset!` | `(reset! a val)` | Set value directly |
| `compare-and-set!` | `(compare-and-set! a old new)` | Set only if current equals old |

## Lonala Functions (Built on Primitives)

These are implemented in pure Lonala using the native primitives above.

| Function | Syntax | Description |
|----------|--------|-------------|
| `swap!` | `(swap! a f & args)` | Update value by applying function (CAS retry loop) |
| `add-watch` | `(add-watch a key f)` | Add watcher function |
| `remove-watch` | `(remove-watch a key)` | Remove watcher |
| `set-validator!` | `(set-validator! a f)` | Set validation function |

## Examples

### Basic Usage

```clojure
;; Create and update atoms
(def counter (atom 0))
@counter                  ; => 0
(swap! counter inc)       ; => 1
(reset! counter 100)      ; => 100
```

### Atomic Compare-and-Set

```clojure
(compare-and-set! counter 100 200)  ; => true (was 100, now 200)
(compare-and-set! counter 100 300)  ; => false (not 100)
```

### Watches

Watches observe changes to atom values:

```clojure
(add-watch counter :logger
  (fn [key atom old-val new-val]
    (println "Changed from" old-val "to" new-val)))
```

### Validators

Validators constrain what values an atom can hold:

```clojure
(set-validator! counter pos?)  ; Only positive values allowed
(reset! counter -1)            ; ERROR: validator rejected value
```

---

