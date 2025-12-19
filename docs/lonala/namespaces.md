# Namespaces
> **Status**: *Planned for Phase 6*

Namespaces organize code and prevent name collisions.

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

