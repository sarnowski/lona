# Namespaces

Namespaces organize code and prevent name collisions. The runtime maintains a registry of namespaces, each containing vars (defined symbols) and references to vars from other namespaces.

## 12.1 Namespace Declaration

```clojure
;; Basic form
(ns my.app)

;; With :require clause
(ns my.app
  (:require [other.ns :as o]           ; alias for qualified access
            [other.ns :refer [foo]]))  ; import specific symbols

;; With :use clause
(ns my.app
  (:use other.ns))                     ; refer all public symbols
```

Every namespace implicitly refers all public symbols from `lona.core` (like Clojure's `clojure.core`). This provides access to core primitives without explicit requires.

See [Special Forms: ns](special-forms.md#69-ns) for complete syntax documentation.

## 12.2 Symbol Resolution Order

Unqualified symbols are resolved in this order:

1. **Local bindings** (let, fn parameters)
2. **Upvalues** (captured from enclosing closures)
3. **Current namespace defs** (vars defined with `def`)
4. **Referred vars** (via `:require :refer`, `:use`, or implicit `lona.core`)

**Shadowing**: Current namespace defs shadow referred vars.

```clojure
(ns my.app)
(def first 42)    ; shadows lona.core/first
first             ; => 42
lona.core/first   ; => the original first function
```

## 12.3 Qualified References

```clojure
lona.core/map     ; fully qualified
str/join          ; using alias (requires :as clause)
println           ; referred directly (requires :refer clause)
```

## 12.4 Creating and Switching

```clojure
(ns my.namespace)          ; switch to namespace (creates if needed)
```

## 12.5 Namespace Loading

When a namespace is required, the runtime loads it on demand:

### Loading Process

1. **Registry Check**: If the namespace is already in the registry, skip loading
2. **Cycle Detection**: Check if this namespace is already being loaded (circular dependency)
3. **Source Retrieval**: Get source code from the source loader
4. **Compilation**: Parse and compile the source to bytecode
5. **Execution**: Execute the bytecode (which typically starts with an `ns` form)
6. **Registration**: Mark the namespace as loaded

### Source Loader

The runtime uses a `SourceLoader` abstraction to retrieve namespace source code. Currently, a `MemorySourceLoader` provides bundled sources. Future implementations will support filesystem loading.

```
Namespace name → Source Loader → Source code → Compiler → Bytecode → VM
```

### Idempotent Loading

A namespace is loaded at most once per runtime session. Subsequent `require` calls for an already-loaded namespace simply set up aliases/refers without reloading.

```clojure
;; In namespace a
(ns a (:require [shared :as s]))

;; In namespace b
(ns b (:require [shared :as sh]))  ; shared is NOT reloaded

;; Both a and b see the same shared namespace
```

### Circular Dependencies

The runtime detects circular dependencies and reports an error:

```clojure
;; a.lona
(ns a (:require [b]))

;; b.lona
(ns b (:require [a]))  ; Error: circular dependency
```

**Error format**:
```
circular dependency: 'a' is already being loaded (dependency chain: user -> a -> b)
```

The dependency chain shows the sequence of namespaces being loaded when the cycle was detected.

## 12.6 Namespace Introspection

### `ns-publics`

Returns a map of public vars in a namespace:

```clojure
(ns-publics 'my.app)
; => {foo #'my.app/foo, bar #'my.app/bar, ...}
```

Private vars (those with `^:private` metadata) are excluded.

### Private Vars

Mark a var as private to exclude it from `:use` and `ns-publics`:

```clojure
(def ^:private internal-helper ...)
(def public-api ...)
```

Private vars are still accessible via fully qualified names:

```clojure
my.app/internal-helper  ; works, but discouraged
```

## 12.7 Best Practices

### Prefer `:require` over `:use`

`:use` imports all public symbols, which can cause name collisions and makes it harder to track where symbols come from:

```clojure
;; Avoid
(ns my.app
  (:use string.utils))

;; Prefer
(ns my.app
  (:require [string.utils :as str]
            [string.utils :refer [trim join]]))
```

### Use Aliases for Common Namespaces

```clojure
(ns my.app
  (:require [string.utils :as str]
            [math.utils :as math]))

(str/join ", " items)
(math/sqrt 16)
```

### Explicit Refers for Heavily Used Functions

```clojure
(ns my.app
  (:require [string.utils :refer [trim split]]))

(trim "  hello  ")  ; more readable than str/trim
```

---
