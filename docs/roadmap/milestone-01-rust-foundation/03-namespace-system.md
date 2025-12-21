## Phase 1.3: Namespace System

Implement namespaces for code organization.

---

### Task 1.3.1: Namespace Data Structure

**Description**: Create namespace representation and registry.

**Files to modify**:
- `crates/lona-kernel/src/namespace/mod.rs` (new)
- `crates/lona-kernel/src/namespace/registry.rs` (new)

**Requirements**:
- Namespace contains: name, mappings (symbol→var), aliases, refers
- Namespace registry maps names to namespaces
- Current namespace tracking (per-process later)
- Core namespace (`lona.core`) created at boot

**Tests**:
- Namespace creation
- Registry lookup
- Current namespace tracking
- Core namespace initialization

**Estimated effort**: 1 context window

---

### Task 1.3.2: Var System - Namespace Extension and Var Quote

**Description**: Extend the foundation Var type (from Task 1.1.7) with namespace field and implement `#'symbol` reader syntax.

**Dependencies**: Task 1.1.7 (Metadata System - Compiler Integration, which includes Var Type and Globals Refactor)

**Files to modify**:
- `crates/lona-core/src/value/var.rs` (extend)
- `crates/lonala-parser/src/lexer/mod.rs` (add `#'` reader macro)
- `crates/lonala-parser/src/parser/mod.rs`
- `crates/lonala-compiler/src/compiler/mod.rs`

**Requirements**:
- Extend `VarData` with `namespace: Option<symbol::Id>` field
- `#'symbol` reader syntax produces `(var symbol)` form
- `var` special form returns the Var itself (not its value)
- `var-get` native: get value from Var
- `var-set!` native: set value in Var (for dynamic vars)
- Vars resolve through namespace system when available

**Tests**:
- `#'x` returns Var object
- `(meta #'x)` returns var metadata
- `(var-get #'x)` returns value
- `(var-set! #'x new-val)` updates value (dynamic vars only)
- Var quote with qualified symbol: `#'ns/name`

**Note**: This task enables `defnative` for native function registration with metadata. See [defnative design](../../development/defnative.md).

**Estimated effort**: 1 context window

---

### Task 1.3.3: Namespace Declaration (`ns`)

**Description**: Implement `ns` special form.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lonala-compiler/src/compiler/namespace.rs` (new)

**Requirements**:
- `(ns name)` creates/switches to namespace
- `(ns name (:require ...))` with require clause
- `(ns name (:use ...))` with use clause
- `(ns name (:refer ...))` for selective import

**Tests**:
- Simple ns declaration
- ns with require
- ns with aliases
- ns with refer

**Estimated effort**: 1-2 context windows

---

### Task 1.3.4: Require/Use/Refer Implementation

**Description**: Implement namespace loading and importing.

**Files to modify**:
- `crates/lona-kernel/src/namespace/loader.rs` (new)
- `crates/lonala-compiler/src/compiler/namespace.rs`

**Requirements**:
- `(:require [ns.name :as alias])` loads and aliases
- `(:require [ns.name :refer [sym1 sym2]])` imports specific
- `(:use ns.name)` imports all public
- Circular dependency detection

**Tests**:
- Basic require
- Aliased require
- Selective refer
- Use all public
- Circular dependency error

**Estimated effort**: 2 context windows

---

### Task 1.3.5: Qualified Symbol Resolution

**Description**: Resolve `ns/name` symbols through namespace system.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- `foo/bar` resolves in namespace `foo`
- Unqualified symbols resolve in current namespace, then refers
- Auto-resolve to `lona.core` for core functions
- Compile-time resolution when possible

**Tests**:
- Qualified symbol resolution
- Unqualified resolution order
- Core auto-resolution
- Undefined symbol error

**Estimated effort**: 1 context window

---

### Task 1.3.6: Private Vars

**Description**: Implement `:private` metadata enforcement.

**Files to modify**:
- `crates/lona-kernel/src/namespace/mod.rs`
- `crates/lonala-compiler/src/compiler/mod.rs`

**Requirements**:
- `(def ^:private x ...)` marks var as private
- Private vars not included in `ns-publics`
- Access from other namespaces is compile-time error
- `ns-interns` includes private vars

**Tests**:
- Private var creation
- Private var access from same ns
- Private var blocked from other ns
- ns-publics excludes private

**Estimated effort**: 0.5 context windows

---

### Task 1.3.7: Dynamic Var Declaration

**Description**: Add support for `^:dynamic` metadata on vars to mark them as rebindable.

**Files to modify**:
- `crates/lona-core/src/value/var.rs`
- `crates/lona-kernel/src/namespace/mod.rs`

**Requirements**:
- `(def ^:dynamic *out* default-output)` marks var as dynamic
- Dynamic vars stored with flag in Var structure
- Non-dynamic vars cannot be rebound (compile-time error)
- `dynamic?` predicate to check var status
- Convention: dynamic vars named with `*earmuffs*`

**Tests**:
- Create dynamic var
- Create non-dynamic var
- Check `dynamic?` predicate
- Attempt to rebind non-dynamic (error)

**Estimated effort**: 0.5 context windows

---

### Task 1.3.8: `defnative` Special Form

**Description**: Implement `defnative` for registering native functions with full metadata support.

**Design**: See [defnative design](../../development/defnative.md) for full rationale.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`
- `crates/lona-kernel/src/vm/natives.rs`
- `lona/core.lona`

**Requirements**:
- `(defnative name docstring arglists)` syntax
- Verifies symbol exists in Rust native registry (load-time error if not)
- Creates function value with native implementation
- Attaches metadata: `{:doc "..." :arglists '(...) :native true}`
- Creates Var and binds in current namespace
- All natives in `lona/core.lona` use `defnative`

**Example**:
```clojure
(defnative cons
  "Returns a new list with x as first and coll as rest."
  [x coll])

(doc cons)        ; → "Returns a new list..."
(meta #'cons)     ; → {:doc "..." :arglists '([x coll]) :native true}
```

**Dependencies**:
- Task 1.1.5-1.1.7: Metadata System
- Task 1.3.2: Var System

**Tests**:
- defnative creates callable function
- Metadata accessible via `meta`
- `doc` and `arglists` work correctly
- Error on non-existent native
- `:native true` in metadata

**Estimated effort**: 1 context window
