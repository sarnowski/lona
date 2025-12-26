# Namespaces
> **Status**: *Data structures implemented (Task 1.3.1). Language integration planned.*

Namespaces organize code and prevent name collisions. The runtime maintains a registry of namespaces, each containing vars (defined symbols) and references to vars from other namespaces.

## 12.1 Namespace Declaration (Planned)

```clojure
(ns my.app
  (:require [lona.core :as c]
            [lona.string :refer [join]]))
```

## 12.2 Qualified References (Planned)

```clojure
lona.core/map        ; fully qualified
c/map                ; using alias
join                 ; referred directly
```

## 12.3 Creating and Switching (Planned)

```clojure
(in-ns 'my.namespace)  ; switch to namespace
(ns-name *ns*)         ; get current namespace name
```

---

