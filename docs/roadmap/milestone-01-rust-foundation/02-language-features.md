## Phase 1.2: Language Feature Completion

Complete core language features required for idiomatic Lonala.

---

### Task 1.2.1: Multi-Arity Function Support

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

### Task 1.2.2: Closure Implementation

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

### Task 1.2.3: Sequential Destructuring

**Description**: Support `[a b & rest]` pattern in bindings.

**Files to modify**:
- `crates/lonala-compiler/src/compiler/mod.rs`
- `crates/lonala-compiler/src/compiler/destructure.rs` (new)

**Requirements**:
- `[a b c]` binds sequential elements
- `[a b & rest]` binds first two, rest to remaining
- `[a _ c]` skips element with `_`
- `:as name` binds entire collection
- Works in `let` and `fn` params

**Tests**:
- Basic vector destructuring
- Rest collection with `&`
- Underscore for ignored elements
- `:as` for whole collection binding
- Nested sequential destructuring

**Estimated effort**: 2 context windows

---

### Task 1.2.4: Associative Destructuring

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

### Task 1.2.5: Nested Destructuring

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

### Task 1.2.6: Proper Tail Calls - Compiler

**Description**: Add tail position tracking to the compiler and emit `TailCall` opcode for calls in tail position. See [docs/architecture/tco.md](../../architecture/tco.md) for full design.

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

### Task 1.2.7: Proper Tail Calls - VM Trampoline

**Description**: Restructure the VM interpreter to use a trampoline loop, enabling tail calls without Rust stack growth. See [docs/architecture/tco.md](../../architecture/tco.md) for full design.

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

### Task 1.2.8: Proper Tail Calls - Integration Tests

**Description**: Comprehensive integration tests for proper tail calls. See [docs/architecture/tco.md](../../architecture/tco.md) for full design.

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

### Task 1.2.9: Pattern Matching - Core Infrastructure

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

### Task 1.2.10: Case Special Form

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

### Task 1.2.11: Gensym Implementation

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

### Task 1.2.12: Anonymous Function Reader Macro

**Description**: Implement `#()` reader macro for anonymous functions.

**Files to modify**:
- `crates/lonala-parser/src/lexer/mod.rs`
- `crates/lonala-parser/src/parser/mod.rs`

**Requirements**:
- `#(+ %1 %2)` expands to `(fn [%1 %2] (+ %1 %2))`
- `%` or `%1` refers to first argument
- `%2`, `%3`, ... refer to subsequent arguments
- `%&` captures rest arguments
- Highest numbered arg determines arity
- Nested `#()` is an error (unlike Clojure)

**Tests**:
- Basic anonymous function
- Multiple arguments
- Rest arguments with `%&`
- Arity detection
- Nested `#()` error

**Estimated effort**: 1 context window

---

### Task 1.2.13: Discard Reader Macro

**Description**: Implement `#_` reader macro for discarding forms.

**Files to modify**:
- `crates/lonala-parser/src/lexer/mod.rs`
- `crates/lonala-parser/src/parser/mod.rs`

**Requirements**:
- `#_expr` reads and discards `expr`
- Next form after `#_expr` is returned
- Useful for commenting out forms in data
- Works with any expression (lists, vectors, maps)
- Multiple `#_` can be chained: `#_#_a b c` returns `c`

**Tests**:
- Discard symbol
- Discard list
- Discard in vector literal
- Chained discards

**Estimated effort**: 0.5 context windows

---

### Task 1.2.14: Regex Literal Syntax

**Description**: Implement `#"pattern"` reader macro for regex literals.

**Files to modify**:
- `crates/lonala-parser/src/lexer/mod.rs`
- `crates/lonala-parser/src/parser/mod.rs`

**Requirements**:
- `#"pattern"` creates a compiled regex value
- Standard regex syntax (like Rust `regex` crate)
- Regex is compiled at read time
- Invalid regex patterns are read-time errors

**Dependencies**: Requires regex primitives (Task 1.8.17) for the compiled type.

**Tests**:
- Simple regex literal
- Regex with escapes
- Invalid regex error at read time

**Estimated effort**: 0.5 context windows
