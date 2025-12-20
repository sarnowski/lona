# Native Function Registration with `defnative`

This document describes the design for `defnative`, a mechanism to register native (Rust-implemented) functions in Lonala with full metadata support.

## Problem Statement

Native functions are currently defined in Rust and registered at runtime with no source representation in Lonala. This creates several problems:

1. **Discoverability**: Native functions are invisible to anyone reading Lonala source
2. **Documentation**: No docstrings, arglists, or metadata
3. **Tooling**: Pure Lonala parsers cannot understand the API
4. **Inconsistency**: Native and Lonala-defined functions have different introspection capabilities

## Design Goal

**Native functions must be inspectable identically to regular functions.**

All introspection tools (`meta`, `doc`, `arglists`) must work uniformly regardless of whether a function is native or Lonala-defined.

## Solution: `defnative` Special Form

```clojure
(defnative cons
  "Returns a new list with x as first and coll as rest."
  [x coll])

(defnative +
  "Returns the sum of nums. (+) returns 0."
  [& nums])
```

### Behavior

1. **Verify** that the symbol exists in the Rust native registry (load-time error if not)
2. **Create** a function value with:
   - Native function pointer as implementation
   - Metadata: `{:doc "..." :arglists '([x coll]) :native true}`
3. **Bind** to the symbol via the Var system

### Result: Uniform Introspection

```clojure
(doc cons)        ; → "Returns a new list with x as first..."
(arglists cons)   ; → ([x coll])
(meta #'cons)     ; → {:doc "..." :arglists '([x coll]) :native true}
(fn? cons)        ; → true
(native? cons)    ; → true
```

## Design Rationale

### Why Metadata Lives in Lonala (Not Rust)

The Rust registry holds only implementations:

```rust
pub struct Registry {
    functions: BTreeMap<symbol::Id, NativeFn>,
}
```

All metadata lives in Lonala source, using the standard metadata system that regular functions use. This provides:

- **Single infrastructure**: No special-case code for native function metadata
- **Lonala-first philosophy**: The public API is defined in Lonala
- **Source representation**: All functions are declared in source files

### Addressing the "Drift" Concern

A valid concern: the Lonala arglists could drift from the Rust implementation.

**Mitigation:**
- Arglists in Lonala are documentation, not enforcement
- If Rust changes arity, calls fail at runtime with immediate feedback
- This is exactly how Clojure handles Java interop
- The `:native true` metadata signals that the implementation is external

## Dependencies

`defnative` requires these systems to be implemented first:

| Task | Description | Why Required |
|------|-------------|--------------|
| 1.1.4 | Metadata System - Value Storage | Values must carry metadata |
| 1.1.5 | Metadata System - Reader Syntax | Parse `^{:doc "..."}` etc. |
| 1.1.6 | Metadata System - Compiler Integration | `def` must support metadata |
| 1.3.2 | Var System | `#'symbol` returns Var with metadata |

Once these are complete, `defnative` can be implemented as a special form that:
1. Looks up the native function in the registry
2. Creates a Var with the function value and metadata
3. Binds it in the current namespace

## Implementation Sketch

### Rust Side (Registry)

```rust
// Minimal registry - just function pointers
pub struct NativeRegistry {
    functions: BTreeMap<symbol::Id, NativeFn>,
}

impl NativeRegistry {
    pub fn get(&self, symbol: symbol::Id) -> Option<NativeFn> {
        self.functions.get(&symbol).copied()
    }
}
```

### Compiler/VM Side

`defnative` compiles to bytecode that:
1. Pushes the symbol to look up
2. Pushes the metadata map (doc, arglists, etc.)
3. Invokes `OpDefNative` which:
   - Looks up the symbol in the native registry
   - Creates a NativeFunction value
   - Creates a Var with the value and metadata
   - Binds the Var in the current namespace

### Usage in `lona/core.lona`

```clojure
(ns lona.core)

;; Collection primitives
(defnative cons
  "Returns a new list with x as the first element and coll as the rest."
  [x coll])

(defnative first
  "Returns the first element of coll, or nil if empty."
  [coll])

(defnative rest
  "Returns a seq of items after the first, or empty list if none."
  [coll])

;; Arithmetic primitives
(defnative +
  "Returns the sum of nums. (+) returns 0. (+ x) returns x."
  [& nums])

(defnative -
  "Subtracts nums from x. (- x) negates x."
  [x & nums])

;; ... etc
```

## Alternative Considered: Rust as Single Source of Truth

An alternative design was considered where metadata lives in Rust:

```rust
#[lona_native(
    name = "+",
    doc = "Returns the sum of nums.",
    arglists = "[& nums]"
)]
fn native_add(args: &[Value]) -> Result<Value, Error> { ... }
```

This was rejected because:
1. **Against Lonala-first principle**: The API should be defined in Lonala
2. **Tooling complexity**: Pure Lonala tools would need to parse Rust
3. **Separate infrastructure**: Would require a parallel metadata system
4. **Less discoverable**: Users read Lonala source, not Rust source

## Future Considerations

### Multi-Arity Native Functions

```clojure
(defnative get
  "Returns the value for key, or not-found/nil."
  ([map key] [map key not-found]))
```

### Deprecated Natives

```clojure
(defnative old-fn
  "DEPRECATED: Use new-fn instead."
  {:deprecated "1.0"}
  [x])
```

### Private Natives

```clojure
(defnative- internal-impl
  "Internal use only."
  [x])
```

These extensions follow naturally from the Var system's support for these features.
