## Milestone 1: Rust Foundation

**Goal**: Complete all Rust code required for the Lona runtime. After this milestone, no new Rust code should be needed.

**Deliverable**: A fully functional VM with processes, GC, domains, condition/restart system, debug infrastructure (Two-Mode Architecture), and all native primitives.

**Phases**: 12 phases covering language features, process model, GC, domain isolation, fault tolerance, native primitives, condition system, introspection, and debug infrastructure.

---

## Current State

### Completed (Phases 1-4.4)

| Component | Status | Details |
|-----------|--------|---------|
| Lexer | ✅ Complete | Full S-expression tokenization |
| Parser | ✅ Complete | AST generation, reader macros (`'`, `` ` ``, `~`, `~@`) |
| Bytecode Compiler | ✅ Complete | 25 opcodes, register-based |
| VM Interpreter | ✅ Complete | Bytecode execution, call stack |
| Special Forms | ✅ Complete | `def`, `let`, `if`, `do`, `fn`, `quote`, `syntax-quote`, `defmacro` |
| Macro System | ✅ Complete | Compile-time expansion, introspection |
| Core Values | ✅ Complete | Nil, Bool, Integer, Float, Ratio, Symbol, String, List, Vector, Map |
| Basic Natives | ✅ Complete | `cons`, `first`, `rest`, `list`, `concat` |
| Collection Constructors | ✅ Complete | `vector`, `hash-map`, `vec` (native bootstrap) |
| REPL (Rust) | ✅ Complete | Interactive evaluation (native bootstrap) |
| Rest Arguments | ✅ Complete | `& rest` syntax in functions and macros |

### Missing (Required for Milestone 1)

| Component | Status | Priority |
|-----------|--------|----------|
| Keyword Value Type | ✅ Complete | High |
| Set Value Type | ✅ Complete | High |
| Binary Value Type | ❌ Not Started | High |
| Metadata System | ❌ Not Started | High |
| Closures | ❌ Not Started | Critical |
| Multi-Arity Functions | ❌ Not Started | High |
| Destructuring | ❌ Not Started | Critical |
| [Proper Tail Calls](../development/tco.md) | ❌ Not Started | Critical |
| Namespace System | ❌ Not Started | High |
| Process Model | ❌ Not Started | Critical |
| Green Thread Scheduler | ❌ Not Started | Critical |
| Garbage Collection | ❌ Not Started | Critical |
| Domain Isolation | ❌ Not Started | Critical |
| Inter-Domain IPC | ❌ Not Started | Critical |
| MMIO/DMA/IRQ Primitives | ❌ Not Started | Critical |
| All Type Predicates | ⚠️ Partial | High |
| Bitwise Operations | ❌ Not Started | High |
| Atom Primitives | ❌ Not Started | Medium |
| Sorted Collections | ❌ Not Started | Low |
| Condition/Restart System | ❌ Not Started | Critical |
| Introspection System | ❌ Not Started | High |
| Debug Infrastructure | ❌ Not Started | High |

---

### Phase 1.0: Arithmetic Primitives

Arithmetic must come first — nearly everything else depends on it.

#### Task 1.0.1: Native Addition and Subtraction

**Description**: Implement `+` and `-` in the VM natives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/vm/numeric.rs`

**Requirements**:
- `+` with zero args returns 0
- `+` with one arg returns that arg
- `+` with multiple args sums them
- `-` with one arg negates it
- `-` with multiple args subtracts subsequent from first
- Handle Integer, Float, and Ratio combinations
- Proper type promotion (Int+Float→Float, Int+Ratio→Ratio)

**Tests**:
- All type permutations (Int+Int, Int+Float, Ratio+Int, etc.)
- Edge cases (overflow to bigint, negative numbers)
- Zero and one argument cases

**Estimated effort**: 1 context window

---

#### Task 1.0.2: Native Multiplication and Division

**Description**: Implement `*` and `/` in the VM natives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/vm/numeric.rs`

**Requirements**:
- `*` with zero args returns 1
- `*` with one arg returns that arg
- `/` with one arg returns reciprocal (1/x)
- Division of integers produces Ratio when inexact
- Division by zero handling (error tuple or NaN for floats)
- Handle all numeric type combinations

**Tests**:
- All type permutations
- Exact division (6/2 → 3 Integer)
- Inexact division (5/2 → 5/2 Ratio)
- Division by zero
- Reciprocal cases

**Estimated effort**: 1 context window

---

#### Task 1.0.3: Modulo

**Description**: Implement `mod` native.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `mod` returns remainder of division
- Float modulo behavior matches IEEE 754
- Ratio modulo support

**Tests**:
- Integer modulo
- Negative number modulo
- Float modulo behavior

**Estimated effort**: 0.5 context windows

> **Note**: `inc` and `dec` are NOT native—they are trivially implemented in Lonala as `(defn inc [x] (+ x 1))` and `(defn dec [x] (- x 1))`. See Phase 2.7 Numeric Functions.

---

#### Task 1.0.4: Comparison - Equality

**Description**: Implement `=` for value equality.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-core/src/value/mod.rs`

**Requirements**:
- Deep structural equality for collections
- Numeric equality across types (1 = 1.0 = 1/1)
- `=` with multiple args checks all pairwise
- NaN is not equal to anything (including itself)
- Symbol equality by identity (interned)

**Tests**:
- All primitive types
- Nested collections
- Mixed numeric types
- NaN behavior
- Multiple argument form

**Estimated effort**: 1 context window

---

#### Task 1.0.5: Comparison - Ordering

**Description**: Implement `<`, `>`, `<=`, `>=` natives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- Work on all numeric types
- Cross-type comparison (1 < 1.5 < 2)
- String comparison (lexicographic)
- Multiple args: `(< a b c)` means a < b < c
- Error on non-comparable types

**Tests**:
- All numeric types
- Cross-type comparison
- String comparison
- Multiple argument chaining
- Error cases

**Estimated effort**: 1 context window

---

### Phase 1.1: Core Value Type Extensions

Extend the value system with missing fundamental types.

#### Task 1.1.1: Keyword Value Type

**Description**: Add `Keyword` as a distinct value type with interning.

**Files to modify**:
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-core/src/value/keyword.rs` (new)
- `crates/lonala-parser/src/parser/mod.rs`

**Requirements**:
- Keywords are interned (like symbols) for fast equality
- Keywords are self-evaluating (evaluate to themselves)
- Keywords can be used as map keys
- Support qualified keywords (`:ns/name`)
- Parser produces `Value::Keyword` for `:foo` syntax

**Tests**:
- Keyword creation and interning
- Keyword equality (interned comparison)
- Keywords as map keys
- Keyword self-evaluation
- Qualified keyword parsing

**Estimated effort**: 1 context window

---

#### Task 1.1.2: Set Value Type

**Description**: Add `Set` as a persistent hash set using HAMT.

**Files to modify**:
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-core/src/value/set.rs` (new)
- `crates/lona-core/src/hamt/` (extend for sets)
- `crates/lonala-parser/src/parser/mod.rs`

**Requirements**:
- Immutable persistent set with structural sharing
- O(log32 n) for insert, remove, contains
- Parser supports `#{1 2 3}` literal syntax
- Duplicate detection in literals (error on `#{1 1}`)
- Set equality is structural

**Tests**:
- Set creation from literal
- Set operations (conj, disj, contains?)
- Set equality
- Duplicate detection in literals
- Empty set `#{}`

**Estimated effort**: 1-2 context windows

---

#### Task 1.1.3: Collection Literal Syntax

**Description**: Implement compiler support for vector, map, and set literal syntax.

**Dependencies**: Task 1.1.1 (Keywords for map keys), Task 1.1.2 (Set type)

**Files to modify**:
- `crates/lonala-parser/src/lexer/mod.rs` (add `#` dispatch character)
- `crates/lonala-compiler/src/compiler/mod.rs` (emit bytecode for literals)

**Requirements**:
- `[1 2 3]` compiles to vector construction
- `{:a 1 :b 2}` compiles to hash-map construction
- `#{1 2 3}` compiles to hash-set construction (requires lexer update for `#` prefix)
- Empty literals: `[]`, `{}`, `#{}` all work
- Nested literals: `[{:a [1 2]}]`

**Tests**:
- Vector literal creates vector
- Map literal creates hash-map
- Set literal creates hash-set
- Empty collection literals
- Nested collection literals
- Duplicate key in map literal (error or last-wins)
- Duplicate element in set literal (error)

**Estimated effort**: 1-2 context windows

---

#### Task 1.1.4: Binary Value Type

**Description**: Add `Binary` for raw byte buffers (mutable, for drivers).

**Files to modify**:
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-core/src/value/binary.rs` (new)

**Requirements**:
- Heap-allocated byte buffer
- Mutable (unlike other Lonala types) for efficiency
- Zero-copy slicing (views into same buffer)
- Track physical address for DMA-capable buffers
- Reference counting for shared access

**Tests**:
- Binary creation with size
- Byte get/set operations
- Slice creation (zero-copy)
- Buffer copy operations
- Physical address tracking (when DMA)

**Estimated effort**: 1 context window

---

#### Task 1.1.5: Metadata System - Value Storage

**Description**: Add optional metadata map storage to values that support it.

**Files to modify**:
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-core/src/value/list.rs`
- `crates/lona-core/src/value/vector.rs`
- `crates/lona-core/src/value/map.rs`
- `crates/lona-core/src/value/symbol.rs`

**Requirements**:
- List, Vector, Map, Symbol can carry metadata
- Metadata is a Map (or nil)
- Metadata does NOT affect equality or hash
- `with-meta` creates new value with metadata
- `meta` retrieves metadata (or nil)

**Tests**:
- Attach metadata to each supported type
- Metadata doesn't affect equality
- Metadata doesn't affect hash
- Nested metadata
- nil metadata is valid

**Estimated effort**: 1-2 context windows

---

#### Task 1.1.6: Metadata System - Reader Syntax

**Description**: Add parser support for `^` metadata reader macro.

**Files to modify**:
- `crates/lonala-parser/src/lexer/mod.rs`
- `crates/lonala-parser/src/parser/mod.rs`

**Requirements**:
- `^{:key val}` attaches metadata map to next form
- `^:keyword` is shorthand for `^{:keyword true}`
- `^Type` is shorthand for `^{:tag Type}`
- Multiple metadata items merge: `^:a ^:b x` → `^{:a true :b true} x`

**Tests**:
- Full metadata map syntax
- Keyword shorthand
- Type tag shorthand
- Multiple metadata merge
- Metadata on various form types

**Estimated effort**: 1 context window

---

#### Task 1.1.7: Metadata System - Compiler Integration

**Description**: Integrate metadata into compilation and var definitions.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lona-kernel/src/vm/globals.rs`

**Requirements**:
- `def` with docstring sets `:doc` metadata
- Compiler tracks `:file`, `:line`, `:column` for definitions
- `defmacro` sets `:macro true` on var
- Var metadata separate from value metadata

**Tests**:
- Docstring becomes `:doc` metadata
- Source location tracking
- Macro metadata flag
- Var vs value metadata distinction

**Estimated effort**: 1 context window

---

### Phase 1.2: Language Feature Completion

Complete core language features required for idiomatic Lonala.

#### Task 1.2.1: Multi-Arity Function Support

**Description**: Support multiple arities in function definitions.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/functions.rs`
- `crates/lona-core/src/value/function.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- `(fn ([x] ...) ([x y] ...))` syntax for multi-arity
- Named multi-arity: `(fn name ([x] ...) ([x y] ...))`
- Dispatch based on argument count
- Exact arity match takes priority over variadic
- Store multiple bodies in Function value

**Tests**:
- Two-arity function
- Three+ arity function
- Named multi-arity
- Arity dispatch correctness
- Variadic fallback when no exact match

**Estimated effort**: 1-2 context windows

---

#### Task 1.2.2: Closure Implementation

**Description**: Enable functions to capture lexical environment.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lonala-compiler/src/compiler/functions.rs`
- `crates/lona-core/src/value/function.rs`
- `crates/lona-core/src/chunk/mod.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- Analyze free variables in function body
- Capture values at function creation time (copy semantics)
- Store captured values in Function value
- New opcode `GetUpvalue` to access captured values
- Nested closures work correctly

**Tests**:
- Simple closure capturing one variable
- Closure capturing multiple variables
- Nested closures
- Closure in loop (each iteration captures current value)
- Closure returned from function

**Estimated effort**: 2-3 context windows

---

#### Task 1.2.3: Sequential Destructuring

**Description**: Support `[a b & rest]` pattern in bindings.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lonala-compiler/src/compiler/destructure.rs` (new)

**Requirements**:
- `[a b c]` binds sequential elements
- `[a b & rest]` binds first two, rest to remaining
- `[a _ c]` skips element with `_`
- `:as name` binds entire collection
- Works in `let`, `fn` params, `loop`

**Tests**:
- Basic vector destructuring
- Rest collection with `&`
- Underscore for ignored elements
- `:as` for whole collection binding
- Nested sequential destructuring

**Estimated effort**: 2 context windows

---

#### Task 1.2.4: Associative Destructuring

**Description**: Support `{:keys [a b]}` pattern in bindings.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/destructure.rs`

**Requirements**:
- `{:keys [a b]}` extracts keyword keys
- `{:strs [a b]}` extracts string keys
- `{:syms [a b]}` extracts symbol keys
- `:or {a default}` provides defaults
- `:as name` binds entire map
- `{a :key-a}` binds specific key to name

**Tests**:
- `:keys` destructuring
- `:strs` destructuring
- `:or` defaults
- `:as` whole map binding
- Explicit key-to-name binding

**Estimated effort**: 1-2 context windows

---

#### Task 1.2.5: Nested Destructuring

**Description**: Support arbitrary nesting of destructuring patterns.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/destructure.rs`

**Requirements**:
- `[[a b] [c d]]` nested sequential
- `{:keys [{:keys [x]}]}` nested associative
- `[{:keys [a]} b]` mixed nesting
- Arbitrary depth supported

**Tests**:
- Two-level sequential nesting
- Two-level associative nesting
- Mixed sequential/associative
- Three+ level nesting
- Complex real-world patterns

**Estimated effort**: 1 context window

---

#### Task 1.2.6: Proper Tail Calls - Compiler

**Description**: Add tail position tracking to the compiler and emit `TailCall` opcode for calls in tail position. See [docs/development/tco.md](../development/tco.md) for full design.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lonala-compiler/src/compiler/special_forms.rs`
- `crates/lonala-compiler/src/compiler/functions.rs`
- `crates/lonala-compiler/src/compiler/calls.rs`

**Requirements**:
- Add `in_tail_position: bool` field to `Compiler` struct
- Create `compile_expr_in_context(&mut self, expr, tail: bool)` method
- Propagate tail position: `fn` body last expr, `do` last expr, `if` branches, `let` body
- `compile_call()` emits `TailCall` when `in_tail_position` is true

**Tests**:
- `TailCall` emitted for: `(fn [x] (f x))`
- `TailCall` emitted for: `(fn [x] (if c (f x) (g x)))`
- `TailCall` emitted for: `(fn [x] (do (println x) (f x)))`
- `Call` emitted for: `(fn [x] (+ 1 (f x)))` (not tail position)

**Estimated effort**: 1 context window

---

#### Task 1.2.7: Proper Tail Calls - VM Trampoline

**Description**: Restructure the VM interpreter to use a trampoline loop, enabling tail calls without Rust stack growth. See [docs/development/tco.md](../development/tco.md) for full design.

**Files to modify**:
- `crates/lona-kernel/src/vm/interpreter/mod.rs`
- `crates/lona-kernel/src/vm/interpreter/ops_control.rs`
- `crates/lona-kernel/src/vm/frame.rs`

**Requirements**:
- Define `RunResult` enum with `Return(Value)` and `TailCall { chunk, base, arguments }`
- Restructure `run()` as outer trampoline loop calling `run_inner()`
- Implement `op_tail_call()` returning `RunResult::TailCall` instead of recursing
- Do NOT increment `call_depth` for tail calls
- Store `Arc<Chunk>` in trampoline loop for frame swapping

**Tests**:
- Deep tail recursion (10,000+ calls) without stack overflow
- Mutual tail recursion between two functions
- Mix of tail and non-tail calls in same function
- Tail call preserves correct return value

**Estimated effort**: 2 context windows

---

#### Task 1.2.8: Proper Tail Calls - Integration Tests

**Description**: Comprehensive integration tests for proper tail calls. See [docs/development/tco.md](../development/tco.md) for full design.

**Files to modify**:
- `crates/lona-spec-tests/src/tco.rs` (new)
- `crates/lona-spec-tests/src/lib.rs`

**Requirements**:
- Self-recursion: `(defn countdown [n] (if (= n 0) :done (countdown (- n 1))))`
- Mutual recursion: `even?`/`odd?` calling each other
- Accumulator pattern with 100,000+ iterations
- State machine pattern (3+ mutually recursive functions)
- Verify non-tail calls DO overflow (negative test)

**Tests**:
- All patterns above with n=100,000
- Correct return values preserved
- Stack overflow on intentionally non-tail recursive code

**Estimated effort**: 1 context window

---

#### Task 1.2.9: Pattern Matching - Core Infrastructure

**Description**: Build pattern matching engine for `receive` and `case`.

**Files to modify**:
- `crates/lona-kernel/src/vm/pattern.rs` (new)
- `crates/lona-core/src/value/mod.rs`

**Requirements**:
- Match literals (numbers, strings, keywords, nil, booleans)
- Match symbols (bind to value)
- Match collections (vector, list, map patterns)
- Match with guards (`:when` clauses)
- Return bindings map on successful match

**Tests**:
- Literal matching (all types)
- Symbol binding
- Vector pattern matching
- Map pattern matching
- Guard clauses
- Nested patterns

**Estimated effort**: 2 context windows

---

#### Task 1.2.10: Case Special Form

**Description**: Implement `case` for value-based dispatch.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lonala-compiler/src/compiler/case.rs` (new)

**Requirements**:
- `(case expr pattern1 result1 pattern2 result2 ...)`
- Patterns evaluated at compile time (must be constants)
- Optional default with `:else` or no pattern
- Efficient dispatch (jump table for small integers)

**Tests**:
- Integer case
- Keyword case
- String case
- Default clause
- No match error

**Estimated effort**: 1-2 context windows

---

#### Task 1.2.11: Gensym Implementation

**Description**: Implement `gensym` for hygienic macro expansion.

**Files to modify**:
- `crates/lona-core/src/symbol.rs`
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(gensym)` returns unique symbol `G__123`
- `(gensym "prefix")` returns `prefix__123`
- Counter is global and monotonic
- Symbols are interned but guaranteed unique

**Tests**:
- Basic gensym uniqueness
- Prefix gensym
- Sequential calls produce different symbols
- Interning works correctly

**Estimated effort**: 0.5 context windows

---

### Phase 1.3: Namespace System

Implement namespaces for code organization.

#### Task 1.3.1: Namespace Data Structure

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

#### Task 1.3.2: Var System

**Description**: Implement first-class Vars for namespace bindings.

**Files to modify**:
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-core/src/value/var.rs` (new)

**Requirements**:
- Var holds: namespace, name, value, metadata
- `#'symbol` syntax returns Var (not value)
- `var-get`, `var-set!` for access
- Vars support metadata (separate from value metadata)

**Tests**:
- Var creation
- Var get/set
- Var metadata
- Var quote reader macro

**Note**: This task enables `defnative` for native function registration with metadata. See [defnative design](../development/defnative.md).

**Estimated effort**: 1 context window

---

#### Task 1.3.3: Namespace Declaration (`ns`)

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

#### Task 1.3.4: Require/Use/Refer Implementation

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

#### Task 1.3.5: Qualified Symbol Resolution

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

#### Task 1.3.6: Private Vars

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

#### Task 1.3.7: Dynamic Var Declaration

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

#### Task 1.3.8: Per-Process Binding Stack

> ⚠️ **DEPENDENCY**: This task modifies `process/pcb.rs` which is created in Task 1.4.1. **Implement after Task 1.4.1**, not in sequential order. See Phase 1.4b below.

**Description**: Each process maintains a stack of dynamic binding frames.

**Files to modify**:
- `crates/lona-kernel/src/process/pcb.rs`
- `crates/lona-kernel/src/process/bindings.rs` (new)

**Requirements**:
- Binding frame: Map of Var → Value
- Stack of frames per process
- Lookup checks frames top-down, falls back to root value
- Push/pop frame operations
- Frame automatically popped on scope exit

**Tests**:
- Push binding frame
- Lookup finds bound value
- Pop restores previous
- Nested frames work correctly

**Estimated effort**: 1 context window

---

#### Task 1.3.9: `binding` Special Form

> ⚠️ **DEPENDENCY**: Depends on Task 1.3.8. **Implement after Task 1.4.1**. See Phase 1.4b below.

**Description**: Implement `binding` to establish dynamic bindings in scope.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lonala-compiler/src/compiler/binding.rs` (new)
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- `(binding [*out* new-out] body...)` syntax
- Pushes frame before body, pops after (including on error)
- Only dynamic vars can be bound (compile-time check)
- Bindings visible to all code called from body
- New opcodes: `PushBindingFrame`, `PopBindingFrame`

**Tests**:
- Simple binding
- Nested bindings
- Binding visible in called functions
- Frame popped on normal exit
- Frame popped on error exit

**Estimated effort**: 1-2 context windows

---

#### Task 1.3.10: `defnative` Special Form

**Description**: Implement `defnative` for registering native functions with full metadata support.

**Design**: See [defnative design](../development/defnative.md) for full rationale.

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
- Task 1.1.4-1.1.6: Metadata System
- Task 1.3.2: Var System

**Tests**:
- defnative creates callable function
- Metadata accessible via `meta`
- `doc` and `arglists` work correctly
- Error on non-existent native
- `:native true` in metadata

**Estimated effort**: 1 context window

---

### Phase 1.4: Process Model

Implement BEAM-style lightweight processes.

#### Task 1.4.1: Process Data Structure

**Description**: Define the process control block (PCB).

**Files to modify**:
- `crates/lona-kernel/src/process/mod.rs` (new)
- `crates/lona-kernel/src/process/pcb.rs` (new)

**Requirements**:
- PID (globally unique identifier)
- Status (running, waiting, suspended, terminated)
- Priority level
- Heap reference
- Stack/registers state
- Mailbox reference
- Links and monitors
- Reduction counter
- Current namespace
- Domain reference

**Tests**:
- PCB creation
- Status transitions
- Field accessors

**Estimated effort**: 1 context window

---

#### Task 1.4.2: Per-Process Heap

**Description**: Implement isolated heap per process.

**Files to modify**:
- `crates/lona-kernel/src/memory/heap.rs` (new)
- `crates/lona-kernel/src/process/pcb.rs`

**Requirements**:
- Each process has own heap allocator
- Heap grows on demand (within domain limits)
- Values allocated in owning process's heap
- Cross-process references require copying

**Tests**:
- Heap creation per process
- Allocation in process heap
- Heap isolation verification
- Heap growth

**Estimated effort**: 1-2 context windows

---

#### Task 1.4.3: Process Registry

**Description**: Track all processes and enable lookup.

**Files to modify**:
- `crates/lona-kernel/src/process/registry.rs` (new)

**Requirements**:
- Global PID → Process mapping
- Named process registration
- Process enumeration
- Dead process cleanup

**Tests**:
- Process registration
- Name lookup
- PID lookup
- Cleanup on termination

**Estimated effort**: 1 context window

---

#### Task 1.4.4: Mailbox Implementation

**Description**: Per-process message queue.

**Files to modify**:
- `crates/lona-kernel/src/process/mailbox.rs` (new)

**Requirements**:
- FIFO queue of messages
- Messages are copied Values
- Save queue for selective receive
- Timeout support

**Tests**:
- Message enqueue
- Message dequeue
- Save queue operation
- Empty mailbox behavior

**Estimated effort**: 1 context window

---

#### Task 1.4.5: Scheduler - Run Queue

**Description**: Implement run queue for process scheduling.

**Files to modify**:
- `crates/lona-kernel/src/scheduler/mod.rs` (new)
- `crates/lona-kernel/src/scheduler/queue.rs` (new)

**Requirements**:
- Priority-based run queues
- O(1) enqueue/dequeue
- Process state transitions
- Fair scheduling within priority

**Tests**:
- Enqueue/dequeue operations
- Priority ordering
- State transition handling

**Estimated effort**: 1 context window

---

#### Task 1.4.6: Scheduler - Context Switching

**Description**: Save/restore process execution state.

**Files to modify**:
- `crates/lona-kernel/src/scheduler/context.rs` (new)
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- Save VM state (IP, registers, stack frames)
- Restore VM state on resume
- Minimal overhead for switch
- Handle mid-instruction yields

**Tests**:
- Context save/restore roundtrip
- Multiple process interleaving
- State integrity verification

**Estimated effort**: 2 context windows

---

#### Task 1.4.7: Scheduler - Cooperative Yielding

**Description**: Implement yield points for cooperative scheduling.

**Files to modify**:
- `crates/lona-kernel/src/scheduler/mod.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- Yield on `receive` when no matching message
- Yield on explicit `yield` call
- Yield on timer expiration
- Resume when condition met

**Tests**:
- Yield on empty receive
- Yield on explicit call
- Resume on message arrival

**Estimated effort**: 1 context window

---

#### Task 1.4.8: Scheduler - Preemptive Scheduling

**Description**: Implement reduction counting for preemption.

**Files to modify**:
- `crates/lona-kernel/src/scheduler/mod.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- Count reductions (roughly: bytecode ops)
- Preempt after N reductions (configurable)
- No process can monopolize CPU
- Reduction count visible for debugging

**Tests**:
- Preemption after reduction limit
- Fair time slicing
- Reduction counter accuracy

**Estimated effort**: 1 context window

---

#### Task 1.4.9: Spawn Primitive

**Description**: Implement `spawn` to create new processes.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- `(spawn fn)` creates process running fn
- `(spawn fn args)` with initial arguments
- Returns PID immediately
- New process starts in runnable state

**Tests**:
- Basic spawn
- Spawn with arguments
- PID uniqueness
- Process starts execution

**Estimated effort**: 1 context window

---

#### Task 1.4.10: Self and Exit Primitives

**Description**: Implement `self` and `exit`.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- `(self)` returns current process PID
- `(exit reason)` terminates with reason
- Exit reason sent to linked processes
- Cleanup process resources

**Tests**:
- Self returns correct PID
- Normal exit
- Exit with error reason
- Resource cleanup

**Estimated effort**: 1 context window

---

#### Task 1.4.11: Send Primitive - Intra-Domain

**Description**: Implement message sending within same domain.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- `(send pid msg)` copies message to target mailbox
- Deep copy of message (process isolation)
- Wake target if waiting for message
- Returns message (for chaining)

**Tests**:
- Basic send
- Message copying verification
- Wake on send
- Send to self

**Estimated effort**: 1 context window

---

#### Task 1.4.12: Receive Special Form - Basic

**Description**: Implement basic `receive` with patterns.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lonala-compiler/src/compiler/receive.rs` (new)
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- `(receive pattern1 body1 pattern2 body2 ...)`
- Pattern match against mailbox messages
- First matching message removed from mailbox
- Blocks if no match (yield to scheduler)

**Tests**:
- Simple pattern receive
- Multiple patterns
- Pattern with binding
- Blocking behavior

**Estimated effort**: 2 context windows

---

#### Task 1.4.13: Receive with Timeout

**Description**: Add timeout support to `receive`.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/receive.rs`
- `crates/lona-kernel/src/scheduler/timer.rs` (new)

**Requirements**:
- `(receive ... (after ms expr))` with timeout
- Timer integration with scheduler
- Timeout triggers alternative expression
- Cancel timer if message received

**Tests**:
- Receive with timeout (timeout triggers)
- Receive with timeout (message arrives first)
- Zero timeout (poll)
- Multiple timers

**Estimated effort**: 1-2 context windows

---

#### Task 1.4.14: Selective Receive

**Description**: Implement BEAM-style selective receive with save queue.

**Files to modify**:
- `crates/lona-kernel/src/process/mailbox.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- Non-matching messages saved for later
- After receive, save queue restored to mailbox
- Preserves message ordering
- Avoids re-scanning saved messages

**Tests**:
- Selective receive skips non-matching
- Save queue restored
- Message ordering preserved
- Performance with many messages

**Estimated effort**: 1 context window

---

### Phase 1.4b: Dynamic Bindings (Deferred from Phase 1.3)

These tasks were defined in Phase 1.3 but must be implemented after Phase 1.4 because they modify `process/pcb.rs`.

> **Implementation Order**: Complete Tasks 1.4.1-1.4.14 first, then return here.

**Tasks in this phase**:
- Task 1.3.8: Per-Process Binding Stack
- Task 1.3.9: `binding` Special Form

See Phase 1.3 for full task specifications.

---

### Phase 1.5: Garbage Collection

Implement per-process incremental garbage collection.

#### Task 1.5.1: Root Discovery

**Description**: Identify GC roots for a process.

**Files to modify**:
- `crates/lona-kernel/src/gc/mod.rs` (new)
- `crates/lona-kernel/src/gc/roots.rs` (new)

**Requirements**:
- Stack frame locals are roots
- Global vars in process's namespace are roots
- Mailbox messages are roots
- Closures' captured values are roots

**Tests**:
- Root enumeration
- All root types discovered
- No roots missed

**Estimated effort**: 1 context window

---

#### Task 1.5.2: Tri-Color Marking

**Description**: Implement tri-color marking algorithm.

**Files to modify**:
- `crates/lona-kernel/src/gc/marker.rs` (new)

**Requirements**:
- White (unvisited), Gray (pending), Black (done)
- Incremental: configurable work per step
- Track object color efficiently
- Handle cycles correctly

**Tests**:
- Simple object graph marking
- Cyclic structures
- Incremental progress
- All reachable marked black

**Estimated effort**: 1-2 context windows

---

#### Task 1.5.3: Write Barrier

**Description**: Implement write barrier for incremental correctness.

**Files to modify**:
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-kernel/src/gc/barrier.rs` (new)

**Requirements**:
- Detect when black object points to white
- Mark target gray (or re-mark source)
- Minimal overhead on writes
- Works with all mutable operations

**Tests**:
- Barrier triggered on mutation
- Correctness maintained during mutation
- Overhead measurement

**Estimated effort**: 1-2 context windows

---

#### Task 1.5.4: Sweep Phase

**Description**: Reclaim unmarked memory.

**Files to modify**:
- `crates/lona-kernel/src/gc/sweep.rs` (new)
- `crates/lona-kernel/src/memory/heap.rs`

**Requirements**:
- Identify unmarked (white) objects
- Return memory to heap
- Update heap statistics
- Handle finalizers (if any)

**Tests**:
- Sweep reclaims unreachable
- Memory returned to heap
- Statistics accuracy

**Estimated effort**: 1 context window

---

#### Task 1.5.5: Generational Optimization

**Description**: Add generational collection for reduced pause times.

**Files to modify**:
- `crates/lona-kernel/src/gc/mod.rs`
- `crates/lona-kernel/src/gc/generations.rs` (new)

**Requirements**:
- Young generation (frequently collected)
- Old generation (rarely collected)
- Promotion after N survivals
- Remember set for old→young references

**Tests**:
- Young generation collection
- Promotion to old
- Remember set accuracy
- Full collection when needed

**Estimated effort**: 2-3 context windows

---

#### Task 1.5.6: GC Scheduling

**Description**: Determine when to run GC.

**Files to modify**:
- `crates/lona-kernel/src/gc/scheduler.rs` (new)
- `crates/lona-kernel/src/scheduler/mod.rs`

**Requirements**:
- Trigger on allocation pressure
- Incremental work between process yields
- Per-process isolation (one process's GC doesn't affect others)
- `gc` and `gc-stats` primitives

**Tests**:
- GC triggered on pressure
- Incremental progress
- Process isolation
- Statistics accuracy

**Estimated effort**: 1-2 context windows

---

### Phase 1.6: Domain Isolation & IPC

Implement seL4-based security domains and inter-domain communication.

#### Task 1.6.1: VSpace Manager

**Description**: Manage seL4 virtual address spaces.

**Files to modify**:
- `crates/lona-runtime/src/domain/vspace.rs` (new)

**Requirements**:
- Create new VSpace (address space)
- Map pages into VSpace
- Unmap pages
- Track mapped regions

**Tests**:
- VSpace creation
- Page mapping
- Region tracking

**Estimated effort**: 2 context windows

---

#### Task 1.6.2: CSpace Manager

**Description**: Manage seL4 capability spaces.

**Files to modify**:
- `crates/lona-runtime/src/domain/cspace.rs` (new)

**Requirements**:
- Create new CSpace
- Allocate capability slots
- Copy/mint capabilities
- Delete capabilities

**Tests**:
- CSpace creation
- Slot allocation
- Capability operations

**Estimated effort**: 2 context windows

---

#### Task 1.6.3: Domain Data Structure

**Description**: Define domain representation.

**Files to modify**:
- `crates/lona-kernel/src/domain/mod.rs` (new)
- `crates/lona-kernel/src/domain/domain.rs` (new)

**Requirements**:
- Domain contains: VSpace, CSpace, processes, capabilities
- Domain hierarchy (parent/child)
- Domain metadata and naming
- Memory limit tracking

**Tests**:
- Domain creation
- Hierarchy tracking
- Resource limits

**Estimated effort**: 1 context window

---

#### Task 1.6.4: Domain Creation Primitive

**Description**: Implement domain spawning.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-runtime/src/domain/mod.rs`

**Requirements**:
- `spawn` with `:domain` option creates new domain
- Capability list specifies granted caps
- Memory limit specification
- Metadata attachment

**Tests**:
- Domain spawn
- Capability granting
- Memory limits
- Metadata

**Estimated effort**: 2 context windows

---

#### Task 1.6.5: Shared Memory Regions

**Description**: Implement shared memory for IPC.

**Files to modify**:
- `crates/lona-runtime/src/domain/shared.rs` (new)

**Requirements**:
- Allocate physically contiguous region
- Map into multiple domains
- Capability-controlled access
- Ring buffer structure for messages

**Tests**:
- Region allocation
- Multi-domain mapping
- Access control

**Estimated effort**: 2 context windows

---

#### Task 1.6.6: Inter-Domain IPC - Notification

**Description**: Use seL4 notifications for IPC signaling.

**Files to modify**:
- `crates/lona-runtime/src/domain/ipc.rs` (new)

**Requirements**:
- Create notification endpoints
- Signal on message send
- Wait for notification
- Integrate with scheduler

**Tests**:
- Notification send/receive
- Integration with wait

**Estimated effort**: 1-2 context windows

---

#### Task 1.6.7: Inter-Domain IPC - Message Passing

**Description**: Implement cross-domain send/receive.

**Files to modify**:
- `crates/lona-kernel/src/process/mod.rs`
- `crates/lona-runtime/src/domain/ipc.rs`

**Requirements**:
- Serialize message to shared buffer
- Notify target domain
- Deserialize on receive
- Transparent to Lonala code

**Tests**:
- Cross-domain send
- Message integrity
- Transparency (same API)

**Estimated effort**: 2 context windows

---

#### Task 1.6.8: Capability Transfer

**Description**: Transfer capabilities across domains.

**Files to modify**:
- `crates/lona-runtime/src/domain/ipc.rs`
- `crates/lona-runtime/src/domain/cspace.rs`

**Requirements**:
- Send capability with message
- Receive and install capability
- Attenuation during transfer
- Revocation support

**Tests**:
- Capability send
- Attenuation
- Revocation cascades

**Estimated effort**: 2 context windows

---

#### Task 1.6.9: Code Sharing Between Domains

**Description**: Share bytecode read-only across domains.

**Files to modify**:
- `crates/lona-runtime/src/domain/code.rs` (new)

**Requirements**:
- Map bytecode pages read-only to children
- Clone dispatch table to children
- Isolation of hot patches

**Tests**:
- Code sharing
- Dispatch table isolation
- Hot patch isolation

**Estimated effort**: 1-2 context windows

---

### Phase 1.7: Fault Tolerance

Implement Erlang-style fault tolerance mechanisms.

#### Task 1.7.1: Process Linking

**Description**: Bidirectional process links for crash propagation.

**Files to modify**:
- `crates/lona-kernel/src/process/links.rs` (new)
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(link pid)` creates bidirectional link
- `(unlink pid)` removes link
- `(spawn-link fn)` atomic spawn+link
- Exit propagates to linked processes

**Tests**:
- Link creation
- Exit propagation
- Unlink stops propagation
- spawn-link atomicity

**Estimated effort**: 1-2 context windows

---

#### Task 1.7.2: Process Monitoring

**Description**: Unidirectional monitoring without crash propagation.

**Files to modify**:
- `crates/lona-kernel/src/process/monitors.rs` (new)
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(monitor pid)` starts monitoring, returns ref
- `(demonitor ref)` stops monitoring
- `:DOWN` message on monitored process exit
- Monitor doesn't propagate crash

**Tests**:
- Monitor creation
- DOWN message delivery
- Demonitor stops messages
- No crash propagation

**Estimated effort**: 1-2 context windows

---

#### Task 1.7.3: Exit Signals

**Description**: Implement exit signal delivery and trapping.

**Files to modify**:
- `crates/lona-kernel/src/process/signals.rs` (new)
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- Normal exit (`:normal`) doesn't crash linked
- Abnormal exit crashes linked (unless trapped)
- `(process-flag :trap-exit true)` enables trapping
- Trapped exits become messages

**Tests**:
- Normal exit behavior
- Abnormal exit propagation
- Exit trapping
- Trap exit messages

**Estimated effort**: 1-2 context windows

---

#### Task 1.7.4: Panic Implementation

**Description**: Implement untrappable `panic!`.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- `(panic! msg)` terminates immediately
- `(panic! msg data)` with context
- Cannot be trapped
- Supervisor still notified

**Tests**:
- Panic terminates
- Cannot be trapped
- Supervisor notification

**Estimated effort**: 0.5 context windows

---

#### Task 1.7.5: Cross-Domain Fault Tolerance

**Description**: Links and monitors work across domain boundaries.

**Files to modify**:
- `crates/lona-kernel/src/process/links.rs`
- `crates/lona-kernel/src/process/monitors.rs`
- `crates/lona-runtime/src/domain/ipc.rs`

**Requirements**:
- Link to process in another domain
- Monitor across domains
- Exit signals cross domains
- Domain crash affects all its processes

**Tests**:
- Cross-domain link
- Cross-domain monitor
- Domain crash handling

**Estimated effort**: 2 context windows

---

### Phase 1.8: Native Primitives

Implement remaining native functions from minimal-rust.md.

#### Task 1.8.1: Type Predicates - Complete Set

**Description**: Implement all type predicate functions.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `nil?`, `boolean?`, `integer?`, `float?`, `ratio?`
- `symbol?`, `keyword?`, `string?`, `binary?`
- `list?`, `vector?`, `map?`, `set?`, `fn?`
- `coll?`, `seq?`

**Tests**:
- Each predicate returns correct result
- Cross-type testing

**Estimated effort**: 1 context window

---

#### Task 1.8.2: Bitwise Operations

**Description**: Implement bitwise primitives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `bit-and`, `bit-or`, `bit-xor`, `bit-not`
- `bit-shift-left`, `bit-shift-right`
- Work on integers

**Tests**:
- Each operation
- Edge cases (negative numbers, large shifts)

**Estimated effort**: 1 context window

---

#### Task 1.8.3: Collection Primitives - nth, count, conj

**Description**: Implement core collection accessors.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(nth coll idx)` - indexed access
- `(count coll)` - collection size
- `(conj coll item)` - add to collection

**Tests**:
- Each function on each collection type
- Edge cases (empty, out of bounds)

**Estimated effort**: 1 context window

---

#### Task 1.8.4: Map Operations - get, assoc, dissoc, keys, vals

**Description**: Implement map manipulation primitives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(get m k)` - get value for key
- `(assoc m k v)` - add/update key
- `(dissoc m k)` - remove key
- `(keys m)`, `(vals m)` - key/value sequences

**Tests**:
- Each operation
- Missing key behavior
- Large maps

**Estimated effort**: 1 context window

---

#### Task 1.8.5: Set Operations - disj, contains?

**Description**: Implement set manipulation primitives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(disj set elem)` - remove element
- `(contains? set elem)` - membership test

**Tests**:
- Each operation
- Missing element behavior

**Estimated effort**: 0.5 context windows

---

#### Task 1.8.6: Binary Operations

**Description**: Implement binary buffer primitives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(make-binary size)` - allocate zeroed buffer
- `(binary-len buf)` - get length
- `(binary-get buf idx)` - get byte
- `(binary-set buf idx byte)` - set byte
- `(binary-slice buf start end)` - zero-copy view
- `(binary-copy! dst dst-off src src-off len)` - copy bytes

**Tests**:
- Each operation
- Bounds checking
- Slice sharing

**Estimated effort**: 1-2 context windows

---

#### Task 1.8.7: Symbol Operations

**Description**: Implement symbol creation primitives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(symbol name)` - create/intern symbol
- `(gensym)` - unique symbol (already done in 1.2.10)
- `(name sym)` - get symbol name as string

**Tests**:
- Symbol creation
- Interning verification
- Name extraction

**Estimated effort**: 0.5 context windows

---

#### Task 1.8.8: Metadata Operations

**Description**: Implement metadata primitives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(meta obj)` - get metadata
- `(with-meta obj m)` - return copy with metadata
- `(vary-meta obj f args...)` - transform metadata

**Tests**:
- Get/set metadata
- vary-meta transformation
- Unsupported types return nil

**Estimated effort**: 1 context window

---

#### Task 1.8.9: MMIO Primitives

**Description**: Implement memory-mapped I/O for drivers.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-runtime/src/platform/mmio.rs` (new)

**Requirements**:
- `peek-u8/u16/u32/u64` - read from address
- `poke-u8/u16/u32/u64` - write to address
- Requires capability to access address range

**Tests**:
- Read/write operations (mock)
- Capability enforcement

**Estimated effort**: 1-2 context windows

---

#### Task 1.8.10: DMA Primitives

**Description**: Implement DMA buffer management.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-runtime/src/platform/dma.rs` (new)

**Requirements**:
- `(dma-alloc size)` - allocate DMA-capable buffer
- `(phys-addr binary)` - get physical address
- `(memory-barrier)` - ensure ordering

**Tests**:
- Allocation
- Physical address retrieval
- Barrier execution

**Estimated effort**: 1-2 context windows

---

#### Task 1.8.11: IRQ Primitives

**Description**: Implement interrupt handling for drivers.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-runtime/src/platform/irq.rs` (new)

**Requirements**:
- `(irq-wait cap)` - block until interrupt
- IRQ wakes process via scheduler
- Requires IRQ capability

**Tests**:
- IRQ wait (mock)
- Process wake on IRQ

**Estimated effort**: 1-2 context windows

---

#### Task 1.8.12: Time Primitives

**Description**: Implement time-related functions.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/scheduler/timer.rs`

**Requirements**:
- `(now-ms)` - current time in milliseconds
- `(send-after pid delay msg)` - delayed message

**Tests**:
- Time retrieval
- Delayed message delivery

**Estimated effort**: 1 context window

---

#### Task 1.8.13: Atom Primitives

**Description**: Implement process-local mutable state.

**Files to modify**:
- `crates/lona-core/src/value/atom.rs` (new)
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lonala-parser/src/parser/mod.rs` (for `@` reader macro)

**Requirements**:
- `(atom val)` - create atom
- `(deref a)` - get current value
- `(reset! a val)` - set value
- `(compare-and-set! a old new)` - CAS
- **Parser**: Add `@` reader macro that expands `@a` to `(deref a)`

**Tests**:
- Atom creation
- Get/set operations
- CAS semantics
- `@` reader macro expansion

**Estimated effort**: 1-2 context windows

---

#### Task 1.8.14: Sorted Collections - Basic

**Description**: Implement sorted map and set.

**Files to modify**:
- `crates/lona-core/src/value/sorted.rs` (new)
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(sorted-map & kvs)` - sorted by key
- `(sorted-set & vals)` - sorted elements
- Iteration in sorted order

**Tests**:
- Creation
- Sorted iteration
- Operations maintain order

**Estimated effort**: 2 context windows

---

#### Task 1.8.15: Sorted Collections - Custom Comparators

**Description**: Add custom comparator support.

**Files to modify**:
- `crates/lona-core/src/value/sorted.rs`
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(sorted-map-by cmp & kvs)` - custom comparator
- `(sorted-set-by cmp & vals)` - custom comparator
- Comparator is a function

**Tests**:
- Custom comparator
- Reverse order
- Complex comparisons

**Estimated effort**: 1 context window

---

#### Task 1.8.16: Regular Expressions - Compilation

**Description**: Implement regex pattern compilation.

**Files to modify**:
- `crates/lona-core/src/value/regex.rs` (new)
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lonala-parser/src/parser/mod.rs`

**Requirements**:
- `(re-pattern str)` - compile regex
- `#"pattern"` reader syntax
- Use Rust regex crate (no_std compatible)

**Tests**:
- Pattern compilation
- Reader syntax
- Invalid pattern error

**Estimated effort**: 1-2 context windows

---

#### Task 1.8.17: Regular Expressions - Matching

**Description**: Implement regex matching functions.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(re-find re str)` - first match
- `(re-matches re str)` - full string match
- `(re-seq re str)` - all matches
- Group extraction

**Tests**:
- Find operations
- Match operations
- Group capture
- No match cases

**Estimated effort**: 1-2 context windows

---

#### Task 1.8.18: String Primitive Operations

**Description**: Core string operations requiring native implementation for UTF-8 handling.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-core/src/value/string.rs`

**Requirements**:
- `(string-length s)` - length in characters (not bytes)
- `(codepoint-at s idx)` - Unicode codepoint at index (returns **Integer**, not String)
- `(subs s start)` and `(subs s start end)` - substring
- Proper UTF-8 handling (index by codepoint, not byte)
- Error on invalid index
- Handle multi-byte characters correctly (emoji, CJK, etc.)

> **CHANGED**: `codepoint-at` returns an **Integer** (Unicode codepoint) instead of a single-character String. This avoids heap allocation during string iteration. Use `(char->string codepoint)` in Lonala when a String is needed.

**Tests**:
- Length of ASCII string
- Length of Unicode string (emoji, etc.)
- codepoint-at various positions (verify returns integer)
- Substring extraction
- UTF-8 boundary handling
- Error on out-of-bounds index

**Estimated effort**: 1-2 context windows

---

#### Task 1.8.19: `apply` Native Primitive (CRITICAL)

**Description**: Implement `apply` for calling functions with runtime argument lists.

**Files to modify**:
- `crates/lona-core/src/opcode/mod.rs`
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- `(apply f args)` - call f with elements of args as arguments
- New opcode `APPLY` that pops function and arg list, then invokes
- Works with any callable (functions, closures, multi-arity)
- Args must be a list/vector/seq

> **Why Native**: `apply` cannot be implemented in Lonala because the argument list length is only known at runtime. A macro cannot handle runtime lists, and there's no way to dynamically construct a function call without VM support.

**Tests**:
- Apply with empty args
- Apply with multiple args
- Apply with multi-arity function
- Apply with variadic function
- Error on non-callable
- Error on non-sequence args

**Estimated effort**: 1-2 context windows

---

#### Task 1.8.20: `type-of` Native Primitive (CRITICAL)

**Description**: Return the type of a value as a keyword.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(type-of x)` returns one of: `:nil`, `:boolean`, `:integer`, `:float`, `:ratio`, `:symbol`, `:keyword`, `:string`, `:binary`, `:list`, `:vector`, `:map`, `:set`, `:function`, `:atom`, `:regex`
- O(1) operation (check internal type tag)
- Used by protocol system for efficient dispatch

> **Why Native**: Type tags are internal to the VM. This enables O(1) protocol dispatch instead of O(N) predicate chains.

**Tests**:
- All value types return correct keyword
- Consistent across equal values

**Estimated effort**: 0.5 context windows

---

#### Task 1.8.21: `identical?` Native Primitive

**Description**: Reference equality check.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(identical? x y)` returns true iff x and y are the same object in memory
- Different from `=` which checks structural equality
- For immutable primitives (integers, keywords), identity may equal value equality
- For collections, only true if same reference

**Tests**:
- Same object is identical
- Equal but different objects are not identical
- Keywords/symbols with same name are identical (interned)
- Two equal vectors are not identical

**Estimated effort**: 0.5 context windows

---

#### Task 1.8.22: `native-print` Bootstrap Primitive (CRITICAL)

**Description**: Temporary print function for Milestone 2 bootstrap.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-runtime/src/platform/uart.rs`

**Requirements**:
- `(native-print x)` - outputs string representation to Rust internal UART
- Handles all value types (uses Rust Debug formatting)
- Returns nil
- **Temporary**: Will be deprecated when Lonala UART driver (M3) is complete

> **Why Needed**: M2 Test Framework needs to output results, but M3 (UART driver) depends on M2. This breaks the circular dependency.

**Tests**:
- Print string
- Print number
- Print collection
- Print nil

**Estimated effort**: 0.5 context windows

---

#### Task 1.8.23: `string-concat` Native Primitive

**Description**: Concatenate two strings.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-core/src/value/string.rs`

**Requirements**:
- `(string-concat s1 s2)` - returns new string with s1 followed by s2
- Both arguments must be strings
- Allocates new string (immutable strings)

> **Why Native**: String allocation requires runtime memory management.

**Tests**:
- Concat two strings
- Concat with empty string
- Concat unicode strings
- Error on non-strings

**Estimated effort**: 0.5 context windows

---

#### Task 1.8.24: `read-string` Native Primitive

**Description**: Parse a string into a Lonala value.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(read-string s)` - parses s and returns the first form
- Returns error tuple on parse failure
- Uses the Rust parser internally

> **Why Native**: The parser is implemented in Rust. This enables REPL and `eval` in Lonala.

**Tests**:
- Parse simple values (numbers, strings, keywords)
- Parse collections
- Parse nested forms
- Parse error handling

**Estimated effort**: 1 context window

---

#### Task 1.8.25: `seq` Native Primitive

**Description**: Coerce a value to a sequence.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(seq nil)` returns nil
- `(seq ())` returns nil (empty list)
- `(seq non-empty-list)` returns the list
- `(seq vector)` returns list of elements
- `(seq map)` returns list of [key value] pairs
- `(seq set)` returns list of elements
- `(seq string)` returns list of codepoints (integers)

> **Why Native**: Needs to iterate internal data structures of vectors, maps, sets.

**Tests**:
- seq on nil
- seq on empty/non-empty list
- seq on vector
- seq on map (returns kv pairs)
- seq on set
- seq on string

**Estimated effort**: 1 context window

---

#### Task 1.8.26: x86 Port I/O Primitives

**Description**: Implement x86-specific port I/O for devices that use I/O ports instead of MMIO.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-runtime/src/platform/port_io.rs` (new)

**Requirements**:
- `(port-in-u8 port)` - read byte from I/O port
- `(port-in-u16 port)` - read 16-bit from I/O port
- `(port-in-u32 port)` - read 32-bit from I/O port
- `(port-out-u8 port val)` - write byte to I/O port
- `(port-out-u16 port val)` - write 16-bit to I/O port
- `(port-out-u32 port val)` - write 32-bit to I/O port
- Uses x86 `in`/`out` instructions
- Requires I/O port capability (domain must have permission)

> **Platform-specific**: Only available on x86/x86_64. ARM uses MMIO exclusively.

**Tests**:
- Port read/write operations (mock on non-x86)
- Capability enforcement

**Estimated effort**: 1 context window

---

### Phase 1.9: Integration & Spec Tests

Ensure all components work together and pass specification tests.

#### Task 1.9.1: Spec Test Framework Enhancement

**Description**: Enhance spec test infrastructure for new features.

**Files to modify**:
- `crates/lona-spec-tests/src/lib.rs`
- Various test files

**Requirements**:
- Tests for all new value types
- Tests for all new primitives
- Tests for process operations
- Tests for domain operations

**Tests**:
- Meta: test framework tests itself

**Estimated effort**: 2-3 context windows

---

#### Task 1.9.2: Process Integration Tests

**Description**: End-to-end tests for process model.

**Files to modify**:
- `crates/lona-spec-tests/src/processes.rs` (new)

**Requirements**:
- Spawn and communication
- Linking and monitoring
- Exit propagation
- Supervision patterns

**Tests**:
- Multi-process scenarios
- Fault tolerance scenarios

**Estimated effort**: 2 context windows

---

#### Task 1.9.3: Domain Integration Tests

**Description**: End-to-end tests for domain isolation.

**Files to modify**:
- `crates/lona-spec-tests/src/domains.rs` (new)

**Requirements**:
- Domain creation
- Inter-domain messaging
- Capability transfer
- Isolation verification

**Tests**:
- Cross-domain scenarios
- Security boundary tests

**Estimated effort**: 2 context windows

---

#### Task 1.9.4: GC Integration Tests

**Description**: Verify GC correctness under load.

**Files to modify**:
- `crates/lona-spec-tests/src/gc.rs` (new)

**Requirements**:
- Long-running allocation
- Cyclic structures
- Cross-generation references
- Concurrent GC with execution

**Tests**:
- Memory pressure scenarios
- Correctness verification

**Estimated effort**: 1-2 context windows

---

#### Task 1.9.5: Full System Integration Test

**Description**: Boot complete system with all features.

**Files to modify**:
- `crates/lona-runtime/src/main.rs`
- Integration test files

**Requirements**:
- Boot to REPL
- All primitives available
- Process spawning works
- Domain creation works

**Tests**:
- Full boot sequence
- Feature availability

**Estimated effort**: 1-2 context windows

---

#### Task 1.9.6: Hot Code Loading Tests

**Description**: Verify hot code loading works correctly.

**Files to create**:
- `crates/lona-spec-tests/src/hot_loading.rs`

**Requirements**:
- Test: redefine function, callers see new version immediately
- Test: recursive function redefined mid-recursion behaves correctly
- Test: closure captures see updated function references
- Test: long-running process sees redefined functions
- Verify dispatch table updates are atomic

**Tests**:
- Immediate caller update
- Recursive update
- Closure behavior
- Long-running process behavior

**Estimated effort**: 1 context window

---

#### Task 1.9.7: Cross-Domain Code Isolation Tests

**Description**: Verify parent patches don't affect children.

**Files to create**:
- `crates/lona-spec-tests/src/domain_code_isolation.rs`

**Requirements**:
- Spawn child domain with copy of parent's dispatch table
- Redefine function in parent
- Verify child still sees old version
- Verify grandchild spawned from child sees child's version
- Test explicit `push-code` propagation when implemented

**Tests**:
- Parent patch doesn't affect child
- Child patch doesn't affect parent
- Grandchild inherits from child
- Explicit propagation works (when implemented)

**Estimated effort**: 1-2 context windows

---

#### Task 1.9.8: Dynamic Binding Tests

**Description**: Test dynamic variable binding system.

**Files to create**:
- `crates/lona-spec-tests/src/dynamic_bindings.rs`

**Requirements**:
- Test `^:dynamic` var declaration
- Test `binding` special form establishes scope
- Test bindings visible in called functions
- Test nested bindings (inner shadows outer)
- Test per-process binding isolation
- Test frame pop on normal and error exits

**Tests**:
- Simple dynamic binding
- Nested bindings
- Cross-function visibility
- Process isolation
- Error cleanup

**Estimated effort**: 1 context window

---

### Phase 1.10: Condition/Restart System

Implement Common Lisp-inspired condition system for recoverable errors.

#### Task 1.10.1: Condition Type and Signal

**Description**: Define condition representation and basic signaling.

**Files to modify**:
- `crates/lona-core/src/value/condition.rs` (new)
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- Condition is a map with at least `:type` key
- `(signal condition)` raises condition without unwinding
- If no handler, becomes process exit with condition as reason
- Condition carries arbitrary data for context
- `condition?` predicate

**Tests**:
- Create condition
- Signal with no handler (process exits)
- Condition data accessible
- Predicate works

**Estimated effort**: 1 context window

---

#### Task 1.10.2: Handler Binding Infrastructure

**Description**: Dynamic binding mechanism for condition handlers.

**Files to modify**:
- `crates/lona-kernel/src/process/conditions.rs` (new)
- `crates/lona-kernel/src/process/pcb.rs`

**Requirements**:
- Handler stack per process (similar to binding stack)
- Handler: `{:type type-keyword :fn handler-fn}`
- Multiple handlers can be bound for same type (most recent wins)
- `find-handler` searches stack for matching type
- Handlers receive condition, can inspect without unwinding

**Tests**:
- Push handler
- Find handler by type
- Most recent handler wins
- No handler returns nil

**Estimated effort**: 1 context window

---

#### Task 1.10.3: `handler-bind` Macro

**Description**: Establish condition handlers for a body of code.

**Files to create**:
- `lona/core/conditions.lona`

**Requirements**:
- `(handler-bind [type handler-fn ...] body)`
- Pushes handlers before body, pops after
- Handler function receives condition map
- Handler can: invoke restart, re-signal, return value
- Multiple type/handler pairs supported

**Tests**:
- Handler called on matching condition
- Handler not called on non-matching
- Multiple handlers
- Handler can access condition data

**Estimated effort**: 1 context window

---

#### Task 1.10.4: Restart Registry

**Description**: Per-signal-point restart registration.

**Files to modify**:
- `crates/lona-kernel/src/process/conditions.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- When condition signaled, restarts are registered
- Restart: `{:name keyword :fn restart-fn :description string}`
- Restarts stored in condition context (not global)
- `available-restarts` returns current restarts
- Restarts cleared when condition handled

**Tests**:
- Register restarts with signal
- List available restarts
- Restarts cleared after handling

**Estimated effort**: 1 context window

---

#### Task 1.10.5: `restart-case` Macro

**Description**: Establish restarts for potentially-signaling code.

**Files to modify**:
- `lona/core/conditions.lona`

**Requirements**:
- ```clojure
  (restart-case expr
    (:retry [] "Try again" (retry-logic))
    (:use-value [v] "Use provided value" v))
  ```
- Each restart becomes a continuation point
- Restart functions receive args from `invoke-restart`
- Descriptions available for interactive selection

**Tests**:
- Define restarts
- Restarts available during signal
- Restart descriptions accessible
- Multiple restarts

**Estimated effort**: 1-2 context windows

---

#### Task 1.10.6: `invoke-restart` Function

**Description**: Choose and invoke a restart from handler.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/conditions.rs`

**Requirements**:
- `(invoke-restart :restart-name args...)`
- Looks up restart by name
- Transfers control to restart point (non-local jump)
- Passes args to restart function
- Stack unwound only to restart point, not further

**Tests**:
- Invoke restart by name
- Args passed correctly
- Control transfers to restart point
- Stack properly unwound

**Estimated effort**: 2 context windows

---

#### Task 1.10.7: Basic Condition REPL Integration

**Description**: Show unhandled conditions in REPL and allow restart selection.

**Files to modify**:
- `crates/lona-runtime/src/repl.rs`

**Requirements**:
- When unhandled condition reaches REPL, show formatted error
- Display condition type and data
- List available restarts with descriptions
- User can type restart number to select
- Basic `:abort` restart returns to REPL prompt

**Note**: This is the minimal integration for conditions in the REPL. Phase 1.12 extends this with full debug mode (attach/detach, breakpoints, stepping, stack inspection).

**Tests**:
- Unhandled condition shows error and restarts
- User can select restart by number
- Abort returns to REPL
- Condition data displayed

**Estimated effort**: 1 context window

---

### Phase 1.11: Introspection System

Implement LISP-machine-style introspection and debugging capabilities as described in `goals.md`.

#### Task 1.11.1: Source Storage and Retrieval

**Description**: Store source code per-definition with provenance tracking.

**Files to modify**:
- `crates/lona-kernel/src/namespace/mod.rs`
- `crates/lona-kernel/src/vm/globals.rs`

**Requirements**:
- Each definition stores original source text
- Store provenance: file, line, timestamp, previous version chain
- Comments preceding definition attached to it
- Source accessible at runtime

**Tests**:
- Definition stores source
- Provenance tracked
- Comments preserved

**Estimated effort**: 1-2 context windows

---

#### Task 1.11.2: `source` and `disassemble` Functions

**Description**: View source code and bytecode of definitions.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/vm/introspection.rs`

**Requirements**:
- `(source fn)` - returns source string with provenance header
- `(disassemble fn)` - returns bytecode representation
- Works for any function or var
- Shows REPL vs file origin

**Tests**:
- Source of file-defined function
- Source of REPL-defined function
- Disassemble output format

**Estimated effort**: 1 context window

---

#### Task 1.11.3: Namespace Introspection

**Description**: Query namespace contents and metadata.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/namespace/mod.rs`

**Requirements**:
- `(ns-map ns)` - all mappings in namespace
- `(ns-publics ns)` - public vars only
- `(ns-interns ns)` - vars defined in this ns
- `(ns-refers ns)` - referred vars from other ns
- `(all-ns)` - list all namespaces

**Tests**:
- Query various namespace contents
- Public vs private distinction
- Referred vars listed

**Estimated effort**: 1 context window

---

#### Task 1.11.4: Process Introspection

**Description**: Inspect process state and metadata.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- `(process-info pid)` - returns map with pid, name, status, heap-size, etc.
- `(process-state pid)` - get process internal state
- `(process-messages pid)` - view mailbox contents
- `(list-processes)` - enumerate all processes

**Tests**:
- Info for running process
- Info for waiting process
- Message queue inspection

**Estimated effort**: 1-2 context windows

---

#### Task 1.11.5: Domain Introspection

**Description**: Query domain hierarchy and capabilities.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/domain/mod.rs`

**Requirements**:
- `(domain-of pid)` - get domain name for process
- `(domain-info name)` - returns map with parent, capabilities, processes, memory
- `(domain-meta name)` - get domain metadata
- `(list-domains)` - enumerate all domains
- `(find-domains query)` - find domains matching metadata query
- `(same-domain? pid1 pid2)` - check if same domain

**Tests**:
- Domain lookup
- Metadata query
- Parent/child relationships

**Estimated effort**: 1-2 context windows

---

#### Task 1.11.6: Tracing Infrastructure

**Description**: Non-blocking observation of system behavior.

**Files to modify**:
- `crates/lona-kernel/src/vm/tracing.rs` (new)
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(trace-calls fn opts)` - trace function invocations
- `(trace-messages pid opts)` - trace message send/receive
- `(untrace fn)` - stop tracing
- Trace output includes timestamps
- Minimal performance overhead

**Tests**:
- Trace function calls
- Trace message passing
- Untrace stops output

**Estimated effort**: 2 context windows

---

#### Task 1.11.7: Hot Code Propagation

**Description**: Explicit code updates between domains.

**Files to modify**:
- `crates/lona-kernel/src/domain/mod.rs`
- `crates/lona-kernel/src/vm/natives.rs`

**Requirements**:
- `(push-code domain fn-name)` - push updated function to child domain
- `(pull-code domain fn-name)` - pull updated function from parent
- `(on-code-push handler)` - register handler for incoming pushes
- Capability-controlled access

**Tests**:
- Push code to child
- Pull code from parent
- Handler can accept/reject

**Estimated effort**: 1-2 context windows

---

### Phase 1.12: Debug Infrastructure

Implement the Two-Mode Architecture for LISP-machine-style debugging within BEAM/OTP-style resilience. See [docs/lonala/debugging.md](../lonala/debugging.md) for full specification.

**Dependencies**: Phase 1.10 (Condition/Restart System), Phase 1.11 (Introspection System)

**Relationship to Phase 1.10**: Phase 1.10 provides the condition/restart mechanism and basic REPL integration. Phase 1.12 extends this with the Two-Mode Architecture: production mode (crash on error) vs debug mode (pause on error), debugger attach/detach, breakpoints, and stepping.

#### Task 1.12.1: Process Debug State

**Description**: Add debug mode flag and `:debugging` state to processes.

**Files to modify**:
- `crates/lona-kernel/src/process/pcb.rs`
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- Add `debug_mode: bool` flag to Process struct
- Add `:debugging` to process state enum
- Add `debug_channel: Option<Channel>` for debug commands
- Supervisor recognizes `:debugging` state (doesn't restart)
- Process enters `:debugging` when debugger attached and error occurs

**Tests**:
- Process state transitions include `:debugging`
- Supervisor ignores debugged processes
- Debug mode flag toggles correctly

**Estimated effort**: 1 context window

---

#### Task 1.12.2: Debug Attach/Detach

**Description**: Implement `debug-attach` and `debug-detach` primitives.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- `(debug-attach pid)` - attach debugger, set debug mode
- `(debug-detach pid)` - detach, return to production mode
- `(debug-attached? pid)` - check if debugger attached
- Requires debug capability for target domain
- Returns `:ok` or `{:error reason}`

**Tests**:
- Attach to own process
- Attach to process in same domain
- Capability enforcement for other domains
- Detach restores production mode

**Estimated effort**: 1 context window

---

#### Task 1.12.3: Panic Behavior in Debug Mode

**Description**: Modify `panic!` to pause instead of crash when debugger attached.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/process/mod.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- In production mode: `panic!` terminates process immediately
- In debug mode: `panic!` pauses process, enters `:debugging` state
- Paused process sends debug event to attached debugger
- Debug event includes: condition, stack frames, locals, available restarts
- Standard restarts available: `:abort`, `:continue` (if possible)

**Tests**:
- Panic in production mode crashes
- Panic in debug mode pauses
- Debug event sent to debugger
- Abort restart crashes process
- Continue restart resumes (when applicable)

**Estimated effort**: 2 context windows

---

#### Task 1.12.4: Stack Frame Reification

**Description**: Expose stack frames as inspectable values.

**Files to modify**:
- `crates/lona-core/src/value/mod.rs`
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- `(debug-frames pid)` - get list of frame maps for paused process
- Each frame: `{:index N :function sym :line N :file "path" :locals {...}}`
- `(debug-locals pid frame-idx)` - get locals map for specific frame
- `(debug-source pid frame-idx)` - get source code for frame
- Only works on paused/debugging processes
- Capability enforcement for cross-domain

**Tests**:
- Get frames of paused process
- Frame contains expected keys
- Locals map is accurate
- Source retrieval works

**Estimated effort**: 2 context windows

---

#### Task 1.12.5: In-Frame Evaluation

**Description**: Evaluate expressions in the context of a specific stack frame.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- `(debug-eval pid frame-idx expr)` - evaluate expr in frame context
- Expression has access to frame's local variables
- Can call functions visible from that frame
- Returns evaluation result or error
- `(debug-set-local! pid frame-idx name value)` - modify local variable

**Tests**:
- Evaluate local variable reference
- Evaluate expression using locals
- Call function from frame context
- Modify local variable
- Error on invalid frame

**Estimated effort**: 2 context windows

---

#### Task 1.12.6: Debug Control Operations

**Description**: Implement pause, continue, and stepping.

**Files to modify**:
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/scheduler/mod.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- `(debug-pause pid)` - externally pause a running process
- `(debug-continue pid)` - resume paused process
- `(debug-step pid)` - execute one expression, then pause
- `(debug-step-over pid)` - step but don't pause in called functions
- `(debug-step-out pid)` - continue until current function returns
- Stepping requires instruction-level bookkeeping

**Tests**:
- Pause running process
- Continue paused process
- Step executes one instruction
- Step-over skips called functions
- Step-out finishes current function

**Estimated effort**: 2-3 context windows

---

#### Task 1.12.7: Breakpoint Infrastructure

**Description**: Implement pattern-matching breakpoints.

**Files to modify**:
- `crates/lona-kernel/src/vm/breakpoints.rs` (new)
- `crates/lona-kernel/src/vm/natives.rs`
- `crates/lona-kernel/src/vm/interpreter/mod.rs`

**Requirements**:
- Breakpoint types: `:call`, `:return`, `:receive`
- Pattern matching on arguments/return values/messages
- Optional guard expressions
- Actions: `:pause`, `:log`, `:trace`
- `(set-breakpoint type target opts)` - create breakpoint
- `(clear-breakpoint id)` - remove breakpoint
- `(list-breakpoints)` - enumerate active breakpoints

**Tests**:
- Entry breakpoint pauses on matching call
- Return breakpoint pauses on matching return value
- Pattern matching works correctly
- Guard expressions evaluated
- Clear removes breakpoint

**Estimated effort**: 3 context windows

---

#### Task 1.12.8: Breakpoint via Dispatch Table

**Description**: Implement breakpoints using dispatch table trampolines.

**Files to modify**:
- `crates/lona-kernel/src/vm/breakpoints.rs`
- `crates/lona-kernel/src/vm/globals.rs`

**Requirements**:
- Original: `foo → bytecode-A`
- With breakpoint: `foo → trampoline → bytecode-A`
- Trampoline checks pattern, pauses if matched
- Return breakpoints wrap return path
- Minimal overhead when pattern doesn't match
- Per-domain breakpoints (don't affect other domains)

**Tests**:
- Trampoline installed correctly
- Pattern checking works
- Non-matching calls have minimal overhead
- Domain isolation maintained

**Estimated effort**: 2 context windows

---

#### Task 1.12.9: Trace-to-Break Upgrade

**Description**: Convert non-blocking traces to blocking breakpoints.

**Files to modify**:
- `crates/lona-kernel/src/vm/tracing.rs`
- `crates/lona-kernel/src/vm/breakpoints.rs`

**Requirements**:
- `(trace-to-break trace-id)` - upgrade trace to breakpoint
- Trace continues logging until pattern matches
- On match, becomes blocking breakpoint
- User can then inspect and step
- `(break-to-trace breakpoint-id)` - downgrade to trace

**Tests**:
- Trace upgraded to breakpoint
- Pattern match triggers pause
- Downgrade returns to tracing

**Estimated effort**: 1 context window

---

#### Task 1.12.10: Debugger REPL Integration

**Description**: Integrate debug mode with REPL interface.

**Files to modify**:
- `crates/lona-runtime/src/repl.rs`
- `lona/debugger.lona`

**Requirements**:
- When process pauses, switch REPL to debug mode
- Debug prompt: `proc-debug[frame]>`
- Commands: `l` (locals), `e` (eval), `u`/`d` (up/down), `c` (continue)
- Numeric input selects restart
- `q` detaches debugger
- Show formatted error/condition on pause

**Tests**:
- REPL enters debug mode on pause
- Commands work correctly
- Restart selection works
- Detach returns to normal REPL

**Estimated effort**: 2 context windows

---

#### Task 1.12.11: Supervisor Debug Awareness

**Description**: Make supervisors aware of debug state.

**Files to modify**:
- `lona/supervisor.lona` (when M2 is implemented)
- `crates/lona-kernel/src/process/mod.rs`

**Requirements**:
- Supervisor checks for `:debugging` state before restart
- Optional `:debug-timeout` configuration
- Supervisor waits for debug to complete or timeout
- After timeout, can force-crash or continue waiting
- "Resume & Crash" option for testing supervisor recovery

**Tests**:
- Supervisor doesn't restart debugging process
- Timeout triggers configured action
- Force-crash works
- Resume & crash option available

**Estimated effort**: 1-2 context windows

---

