# Metadata

Metadata is a map of data attached to vars, providing information about definitions without affecting their values. Metadata enables documentation, compiler hints, and runtime introspection.

---

## Attaching Metadata

Metadata is attached using the `^` reader macro:

```clojure
;; Full map syntax
(def ^%{:doc "The ratio of circumference to diameter" :added "1.0"} pi 3.14159)

;; Shorthand for boolean true
(def ^:private helper-fn (fn* [x] ...))

;; Multiple shorthands
(def ^:private ^:process-bound *counter* 0)

;; Combined
(def ^:private ^%{:doc "Internal counter"} *counter* 0)
```

---

## Accessing Metadata

```clojure
(meta #'pi)
; → {:doc "The ratio of circumference to diameter"
;    :added "1.0"
;    :name pi
;    :ns #namespace[lona.core]}

(meta (var pi))  ; equivalent to above
```

---

## System-Recognized Keys

The following metadata keys have special meaning to the compiler and runtime.

### Definition Type

| Key | Type | Description |
|-----|------|-------------|
| `:native` | boolean | Var is bound to a VM intrinsic. The intrinsic must exist in the VM's registry. |
| `:special-form` | boolean | Var is a special form (implies `:native`). Cannot be used as a first-class value. |
| `:macro` | boolean | Var is a macro. Function is invoked at compile-time with unevaluated forms. |

### Documentation

| Key | Type | Description |
|-----|------|-------------|
| `:doc` | string | Documentation string (1-3 lines recommended). |
| `:arglists` | list | List of argument vectors, one per arity. Example: `([x] [x y] [x y & more])` |

### Visibility

| Key | Type | Description |
|-----|------|-------------|
| `:private` | boolean | Var is implementation detail. Not exported by `refer`. |

### Binding Behavior

| Key | Type | Description |
|-----|------|-------------|
| `:process-bound` | boolean | Each process has its own value. Inherited from parent at spawn time. |

### Compiler-Added

These keys are added automatically by the compiler:

| Key | Type | Description |
|-----|------|-------------|
| `:name` | symbol | The simple name of the var. |
| `:ns` | namespace | The namespace containing the var. |
| `:file` | string | Source file path. |
| `:line` | integer | Line number in source file. |

---

## Native Intrinsics

The `:native` metadata connects a var to a VM intrinsic function:

```clojure
(def ^%{:native true
        :doc "Returns the sum of nums. (+) returns 0."
        :arglists '([] [x] [x y] [x y & more])}
  +)
```

**Behavior:**

1. The VM looks up the symbol in its intrinsic registry
2. If found, the var is bound to the intrinsic function
3. If not found, an error is raised at load time

**Validation:** Declaring a native that doesn't exist in the VM is a load-time error. This ensures the Lonala source and VM implementation stay synchronized.

### Special Forms

Special forms are a subset of natives with `:special-form true`:

```clojure
(def ^%{:native true
        :special-form true
        :doc "Creates or updates a var binding."
        :arglists '([name] [name value])}
  def)
```

**Additional restrictions:**

- Cannot be passed as values (e.g., `(map if ...)` is an error)
- Evaluated by the compiler before var resolution

### Bootstrap

The `def` special form is the only var seeded by the VM. All other natives and special forms are registered using `def`:

```
VM starts
    ↓
VM seeds: def
    ↓
(def ^:native ^:special-form fn* ...)
(def ^:native ^:special-form match ...)
(def ^:native ^:special-form do ...)
(def ^:native ^:special-form quote ...)
    ↓
(def ^:native + ...)
(def ^:native first ...)
... all other intrinsics ...
    ↓
(def ^%{:macro true} defmacro ...)
    ↓
Derived macros and functions
```

---

## Macros

The `:macro` metadata marks a var as a macro:

```clojure
(def ^%{:macro true
        :doc "Evaluates test. If truthy, evaluates body."
        :arglists '([test & body])}
  when
  (fn* [test & body]
    `(match ~test
       false nil
       nil nil
       _ (do ~@body))))
```

**Behavior:**

1. At compile-time, when `when` appears in call position
2. The compiler invokes the function with unevaluated forms
3. The returned form replaces the original in the AST
4. Expansion continues recursively

---

## Process-Bound Vars

The `:process-bound` metadata creates per-process bindings:

```clojure
(def ^%{:process-bound true
        :doc "Current namespace for this process."}
  *ns*
  (find-ns 'lona.core))
```

**Behavior:**

- A single Var exists in the realm with the `PROCESS_BOUND` flag
- Each process can shadow the root value via a per-process binding table
- `def` on a process-bound var updates only the current process's binding (not the realm root)
- Spawned processes inherit parent's binding values at spawn time

**Use cases:**

- `*ns*` — current namespace (each process can be in different namespace)
- Dynamic context that shouldn't leak between processes

---

## Private Vars

The `:private` metadata marks implementation details:

```clojure
(def ^:private parse-options
  (fn* [opts] ...))
```

**Behavior:**

- Not included when using `(refer 'namespace)`
- Still accessible via fully-qualified name: `my.ns/parse-options`
- Signals intent, not enforcement

---

## Arglists Format

The `:arglists` key documents function signatures:

```clojure
;; Single arity
:arglists '([coll])

;; Multiple arities
:arglists '([coll] [coll not-found])

;; Variadic
:arglists '([x] [x y] [x y & more])

;; Destructuring shown for documentation
:arglists '([{:keys [host port timeout]}])
```

**Conventions:**

- Each inner vector is one valid arity
- Parameter names should be descriptive
- `& rest` indicates variadic
- Ordered from fewest to most arguments

---

## Metadata Equality

Metadata does not affect equality:

```clojure
(= (with-meta 'foo {:a 1})
   (with-meta 'foo {:b 2}))
; → true
```

Two values differing only in metadata are equal. This allows metadata to annotate without changing semantics.

---

## Summary Table

| Key | Added By | Purpose |
|-----|----------|---------|
| `:native` | User | Bind to VM intrinsic |
| `:special-form` | User | Mark as special form (implies `:native`) |
| `:macro` | User/`defmacro` | Mark as compile-time macro |
| `:doc` | User | Documentation string |
| `:arglists` | User/Compiler | Argument signatures |
| `:private` | User/`defn-` | Mark as internal |
| `:process-bound` | User | Per-process binding |
| `:name` | Compiler | Var's simple name |
| `:ns` | Compiler | Containing namespace |
| `:file` | Compiler | Source file |
| `:line` | Compiler | Source line |
