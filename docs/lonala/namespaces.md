# Namespaces
> **Status**: *`ns` form with clauses implemented (Tasks 1.3.1-1.3.3). File loading deferred (Task 1.3.4).*

Namespaces organize code and prevent name collisions. The runtime maintains a registry of namespaces, each containing vars (defined symbols) and references to vars from other namespaces.

## 12.1 Namespace Declaration

```clojure
;; Basic form
(ns my.app)

;; With :require clause (aliases and refers are compile-time)
(ns my.app
  (:require [other.ns :as o]           ; alias for qualified access
            [other.ns :refer [foo]]))  ; import specific symbols

;; With :use clause (loading deferred to Task 1.3.4)
(ns my.app
  (:use other.ns))
```

Every namespace implicitly refers `lona.core` (like Clojure's `clojure.core`). This is currently implemented via VM fallback for unqualified primitive lookup.

## 12.2 Symbol Resolution Order

Unqualified symbols are resolved in this order:

1. **Local bindings** (let, fn parameters)
2. **Upvalues** (captured from enclosing closures)
3. **Current namespace defs** (vars defined with `def`)
4. **Referred vars** (via `:require :refer` or implicit `lona.core`)

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

---

